// dispatch.go — the wrapper itself: pgb invoked as cc, gcc, c++, g++ or cpp.
//
// It reads the manifest written when the wrapper directory was created and
// execs the real compiler. No shell is started and no flags are recomputed, so
// a build system calling it thousands of times pays one file read and one
// exec.
//
// SPDX-License-Identifier: MIT
package wrapper

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
)

// mode is what a command line turns out to be.
type mode int

const (
	modeLink mode = iota
	modeCompile
	modeShared
	modeQuery // -print-*, --version and friends: never decorated
)

// classify decides what an argv means. A query anywhere on the line wins,
// because configure, libtool and CMake parse the output of those and a flag
// pgb added would change what they read.
func classify(args []string) mode {
	m := modeLink
	for _, a := range args {
		switch a {
		case "-c", "-E", "-S", "-M", "-MM":
			if m == modeLink {
				m = modeCompile
			}
			continue
		case "-shared", "-dynamiclib":
			m = modeShared
			continue
		case "-dumpmachine", "-dumpversion", "-dumpspecs", "--version", "-V":
			return modeQuery
		}
		if strings.HasPrefix(a, "-print-") || strings.HasPrefix(a, "--print-") {
			return modeQuery
		}
	}
	return m
}

// rewriteNative replaces the one caller flag pgb overrides. -march=native bakes
// in the build machine's CPU and the binary then dies elsewhere with SIGILL.
// Any other -march the caller passes wins: a project compiling one translation
// unit per ISA level behind a runtime check is doing the right thing.
func rewriteNative(args []string, baseline string) ([]string, bool) {
	changed := false
	out := make([]string, 0, len(args))
	for _, a := range args {
		switch a {
		case "-march=native":
			changed = true
			if baseline != "" {
				out = append(out, "-march="+baseline)
			} else {
				out = append(out, "-mtune=generic")
			}
		case "-mtune=native", "-mcpu=native":
			changed = true
			out = append(out, "-mtune=generic")
		default:
			out = append(out, a)
		}
	}
	return out, changed
}

// callerLibs re-emits the caller's own -l and -L after the plugin objects.
// An object's undefined references can only be resolved by a library that
// comes after it, and the caller cannot fix that because they do not control
// where pgb puts those objects. Repeating a -l is safe: an archive member is
// pulled in once. Nothing is invented, so a plugin needing a library the
// program never named still fails.
func callerLibs(args []string) []string {
	var out []string
	expectValue := ""
	for _, a := range args {
		if expectValue != "" {
			out = append(out, expectValue, a)
			expectValue = ""
			continue
		}
		switch {
		case a == "-l" || a == "-L":
			expectValue = a
		case strings.HasPrefix(a, "-l") || strings.HasPrefix(a, "-L"):
			out = append(out, a)
		}
	}
	return out
}

// findManifest locates the wrapper directory this invocation came from.
func findManifest() (*Manifest, string, error) {
	if strings.ContainsRune(os.Args[0], os.PathSeparator) {
		dir := filepath.Dir(os.Args[0])
		if m, err := LoadManifest(dir); err == nil {
			return m, dir, nil
		}
	}
	if dir := os.Getenv("PGB_WRAPPER_DIR"); dir != "" {
		if m, err := LoadManifest(dir); err == nil {
			return m, dir, nil
		}
	}
	for _, dir := range filepath.SplitList(os.Getenv("PATH")) {
		if dir == "" {
			continue
		}
		if m, err := LoadManifest(dir); err == nil {
			return m, dir, nil
		}
	}
	return nil, "", fmt.Errorf("no %s beside this wrapper, in $PGB_WRAPPER_DIR, or on $PATH", ManifestName)
}

// Dispatch runs one wrapper invocation. It does not return on success: the
// real compiler replaces this process.
func Dispatch(name string, args []string) int {
	m, dir, err := findManifest()
	if err != nil {
		fmt.Fprintf(os.Stderr, "pgb %s wrapper: %v\n", name, err)
		return 2
	}
	real := m.Real[name]
	if real == "" {
		fmt.Fprintf(os.Stderr, "pgb %s wrapper: %s/%s names no real compiler for %q\n",
			name, dir, ManifestName, name)
		return 2
	}
	verbose := os.Getenv("PGB_VERBOSE") != "" || os.Getenv("PGB_LOG") == "debug" ||
		os.Getenv("PGB_LOG") == "trace"

	// The preprocessor is a query tool: cpp never links, and decorating it
	// makes a configure test about predefined macros answer nonsense. -march
	// changes the predefined macro set, so the compile flags are dropped too.
	if name == "cpp" {
		return execReal(real, args, verbose, "passthrough")
	}

	md := classify(args)
	if md == modeQuery || md == modeShared {
		what := "query"
		if md == modeShared {
			what = "shared,passthrough"
		}
		return execReal(real, args, verbose, what)
	}

	args, rewrote := rewriteNative(args, m.Baseline)
	if rewrote && verbose {
		fmt.Fprintf(os.Stderr, "pgb[rewrote -march=native to %s]\n", m.Baseline)
	}

	var argv []string
	switch md {
	case modeCompile:
		argv = append(argv, m.Compile...)
		argv = append(argv, args...)
		if verbose {
			fmt.Fprintf(os.Stderr, "pgb[compile] %s %s\n", real, strings.Join(argv, " "))
		}
	default:
		link := m.Link
		if name == "c++" || name == "g++" {
			link = m.LinkCXX
		}
		argv = append(argv, m.Compile...)
		argv = append(argv, args...)
		argv = append(argv, link...)
		if m.WrapDlopen {
			argv = append(argv, callerLibs(args)...)
		}
		if verbose {
			fmt.Fprintf(os.Stderr, "pgb[link] %s %s\n", real, strings.Join(argv, " "))
		}
	}
	return execArgv(real, argv)
}

func execReal(real string, args []string, verbose bool, what string) int {
	if verbose {
		fmt.Fprintf(os.Stderr, "pgb[%s] %s %s\n", what, real, strings.Join(args, " "))
	}
	return execArgv(real, args)
}

func execArgv(real string, args []string) int {
	argv := append([]string{real}, args...)
	if err := syscall.Exec(real, argv, os.Environ()); err != nil {
		fmt.Fprintf(os.Stderr, "pgb wrapper: cannot exec %s: %v\n", real, err)
		return 2
	}
	return 0
}
