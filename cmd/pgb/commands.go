// commands.go — subcommand dispatch.
//
// SPDX-License-Identifier: MIT
package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/buildx"
	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/envx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/ociimg"
	"github.com/polaris0xff/glibc-research/internal/rootfs"
	"github.com/polaris0xff/glibc-research/internal/selftest"
	"github.com/polaris0xff/glibc-research/internal/verifyx"
	"github.com/polaris0xff/glibc-research/internal/wrapper"
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
	case "rootfs":
		return rootfsCommand(c, args)
	case "elf":
		return elfCommand(c, args)
	case "selftest":
		return selftestCommand(c, args)
	case "debug":
		return debugCommand(c, args)
	case buildx.InnerBuild:
		return buildx.Inner(c, args)
	case buildx.InnerShell:
		return buildx.Inner(c, []string{shellOrDefault()})
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
		return elfNeeded(rest[0])
	case "info":
		return elfInfo(rest[0])
	}
	return fail.Cannot("unknown: pgb elf %s (needed, info)", sub)
}

func selftestCommand(c *cfg.Config, args []string) error {
	only := map[string]bool{}
	for _, a := range args {
		only[a] = true
	}
	want := func(name string) bool { return len(only) == 0 || only[name] }

	all := selftest.New("pgb")
	if want("oci-pull") {
		all.Merge(ociimg.Selftest())
	}
	if want("rootfs-run") {
		all.Merge(rootfs.Selftest())
	}
	if want("elf") {
		all.Merge(elfSelftest())
	}
	if all.Print() != 0 {
		return fail.Exit(1)
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
