// flags.go — exactly what pgb injects into a compile and a link.
//
// Compile flags LEAD the command line, so they are defaults the caller can
// override. Link flags TRAIL it, because link order is meaning: an archive is
// scanned where it appears.
//
// SPDX-License-Identifier: MIT
package wrapper

import (
	"os"
	"path/filepath"

	"github.com/polaris0xff/glibc-research/internal/cfg"
)

// CompileFlags are prepended to a compile.
func CompileFlags(c *cfg.Config) []string {
	var out []string
	if bl := c.Baseline(); bl != "" {
		out = append(out, "-march="+bl)
	}
	return append(out, "-fno-plt")
}

// LinkFlags are appended to an executable link. cxx selects the C++ driver's
// extra symbol forcing.
//
// -Wl,--eh-frame-hdr is on every link because gcc suppresses it for -static,
// leaving the binary with no PT_GNU_EH_FRAME. The GNU unwinder does not need
// it, but one that discovers tables only through the segment finds nothing and
// terminates at the first throw.
//
// -u,pgb_runtime_anchor forces the NSS constructor to stay: a constructor with
// no referenced symbol is dropped from an archive, and an nssfix that silently
// did not link is the failure the mechanism exists to prevent.
func LinkFlags(c *cfg.Config, rd string, cxx bool) []string {
	out := []string{
		"-static",
		"-Wl,--eh-frame-hdr",
		filepath.Join(rd, "pgb-nssfix.o"),
		"-Wl,-u,pgb_runtime_anchor",
	}
	if c.UseIconv {
		out = append(out, "-Wl,--wrap=iconv_open,--wrap=iconv,--wrap=iconv_close")
		// libstdc++ and static libraries the caller re-emits are scanned after
		// -lpgbruntime, so their __wrap_iconv* references would arrive with the
		// defining archive already behind the linker. -u pulls the member in at
		// -lpgbruntime instead.
		if cxx || len(c.WrapDlopen) > 0 {
			out = append(out,
				"-Wl,-u,__wrap_iconv_open",
				"-Wl,-u,__wrap_iconv",
				"-Wl,-u,__wrap_iconv_close")
		}
		out = append(out,
			"-L"+rd, "-lpgbruntime",
			"-L"+filepath.Join(c.LibiconvPrefix, "lib"), "-liconv")
	}
	if c.EmbedLocale {
		out = append(out, "-Wl,--wrap=setlocale",
			filepath.Join(rd, "pgb-locale.o"), filepath.Join(rd, "pgb-locale-data.o"))
	}
	if c.EmbedCacert {
		out = append(out, "-Wl,-u,pgb_cacert_anchor",
			filepath.Join(rd, "pgb-cacert.o"), filepath.Join(rd, "pgb-cacert-data.o"))
	}
	if c.EmbedTerminfo {
		out = append(out, "-Wl,-u,pgb_terminfo_anchor",
			filepath.Join(rd, "pgb-terminfo.o"), filepath.Join(rd, "pgb-terminfo-data.o"))
	}
	if len(c.WrapDlopen) > 0 {
		if _, err := os.Stat(filepath.Join(rd, "pgb-dlopen-table.o")); err == nil {
			out = append(out, "-Wl,--wrap=dlopen,--wrap=dlsym,--wrap=dlclose,--wrap=dlerror",
				filepath.Join(rd, "pgb-dlopen.o"), filepath.Join(rd, "pgb-dlopen-table.o"))
			out = append(out, dlopenObjects(rd)...)
		}
	}
	// --host-dlopen reuses the same --wrap redirection, so it is added only
	// when --wrap-dlopen did not already ask for it. pgb-dlopen.c consults its
	// own compiled-in plugin table first and falls through to the ELF loader,
	// which is the order that keeps a program's own plugins from being mapped
	// when they are already in the link.
	if c.HostDlopen {
		if flags := hostDlopenLinkFlags(rd); flags != nil {
			if len(c.WrapDlopen) == 0 {
				out = append(out, "-Wl,--wrap=dlopen,--wrap=dlsym,--wrap=dlclose,--wrap=dlerror",
					filepath.Join(rd, "pgb-dlopen.o"))
			}
			out = append(out, flags...)
		}
	}
	return out
}
