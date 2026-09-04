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
	"strings"

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

	// ⛔ A SCRIPT IS NOT A SHELL WRAPPER, and this is the defect that put
	// PYTHON SOURCE in a bundle's .env. readShellWrapper scans for
	// `key=value` lines and a Python script is full of them; `meld`'s
	// `message_type=Gtk.MessageType.ERROR,` became an environment variable.
	// The distinguishing property is the shebang naming a SHELL.
	py := filepath.Join(dir, "pyscript")
	_ = os.WriteFile(py, []byte("#!/nix/store/dddddddddddddddddddddddddddddddd-python3-3.14.7/bin/python3\n"+
		"dialog = Gtk.MessageDialog(\n    message_type=Gtk.MessageType.ERROR,\n    buttons=Gtk.ButtonsType.CLOSE,\n)\n"), 0o755)
	r.CheckInt("a PYTHON script yields no wrapper records", len(ReadWrapper(py)), 0)
	sh2 := filepath.Join(dir, "envshell")
	_ = os.WriteFile(sh2, []byte("#!/usr/bin/env bash\nexport FOO=/nix/store/cccccccccccccccccccccccccccccccc-qt/x\n"+
		"exec /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog\n"), 0o755)
	r.CheckBool("...but `env bash` still reads as one", len(ReadWrapper(sh2)) > 0, true)

	// ⛔ ${SHARUN_DIR} MUST SURVIVE THE STORE-PATH REWRITE AS A LITERAL.
	// See StoreRefToBundle: Go read it as a capture-group reference and
	// deleted it, so a bundle's own .env named /store/... on every row.
	r.Check("a lifted store path keeps ${SHARUN_DIR} as a literal",
		StoreRefToBundle("/nix/store/cccccccccccccccccccccccccccccccc-qt-6.11.1/plugins"),
		"${SHARUN_DIR}/store/qt-6.11.1/plugins")
	r.Check("...on every element of a path list, and only on store paths",
		StoreRefToBundle("/nix/store/cccccccccccccccccccccccccccccccc-a-1/x:/usr/share:/nix/store/dddddddddddddddddddddddddddddddd-b-2/y"),
		"${SHARUN_DIR}/store/a-1/x:/usr/share:${SHARUN_DIR}/store/b-2/y")

	// ⛔ WHERE A STORE REFERENCE ENDS, WHICH THE REGEX GOT WRONG UNTIL A BUILD
	// LOG SHOWED IT. The class was `[^" ']*` — three excluded characters — so
	// in a BINARY the match ran through the terminating NUL into the next
	// string, and in XML through the closing `<`. THREE of the six paths the
	// galculator bundle reported as "does not resolve inside the bundle" on
	// 2026-09-04 were that, not a real finding.
	// docs/design/store-paths.md §3.
	r.Check("a store reference ends at the C string's NUL",
		storeRefRe.FindString("/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-python3-3.14.7\x00\x00Exception"),
		"/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-python3-3.14.7")
	r.Check("...and at an XML markup boundary",
		storeRefRe.FindString("<dir>/nix/store/cccccccccccccccccccccccccccccccc-dejavu-fonts-minimal-2.37</dir>"),
		"/nix/store/cccccccccccccccccccccccccccccccc-dejavu-fonts-minimal-2.37")
	// ⭐ AND OVER-CAPTURE OF THE TAIL IS THE SAFE DIRECTION, SO THE CLASS IS A
	// BLACKLIST. Every rewrite copies the tail verbatim after substituting the
	// prefix, so an over-captured tail comes out unchanged; a TRUNCATED tail
	// would be written back short and corrupt the file. Nix does not constrain
	// the names of files inside a store path, so a whitelist would truncate.
	r.Check("...but a legitimate path tail is not truncated",
		storeRefRe.FindString("/nix/store/cccccccccccccccccccccccccccccccc-a-1/share/x.ui:/usr/share"),
		"/nix/store/cccccccccccccccccccccccccccccccc-a-1/share/x.ui:/usr/share")
	// ⭐ AND THE BASE IS THE VALUE THAT DECIDES EVERYTHING. Each caller cuts
	// the match at the first `/` and looks the result up in the closure, so a
	// boundary error does not merely widen a string — it produces a base no
	// closure can contain, which is reported as "does not resolve" and left
	// unrewritten. ⚠ The first draft of this block asserted on
	// StoreRefToBundle instead, which does not use this regex at all and
	// passed under the defect: a case that cannot fail for the reason it names
	// is worse than no case.
	xmlBase, _, _ := strings.Cut(strings.TrimPrefix(storeRefRe.FindString(
		"<dir>/nix/store/cccccccccccccccccccccccccccccccc-dejavu-fonts-minimal-2.37</dir>"),
		"/nix/store/"), "/")
	r.Check("...and the BASE every caller looks up carries no markup",
		xmlBase, "cccccccccccccccccccccccccccccccc-dejavu-fonts-minimal-2.37")

	// ⭐ THE REAL COUPLING BETWEEN THE .env AND THE FARM IS A NAME.
	// StoreRefToBundle writes `store/<name>` into a lifted value;
	// buildStoreFarm creates the directory `store/<shortStoreName(base)>`. If
	// the two derive the name differently, the value points at a directory the
	// bundle does not have — and nothing reads a .env value back against the
	// tree (`integrity()` walks DT_NEEDED; `manifestIntegrity()` reads ICD
	// manifests). ⚠ A review expecting over-capture to CORRUPT the value found
	// it does not: the substitution is `"store/" + name`, so junk in the name
	// is reproduced verbatim and the text is unchanged. The coupling is what
	// can actually break, so it is what is asserted.
	for _, s := range []string{
		`'/nix/store/cccccccccccccccccccccccccccccccc-gi-1.2'`,
		"/nix/store/cccccccccccccccccccccccccccccccc-a-1,/x",
		"<d>/nix/store/cccccccccccccccccccccccccccccccc-dejavu-2.37</d>",
	} {
		// ⛔ ASSERTED ON THE NAME, NOT ON THE TEXT. A `strings.Contains` of the
		// rewritten value passes on an over-captured name too, because the
		// correct name is a PREFIX of the wrong one — the first version of this
		// case did that and could not fail.
		base, _, _ := strings.Cut(strings.TrimPrefix(storeRefRe.FindString(s), "/nix/store/"), "/")
		g := storeRefName.FindStringSubmatch(s)
		r.Check("the lifted value names the farm directory the closure builds: "+shortStoreName(base),
			g[1], shortStoreName(base))
	}

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

	// ⛔ A WRAPPER'S `--set` REPLACES, AND THE BUNDLE'S OWN DIRECTORY HAS TO
	// SURVIVE IT. nixpkgs wraps meld with `--set XDG_DATA_DIRS <its own>`, so
	// the bundler's earlier line -- the one naming ${SHARUN_DIR}/share, the
	// merged tree holding every icon theme and schema in the closure -- was
	// resolved away. writeEnv now puts it back at the front AFTER the lift.
	// ⭐ THE LAST LINE COMES FROM THE MECHANISM, NOT FROM THE FIXTURE. Writing
	// it into the fixture by hand would have asserted FoldEnv's behaviour given
	// a line, which is true whether or not writeEnv ever adds one.
	written := FinalEnvLines([]string{
		"XDG_DATA_DIRS=${SHARUN_DIR}/share:${XDG_DATA_DIRS}:/usr/share",
		"XDG_DATA_DIRS=${SHARUN_DIR}/store/meld/share",
	}, func(string) bool { return true })
	r.CheckInt("the bundle's own share is appended by the MECHANISM", len(written), 3)
	env2 := filepath.Join(dir, ".env2")
	_ = os.WriteFile(env2, []byte(strings.Join(written, "\n")+"\n"), 0o644)
	if _, _, _, err := FoldEnv(env2); err != nil {
		r.Fail("fold the second env file", err.Error(), "no error")
		return r
	}
	folded2, _ := os.ReadFile(env2)
	r.Check("the bundle's own share survives a wrapper's --set",
		firstLineFor(string(folded2), "XDG_DATA_DIRS"),
		"XDG_DATA_DIRS=${SHARUN_DIR}/share:${SHARUN_DIR}/store/meld/share")

	// ---- the desktop-entry rules T-081 names, all pure logic --------------
	//
	// ⛔ THE ICON POLICY IS ASSERTED BECAUSE IT IS A POLICY. T-081: "≥128×128,
	// preferring 128 then 512 or 1024 — never a smaller bucket". The previous
	// rule was "biggest wins", which puts a 1024×1024 icon in a 48-pixel
	// panel, and the field's own selectors are accidents (first match; then
	// shortest path).
	r.CheckInt("128 is the best bucket", iconRank("/x/128x128/a.png", 128), 0)
	r.CheckInt("512 is next", iconRank("/x/512x512/a.png", 512), 1)
	r.CheckInt("then 1024", iconRank("/x/1024x1024/a.png", 1024), 2)
	r.CheckInt("another size above 128 comes after those",
		iconRank("/x/256x256/a.png", 256), 3)
	r.CheckInt("scalable is below every declared size above 128",
		iconRank("/x/scalable/apps/a.svg", 0), 4)
	r.CheckBool("⛔ a bucket BELOW 128 ranks worse than scalable",
		iconRank("/x/48x48/a.png", 48) > iconRank("/x/scalable/a.svg", 0), true)

	// X-AppImage-Version: two of the four managers read it and we emitted none.
	r.Check("the version comes out of the store path",
		storeVersion("ls8wzmc3wrwwi01czkihav804jgr68zq-galculator-2.1.4"), "2.1.4")
	r.Check("...including a name that itself contains a dash",
		storeVersion("ls8wzmc3wrwwi01czkihav804jgr68zq-gtk+3-3.24.52"), "3.24.52")
	r.Check("a store path with no version yields none",
		storeVersion("ls8wzmc3wrwwi01czkihav804jgr68zq-hello"), "")
	got2 := withAppImageVersion([]string{"[Desktop Entry]", "Name=X"}, "1.2")
	r.Check("the key lands inside [Desktop Entry], not at the end",
		strings.Join(got2, "|"), "[Desktop Entry]|X-AppImage-Version=1.2|Name=X")
	got2 = withAppImageVersion([]string{"[Desktop Entry]", "X-AppImage-Version=9"}, "1.2")
	r.Check("an entry that already has one is left alone",
		strings.Join(got2, "|"), "[Desktop Entry]|X-AppImage-Version=9")

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

	// ⭐ T-092: ONE NAMING RULE, and these cases fail under the two that were
	// here before. `carryBakedPaths` computed `base[33:]` inline with no
	// collision fallback while `buildStoreFarm` fell back to the full
	// `<hash>-<name>`, so on a closure carrying two builds of one package the
	// `.env` named a directory the farm never created.
	fn := filepath.Join(dir, "farm")
	h1 := "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
	h2 := "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
	h3 := "cccccccccccccccccccccccccccccccc"
	for _, d := range []string{h1 + "-gst-1.0", h2 + "-gst-1.0", h3 + "-solo-2.0"} {
		_ = os.MkdirAll(filepath.Join(fn, d), 0o755)
	}
	fb := &Builder{C: c, Root: fn}
	// ⛔ THE COLLIDING PAIR KEEPS ITS HASH. Both would have been "gst-1.0".
	r.Check("a colliding short name keeps the full base (1)",
		fb.farmDirName(h1+"-gst-1.0"), h1+"-gst-1.0")
	r.Check("a colliding short name keeps the full base (2)",
		fb.farmDirName(h2+"-gst-1.0"), h2+"-gst-1.0")
	// ...and a unique one is still shortened, or every path in every bundle
	// would change and the fix would be a regression wearing a fix's clothes.
	r.Check("a unique short name is still shortened",
		fb.farmDirName(h3+"-solo-2.0"), "solo-2.0")
	// ⚠ AND THE TWO SIDES MUST AGREE ON THE PATH THEY BUILD, which is the
	// actual property and is NOT what asking farmDirName twice tests.
	// ⛔ THE FIRST VERSION OF THIS CASE COMPARED farmDirName WITH
	// buildStoreFarmNames -- both of which CALL farmDirName -- so it passed
	// under a planted regression that made the other two cases fail. A case
	// that cannot fail is corrections.md C28 happening again, in the commit
	// that cites it.
	//
	// ⭐ So it compares the STRINGS the two sides actually construct: the farm
	// writes `"store/" + dirName`, and bakedOverride writes
	// `${SHARUN_DIR}/store/<name>/<sub>`. A divergence in either format --
	// not only in the name -- is what this catches.
	entries, _ := fb.buildStoreFarmNames()
	envLine := bakedOverride(fb.farmDirName(h1+"-gst-1.0"), "lib/gstreamer-1.0")[0]
	_, envValue, _ := strings.Cut(envLine, "=")
	m := envStoreRef.FindStringSubmatch(envValue)
	envDir := "(no match)"
	if m != nil {
		envDir = "store/" + m[1]
	}
	r.Check("⭐ the farm and the .env build the SAME path",
		envDir, entries[h1+"-gst-1.0"])

	// ⭐ And the regex envIntegrity reads `.env` back with. ⛔ It must stop at
	// `:` — a path-list value carries several — and it must accept a bare
	// store reference with no tail.
	envCase := func(v string) string {
		m := envStoreRef.FindStringSubmatch(v)
		if m == nil {
			return "(no match)"
		}
		return m[1] + m[2]
	}
	r.Check("an env store reference with a tail",
		envCase("${SHARUN_DIR}/store/gst-1.0/lib/gstreamer-1.0"), "gst-1.0/lib/gstreamer-1.0")
	r.Check("⛔ ...stops at the ':' of a path list",
		envCase("${SHARUN_DIR}/store/gst-1.0/lib:${SHARUN_DIR}/store/other/lib"), "gst-1.0/lib")
	r.Check("a bare store reference with no tail",
		envCase("${SHARUN_DIR}/store/gst-1.0"), "gst-1.0")
	r.Check("a value naming no store path does not match",
		envCase("${SHARUN_DIR}/lib/gconv"), "(no match)")
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
