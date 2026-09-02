// huff.go — the Huffman decoder for a compressed literals section.
//
// The tree is described by one weight per symbol, either packed four bits at a
// time or itself FSE-compressed. The weight of the last symbol is not stored:
// it is whatever makes the total a power of two, which is also the check that
// the description is well formed.
//
// SPDX-License-Identifier: MIT
package zstd

// huffTable is a flat decoding table indexed by the next log bits.
type huffTable struct {
	log    int
	symbol []uint8
	nbits  []uint8
}

const huffMaxLog = 12

// readHuffTable reads a tree description, returning the table and its size in
// bytes.
func readHuffTable(src []byte) (*huffTable, int, error) {
	if len(src) < 1 {
		return nil, 0, errCorrupt("truncated Huffman description")
	}
	header := int(src[0])
	var weights []uint8
	var used int

	if header >= 128 {
		n := header - 127
		nbytes := (n + 1) / 2
		if 1+nbytes > len(src) {
			return nil, 0, errCorrupt("truncated Huffman weights")
		}
		weights = make([]uint8, n)
		for i := range n {
			b := src[1+i/2]
			if i%2 == 0 {
				weights[i] = b >> 4
			} else {
				weights[i] = b & 0xF
			}
		}
		used = 1 + nbytes
	} else {
		if 1+header > len(src) {
			return nil, 0, errCorrupt("truncated Huffman weight stream")
		}
		body := src[1 : 1+header]
		counts, log, hdr, err := readNCount(body, 255, 6)
		if err != nil {
			return nil, 0, err
		}
		table, err := buildFSE(counts, log)
		if err != nil {
			return nil, 0, err
		}
		w, err := table.decompress(body[hdr:], 255)
		if err != nil {
			return nil, 0, err
		}
		weights = w
		used = 1 + header
	}

	// The stored weights account for all but the last symbol; the remainder
	// of the next power of two is the last symbol's own weight.
	total := uint32(0)
	for _, w := range weights {
		if int(w) > huffMaxLog {
			return nil, 0, errCorrupt("Huffman weight out of range")
		}
		total += (1 << w) >> 1
	}
	if total == 0 {
		return nil, 0, errCorrupt("Huffman description is empty")
	}
	log := highBit(total) + 1
	if log > huffMaxLog {
		return nil, 0, errCorrupt("Huffman table log too large")
	}
	rest := uint32(1)<<log - total
	if rest == 0 || rest&(rest-1) != 0 {
		return nil, 0, errCorrupt("Huffman weights do not close to a power of two")
	}
	weights = append(weights, uint8(highBit(rest)+1))

	var rank [huffMaxLog + 2]int
	for _, w := range weights {
		rank[w]++
	}
	var start [huffMaxLog + 2]int
	pos := 0
	for w := 1; w <= log; w++ {
		start[w] = pos
		pos += rank[w] << (w - 1)
	}
	if pos != 1<<log {
		return nil, 0, errCorrupt("Huffman table does not fill")
	}

	t := &huffTable{log: log, symbol: make([]uint8, 1<<log), nbits: make([]uint8, 1<<log)}
	for s, w := range weights {
		if w == 0 {
			continue
		}
		span := 1 << (w - 1)
		nb := uint8(log + 1 - int(w))
		for i := range span {
			t.symbol[start[w]+i] = uint8(s)
			t.nbits[start[w]+i] = nb
		}
		start[w] += span
	}
	return t, used, nil
}

// decodeStream fills out with symbols from one Huffman bitstream. The table is
// indexed by a full log bits and only the symbol's own length is consumed.
func (t *huffTable) decodeStream(src []byte, out []byte) error {
	if len(out) == 0 {
		return nil
	}
	br, err := newBitReader(src)
	if err != nil {
		return err
	}
	for i := range out {
		idx := int(br.peek(t.log))
		out[i] = t.symbol[idx]
		br.skip(int(t.nbits[idx]))
	}
	if br.over {
		return errCorrupt("Huffman stream ended early")
	}
	return nil
}

// decodeLiterals decodes either the single- or the four-stream layout.
func (t *huffTable) decodeLiterals(src []byte, streams int, out []byte) error {
	if streams == 1 {
		return t.decodeStream(src, out)
	}
	if len(src) < 6 {
		return errCorrupt("truncated Huffman jump table")
	}
	s1 := int(src[0]) | int(src[1])<<8
	s2 := int(src[2]) | int(src[3])<<8
	s3 := int(src[4]) | int(src[5])<<8
	body := src[6:]
	if s1+s2+s3 > len(body) {
		return errCorrupt("Huffman jump table overruns the section")
	}
	s4 := len(body) - s1 - s2 - s3

	seg := (len(out) + 3) / 4
	bounds := [4]int{seg, seg, seg, len(out) - 3*seg}
	if bounds[3] < 0 {
		return errCorrupt("literals section too small for four streams")
	}
	sizes := [4]int{s1, s2, s3, s4}
	off, pos := 0, 0
	for i := range 4 {
		if err := t.decodeStream(body[off:off+sizes[i]], out[pos:pos+bounds[i]]); err != nil {
			return err
		}
		off += sizes[i]
		pos += bounds[i]
	}
	return nil
}
