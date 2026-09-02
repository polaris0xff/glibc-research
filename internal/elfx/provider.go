// provider.go — the symbol list a compiled-in ELF loader can define.
//
// This is DefinedExternalSymbols' problem with two differences that both cost
// a measured failure to find, so they are here rather than folded into it:
//
//  1. ⛔ A GNU ld SCRIPT IS NOT AN ARCHIVE. /usr/lib/x86_64-linux-gnu/libm.a
//     is the text `GROUP ( libm-2.39.a libmvec.a )`, and a reader that opens
//     it as `ar` finds zero symbols and says nothing. Reading it that way put
//     4,891 names in the first provider table instead of 7,216 — every math
//     symbol missing — and the loader then failed on `pow` for a third of the
//     objects it tried. docs/history/corrections.md.
//
//  2. ⛔ A TLS SYMBOL CANNOT BE REFERENCED AS DATA. The generated table takes
//     each name's address; for a thread-local that is a link error, not a
//     wrong answer:
//     "__libc_errno: TLS reference in libc.a(check_fds.o) mismatches non-TLS
//     reference". 30 of libc.a's exports are thread-local, so they are
//     reported separately and the loader reaches the two that matter — errno
//     and h_errno — by address instead.
//
// SPDX-License-Identifier: MIT
package elfx

import (
	"bytes"
	"debug/elf"
	"errors"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// ProviderSymbols is what an archive set can define for a loaded object.
// Defined holds the ordinary symbols, sorted and unique — the table's rows,
// and the order the loader binary-searches. TLS holds the thread-local ones,
// which are excluded from Defined for the reason above.
type ProviderSymbols struct {
	Defined []string
	TLS     []string
}

// ReadProviderSymbols reads every archive in paths, expanding GNU ld scripts.
func ReadProviderSymbols(paths []string) (ProviderSymbols, error) {
	var defined, tls []string

	expanded, err := ExpandLinkerScripts(paths)
	if err != nil {
		return ProviderSymbols{}, err
	}
	for _, p := range expanded {
		err := forEachObject(p, func(name string, data []byte) error {
			d, t := providerInObject(data)
			defined = append(defined, d...)
			tls = append(tls, t...)
			return nil
		})
		if err != nil {
			return ProviderSymbols{}, err
		}
	}
	sort.Strings(defined)
	sort.Strings(tls)
	defined, tls = dedup(defined), dedup(tls)

	// A name that is thread-local anywhere is excluded everywhere: the link
	// fails on the first mismatched reference, so a second, non-TLS definition
	// of the same name elsewhere does not rescue it.
	drop := make(map[string]bool, len(tls))
	for _, s := range tls {
		drop[s] = true
	}
	keep := defined[:0]
	for _, s := range defined {
		if !drop[s] {
			keep = append(keep, s)
		}
	}
	return ProviderSymbols{Defined: keep, TLS: tls}, nil
}

// ExpandLinkerScripts replaces any GNU ld script in paths with the archives it
// names. A path that is neither an archive nor a script is passed through, so
// the caller still sees a missing file as a missing file.
func ExpandLinkerScripts(paths []string) ([]string, error) {
	var out []string
	seen := make(map[string]bool)

	for _, p := range paths {
		data, err := os.ReadFile(p)
		if err != nil {
			if errors.Is(err, os.ErrNotExist) {
				continue // an optional archive this glibc does not ship
			}
			return nil, err
		}
		if bytes.HasPrefix(data, []byte("!<arch>\n")) || !isLinkerScript(data) {
			if !seen[p] {
				seen[p] = true
				out = append(out, p)
			}
			continue
		}
		for _, ref := range linkerScriptInputs(string(data), filepath.Dir(p)) {
			if !seen[ref] {
				seen[ref] = true
				out = append(out, ref)
			}
		}
	}
	return out, nil
}

func isLinkerScript(data []byte) bool {
	head := data
	if len(head) > 512 {
		head = head[:512]
	}
	return bytes.Contains(head, []byte("GNU ld script")) ||
		bytes.Contains(head, []byte("GROUP")) ||
		bytes.Contains(head, []byte("INPUT"))
}

// linkerScriptInputs pulls the archive paths out of GROUP(...)/INPUT(...).
// Relative names are resolved against the script's own directory, which is
// what ld does.
func linkerScriptInputs(text, dir string) []string {
	repl := strings.NewReplacer("(", " ", ")", " ", ",", " ", "\n", " ", "\t", " ")
	var out []string
	for _, tok := range strings.Fields(repl.Replace(text)) {
		if !strings.HasSuffix(tok, ".a") && !strings.Contains(tok, ".so") {
			continue
		}
		if strings.HasPrefix(tok, "AS_NEEDED") {
			continue
		}
		if !filepath.IsAbs(tok) {
			tok = filepath.Join(dir, tok)
		}
		if st, err := os.Stat(tok); err == nil && st.Mode().IsRegular() {
			out = append(out, tok)
		}
	}
	return out
}

func providerInObject(data []byte) (defined, tls []string) {
	f, err := elf.NewFile(bytes.NewReader(data))
	if err != nil {
		return nil, nil // an archive member that is not ELF at all
	}
	defer f.Close()
	syms, err := f.Symbols()
	if err != nil && !errors.Is(err, elf.ErrNoSymbols) {
		return nil, nil
	}
	for _, s := range syms {
		if s.Name == "" || !cIdent.MatchString(s.Name) {
			continue
		}
		switch elf.ST_BIND(s.Info) {
		case elf.STB_GLOBAL, elf.STB_WEAK:
		default:
			continue
		}
		if elf.ST_TYPE(s.Info) == elf.STT_TLS {
			tls = append(tls, s.Name)
			continue
		}
		switch elf.ST_TYPE(s.Info) {
		case elf.STT_SECTION, elf.STT_FILE:
			continue
		}
		switch s.Section {
		case elf.SHN_UNDEF, elf.SHN_ABS, elf.SHN_COMMON:
			continue
		}
		defined = append(defined, s.Name)
	}
	return defined, tls
}
