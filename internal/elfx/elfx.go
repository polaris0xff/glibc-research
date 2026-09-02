// Package elfx reads ELF objects, shared libraries and `ar` archives.
//
// It replaces the nm/readelf/objdump text parsing the shell tool relied on:
// the structures are read directly, so a locale, a binutils version or a
// column order cannot change what pgb concludes about a file.
//
// SPDX-License-Identifier: MIT
package elfx

import (
	"bytes"
	"debug/elf"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
)

// Needed lists a file's DT_NEEDED entries in order.
func Needed(path string) ([]string, error) {
	f, err := elf.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	return f.DynString(elf.DT_NEEDED)
}

// Info summarises what pgb asserts about a produced binary.
type Info struct {
	Class      string // ELF32 | ELF64
	Machine    string
	Type       string
	Interp     string // empty when there is no PT_INTERP
	Needed     []string
	RunPath    []string
	SoName     string
	HasEHFrame bool // a PT_GNU_EH_FRAME segment is present
	Static     bool // no PT_INTERP and no DT_NEEDED
}

// Inspect reads the properties `pgb verify` reports without shelling out.
func Inspect(path string) (*Info, error) {
	f, err := elf.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	i := &Info{
		Class:   f.Class.String(),
		Machine: f.Machine.String(),
		Type:    f.Type.String(),
	}
	for _, p := range f.Progs {
		switch p.Type {
		case elf.PT_INTERP:
			b, err := io.ReadAll(p.Open())
			if err == nil {
				i.Interp = strings.TrimRight(string(b), "\x00")
			}
		case elf.PT_GNU_EH_FRAME:
			i.HasEHFrame = true
		}
	}
	if n, err := f.DynString(elf.DT_NEEDED); err == nil {
		i.Needed = n
	}
	if rp, err := f.DynString(elf.DT_RUNPATH); err == nil && len(rp) > 0 {
		i.RunPath = splitPathList(rp)
	} else if rp, err := f.DynString(elf.DT_RPATH); err == nil {
		i.RunPath = splitPathList(rp)
	}
	if so, err := f.DynString(elf.DT_SONAME); err == nil && len(so) > 0 {
		i.SoName = so[0]
	}
	i.Static = i.Interp == "" && len(i.Needed) == 0
	return i, nil
}

func splitPathList(vals []string) []string {
	var out []string
	for _, v := range vals {
		for _, p := range strings.Split(v, ":") {
			if p != "" {
				out = append(out, p)
			}
		}
	}
	return out
}

var cIdent = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*$`)

// DefinedExternalSymbols lists the symbols an object or archive DEFINES and
// exports, which is what a plugin's dlsym table is made of.
//
// Undefined references are excluded: they are what the plugin imports, and
// declaring them would make the link fail on symbols it merely calls. Absolute
// and common symbols are excluded for the same reason nm's letter set omits
// them — they have no address in a section to take.
func DefinedExternalSymbols(path string) ([]string, error) {
	var out []string
	err := forEachObject(path, func(name string, data []byte) error {
		syms, err := definedInObject(data)
		if err != nil {
			// An archive can hold members that are not ELF at all; skip them
			// rather than failing the whole read.
			return nil
		}
		out = append(out, syms...)
		return nil
	})
	if err != nil {
		return nil, err
	}
	sort.Strings(out)
	return dedup(out), nil
}

func definedInObject(data []byte) ([]string, error) {
	f, err := elf.NewFile(bytes.NewReader(data))
	if err != nil {
		return nil, err
	}
	defer f.Close()
	syms, err := f.Symbols()
	if err != nil && !errors.Is(err, elf.ErrNoSymbols) {
		return nil, err
	}
	var out []string
	for _, s := range syms {
		if s.Name == "" || !cIdent.MatchString(s.Name) {
			continue
		}
		switch elf.ST_BIND(s.Info) {
		case elf.STB_GLOBAL, elf.STB_WEAK:
		default:
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
		out = append(out, s.Name)
	}
	return out, nil
}

// forEachObject calls fn for a plain object, or for each member of an archive.
func forEachObject(path string, fn func(name string, data []byte) error) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	if !bytes.HasPrefix(data, []byte("!<arch>\n")) {
		return fn(filepath.Base(path), data)
	}
	return forEachArchiveMember(data, fn)
}

// forEachArchiveMember walks a System V / GNU `ar` archive.
//
// Long member names live in the "//" string table and are referenced as "/N";
// the symbol index members ("/" and "/SYM64/") carry no object code.
func forEachArchiveMember(data []byte, fn func(name string, data []byte) error) error {
	const hdrLen = 60
	pos := len("!<arch>\n")
	var longNames []byte
	for pos+hdrLen <= len(data) {
		hdr := data[pos : pos+hdrLen]
		if hdr[58] != 0x60 || hdr[59] != 0x0a {
			return fmt.Errorf("archive: bad member header at offset %d", pos)
		}
		rawName := strings.TrimRight(string(hdr[0:16]), " ")
		sizeField := strings.TrimSpace(string(hdr[48:58]))
		size, err := strconv.Atoi(sizeField)
		if err != nil {
			return fmt.Errorf("archive: bad member size %q at offset %d", sizeField, pos)
		}
		body := pos + hdrLen
		if body+size > len(data) {
			return fmt.Errorf("archive: member at %d runs past the end", pos)
		}
		member := data[body : body+size]

		switch {
		case rawName == "/" || rawName == "/SYM64/":
			// symbol index
		case rawName == "//":
			longNames = member
		default:
			name := rawName
			if strings.HasPrefix(name, "/") {
				if off, err := strconv.Atoi(name[1:]); err == nil && off < len(longNames) {
					end := bytes.IndexAny(longNames[off:], "/\n")
					if end < 0 {
						end = len(longNames) - off
					}
					name = string(longNames[off : off+end])
				}
			}
			name = strings.TrimSuffix(name, "/")
			if err := fn(name, member); err != nil {
				return err
			}
		}
		pos = body + size
		if size%2 == 1 {
			pos++
		}
	}
	return nil
}

// ArchiveMembers lists the member names of an archive, for diagnostics.
func ArchiveMembers(path string) ([]string, error) {
	var out []string
	err := forEachObject(path, func(name string, _ []byte) error {
		out = append(out, name)
		return nil
	})
	return out, err
}

// IsELF reports whether a file starts with the ELF magic, without reading the
// rest of it.
func IsELF(path string) bool {
	f, err := os.Open(path)
	if err != nil {
		return false
	}
	defer f.Close()
	var magic [4]byte
	if _, err := io.ReadFull(f, magic[:]); err != nil {
		return false
	}
	return magic == [4]byte{0x7f, 'E', 'L', 'F'}
}

func dedup(in []string) []string {
	if len(in) == 0 {
		return in
	}
	out := in[:1]
	for _, s := range in[1:] {
		if s != out[len(out)-1] {
			out = append(out, s)
		}
	}
	return out
}
