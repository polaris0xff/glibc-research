// storefix.go — the compiled-in store path, resolved against the bundle.
//
// ⛔ THIS IS THE MECHANISM T-081 IS ABOUT, AND ITS COST IS MEASURED.
// `experiments/64-` arm G: a GTK 3 application out of a nixpkgs closure loads
// GTK, connects to a real X server, opens zero host shared objects — and draws
// nothing on 11 of 11, because it opens its own `/nix/store/…/share/…/main_frame.ui`
// and that path does not exist on the target. The file IS in the bundle.
//
// ⛔ THE FIELD'S ROUTE IS FIVE OVERLAPPING sed REGEXES ending in "replace any
// remaining store path with /". `quick-sharun.sh` also carries the same idea
// this file takes — `_map_paths_ld_preload_open` builds `path-mapping.so` from
// `fritzw/ld-preload-open` and drives it from a `PATH_MAPPING` variable — and
// the difference is the whole entry: THEIRS IS WRITTEN BY HAND, PER RECIPE.
// `pgb` computes the closure, so the set of store paths in the bundle is known
// and finite, every rewrite is an exact match against it, and a store path that
// is NOT in the set is a FINDING rather than a silent substitution.
//
// ⛔ AND THE OTHER OBVIOUS ROUTE IS REFUSED ON SECURITY GROUNDS BEFORE IT WAS
// BUILT: `/nix/store/` and `/tmp/.pgbs/` are both 11 bytes, so a same-length
// prefix rewrite inside the ELF needs no relocation — and a fixed, predictable
// path under a world-writable directory is squattable by any local user, on a
// tree that is loadable code. docs/design/store-paths.md §2.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"debug/elf"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

// storefixSO is the name the bundle carries the interposer under, and the name
// `.preload` names. A bare soname is what sharun hands the loader, which
// resolves it out of the bundle's own --library-path.
const storefixSO = "libpgb-storefix.so"

// mergedFor maps a store path's top-level directory to the AppDir directory
// that now holds its contents.
//
// ⛔ IT IS A TABLE, NOT A SEARCH. The bundle FLATTENS a closure — every
// library into lib/, every program into shared/bin/, every share/ tree merged —
// so where a store path's contents went is something the bundler knows rather
// than something the run time should look for. A top-level directory not in
// this table is REPORTED, so the list grows on evidence.
var mergedFor = map[string]string{
	"bin":     "shared/bin",
	"sbin":    "shared/bin",
	"lib":     "lib",
	"lib64":   "lib",
	"lib32":   "lib32",
	"share":   "share",
	"etc":     "etc",
	"libexec": "libexec",
}

// mergedTopLevel are the trees copyClosureTrees merges out of every store path.
// share/ was the only one until T-081; etc/ and libexec/ joined it because a
// compiled-in path naming them has nowhere to resolve to otherwise.
var mergedTopLevel = []string{"share", "etc", "libexec"}

// StoreMapEntry is one row of the map the interposer reads.
type StoreMapEntry struct {
	Base string // "<32-char hash>-<name>-<version>"
	Dir  string // AppDir-relative directory holding that store path's tree
}

// storefix builds the whole mechanism: the per-store-path directories, the
// map, the interposer, and the report.
//
// ⚠ IT RUNS AFTER writeEnv, DELIBERATELY. carryBakedPaths has by then copied
// the subtrees an override variable can redirect into store/<name>, and those
// are REAL directories that must win over a symlink into the merged tree.
func (b *Builder) storefix() error {
	if b.O.NoStorefix {
		// ⭐ THE NEGATIVE CONTROL IS A SHIPPED FLAG, not a comment. A success
		// criterion that cannot be made to fail is not an instrument —
		// PROGRESS.md delivery rule 6 — so `--no-storefix` builds the same
		// bundle with this one mechanism absent and nothing else changed.
		logx.Warnf("--no-storefix: compiled-in store paths are NOT resolved")
		_ = os.Remove(filepath.Join(b.AppDir, ".storemap"))
		_ = os.Remove(filepath.Join(b.AppDir, ".preload"))
		_ = os.Remove(filepath.Join(b.AppDir, "lib", storefixSO))
		b.reportStorePaths(nil)
		return nil
	}
	entries, err := b.buildStoreFarm()
	if err != nil {
		return err
	}
	if err := b.writeStoreMap(entries); err != nil {
		return err
	}
	b.rewriteTextStorePaths(entries)
	if err := b.installStorefix(); err != nil {
		// ⚠ REPORTED, NOT FATAL, and the report says what the bundle will do
		// without it. A bundle that silently loses this mechanism is worse
		// than one that says it has.
		logx.Warnf("the store-path interposer was NOT installed: %v", err)
		logx.Warnf("   a compiled-in /nix/store path will NOT resolve at run time")
	}
	b.reportStorePaths(entries)
	return nil
}

// copyClosureTrees merges the closure's top-level data trees into the AppDir.
func (b *Builder) copyClosureTrees() {
	for _, top := range mergedTopLevel {
		matches, _ := filepath.Glob(filepath.Join(b.Root, "*", top))
		for _, d := range matches {
			if fi, err := os.Stat(d); err != nil || !fi.IsDir() {
				continue
			}
			_ = copyTreeNoClobber(d, filepath.Join(b.AppDir, top))
		}
	}
}

// buildStoreFarm gives every store path in the closure a directory inside the
// bundle that holds the same tree, and returns the map from one to the other.
//
// ⭐ IT IS SYMLINKS, NOT COPIES. The contents are already in the bundle, once,
// flattened; a second copy per store path would multiply a 150 MB bundle by the
// number of packages in its closure. store/.root carries one symlink per
// top-level directory and every store path points at it — except the ones
// carryBakedPaths already materialised, which keep their real contents and only
// gain the entries they were missing.
// buildStoreFarmNames is what buildStoreFarm would WRITE, without creating
// anything. It exists so a selftest can compare the farm's side of the naming
// rule against the `.env`'s side; asking farmDirName twice would prove nothing.
func (b *Builder) buildStoreFarmNames() (map[string]string, error) {
	names, err := os.ReadDir(b.Root)
	if err != nil {
		return nil, err
	}
	out := map[string]string{}
	for _, e := range names {
		if e.IsDir() {
			out[e.Name()] = "store/" + b.farmDirName(e.Name())
		}
	}
	return out, nil
}

func (b *Builder) buildStoreFarm() ([]StoreMapEntry, error) {
	storeDir := filepath.Join(b.AppDir, "store")
	root := filepath.Join(storeDir, ".root")
	if err := os.MkdirAll(root, 0o755); err != nil {
		return nil, err
	}
	// One symlink per top-level name, pointing at where the bundle put that
	// kind of thing. Written whether or not the directory exists yet: the
	// debloater can remove a tree later, and a dangling symlink resolves to
	// ENOENT, which is the same answer the program would have got anyway.
	for name, target := range mergedFor {
		link := filepath.Join(root, name)
		if _, err := os.Lstat(link); err == nil {
			continue
		}
		_ = os.Symlink(filepath.Join("../..", target), link)
	}

	names, err := os.ReadDir(b.Root)
	if err != nil {
		return nil, err
	}
	// ⚠ TWO STORE PATHS CAN SHARE A NAME AND VERSION and differ only in the
	// hash — a closure legitimately carries two builds of one package. The
	// short name is used when it is unique and the full one when it is not, so
	// the map is never ambiguous.
	var out []StoreMapEntry
	unmapped := map[string]bool{}
	for _, e := range names {
		if !e.IsDir() {
			continue
		}
		base := e.Name()
		dirName := b.farmDirName(base)
		dst := filepath.Join(storeDir, dirName)
		fi, err := os.Lstat(dst)
		switch {
		case err != nil:
			// Nothing here yet: one symlink to the shared root.
			if err := os.Symlink(".root", dst); err != nil {
				continue
			}
		case fi.Mode()&os.ModeSymlink != 0:
			// Already ours.
		case fi.IsDir():
			// carryBakedPaths materialised part of this package. Fill in the
			// top-level names it did not carry, so the rest of the package
			// still resolves.
			b.fillFarmDir(base, dst)
		}
		for _, top := range topLevelNames(filepath.Join(b.Root, base)) {
			if _, ok := mergedFor[top]; !ok {
				unmapped[top] = true
			}
		}
		out = append(out, StoreMapEntry{Base: base, Dir: "store/" + dirName})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Base < out[j].Base })

	if len(unmapped) > 0 {
		// ⛔ AN ABSENCE IS NOT A ZERO. These are top-level directories the
		// bundle does not merge, so a compiled-in path naming one cannot
		// resolve. Said out loud rather than left to be discovered at a user's
		// double-click.
		var list []string
		for k := range unmapped {
			list = append(list, k)
		}
		sort.Strings(list)
		// ⚠ "entries", not "directories": topLevelNames returns FILES too, and
		// the case that made this warning fire in anger was 200 bare `.so`
		// files at a store path's top level (helix's tree-sitter grammars).
		// Calling them directories sent the reader looking for the wrong thing.
		logx.Warnf("store paths carry %d top-level entries the bundle does not merge", len(list))
		logx.Warnf("   a compiled-in path naming one of these cannot resolve: %s",
			strings.Join(list, " "))
	}
	logx.Say("store map   %d store path(s) resolve inside the bundle", len(out))
	return out, nil
}

// fillFarmDir adds the top-level entries a materialised store directory is
// missing, as symlinks into the merged trees.
func (b *Builder) fillFarmDir(base, dst string) {
	for _, top := range topLevelNames(filepath.Join(b.Root, base)) {
		target, ok := mergedFor[top]
		if !ok {
			continue
		}
		link := filepath.Join(dst, top)
		if _, err := os.Lstat(link); err == nil {
			continue
		}
		_ = os.Symlink(filepath.Join("../..", target), link)
	}
}

func topLevelNames(dir string) []string {
	entries, err := os.ReadDir(dir)
	if err != nil {
		return nil
	}
	out := make([]string, 0, len(entries))
	for _, e := range entries {
		out = append(out, e.Name())
	}
	return out
}

// farmDirName is THE rule for what a store path is called inside the bundle,
// and it exists because there were two of them.
//
// ⛔ THE DIVERGENCE IT CLOSES. buildStoreFarm falls back to the FULL
// `<hash>-<name>` when two store paths share a short name — a closure
// legitimately carries two builds of one package. `carryBakedPaths` computed
// `base[33:]` inline and had no such fallback, so on a collision the `.env`
// named `store/<short>` while the farm had created `store/<hash>-<short>`, and
// every variable pointing into that package resolved to nothing. Nothing
// caught it: `integrity()` walks DT_NEEDED and `manifestIntegrity()` reads ICD
// manifests, and neither reads a `.env` value back against the tree.
// docs/history/corrections.md C28, T-092.
//
// ⚠ It is keyed on b.Root, the CLOSURE, not on the AppDir — so it gives the
// same answer before the farm is built (writeEnv) and after (storefix).
func (b *Builder) farmDirName(base string) string {
	if b.farmNames == nil {
		b.farmNames = map[string]string{}
		count := map[string]int{}
		names, _ := os.ReadDir(b.Root)
		for _, e := range names {
			if e.IsDir() {
				count[shortStoreName(e.Name())]++
			}
		}
		for _, e := range names {
			if !e.IsDir() {
				continue
			}
			short := shortStoreName(e.Name())
			if count[short] > 1 {
				b.farmNames[e.Name()] = e.Name()
			} else {
				b.farmNames[e.Name()] = short
			}
		}
	}
	if n, ok := b.farmNames[base]; ok {
		return n
	}
	return shortStoreName(base)
}

func shortStoreName(base string) string {
	if len(base) > 33 {
		return base[33:]
	}
	return base
}

func (b *Builder) writeStoreMap(entries []StoreMapEntry) error {
	var sb strings.Builder
	fmt.Fprintf(&sb, "# pgb store map: %d store path(s) in this bundle's closure.\n", len(entries))
	sb.WriteString("# <store base name>\\t<AppDir-relative directory>\n")
	sb.WriteString("# ⛔ EXACT MATCH. A store path not listed here is reported, never guessed.\n")
	for _, e := range entries {
		fmt.Fprintf(&sb, "%s\t%s\n", e.Base, e.Dir)
	}
	return os.WriteFile(filepath.Join(b.AppDir, ".storemap"), []byte(sb.String()), 0o644)
}

// installStorefix compiles the interposer, checks it against the bundle's own
// libc, and names it in .preload.
func (b *Builder) installStorefix() error {
	// ⛔ A musl CLOSURE IS NOT SERVED BY THIS OBJECT and saying so is the
	// whole point of checking. It is compiled against the build host's glibc;
	// pointing it at a musl loader would fail at run time with a message about
	// a symbol rather than about the mechanism.
	if _, err := os.Stat(filepath.Join(b.AppDir, "lib")); err != nil {
		return fmt.Errorf("the bundle has no lib/")
	}
	if libs, _ := filepath.Glob(filepath.Join(b.AppDir, "lib", "ld-musl-*")); len(libs) > 0 {
		return fmt.Errorf("this closure is musl-based; the interposer is built against glibc")
	}
	src, err := materialiseAppRunSource(b.C.RuntimeSrcDir())
	if err != nil {
		return err
	}
	src = filepath.Join(filepath.Dir(src), "pgb-storefix.c")
	out := filepath.Join(b.AppDir, "lib", storefixSO)
	r, err := (&proc.Cmd{Argv: []string{"cc", "-O2", "-fPIC", "-shared", "-o", out, src},
		Subsys: "bundle"}).Output()
	if err != nil {
		return err
	}
	if r.Failed() {
		return fmt.Errorf("cc exited %d", r.Code)
	}
	if err := b.checkStorefixABI(out); err != nil {
		_ = os.Remove(out)
		return err
	}
	if err := appendPreload(filepath.Join(b.AppDir, ".preload"), storefixSO); err != nil {
		return err
	}
	fi, _ := os.Stat(out)
	logx.Say("storefix    %s, %d bytes, named in .preload", storefixSO, fi.Size())
	return nil
}

// checkStorefixABI asserts that every versioned symbol the interposer imports
// is one the BUNDLE's libc defines.
//
// ⛔ THE FAILURE THIS CATCHES IS SILENT OTHERWISE. The object is compiled by
// the build host's compiler against the build host's glibc; if that glibc is
// NEWER than the closure's, the loader on the target refuses the object with a
// version error and the bundle simply does not run. `experiments/73-` is the
// same measurement pointed the other way — what a static glibc can define for
// a host object — and this is its cheap, per-build form.
func (b *Builder) checkStorefixABI(so string) error {
	f, err := elf.Open(so)
	if err != nil {
		return err
	}
	defer f.Close()
	imports, err := f.ImportedSymbols()
	if err != nil {
		return err
	}
	defined := map[string]map[string]bool{}
	versions := map[string]map[string]bool{}
	var missing, tooNew []string
	for _, sym := range imports {
		lib := sym.Library
		if lib == "" {
			continue
		}
		if _, ok := defined[lib]; !ok {
			p := filepath.Join(b.AppDir, "lib", lib)
			// ⛔ .dynsym, NOT .symtab. `elfx.DefinedExternalSymbols` reads
			// `.symtab`, which is what an ARCHIVE has; a shipped libc.so.6 is
			// stripped and exports through `.dynsym` alone, so that reader
			// reported glibc as defining neither `dlsym` nor `fclose` and this
			// check refused every bundle it was asked about.
			d, v, derr := definedDynamic(p)
			if derr != nil {
				return fmt.Errorf("the bundle has no readable %s: %v", lib, derr)
			}
			defined[lib], versions[lib] = d, v
		}
		if !defined[lib][sym.Name] {
			missing = append(missing, sym.Name+" ("+lib+")")
			continue
		}
		if sym.Version != "" && !versions[lib][sym.Version] {
			tooNew = append(tooNew, sym.Name+"@"+sym.Version+" ("+lib+")")
		}
	}
	if len(missing) == 0 && len(tooNew) == 0 {
		logx.Say("storefix    %d imported symbol(s), all defined by the bundle's own libc",
			len(imports))
		return nil
	}
	sortStrings(missing)
	sortStrings(tooNew)
	if len(missing) > 0 {
		return fmt.Errorf("the bundle's libc does not define %s", strings.Join(missing, ", "))
	}
	return fmt.Errorf("the build host's glibc is NEWER than the closure's: %s",
		strings.Join(tooNew, ", "))
}

// definedDynamic reads what a SHARED OBJECT exports: the names in .dynsym that
// are defined here, and the symbol-version names it declares.
//
// ⚠ The version names live in .dynstr. A caller asking for `dlsym@GLIBC_2.34`
// is satisfied only if the library declares that version, which is exactly the
// check that catches a build host whose glibc is newer than the closure's.
func definedDynamic(path string) (names, vers map[string]bool, err error) {
	f, err := elf.Open(path)
	if err != nil {
		return nil, nil, err
	}
	defer f.Close()
	syms, err := f.DynamicSymbols()
	if err != nil {
		return nil, nil, err
	}
	names = map[string]bool{}
	for _, s := range syms {
		if s.Section == elf.SHN_UNDEF || s.Name == "" {
			continue
		}
		names[s.Name] = true
	}
	vers = map[string]bool{}
	if sec := f.Section(".dynstr"); sec != nil {
		if data, derr := sec.Data(); derr == nil {
			for _, s := range strings.Split(string(data), "\x00") {
				if strings.HasPrefix(s, "GLIBC_") || strings.HasPrefix(s, "GCC_") ||
					strings.HasPrefix(s, "GLIBCXX_") || strings.HasPrefix(s, "CXXABI_") {
					vers[s] = true
				}
			}
		}
	}
	return names, vers, nil
}

// preloadNames reads .preload, which is sharun's own mechanism: the objects it
// hands the loader as --preload. Nothing links against them, so a reachability
// sweep has to be told they are roots.
func preloadNames(appDir string) []string {
	data, err := os.ReadFile(filepath.Join(appDir, ".preload"))
	if err != nil {
		return nil
	}
	var out []string
	for _, l := range strings.Split(string(data), "\n") {
		if l = strings.TrimSpace(l); l != "" && !strings.HasPrefix(l, "#") {
			out = append(out, l)
		}
	}
	return out
}

func appendPreload(path, name string) error {
	if data, err := os.ReadFile(path); err == nil {
		for _, l := range strings.Split(string(data), "\n") {
			if strings.TrimSpace(l) == name {
				return nil
			}
		}
		return os.WriteFile(path, append(data, []byte(name+"\n")...), 0o644)
	}
	return os.WriteFile(path, []byte(name+"\n"), 0o644)
}

// rewriteTextStorePaths handles the two text forms, each by the rule that
// actually applies to it.
//
// ⛔ SCRIPTS ARE NOT ON THIS LIST, AND THAT IS DELIBERATE. A Python program's
// `sys.path` entry or data path reaches libc through CPython, so the
// interposer resolves it at run time; rewriting the script's text would have
// to name a mount point that does not exist until the AppImage runs.
//
//	.env       ${SHARUN_DIR} is what sharun expands, so a store path becomes
//	           one — and liftWrapperEnv's own rewrite already produces the same
//	           store/<name-version> shape the farm answers.
//	.desktop   ⛔ NOT ${SHARUN_DIR}: a desktop entry is read by a file manager,
//	           which expands nothing. Icon= becomes the bundled icon's NAME,
//	           DBusActivatable goes (a bundle cannot be D-Bus activated), and
//	           any other store path is REPORTED rather than mangled.
func (b *Builder) rewriteTextStorePaths(entries []StoreMapEntry) {
	byBase := map[string]string{}
	for _, e := range entries {
		byBase[e.Base] = e.Dir
	}
	refs, changed := 0, 0
	for _, f := range globAll(b.AppDir, ".env") {
		data, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		orig := string(data)
		out := storeRefRe.ReplaceAllStringFunc(orig, func(m string) string {
			rest := strings.TrimPrefix(m, "/nix/store/")
			base, tail, _ := strings.Cut(rest, "/")
			dir, ok := byBase[base]
			if !ok {
				return m
			}
			refs++
			if tail == "" {
				return "${SHARUN_DIR}/" + dir
			}
			return "${SHARUN_DIR}/" + dir + "/" + tail
		})
		if out != orig {
			_ = os.WriteFile(f, []byte(out), 0o644)
			changed++
		}
	}
	if changed > 0 {
		logx.Say("env paths   %d store reference(s) in .env rewritten into the bundle", refs)
	}
	b.fixDesktopEntries(byBase)
}

// fixDesktopEntries applies the three rules §5 of docs/design/store-paths.md
// names, and reports what it did not touch.
func (b *Builder) fixDesktopEntries(byBase map[string]string) {
	icon := b.bundledIconName()
	for _, f := range globAll(b.AppDir, "*.desktop", "share/applications/*.desktop") {
		data, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		var out []string
		dropped, left, inClosure := 0, 0, 0
		for _, l := range strings.Split(string(data), "\n") {
			switch {
			case strings.HasPrefix(l, "DBusActivatable="):
				// ⛔ A BUNDLE CANNOT BE D-BUS ACTIVATED. The name a desktop
				// entry claims is not on the session bus, so a launcher that
				// believes this waits for a service that never appears.
				dropped++
				continue
			case strings.HasPrefix(l, "Icon=") && icon != "":
				out = append(out, "Icon="+icon)
				continue
			}
			// ⛔ THE TWO CASES ARE DIFFERENT FINDINGS. A store path that IS
			// in the closure means the bundler should have rewritten this
			// key and did not; one that is NOT means the entry points off
			// the bundle entirely. Reporting them as one number would hide
			// the first behind the second.
			if strings.Contains(l, "/nix/store/") {
				for _, m := range storeRefRe.FindAllString(l, -1) {
					base, _, _ := strings.Cut(strings.TrimPrefix(m, "/nix/store/"), "/")
					if _, ok := byBase[base]; ok {
						inClosure++
					} else {
						left++
					}
				}
			}
			out = append(out, l)
		}
		_ = os.WriteFile(f, []byte(strings.Join(out, "\n")), 0o644)
		if dropped > 0 {
			logx.Say("desktop     %s: DBusActivatable removed (a bundle cannot be activated)",
				filepath.Base(f))
		}
		if inClosure > 0 {
			logx.Warnf("%s names %d store path(s) that ARE in this bundle and were not rewritten",
				filepath.Base(f), inClosure)
		}
		if left > 0 {
			logx.Warnf("%s names %d store path(s) that are NOT in this bundle at all",
				filepath.Base(f), left)
		}
	}
}

// bundledIconName is the icon the bundle actually carries at its top level,
// which is the only name a package manager can resolve.
//
// ⚠ THE PROGRAM'S OWN NAME WINS. Taking the first top-level image would be a
// proxy for nothing — the same mistake `findIcon`'s comment calls out about
// choosing an icon by path length — and a bundle can legitimately carry more
// than one.
func (b *Builder) bundledIconName() string {
	entries, err := os.ReadDir(b.AppDir)
	if err != nil {
		return ""
	}
	first := ""
	for _, e := range entries {
		n := e.Name()
		if n == ".DirIcon" || e.IsDir() {
			continue
		}
		if !strings.HasSuffix(n, ".png") && !strings.HasSuffix(n, ".svg") {
			continue
		}
		stem := strings.TrimSuffix(strings.TrimSuffix(n, ".png"), ".svg")
		if stem == b.Prog {
			return stem
		}
		if first == "" {
			first = stem
		}
	}
	return first
}

func globAll(dir string, pats ...string) []string {
	var out []string
	for _, g := range pats {
		files, _ := filepath.Glob(filepath.Join(dir, g))
		out = append(out, files...)
	}
	return out
}

// reportStorePaths says what is compiled into the shipped binaries, how many
// of those paths this bundle can resolve, and which it cannot.
//
// ⛔ THE COUNT IS THE DELIVERABLE. Before T-081 a bundle with a compiled-in
// path built cleanly, packed cleanly and drew nothing, and the first sign of
// trouble was a user double-clicking it.
func (b *Builder) reportStorePaths(entries []StoreMapEntry) {
	known := map[string]bool{}
	for _, e := range entries {
		known[e.Base] = true
	}
	var files []string
	for _, d := range []string{filepath.Join(b.AppDir, "shared", "bin"), filepath.Join(b.AppDir, "lib")} {
		es, err := os.ReadDir(d)
		if err != nil {
			continue
		}
		for _, e := range es {
			if !e.IsDir() {
				files = append(files, filepath.Join(d, e.Name()))
			}
		}
	}
	inBundle := map[string]bool{}
	outside := map[string]bool{}
	for _, f := range files {
		data, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		for _, m := range storeRefRe.FindAllString(string(data), -1) {
			base, _, _ := strings.Cut(strings.TrimPrefix(m, "/nix/store/"), "/")
			if known[base] {
				inBundle[base] = true
			} else {
				outside[base] = true
			}
		}
	}
	if len(inBundle) == 0 && len(outside) == 0 {
		logx.Say("store paths no binary in this bundle names a /nix/store path")
		return
	}
	logx.Say("store paths %s compiled in: %d resolve inside the bundle, %d do not",
		strconv.Itoa(len(inBundle)+len(outside)), len(inBundle), len(outside))
	if len(outside) > 0 {
		var list []string
		for k := range outside {
			list = append(list, k)
		}
		sort.Strings(list)
		for _, s := range list {
			// ⛔ A FINDING, NOT A SUBSTITUTION. The field's cascade maps this
			// to /usr/local/bin, which is a bet that the host has the program.
			logx.Warnf("compiled-in store path with NO target in this bundle: /nix/store/%s", s)
		}
	}
}
