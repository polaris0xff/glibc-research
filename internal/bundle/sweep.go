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
	"bytes"
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

	// ⭐ CutEdges MAKES THE ALLOWLIST'S CEILING MEASURABLE. TODO T-066 route A.
	//
	// Each entry is `FROM=>TO`: a DT_NEEDED edge to treat as absent, where
	// FROM is the base name of the depending file (or `*` for any) and TO is
	// the soname it names. The sweep then reports what becomes unreachable
	// without that edge, and the delta against the same sweep with no cut is
	// the size of the subtree reachable ONLY through it.
	//
	// ⛔ WHY THIS IS THE QUESTION AND NOT AN OPTIMISATION. An allowlist chooses
	// which PATHS to carry; it cannot remove a dependency a library DECLARES.
	// A perfect allowlist naming only kdenlive's true dependencies still
	// carries libicudata.so, because the libQt6Core.so in the closure has a
	// DT_NEEDED on it — which is exactly what b.integrity() asserts must hold.
	// Only a REBUILD removes the edge, and `qt6-base-mini.sh`'s
	// `-DFEATURE_icu=OFF` is that rebuild. So the bytes reachable only through
	// the edges those recipes delete are the bytes no allowlist can reach, and
	// that is the ceiling this entry asked to be measured before the allowlist
	// is built.
	//
	// ⚠ NOTHING IS MODIFIED. The cut is applied to the graph walk, not to the
	// bundle: no ELF is rewritten and no file is removed, so the same AppDir
	// answers for every edge in turn and the measurement is repeatable.
	CutEdges []string
}

// cutSet is the parsed form of SweepOptions.CutEdges: for each depending base
// name (or "*"), the set of sonames whose edge is treated as absent.
type cutSet map[string]map[string]bool

func parseCutEdges(in []string) (cutSet, error) {
	if len(in) == 0 {
		return nil, nil
	}
	cs := cutSet{}
	for _, e := range in {
		from, to, ok := strings.Cut(e, "=>")
		if !ok {
			return nil, fail.Cannot("bundle sweep: --cut wants FROM=>TO, got %q", e)
		}
		from = strings.TrimSpace(from)
		to = strings.TrimSpace(to)
		if from == "" || to == "" {
			return nil, fail.Cannot("bundle sweep: --cut wants FROM=>TO, got %q", e)
		}
		if cs[from] == nil {
			cs[from] = map[string]bool{}
		}
		cs[from][to] = true
	}
	return cs, nil
}

// expandToFile widens every cut target to all index names for the same real
// file, so a cut names a DEPENDENCY rather than one spelling of it.
func (cs cutSet) expandToFile(index map[string]string) cutSet {
	if cs == nil {
		return nil
	}
	groups := selfKeys(index)
	byName := map[string]map[string]bool{}
	for _, keys := range groups {
		for k := range keys {
			byName[k] = keys
		}
	}
	out := cutSet{}
	for from, tos := range cs {
		out[from] = map[string]bool{}
		for to := range tos {
			out[from][to] = true
			for k := range byName[to] {
				out[from][k] = true
			}
		}
	}
	return out
}

// cut reports whether the edge from `fromBase` to soname `to` is treated as
// absent. A `*` entry matches any depending object.
func (cs cutSet) cut(fromBase, to string) bool {
	if cs == nil {
		return false
	}
	return cs[fromBase][to] || cs["*"][to]
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

	// ⛔ HOW MANY EDGES THE CUT ACTUALLY MATCHED, and it has to be reported.
	// A `--cut` naming an edge this bundle does not have removes nothing, so
	// the unreachable delta is zero — which reads exactly like "that edge
	// costs nothing to carry" and means the opposite. Zero here says the
	// measurement did not happen. docs/AGENTS.md §0b: an absence is not a zero.
	CutEdgesHit int
	// CutEdges echoes what was asked for, so Report can tell "no cut was
	// requested" from "a cut was requested and matched nothing".
	CutEdges   []string
	Unresolved []string // DT_NEEDED names nothing in the bundle provides
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

	// ⛔ Parsed BEFORE the index walk, because the cut reaches the soname
	// string scan as well as the DT_NEEDED walk -- see
	// sonamesMentionedInObjects.
	cuts, err := parseCutEdges(o.CutEdges)
	if err != nil {
		return nil, err
	}

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

	// ⛔ A LIBRARY NAMED IN A JSON MANIFEST IS A ROOT, and leaving it out is
	// the same defect as the MLT one, aimed at the GL stack.
	//
	// libglvnd finds its vendor by reading share/glvnd/egl_vendor.d/*.json and
	// dlopen'ing the "library_path" it names; the Vulkan loader does the same
	// with share/vulkan/icd.d/*.json. Those libraries sit in lib/ ITSELF, not
	// in a plugin subdirectory -- and the plugin-directory rule above skips
	// `p == root` deliberately, so lib/ is never one. Nothing carries
	// DT_NEEDED libEGL_mesa.so.0 either, because nothing links against a
	// vendor library.
	//
	// ⚠ So without this, a bundle's GL stack is UNREACHABLE by construction: a
	// sweep that deletes on reachability removes libEGL_mesa.so.0 and
	// libGLX_mesa.so.0, the integrity check passes because no DT_NEEDED points
	// at them, and the bundle fails at run time with `eglInitialize failed` --
	// which is a symptom TODO/research.md already records from an earlier
	// cause. docs/history/corrections.md C20 is the same shape.
	for _, r := range librariesNamedInManifests(o.Dir) {
		if p, ok := index[r]; ok {
			roots = append(roots, p)
		}
	}

	// ⛔ A SONAME SPELLED OUT INSIDE AN ELF IS A ROOT, because that is what a
	// dlopen(3) call by name looks like from the outside.
	//
	// ⚠ THIS IS THE THIRD CLASS OF RUNTIME-LOADED LIBRARY THIS SWEEP MISSED,
	// all three found on 2026-09-02c and each by a different symptom:
	//
	//   MLT's modules        loaded out of a directory named in .env
	//                        -> the .env rule, once it ran AFTER writeEnv
	//   libEGL_mesa.so.0     named in a vendor JSON, living in lib/ itself
	//                        -> the manifest rule above
	//   ⛔ libSDL3.so.0      dlopen'd BY NAME from inside an MLT module, with
	//                        no data file naming it anywhere
	//                        -> this rule. `melt` reported "Failed loading
	//                        SDL3 library." on all eleven environments
	//
	// The third has no manifest and no directory to key on. What it does have
	// is the string: a `dlopen("libSDL3.so.0")` puts that soname in the
	// caller's .rodata, so scanning every ELF in the bundle for names the
	// index already knows finds it without knowing anything about SDL or MLT.
	//
	// ⭐ Deliberately over-broad. A soname mentioned in a log message or a
	// version banner also counts, and keeping a library nothing loads costs
	// space where deleting one something loads costs the application. The
	// sweep's own contract says which way to err.
	// ⛔ THE CUT IS BY TARGET FILE, NOT BY NAME, and it has to be resolved
	// against the index before it is used.
	//
	// ⚠ MEASURED, and the first version got it wrong. One library is in the
	// index under several names: `libunistring.so`, `libunistring.so.5` and
	// `libunistring.so.5.2.1` all resolve to one file. Cutting
	// `libidn2=>libunistring.so.5` suppressed that one needle and left
	// `libunistring.so` -- which is a SUBSTRING of the same string in libidn2's
	// .dynstr -- to make the library a root anyway. The roots count fell by one
	// and not a byte moved.
	//
	// ⭐ So a cut naming any one of a file's names cuts all of them. That is
	// also what a rebuild does: the dependency is gone, not renamed.
	cuts = cuts.expandToFile(index)

	for _, r := range sonamesMentionedInObjects(o.Dir, allObjects, index, cuts) {
		roots = append(roots, r)
	}
	roots = uniq(roots)

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
	cutHits := 0
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
			base := filepath.Base(n)
			// ⭐ The edge is dropped from the WALK, never from the bundle.
			if cuts.cut(filepath.Base(real), base) {
				cutHits++
				continue
			}
			target, ok := index[base]
			if !ok {
				unresolved[base] = true
				continue
			}
			queue = append(queue, target)
		}
	}

	res := &SweepResult{
		Roots:       roots,
		Reachable:   reachable,
		TotalFiles:  totalFiles,
		TotalBytes:  totalBytes,
		CutEdgesHit: cutHits,
		CutEdges:    o.CutEdges,
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
		for part := range strings.SplitSeq(value, ":") {
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
	// ⛔ Printed whenever a cut was asked for, INCLUDING when it matched
	// nothing -- that is the reading the caller most needs and the one a
	// silent zero would hide.
	if len(r.CutEdges) > 0 {
		fmt.Fprintf(w, "cut requested    %d: %s\n", len(r.CutEdges), strings.Join(r.CutEdges, " "))
		fmt.Fprintf(w, "cut edges hit    %d%s\n", r.CutEdgesHit,
			map[bool]string{true: "   ⛔ NOTHING MATCHED -- this bundle has no such edge, so a zero delta below measures nothing", false: ""}[r.CutEdgesHit == 0])
	}
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

// manifestLibrary matches the library a vendor or ICD JSON names. The value
// may be a bare soname (what `pgb bundle` rewrites them to) or an absolute
// path (what nixpkgs ships); only the base name is wanted either way.
var manifestLibrary = regexp.MustCompile(`"library_path"\s*:\s*"([^"]+)"`)

// manifestGlobs are the JSON trees whose contents name libraries by path.
var manifestGlobs = []string{
	"share/glvnd/egl_vendor.d/*.json",
	"share/vulkan/icd.d/*.json",
	"share/vulkan/implicit_layer.d/*.json",
	"share/vulkan/explicit_layer.d/*.json",
	"etc/OpenCL/vendors/*.icd",
}

// librariesNamedInManifests returns the base names those manifests name.
//
// ⚠ Read with a regexp rather than a JSON parser on purpose: these files are
// third-party, occasionally have comments or trailing commas, and a parse
// error here must not cost the bundle its GL stack. A regexp that finds
// nothing degrades to the previous behaviour; a strict parser that refuses the
// file would delete the libraries it could not read about.
func librariesNamedInManifests(dir string) []string {
	var out []string
	for _, g := range manifestGlobs {
		matches, _ := filepath.Glob(filepath.Join(dir, g))
		for _, f := range matches {
			data, err := os.ReadFile(f)
			if err != nil {
				continue
			}
			for _, m := range manifestLibrary.FindAllSubmatch(data, -1) {
				out = append(out, filepath.Base(string(m[1])))
			}
			// An .icd file may be a bare soname on one line, no JSON at all.
			if strings.HasSuffix(f, ".icd") {
				for _, line := range strings.Fields(string(data)) {
					if IsSharedObject(line) {
						out = append(out, filepath.Base(line))
					}
				}
			}
		}
	}
	return uniq(out)
}

// dotSo is the substring every shared-object name contains, by IsSharedObject's
// own definition. It is what makes the run filter below exact rather than a
// heuristic.
var dotSo = []byte(".so")

// sonamesMentionedInObjects finds, for every ELF in the bundle, the shared
// object names it spells out literally -- the fingerprint of a dlopen by name.
//
// ⚠ Substring search over the whole file rather than a .rodata walk: a soname
// can also arrive through a string table, a build-id note, or a constant
// assembled at compile time into an unexpected section, and reading the wrong
// section is how the previous three misses happened.
//
// ⛔ AND IT WAS QUADRATIC, WHICH IS NOT A STYLE POINT — IT IS MOST OF WHY
// `--debloat aggressive` COSTS WHAT IT COSTS. The obvious shape is a
// `bytes.Contains` per needle per object, which re-reads every byte of the
// bundle once for every library in it: on kdenlive that is roughly a thousand
// objects against a thousand names over a 2 GiB tree.
//
// ⭐ THE FAST PATH IS EXACTLY EQUIVALENT, BY CONSTRUCTION RATHER THAN BY
// CARE. Every needle is a shared-object name, so:
//
//   - build the ALPHABET as the set of bytes that occur in some needle. A
//     needle occurring in the data consists only of those bytes, so it lies
//     entirely inside one maximal run of them. Extracting the runs therefore
//     cannot lose an occurrence, and the alphabet is derived from the needles
//     rather than written down, so it cannot drift from them;
//   - keep only runs containing `.so`. `IsSharedObject` requires the name to
//     end in `.so` or `.so.N`, so every needle contains that substring, so
//     every run containing a needle contains it too;
//   - a run's needles are computed ONCE and cached, because a bundle spells
//     the same soname out in hundreds of objects.
//
// The result is one pass over the bytes plus a needle scan over the few
// distinct soname-shaped strings the bundle actually contains.
// `sonamesMentionedNaive` below is the original, kept as the CONTROL its
// selftest compares against — this is a change to the function that decides
// what gets DELETED, and "it looks equivalent" is not the standard.
// selfKeys groups the index's keys by the real file they resolve to, so a scan
// can ask "which of these names name THIS object" instead of comparing one
// string.
//
// ⛔ THE DEFECT IT REPLACES, AND IT DISABLED THE SWEEP'S LARGEST LEVER.
// Both scans below excluded the scanned object's own base name — `n != self`
// with `self = filepath.Base(o)` — so that a library mentioning its own name
// did not become a root of itself. ⚠ **For a versioned library that check can
// never fire.** `libunistring.so.5.2.1` carries `DT_SONAME libunistring.so.5`;
// the index holds `libunistring.so.5`, because the symlink beside it is an
// index key; and `"libunistring.so.5" != "libunistring.so.5.2.1"`. So the
// needle matched the SONAME string sitting in the object's own `.dynstr`, the
// self-check compared it against the FILENAME, and the library became a root
// of itself.
//
// ⭐ MEASURED ON A REAL BUNDLE, not reasoned from the shape of the check: in
// the `jq` AppDir, `libunistring.so.5` is reachable with its only DT_NEEDED
// edge cut, and the only two files containing that string are `libidn2` and
// `libunistring.so.5.2.1` itself. Nearly every ordinary shared library on a
// Linux system has a SONAME that differs from its filename, so this held for
// nearly all of them — `DropUnreachable` could not drop a versioned library
// whatever the graph said. TODO T-066.
//
// ⚠ It is deliberately by REAL FILE rather than by name: the group for
// `libunistring.so.5.2.1` is `{libunistring.so, libunistring.so.5,
// libunistring.so.5.2.1}`, which is the filename, the SONAME symlink and the
// development symlink at once, without a rule about version suffixes.
func selfKeys(index map[string]string) map[string]map[string]bool {
	byReal := map[string]map[string]bool{}
	for k, p := range index {
		real, err := filepath.EvalSymlinks(p)
		if err != nil {
			real = p
		}
		if byReal[real] == nil {
			byReal[real] = map[string]bool{}
		}
		byReal[real][k] = true
	}
	return byReal
}

// selfSetFor returns the index keys naming the same file as o.
func selfSetFor(groups map[string]map[string]bool, o string) map[string]bool {
	real, err := filepath.EvalSymlinks(o)
	if err != nil {
		real = o
	}
	if s := groups[real]; s != nil {
		return s
	}
	return map[string]bool{filepath.Base(o): true}
}

// ⭐ THE CUT REACHES THE STRING SCAN TOO, AND IT HAS TO.
//
// A `-mini` rebuild does not merely stop linking against a library: it removes
// the `DT_NEEDED` entry, and a DT_NEEDED entry IS a string in the object's own
// `.dynstr`. So the rebuilt `libQt6Core.so.6` does not mention `libicuuc` at
// all. A cut that suppressed the graph edge but left the string behind would
// keep the library reachable through this rule and report a ceiling of zero
// for every edge — which is exactly what the first version of this measured on
// the `jq` bundle. TODO T-066 route A.
func sonamesMentionedInObjects(root string, objects []string, index map[string]string, cuts cutSet) []string {
	// Only names the bundle actually has can be roots, so the needle set is
	// the index rather than every plausible soname.
	var needles []string
	var alphabet [256]bool
	minLen := 0
	for n := range index {
		if !IsSharedObject(n) {
			continue
		}
		needles = append(needles, n)
		for i := 0; i < len(n); i++ {
			alphabet[n[i]] = true
		}
		if minLen == 0 || len(n) < minLen {
			minLen = len(n)
		}
	}
	if len(needles) == 0 {
		return nil
	}

	runHits := map[string][]string{} // a distinct run -> the needles inside it
	found := map[string]bool{}
	groups := selfKeys(index)
	for _, o := range objects {
		data, err := os.ReadFile(o)
		if err != nil {
			continue
		}
		self := selfSetFor(groups, o)
		runs := map[string]bool{}
		start := -1
		for i := 0; i <= len(data); i++ {
			if i < len(data) && alphabet[data[i]] {
				if start < 0 {
					start = i
				}
				continue
			}
			if start >= 0 {
				if r := data[start:i]; len(r) >= minLen && bytes.Contains(r, dotSo) {
					runs[string(r)] = true
				}
				start = -1
			}
		}
		for r := range runs {
			hits, done := runHits[r]
			if !done {
				for _, n := range needles {
					if len(n) <= len(r) && strings.Contains(r, n) {
						hits = append(hits, n)
					}
				}
				if hits == nil {
					hits = []string{}
				}
				runHits[r] = hits
			}
			for _, n := range hits {
				if !self[n] && !cuts.cut(filepath.Base(o), n) {
					found[n] = true
				}
			}
		}
	}
	var out []string
	for n := range found {
		if p, ok := index[n]; ok {
			out = append(out, p)
		}
	}
	sort.Strings(out)
	return out
}

// sonamesMentionedNaive is the ORIGINAL implementation of the function above:
// one `bytes.Contains` per needle per object. It is kept, and it is not dead
// code — it is the control `SonameScanSelftest` compares the fast path
// against.
//
// ⛔ WHY A CONTROL AND NOT A READING. This function decides which libraries a
// bundle KEEPS. A speedup that silently stopped finding one name would delete
// a library the application dlopens by name, on somebody else's machine, with
// every DT_NEEDED still resolving and every gate in this tree green — which is
// exactly how the libSDL3 miss reached a run. The two implementations are
// compared on a fixture instead.
func sonamesMentionedNaive(root string, objects []string, index map[string]string, cuts cutSet) []string {
	needles := make([][]byte, 0, len(index))
	names := make([]string, 0, len(index))
	for n := range index {
		if !IsSharedObject(n) {
			continue
		}
		needles = append(needles, []byte(n))
		names = append(names, n)
	}
	found := map[string]bool{}
	groups := selfKeys(index)
	for _, o := range objects {
		data, err := os.ReadFile(o)
		if err != nil {
			continue
		}
		self := selfSetFor(groups, o)
		for i, nd := range needles {
			if self[names[i]] || found[names[i]] || cuts.cut(filepath.Base(o), names[i]) {
				continue
			}
			if bytes.Contains(data, nd) {
				found[names[i]] = true
			}
		}
	}
	var out []string
	for n := range found {
		if p, ok := index[n]; ok {
			out = append(out, p)
		}
	}
	sort.Strings(out)
	return out
}
