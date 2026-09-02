// needed.go — read, and shorten, a DT_NEEDED that names an absolute path.
//
// A bundle's loader is invoked with --library-path, which resolves a
// DT_NEEDED by NAME. Some libraries are linked with an absolute path, and an
// absolute DT_NEEDED is opened as a path with the search path never consulted,
// so the bundle fails on a machine that has the library sitting beside it.
//
// A basename is always shorter than the path it came from, so it is written
// OVER the original string at the same .dynstr offset with its own NUL. The
// entry still points at that offset and the bytes after the NUL become
// unreachable padding: nothing moves, no section grows, no offset changes.
//
// RPATH and RUNPATH are left alone. A stale entry in them is searched, found
// missing and skipped, which costs a failed stat and nothing else.
//
// SPDX-License-Identifier: MIT
package elfx

import (
	"debug/elf"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// ShortenResult reports one file's rewrite.
type ShortenResult struct {
	Path    string
	Changed []string // "old -> new"
}

// Shorten rewrites absolute DT_NEEDED entries in place to their basenames.
func Shorten(path string) (ShortenResult, error) {
	res := ShortenResult{Path: path}
	f, err := elf.Open(path)
	if err != nil {
		return res, err
	}
	needed, err := f.DynString(elf.DT_NEEDED)
	if err != nil {
		f.Close()
		return res, err
	}
	strtabOff, strtabSize, err := dynstrExtent(f)
	f.Close()
	if err != nil {
		return res, err
	}

	var absolute []string
	for _, n := range needed {
		if strings.HasPrefix(n, "/") {
			absolute = append(absolute, n)
		}
	}
	if len(absolute) == 0 {
		return res, nil
	}

	fh, err := os.OpenFile(path, os.O_RDWR, 0)
	if err != nil {
		return res, err
	}
	defer fh.Close()
	buf := make([]byte, strtabSize)
	if _, err := fh.ReadAt(buf, int64(strtabOff)); err != nil {
		return res, err
	}

	for _, old := range absolute {
		base := filepath.Base(old)
		idx := findString(buf, old)
		if idx < 0 {
			return res, fmt.Errorf("%s: %q is not in .dynstr", path, old)
		}
		if len(base) >= len(old) {
			return res, fmt.Errorf("%s: %q is not shorter than %q", path, base, old)
		}
		copy(buf[idx:], base)
		buf[idx+len(base)] = 0
		if _, err := fh.WriteAt(buf[idx:idx+len(base)+1], int64(strtabOff)+int64(idx)); err != nil {
			return res, err
		}
		res.Changed = append(res.Changed, old+" -> "+base)
	}
	return res, nil
}

// findString locates a NUL-terminated string starting at a string-table entry
// boundary, so a suffix of a longer entry is never matched.
func findString(tab []byte, want string) int {
	start := 0
	for i := 0; i < len(tab); i++ {
		if tab[i] != 0 {
			continue
		}
		if string(tab[start:i]) == want {
			return start
		}
		start = i + 1
	}
	return -1
}

// dynstrExtent finds the file offset and size of .dynstr, preferring the
// section header and falling back to the DT_STRTAB/DT_STRSZ pair for a file
// whose sections were stripped.
func dynstrExtent(f *elf.File) (off, size uint64, err error) {
	if s := f.Section(".dynstr"); s != nil {
		return s.Offset, s.Size, nil
	}
	var addr, sz uint64
	for _, prog := range f.Progs {
		if prog.Type != elf.PT_DYNAMIC {
			continue
		}
		data := make([]byte, prog.Filesz)
		if _, err := prog.ReadAt(data, 0); err != nil {
			return 0, 0, err
		}
		entry := 16
		if f.Class == elf.ELFCLASS32 {
			entry = 8
		}
		for i := 0; i+entry <= len(data); i += entry {
			var tag, val uint64
			if f.Class == elf.ELFCLASS32 {
				tag = uint64(f.ByteOrder.Uint32(data[i:]))
				val = uint64(f.ByteOrder.Uint32(data[i+4:]))
			} else {
				tag = f.ByteOrder.Uint64(data[i:])
				val = f.ByteOrder.Uint64(data[i+8:])
			}
			switch elf.DynTag(tag) {
			case elf.DT_STRTAB:
				addr = val
			case elf.DT_STRSZ:
				sz = val
			}
		}
	}
	if addr == 0 || sz == 0 {
		return 0, 0, fmt.Errorf("no .dynstr and no DT_STRTAB")
	}
	for _, prog := range f.Progs {
		if prog.Type == elf.PT_LOAD && addr >= prog.Vaddr && addr < prog.Vaddr+prog.Filesz {
			return prog.Off + (addr - prog.Vaddr), sz, nil
		}
	}
	return 0, 0, fmt.Errorf("DT_STRTAB 0x%x is in no PT_LOAD segment", addr)
}
