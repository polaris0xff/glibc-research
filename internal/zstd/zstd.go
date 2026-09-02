// Package zstd decodes the zstd format (RFC 8878) with nothing outside the
// standard library.
//
// pgb reads zstd because that is what cache.nixos.org serves its NARs as, and
// because an OCI layer may be compressed with it. Shelling out to a zstd
// binary would put a host dependency in the one place this project exists to
// remove it: pgb runs inside pinned environments that carry no zstd, and a
// tool that promises "one file you copy and run" cannot then ask for a
// decoder.
//
// Decoding only. Compression is not needed and is not here.
//
// SPDX-License-Identifier: MIT
package zstd

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"io"
)

const (
	magic         = 0xFD2FB528
	skippableLow  = 0x184D2A50
	skippableHigh = 0x184D2A5F

	// blockMax is the format's largest block. maxWindow bounds what a frame
	// may ask this process to hold; the reference decoder's own default limit
	// is the same 128 MiB.
	blockMax  = 1 << 17
	maxWindow = 1 << 27

	// trimSlack is how much delivered history is allowed to accumulate
	// before the window is compacted.
	trimSlack = 1 << 20
)

type corruptError string

func (e corruptError) Error() string { return "zstd: " + string(e) }

func errCorrupt(what string) error { return corruptError(what) }

// decoder holds the state one frame's blocks share.
type decoder struct {
	window     []byte // history, followed by output not yet delivered
	deliver    int    // index in window of the next byte to hand out
	frameStart int    // index in window where the current frame's output began
	rep        [3]int
	tables     seqTables
	winSize    int
	litRepeat  *huffTable // the tree a Treeless literals section reuses
}

// Reader decodes a zstd stream. Frames are read one after another, so a
// concatenation of frames reads as the concatenation of their contents.
type Reader struct {
	src io.Reader
	d   decoder
	err error

	inFrame  bool
	lastSeen bool
	checksum *xxh64
	contentN uint64
	haveSize bool
	produced uint64

	hdr [12]byte
}

// NewReader returns a Reader decoding r.
func NewReader(r io.Reader) *Reader { return &Reader{src: r} }

func (r *Reader) Read(p []byte) (int, error) {
	for r.d.deliver == len(r.d.window) {
		if r.err != nil {
			return 0, r.err
		}
		if err := r.advance(); err != nil {
			r.err = err
			return 0, err
		}
	}
	n := copy(p, r.d.window[r.d.deliver:])
	r.d.deliver += n
	r.trim()
	return n, nil
}

// trim drops history the window can no longer reach back to. Compaction moves
// the whole buffer, so it waits until there is a worthwhile amount to drop:
// doing it on every call would make a caller that reads a byte at a time
// quadratic.
func (r *Reader) trim() {
	excess := r.d.deliver - r.d.winSize
	if excess < trimSlack {
		return
	}
	drop := excess
	r.d.window = append(r.d.window[:0], r.d.window[drop:]...)
	r.d.deliver -= drop
	if r.d.frameStart -= drop; r.d.frameStart < 0 {
		r.d.frameStart = 0
	}
}

// advance decodes one more block, crossing into the next frame when the
// current one has ended.
func (r *Reader) advance() error {
	if r.inFrame && r.lastSeen {
		if err := r.finishFrame(); err != nil {
			return err
		}
	}
	if !r.inFrame {
		if err := r.readFrameHeader(); err != nil {
			return err
		}
	}
	return r.readBlock()
}

func (r *Reader) readFrameHeader() error {
	for {
		var m [4]byte
		if _, err := io.ReadFull(r.src, m[:]); err != nil {
			if err == io.ErrUnexpectedEOF {
				return errCorrupt("truncated frame magic")
			}
			return err // io.EOF between frames is the clean end of the stream
		}
		v := binary.LittleEndian.Uint32(m[:])
		if v >= skippableLow && v <= skippableHigh {
			var sz [4]byte
			if _, err := io.ReadFull(r.src, sz[:]); err != nil {
				return errCorrupt("truncated skippable frame size")
			}
			if _, err := io.CopyN(io.Discard, r.src, int64(binary.LittleEndian.Uint32(sz[:]))); err != nil {
				return errCorrupt("truncated skippable frame")
			}
			continue
		}
		if v != magic {
			return errCorrupt(fmt.Sprintf("not a zstd frame: magic %#08x", v))
		}
		break
	}

	var desc [1]byte
	if _, err := io.ReadFull(r.src, desc[:]); err != nil {
		return errCorrupt("truncated frame header")
	}
	fcsFlag := desc[0] >> 6 & 3
	singleSegment := desc[0]>>5&1 == 1
	checksumFlag := desc[0]>>2&1 == 1
	dictFlag := desc[0] & 3
	if desc[0]>>3&1 != 0 {
		return errCorrupt("reserved bit set in the frame header")
	}

	winSize := 0
	if !singleSegment {
		var w [1]byte
		if _, err := io.ReadFull(r.src, w[:]); err != nil {
			return errCorrupt("truncated window descriptor")
		}
		exp, mant := int(w[0]>>3), int(w[0]&7)
		if exp > 21 {
			return errCorrupt("window descriptor out of range")
		}
		base := 1 << (10 + exp)
		winSize = base + (base/8)*mant
	}

	dictLen := []int{0, 1, 2, 4}[dictFlag]
	fcsLen := []int{0, 2, 4, 8}[fcsFlag]
	if fcsFlag == 0 && singleSegment {
		fcsLen = 1
	}
	if n := dictLen + fcsLen; n > 0 {
		if _, err := io.ReadFull(r.src, r.hdr[:n]); err != nil {
			return errCorrupt("truncated frame header")
		}
	}
	for i := range dictLen {
		if r.hdr[i] != 0 {
			return errCorrupt("frame needs a dictionary, which pgb does not carry")
		}
	}
	r.haveSize = fcsLen > 0
	r.contentN = 0
	switch fcsLen {
	case 1:
		r.contentN = uint64(r.hdr[dictLen])
	case 2:
		r.contentN = uint64(binary.LittleEndian.Uint16(r.hdr[dictLen:])) + 256
	case 4:
		r.contentN = uint64(binary.LittleEndian.Uint32(r.hdr[dictLen:]))
	case 8:
		r.contentN = binary.LittleEndian.Uint64(r.hdr[dictLen:])
	}
	if singleSegment {
		if r.contentN > maxWindow {
			return errCorrupt("single-segment frame larger than the window limit")
		}
		winSize = int(r.contentN)
	}
	if winSize > maxWindow {
		return errCorrupt(fmt.Sprintf("frame asks for a %d-byte window, over the %d-byte limit",
			winSize, maxWindow))
	}

	// Undelivered output survives the frame boundary; nothing else does, and
	// frameStart is what keeps a match in this frame from reaching into the
	// previous one.
	r.d = decoder{
		window:     r.d.window,
		deliver:    r.d.deliver,
		frameStart: len(r.d.window),
		rep:        [3]int{1, 4, 8},
		winSize:    winSize,
	}
	r.produced = 0
	r.inFrame = true
	r.lastSeen = false
	r.checksum = nil
	if checksumFlag {
		r.checksum = newXXH64()
	}
	return nil
}

func (r *Reader) readBlock() error {
	var h [3]byte
	if _, err := io.ReadFull(r.src, h[:]); err != nil {
		return errCorrupt("truncated block header")
	}
	raw := uint32(h[0]) | uint32(h[1])<<8 | uint32(h[2])<<16
	last := raw&1 == 1
	kind := raw >> 1 & 3
	size := int(raw >> 3)
	if size > blockMax {
		return errCorrupt("block larger than the format allows")
	}

	before := len(r.d.window)
	switch kind {
	case 0: // raw
		r.d.window = append(r.d.window, make([]byte, size)...)
		if _, err := io.ReadFull(r.src, r.d.window[before:]); err != nil {
			return errCorrupt("truncated raw block")
		}
	case 1: // one byte, repeated
		var b [1]byte
		if _, err := io.ReadFull(r.src, b[:]); err != nil {
			return errCorrupt("truncated RLE block")
		}
		for range size {
			r.d.window = append(r.d.window, b[0])
		}
	case 2:
		body := make([]byte, size)
		if _, err := io.ReadFull(r.src, body); err != nil {
			return errCorrupt("truncated compressed block")
		}
		if err := r.d.decodeCompressedBlock(body); err != nil {
			return err
		}
	default:
		return errCorrupt("reserved block type")
	}

	if r.checksum != nil {
		r.checksum.write(r.d.window[before:])
	}
	r.produced += uint64(len(r.d.window) - before)
	r.lastSeen = last
	return nil
}

// finishFrame validates the frame that just ended and consumes its checksum.
func (r *Reader) finishFrame() error {
	if r.haveSize && r.produced != r.contentN {
		return errCorrupt(fmt.Sprintf("frame declared %d bytes and produced %d",
			r.contentN, r.produced))
	}
	if r.checksum != nil {
		var c [4]byte
		if _, err := io.ReadFull(r.src, c[:]); err != nil {
			return errCorrupt("truncated frame checksum")
		}
		if want, got := binary.LittleEndian.Uint32(c[:]), uint32(r.checksum.sum()); got != want {
			return errCorrupt(fmt.Sprintf("frame checksum %#08x, expected %#08x", got, want))
		}
	}
	r.inFrame = false
	return nil
}

// decodeCompressedBlock splits a block into its literals and its sequences.
func (d *decoder) decodeCompressedBlock(body []byte) error {
	lit, used, err := d.readLiterals(body)
	if err != nil {
		return err
	}
	return d.decodeSequences(body[used:], lit)
}

// readLiterals decodes the literals section, returning it and its size.
func (d *decoder) readLiterals(src []byte) ([]byte, int, error) {
	if len(src) < 1 {
		return nil, 0, errCorrupt("empty block body")
	}
	kind := src[0] & 3
	format := src[0] >> 2 & 3

	if kind < 2 { // raw, or one byte repeated
		var regen, hdr int
		switch format {
		case 0, 2:
			regen, hdr = int(src[0]>>3), 1
		case 1:
			if len(src) < 2 {
				return nil, 0, errCorrupt("truncated literals header")
			}
			regen, hdr = int(src[0]>>4)|int(src[1])<<4, 2
		default:
			if len(src) < 3 {
				return nil, 0, errCorrupt("truncated literals header")
			}
			regen, hdr = int(src[0]>>4)|int(src[1])<<4|int(src[2])<<12, 3
		}
		if kind == 0 {
			if hdr+regen > len(src) {
				return nil, 0, errCorrupt("raw literals run past the block")
			}
			return src[hdr : hdr+regen], hdr + regen, nil
		}
		if hdr >= len(src) {
			return nil, 0, errCorrupt("truncated RLE literals")
		}
		lit := make([]byte, regen)
		for i := range lit {
			lit[i] = src[hdr]
		}
		return lit, hdr + 1, nil
	}

	// Huffman-coded, with the tree either sent here or reused from the last
	// block that sent one.
	var regen, comp, hdr, streams int
	switch format {
	case 0, 1:
		if len(src) < 3 {
			return nil, 0, errCorrupt("truncated literals header")
		}
		v := int(src[0])>>4 | int(src[1])<<4 | int(src[2])<<12
		regen, comp, hdr, streams = v&0x3FF, v>>10&0x3FF, 3, 1
		if format == 1 {
			streams = 4
		}
	case 2:
		if len(src) < 4 {
			return nil, 0, errCorrupt("truncated literals header")
		}
		v := int(src[0])>>4 | int(src[1])<<4 | int(src[2])<<12 | int(src[3])<<20
		regen, comp, hdr, streams = v&0x3FFF, v>>14&0x3FFF, 4, 4
	default:
		if len(src) < 5 {
			return nil, 0, errCorrupt("truncated literals header")
		}
		v := int(src[0])>>4 | int(src[1])<<4 | int(src[2])<<12 | int(src[3])<<20 | int(src[4])<<28
		regen, comp, hdr, streams = v&0x3FFFF, v>>18&0x3FFFF, 5, 4
	}
	if hdr+comp > len(src) {
		return nil, 0, errCorrupt("literals section runs past the block")
	}
	section := src[hdr : hdr+comp]

	table := d.litRepeat
	if kind == 2 {
		t, used, err := readHuffTable(section)
		if err != nil {
			return nil, 0, err
		}
		table, section = t, section[used:]
		d.litRepeat = t
	} else if table == nil {
		return nil, 0, errCorrupt("treeless literals with no tree to reuse")
	}

	lit := make([]byte, regen)
	if err := table.decodeLiterals(section, streams, lit); err != nil {
		return nil, 0, err
	}
	return lit, hdr + comp, nil
}

// Decode reads a whole zstd stream into memory, for callers that already hold
// the compressed bytes and want the result in one piece.
func Decode(src []byte) ([]byte, error) {
	return io.ReadAll(NewReader(bytes.NewReader(src)))
}
