// fse.go — Finite State Entropy tables: reading normalised counts, building a
// decoding table, and the two-state decompressor zstd uses for Huffman
// weights.
//
// SPDX-License-Identifier: MIT
package zstd

import "math/bits"

// fseTable is a built decoding table: one entry per state.
type fseTable struct {
	log    int
	symbol []uint8
	nbits  []uint8
	base   []uint16
}

// build turns normalised counts into a decoding table. A count of -1 means a
// symbol of less-than-one probability, which is placed at the top of the table
// so it always costs the full table log to reach.
func buildFSE(counts []int16, log int) (*fseTable, error) {
	size := 1 << log
	t := &fseTable{
		log:    log,
		symbol: make([]uint8, size),
		nbits:  make([]uint8, size),
		base:   make([]uint16, size),
	}
	high := size - 1
	next := make([]uint16, len(counts))
	for s, c := range counts {
		if c == -1 {
			t.symbol[high] = uint8(s)
			high--
			next[s] = 1
			continue
		}
		next[s] = uint16(c)
	}

	step := (size >> 1) + (size >> 3) + 3
	mask := size - 1
	pos := 0
	for s, c := range counts {
		for range c {
			t.symbol[pos] = uint8(s)
			pos = (pos + step) & mask
			for pos > high {
				pos = (pos + step) & mask
			}
		}
	}
	if pos != 0 {
		return nil, errCorrupt("FSE table does not close")
	}

	for u := range size {
		s := t.symbol[u]
		state := next[s]
		next[s]++
		nb := log - highBit(uint32(state))
		t.nbits[u] = uint8(nb)
		t.base[u] = uint16(int(state)<<nb - size)
	}
	return t, nil
}

// step advances one state, returning the symbol it held.
func (t *fseTable) step(state *int, br *bitReader) uint8 {
	sym := t.symbol[*state]
	nb := int(t.nbits[*state])
	*state = int(t.base[*state]) + int(br.read(nb))
	return sym
}

func highBit(v uint32) int { return 31 - bits.LeadingZeros32(v) }

// readNCount reads a normalised-count table header, returning the counts, the
// accuracy log and how many bytes the header occupied.
func readNCount(src []byte, maxSymbol, maxLog int) ([]int16, int, int, error) {
	if len(src) < 1 {
		return nil, 0, 0, errCorrupt("truncated FSE header")
	}
	// A padded copy removes the reference decoder's short-input special case:
	// the reader always has four readable bytes ahead of it.
	buf := make([]byte, len(src)+16)
	copy(buf, src)

	read32 := func(i int) uint32 {
		return uint32(buf[i]) | uint32(buf[i+1])<<8 | uint32(buf[i+2])<<16 | uint32(buf[i+3])<<24
	}

	ip := 0
	stream := read32(ip)
	log := int(stream&0xF) + 5
	if log > maxLog {
		return nil, 0, 0, errCorrupt("FSE accuracy log too large")
	}
	stream >>= 4
	bitCount := 4

	counts := make([]int16, maxSymbol+1)
	remaining := 1<<log + 1
	threshold := 1 << log
	nbBits := log + 1
	charnum := 0
	previous0 := false

	// The padding gives the reader four readable bytes past the header; going
	// beyond that means the counts never closed, which is corruption rather
	// than a position to clamp to.
	overrun := false
	reload := func() {
		ip += bitCount >> 3
		bitCount &= 7
		if ip+4 > len(buf) {
			overrun = true
			return
		}
		stream = read32(ip) >> bitCount
	}

	for remaining > 1 && charnum <= maxSymbol && !overrun {
		if previous0 {
			// Each 0b11 pair is another run of three zero-probability
			// symbols; the first pair that is not 0b11 ends the run and its
			// value is how many more follow. Six pairs are taken at a time,
			// reloading each round so the window never runs dry mid-run.
			for stream&0xFFF == 0xFFF {
				charnum += 18
				bitCount += 12
				reload()
			}
			for stream&3 == 3 {
				charnum += 3
				stream >>= 2
				bitCount += 2
			}
			charnum += int(stream & 3)
			bitCount += 2
			if charnum >= maxSymbol+1 {
				break
			}
			reload()
			previous0 = false
			continue
		}

		max := (2*threshold - 1) - remaining
		var count int
		if int(stream&uint32(threshold-1)) < max {
			count = int(stream & uint32(threshold-1))
			bitCount += nbBits - 1
		} else {
			count = int(stream & uint32(2*threshold-1))
			if count >= threshold {
				count -= max
			}
			bitCount += nbBits
		}
		count--
		if count >= 0 {
			remaining -= count
		} else {
			remaining += count
		}
		counts[charnum] = int16(count)
		charnum++
		previous0 = count == 0

		if remaining < threshold {
			if remaining <= 1 {
				break
			}
			nbBits = highBit(uint32(remaining)) + 1
			threshold = 1 << (nbBits - 1)
		}
		if charnum >= maxSymbol+1 {
			break
		}
		reload()
	}
	if overrun {
		return nil, 0, 0, errCorrupt("FSE header runs past its section")
	}
	if remaining != 1 {
		return nil, 0, 0, errCorrupt("FSE counts do not sum to the table size")
	}
	used := ip + (bitCount+7)/8
	if used > len(src) {
		return nil, 0, 0, errCorrupt("FSE header runs past its section")
	}
	return counts[:charnum], log, used, nil
}

// decompress runs the two interleaved states zstd uses for Huffman weights.
func (t *fseTable) decompress(src []byte, max int) ([]byte, error) {
	br, err := newBitReader(src)
	if err != nil {
		return nil, err
	}
	state := [2]int{int(br.read(t.log)), int(br.read(t.log))}
	out := make([]byte, 0, max)
	// The two states take turns. When the stream runs out, the state whose
	// turn it is still holds a symbol, and that symbol is the last one.
	for i := 0; ; i++ {
		if len(out) == max {
			return nil, errCorrupt("FSE output longer than the table allows")
		}
		cur := &state[i&1]
		if br.over {
			out = append(out, t.symbol[*cur])
			break
		}
		out = append(out, t.step(cur, br))
	}
	return out, nil
}
