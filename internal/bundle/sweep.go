// Package bundle inspects and reduces a bundled application directory.
//
// sweep.go answers one question: which of the shared objects in a bundle can
// anything actually reach? A closure carries every path the derivations
// declared, and a bundle built from one carries libraries no program in it
// needs — but "needs" has to include what is loaded by name at run time, so
// the sweep starts from the programs AND from every directory something could
// plausibly load a plugin out of.
//
// The rules are structural, not a list of names:
//
//   - a root is an executable in the bundle's program directories;
//   - a plugin directory is a directory under the library root that holds
//     shared objects, or one an environment file names;
//   - reachability is the DT_NEEDED closure of the roots plus every object in
//     a plugin directory, resolved by base name inside the bundle.
//
// Anything a rule cannot classify counts as REACHABLE. A sweep that guesses
// wrong in that direction wastes space; guessing wrong the other way deletes a
// library the application loads.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/elfx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
)

var log = logx.New("bundle")

// soName matches a shared object: a name ending in .so or .so.N. Matching the
// substring ".so" anywhere also matches ld.so.cache and a directory called
// something.sources.
var soName = regexp.MustCompile(`\.so(\.[0-9]+)*$`)

// IsSharedObject reports whether a base name names a shared object.
func IsSharedObject(name string) bool { return soName.MatchString(name) }

// SweepOptions describes where to look.
type SweepOptions struct {
	Dir string // the bundle root

	// ProgramDirs and LibDirs are relative to Dir. Empty means "discover".
	ProgramDirs []string
	LibDirs     []string

	// EnvFiles are read for variables naming a directory inside the bundle;
	// a directory named that way is a plugin directory even when nothing in it
	// looks like one.
	EnvFiles []string

	// ExtraRoots are additional programs to start from, by path or base name.
	ExtraRoots []string
}

// SweepResult is what the sweep found.
type SweepResult struct {
	Roots        []string
	PluginDirs   []string
	Reachable    map[string]bool
	Unreachable  []string
	TotalFiles   int
	TotalBytes   int64
	UnreachFiles int
	UnreachBytes int64
	Unresolved   []string // DT_NEEDED names nothing in the bundle provides
}

// discoverDirs finds the bundle's program and library roots by looking, so a
// layout that is not the one this tool writes still sweeps.
func discoverDirs(dir string, want func(name string) bool) []string {
	var out []string
	for _, cand := range []string{".", "shared", "usr", "usr/local"} {
		base := filepath.Join(dir, cand)
		entries, err := os.ReadDir(base)
		if err != nil {
			continue
		}
		for _, e := range entries {
			if e.IsDir() && want(e.Name()) {
				rel, err := filepath.Rel(dir, filepath.Join(base, e.Name()))
				if err == nil {
					out = append(out, rel)
				}
			}
		}
	}
	sort.Strings(out)
	return uniq(out)
}

// Sweep computes reachability over a bundle directory.
func Sweep(o SweepOptions) (*SweepResult, error) {
	if o.Dir == "" {
		return nil, fail.Cannot("bundle sweep needs a directory")
	}
	if fi, err := os.Stat(o.Dir); err != nil || !fi.IsDir() {
		return nil, fail.Cannot("%s is not a directory", o.Dir)
	}
	progDirs := o.ProgramDirs
	if len(progDirs) == 0 {
		progDirs = discoverDirs(o.Dir, func(n string) bool { return n == "bin" || n == "sbin" || n == "libexec" })
	}
	libDirs := o.LibDirs
	if len(libDirs) == 0 {
		libDirs = discoverDirs(o.Dir, func(n string) bool {
			return n == "lib" || n == "lib32" || n == "lib64"
		})
	}
	if len(libDirs) == 0 {
		return nil, fail.Cannot("%s has no library directory to sweep", o.Dir)
	}
	log.Debugf("program dirs: %v", progDirs)
	log.Debugf("library dirs: %v", libDirs)

	// The index: every shared object in the bundle, by base name. A symlink is
	// followed to whatever it names inside the bundle; a name that resolves to
	// two files keeps the first, because both are the same library.
	index := map[string]string{}
	var allObjects []string
	var totalFiles int
	var totalBytes int64
	for _, ld := range libDirs {
		root := filepath.Join(o.Dir, ld)
		_ = filepath.Walk(root, func(p string, fi os.FileInfo, err error) error {
			if err != nil {
				return nil
			}
			if fi.IsDir() {
				return nil
			}
			if fi.Mode()&os.ModeSymlink != 0 {
				if _, seen := index[fi.Name()]; !seen {
					index[fi.Name()] = p
				}
				return nil
			}
			totalFiles++
			totalBytes += fi.Size()
			if !IsSharedObject(fi.Name()) && !elfx.IsELF(p) {
				return nil
			}
			allObjects = append(allObjects, p)
			if _, seen := index[fi.Name()]; !seen {
				index[fi.Name()] = p
			}
			return nil
		})
		// Symlinks are indexed in a second pass so a real file always wins.
		_ = filepath.Walk(root, func(p string, fi os.FileInfo, err error) error {
			if err != nil || fi.IsDir() {
				return nil
			}
			if lfi, err := os.Lstat(p); err == nil && lfi.Mode()&os.ModeSymlink != 0 {
				if _, seen := index[filepath.Base(p)]; !seen {
					index[filepath.Base(p)] = p
				}
			}
			return nil
		})
	}

	// The roots: every executable in the program directories, plus anything
	// the caller named.
	var roots []string
	for _, pd := range progDirs {
		entries, err := os.ReadDir(filepath.Join(o.Dir, pd))
		if err != nil {
			continue
		}
		for _, e := range entries {
			p := filepath.Join(o.Dir, pd, e.Name())
			fi, err := os.Stat(p)
			if err != nil || fi.IsDir() {
				continue
			}
			if elfx.IsELF(p) {
				roots = append(roots, p)
			}
		}
	}
	for _, r := range o.ExtraRoots {
		if filepath.IsAbs(r) || strings.ContainsRune(r, os.PathSeparator) {
			roots = append(roots, r)
			continue
		}
		if p, ok := index[r]; ok {
			roots = append(roots, p)
		}
	}
	roots = uniq(roots)
	if len(roots) == 0 {
		return nil, fail.Ran("%s has no executable to start from", o.Dir)
	}

	// The plugin directories: a directory under a library root that holds
	// shared objects is one, because nothing links against a plugin — it is
	// loaded by name. An environment file naming a directory makes it one too,
	// even when its contents do not look like plugins.
	pluginDirs := map[string]bool{}
	for _, ld := range libDirs {
		root := filepath.Join(o.Dir, ld)
		_ = filepath.Walk(root, func(p string, fi os.FileInfo, err error) error {
			if err != nil || !fi.IsDir() || p == root {
				return nil
			}
			entries, err := os.ReadDir(p)
			if err != nil {
				return nil
			}
			for _, e := range entries {
				if !e.IsDir() && IsSharedObject(e.Name()) {
					pluginDirs[p] = true
					return nil
				}
			}
			return nil
		})
	}
	for _, env := range o.EnvFiles {
		for _, d := range dirsNamedIn(o.Dir, env) {
			pluginDirs[d] = true
		}
	}

	// Everything in a plugin directory is a root of its own closure.
	for dir := range pluginDirs {
		entries, err := os.ReadDir(dir)
		if err != nil {
			continue
		}
		for _, e := range entries {
			if e.IsDir() {
				continue
			}
			p := filepath.Join(dir, e.Name())
			if IsSharedObject(e.Name()) || elfx.IsELF(p) {
				roots = append(roots, p)
			}
		}
	}
	roots = uniq(roots)

	// The closure.
	reachable := map[string]bool{}
	unresolved := map[string]bool{}
	queue := append([]string(nil), roots...)
	for len(queue) > 0 {
		p := queue[len(queue)-1]
		queue = queue[:len(queue)-1]
		real, err := filepath.EvalSymlinks(p)
		if err != nil {
			real = p
		}
		if reachable[real] {
			continue
		}
		reachable[real] = true
		reachable[p] = true
		needed, err := elfx.Needed(real)
		if err != nil {
			continue
		}
		for _, n := range needed {
			target, ok := index[filepath.Base(n)]
			if !ok {
				unresolved[filepath.Base(n)] = true
				continue
			}
			queue = append(queue, target)
		}
	}

	res := &SweepResult{
		Roots:      roots,
		Reachable:  reachable,
		TotalFiles: totalFiles,
		TotalBytes: totalBytes,
	}
	for d := range pluginDirs {
		if rel, err := filepath.Rel(o.Dir, d); err == nil {
			res.PluginDirs = append(res.PluginDirs, rel)
		}
	}
	sort.Strings(res.PluginDirs)
	for u := range unresolved {
		res.Unresolved = append(res.Unresolved, u)
	}
	sort.Strings(res.Unresolved)

	for _, p := range allObjects {
		real, err := filepath.EvalSymlinks(p)
		if err != nil {
			real = p
		}
		if reachable[real] || reachable[p] {
			continue
		}
		fi, err := os.Stat(p)
		if err != nil {
			continue
		}
		rel, err := filepath.Rel(o.Dir, p)
		if err != nil {
			rel = p
		}
		res.Unreachable = append(res.Unreachable, rel)
		res.UnreachFiles++
		res.UnreachBytes += fi.Size()
	}
	sort.Strings(res.Unreachable)
	return res, nil
}

// envAssign matches NAME=VALUE in a bundle's environment file.
var envAssign = regexp.MustCompile(`(?m)^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)=(.*)$`)

// dirsNamedIn reads an environment file and returns the directories inside the
// bundle that its values name. A value can be a colon-separated list and can
// carry ${SHARUN_DIR} or $APPDIR, which are the bundle root.
func dirsNamedIn(root, envFile string) []string {
	b, err := os.ReadFile(envFile)
	if err != nil {
		return nil
	}
	var out []string
	for _, m := range envAssign.FindAllStringSubmatch(string(b), -1) {
		value := strings.Trim(strings.TrimSpace(m[2]), `"'`)
		for _, part := range strings.Split(value, ":") {
			p := part
			for _, v := range []string{"${SHARUN_DIR}", "$SHARUN_DIR", "${APPDIR}", "$APPDIR", "${ORIGIN}", "$ORIGIN"} {
				p = strings.ReplaceAll(p, v, root)
			}
			if p == "" || !strings.HasPrefix(p, root) {
				continue
			}
			if fi, err := os.Stat(p); err == nil && fi.IsDir() {
				out = append(out, filepath.Clean(p))
			}
		}
	}
	return uniq(out)
}

// Report renders the sweep.
func (r *SweepResult) Report(w *strings.Builder, listAll bool) {
	fmt.Fprintf(w, "roots            %d\n", len(r.Roots))
	fmt.Fprintf(w, "plugin dirs      %d%s\n", len(r.PluginDirs), joinIfShort(r.PluginDirs))
	fmt.Fprintf(w, "library files    %d, %s\n", r.TotalFiles, humanBytes(r.TotalBytes))
	fmt.Fprintf(w, "unreachable      %d files, %d bytes (%s, %.1f%%)\n",
		r.UnreachFiles, r.UnreachBytes, humanBytes(r.UnreachBytes),
		percent(r.UnreachBytes, r.TotalBytes))
	if len(r.Unresolved) > 0 {
		fmt.Fprintf(w, "unresolved       %d DT_NEEDED name(s) nothing in the bundle provides:\n",
			len(r.Unresolved))
		for _, u := range r.Unresolved {
			fmt.Fprintf(w, "  %s\n", u)
		}
	}
	if listAll {
		for _, u := range r.Unreachable {
			fmt.Fprintf(w, "  %s\n", u)
		}
	}
}

func percent(a, b int64) float64 {
	if b == 0 {
		return 0
	}
	return float64(a) * 100 / float64(b)
}

func joinIfShort(v []string) string {
	if len(v) == 0 || len(v) > 8 {
		return ""
	}
	return "  " + strings.Join(v, " ")
}

func humanBytes(n int64) string {
	switch {
	case n < 1024:
		return fmt.Sprintf("%d B", n)
	case n < 1024*1024:
		return fmt.Sprintf("%.1f KiB", float64(n)/1024)
	case n < 1024*1024*1024:
		return fmt.Sprintf("%.1f MiB", float64(n)/(1024*1024))
	default:
		return fmt.Sprintf("%.2f GiB", float64(n)/(1024*1024*1024))
	}
}

func uniq(in []string) []string {
	seen := map[string]bool{}
	out := in[:0]
	for _, s := range in {
		if s == "" || seen[s] {
			continue
		}
		seen[s] = true
		out = append(out, s)
	}
	return out
}
