// nix.go — the `pgb nix` subcommands that use nixpkgs as the planner.
//
// SPDX-License-Identifier: MIT
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/nixx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

func nixCommand(c *cfg.Config, args []string) error {
	if len(args) == 0 {
		return fail.Cannot("pgb nix needs a subcommand (nar, index, hydra, drv, plan)")
	}
	sub, rest := args[0], args[1:]
	switch sub {
	case "nar":
		return narCommand(c, rest)
	case "index":
		if len(rest) < 2 {
			return fail.Cannot("pgb nix index needs PACKAGES_JSON and OUT_TSV")
		}
		return nixx.BuildIndex(rest[0], rest[1])
	case "hydra":
		system := ""
		var files []string
		for i := 0; i < len(rest); i++ {
			if rest[i] == "--system" {
				i++
				if i < len(rest) {
					system = rest[i]
				}
				continue
			}
			files = append(files, rest[i])
		}
		if len(files) == 0 {
			return fail.Cannot("pgb nix hydra needs a JSON file")
		}
		return nixx.ReadHydra(files[0], system)
	case "drv":
		if len(rest) == 0 {
			return fail.Cannot("pgb nix drv needs a .drv file")
		}
		return drvCommand(rest)
	case "plan":
		return nixPlanCommand(c, rest)
	case "plan-doc":
		// The planner on its own: a `nix derivation show --recursive` document
		// on stdin, one plan on stdout.
		return planCommand(rest)
	case "deps":
		return nixDepsCommand(c, rest)
	case "build":
		return nixBuildCommand(c, rest)
	case "fetch":
		return nixFetchCommand(c, rest)
	case "cache":
		return nixCacheCommand(c, rest)
	case "", "help", "-h", "--help":
		fmt.Print(nixUsage)
		return nil
	}
	return fail.Cannot("unknown: pgb nix %s (plan, deps, build, fetch, cache, nar, drv, index, hydra, plan-doc)", sub)
}

func narCommand(c *cfg.Config, args []string) error {
	if len(args) == 0 {
		return fail.Cannot("pgb nix nar needs a subcommand (extract, dump, hash, verify-narinfo)")
	}
	sub, rest := args[0], args[1:]
	switch sub {
	case "extract":
		if len(rest) == 0 {
			return fail.Cannot("pgb nix nar extract needs a destination directory")
		}
		if err := nixx.NarExtract(os.Stdin, rest[0]); err != nil {
			return fail.Ran("%v", err)
		}
		return nil
	case "dump":
		if len(rest) == 0 {
			return fail.Cannot("pgb nix nar dump needs a source directory")
		}
		if err := nixx.NarDump(rest[0], os.Stdout); err != nil {
			return fail.Ran("%v", err)
		}
		return nil
	case "hash":
		h, n, err := nixx.NarHash(os.Stdin)
		if err != nil {
			return fail.Ran("%v", err)
		}
		logx.Say("%s", h)
		logx.Infof("%d bytes", n)
		return nil
	case "verify-narinfo":
		keys := nixx.DefaultKeys()
		for _, spec := range rest {
			name, key, err := nixx.ParseKeySpec(spec)
			if err != nil {
				return fail.Cannot("%v", err)
			}
			keys[name] = key
		}
		b, err := readAllStdin()
		if err != nil {
			return fail.Cannot("%v", err)
		}
		info := nixx.ParseNarinfo(string(b))
		ok, why := info.Verify(keys)
		if !ok {
			return fail.Ran("narinfo NOT verified: %s", why)
		}
		logx.Say("verified by %s", why)
		return nil
	}
	return fail.Cannot("unknown: pgb nix nar %s", sub)
}

// planCommand reads a `nix derivation show --recursive` document on stdin and
// writes one build plan.
func planCommand(args []string) error {
	var positional []string
	nixPrefix, nixpkgsVersion := "", ""
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--nix-prefix":
			i++
			if i < len(args) {
				nixPrefix = args[i]
			}
		case "--nixpkgs-version":
			i++
			if i < len(args) {
				nixpkgsVersion = args[i]
			}
		default:
			if strings.HasPrefix(args[i], "-") {
				return fail.Cannot("pgb nix plan: unknown argument: %s", args[i])
			}
			positional = append(positional, args[i])
		}
	}
	attr, drvPath := "?", ""
	if len(positional) > 0 {
		attr = positional[0]
	}
	if len(positional) > 1 {
		drvPath = positional[1]
	}
	if nixPrefix != "" && nixpkgsVersion == "" {
		out, code := proc.CaptureAllowFail(filepath.Join(nixPrefix, "nix-instantiate"),
			"--eval", "--expr", "(import <nixpkgs> {}).lib.version")
		if code == 0 {
			nixpkgsVersion = strings.Trim(strings.TrimSpace(out), `"`)
		}
	}
	var doc nixx.ShowRecursive
	if err := json.NewDecoder(os.Stdin).Decode(&doc); err != nil {
		return fail.Ran("the derivation document is not JSON: %v", err)
	}
	plan, err := nixx.BuildPlan(doc, attr, drvPath, nixpkgsVersion, nixPrefix)
	if err != nil {
		return fail.Ran("%v", err)
	}
	return nixx.WritePlan(os.Stdout, plan)
}

func drvCommand(args []string) error {
	format := "text"
	var files []string
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--json":
			format = "json"
		case "--show":
			format = "show"
		default:
			if strings.HasPrefix(args[i], "-") {
				return fail.Cannot("pgb nix drv: unknown argument: %s", args[i])
			}
			files = append(files, args[i])
		}
	}
	if len(files) == 0 {
		return fail.Cannot("pgb nix drv needs a .drv file")
	}
	if format == "show" {
		doc, err := nixx.ShowFiles(files)
		if err != nil {
			return fail.Ran("%v", err)
		}
		enc := json.NewEncoder(os.Stdout)
		enc.SetIndent("", " ")
		return enc.Encode(doc)
	}
	d, err := nixx.ParseDrvFile(files[0])
	if err != nil {
		return fail.Ran("%v", err)
	}
	if format == "json" {
		return d.WriteJSON(os.Stdout)
	}
	d.WriteText(os.Stdout)
	return nil
}

func readAllStdin() ([]byte, error) {
	var chunks []byte
	buf := make([]byte, 64*1024)
	for {
		n, err := os.Stdin.Read(buf)
		chunks = append(chunks, buf[:n]...)
		if err != nil {
			if err.Error() == "EOF" {
				return chunks, nil
			}
			return chunks, err
		}
		if n == 0 {
			return chunks, nil
		}
	}
}

// nixFixtureDir is where the committed narinfo fixtures live.
func nixFixtureDir(c *cfg.Config) string {
	return filepath.Join(c.Self, "scripts", "common", "fixtures", "nix")
}
