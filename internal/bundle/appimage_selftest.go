// appimage_selftest.go — the entry-point resolver's cases, offline.
//
// The resolver decides which program a bundle ships, and getting it wrong is
// silent: a name pgb DERIVED from a store path that misses may fall back, and
// a name the CALLER asked for must not — substituting a different program for
// one that was named has shipped the wrong binary.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"os"
	"path/filepath"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// AppImageSelftest exercises resolveEntry against a fixture closure.
func AppImageSelftest(c *cfg.Config) *selftest.Report {
	r := selftest.New("bundle-appimage")

	dir, err := os.MkdirTemp("", "pgb-appimage-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "created")
		return r
	}
	defer os.RemoveAll(dir)

	realDir := filepath.Join(dir, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real")
	pkgDir := filepath.Join(dir, "pkg")
	for _, d := range []string{filepath.Join(realDir, "bin"), filepath.Join(pkgDir, "bin")} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			r.Fail("mkdir", err.Error(), "created")
			return r
		}
	}
	// A file that begins with the ELF magic is enough: the resolver asks what
	// the file IS, not whether it links.
	_ = os.WriteFile(filepath.Join(realDir, "bin", "prog"), []byte("\x7fELF fake"), 0o755)
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "prog"),
		[]byte("#!/bin/sh\nexec /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog \"$@\"\n"), 0o755)

	b := &Builder{C: c, Root: dir}
	got, err := b.resolveEntry(pkgDir, "prog", false)
	r.Check("a wrapper script is followed to its ELF",
		relOr(got.ELF, dir, err), "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog")
	r.Check("and it is not reported as a script entry", got.Script, "")

	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "prog"),
		[]byte("#!/bin/sh\nexec /nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-gone/bin/x\n"), 0o755)
	_, err = b.resolveEntry(pkgDir, "prog", false)
	r.CheckBool("a wrapper pointing nowhere is refused", err != nil, true)

	// Two binaries, neither named what was asked for. With a caller-supplied
	// name it must REFUSE; without one it may fall back, and the two must not
	// be the same answer.
	_ = os.Remove(filepath.Join(pkgDir, "bin", "prog"))
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "aardvark"), []byte("\x7fELF one"), 0o755)
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "zebra"), []byte("\x7fELF two"), 0o755)

	_, err = b.resolveEntry(pkgDir, "nosuchprog", true)
	r.CheckBool("--name naming no program is refused, not substituted", err != nil, true)

	got, err = b.resolveEntry(pkgDir, "nosuchprog", false)
	r.CheckBool("a DERIVED name that misses still falls back", err == nil && got.ELF != "", true)

	// ---- T-081's second blocker: a SCRIPT entry point ---------------------
	//
	// ⛔ WHAT THIS CATCHES, AND IT COST EVERY PYTHON GUI APPLICATION. A
	// nixpkgs Python program is a script whose shebang names an interpreter in
	// the closure. Asking for ONE path made the resolver scan the script's own
	// text, land back on the wrapper that pointed at it, and report
	// `no entry point` after five hops. The answer is a PAIR.
	pyDir := filepath.Join(dir, "dddddddddddddddddddddddddddddddd-python3-3.14.7")
	if err := os.MkdirAll(filepath.Join(pyDir, "bin"), 0o755); err != nil {
		r.Fail("mkdir python", err.Error(), "created")
		return r
	}
	_ = os.WriteFile(filepath.Join(pyDir, "bin", "python3"), []byte("\x7fELF py"), 0o755)
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "pyprog"),
		[]byte("#!/nix/store/dddddddddddddddddddddddddddddddd-python3-3.14.7/bin/python3\nimport sys\n"), 0o755)
	got, err = b.resolveEntry(pkgDir, "pyprog", true)
	r.Check("a script entry resolves to its INTERPRETER",
		relOr(got.ELF, dir, err), "dddddddddddddddddddddddddddddddd-python3-3.14.7/bin/python3")
	r.Check("and the script itself is the argument",
		relOr(got.Script, dir, err), "pkg/bin/pyprog")

	// `#!/nix/store/<coreutils>/bin/env python3` is the other shape nixpkgs
	// writes, and the interpreter is then a NAME to find in the closure.
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "envprog"),
		[]byte("#!/nix/store/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-coreutils/bin/env python3\nprint(1)\n"), 0o755)
	got, err = b.resolveEntry(pkgDir, "envprog", true)
	r.Check("an `env <interp>` shebang finds the interpreter in the closure",
		relOr(got.ELF, dir, err), "dddddddddddddddddddddddddddddddd-python3-3.14.7/bin/python3")

	// ⛔ AND A HOST INTERPRETER IS REFUSED, WHICH IS THE POINT. `#!/bin/sh`
	// with nothing else to follow would put the HOST's interpreter and its
	// libc in the process — measured at 1-4 host shared objects per glibc row
	// in experiments/90-, and the one thing a bundle may not do.
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "hostprog"),
		[]byte("#!/bin/sh\necho hello\n"), 0o755)
	_, err = b.resolveEntry(pkgDir, "hostprog", true)
	r.CheckBool("a HOST-interpreter shebang is refused, not adopted", err != nil, true)

	// The wrapper reader, on both shapes.
	shell := filepath.Join(dir, "shellwrap")
	_ = os.WriteFile(shell, []byte("#!/bin/sh\nexport QT_PLUGIN_PATH='/nix/store/cccccccccccccccccccccccccccccccc-qt/plugins':$QT_PLUGIN_PATH\n"+
		"exec /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog \"$@\"\n"), 0o755)
	recs := ReadWrapper(shell)
	r.Check("a shell wrapper's target is read", WrapperTarget(recs),
		"/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog")
	found := ""
	for _, rec := range recs {
		if rec.Var == "QT_PLUGIN_PATH" {
			found = string(rec.Op)
		}
	}
	r.Check("and its prefix assignment is a prefix", found, "prefix")

	// The .env folder, which is what keeps a multi-program bundle from asking
	// Qt to scan the same directories once per program.
	envFile := filepath.Join(dir, ".env")
	_ = os.WriteFile(envFile, []byte(
		"QT_PLUGIN_PATH=/a:${QT_PLUGIN_PATH}\n"+
			"QT_PLUGIN_PATH=/a:/b:${QT_PLUGIN_PATH}\n"+
			"XDG_DATA_DIRS=/x:${XDG_DATA_DIRS}\n"), 0o644)
	keys, before, after, ferr := FoldEnv(envFile)
	if ferr != nil {
		r.Fail("fold the env file", ferr.Error(), "no error")
		return r
	}
	r.Check("folding keeps one line per key", itoa(keys), "2")
	r.CheckBool("and the folded file is smaller", after < before, true)
	folded, _ := os.ReadFile(envFile)
	r.Check("a repeated path component is dropped, first position kept",
		firstLineFor(string(folded), "QT_PLUGIN_PATH"),
		"QT_PLUGIN_PATH=/a:/b:${QT_PLUGIN_PATH}")

	// ---- elfClass: the decision that keeps lib32 apart from lib ----------
	//
	// ⛔ WHY THIS IS ASSERTED. `copyLibraries` routes an object to `lib32` or
	// `lib` on this one value, and its own comment says what getting it wrong
	// costs: *"a flat directory holding an i386 and an x86_64 libfoo.so.1 gives
	// the loader whichever landed first"*. That is a silent wrong-architecture
	// load, not a build failure. ⚠ Nothing covered it — T-057's 32-bit item.
	//
	// ⭐ Hermetic: `elfClass` reads five bytes, so the fixtures are five bytes.
	// No compiler, no multilib, and it runs on a machine that has neither.
	ec := filepath.Join(dir, "elfclass")
	if err := os.MkdirAll(ec, 0o755); err != nil {
		r.Fail("mkdir elfclass", err.Error(), "created")
		return r
	}
	classOf := func(name string, body []byte) int {
		p := filepath.Join(ec, name)
		if err := os.WriteFile(p, body, 0o644); err != nil {
			return -1
		}
		return elfClass(p)
	}
	elf32 := []byte{0x7f, 'E', 'L', 'F', 1, 1, 1, 0}
	elf64 := []byte{0x7f, 'E', 'L', 'F', 2, 1, 1, 0}
	r.CheckInt("an ELFCLASS32 object is 32", classOf("m32.so", elf32), 32)
	r.CheckInt("an ELFCLASS64 object is 64", classOf("m64.so", elf64), 64)
	// ⛔ THE THREE ZEROS, and each is a different way of not being an object.
	// Zero means "not sorted into lib32", which is the safe direction only
	// because `copyLibraries` treats everything that is not 32 as `lib`.
	r.CheckInt("a file with no ELF magic is 0",
		classOf("text.so", []byte("#!/bin/sh\necho hi\n")), 0)
	r.CheckInt("an unknown EI_CLASS is 0, not guessed",
		classOf("weird.so", []byte{0x7f, 'E', 'L', 'F', 9, 1, 1, 0}), 0)
	// ⚠ A file SHORTER than the header cannot be classified, and the read has
	// to fail rather than index past the end.
	r.CheckInt("a file shorter than the header is 0",
		classOf("short.so", []byte{0x7f, 'E', 'L'}), 0)
	r.CheckInt("an empty file is 0", classOf("empty.so", nil), 0)
	r.CheckInt("a file that does not exist is 0",
		elfClass(filepath.Join(ec, "absent.so")), 0)
	return r
}

func relOr(p, root string, err error) string {
	if err != nil {
		return "error: " + err.Error()
	}
	rel, rerr := filepath.Rel(root, p)
	if rerr != nil {
		return p
	}
	return rel
}

func firstLineFor(text, key string) string {
	for _, line := range splitLines(text) {
		if len(line) > len(key) && line[:len(key)+1] == key+"=" {
			return line
		}
	}
	return ""
}

func splitLines(s string) []string {
	var out []string
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == '\n' {
			out = append(out, s[start:i])
			start = i + 1
		}
	}
	if start < len(s) {
		out = append(out, s[start:])
	}
	return out
}
