// xxh64.go — XXH64, the hash a zstd frame's optional content checksum uses.
//
// SPDX-License-Identifier: MIT
package zstd

import (
	"encoding/binary"
	"math/bits"
)

const (
	prime1 uint64 = 11400714785074694791
	prime2 uint64 = 14029467366897019727
	prime3 uint64 = 1609587929392839161
	prime4 uint64 = 9650029242287828579
	prime5 uint64 = 2870177450012600261
)

type xxh64 struct {
	v      [4]uint64
	total  uint64
	buf    [32]byte
	buffed int
}

func newXXH64() *xxh64 {
	// The seed is zero, so the four accumulators start at their canonical
	// offsets from it. The sums are built at run time because two of them
	// wrap, which a constant expression is not allowed to do.
	var h xxh64
	h.v[0] = prime1
	h.v[0] += prime2
	h.v[1] = prime2
	h.v[3] -= prime1
	return &h
}

func round(acc, input uint64) uint64 {
	acc += input * prime2
	return bits.RotateLeft64(acc, 31) * prime1
}

func mergeRound(acc, val uint64) uint64 {
	acc ^= round(0, val)
	return acc*prime1 + prime4
}

func (h *xxh64) write(p []byte) {
	h.total += uint64(len(p))
	if h.buffed > 0 {
		n := copy(h.buf[h.buffed:], p)
		h.buffed += n
		p = p[n:]
		if h.buffed < 32 {
			return
		}
		h.consume(h.buf[:])
		h.buffed = 0
	}
	for len(p) >= 32 {
		h.consume(p[:32])
		p = p[32:]
	}
	if len(p) > 0 {
		h.buffed = copy(h.buf[:], p)
	}
}

func (h *xxh64) consume(b []byte) {
	h.v[0] = round(h.v[0], binary.LittleEndian.Uint64(b[0:]))
	h.v[1] = round(h.v[1], binary.LittleEndian.Uint64(b[8:]))
	h.v[2] = round(h.v[2], binary.LittleEndian.Uint64(b[16:]))
	h.v[3] = round(h.v[3], binary.LittleEndian.Uint64(b[24:]))
}

func (h *xxh64) sum() uint64 {
	var acc uint64
	if h.total >= 32 {
		acc = bits.RotateLeft64(h.v[0], 1) + bits.RotateLeft64(h.v[1], 7) +
			bits.RotateLeft64(h.v[2], 12) + bits.RotateLeft64(h.v[3], 18)
		acc = mergeRound(acc, h.v[0])
		acc = mergeRound(acc, h.v[1])
		acc = mergeRound(acc, h.v[2])
		acc = mergeRound(acc, h.v[3])
	} else {
		acc = h.v[2] + prime5
	}
	acc += h.total

	tail := h.buf[:h.buffed]
	for len(tail) >= 8 {
		acc ^= round(0, binary.LittleEndian.Uint64(tail))
		acc = bits.RotateLeft64(acc, 27)*prime1 + prime4
		tail = tail[8:]
	}
	if len(tail) >= 4 {
		acc ^= uint64(binary.LittleEndian.Uint32(tail)) * prime1
		acc = bits.RotateLeft64(acc, 23)*prime2 + prime3
		tail = tail[4:]
	}
	for _, b := range tail {
		acc ^= uint64(b) * prime5
		acc = bits.RotateLeft64(acc, 11) * prime1
	}
	acc ^= acc >> 33
	acc *= prime2
	acc ^= acc >> 29
	acc *= prime3
	acc ^= acc >> 32
	return acc
}
