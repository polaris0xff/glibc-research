// bundle.go — `pgb bundle`, the bundler and its reachability sweep.
//
// SPDX-License-Identifier: MIT
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/bundle"
	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
)

func bundleCommand(c *cfg.Config, args []string) error {
	if len(args) == 0 {
		return fail.Cannot("pgb bundle needs a subcommand (appimage, sweep, fold-env, onelf-recipe)")
	}
	sub, rest := args[0], args[1:]
	switch sub {
	case "appimage":
		if len(rest) == 1 && (rest[0] == "--selftest" || rest[0] == "selftest") {
			if bundle.AppImageSelftest(c).Print() != 0 {
				return fail.Exit(1)
			}
			return nil
		}
		return bundleAppImage(c, rest)
	case "fold-env":
		if len(rest) == 0 {
			return fail.Cannot("pgb bundle fold-env needs a .env file")
		}
		keys, before, after, err := bundle.FoldEnv(rest[0])
		if err != nil {
			return fail.Ran("%v", err)
		}
		fmt.Printf("env-fold: %d keys, %d -> %d bytes\n", keys, before, after)
		return nil
	case "onelf-recipe":
		if len(rest) < 2 {
			return fail.Cannot("pgb bundle onelf-recipe needs APPDIR and the main program")
		}
		level := 19
		var pos []string
		for i := 0; i < len(rest); i++ {
			if rest[i] == "--level" && i+1 < len(rest) {
				i++
				n, err := strconv.Atoi(rest[i])
				if err != nil {
					return fail.Cannot("--level wants a number, got %q", rest[i])
				}
				level = n
				continue
			}
			pos = append(pos, rest[i])
		}
		if len(pos) < 2 {
			return fail.Cannot("pgb bundle onelf-recipe needs APPDIR and the main program")
		}
		return bundle.WriteOnelfRecipe(os.Stdout, pos[0], pos[1], level)
	case "sweep":
		if len(rest) == 1 && (rest[0] == "--selftest" || rest[0] == "selftest") {
			if bundle.Selftest().Print() != 0 {
				return fail.Exit(1)
			}
			return nil
		}
		return bundleSweep(rest)
	case "manifests":
		if len(rest) != 1 {
			return fail.Cannot("pgb bundle manifests needs one APPDIR")
		}
		return bundleManifests(rest[0])
	}
	return fail.Cannot("unknown: pgb bundle %s (appimage, sweep, manifests, fold-env, onelf-recipe)", sub)
}

// bundleManifests asserts the bundle's DATA is coherent: every vendor and ICD
// manifest names a library that is in the bundle, by a name the loader can
// resolve there.
//
// ⛔ IT EXITS NON-ZERO, WHICH IS WHY IT EXISTS SEPARATELY FROM THE BUILD. The
// build reports the same finding and carries on, because a closure can
// legitimately carry a manifest for a vendor it did not bundle. An EXPERIMENT
// needs the other thing: a verdict it can fail on, with a negative control
// that a deliberately un-rewritten manifest is caught. `TODO` T-071's Prove.
func bundleManifests(appDir string) error {
	n, outside, absent := bundle.CheckManifests(appDir)
	fmt.Printf("manifests read: %d\n", n)
	for _, s := range outside {
		fmt.Printf("OUTSIDE  %s\n", s)
	}
	for _, s := range absent {
		fmt.Printf("ABSENT   %s\n", s)
	}
	if n == 0 {
		// ⚠ Not a pass. A bundle with no manifests at all has not been
		// checked, and saying "ok" here is the quiet no-op docs/AGENTS.md §0b
		// calls the worst answer this codebase can give.
		return fail.Cannot("no manifest was found under %s, so nothing was checked", appDir)
	}
	if len(outside) > 0 || len(absent) > 0 {
		return fail.Exit(1)
	}
	fmt.Printf("VERDICT: every manifest names a library present in the bundle.\n")
	return nil
}

// bundleSweep reports which shared objects in a bundle nothing can reach.
func bundleSweep(args []string) error {
	var o bundle.SweepOptions
	listAll := false
	for i := 0; i < len(args); i++ {
		a := args[i]
		next := func() (string, error) {
			if i+1 >= len(args) {
				return "", fail.Cannot("pgb bundle sweep: %s needs a value", a)
			}
			i++
			return args[i], nil
		}
		var v string
		var err error
		switch a {
		case "--program":
			if v, err = next(); err == nil {
				o.ExtraRoots = append(o.ExtraRoots, v)
			}
		case "--env":
			if v, err = next(); err == nil {
				o.EnvFiles = append(o.EnvFiles, v)
			}
		case "--lib-dir":
			if v, err = next(); err == nil {
				o.LibDirs = append(o.LibDirs, v)
			}
		case "--program-dir":
			if v, err = next(); err == nil {
				o.ProgramDirs = append(o.ProgramDirs, v)
			}
		case "--cut":
			// ⭐ TODO T-066 route A: treat a DT_NEEDED edge as absent and
			// report what becomes unreachable without it. The delta against
			// the uncut sweep is the size of the subtree reachable ONLY
			// through that edge -- the bytes an allowlist cannot reach,
			// because only a rebuild removes a declared dependency.
			if v, err = next(); err == nil {
				o.CutEdges = append(o.CutEdges, v)
			}
		case "--fixpoint":
			// ⭐ TODO T-066 route A, lever B3: count soname mentions only from
			// objects that are themselves reachable, iterated to a fixpoint.
			// ⛔ A MEASURING DEVICE, not a policy: nothing in the build path
			// sets this, because it makes the sweep delete more and
			// `experiments/89-` is the control that would have to pass first.
			o.Fixpoint = true
		case "--list":
			listAll = true
		default:
			if strings.HasPrefix(a, "-") {
				return fail.Cannot("pgb bundle sweep: unknown argument: %s", a)
			}
			o.Dir = a
		}
		if err != nil {
			return err
		}
	}
	if o.Dir == "" {
		return fail.Cannot("pgb bundle sweep needs a bundle directory")
	}
	// A bundle's own environment file is read by default when it is there: a
	// directory an environment variable names is a plugin directory even when
	// its contents do not look like plugins.
	if len(o.EnvFiles) == 0 {
		for _, cand := range []string{".env", "AppRun.env", "shared/.env"} {
			o.EnvFiles = append(o.EnvFiles, filepath.Join(o.Dir, cand))
		}
	}
	res, err := bundle.Sweep(o)
	if err != nil {
		return err
	}
	var b strings.Builder
	res.Report(&b, listAll)
	fmt.Print(b.String())
	return nil
}

// bundleAppImage packs a nixpkgs closure the Anylinux way.
func bundleAppImage(c *cfg.Config, args []string) error {
	var o bundle.AppImageOptions
	for i := 0; i < len(args); i++ {
		a := args[i]
		next := func() (string, error) {
			if i+1 >= len(args) {
				return "", fail.Cannot("pgb bundle appimage: %s needs a value", a)
			}
			i++
			return args[i], nil
		}
		var v string
		var err error
		switch a {
		case "--out":
			if v, err = next(); err == nil {
				o.Out = v
			}
		case "--name":
			if v, err = next(); err == nil {
				o.Name = v
			}
		case "--debloat":
			if v, err = next(); err == nil {
				o.Debloat = v
			}
		case "--keep-locales":
			if v, err = next(); err == nil {
				o.KeepLocales = append(o.KeepLocales, strings.Split(v, ",")...)
			}
		case "--with-program":
			if v, err = next(); err == nil {
				o.WithPrograms = append(o.WithPrograms, v)
			}
		case "--extra":
			if v, err = next(); err == nil {
				o.Extra = append(o.Extra, v)
			}
		case "--cache":
			if v, err = next(); err == nil {
				o.Cache = v
			}
		case "--no-gl":
			o.NoGL = true
		case "--no-storefix":
			// ⭐ The negative control for T-081: the same bundle without the
			// mechanism that resolves a compiled-in /nix/store path.
			o.NoStorefix = true
		case "--keep":
			// accepted: the AppDir is always kept
		default:
			if strings.HasPrefix(a, "-") {
				return fail.Cannot("pgb bundle appimage: unknown argument: %s", a)
			}
			o.Target = a
		}
		if err != nil {
			return err
		}
	}
	if o.Target == "" {
		return fail.Cannot("pgb bundle appimage needs a nixpkgs attribute or a store path")
	}
	switch o.Debloat {
	case "", "none", "safe", "aggressive":
	default:
		return fail.Cannot("--debloat wants none, safe or aggressive; got %q", o.Debloat)
	}
	return bundle.NewAppImageBuilder(c, o).Build()
}
