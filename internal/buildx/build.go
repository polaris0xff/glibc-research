// Package buildx runs a build with the portable toolchain injected.
//
// `pgb build` re-enters itself inside the build environment as
// `pgb __inner-build`, so the options the outer process parsed have to cross
// that boundary. They travel through the environment, not the argv: a chroot
// inherits the caller's environment and a container does not, so both arms
// pass the same variable list explicitly.
//
// Argv is passed as argv and never flattened into a string, so an argument
// containing spaces stays one argument.
//
// SPDX-License-Identifier: MIT
package buildx

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/envx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
	"github.com/polaris0xff/glibc-research/internal/rootfs"
	"github.com/polaris0xff/glibc-research/internal/wrapper"
)

var log = logx.New("build")

// InnerBuild and InnerShell are the hidden re-entry points.
const (
	InnerBuild = "__inner-build"
	InnerShell = "__inner-shell"
)

// Build runs argv with the wrappers on PATH, inside the chosen environment.
func Build(c *cfg.Config, argv []string) error {
	if len(argv) == 0 {
		return fail.Cannot("pgb build needs a command, e.g. pgb build -- make")
	}
	c.Export()
	engine := c.Engine()
	// Checked before anything is mounted or any container starts: the engines
	// keep independent environments and the chosen one may not be the one
	// `pgb env create` built for.
	if err := envx.RequireCurrent(c, engine); err != nil {
		return err
	}
	switch engine {
	case cfg.EngineHost:
		return Inner(c, argv, false)
	case cfg.EngineChroot:
		return enterChroot(c, argv, true)
	default:
		return enterContainer(c, string(engine), argv, false)
	}
}

// Shell opens an interactive shell inside the build environment.
func Shell(c *cfg.Config) error {
	engine := c.Engine()
	switch engine {
	case cfg.EngineChroot:
		if fi, err := os.Stat(c.EnvRoot()); err != nil || !fi.IsDir() {
			return fail.Cannot("no build environment. run: pgb env create")
		}
		c.Export()
		return enterChroot(c, []string{shellPath()}, false)
	case cfg.EngineDocker, cfg.EnginePodman:
		if err := envx.RequireCurrent(c, engine); err != nil {
			return err
		}
		c.Export()
		return enterContainer(c, string(engine), []string{shellPath()}, true)
	default:
		return Inner(c, []string{shellPath()}, true)
	}
}

func shellPath() string {
	if s := os.Getenv("SHELL"); s != "" {
		return s
	}
	return "/bin/sh"
}

// ChrootBinds is what enterChroot mounts: the working directory, the pgb tree
// and the state directory at the same absolute paths inside, so every absolute
// path a build system bakes into a Makefile still resolves.
//
// ⭐ SPLIT OUT OF enterChroot SO IT CAN BE ASSERTED WITHOUT A BED. T-062: this
// package's acceptance was a build environment, a network and half an hour,
// which is not something a change can be checked against while it is being
// made. `selfExists` is the one thing here that touches the filesystem and it
// is a parameter, so a case can drive both branches.
func ChrootBinds(c *cfg.Config, wd, self string, selfExists func(string) bool) []string {
	binds := []string{
		wd + ":" + wd,
		c.State + ":" + c.State,
		self + ":" + selfInside,
	}
	if c.Self != "" && c.Self != wd && selfExists(c.Self) {
		binds = append(binds, c.Self+":"+c.Self)
	}
	for _, b := range c.ExtraBinds {
		binds = append(binds, cfg.AbsBindspec(b))
	}
	return binds
}

func pathExists(p string) bool { _, err := os.Stat(p); return err == nil }

// enterChroot bind-mounts the working directory, the pgb tree and the state
// directory at the same absolute paths inside, so every absolute path a build
// system bakes into a Makefile still resolves.
func enterChroot(c *cfg.Config, argv []string, stream bool) error {
	wd, err := os.Getwd()
	if err != nil {
		return fail.Cannot("cannot read the working directory: %v", err)
	}
	self, err := os.Executable()
	if err != nil {
		return fail.Cannot("cannot locate the running pgb: %v", err)
	}
	binds := ChrootBinds(c, wd, self, pathExists)

	inner := append([]string{selfInside, InnerBuild}, argv...)
	opts := rootfs.Options{
		Root:    c.EnvRoot(),
		Bind:    binds,
		Workdir: wd,
		Env:     []string{"PGB_INNER=1"},
		Stdin:   os.Stdin,
	}
	// The environment's whole output is timestamped from out here, where pgb
	// is the process talking to the terminal. The pgb inside then sees a pipe
	// and leaves its own child's lines alone, so nothing is stamped twice.
	if stream {
		if st := logx.StreamStamper(); st != nil {
			defer st.Close()
			opts.Stdout, opts.Stderr = st, st
			opts.Env = append(opts.Env, "PGB_TS=0")
		}
	}
	code, err := rootfs.Run(opts, inner)
	if err != nil {
		return err
	}
	if code != 0 {
		return fail.Exit(code)
	}
	return nil
}

// selfInside is where pgb is bind-mounted inside the build environment. A
// fixed path keeps the re-entry independent of where the caller's copy lives.
const selfInside = "/pgb"

// ContainerRunArgv composes the whole `docker run` / `podman run` command line.
//
// ⛔ THIS FUNCTION IS WHERE T-019 LIVED, AND THAT IS WHY IT IS SPLIT OUT.
// `chroot` inherits the caller's environment and a container does not, so every
// `PGB_OPT_*` option has to be named here explicitly. It once was not: the
// docker branch passed exactly `-e PGB_INNER=1` and **every documented build
// option silently did nothing** — no warning, exit 0, and a binary without the
// mechanism the caller asked for. ⚠ And it hid behind a real result, because
// the engine cross-check that would have caught it was run on a build with no
// options, which is the one case where dropping them all changes nothing.
//
// ⭐ Composing the argv is now separable from running it, so a carried case can
// assert that what the chroot inherits and what the container is handed are the
// same set. T-062.
func ContainerRunArgv(c *cfg.Config, engine string, argv []string, interactive bool,
	wd, self string, selfExists func(string) bool, caAnchor string) []string {
	run := []string{engine, "run", "--rm"}
	if interactive {
		run = append(run, "-it")
	}
	run = append(run,
		"-v", wd+":"+wd,
		"-v", c.State+":"+c.State,
		"-v", self+":"+selfInside+":ro",
	)
	if c.Self != "" && c.Self != wd && selfExists(c.Self) {
		run = append(run, "-v", c.Self+":"+c.Self)
	}
	for _, b := range c.ExtraBinds {
		run = append(run, "-v", cfg.AbsBindspec(b))
	}
	if caAnchor != "" {
		run = append(run, "-v", caAnchor+":"+caAnchor+":ro")
	}
	run = append(run, "-w", wd, "-e", "PGB_INNER=1")
	run = append(run, c.ContainerEnvArgs()...)
	run = append(run, "pgb-env:"+cfg.Version, selfInside, InnerBuild)
	return append(run, argv...)
}

func enterContainer(c *cfg.Config, engine string, argv []string, interactive bool) error {
	wd, err := os.Getwd()
	if err != nil {
		return fail.Cannot("cannot read the working directory: %v", err)
	}
	self, err := os.Executable()
	if err != nil {
		return fail.Cannot("cannot locate the running pgb: %v", err)
	}

	run := ContainerRunArgv(c, engine, argv, interactive, wd, self, pathExists, cfg.CAAnchor())

	cmd := &proc.Cmd{Argv: run, Stdin: os.Stdin, Subsys: "build",
		Stdout: os.Stdout, Stderr: os.Stderr, Stream: !interactive}
	r, err := cmd.Run()
	if err != nil {
		return fail.Cannot("%s: %v", engine, err)
	}
	if r.Failed() {
		return fail.Exit(r.Code)
	}
	return nil
}

// Inner is what runs inside the environment: build the runtime, install the
// wrappers, put them on PATH and run the caller's command.
func Inner(c *cfg.Config, argv []string, interactive bool) error {
	if len(argv) == 0 {
		return fail.Cannot("pgb %s needs a command", InnerBuild)
	}
	b := wrapper.NewBuilder(c)
	rd, err := b.Build()
	if err != nil {
		return err
	}
	m := wrapper.BuildManifest(c, rd)
	wd, err := wrapper.Make(c, m)
	if err != nil {
		return err
	}
	log.Infof("wrappers: %s", wd)
	log.Infof("runtime:  %s", rd)

	env := []string{
		"PATH=" + wd + string(os.PathListSeparator) + os.Getenv("PATH"),
		"CC=" + filepath.Join(wd, "cc"),
		"CXX=" + filepath.Join(wd, "c++"),
		"PGB_WRAPPER_DIR=" + wd,
	}
	if c.Verbose {
		env = append(env, "PGB_VERBOSE=1")
	}

	cmd := &proc.Cmd{
		Argv:   argv,
		Env:    env,
		Stdin:  os.Stdin,
		Stdout: os.Stdout,
		Stderr: os.Stderr,
		Stream: !interactive,
		Subsys: "build",
	}
	r, err := cmd.Run()
	if err != nil {
		return fail.Cannot("cannot run %s: %v", argv[0], err)
	}
	if r.Failed() {
		return fail.Exit(r.Code)
	}
	return nil
}

// CCDir prepares the wrapper directory and returns its path, for a caller that
// wants to drive the compiler by hand.
func CCDir(c *cfg.Config) (string, error) {
	b := wrapper.NewBuilder(c)
	rd, err := b.Build()
	if err != nil {
		return "", err
	}
	m := wrapper.BuildManifest(c, rd)
	return wrapper.Make(c, m)
}

// Describe renders the build for a log line.
func Describe(c *cfg.Config, argv []string) string {
	return string(c.Engine()) + ": " + strings.Join(argv, " ")
}
