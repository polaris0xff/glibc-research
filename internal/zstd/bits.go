// bits.go — the backward bit reader zstd entropy streams are written for.
//
// A zstd bitstream is stored forward in memory but read from the last byte
// toward the first, most significant bit first. The final byte carries a set
// bit marking where the padding ends; everything below it is data.
//
// SPDX-License-Identifier: MIT
package zstd

import "math/bits"

// bitReader hands out bits from the end of a stream. Reading past the end
// yields zeros and sets over, which is how the entropy decoders learn that a
// stream is spent.
type bitReader struct {
	src  []byte
	next int    // index of the next byte to load, moving down
	acc  uint64 // valid bits live in the low n, most significant first out
	n    int
	over bool
}

func newBitReader(src []byte) (*bitReader, error) {
	if len(src) == 0 {
		return nil, errCorrupt("empty bitstream")
	}
	last := src[len(src)-1]
	if last == 0 {
		return nil, errCorrupt("bitstream has no end marker")
	}
	usable := 7 - bits.LeadingZeros8(last)
	return &bitReader{
		src:  src,
		next: len(src) - 2,
		acc:  uint64(last) & (1<<usable - 1),
		n:    usable,
	}, nil
}

func (b *bitReader) fill() {
	for b.n <= 56 && b.next >= 0 {
		b.acc = b.acc<<8 | uint64(b.src[b.next])
		b.n += 8
		b.next--
	}
}

// read returns the next n bits, at most 32 at a time.
func (b *bitReader) read(n int) uint32 {
	if n == 0 {
		return 0
	}
	if b.n < n {
		b.fill()
	}
	if b.n < n {
		short := n - b.n
		v := uint32(b.acc&(1<<b.n-1)) << short
		b.acc, b.n, b.over = 0, 0, true
		return v
	}
	b.n -= n
	return uint32(b.acc>>b.n) & (1<<n - 1)
}

// peek returns the next n bits without consuming them. Past the end it pads
// with zeros, which is what a table-log-wide window does at the tail of a
// well-formed stream, so it never marks the reader spent.
func (b *bitReader) peek(n int) uint32 {
	if b.n < n {
		b.fill()
	}
	if b.n < n {
		return uint32(b.acc&(1<<b.n-1)) << (n - b.n)
	}
	return uint32(b.acc>>(b.n-n)) & (1<<n - 1)
}

// skip consumes n bits peek has already returned.
func (b *bitReader) skip(n int) {
	if b.n < n {
		b.fill()
	}
	if b.n < n {
		b.acc, b.n, b.over = 0, 0, true
		return
	}
	b.n -= n
}
