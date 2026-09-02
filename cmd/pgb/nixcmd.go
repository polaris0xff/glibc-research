// nixcmd.go — `pgb nix plan|deps|build|fetch` and `pgb nix cache ...`.
//
// SPDX-License-Identifier: MIT
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/nixx"
)

const nixUsage = `pgb nix -- use nixpkgs as the dependency planner, and build with glibc

  pgb nix plan  ATTR [--out FILE]
        Resolve a nixpkgs attribute and write a build plan: the source, its
        upstream URL and hash, the patches, the configure flags, the
        dependency derivations. Two nix-free routes are tried first.

  pgb nix deps  ATTR|--plan FILE
        Build the plan's DEPENDENCIES into the shared static prefix and stop.
        For a package whose own build is driven by hand, the dependency
        closure is still the expensive part and nixpkgs still knows it.
        $NIX_PREFIX selects the prefix.

  pgb nix fetch ATTR|--plan FILE [--out DIR]
        Fetch the plan's source and patches. cache.nixos.org first (signed,
        hash-checked, no nix needed), the upstream URL second.

  pgb nix build ATTR|--plan FILE [--out DIR] [--configure FLAGS] [--no-deps]
        Fetch, unpack, patch and build through pgb's static-glibc toolchain.

  pgb nix cache SUBCOMMAND
        The binary-cache client on its own:
          channel                    the channel's release URL and revision
          resolve REGEX [--limit N]  grep the channel's store-path index
          attr    NAME               resolve a name to an attribute row
          drv     ATTR               the derivation, through hydra
          info    PATH|HASH          the verified narinfo
          closure PATH               the transitive references
          fetch   PATH --out DIR     fetch and verify, with the closure

  pgb nix nar|drv|index|hydra|plan
        The formats underneath: NAR, ATerm derivations, the package index.

nixpkgs' own pkgsStatic is MUSL on Linux. This builds the glibc half.
`

type nixArgs struct {
	Attr      string
	PlanFile  string
	Out       string
	Configure []string
	NoDeps    bool
	Limit     int
	System    string
	Channel   string
	Jobset    string
}

func parseNixArgs(args []string) (nixArgs, error) {
	a := nixArgs{Limit: 40}
	for i := 0; i < len(args); i++ {
		flag := args[i]
		value := func() (string, error) {
			if i+1 >= len(args) {
				return "", fail.Cannot("pgb nix: %s needs a value", flag)
			}
			i++
			return args[i], nil
		}
		var v string
		var err error
		switch flag {
		case "--plan":
			if v, err = value(); err == nil {
				a.PlanFile = v
			}
		case "--out":
			if v, err = value(); err == nil {
				a.Out = v
			}
		case "--configure":
			if v, err = value(); err == nil {
				a.Configure = append(a.Configure, strings.Fields(v)...)
			}
		case "--limit":
			if v, err = value(); err == nil {
				n, cerr := strconv.Atoi(v)
				if cerr != nil {
					return a, fail.Cannot("pgb nix: --limit wants a number, got %q", v)
				}
				a.Limit = n
			}
		case "--system":
			if v, err = value(); err == nil {
				a.System = v
			}
		case "--channel":
			if v, err = value(); err == nil {
				a.Channel = v
			}
		case "--jobset":
			if v, err = value(); err == nil {
				a.Jobset = v
			}
		case "--no-deps":
			a.NoDeps = true
		case "--with-deps":
			a.NoDeps = false
		case "--keep", "--try-nix":
			// accepted and ignored: kept so existing call sites still parse
		default:
			if strings.HasPrefix(flag, "-") {
				return a, fail.Cannot("pgb nix: unknown option %s", flag)
			}
			a.Attr = flag
		}
		if err != nil {
			return a, err
		}
	}
	return a, nil
}

func (a nixArgs) applyTo(b *nixx.Builder) {
	if a.System != "" {
		b.System = a.System
		b.Client.System = a.System
	}
	if a.Channel != "" {
		b.Client.Channel = a.Channel
	}
	if a.Jobset != "" {
		b.Client.Jobset = a.Jobset
	}
	b.ConfigureExtra = a.Configure
	b.WithDeps = !a.NoDeps
}

func nixPlanCommand(c *cfg.Config, args []string) error {
	a, err := parseNixArgs(args)
	if err != nil {
		return err
	}
	if a.Attr == "" {
		return fail.Cannot("pgb nix plan needs an attribute, e.g. pgb nix plan bash")
	}
	b := nixx.NewBuilder(c)
	a.applyTo(b)
	_, err = b.Plan(a.Attr, a.Out)
	return err
}

func nixFetchCommand(c *cfg.Config, args []string) error {
	a, err := parseNixArgs(args)
	if err != nil {
		return err
	}
	b := nixx.NewBuilder(c)
	a.applyTo(b)
	_, plan, err := b.ResolvePlan(a.Attr, a.PlanFile)
	if err != nil {
		return err
	}
	dest := a.Out
	if dest == "" {
		dest = filepath.Join(c.State, "nix-src")
	}
	src, patches, err := b.FetchPlan(plan, dest)
	if err != nil {
		return err
	}
	logx.Say("%s", src)
	for _, p := range patches {
		logx.Say("%s", p)
	}
	return nil
}

func nixDepsCommand(c *cfg.Config, args []string) error {
	a, err := parseNixArgs(args)
	if err != nil {
		return err
	}
	b := nixx.NewBuilder(c)
	a.applyTo(b)
	_, plan, err := b.ResolvePlan(a.Attr, a.PlanFile)
	if err != nil {
		return err
	}
	logx.Say("attr        %s", plan.Attr)
	logx.Say("prefix      %s", b.Prefix)
	if err := b.BuildDeps(plan, 1); err != nil {
		return err
	}
	logx.Say("")
	logx.Say("built into  %s", b.Prefix)
	if entries, err := os.ReadDir(filepath.Join(b.Prefix, ".built")); err == nil {
		for _, e := range entries {
			logx.Say("  ok    %s", e.Name())
		}
	}
	return nil
}

func nixBuildCommand(c *cfg.Config, args []string) error {
	a, err := parseNixArgs(args)
	if err != nil {
		return err
	}
	b := nixx.NewBuilder(c)
	a.applyTo(b)
	_, plan, err := b.ResolvePlan(a.Attr, a.PlanFile)
	if err != nil {
		return err
	}
	work := a.Out
	if work == "" {
		work = filepath.Join(c.State, "nix-build", plan.Pname+"-"+plan.Version)
	}
	for _, d := range []string{"dl", "build", "out"} {
		if err := os.MkdirAll(filepath.Join(work, d), 0o755); err != nil {
			return err
		}
	}
	logx.Say("attr        %s", plan.Attr)
	logx.Say("package     %s %s", plan.Pname, plan.Version)
	logx.Say("nixpkgs     %s", plan.Nixpkgs)
	logx.Say("work        %s", work)

	if b.WithDeps {
		if err := b.BuildDeps(plan, 1); err != nil {
			return err
		}
	}
	src, patches, err := b.FetchPlan(plan, filepath.Join(work, "dl"))
	if err != nil {
		return err
	}
	logx.Say("source      %s", src)
	top, err := b.PrepareSource(plan, src, patches, work)
	if err != nil {
		return err
	}
	logx.Say("unpacked    %s", top)
	if err := b.BuildTree(top, plan, work, false); err != nil {
		return err
	}
	n, err := nixx.Collect(top, filepath.Join(work, "out"))
	if err != nil {
		return err
	}
	logx.Say("")
	logx.Say("built into  %s  (%d executable(s))", filepath.Join(work, "out"), n)
	if entries, err := os.ReadDir(filepath.Join(work, "out")); err == nil {
		for _, e := range entries {
			if fi, err := e.Info(); err == nil {
				logx.Say("  %-30s %d bytes", e.Name(), fi.Size())
			}
		}
	}
	return nil
}

func nixCacheCommand(c *cfg.Config, args []string) error {
	if len(args) == 0 {
		return fail.Cannot("pgb nix cache needs a subcommand " +
			"(channel, resolve, attr, drv, info, closure, fetch)")
	}
	sub, rest := args[0], args[1:]
	a, err := parseNixArgs(rest)
	if err != nil {
		return err
	}
	b := nixx.NewBuilder(c)
	a.applyTo(b)
	cl := b.Client

	switch sub {
	case "channel":
		url, err := cl.ReleaseURL()
		if err != nil {
			return err
		}
		logx.Say("channel   %s", cl.Channel)
		logx.Say("release   %s", url)
		logx.Say("revision  %s", nixx.Revision(url))
		return nil
	case "resolve":
		if a.Attr == "" {
			return fail.Cannot("resolve needs a pattern")
		}
		hits, err := cl.Resolve(a.Attr, a.Limit)
		if err != nil {
			return err
		}
		for _, h := range hits {
			logx.Say("%s", h)
		}
		return nil
	case "attr":
		if a.Attr == "" {
			return fail.Cannot("attr needs an attribute path, e.g. jq")
		}
		m, err := cl.Attr(a.Attr)
		if err != nil {
			return err
		}
		logx.Say("Attr: %s", m.Attr)
		logx.Say("Name: %s", m.Name)
		logx.Say("Pname: %s", m.Pname)
		logx.Say("Version: %s", m.Version)
		logx.Say("System: %s", m.System)
		logx.Say("OutputName: %s", m.OutputName)
		logx.Say("Outputs: %s", strings.Join(m.Outputs, ","))
		logx.Say("Matched: %s", m.MatchedBy)
		logx.Say("Candidates: %d", m.Candidates)
		return nil
	case "drv":
		if a.Attr == "" {
			return fail.Cannot("drv needs an attribute path, e.g. jq")
		}
		return nixCacheDrv(cl, a.Attr)
	case "info":
		if a.Attr == "" {
			return fail.Cannot("info needs a store path or hash")
		}
		info, err := cl.Narinfo(a.Attr)
		if err != nil {
			return err
		}
		for _, k := range []string{"StorePath", "URL", "Compression", "FileHash",
			"FileSize", "NarHash", "NarSize", "References", "Deriver", "Sig"} {
			if v, ok := info[k]; ok {
				logx.Say("%s: %s", k, v)
			}
		}
		return nil
	case "closure":
		if a.Attr == "" {
			return fail.Cannot("closure needs a store path")
		}
		paths, err := cl.Closure(a.Attr)
		if err != nil {
			return err
		}
		for _, p := range paths {
			logx.Say("%s", p)
		}
		return nil
	case "fetch":
		if a.Attr == "" {
			return fail.Cannot("fetch needs a store path")
		}
		if a.Out == "" {
			return fail.Cannot("fetch needs --out DIR")
		}
		written, err := cl.Fetch(a.Attr, nixx.FetchOptions{Out: a.Out, WithClosure: true})
		if err != nil {
			return err
		}
		for _, p := range written {
			logx.Say("%s", p)
		}
		return nil
	}
	return fail.Cannot("unknown: pgb nix cache %s", sub)
}

func nixCacheDrv(cl *nixx.Client, want string) error {
	attr := want
	if m, err := cl.Attr(want); err == nil {
		if m.Attr != want {
			logx.Warnf("%s -> attribute %s (matched by %s)", want, m.Attr, m.MatchedBy)
		}
		attr = m.Attr
	}
	jobFile, err := cl.HydraJob(attr)
	if err != nil {
		return err
	}
	return nixx.ReadHydra(jobFile, cl.System)
}

// buildRootCommand prints the directory holding the first of the named build
// files, taking the shallowest match so a contrib or example copy cannot win.
// The build file is not always at the top of a tarball: zstd keeps its
// CMakeLists.txt in build/cmake, libblake3 in c/, icu4c its configure in
// icu4c/source.
func buildRootCommand(args []string) error {
	if len(args) == 0 {
		return fail.Cannot("pgb build-root needs at least one file name")
	}
	for _, want := range args {
		if _, err := os.Stat(want); err == nil {
			fmt.Println(".")
			return nil
		}
	}
	skip := map[string]bool{
		"_pgbbuild": true, "demos": true, "demo": true, "examples": true,
		"example": true, "test": true, "tests": true, "doc": true, "docs": true,
		"contrib": true, "fuzz": true, "samples": true, "benchmarks": true,
	}
	best, bestDepth := "", 1<<30
	for _, want := range args {
		_ = filepath.Walk(".", func(p string, fi os.FileInfo, err error) error {
			if err != nil {
				return nil
			}
			depth := strings.Count(p, string(os.PathSeparator))
			if fi.IsDir() {
				if depth >= 4 {
					return filepath.SkipDir
				}
				if skip[fi.Name()] {
					return filepath.SkipDir
				}
				return nil
			}
			if fi.Name() != want || depth < 1 {
				return nil
			}
			if depth < bestDepth {
				best, bestDepth = filepath.Dir(p), depth
			}
			return nil
		})
		if best != "" {
			fmt.Fprintf(os.Stderr, "pgb: build root is %s (found %s there, not at the top)\n", best, want)
			fmt.Println(best)
			return nil
		}
	}
	fmt.Println(".")
	return nil
}
