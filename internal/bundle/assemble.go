// assemble.go — the AppDir, in the shape sharun wants.
//
// The layout is the Anylinux fork's, not upstream sharun's, and they differ:
//
//	AppDir/lib          the libraries AND the dynamic loader
//	AppDir/<name>       a hardlink of sharun, one per program, at the TOP level
//	AppDir/shared/bin   the real ELF binaries
//	AppDir/shared/lib   a symlink to ../lib
//
// Building it upstream's way — libraries under shared/lib — produces an
// AppImage that mounts, starts sharun, and prints "Interpreter not found!",
// because sharun looks for the loader in $SHARUN_DIR/lib.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/elfx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

// ⚠ THE NINE-NAME WHITELIST THAT USED TO BE HERE IS GONE, and history/
// carries why: `copyLibraries` now takes every directory under any store
// path's lib/, because a list of the ones somebody had hit is the same shape
// as the regex cascade T-081 exists to replace. See copyLibraries.

// reservedNames are layout entries a program must not overwrite.
var reservedNames = map[string]bool{
	"AppRun": true, "sharun": true, "bin": true, "lib": true, "lib32": true,
	"share": true, "shared": true, "store": true, ".env": true,
}

func (b *Builder) assemble() error {
	if err := os.RemoveAll(b.AppDir); err != nil {
		return err
	}
	for _, d := range []string{"shared/bin", "lib", "bin", "share"} {
		if err := os.MkdirAll(filepath.Join(b.AppDir, d), 0o755); err != nil {
			return err
		}
	}
	_ = os.Remove(filepath.Join(b.AppDir, "shared", "lib"))
	if err := os.Symlink("../lib", filepath.Join(b.AppDir, "shared", "lib")); err != nil {
		return err
	}

	entryDir := filepath.Join(b.Root, b.Base)
	entry, err := b.resolveEntry(entryDir, b.Prog, b.O.Name != "")
	if err != nil {
		return fail.Ran("no entry point in %s/bin", b.Base)
	}
	logx.Say("entry       %s", entry.ELF)
	if err := b.installProgram(b.Prog, entry); err != nil {
		return fail.Ran("could not install the entry point: %v", err)
	}

	// Every other program in the same bin/, because an application is often a
	// set and a bundle with only the main window cannot be compared against
	// one that can render. They cost almost nothing: the libraries they share
	// are already in lib/.
	extra := 0
	if entries, err := os.ReadDir(filepath.Join(entryDir, "bin")); err == nil {
		for _, e := range entries {
			// ⚠ A DOT-NAMED FILE IN bin/ IS NOT A PROGRAM. `.meld-wrapped` is
			// the target a makeBinaryWrapper execs, and installing it as a
			// second program gave the bundle two entry points for one
			// application — and, once a script could resolve, a second static
			// trampoline and a second copy of the script.
			if e.IsDir() || e.Name() == b.Prog || strings.HasPrefix(e.Name(), ".") {
				continue
			}
			r, err := b.resolveEntry(entryDir, e.Name(), false)
			if err != nil || r.ELF == "" {
				continue
			}
			if b.installProgram(e.Name(), r) == nil {
				extra++
			}
		}
	}
	// And any program named with --with-program, found anywhere in the
	// closure: an application's helper often lives in a dependency's store
	// path. The shallowest match wins and the path it came from is printed,
	// because two store paths can carry the same program name.
	for _, w := range b.O.WithPrograms {
		if w == "" {
			continue
		}
		if _, err := os.Stat(filepath.Join(b.AppDir, "shared", "bin", w)); err == nil {
			continue
		}
		p := b.findProgram(w)
		if p == "" {
			logx.Warnf("--with-program %s: no such program in the closure", w)
			continue
		}
		owner := filepath.Dir(filepath.Dir(p))
		e := Entry{ELF: p}
		if r, err := b.resolveEntry(owner, w, false); err == nil && r.ELF != "" {
			e = r
		}
		if b.installProgram(w, e) != nil {
			continue
		}
		logx.Say("program     %s  <- %s", w, filepath.Base(owner))
		extra++
	}
	// ⭐ gst-plugin-scanner IS A PROGRAM, NOT A DATA FILE, and that is the
	// whole reason it is installed here rather than pointed at in place.
	// GStreamer runs it as a child process; the copy merged into the bundle's
	// libexec/ is an ordinary dynamic ELF, so starting it directly would bring
	// up the HOST loader and the host libc inside a bundle whose whole claim
	// is that it does not. ⚠ It is also why a host-object count taken on a
	// GStreamer subject has to say WHICH process it counted.
	//
	// ⭐ The field reaches the same answer from the other side: quick-sharun's
	// `_handle_bins_scripts` hardlinks sharun over every `gst-*` binary and
	// puts it in the gstreamer libdir, because sharun only sets
	// GST_PLUGIN_SCANNER when the scanner sits beside the plugins.
	if n := b.installGstScanner(); n > 0 {
		extra += n
	}

	if extra > 0 {
		logx.Say("programs    %s + %d more", b.Prog, extra)
	}

	if err := b.copyLibraries(); err != nil {
		return err
	}
	if err := b.copyLoader(); err != nil {
		return err
	}

	// Absolute DT_NEEDED entries are rewritten to base names: an absolute
	// DT_NEEDED is opened as a path and the loader never consults
	// --library-path for it, so the bundle dies on a library sitting in the
	// same directory.
	logx.Say("rewriting absolute DT_NEEDED entries to basenames")
	rewritten := 0
	targets, _ := filepath.Glob(filepath.Join(b.AppDir, "lib", "*.so*"))
	targets = append(targets, filepath.Join(b.AppDir, "shared", "bin", b.Prog))
	for _, t := range targets {
		res, err := elfx.Shorten(t)
		if err != nil {
			continue
		}
		rewritten += len(res.Changed)
	}
	logx.Say("patched     %d absolute DT_NEEDED entries", rewritten)
	return nil
}

// installGstScanner installs gst-plugin-scanner as a bundle program and names
// it in GST_PLUGIN_SCANNER. It returns how many were installed (0 or 1), so a
// closure with no GStreamer in it costs nothing and says nothing.
//
// ⛔ WHY THE SEARCH IS libexec/ AND NOT lib/. Anylinux-sharun sets this
// variable itself, but only for `<library_path>/gstreamer-*/gst-plugin-scanner`
// — the scanner sitting BESIDE the plugins. nixpkgs puts it in
// `libexec/gstreamer-1.0/` instead, so that test cannot succeed on a nixpkgs
// closure however the plugin directory is laid out. Nothing sets it unless
// this does.
//
// ⚠ THE VALUE POINTS AT bin/, NOT shared/bin/. `bin/<name>` is the sharun
// hardlink that sets the library path and runs the bundled loader;
// `shared/bin/<name>` is the raw payload, and naming that would reintroduce
// exactly the host loader this function exists to keep out. It is the same
// distinction pgb-apprun.c makes for the same reason.
func (b *Builder) installGstScanner() int {
	name := gstScannerName
	if _, err := os.Stat(filepath.Join(b.AppDir, "shared", "bin", name)); err == nil {
		return 0
	}
	// The merged libexec/ tree is searched first because that is where the
	// bundle's own copy lands; the closure is the fallback for a layout that
	// did not merge.
	var found string
	for _, root := range []string{filepath.Join(b.AppDir, "libexec"), b.Root} {
		matches, _ := filepath.Glob(filepath.Join(root, "gstreamer-*", name))
		if len(matches) == 0 {
			matches, _ = filepath.Glob(filepath.Join(root, "*", "libexec", "gstreamer-*", name))
		}
		if len(matches) > 0 {
			found = matches[0]
			break
		}
	}
	if found == "" {
		return 0
	}
	if err := b.installProgram(name, Entry{ELF: found}); err != nil {
		logx.Warnf("gst-plugin-scanner found at %s but not installed: %v", found, err)
		return 0
	}
	// ⛔ THE VARIABLE IS NOT EMITTED HERE. writeEnv() names it, keyed on
	// `shared/bin/<gstScannerName>` existing -- the same "set it only if the
	// bundle actually has it" rule every other line there follows. Emitting it
	// from this side would be a second place that has to agree about the name,
	// which is the coupling class T-092 is about.
	logx.Say("gstreamer   scanner installed as a program (GST_PLUGIN_SCANNER follows)")
	return 1
}

// installProgram puts one program in shared/bin under the name the bundle
// dispatches on.
//
// ⭐ A SCRIPT ENTRY POINT BECOMES THREE FILES, and this is T-081's second
// blocker resolved: the interpreter (an ELF already in the closure) under its
// own name so sharun can start it with the bundled library path, the script
// under shared/script/, and a static trampoline at shared/bin/<name> that
// joins them. ⛔ sharun reads shared/bin/<name> as an ELF and exits when it is
// not one, so the trampoline is what makes the layout legal — there is no
// arrangement of a script alone that sharun will start.
func (b *Builder) installProgram(name string, e Entry) error {
	dst := filepath.Join(b.AppDir, "shared", "bin", name)
	if e.Script == "" {
		return copyResolved(e.ELF, dst, 0o755)
	}
	interp := filepath.Base(e.ELF)
	if interp == name {
		// An interpreter that shares the program's name would be overwritten
		// by its own trampoline.
		interp = name + "-interp"
	}
	if err := copyResolved(e.ELF, filepath.Join(b.AppDir, "shared", "bin", interp), 0o755); err != nil {
		return err
	}
	rel := filepath.Join("shared", "script", name)
	if err := copyResolved(e.Script, filepath.Join(b.AppDir, rel), 0o755); err != nil {
		return err
	}
	if err := b.buildTrampoline(dst, interp, rel); err != nil {
		// ⚠ REPORTED, NOT SILENT. Without a trampoline this program cannot be
		// started at all, and a bundle that quietly drops its entry point is
		// the failure mode T-081 exists to end.
		logx.Warnf("no trampoline could be built for the script entry %s: %v", name, err)
		return err
	}
	logx.Say("script      %s = %s + %s (static trampoline)", name, interp, rel)
	return nil
}

// buildTrampoline compiles pgb-exec.c with the interpreter and script baked in.
func (b *Builder) buildTrampoline(out, interp, script string) error {
	src, err := materialiseAppRunSource(b.C.RuntimeSrcDir())
	if err != nil {
		return err
	}
	src = filepath.Join(filepath.Dir(src), "pgb-exec.c")
	r, err := (&proc.Cmd{Argv: []string{"cc", "-O2", "-static",
		"-DPGB_EXEC_INTERP=\"" + interp + "\"",
		"-DPGB_EXEC_SCRIPT=\"" + script + "\"",
		"-o", out, src}, Subsys: "bundle"}).Output()
	if err != nil {
		return err
	}
	if r.Failed() {
		return fmt.Errorf("cc exited %d", r.Code)
	}
	info, err := elfx.Inspect(out)
	if err != nil {
		return err
	}
	if info.Interp != "" {
		// A dynamic trampoline would put the HOST's loader in the bundle.
		_ = os.Remove(out)
		return fmt.Errorf("the compiler produced a DYNAMIC trampoline")
	}
	return nil
}

func (b *Builder) findProgram(name string) string {
	matches, _ := filepath.Glob(filepath.Join(b.Root, "*", "bin", name))
	for _, m := range matches {
		if fi, err := os.Stat(m); err == nil && !fi.IsDir() {
			return m
		}
	}
	found := ""
	_ = filepath.Walk(b.Root, func(p string, fi os.FileInfo, err error) error {
		if err != nil || found != "" || fi.IsDir() {
			return nil
		}
		if fi.Name() == name && fi.Mode()&0o111 != 0 {
			found = p
		}
		return nil
	})
	return found
}

// copyLibraries takes every shared object in the closure, not the ones ldd
// happens to name, and keeps the 32-bit half apart: a flat directory holding
// an i386 and an x86_64 libfoo.so.1 gives the loader whichever landed first.
func (b *Builder) copyLibraries() error {
	lib := filepath.Join(b.AppDir, "lib")
	lib32 := filepath.Join(b.AppDir, "lib32")
	n32 := 0
	err := filepath.Walk(b.Root, func(p string, fi os.FileInfo, err error) error {
		if err != nil || fi.IsDir() || !fi.Mode().IsRegular() {
			return nil
		}
		if !IsSharedObject(fi.Name()) {
			return nil
		}
		class := elfClass(p)
		dst := filepath.Join(lib, fi.Name())
		if class == 32 {
			if err := os.MkdirAll(lib32, 0o755); err != nil {
				return err
			}
			dst = filepath.Join(lib32, fi.Name())
			n32++
		}
		if _, err := os.Lstat(dst); err == nil {
			return nil
		}
		_ = copyResolved(p, dst, 0o755)
		return nil
	})
	if err != nil {
		return err
	}

	// Symlinks too: libfoo.so.6 -> libfoo.so.6.0.1 is how a DT_NEEDED
	// resolves. The pass repeats because a closure has symlinks TO symlinks
	// and one pass is order-dependent.
	for range 6 {
		made := 0
		_ = filepath.Walk(b.Root, func(p string, fi os.FileInfo, err error) error {
			if err != nil || fi.IsDir() {
				return nil
			}
			lfi, err := os.Lstat(p)
			if err != nil || lfi.Mode()&os.ModeSymlink == 0 || !strings.Contains(lfi.Name(), ".so") {
				return nil
			}
			target, err := os.Readlink(p)
			if err != nil {
				return nil
			}
			name := lfi.Name()
			if _, err := os.Lstat(filepath.Join(lib, name)); err == nil {
				return nil
			}
			if _, err := os.Stat(filepath.Join(lib, filepath.Base(target))); err != nil {
				return nil
			}
			if os.Symlink(filepath.Base(target), filepath.Join(lib, name)) == nil {
				made++
			}
			return nil
		})
		if made == 0 {
			break
		}
	}

	// ⭐ EVERY DIRECTORY UNDER ANY STORE PATH'S lib/, NOT A WHITELIST OF NINE.
	//
	// ⛔ THE WHITELIST WAS A LIST OF THE ONES SOMEBODY HAD HIT. It is the same
	// shape as the field's regex cascade — a pattern that grows one entry per
	// bug report — and T-081's rule is that the closure decides, not a list.
	// Two things it was silently missing: `lib/gconv`, which `.env` sets
	// GCONV_PATH to and which therefore pointed at a directory that did not
	// exist, and an interpreter's own library tree (`lib/python3.14`), without
	// which a script entry point resolves and then finds no stdlib.
	// ⚠ The cost is bounded: `include`, `pkgconfig`, `cmake` and `aclocal` are
	// already dropped at `--debloat safe`, which is the default.
	subdirs := map[string]bool{}
	matches, _ := filepath.Glob(filepath.Join(b.Root, "*", "lib", "*"))
	for _, d := range matches {
		if fi, err := os.Stat(d); err != nil || !fi.IsDir() {
			continue
		}
		name := filepath.Base(d)
		subdirs[name] = true
		_ = copyTreeNoClobber(d, filepath.Join(lib, name))
	}
	if len(subdirs) > 0 {
		logx.Say("lib trees   %d directories under lib/ carried whole", len(subdirs))
	}
	entries, _ := os.ReadDir(lib)
	logx.Say("libraries   %d from the closure", len(entries))
	if n32 > 0 {
		logx.Say("lib32       %d 32-bit objects", n32)
	}
	return nil
}

// copyLoader takes the closure's own loader. A loader from a different glibc
// than the libraries beside it is the exact pairing that fails.
// glibcVerRe reads a glibc version out of the names the closure uses for it:
// `ld-2.26.so`, `libc-2.26.so`, or the store path `…-glibc-2.26-115`.
var glibcVerRe = regexp.MustCompile(`(?:^|/|-)(?:ld|libc|glibc)-([0-9]+)\.([0-9]+)`)

// checkLoaderOptions warns when the closure's loader is too old for the
// command line sharun builds for it.
//
// ⛔ THE SYMPTOM THIS EXISTS TO NAME, because it is unreadable otherwise.
// sharun starts a dynamic payload as
//
//	<loader> --library-path <path> --argv0 <arg0> [--preload …] <bin> <args>
//
// and `ld.so` only learned to parse options in glibc 2.30 (`--preload`) and
// 2.33 (`--argv0`). An OLDER loader has no option parsing at all and takes the
// first argument as the program to run, so the bundle dies with
//
//	--argv0: error while loading shared libraries: --argv0: cannot open
//	shared object file: No such file or directory
//
// ⭐ MEASURED on `neovim`, whose nixpkgs closure carries glibc 2.26: the
// loader rejects even `--version` the same way. ⚠ The same closure also
// triggers the interposer's "libc does not define dladdr, dlsym" warning, and
// that is the SAME root cause — those symbols lived in `libdl.so` until glibc
// 2.34. One old glibc, two unrelated-looking messages.
//
// ⚠ IT WARNS RATHER THAN REFUSING, and the reason is real rather than
// timidity: sharun skips the loader invocation entirely for a static or
// already-patched payload (`is_static_bin`, `is_patched_bin`), so a closure
// with an old glibc and such an entry point still works. Refusing would be
// wrong for those; saying nothing was wrong for this one.
func (b *Builder) checkLoaderOptions(ld string) {
	const needMajor, needMinor = 2, 33
	// The loader's own name first (ld-linux-… is a symlink to ld-<ver>.so),
	// then the full path, which carries the `glibc-<ver>` store path.
	maj, min := glibcVersionFrom(filepath.Base(resolveLink(ld)))
	if maj == 0 {
		maj, min = glibcVersionFrom(ld)
	}
	if maj == 0 {
		return // musl, or a name that carries no version: nothing to say
	}
	if maj > needMajor || (maj == needMajor && min >= needMinor) {
		return
	}
	logx.Warnf("the closure's loader is glibc %d.%d, and sharun starts a dynamic", maj, min)
	logx.Warnf("   payload with `--argv0`, which ld.so only understands from 2.33.")
	logx.Warnf("   An older loader takes the first option as the PROGRAM, so this")
	logx.Warnf("   bundle will very likely die with:")
	logx.Warnf("     --argv0: error while loading shared libraries: --argv0: ...")
	logx.Warnf("   ⚠ Unless its entry point is static or already patched, which")
	logx.Warnf("   sharun starts without the loader command line.")
}

// glibcVersionFrom reads a glibc major.minor out of a name, or 0,0 when there
// is none — which is the musl case and the case of a loader whose name carries
// no version at all.
func glibcVersionFrom(name string) (int, int) {
	m := glibcVerRe.FindStringSubmatch(name)
	if m == nil {
		return 0, 0
	}
	maj, err1 := strconv.Atoi(m[1])
	min, err2 := strconv.Atoi(m[2])
	if err1 != nil || err2 != nil {
		return 0, 0
	}
	return maj, min
}

// resolveLink follows one symlink hop, which is how a closure names its loader:
// ld-linux-x86-64.so.2 -> ld-2.26.so.
func resolveLink(p string) string {
	if t, err := os.Readlink(p); err == nil {
		if filepath.IsAbs(t) {
			return t
		}
		return filepath.Join(filepath.Dir(p), t)
	}
	return p
}

func (b *Builder) copyLoader() error {
	ld := b.findFile(func(name string) bool {
		return strings.HasPrefix(name, "ld-linux-") && strings.Contains(name, ".so.") ||
			strings.HasPrefix(name, "ld-musl-") && strings.Contains(name, ".so.")
	})
	if ld == "" {
		return fail.Ran("the closure carries no dynamic loader")
	}
	if err := copyResolved(ld, filepath.Join(b.AppDir, "lib", filepath.Base(ld)), 0o755); err != nil {
		return err
	}
	logx.Say("loader      %s (the closure's own, never the host's)", filepath.Base(ld))
	b.checkLoaderOptions(ld)

	if fi, err := os.Stat(filepath.Join(b.AppDir, "lib32")); err == nil && fi.IsDir() {
		// The 32-bit half needs its own loader, and it is a different file
		// with a different name.
		ld32 := b.findFile(func(name string) bool {
			return name == "ld-linux.so.2" || name == "ld-linux-armhf.so.3"
		})
		if ld32 == "" {
			logx.Warnf("the closure has 32-bit objects but no 32-bit loader; lib32 will not run")
		} else {
			_ = copyResolved(ld32, filepath.Join(b.AppDir, "lib32", filepath.Base(ld32)), 0o755)
			logx.Say("loader32    %s", filepath.Base(ld32))
		}
		_ = os.Remove(filepath.Join(b.AppDir, "shared", "lib32"))
		_ = os.Symlink("../lib32", filepath.Join(b.AppDir, "shared", "lib32"))
	}
	return nil
}

func (b *Builder) findFile(match func(name string) bool) string {
	found := ""
	_ = filepath.Walk(b.Root, func(p string, fi os.FileInfo, err error) error {
		if err != nil || fi.IsDir() || found != "" {
			return nil
		}
		if match(fi.Name()) {
			found = p
		}
		return nil
	})
	return found
}

// elfClass reads byte 5 of the header: 1 = 32-bit, 2 = 64-bit.
func elfClass(path string) int {
	f, err := os.Open(path)
	if err != nil {
		return 0
	}
	defer f.Close()
	var h [5]byte
	if n, err := f.Read(h[:]); err != nil || n != 5 {
		return 0
	}
	if h[0] != 0x7f || h[1] != 'E' || h[2] != 'L' || h[3] != 'F' {
		return 0
	}
	switch h[4] {
	case 1:
		return 32
	case 2:
		return 64
	}
	return 0
}

// copyResolved copies a file, following symlinks through the closure first.
func copyResolved(src, dst string, mode os.FileMode) error {
	real, err := filepath.EvalSymlinks(src)
	if err != nil {
		real = src
	}
	data, err := os.ReadFile(real)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return err
	}
	return os.WriteFile(dst, data, mode)
}

// copyTreeNoClobber copies a directory tree, resolving symlinks and leaving
// anything already at the destination alone.
func copyTreeNoClobber(src, dst string) error {
	return filepath.Walk(src, func(p string, fi os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		rel, err := filepath.Rel(src, p)
		if err != nil {
			return nil
		}
		target := filepath.Join(dst, rel)
		if fi.IsDir() {
			return os.MkdirAll(target, 0o755)
		}
		if _, err := os.Lstat(target); err == nil {
			return nil
		}
		mode := fi.Mode().Perm()
		if mode&0o111 != 0 {
			mode = 0o755
		}
		_ = copyResolved(p, target, mode)
		return nil
	})
}

// icdLibraryPath matches the absolute store path an ICD JSON names.
var icdLibraryPath = regexp.MustCompile(`("library_path"\s*:\s*")/nix/store/[^"]*/([^/"]+)"`)

// storePathLine matches a whole line that is nothing but an absolute store
// path. An OpenCL vendor .icd is not JSON at all: it is one library per line,
// and nixpkgs writes the store path there.
var storePathLine = regexp.MustCompile(`(?m)^[ \t]*(/nix/store/\S*/([^/\s]+))[ \t]*$`)

// rewriteManifestPaths turns every absolute store path inside the bundle's
// vendor and ICD manifests into the bare soname beside it, and returns how
// many files it looked at.
//
// ⛔ IT ITERATES THE SWEEP'S OWN manifestGlobs, AND THAT IS THE POINT. These
// two lists were separate and had drifted: the sweep knew about
// share/vulkan/explicit_layer.d and etc/OpenCL/vendors and the rewrite did
// not, so a layer's library was correctly kept as a reachability root and
// then kept a `/nix/store` path inside the file that named it. ⚠ That is
// worse than either mistake alone — the bundle carries the library, the
// manifest points somewhere that does not exist on the target, and the
// integrity check sees nothing wrong because no DT_NEEDED is involved. One
// list means the two rules cannot disagree about which files matter.
func rewriteManifestPaths(appDir string) int {
	n := 0
	for _, pat := range manifestGlobs {
		files, _ := filepath.Glob(filepath.Join(appDir, pat))
		for _, f := range files {
			data, err := os.ReadFile(f)
			if err != nil {
				continue
			}
			out := icdLibraryPath.ReplaceAll(data, []byte(`${1}${2}"`))
			// A bare-path line is rewritten only when what it names looks
			// like a shared object; a manifest that lists a data file by
			// path is left exactly as it was.
			out = storePathLine.ReplaceAllFunc(out, func(m []byte) []byte {
				g := storePathLine.FindSubmatch(m)
				if g == nil || !IsSharedObject(string(g[2])) {
					return m
				}
				return g[2]
			})
			if string(out) != string(data) {
				_ = os.WriteFile(f, out, 0o644)
			}
			n++
		}
	}
	return n
}

func (b *Builder) desktopAndIcon() error {
	// Every share/, etc/ and libexec/ tree in the closure, merged.
	//
	// ⚠ IT WAS share/ ALONE UNTIL T-081. A store path's compiled-in reference
	// to its own etc/ or libexec/ had nowhere in the bundle to resolve to, so
	// the store map could not answer it — storefix.go's mergedFor is the table
	// that says where each of these went, and it and this list are one rule.
	b.copyClosureTrees()

	// The manifests name an absolute store path, which is the last hop of the
	// OpenGL problem: libglvnd finds the vendor file, opens the path it names
	// and fails, on a bundle that has the library sitting in lib/ beside it.
	// Rewritten to the bare soname, which the loader resolves.
	if n := rewriteManifestPaths(b.AppDir); n > 0 {
		logx.Say("icd json    %d rewritten to bare sonames", n)
	}

	// The application's own store path is searched FIRST: a closure carries
	// every dependency's share/ too, and taking the first .desktop in the
	// merged tree has advertised a bundle as GTK's demo.
	desktop := ""
	if files, _ := filepath.Glob(filepath.Join(b.Root, b.Base, "share", "applications", "*.desktop")); len(files) > 0 {
		desktop = b.storeResolve(files[0])
	}
	if desktop == "" {
		for _, pat := range []string{b.Prog + ".desktop", "*" + b.Prog + "*.desktop"} {
			if files, _ := filepath.Glob(filepath.Join(b.AppDir, "share", "applications", pat)); len(files) > 0 {
				desktop = files[0]
				break
			}
		}
	}
	iconName := b.Prog
	dst := filepath.Join(b.AppDir, b.Prog+".desktop")
	version := storeVersion(b.Base)
	if desktop != "" {
		if err := copyResolved(desktop, dst, 0o644); err != nil {
			logx.Warnf("the .desktop entry %s could not be copied; generating one instead", desktop)
			desktop = ""
		}
	}
	if desktop != "" {
		data, _ := os.ReadFile(dst)
		lines := strings.Split(string(data), "\n")
		for i, l := range lines {
			switch {
			case strings.HasPrefix(l, "Icon="):
				iconName = strings.TrimPrefix(l, "Icon=")
			case strings.HasPrefix(l, "Exec="):
				// sharun resolves the program from the desktop entry, so an
				// Exec naming a binary that is not in shared/bin produces a
				// bundle that mounts, starts, and reports a missing file.
				rest := ""
				if i := strings.IndexByte(l, ' '); i >= 0 {
					rest = l[i:]
				}
				lines[i] = "Exec=" + b.Prog + rest
			case strings.HasPrefix(l, "TryExec="):
				lines[i] = "TryExec=" + b.Prog
			}
		}
		lines = withAppImageVersion(lines, version)
		_ = os.WriteFile(dst, []byte(strings.Join(lines, "\n")), 0o644)
		logx.Say("desktop     %s  (Icon=%s, Exec rewritten to %s, X-AppImage-Version=%s)",
			filepath.Base(desktop), iconName, b.Prog, version)
	} else {
		// A generated entry is marked as generated: inventing one that claims
		// to be the application's own is worse than saying it was made up.
		logx.Warnf("the closure has no .desktop file; writing a minimal generated one")
		body := fmt.Sprintf(`[Desktop Entry]
Type=Application
Name=%s
Exec=%s
Icon=%s
Categories=Utility;
X-AppImage-Version=%s
Comment=generated by pgb bundle appimage -- the closure carried no desktop entry
`, b.Prog, b.Prog, b.Prog, version)
		if err := os.WriteFile(dst, []byte(body), 0o644); err != nil {
			return err
		}
	}

	// The icon is chosen by a stated POLICY -- iconRank -- not by path length
	// and not by "biggest wins", both of which are proxies for nothing.
	icon := b.findIcon(iconName)
	if icon != "" {
		icon = b.storeResolve(icon)
	}
	if icon != "" && copyResolved(icon, filepath.Join(b.AppDir, filepath.Base(icon)), 0o644) == nil {
		_ = copyResolved(icon, filepath.Join(b.AppDir, ".DirIcon"), 0o644)
		logx.Say("icon        %s", filepath.Base(icon))
	} else {
		// ⛔ A DANGLING Icon= IS WORSE THAN AN ABSENT ONE, and this was a named
		// gap in docs/research/bundle-capabilities.md §2: `jq`'s generated
		// entry said `Icon=jq` with no `jq.png` beside it, so every manager
		// that resolves the key found nothing and had nothing to fall back on.
		// Removing the key makes them use their own default.
		logx.Warnf("no icon named %q in the closure; Icon= is REMOVED rather than left dangling",
			iconName)
		dropDesktopKey(dst, "Icon=")
	}
	return b.writeUsrTree()
}

var resolutionDir = regexp.MustCompile(`/([0-9]+)x[0-9]+/`)

// iconRank orders the candidates for a desktop icon.
//
// ⛔ IT IS A POLICY, NOT AN ACCIDENT, and T-081's entry says which one:
// *"≥128×128, preferring 128 then 512 or 1024 — never a smaller bucket"*.
// ⚠ The field's own selectors are accidents rather than policy — one takes
// the first match, which makes its own sort dead code, and another takes the
// shortest path. This tree's previous rule (largest wins) was a third
// accident: a 1024×1024 icon is a slow, ugly downscale in a 48-pixel panel.
//
// Lower is better. The `<128` bucket is last on purpose and its use is
// reported, because taking a 16×16 icon silently is how a bundle ends up
// looking broken in a launcher.
func iconRank(path string, res int) int {
	switch {
	case res == 128:
		return 0
	case res == 512:
		return 1
	case res == 1024:
		return 2
	case res > 128:
		return 3
	case res == 0 && strings.Contains(path, "scalable"):
		return 4
	case res == 0:
		return 5 // share/pixmaps and friends: usually 48, never declared
	}
	return 6 // a declared bucket smaller than 128
}

func (b *Builder) findIcon(name string) string {
	roots := []string{
		filepath.Join(b.Root, b.Base, "share", "icons"),
		filepath.Join(b.Root, b.Base, "share", "pixmaps"),
		filepath.Join(b.AppDir, "share", "icons"),
		filepath.Join(b.AppDir, "share", "pixmaps"),
	}
	for _, root := range roots {
		if fi, err := os.Stat(root); err != nil || !fi.IsDir() {
			continue
		}
		best, bestRank, bestRes := "", 99, 0
		_ = filepath.Walk(root, func(p string, fi os.FileInfo, err error) error {
			if err != nil {
				return nil
			}
			base := fi.Name()
			if base != name+".png" && base != name+".svg" {
				return nil
			}
			res := 0
			if m := resolutionDir.FindStringSubmatch(p); m != nil {
				fmt.Sscanf(m[1], "%d", &res)
			}
			rank := iconRank(p, res)
			// Within the "bigger than 128 but not 512 or 1024" bucket the
			// SMALLER one wins: it is the closest to the size a launcher wants.
			if rank < bestRank || (rank == bestRank && rank == 3 && res < bestRes) {
				best, bestRank, bestRes = p, rank, res
			}
			return nil
		})
		if best != "" {
			if bestRank >= 5 {
				logx.Warnf("the only icon named %q is below 128x128 or undeclared: %s",
					name, filepath.Base(filepath.Dir(best)))
			}
			return best
		}
	}
	return ""
}

// writeUsrTree gives the bundle the `usr/` layout the AppImage managers hunt
// in, without the self-referential `usr -> .` symlink the field sometimes
// uses.
//
// ⛔ `usr -> .` IS A PATH LOOP: `usr/usr/usr/...` resolves forever, and this
// tree walks its own AppDir in the sweep, the debloater and the integrity
// check. A directory of three ordinary symlinks answers the same question and
// terminates. `AM` hunts `usr/share`/`share` × `22x22 … 512x512` for the icon
// named by `Icon=` (docs/research/bundle-capabilities.md §2).
func (b *Builder) writeUsrTree() error {
	usr := filepath.Join(b.AppDir, "usr")
	if err := os.MkdirAll(usr, 0o755); err != nil {
		return err
	}
	for name, target := range map[string]string{
		"share": "../share", "bin": "../shared/bin", "lib": "../lib",
	} {
		link := filepath.Join(usr, name)
		if _, err := os.Lstat(link); err == nil {
			continue
		}
		_ = os.Symlink(target, link)
	}
	return nil
}

// integrity checks that every DT_NEEDED of every ELF left in the bundle
// resolves inside it. It is a REPORT, not a refusal: a closure legitimately
// contains libraries that dlopen things nothing links against.
// envStoreRef matches what an override variable actually emits: a value under
// the bundle's own store farm. The name is the part that has to agree with
// what buildStoreFarm created.
var envStoreRef = regexp.MustCompile(`\$\{SHARUN_DIR\}/store/([^/:\s]+)(/[^:\s]*)?`)

// envIntegrity reads every value the bundle wrote into `.env` back against the
// tree it ships.
//
// ⛔ IT IS THE CHECK C28 FOUND MISSING. Two integrity passes existed and
// neither looked at `.env`: integrity() walks DT_NEEDED, manifestIntegrity()
// reads ICD manifests. So a variable naming a directory that does not exist
// was invisible until an application failed at a user's double-click — which
// is exactly what a short-name collision between farmDirName's two callers
// would have produced, silently, on the subset of closures that carry two
// builds of one package.
//
// ⚠ IT WARNS RATHER THAN FAILING, deliberately: `.env` legitimately names
// paths the debloater removed later, and a dangling override resolves to
// ENOENT, which is the answer the program would have had anyway. What must not
// happen is that nobody is told.
func (b *Builder) envIntegrity() {
	data, err := os.ReadFile(filepath.Join(b.AppDir, ".env"))
	if err != nil {
		return
	}
	seen := map[string]bool{}
	bad := 0
	checked := 0
	for _, line := range strings.Split(string(data), "\n") {
		key, value, ok := strings.Cut(line, "=")
		if !ok || strings.HasPrefix(line, "#") {
			continue
		}
		for _, m := range envStoreRef.FindAllStringSubmatch(value, -1) {
			rel := filepath.Join("store", m[1]+m[2])
			if seen[rel] {
				continue
			}
			seen[rel] = true
			checked++
			// The farm answers through a symlink to .root, so the target is
			// what matters rather than the link.
			if _, err := os.Stat(filepath.Join(b.AppDir, rel)); err != nil {
				logx.Warnf("`.env` %s names %s, which the bundle does not have", key, rel)
				bad++
			}
		}
	}
	if checked > 0 {
		logx.Say("env paths   %d store reference(s) checked against the tree, %d unresolved",
			checked, bad)
	}
}

func (b *Builder) integrity() {
	provided := map[string]bool{}
	for _, d := range []string{"lib", "lib32"} {
		entries, err := os.ReadDir(filepath.Join(b.AppDir, d))
		if err != nil {
			continue
		}
		for _, e := range entries {
			provided[e.Name()] = true
		}
	}
	missing := map[string]bool{}
	check := func(p string) {
		needed, err := elfx.Needed(p)
		if err != nil {
			return
		}
		for _, n := range needed {
			if strings.HasPrefix(n, "ld-linux") || strings.HasPrefix(n, "ld-musl") ||
				strings.HasPrefix(n, "/") {
				continue
			}
			if !provided[n] {
				missing[n] = true
			}
		}
	}
	if entries, err := os.ReadDir(filepath.Join(b.AppDir, "shared", "bin")); err == nil {
		for _, e := range entries {
			check(filepath.Join(b.AppDir, "shared", "bin", e.Name()))
		}
	}
	_ = filepath.Walk(filepath.Join(b.AppDir, "lib"), func(p string, fi os.FileInfo, err error) error {
		if err == nil && fi.Mode().IsRegular() && IsSharedObject(fi.Name()) {
			check(p)
		}
		return nil
	})
	if len(missing) == 0 {
		logx.Say("integrity   every DT_NEEDED in the bundle resolves inside it")
		return
	}
	var names []string
	for n := range missing {
		names = append(names, n)
	}
	sortStrings(names)
	logx.Warnf("%d DT_NEEDED name(s) do not resolve inside the bundle:", len(names))
	for _, n := range names {
		logx.Warnf("             %s", n)
	}
}

// manifestIntegrity checks the half of the bundle that is DATA rather than
// code: every vendor and ICD manifest must name a library that is IN the
// bundle, by a name the loader can resolve there.
//
// ⛔ THE GAP IT CLOSES, AND FOUR FAILURES WENT THROUGH IT. `integrity()` above
// walks DT_NEEDED, which is the graph the linker wrote. The GL stack is not on
// that graph at all: libglvnd finds its vendor by reading a JSON file and
// `dlopen`ing the string inside it, so a manifest naming
// `/nix/store/…/libEGL_mesa.so.0` produces a bundle where every DT_NEEDED
// resolves, every file is present, every check passes — and `eglInitialize`
// fails on the target because that path does not exist there. `TODO` T-071
// lists four distinct failures of this stack and every one of them was in
// data; this is the first check that reads data.
//
// ⚠ A REPORT, not a refusal, for the same reason `integrity()` is one: a
// closure can legitimately carry a manifest for a vendor it did not bundle.
// What must not happen is that it goes unsaid.
func (b *Builder) manifestIntegrity() {
	n, outside, absent := CheckManifests(b.AppDir)
	if n == 0 {
		return
	}
	if len(outside) == 0 && len(absent) == 0 {
		logx.Say("manifests   %d name only libraries present in the bundle", n)
		return
	}
	for _, s := range outside {
		logx.Warnf("a manifest still names a path OUTSIDE the bundle: %s", s)
	}
	for _, s := range absent {
		logx.Warnf("a manifest names a library the bundle does not have: %s", s)
	}
}

// CheckManifests is manifestIntegrity's measurement, separated from its
// reporting so `pgb bundle manifests` asserts on the SAME code a build runs
// rather than on a second implementation of the same rule.
//
// It returns how many manifests it read, the ones naming a path outside the
// bundle, and the ones naming a library the bundle does not carry.
func CheckManifests(appDir string) (n int, outside, absent []string) {
	provided := map[string]bool{}
	for _, d := range []string{"lib", "lib32"} {
		entries, err := os.ReadDir(filepath.Join(appDir, d))
		if err != nil {
			continue
		}
		for _, e := range entries {
			provided[e.Name()] = true
		}
	}
	for _, g := range manifestGlobs {
		files, _ := filepath.Glob(filepath.Join(appDir, g))
		for _, f := range files {
			data, err := os.ReadFile(f)
			if err != nil {
				continue
			}
			n++
			rel, _ := filepath.Rel(appDir, f)
			// The same two forms librariesNamedInManifests reads: a JSON
			// "library_path", and — for an OpenCL .icd, which is not JSON —
			// one library per line.
			var named []string
			for _, m := range manifestLibrary.FindAllSubmatch(data, -1) {
				named = append(named, string(m[1]))
			}
			if strings.HasSuffix(f, ".icd") {
				for _, line := range strings.Fields(string(data)) {
					if IsSharedObject(line) {
						named = append(named, line)
					}
				}
			}
			for _, v := range named {
				// An absolute path is the failure mode by itself: it can only
				// resolve on the machine the closure was built on.
				if strings.HasPrefix(v, "/") {
					outside = append(outside, rel+" -> "+v)
				}
				if !provided[filepath.Base(v)] {
					absent = append(absent, rel+" -> "+filepath.Base(v))
				}
			}
		}
	}
	sortStrings(outside)
	sortStrings(absent)
	return n, outside, absent
}

func sortStrings(s []string) {
	for i := 1; i < len(s); i++ {
		for j := i; j > 0 && s[j] < s[j-1]; j-- {
			s[j], s[j-1] = s[j-1], s[j]
		}
	}
}

// storeVersion is the version part of a store path's name, which is what
// `X-AppImage-Version=` wants.
//
// ⛔ IT WAS A NAMED GAP: two of the four package managers read that key
// (`gearlever` and `AppManager`) and this bundler emitted none, so both had to
// invent one. docs/research/bundle-capabilities.md §2. The version is not a
// guess — nixpkgs puts it in the store path the closure was resolved to.
func storeVersion(base string) string {
	// ⚠ shortStoreName for the same reason deriveProgramName uses it: this is
	// the package's own version, not a directory the bundle answers under.
	name := shortStoreName(base)
	// The first `-` followed by a digit starts the version, which is the same
	// rule deriveProgramName uses to strip it.
	for i := 0; i+1 < len(name); i++ {
		if name[i] == '-' && name[i+1] >= '0' && name[i+1] <= '9' {
			return name[i+1:]
		}
	}
	return ""
}

// withAppImageVersion adds X-AppImage-Version to a desktop entry that has
// none, immediately after the [Desktop Entry] header so it lands in the right
// group rather than in whatever group happens to be last.
func withAppImageVersion(lines []string, version string) []string {
	if version == "" {
		return lines
	}
	for _, l := range lines {
		if strings.HasPrefix(l, "X-AppImage-Version=") {
			return lines
		}
	}
	out := make([]string, 0, len(lines)+1)
	added := false
	for _, l := range lines {
		out = append(out, l)
		if !added && strings.TrimSpace(l) == "[Desktop Entry]" {
			out = append(out, "X-AppImage-Version="+version)
			added = true
		}
	}
	if !added {
		return lines
	}
	return out
}

// dropDesktopKey removes every line beginning with the given key.
func dropDesktopKey(path, key string) {
	data, err := os.ReadFile(path)
	if err != nil {
		return
	}
	var out []string
	for _, l := range strings.Split(string(data), "\n") {
		if strings.HasPrefix(l, key) {
			continue
		}
		out = append(out, l)
	}
	_ = os.WriteFile(path, []byte(strings.Join(out, "\n")), 0o644)
}
