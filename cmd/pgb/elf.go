// elf.go — the `pgb elf` subcommands.
//
// SPDX-License-Identifier: MIT
package main

import (
	"os"
	"path/filepath"
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
			found := false
			for _, s := range syms {
				if s == "__nss_configure_lookup" {
					found = true
					break
				}
			}
			r.CheckBool("archive-reader-finds-nss-symbol", found, true)
			r.CheckBool("archive-reader-returns-many", len(syms) > 100, true)
		}
	} else {
		r.Skip("no libc.a, so the archive reader was not exercised")
	}
	return r
}
