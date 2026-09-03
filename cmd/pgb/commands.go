// commands.go — subcommand dispatch.
//
// SPDX-License-Identifier: MIT
package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/bootstrapx"
	"github.com/polaris0xff/glibc-research/internal/buildx"
	"github.com/polaris0xff/glibc-research/internal/bundle"
	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/envx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/nixx"
	"github.com/polaris0xff/glibc-research/internal/ociimg"
	"github.com/polaris0xff/glibc-research/internal/rootfs"
	"github.com/polaris0xff/glibc-research/internal/selftest"
	"github.com/polaris0xff/glibc-research/internal/verifyx"
	"github.com/polaris0xff/glibc-research/internal/wrapper"
	"github.com/polaris0xff/glibc-research/internal/zstd"
)

func runCommand(c *cfg.Config, cmd string, args []string) error {
	switch cmd {
	case "", "help":
		fmt.Print(usage)
		return nil
	case "doctor":
		return doctor(c)
	case "explain":
		var b strings.Builder
		wrapper.Explain(c, &b)
		fmt.Print(b.String())
		return nil
	case "env":
		return envCommand(c, args)
	case "build":
		return buildx.Build(c, args)
	case "shell":
		return buildx.Shell(c)
	case "cc-dir":
		dir, err := buildx.CCDir(c)
		if err != nil {
			return err
		}
		logx.Say("%s", dir)
		return nil
	case "verify":
		if len(args) == 0 {
			return fail.Cannot("pgb verify NEEDS a binary")
		}
		return verifyx.Verify(c, args[0], args[1:])
	case "nix":
		return nixCommand(c, args)
	case "rootfs":
		return rootfsCommand(c, args)
	case "elf":
		return elfCommand(c, args)
	case "selftest":
		return selftestCommand(c, args)
	case "debug":
		return debugCommand(c, args)
	case "build-root":
		return buildRootCommand(args)
	case "bootstrap":
		return bootstrapCommand(c, args)
	case "bundle":
		return bundleCommand(c, args)
	case buildx.InnerBuild:
		return buildx.Inner(c, args, false)
	case buildx.InnerShell:
		return buildx.Inner(c, []string{shellOrDefault()}, true)
	case rootfs.InnerCommand():
		return rootfs.Inner(args)
	case envx.InnerLibiconv:
		return libiconvCommand(c, args)
	}
	return fail.Cannot("unknown command: %s (try: pgb help)", cmd)
}

func shellOrDefault() string {
	if s := os.Getenv("SHELL"); s != "" {
		return s
	}
	return "/bin/sh"
}

func envCommand(c *cfg.Config, args []string) error {
	sub := "info"
	if len(args) > 0 {
		sub = args[0]
	}
	switch sub {
	case "create":
		return envx.Create(c)
	case "info":
		return envx.Info(c)
	}
	return fail.Cannot("unknown: pgb env %s (create, info)", sub)
}

func libiconvCommand(c *cfg.Config, args []string) error {
	prefix := c.LibiconvPrefix
	version := envx.LibiconvVersion
	force := false
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--prefix":
			i++
			if i < len(args) {
				prefix = args[i]
			}
		case "--version":
			i++
			if i < len(args) {
				version = args[i]
			}
		case "--force":
			force = true
		}
	}
	return envx.BuildLibiconv(prefix, version, force)
}

func rootfsCommand(c *cfg.Config, args []string) error {
	if len(args) == 0 {
		return fail.Cannot("pgb rootfs needs a subcommand (run, fetch, pull, list)")
	}
	sub, rest := args[0], args[1:]
	// Each subcommand answers --selftest for the part of the bed it owns, so a
	// caller can probe one piece without running the whole set.
	if len(rest) == 1 && (rest[0] == "--selftest" || rest[0] == "selftest") {
		var r *selftest.Report
		switch sub {
		case "run":
			r = rootfs.Selftest()
		case "pull":
			r = ociimg.Selftest()
		default:
			return fail.Cannot("pgb rootfs %s has no selftest", sub)
		}
		if r.Print() != 0 {
			return fail.Exit(1)
		}
		return nil
	}
	switch sub {
	case "run":
		o, argv, err := rootfs.ParseOptions(rest)
		if err != nil {
			return err
		}
		if o.Root == "" {
			return fail.Cannot("pgb rootfs run needs a root filesystem")
		}
		if len(argv) == 0 {
			return fail.Cannot("pgb rootfs run needs a command (use -- CMD ...)")
		}
		o.Stdin = os.Stdin
		code, err := rootfs.Run(o, argv)
		if err != nil {
			return err
		}
		if code != 0 {
			return fail.Exit(code)
		}
		return nil
	case "fetch", "list":
		o, err := rootfs.ParseFetchArgs(rest, c.RootfsDir)
		if err != nil {
			return err
		}
		if sub == "list" {
			o.List = true
		}
		images, err := cfg.ReadImages(c.ImagesFile())
		if err != nil {
			return err
		}
		return rootfs.Fetch(images, o)
	case "pull":
		return pullCommand(rest)
	}
	return fail.Cannot("unknown: pgb rootfs %s (run, fetch, pull, list)", sub)
}

func pullCommand(args []string) error {
	var o ociimg.Options
	for i := 0; i < len(args); i++ {
		a := args[i]
		next := func() string {
			i++
			if i < len(args) {
				return args[i]
			}
			return ""
		}
		switch a {
		case "--out":
			o.Out = next()
		case "--arch":
			o.Arch = next()
		case "--digest":
			o.Digest = next()
		case "--quiet":
			o.Quiet = true
		default:
			if strings.HasPrefix(a, "-") {
				return fail.Cannot("pgb rootfs pull: unknown argument: %s", a)
			}
			o.Ref = a
		}
	}
	_, err := ociimg.Pull(o)
	return err
}

func elfCommand(c *cfg.Config, args []string) error {
	if len(args) == 0 {
		return fail.Cannot("pgb elf needs a subcommand (needed, info)")
	}
	sub, rest := args[0], args[1:]
	if len(rest) == 0 {
		return fail.Cannot("pgb elf %s needs a file", sub)
	}
	switch sub {
	case "needed":
		for _, p := range rest {
			if err := elfNeeded(p); err != nil {
				return err
			}
		}
		return nil
	case "print":
		// FILE<tab>NEEDED, one per line, so a caller can attribute an
		// unresolved entry to the file that wants it.
		for _, p := range rest {
			if err := elfPrint(p); err != nil {
				return err
			}
		}
		return nil
	case "info":
		return elfInfo(rest[0])
	case "shorten":
		return elfShorten(rest)
	}
	return fail.Cannot("unknown: pgb elf %s (needed, print, info, shorten)", sub)
}

// A suite is one carried selftest. Adding one is a line in selftestSuites and
// nothing else in the tree knows the set.
type suite struct {
	name string
	run  func() *selftest.Report
}

func selftestSuites(c *cfg.Config) []suite {
	return []suite{
		{"oci-pull", ociimg.Selftest},
		{"rootfs-run", rootfs.Selftest},
		{"elf", elfSelftest},
		{"cfg", cfg.Selftest},
		{"env-stamp", envx.StampSelftest},
		{"wrapper-flags", wrapper.FlagsSelftest},
		{"zstd", zstd.Selftest},
		{"nix-nar", func() *selftest.Report { return nixx.Selftest(nixFixtureDir(c)) }},
		{"nix-drv", nixx.DrvSelftest},
		{"nix-index", nixx.IndexSelftest},
		{"nix-diagnose", nixx.DiagnoseSelftest},
		{"bootstrap", func() *selftest.Report { return bootstrapx.Selftest(c) }},
		{"bundle-sweep", bundle.Selftest},
		{"bundle-hostpolicy", bundle.HostPolicySelftest},
		{"bundle-manifest-roots", bundle.ManifestSelftest},
		{"bundle-manifest-rewrite", bundle.ManifestRewriteSelftest},
		{"bundle-soname-scan", bundle.SonameScanSelftest},
		{"bundle-appimage", func() *selftest.Report { return bundle.AppImageSelftest(c) }},
	}
}

func selftestCommand(c *cfg.Config, args []string) error {
	all := selftestSuites(c)
	names := make([]string, len(all))
	known := make(map[string]bool, len(all))
	for i, s := range all {
		names[i], known[s.name] = s.name, true
	}

	only := map[string]bool{}
	for _, a := range args {
		if a == "--list" {
			for _, n := range names {
				logx.Say("%s", n)
			}
			return nil
		}
		// A name that matches nothing would select nothing and then report
		// "all pass", which is the most misleading answer this command has.
		if !known[a] {
			return fail.Cannot("no such selftest: %s (have: %s)", a, strings.Join(names, ", "))
		}
		only[a] = true
	}

	report := selftest.New("pgb")
	for _, s := range all {
		if len(only) == 0 || only[s.name] {
			report.Merge(s.run())
		}
	}
	// 0 all pass, 1 a case ran and failed, 2 something could not run here.
	if code := report.Print(); code != 0 {
		return fail.Exit(code)
	}
	return nil
}

func debugCommand(c *cfg.Config, args []string) error {
	logx.Say("log level      %s", logx.CurrentLevel())
	spec := logx.SubsysSpec()
	if spec == "" {
		spec = "(none selected: every subsystem emits at the configured level)"
	}
	logx.Say("subsystems     %s", spec)
	logx.Say("")
	logx.Say("select with --debug NAME[,NAME] or PGB_DEBUG; '-name' drops one.")
	logx.Say("subsystems seen so far in this process:")
	for _, s := range logx.KnownSubsystems() {
		logx.Say("  %s", s)
	}
	return nil
}

// bootstrapCommand prepares a fresh machine.
func bootstrapCommand(c *cfg.Config, args []string) error {
	o := bootstrapx.DefaultOptions()
	for _, a := range args {
		switch a {
		case "--check":
			o.Check = true
		case "--detach":
			o.Detach = true
		case "--wait":
			o.Detach = false
		case "--no-nix":
			o.Nix = false
		case "--no-bed":
			o.Bed = false
		case "--no-env":
			o.Env = false
		case "--no-docker":
			o.Docker = false
		case "--selftest":
			if bootstrapx.Selftest(c).Print() != 0 {
				return fail.Exit(1)
			}
			return nil
		default:
			return fail.Cannot("pgb bootstrap: unknown argument: %s", a)
		}
	}
	return bootstrapx.Run(c, o)
}
