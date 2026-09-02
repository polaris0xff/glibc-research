// create.go — `pgb env create` and `pgb env info`.
//
// The chroot arm unpacks the pinned image and installs into it; the container
// arm builds an image from a generated Dockerfile. Both install the same
// packages and build the same static libiconv, and both record the same stamp,
// which is what keeps the two from drifting.
//
// SPDX-License-Identifier: MIT
package envx

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/ociimg"
	"github.com/polaris0xff/glibc-research/internal/proc"
	"github.com/polaris0xff/glibc-research/internal/rootfs"
)

// InnerLibiconv is the hidden subcommand that builds libiconv from inside a
// build environment.
const InnerLibiconv = "__build-libiconv"

// Create builds the environment for the chosen engine.
func Create(c *cfg.Config) error {
	switch e := c.Engine(); e {
	case cfg.EngineChroot:
		return createChroot(c)
	case cfg.EngineDocker, cfg.EnginePodman:
		return createImage(c, string(e))
	default:
		logx.Say("engine 'host': nothing to create. Builds use THIS machine's glibc.")
		return nil
	}
}

func createChroot(c *cfg.Config) error {
	if os.Geteuid() != 0 {
		return fail.Cannot("the chroot engine needs root")
	}
	root := c.EnvRoot()
	if _, err := os.Stat(filepath.Join(root, ".pgb-env")); err == nil {
		logx.Say("environment already at %s (delete it to rebuild)", root)
		return nil
	}
	logx.Say("creating %s from %s (%s)", c.EnvName, c.EnvImage, c.EnvDigest)
	if _, err := ociimg.Pull(ociimg.Options{
		Ref: c.EnvImage, Digest: c.EnvDigest, Out: root,
	}); err != nil {
		return fail.Ran("image pull failed: %v", err)
	}

	logFile := root + ".install.log"
	lf, err := os.Create(logFile)
	if err != nil {
		return fail.Cannot("cannot write %s: %v", logFile, err)
	}
	defer lf.Close()

	run := func(what string, argv ...string) error {
		code, err := rootfs.Run(rootfs.Options{
			Root:   root,
			Stdout: lf,
			Stderr: lf,
			Env:    []string{"DEBIAN_FRONTEND=noninteractive"},
		}, argv)
		if err != nil {
			return err
		}
		if code != 0 {
			showTail(logFile)
			return fail.Ran("%s failed (exit %d); see %s", what, code, logFile)
		}
		return nil
	}

	logx.Say("installing: %s", strings.Join(c.EnvPackages, " "))
	if err := run("apt-get update", "/usr/bin/apt-get", "update", "-qq"); err != nil {
		return err
	}
	install := append([]string{"/usr/bin/apt-get", "install", "-y", "-qq",
		"--no-install-recommends"}, c.EnvPackages...)
	if err := run("package install", install...); err != nil {
		return err
	}
	if len(c.EnvPip) > 0 {
		logx.Say("installing (pip): %s", strings.Join(c.EnvPip, " "))
		pip := append([]string{"/usr/bin/python3", "-m", "pip", "install",
			"--break-system-packages", "--no-cache-dir", "-q"}, c.EnvPip...)
		if err := run("pip install", pip...); err != nil {
			return err
		}
	}

	if c.UseIconv {
		// libiconv must be built by the environment's own compiler against the
		// environment's own glibc: an archive built on the host would link and
		// then carry the host's ABI assumptions into the pinned build.
		logx.Say("building GNU libiconv inside the environment")
		self, err := os.Executable()
		if err != nil {
			return fail.Cannot("cannot locate the running pgb: %v", err)
		}
		code, err := rootfs.Run(rootfs.Options{
			Root:   root,
			Bind:   []string{self + ":/pgb-tool"},
			Stdout: lf,
			Stderr: lf,
		}, []string{"/pgb-tool", InnerLibiconv, "--prefix", c.LibiconvPrefix})
		if err != nil {
			return err
		}
		if code != 0 {
			showTail(logFile)
			return fail.Ran("libiconv build failed (exit %d); see %s", code, logFile)
		}
	}

	gccVersion := firstLineOf(captureInRoot(root, "/usr/bin/gcc", "--version"))
	glibcVersion := firstLineOf(captureInRoot(root, "/usr/bin/ldd", "--version"))

	var desc strings.Builder
	fmt.Fprintf(&desc, "image: %s\n", c.EnvImage)
	fmt.Fprintf(&desc, "digest: %s\n", c.EnvDigest)
	fmt.Fprintf(&desc, "packages: %s\n", strings.Join(c.EnvPackages, " "))
	fmt.Fprintf(&desc, "pip: %s\n", strings.Join(c.EnvPip, " "))
	fmt.Fprintf(&desc, "created: %s\n", time.Now().UTC().Format("2006-01-02T15:04:05Z"))
	fmt.Fprintf(&desc, "gcc: %s\n", gccVersion)
	fmt.Fprintf(&desc, "glibc: %s\n", glibcVersion)
	if err := os.WriteFile(filepath.Join(root, ".pgb-env"), []byte(desc.String()), 0o644); err != nil {
		return fail.Cannot("cannot write the environment description: %v", err)
	}
	// The machine-readable half is written last, so a half-built environment
	// has no stamp and is refused rather than trusted.
	if err := os.WriteFile(filepath.Join(root, ".pgb-env-stamp"),
		[]byte(Want(c).String()), 0o644); err != nil {
		return fail.Cannot("cannot write the environment stamp: %v", err)
	}
	logx.Say("created. %s", gccVersion)
	return nil
}

func createImage(c *cfg.Config, engine string) error {
	logx.Say("building image pgb-env:%s with %s", cfg.Version, engine)
	ctx := filepath.Join(c.State, "docker")
	if err := os.RemoveAll(ctx); err != nil {
		return fail.Cannot("cannot clear %s: %v", ctx, err)
	}
	if err := os.MkdirAll(filepath.Join(ctx, "ca"), 0o755); err != nil {
		return fail.Cannot("cannot create %s: %v", ctx, err)
	}
	self, err := os.Executable()
	if err != nil {
		return fail.Cannot("cannot locate the running pgb: %v", err)
	}
	if err := copyFile(self, filepath.Join(ctx, "pgb"), 0o755); err != nil {
		return fail.Cannot("cannot stage pgb into the build context: %v", err)
	}

	var caStep string
	if anchor := cfg.CAAnchor(); anchor != "" {
		// Where the environment terminates TLS, the image needs the anchor the
		// caller's own variables name or every HTTPS fetch inside fails with a
		// message about certificates that reads as a broken dependency.
		b, err := os.ReadFile(anchor)
		if err != nil {
			return fail.Cannot("cannot read CA anchor %s: %v", anchor, err)
		}
		if err := os.WriteFile(filepath.Join(ctx, "ca", "pgb-proxy-ca.crt"), b, 0o644); err != nil {
			return err
		}
		logx.Say("carrying the trust anchor named by your environment: %s", anchor)
		caStep = "COPY ca/pgb-proxy-ca.crt /usr/local/share/ca-certificates/pgb-proxy-ca.crt\nRUN update-ca-certificates"
	} else {
		if err := os.WriteFile(filepath.Join(ctx, "ca", ".keep"), nil, 0o644); err != nil {
			return err
		}
	}

	// pip talks HTTPS to pypi, so it goes after the anchor step, not before.
	pipStep := ""
	if len(c.EnvPip) > 0 {
		pipStep = "RUN python3 -m pip install --break-system-packages --no-cache-dir " +
			strings.Join(c.EnvPip, " ")
	}
	iconvStep := ""
	if c.UseIconv {
		iconvStep = fmt.Sprintf("RUN /opt/pgb/pgb %s --prefix %s", InnerLibiconv, c.LibiconvPrefix)
	}

	dockerfile := fmt.Sprintf(`FROM %s@%s
LABEL org.pgb.stamp="%s"
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
      %s curl ca-certificates && rm -rf /var/lib/apt/lists/*
%s
%s
COPY pgb /opt/pgb/pgb
%s
`, c.EnvImage, c.EnvDigest, Want(c).String(),
		strings.Join(c.EnvPackages, " "), caStep, pipStep, iconvStep)

	if err := os.WriteFile(filepath.Join(ctx, "Dockerfile"), []byte(dockerfile), 0o644); err != nil {
		return err
	}
	r, err := proc.Run(engine, "build", "-t", "pgb-env:"+cfg.Version, ctx)
	if err != nil {
		return fail.Cannot("%s: %v", engine, err)
	}
	if r.Failed() {
		return fail.Ran("image build failed")
	}
	logx.Say("built pgb-env:%s", cfg.Version)
	return nil
}

// Info prints what the environment is.
func Info(c *cfg.Config) error {
	root := c.EnvRoot()
	logx.Say("build environment")
	logx.Say("  %-22s %s", "engine", c.Engine())
	logx.Say("  %-22s %s", "image", c.EnvImage)
	logx.Say("  %-22s %s", "digest", c.EnvDigest)
	logx.Say("  %-22s %s", "packages", strings.Join(c.EnvPackages, " "))
	logx.Say("  %-22s %s", "root", root)
	if fi, err := os.Stat(root); err == nil && fi.IsDir() {
		logx.Say("  %-22s %s", "state", "created")
		if b, err := os.ReadFile(filepath.Join(root, ".pgb-env")); err == nil {
			for line := range strings.SplitSeq(strings.TrimRight(string(b), "\n"), "\n") {
				logx.Say("    %s", line)
			}
		}
	} else {
		logx.Say("  %-22s %s", "state", "NOT created -- run: pgb env create")
	}
	logx.Say("")
	logx.Say("  why this image: glibc 2.36. At or above 2.34 the 'files' and 'dns'")
	logx.Say("  NSS services are implemented inside libc, which is what leaves the")
	logx.Say("  NSS override with nothing to dlopen. Below that floor it would move")
	logx.Say("  the dlopen rather than remove it -- see experiments/21.")
	return nil
}

func captureInRoot(root string, argv ...string) string {
	var out strings.Builder
	code, err := rootfs.Run(rootfs.Options{Root: root, Stdout: &out, Stderr: &out}, argv)
	if err != nil || code != 0 {
		return ""
	}
	return out.String()
}

func firstLineOf(s string) string {
	if before, _, ok := strings.Cut(s, "\n"); ok {
		return before
	}
	return strings.TrimSpace(s)
}

func showTail(path string) {
	b, err := os.ReadFile(path)
	if err != nil {
		return
	}
	lines := strings.Split(strings.TrimRight(string(b), "\n"), "\n")
	if len(lines) > 20 {
		lines = lines[len(lines)-20:]
	}
	for _, l := range lines {
		fmt.Fprintln(os.Stderr, l)
	}
}

func copyFile(src, dst string, mode os.FileMode) error {
	b, err := os.ReadFile(src)
	if err != nil {
		return err
	}
	return os.WriteFile(dst, b, mode)
}
