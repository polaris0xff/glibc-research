// bundle.go — `pgb bundle`, the bundler and its reachability sweep.
//
// SPDX-License-Identifier: MIT
package main

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/bundle"
	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
)

func bundleCommand(c *cfg.Config, args []string) error {
	if len(args) == 0 {
		return fail.Cannot("pgb bundle needs a subcommand (sweep)")
	}
	sub, rest := args[0], args[1:]
	switch sub {
	case "sweep":
		if len(rest) == 1 && (rest[0] == "--selftest" || rest[0] == "selftest") {
			if bundle.Selftest().Print() != 0 {
				return fail.Exit(1)
			}
			return nil
		}
		return bundleSweep(rest)
	}
	return fail.Cannot("unknown: pgb bundle %s (sweep)", sub)
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
