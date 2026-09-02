// Package rootfs runs a command inside an unpacked root filesystem, in a
// private mount namespace, with nothing of the host's userland visible.
//
// The target distribution's /lib, /usr/lib, /etc and its loader are the only
// ones the process can see. Nothing of the host is mounted in except /proc,
// /sys, /dev and a resolv.conf — each a kernel or network interface rather
// than a userland one. A test that needs an artefact inside passes Copy, so
// the copy is visibly part of the experiment.
//
// This is not a security boundary and not a container: the PID, network, user
// and IPC namespaces are shared with the host unless PrivateNet is set, and
// the kernel is the host kernel.
//
// SPDX-License-Identifier: MIT
package rootfs

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

var log = logx.New("rootfs")

// Options describes one run inside a root filesystem.
type Options struct {
	Root       string   // the unpacked filesystem
	Copy       []string // SRC[:DEST], copied in before the namespace is entered
	Bind       []string // SRC[:DEST], bind-mounted inside
	Workdir    string   // "" or "/" means the root
	NoNet      bool     // do not replicate resolv.conf or the TLS anchor
	PrivateNet bool     // also unshare the network namespace

	DNS    string // override the nameserver written to /etc/resolv.conf
	Stdin  io.Reader
	Stdout io.Writer
	Stderr io.Writer
	Env    []string // extra NAME=VALUE for the command
}

// innerCommand is the hidden re-entry that does the mounts. pgb re-execs
// itself with CLONE_NEWNS so the mount namespace belongs to the whole child
// process rather than to one thread of this one.
const innerCommand = "__rootfs-inner"

// InnerCommand is the subcommand name the dispatcher must route here.
func InnerCommand() string { return innerCommand }

// Run enters the root filesystem and runs argv, returning the command's own
// exit status.
func Run(o Options, argv []string) (int, error) {
	if o.Root == "" {
		return 2, fail.Cannot("rootfs: no root filesystem given")
	}
	if len(argv) == 0 {
		return 2, fail.Cannot("rootfs: no command given")
	}
	root, err := filepath.Abs(o.Root)
	if err != nil {
		return 2, fail.Cannot("rootfs: cannot resolve %s: %v", o.Root, err)
	}
	if fi, err := os.Stat(root); err != nil || !fi.IsDir() {
		return 2, fail.Cannot("rootfs: %s is not a directory", o.Root)
	}
	if os.Geteuid() != 0 {
		return 2, fail.Cannot("rootfs: needs root for mount and chroot")
	}

	// Copies happen outside the namespace so they persist and can be inspected
	// after the run: an experiment's inputs should still be on disk afterwards.
	for _, spec := range o.Copy {
		if err := copyInto(root, spec); err != nil {
			return 2, err
		}
	}
	if !o.NoNet {
		if err := replicateNetwork(root, o.DNS); err != nil {
			return 2, err
		}
	}

	self, err := os.Executable()
	if err != nil {
		return 2, fail.Cannot("rootfs: cannot locate the running pgb: %v", err)
	}

	inner := append([]string{self, innerCommand,
		"--root", root,
		"--workdir", defaultWorkdir(o.Workdir),
	}, bindArgs(o.Bind)...)
	inner = append(inner, "--")
	inner = append(inner, argv...)

	log.Debugf("entering %s: %s", root, logx.QuoteArgs(argv))

	cmd := exec.Command(inner[0], inner[1:]...)
	cmd.Stdin, cmd.Stdout, cmd.Stderr = o.Stdin, o.Stdout, o.Stderr
	if cmd.Stdout == nil {
		cmd.Stdout = os.Stdout
	}
	if cmd.Stderr == nil {
		cmd.Stderr = os.Stderr
	}
	if len(o.Env) > 0 {
		cmd.Env = append(os.Environ(), o.Env...)
	}
	flags := uintptr(syscall.CLONE_NEWNS)
	if o.PrivateNet {
		flags |= syscall.CLONE_NEWNET
	}
	cmd.SysProcAttr = &syscall.SysProcAttr{Unshareflags: flags}

	if err := cmd.Run(); err != nil {
		if ee, ok := err.(*exec.ExitError); ok {
			if ws, ok := ee.Sys().(syscall.WaitStatus); ok && ws.Signaled() {
				return 128 + int(ws.Signal()), nil
			}
			return ee.ExitCode(), nil
		}
		return 2, fail.Cannot("rootfs: cannot start the runner: %v", err)
	}
	return 0, nil
}

func bindArgs(binds []string) []string {
	var out []string
	for _, b := range binds {
		if b != "" {
			out = append(out, "--bind", b)
		}
	}
	return out
}

func defaultWorkdir(w string) string {
	if w == "" {
		return "/"
	}
	return w
}

// Inner performs the mounts and the chroot. It runs in the child process, in
// its own mount namespace, and never returns on success.
func Inner(args []string) error {
	var root, workdir string
	var binds []string
	var argv []string
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--root":
			i++
			if i < len(args) {
				root = args[i]
			}
		case "--workdir":
			i++
			if i < len(args) {
				workdir = args[i]
			}
		case "--bind":
			i++
			if i < len(args) {
				binds = append(binds, args[i])
			}
		case "--":
			argv = args[i+1:]
			i = len(args)
		}
	}
	if root == "" || len(argv) == 0 {
		return fail.Cannot("%s: needs --root and a command", innerCommand)
	}

	for _, d := range []string{"proc", "sys", "dev", "tmp"} {
		_ = os.MkdirAll(filepath.Join(root, d), 0o755)
	}
	mountQuiet("none", filepath.Join(root, "proc"), "proc", 0, "")
	mountQuiet("none", filepath.Join(root, "sys"), "sysfs", 0, "")
	mountQuiet("/dev", filepath.Join(root, "dev"), "", syscall.MS_BIND|syscall.MS_REC, "")
	mountQuiet("none", filepath.Join(root, "dev"), "", syscall.MS_SLAVE|syscall.MS_REC, "")
	mountQuiet("none", filepath.Join(root, "tmp"), "tmpfs", 0, "")

	for _, spec := range binds {
		src, dst, ok := strings.Cut(spec, ":")
		if !ok {
			dst = src
		}
		target := filepath.Join(root, dst)
		if fi, err := os.Stat(src); err == nil && !fi.IsDir() {
			_ = os.MkdirAll(filepath.Dir(target), 0o755)
			if _, err := os.Stat(target); err != nil {
				_ = os.WriteFile(target, nil, 0o644)
			}
		} else {
			_ = os.MkdirAll(target, 0o755)
		}
		if err := syscall.Mount(src, target, "", syscall.MS_BIND|syscall.MS_REC, ""); err != nil {
			return fail.Cannot("cannot bind %s onto %s: %v", src, target, err)
		}
	}

	if err := syscall.Chroot(root); err != nil {
		return fail.Cannot("chroot %s: %v", root, err)
	}
	// The working directory must be set after the chroot, and a directory that
	// does not exist inside is the caller's error rather than the runner's.
	if err := os.Chdir(defaultWorkdir(workdir)); err != nil {
		return fail.Cannot("cd %s inside the root filesystem: %v", workdir, err)
	}

	// The command is exec'd directly. Routing through the target's /bin/sh
	// would turn every distroless or single-binary filesystem into exit 127,
	// which reads as "the binary failed" when it means "the runner could not
	// start it".
	path := argv[0]
	if !strings.ContainsRune(path, os.PathSeparator) {
		if p, err := exec.LookPath(path); err == nil {
			path = p
		}
	}
	if err := syscall.Exec(path, argv, os.Environ()); err != nil {
		return fail.Cannot("cannot run %s inside %s: %v", argv[0], root, err)
	}
	return nil
}

func mountQuiet(source, target, fstype string, flags uintptr, data string) {
	if err := syscall.Mount(source, target, fstype, flags, data); err != nil {
		log.Tracef("mount %s on %s: %v", source, target, err)
	}
}

// replicateNetwork copies the two things that are network interfaces rather
// than host userland: the resolver configuration, and the TLS trust anchor the
// caller's own environment already names.
func replicateNetwork(root, dns string) error {
	etc := filepath.Join(root, "etc")
	if err := os.MkdirAll(etc, 0o755); err != nil {
		return fail.Cannot("cannot create %s: %v", etc, err)
	}
	resolv := filepath.Join(etc, "resolv.conf")
	switch {
	case dns != "":
		if err := os.WriteFile(resolv, []byte("nameserver "+dns+"\n"), 0o644); err != nil {
			return fail.Cannot("cannot write %s: %v", resolv, err)
		}
	default:
		if fi, err := os.Stat(resolv); err == nil && fi.Size() > 0 {
			break
		}
		if b, err := os.ReadFile("/etc/resolv.conf"); err == nil && len(b) > 0 {
			_ = os.WriteFile(resolv, b, 0o644)
		}
	}

	// Only the file the caller's variables already name is copied, to the same
	// absolute path so the inherited variable keeps resolving. This adds no
	// trust the caller did not already have.
	if anchor := cfg.CAAnchor(); anchor != "" {
		dst := filepath.Join(root, anchor)
		if _, err := os.Stat(dst); err != nil {
			if err := os.MkdirAll(filepath.Dir(dst), 0o755); err == nil {
				if b, err := os.ReadFile(anchor); err == nil {
					_ = os.WriteFile(dst, b, 0o644)
				}
			}
		}
	}
	return nil
}

// copyInto handles a SRC[:DEST] copy specification.
func copyInto(root, spec string) error {
	src, dst, ok := strings.Cut(spec, ":")
	if !ok {
		dst = "/" + filepath.Base(src)
	}
	target := filepath.Join(root, dst)
	if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
		return fail.Cannot("cannot create %s in the root filesystem: %v", filepath.Dir(dst), err)
	}
	fi, err := os.Stat(src)
	if err != nil {
		return fail.Cannot("cannot copy %s: %v", src, err)
	}
	if fi.IsDir() {
		if r, err := proc.Run("cp", "-a", src, target); err != nil || r.Failed() {
			return fail.Cannot("cannot copy %s -> %s", src, dst)
		}
		return nil
	}
	in, err := os.Open(src)
	if err != nil {
		return fail.Cannot("cannot read %s: %v", src, err)
	}
	defer in.Close()
	out, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, fi.Mode().Perm())
	if err != nil {
		return fail.Cannot("cannot write %s: %v", target, err)
	}
	if _, err := io.Copy(out, in); err != nil {
		out.Close()
		return fail.Cannot("cannot copy %s -> %s: %v", src, dst, err)
	}
	return out.Close()
}

// ParseOptions reads the flags `pgb rootfs run` accepts, returning the options
// and the command after `--`.
func ParseOptions(args []string) (Options, []string, error) {
	var o Options
	var argv []string
	for i := 0; i < len(args); i++ {
		a := args[i]
		val := func() (string, error) {
			if i+1 >= len(args) {
				return "", fail.Cannot("rootfs: %s needs a value", a)
			}
			i++
			return args[i], nil
		}
		var err error
		var v string
		switch a {
		case "--copy":
			if v, err = val(); err == nil {
				o.Copy = append(o.Copy, v)
			}
		case "--bind":
			if v, err = val(); err == nil {
				o.Bind = append(o.Bind, v)
			}
		case "--dns":
			if v, err = val(); err == nil {
				o.DNS = v
			}
		case "--workdir":
			if v, err = val(); err == nil {
				o.Workdir = v
			}
		case "--no-net":
			o.NoNet = true
		case "--private-net":
			o.PrivateNet = true
		case "--":
			argv = args[i+1:]
			i = len(args)
		default:
			if strings.HasPrefix(a, "-") {
				return o, nil, fail.Cannot("rootfs: unknown argument: %s", a)
			}
			o.Root = a
		}
		if err != nil {
			return o, nil, err
		}
	}
	return o, argv, nil
}

// Describe renders the run for a log line.
func Describe(o Options, argv []string) string {
	return fmt.Sprintf("%s binds=%v workdir=%s -- %s",
		o.Root, o.Bind, defaultWorkdir(o.Workdir), logx.QuoteArgs(argv))
}
