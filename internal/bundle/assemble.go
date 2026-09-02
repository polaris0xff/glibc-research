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
	"strings"

	"github.com/polaris0xff/glibc-research/internal/elfx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/nixx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

// libSubtrees are library trees that are directories and break when
// flattened: each is found through a variable naming the DIRECTORY, so a
// flattened copy is invisible to the loader that wants it.
var libSubtrees = []string{
	"dri", "gbm", "gtk-3.0", "gtk-4.0", "gdk-pixbuf-2.0",
	"girepository-1.0", "pipewire-0.3", "spa-0.2", "vdpau",
}

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
	logx.Say("entry       %s", entry)
	if err := copyResolved(entry, filepath.Join(b.AppDir, "shared", "bin", b.Prog), 0o755); err != nil {
		return fail.Ran("could not copy the entry point: %v", err)
	}

	// Every other program in the same bin/, because an application is often a
	// set and a bundle with only the main window cannot be compared against
	// one that can render. They cost almost nothing: the libraries they share
	// are already in lib/.
	extra := 0
	if entries, err := os.ReadDir(filepath.Join(entryDir, "bin")); err == nil {
		for _, e := range entries {
			if e.IsDir() || e.Name() == b.Prog {
				continue
			}
			r, err := b.resolveEntry(entryDir, e.Name(), false)
			if err != nil || r == "" {
				continue
			}
			if copyResolved(r, filepath.Join(b.AppDir, "shared", "bin", e.Name()), 0o755) == nil {
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
		src := p
		if r, err := b.resolveEntry(owner, w, false); err == nil && r != "" {
			src = r
		}
		if copyResolved(src, filepath.Join(b.AppDir, "shared", "bin", w), 0o755) != nil {
			continue
		}
		logx.Say("program     %s  <- %s", w, filepath.Base(owner))
		extra++
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
	for round := 0; round < 6; round++ {
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

	for _, sub := range libSubtrees {
		matches, _ := filepath.Glob(filepath.Join(b.Root, "*", "lib", sub))
		for _, d := range matches {
			if fi, err := os.Stat(d); err != nil || !fi.IsDir() {
				continue
			}
			_ = copyTreeNoClobber(d, filepath.Join(lib, sub))
		}
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

func (b *Builder) desktopAndIcon() error {
	// Every share/ tree in the closure, merged.
	matches, _ := filepath.Glob(filepath.Join(b.Root, "*", "share"))
	for _, d := range matches {
		_ = copyTreeNoClobber(d, filepath.Join(b.AppDir, "share"))
	}

	// The ICD JSONs name an absolute store path, which is the last hop of the
	// OpenGL problem: libglvnd finds the vendor file, opens the path it names
	// and fails, on a bundle that has the library sitting in lib/ beside it.
	// Rewritten to the bare soname, which the loader resolves.
	n := 0
	for _, pat := range []string{
		"share/glvnd/egl_vendor.d/*.json",
		"share/vulkan/icd.d/*.json",
		"share/vulkan/implicit_layer.d/*.json",
	} {
		files, _ := filepath.Glob(filepath.Join(b.AppDir, pat))
		for _, f := range files {
			data, err := os.ReadFile(f)
			if err != nil {
				continue
			}
			out := icdLibraryPath.ReplaceAll(data, []byte(`${1}${2}"`))
			if string(out) != string(data) {
				_ = os.WriteFile(f, out, 0o644)
			}
			n++
		}
	}
	if n > 0 {
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
		_ = os.WriteFile(dst, []byte(strings.Join(lines, "\n")), 0o644)
		logx.Say("desktop     %s  (Icon=%s, Exec rewritten to %s)",
			filepath.Base(desktop), iconName, b.Prog)
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
Comment=generated by pgb bundle appimage -- the closure carried no desktop entry
`, b.Prog, b.Prog, b.Prog)
		if err := os.WriteFile(dst, []byte(body), 0o644); err != nil {
			return err
		}
	}

	// The icon is chosen by RESOLUTION, not by path length, which is a proxy
	// for nothing.
	icon := b.findIcon(iconName)
	if icon != "" {
		icon = b.storeResolve(icon)
	}
	if icon != "" && copyResolved(icon, filepath.Join(b.AppDir, filepath.Base(icon)), 0o644) == nil {
		_ = copyResolved(icon, filepath.Join(b.AppDir, ".DirIcon"), 0o644)
		logx.Say("icon        %s", filepath.Base(icon))
	} else {
		logx.Warnf("no icon named %q in the closure; the AppImage will have none", iconName)
	}
	return nil
}

var resolutionDir = regexp.MustCompile(`/([0-9]+)x[0-9]+/`)

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
		best, bestRes := "", -1
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
			if res > bestRes {
				best, bestRes = p, res
			}
			return nil
		})
		if best != "" {
			return best
		}
	}
	return ""
}

// integrity checks that every DT_NEEDED of every ELF left in the bundle
// resolves inside it. It is a REPORT, not a refusal: a closure legitimately
// contains libraries that dlopen things nothing links against.
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

func sortStrings(s []string) {
	for i := 1; i < len(s); i++ {
		for j := i; j > 0 && s[j] < s[j-1]; j-- {
			s[j], s[j-1] = s[j-1], s[j]
		}
	}
}

var _ = nixx.HashOf
var _ = proc.Look
