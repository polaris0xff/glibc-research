// seq.go — the sequences section: three FSE-coded symbol streams interleaved
// in one backward bitstream, and the literal/match copies they describe.
//
// SPDX-License-Identifier: MIT
package zstd

var (
	llBase = [36]uint32{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
		16, 18, 20, 22, 24, 28, 32, 40, 48, 64, 0x80, 0x100, 0x200, 0x400,
		0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000}
	llBits = [36]uint8{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16}

	mlBase = [53]uint32{3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
		19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34,
		35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
		0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003}
	mlBits = [53]uint8{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11,
		12, 13, 14, 15, 16}

	llDefault = []int16{4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1,
		2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1, -1, -1, -1, -1}
	mlDefault = []int16{1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1,
		-1, -1, -1, -1, -1}
	ofDefault = []int16{1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
		1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1}
)

// seqMode is how one of the three symbol tables is described.
type seqMode uint8

const (
	modePredefined seqMode = 0
	modeRLE        seqMode = 1
	modeFSE        seqMode = 2
	modeRepeat     seqMode = 3
)

// seqTables are carried across blocks so a Repeat mode has something to
// repeat.
type seqTables struct {
	ll, of, ml *fseTable
}

// rleTable is a one-state table: the symbol is fixed and costs no bits.
func rleTable(sym uint8) *fseTable {
	return &fseTable{log: 0, symbol: []uint8{sym}, nbits: []uint8{0}, base: []uint16{0}}
}

type tableSpec struct {
	name     string
	def      []int16
	defLog   int
	maxSym   int
	maxLog   int
	existing **fseTable
}

// readSeqTable resolves one of the three tables from its mode.
func readSeqTable(src []byte, mode seqMode, spec tableSpec) (int, error) {
	switch mode {
	case modePredefined:
		t, err := buildFSE(spec.def, spec.defLog)
		if err != nil {
			return 0, err
		}
		*spec.existing = t
		return 0, nil
	case modeRLE:
		if len(src) < 1 {
			return 0, errCorrupt("truncated " + spec.name + " RLE table")
		}
		if int(src[0]) > spec.maxSym {
			return 0, errCorrupt(spec.name + " RLE symbol out of range")
		}
		*spec.existing = rleTable(src[0])
		return 1, nil
	case modeFSE:
		counts, log, used, err := readNCount(src, spec.maxSym, spec.maxLog)
		if err != nil {
			return 0, err
		}
		t, err := buildFSE(counts, log)
		if err != nil {
			return 0, err
		}
		*spec.existing = t
		return used, nil
	default:
		if *spec.existing == nil {
			return 0, errCorrupt(spec.name + " asks to repeat a table that was never sent")
		}
		return 0, nil
	}
}

// decodeSequences reads the section header and tables, then replays every
// sequence into out, drawing literals from lit and matches from the window.
func (d *decoder) decodeSequences(src, lit []byte) error {
	if len(src) == 0 {
		return d.emitLiterals(lit)
	}
	i := 0
	n := int(src[i])
	i++
	switch {
	case n == 0:
		if i != len(src) {
			return errCorrupt("trailing bytes after an empty sequences section")
		}
		return d.emitLiterals(lit)
	case n < 128:
	case n < 255:
		if i >= len(src) {
			return errCorrupt("truncated sequence count")
		}
		n = (n-128)<<8 + int(src[i])
		i++
	default:
		if i+2 > len(src) {
			return errCorrupt("truncated sequence count")
		}
		n = int(src[i]) + int(src[i+1])<<8 + 0x7F00
		i += 2
	}
	if i >= len(src) {
		return errCorrupt("truncated sequence modes")
	}
	modes := src[i]
	i++
	llMode := seqMode(modes >> 6 & 3)
	ofMode := seqMode(modes >> 4 & 3)
	mlMode := seqMode(modes >> 2 & 3)
	if modes&3 != 0 {
		return errCorrupt("reserved bits set in the sequence modes")
	}

	for _, spec := range []struct {
		mode seqMode
		spec tableSpec
	}{
		{llMode, tableSpec{"literal-length", llDefault, 6, 35, 9, &d.tables.ll}},
		{ofMode, tableSpec{"offset", ofDefault, 5, 31, 8, &d.tables.of}},
		{mlMode, tableSpec{"match-length", mlDefault, 6, 52, 9, &d.tables.ml}},
	} {
		used, err := readSeqTable(src[i:], spec.mode, spec.spec)
		if err != nil {
			return err
		}
		i += used
	}

	br, err := newBitReader(src[i:])
	if err != nil {
		return err
	}
	llState := int(br.read(d.tables.ll.log))
	ofState := int(br.read(d.tables.of.log))
	mlState := int(br.read(d.tables.ml.log))

	litPos := 0
	for s := 0; s < n; s++ {
		llSym := d.tables.ll.symbol[llState]
		ofSym := d.tables.of.symbol[ofState]
		mlSym := d.tables.ml.symbol[mlState]
		if int(llSym) >= len(llBase) || int(mlSym) >= len(mlBase) || ofSym > 31 {
			return errCorrupt("sequence symbol out of range")
		}

		// Offset first, then match length, then literal length.
		offsetValue := uint64(1)<<ofSym + uint64(br.read(int(ofSym)))
		matchLen := int(mlBase[mlSym]) + int(br.read(int(mlBits[mlSym])))
		litLen := int(llBase[llSym]) + int(br.read(int(llBits[llSym])))

		if s < n-1 {
			d.tables.ll.step(&llState, br)
			d.tables.ml.step(&mlState, br)
			d.tables.of.step(&ofState, br)
		}
		if br.over {
			return errCorrupt("sequence bitstream ended early")
		}

		offset, err := d.resolveOffset(offsetValue, litLen)
		if err != nil {
			return err
		}
		if litPos+litLen > len(lit) {
			return errCorrupt("sequence asks for more literals than the block holds")
		}
		d.window = append(d.window, lit[litPos:litPos+litLen]...)
		litPos += litLen
		if err := d.copyMatch(offset, matchLen); err != nil {
			return err
		}
	}
	d.window = append(d.window, lit[litPos:]...)
	return nil
}

// resolveOffset turns an offset value into a real distance, maintaining the
// three repeated offsets.
func (d *decoder) resolveOffset(value uint64, litLen int) (int, error) {
	if value > 3 {
		off := int(value - 3)
		d.rep[2], d.rep[1], d.rep[0] = d.rep[1], d.rep[0], off
		return off, nil
	}
	code := int(value) - 1
	if litLen == 0 {
		code++
	}
	if code == 0 {
		return d.rep[0], nil
	}
	var off int
	if code == 3 {
		off = d.rep[0] - 1
	} else {
		off = d.rep[code]
	}
	if off <= 0 {
		return 0, errCorrupt("repeated offset resolves to zero")
	}
	if code >= 2 {
		d.rep[2] = d.rep[1]
	}
	d.rep[1] = d.rep[0]
	d.rep[0] = off
	return off, nil
}

// copyMatch appends matchLen bytes from offset back in the window. The copy is
// byte at a time because a match may overlap its own source.
func (d *decoder) copyMatch(offset, matchLen int) error {
	if offset <= 0 || offset > len(d.window)-d.frameStart {
		return errCorrupt("match offset reaches before this frame's output")
	}
	start := len(d.window) - offset
	for i := range matchLen {
		d.window = append(d.window, d.window[start+i])
	}
	return nil
}

// emitLiterals handles a block that is literals and nothing else.
func (d *decoder) emitLiterals(lit []byte) error {
	d.window = append(d.window, lit...)
	return nil
}
