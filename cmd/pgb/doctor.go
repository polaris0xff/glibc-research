// doctor.go — what this machine can and cannot do.
//
// SPDX-License-Identifier: MIT
package main

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"

	assets "github.com/polaris0xff/glibc-research"
	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/elfx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
	"github.com/polaris0xff/glibc-research/internal/rootfs"
)

func doctor(c *cfg.Config) error {
	ok := true
	logx.Say("pgb %s -- host report", cfg.Version)
	logx.Say("")
	logx.Say("  %-34s %s", "kernel", unameSR())
	logx.Say("  %-34s %s", "architecture", runtime.GOARCH)
	logx.Say("  %-34s %s", "distribution", prettyName())
	logx.Say("  %-34s %s", "host libc", firstLine(capture("ldd", "--version")))
	logx.Say("  %-34s %s", "cc", firstLine(capture(compilerName(), "--version")))
	logx.Say("")

	check := func(label string, good bool, advice string) {
		if good {
			logx.Say("  ok    %-32s", label)
			return
		}
		logx.Say("  MISS  %-32s %s", label, advice)
		ok = false
	}

	check("a C compiler", proc.Look(compilerName()), "install gcc or clang")
	check("static linking", canLinkStatic(), "install libc6-dev / glibc-static")
	check("__nss_configure_lookup in libc.a", hasNSSConfigureLookup(), "the NSS fix needs a glibc libc.a")

	// The rows above describe THIS machine, which is only the right answer for
	// the host engine: libiconv is built inside the build environment by the
	// environment's own compiler.
	engine := c.Engine()
	switch engine {
	case cfg.EngineChroot:
		root := c.EnvRoot()
		if fi, err := os.Stat(root); err == nil && fi.IsDir() {
			check("GNU libiconv (static), in the chroot environment",
				exists(filepath.Join(root, c.LibiconvPrefix, "lib", "libiconv.a")),
				"run: pgb env create")
		} else {
			logx.Say("  --    %-32s %s", "GNU libiconv (static)", "no chroot environment yet")
		}
	case cfg.EngineDocker, cfg.EnginePodman:
		logx.Say("  --    %-32s inside the %s image; pgb build checks it",
			"GNU libiconv (static)", engine)
	default:
		check("GNU libiconv (static), on this machine",
			exists(filepath.Join(c.LibiconvPrefix, "lib", "libiconv.a")),
			"run: pgb env create --engine host")
	}

	logx.Say("")
	logx.Say("  build environment engines:")
	for _, e := range []string{"docker", "podman"} {
		switch {
		case proc.Look(e) && infoOK(e):
			logx.Say("    ok    %-10s usable", e)
		case proc.Look(e):
			logx.Say("    --    %-10s present but no daemon", e)
		default:
			logx.Say("    --    %-10s absent", e)
		}
	}
	if os.Geteuid() == 0 && proc.Look("unshare") {
		logx.Say("    ok    %-10s usable (root + CAP_SYS_ADMIN)", "chroot")
	} else {
		logx.Say("    --    %-10s needs root and CAP_SYS_ADMIN", "chroot")
	}
	logx.Say("    ok    %-10s always available, but see the warning below", "host")
	logx.Say("")
	logx.Say("  chosen engine: %s", engine)
	if engine == cfg.EngineHost {
		logx.Say("")
		logx.Say("  THE host ENGINE BUILDS AGAINST WHATEVER GLIBC THIS MACHINE HAS.")
		logx.Say("    That is not a controlled environment: the binary inherits this")
		logx.Say("    host's glibc version, its headers and its CPU defaults. Use it to")
		logx.Say("    experiment, not to ship. `pgb env create` gives the pinned one.")
	}

	logx.Say("")
	logx.Say("  target root filesystems for `pgb verify`:")
	images, err := cfg.ReadImages(c.ImagesFile())
	if err != nil {
		images = nil
		logx.Say("    (image list unreadable: %v)", err)
	}
	if images != nil {
		n := rootfs.Present(images, c.RootfsDir)
		logx.Say("    %d present under %s", n, c.RootfsDir)
		if n == 0 {
			logx.Say("    run: pgb rootfs fetch")
		}
	}

	logx.Say("")
	logx.Say("  carried in this binary:")
	logx.SayRaw(assets.EmbeddedManifest())

	if !ok {
		return fail.Exit(1)
	}
	return nil
}

func compilerName() string {
	if cc := os.Getenv("CC"); cc != "" {
		return cc
	}
	return "cc"
}

func canLinkStatic() bool {
	dir, err := os.MkdirTemp("", "pgb-doctor-")
	if err != nil {
		return false
	}
	defer os.RemoveAll(dir)
	src := filepath.Join(dir, "probe.c")
	if err := os.WriteFile(src, []byte("int main(void){return 0;}\n"), 0o644); err != nil {
		return false
	}
	r, err := proc.Quiet(compilerName(), "-static", "-o", filepath.Join(dir, "probe"), src)
	return err == nil && !r.Failed()
}

// hasNSSConfigureLookup checks the machine's own libc.a for the public symbol
// the NSS mechanism calls.
func hasNSSConfigureLookup() bool {
	path, err := proc.Capture(compilerName(), "-print-file-name=libc.a")
	if err != nil || path == "" || !filepath.IsAbs(path) {
		return false
	}
	syms, err := elfx.DefinedExternalSymbols(path)
	if err != nil {
		return false
	}
	for _, s := range syms {
		if s == "__nss_configure_lookup" {
			return true
		}
	}
	return false
}

func infoOK(engine string) bool {
	r, err := proc.Quiet(engine, "info")
	return err == nil && !r.Failed()
}

func exists(p string) bool { _, err := os.Stat(p); return err == nil }

func capture(argv ...string) string {
	out, _ := proc.CaptureAllowFail(argv...)
	return out
}

func firstLine(s string) string {
	if i := strings.IndexByte(s, '\n'); i >= 0 {
		return s[:i]
	}
	if s == "" {
		return "none"
	}
	return s
}

func unameSR() string {
	out := capture("uname", "-sr")
	if out == "" {
		return runtime.GOOS
	}
	return out
}

func prettyName() string {
	b, err := os.ReadFile("/etc/os-release")
	if err != nil {
		return "unknown"
	}
	for _, line := range strings.Split(string(b), "\n") {
		if v, ok := strings.CutPrefix(line, "PRETTY_NAME="); ok {
			return strings.Trim(v, `"`)
		}
	}
	return "unknown"
}
