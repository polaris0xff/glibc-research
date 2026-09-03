// elf.go — the `pgb elf` subcommands.
//
// SPDX-License-Identifier: MIT
package main

import (
	"os"
	"path/filepath"
	"slices"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/elfx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
	"github.com/polaris0xff/glibc-research/internal/selftest"
)

func elfNeeded(path string) error {
	needed, err := elfx.Needed(path)
	if err != nil {
		return fail.Ran("%s: %v", path, err)
	}
	for _, n := range needed {
		logx.Say("%s", n)
	}
	return nil
}

// elfPrint writes FILE<tab>NEEDED for each entry. A file that is not an ELF is
// skipped silently: a caller sweeping a directory passes non-objects.
func elfPrint(path string) error {
	needed, err := elfx.Needed(path)
	if err != nil {
		return nil
	}
	for _, n := range needed {
		logx.Say("%s\t%s", path, n)
	}
	return nil
}

// elfShorten rewrites absolute DT_NEEDED entries to their basenames, in place,
// and reports what changed.
func elfShorten(paths []string) error {
	changed := 0
	for _, p := range paths {
		res, err := elfx.Shorten(p)
		if err != nil {
			logx.Warnf("%s: %v", p, err)
			continue
		}
		for _, c := range res.Changed {
			logx.Say("%s: %s", p, c)
			changed++
		}
	}
	logx.Infof("%d DT_NEEDED entr(y|ies) shortened", changed)
	return nil
}

func elfInfo(path string) error {
	info, err := elfx.Inspect(path)
	if err != nil {
		return fail.Ran("%s: %v", path, err)
	}
	logx.Say("%-18s %s", "class", info.Class)
	logx.Say("%-18s %s", "machine", info.Machine)
	logx.Say("%-18s %s", "type", info.Type)
	logx.Say("%-18s %s", "interpreter", orNone(info.Interp))
	logx.Say("%-18s %s", "soname", orNone(info.SoName))
	logx.Say("%-18s %s", "runpath", orNone(strings.Join(info.RunPath, ":")))
	logx.Say("%-18s %s", "PT_GNU_EH_FRAME", yesNo(info.HasEHFrame))
	logx.Say("%-18s %s", "static", yesNo(info.Static))
	logx.Say("%-18s %s", "needed", orNone(strings.Join(info.Needed, " ")))
	return nil
}

func orNone(s string) string {
	if s == "" {
		return "(none)"
	}
	return s
}

func yesNo(b bool) string {
	if b {
		return "yes"
	}
	return "no"
}

// elfSelftest proves the DT_NEEDED reader and the in-place shortener against a
// binary this machine builds, so the fixture is a real ELF rather than a
// hand-written one.
func elfSelftest() *selftest.Report {
	r := selftest.New("elf")
	if !proc.Look("cc") {
		r.Skip("no C compiler, so no fixture can be built")
		return r
	}
	dir, err := os.MkdirTemp("", "pgb-elf-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "a writable temporary directory")
		return r
	}
	defer os.RemoveAll(dir)

	src := filepath.Join(dir, "probe.c")
	if err := os.WriteFile(src, []byte("int main(void){return 0;}\n"), 0o644); err != nil {
		r.Fail("write fixture", err.Error(), "created")
		return r
	}

	dyn := filepath.Join(dir, "dynamic")
	if res, err := proc.Quiet("cc", "-o", dyn, src); err != nil || res.Failed() {
		r.Skip("cannot build a dynamic fixture")
	} else {
		info, err := elfx.Inspect(dyn)
		if err != nil {
			r.Fail("inspect-dynamic", err.Error(), "an ELF")
		} else {
			r.CheckBool("dynamic-has-interp", info.Interp != "", true)
			r.CheckBool("dynamic-has-needed", len(info.Needed) > 0, true)
			r.CheckBool("dynamic-not-static", info.Static, false)
		}
	}

	st := filepath.Join(dir, "static")
	if res, err := proc.Quiet("cc", "-static", "-o", st, src); err != nil || res.Failed() {
		r.Skip("cannot build a static fixture")
	} else {
		info, err := elfx.Inspect(st)
		if err != nil {
			r.Fail("inspect-static", err.Error(), "an ELF")
		} else {
			r.CheckBool("static-no-interp", info.Interp == "", true)
			r.CheckBool("static-no-needed", len(info.Needed) == 0, true)
			r.CheckBool("static-is-static", info.Static, true)
		}
	}

	// The archive reader: libc.a is the archive every machine with a static
	// toolchain has, and __nss_configure_lookup is the symbol pgb depends on.
	if path, err := proc.Capture("cc", "-print-file-name=libc.a"); err == nil && filepath.IsAbs(path) {
		syms, err := elfx.DefinedExternalSymbols(path)
		if err != nil {
			r.Fail("archive-symbols", err.Error(), "a symbol list")
		} else {
			found := slices.Contains(syms, "__nss_configure_lookup")
			r.CheckBool("archive-reader-finds-nss-symbol", found, true)
			r.CheckBool("archive-reader-returns-many", len(syms) > 100, true)
		}
	} else {
		r.Skip("no libc.a, so the archive reader was not exercised")
	}
	return r
}

// cxxRuntimeSelftest checks the detector that decides whether a C link needs
// the C++ runtime. TODO T-063.
//
// ⛔ WHY IT BUILDS A FIXTURE RATHER THAN ASSERTING ON A CRAFTED BYTE STRING.
// The question is "does this archive have an UNDEFINED reference to operator
// delete", and the only honest way to ask it is of an archive a real compiler
// produced. ⚠ It SKIPS VISIBLY where there is no C++ compiler, the way the
// `elf` suite above skips without one: a selftest that quietly runs nothing
// reports success, which docs/AGENTS.md §0b calls the worst answer here.
func cxxRuntimeSelftest() *selftest.Report {
	r := selftest.New("cxx-runtime")
	if !proc.Look("c++") || !proc.Look("cc") || !proc.Look("ar") {
		r.Skip("needs c++, cc and ar to build the fixtures")
		return r
	}
	dir, err := os.MkdirTemp("", "pgb-cxx-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "a writable temporary directory")
		return r
	}
	defer os.RemoveAll(dir)

	write := func(name, body string) bool {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(body), 0o644); err != nil {
			r.Fail("write "+name, err.Error(), "created")
			return false
		}
		return true
	}
	// ⭐ THE SUBJECT: C++ with a C entry point, which is the shape libicuuc.a
	// has and the shape that makes the driver's argv[0] rule wrong. `new` and
	// `delete` and a vtable are what the link then cannot resolve.
	if !write("thing.cpp", `struct Thing { virtual ~Thing() {} int v; };
extern "C" int thing_answer(void) {
    Thing *t = new Thing(); t->v = 42; int v = t->v; delete t; return v;
}
`) {
		return r
	}
	// ⛔ THE NEGATIVE CONTROL, and it is the half that matters: ordinary C must
	// NOT demand a C++ runtime. A detector that answered yes for everything
	// would pass every positive case in this file.
	if !write("plain.c", "int plain_answer(void){ return 42; }\n") {
		return r
	}

	obj := filepath.Join(dir, "thing.o")
	if res, err := proc.Quiet("c++", "-c", "-o", obj, filepath.Join(dir, "thing.cpp")); err != nil || res.Failed() {
		r.Skip("the C++ fixture did not compile")
		return r
	}
	arc := filepath.Join(dir, "libthing.a")
	if res, err := proc.Quiet("ar", "rcs", arc, obj); err != nil || res.Failed() {
		r.Skip("ar could not build the fixture archive")
		return r
	}
	cobj := filepath.Join(dir, "plain.o")
	if res, err := proc.Quiet("cc", "-c", "-o", cobj, filepath.Join(dir, "plain.c")); err != nil || res.Failed() {
		r.Skip("the C fixture did not compile")
		return r
	}

	need, sym := elfx.NeedsCXXRuntime(obj)
	r.CheckBool("a C++ object demands the C++ runtime", need, true)
	r.CheckBool("...and it names the symbol that says so", sym != "", true)

	needA, symA := elfx.NeedsCXXRuntime(arc)
	r.CheckBool("an ARCHIVE of it does too", needA, true)
	r.CheckBool("...and names a symbol", symA != "", true)

	// ⛔ THE NEGATIVE CONTROL.
	needC, _ := elfx.NeedsCXXRuntime(cobj)
	r.CheckBool("an ordinary C object does NOT", needC, false)

	// ⚠ A file that is not an object at all must be a quiet no, not an error:
	// a link line carries .a members that are not ELF, and linker scripts.
	if write("notanobject.a", "this is not an archive\n") {
		needN, _ := elfx.NeedsCXXRuntime(filepath.Join(dir, "notanobject.a"))
		r.CheckBool("a file that is not an object is a quiet no", needN, false)
	}
	needM, _ := elfx.NeedsCXXRuntime(filepath.Join(dir, "no-such-file.a"))
	r.CheckBool("a missing file is a quiet no", needM, false)

	return r
}
