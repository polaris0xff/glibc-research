// selftest.go — the decoder against frames the reference encoder produced,
// offline.
//
// The frames are carried here because pgb has no zstd encoder to make them
// with: each is `zstd` at the named level over a subject this file also
// generates, so a mismatch is the decoder's and not the fixture's. Between
// them they cover raw, RLE and Huffman literals in one and four streams,
// predefined and FSE-coded sequence tables, several blocks in a frame,
// concatenated frames, and a skippable frame.
//
// SPDX-License-Identifier: MIT
package zstd

import (
	"bytes"
	"encoding/base64"
	"encoding/binary"
	"fmt"
	"io"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// subjects generates the plaintext each carried frame was made from.
func subjects() map[string][]byte {
	m := map[string][]byte{}
	m["empty"] = []byte{}
	m["one"] = []byte{7}
	m["run"] = []byte(strings.Repeat("Z", 5000))

	var alt []byte
	for i := range 200000 {
		alt = append(alt, byte(i%2))
	}
	m["alt"] = alt

	var counter []byte
	for i := range 20000 {
		counter = append(counter, []byte(fmt.Sprintf("%08d,", i))...)
	}
	m["counter"] = counter

	var prose []byte
	for i := range 700 {
		prose = append(prose, []byte(fmt.Sprintf(
			"the %d quick brown foxes jump over %d lazy dogs while the band plays on\n", i, i*7))...)
	}
	m["prose"] = prose
	return m
}

// dribble hands over one byte per Read, so the decoder's frame and block
// boundaries cannot depend on how much the caller supplies at a time.
type dribble struct{ b []byte }

func (d *dribble) Read(p []byte) (int, error) {
	if len(d.b) == 0 {
		return 0, io.EOF
	}
	if len(p) == 0 {
		return 0, nil
	}
	p[0] = d.b[0]
	d.b = d.b[1:]
	return 1, nil
}

// Selftest decodes every carried frame, then checks that damage is refused.
func Selftest() *selftest.Report {
	r := selftest.New("zstd")
	subs := subjects()

	for _, name := range []string{"empty-1", "one-1", "run-1", "run-19",
		"alt-1", "counter-1", "prose-1", "prose-12"} {
		src, err := base64.StdEncoding.DecodeString(frames[name])
		if err != nil {
			r.Fail(name, err.Error(), "decodes")
			continue
		}
		want := subs[strings.SplitN(name, "-", 2)[0]]
		got, err := Decode(src)
		if err != nil {
			r.Fail(name, err.Error(), "no error")
			continue
		}
		r.CheckBool(fmt.Sprintf("%s decodes to its %d bytes", name, len(want)),
			bytes.Equal(got, want), true)

		var out bytes.Buffer
		if _, err := io.Copy(&out, NewReader(&dribble{b: src})); err != nil {
			r.Fail(name+" one byte at a time", err.Error(), "no error")
			continue
		}
		r.CheckBool(name+" is the same one byte at a time", bytes.Equal(out.Bytes(), want), true)
	}

	// Frames run back to back are the concatenation of their contents, and a
	// skippable frame is stepped over rather than decoded.
	a, _ := base64.StdEncoding.DecodeString(frames["prose-1"])
	b, _ := base64.StdEncoding.DecodeString(frames["run-1"])
	joined, err := Decode(append(append([]byte{}, a...), b...))
	r.CheckBool("two frames decode to two contents joined",
		err == nil && bytes.Equal(joined, append(append([]byte{}, subs["prose"]...), subs["run"]...)), true)

	skip := make([]byte, 8)
	binary.LittleEndian.PutUint32(skip, 0x184D2A50)
	binary.LittleEndian.PutUint32(skip[4:], 4)
	skip = append(skip, 'p', 'a', 'd', '!')
	withSkip, err := Decode(append(append(append([]byte{}, skip...), b...), skip...))
	r.CheckBool("a skippable frame is stepped over",
		err == nil && bytes.Equal(withSkip, subs["run"]), true)

	// Damage must be refused rather than decoded to something else.
	full, _ := base64.StdEncoding.DecodeString(frames["counter-1"])
	truncOK := 0
	for cut := 1; cut < 600; cut += 37 {
		if _, err := Decode(full[:len(full)-cut]); err == nil {
			truncOK++
		}
	}
	r.Check("truncations accepted", itoa(truncOK), "0")

	// A damaged frame may be refused, and it may still decode correctly when
	// the flip lands somewhere the output does not depend on. What it must
	// never do is return DIFFERENT content without an error.
	flipped, silent := 0, 0
	for i := 8; i < len(full); i += len(full) / 60 {
		bad := append([]byte(nil), full...)
		bad[i] ^= 0x40
		flipped++
		if got, err := Decode(bad); err == nil && !bytes.Equal(got, subs["counter"]) {
			silent++
		}
	}
	r.Check(fmt.Sprintf("of %d flipped bytes, ones accepted with wrong content", flipped),
		itoa(silent), "0")

	// XXH64 against its published vectors. Every carried frame above also
	// verifies a content checksum, so the hash is exercised over real data by
	// the decodes themselves; these two pin the empty and one-byte tails that
	// no frame reaches.
	r.Check("xxh64 of the empty input", hex64(hashOf(nil)), "ef46db3751d8e999")
	r.Check("xxh64 of one byte", hex64(hashOf([]byte("a"))), "d24ec4f1a98c6e5b")
	return r
}

func hashOf(b []byte) uint64 {
	h := newXXH64()
	h.write(b)
	return h.sum()
}

func hex64(v uint64) string { return fmt.Sprintf("%016x", v) }

func itoa(n int) string { return fmt.Sprintf("%d", n) }

var frames = map[string]string{
	"alt-1": "KLUv/aRADQMAVAAAEAABAQD7/+UOC00AAAgAAQA8DTkQAuztkiM=",
	"counter-1": "KLUv/aQgvwIArOUAasfPXQuwFVwDJElChJzEnUUHrAeIBkREREREREREREREREREREREREREREREREREREREMzMzMzMz" +
		"MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIi" +
		"IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIhERERERERERERERERERERERERERERERERERERERERERERERERERERERERER" +
		"ERERERER////////////////H0gggQQSSCCBBBJIIIEEEkgggQQSSCCBBBJIIIEEEkgggQQSSCCBBBJIIIEEEkgggQQS" +
		"SCCBBBJIIIEEEkgggQQSSCCBBBJIIIEEEkgggQQSSCCBBBJIIIEEEkgggQQSSCCBBBJIIIEEEkgggQQSSCCBBBJIIIEE" +
		"EkgggQQSQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgh3" +
		"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d2ZmZmZmZmZmZmZmZmZmZmZmZmZm" +
		"ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV" +
		"VVVVVVVVVVVVVVVERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERDMzMzMzMzMz" +
		"MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIi" +
		"IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIRERERERERERERERERERERERERERERERERERERERERERERERERERERERERERER" +
		"EREREf///////////////x9EEEEEEUQQQQQRRBBBBBFEEEEEEUQQQQQRRBBBBBFEEEEEEUQQQQQRRBBBBBFEEEEEEUQQ" +
		"QQQRRBBBBBFEEEEEEUQQQQQRRBBBBBFEEEEEEUQQQQQRRBBBBBFEEEEEEUQQQQQRRBBBBBFEEEEEEUQQQQQRRBBBBBFE" +
		"EEEEEUIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIId3d3" +
		"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3dmZmZmZmZmZmZmZmZmZmZmZmZmZmZm" +
		"ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZlVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV" +
		"VVVVVVVVVVVVREREREREREREREREREREREREREREREREREREREREREREREREREREREREREREREREREQzMzMzMzMzMzMz" +
		"MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMyIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIi" +
		"IiIiIiIiIiIiIiIiIiIiIiIiIiIiERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERER" +
		"ERH///////////////8fDAaDwWAwGAwGg8FgMBgMBoPBYDAYDAaDwWAwGAwGg8FgMBgMBoPBYDAYDAaDwWAwGAwGg8Fg" +
		"MBgMBoPBYDAYDAaDwWAwGAwGg8FgMBgMBoPBYDAYDAaDIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQggh" +
		"hBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYRwd3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3" +
		"d3d3d3d3d2dmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmVlVVVVVVVVVVVVVV" +
		"VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVFRERERERERERERERERERERERERERERERERERERERE" +
		"RERERERERERERERERERERERERDQzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMz" +
		"IyIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiISERERERERERERERERERERERER" +
		"EREREREREREREREREREREREREREREREREREREREREfH////////////////hrGgkBiGcFY3E4MJZ0UgMLJwVjcSgwlnR" +
		"SAwonBWNxGDCWdFIDCScFY3EIMJZ0UgMhrOikRgAQRCBBRZYYIEFFlhggQUWWGCBBRZYYIEFFlhggQUWWGCBBRZYYIEF" +
		"FlhggQUWWGCBBRZYYIEFFlhggQUWWGCBBRZYYIEFFlhggQUWWGCBBRZYYCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQggh" +
		"hBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEcHd3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3" +
		"d3d3d3d3d3d3d3d3d3d3d3dnZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZlZV" +
		"VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVRURERERERERERERERERERERERERE" +
		"REREREREREREREREREREREREREREREREREREREQ0MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMz" +
		"MzMzMzMzMzMzMyMiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiEhERERERERER" +
		"ERERERERERERERERERERERERERERERERERERERERERERERERERERERHx////////////////QQUVVFBBBRVUUEEFFVRQ" +
		"QQUVVFBBBRVUUEEFFVRQQQUVVFBBBRVUUEEFFVRQQQUVVFBBBRVUUEEFFVRQQQUVVFBBBRVUUEEFFVRQQQUVVFBBBRVU" +
		"UEEFFVRQQQUVVFBBBRVUUEEFFVRQQQUVVFBBBRVUUEEFFVRQQQUVVFAhhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQ" +
		"QgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhHB3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3" +
		"d3d3d3d3d3d3d3d3d3d3Z2ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZWVVVV" +
		"VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVUVERERERERERERERERERERERERERERE" +
		"RERERERERERERERERERERERERERERERERERENDMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMz" +
		"MzMzMzMzMzMjIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIhIRERERERERERER" +
		"ERERERERERERERERERERERERERERERERERERERERERERERERERER8f///////////////wEFFFBAAQUUUEABBRRQQAEF" +
		"FFBAAQUUUEABBRRQQAEFFFBAAQUUUEABBRRQQAEFFFBAAQUUUEABBRRQQAEFFFBAAQUUUEABBRRQQAEFFFBAAQUUUEAB" +
		"BRRQQAEFFFBAAQUUUEABBRRQQAEFFFBAAQUUUEABBRRQQAEFFFBAIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEII" +
		"IYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYRwd3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3" +
		"d3d3d3d3d3d3d3d3d2dmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmVlVVVVVV" +
		"VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVFRERERERERERERERERERERERERERERERE" +
		"RERERERERERERERERERERERERERERERERDQzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMz" +
		"MzMzMzMzIyIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiISERERERERERERERER" +
		"EREREREREREREREREREREREREREREREREREREREREREREREREfH////////////////BBBNMMMEEE0wwwQQTTDDBBBNM" +
		"MMEEE0wwwQQTTDDBBBNMMMEEE0wwwQQTTDDBBBNMMMEEE0wwwQQTTDDBBBNMMMEEE0wwwQQTTDDBBBNMMMEEE0wwwQQT" +
		"TDDBBBNMMMEEE0wwwQQTTDDBBBNMMMEEE0wwwQQTTDDBBBNMMCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGE" +
		"EEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEcHd3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3" +
		"d3d3d3d3d3d3d3dnZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZlZVVVVVVVVV" +
		"VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVRUREREREREREREREREREREREREREFP//////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"/////////////////yAYQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGE" +
		"EEIIIYQQQgh3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d2ZmZmZmZmZmZmZm" +
		"ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV" +
		"VVVVVVVVVVVVVVVVVVVVVVVVVVVERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERE" +
		"RDMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzIiIiIiIiIiIiIiIiIiIiIiIi" +
		"IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIRERERERERERERERERERERERERERERERERERERERERERERERER" +
		"EREREREREREREREREf///////////////x+EIAQhCEEIQhCCEIQgBCEIQQhCEIIQhCAEIQhBCEIQghCEIAQhCEEIQhCC" +
		"EIQgBCEIQQhCEIIQhCAEIQhBCEIQghCEIAQhCEEIQhCCEIQgBCEIQQhCEIIQhCAEIQhBCEIQghCEIAQhCEEIQhCCEIQg" +
		"BCEIQQhCEIIQhCAEIQhBCEIQghCEIAQhCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBC" +
		"CCGEEEIIIYQQQgghhBBCCCGEcHd3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3dn" +
		"ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZlZVVVVVVVVVVVVVVVVVVVVVVVVV" +
		"VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVRURERERERERERERERERERERERERERERERERERERERERERERERERE" +
		"REREREREREREREQ0MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMyMiIiIiIiIi" +
		"IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiEhERERERERERERERERERERERERERERERERER" +
		"ERERERERERERERERERERERERERERERHx////////////////wQUXXHDBBRdccMEFF1xwwQUXXHDBBRdccMEFF1xwwQUX" +
		"XHDBBRdccMEFF1xwwQUXXHDBBRdccMEFF1xwwQUXXHDBBRdccMEFF1xwwQUXXHDBBRdccMEFF1xwwQUXXHDBBRdccMEF" +
		"F1xwwQUXXHDBBRdccMEFF1xwwQUXXHAhhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQgghhBBCCCGEEEIIIYQQQggh" +
		"hBBCCCGEEEIIIYQQQgghhHB3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3Z2Zm" +
		"ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZWVVVVVVVVVVVVVVVVVVVVVVVVVVVV" +
		"VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVUVERERERERERERERERERERERERERERERERERERERERERERERERERERE" +
		"RERERERERERENDMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMjIiIiIiIiIiIi" +
		"IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIhIRERERERERERERERERERERERERERERERERERER" +
		"ERERERERERERERERERERERERERER8f///////////////4EFFlhggQUWWGCBBRZYYIEFFlhggQUWWGCBBRZYYIEFFlhg" +
		"gQUWWGCBBRZYYAEzCAIgiub/////////////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"/////////////////////7jsqATguw/jr+hoARQAAAAAgPgPgdWy6PBw8LREy4PDUyUtHg6dlmR5aHiapGXD4dGSKg8H" +
		"T4u0PDg8WtLk4eBpiZYHhydLWjwcNi2p6sjwuLC0SsPDw6ItDR4PljZpeHhYsqXB48HSFg2PDku1NHQ8TNqS4XFhaZWG" +
		"i4dFWyo8HixdsujwcPC0rCe/bsbv//////7////v//////7////v//////7////v//////7////v//////2dM2lLhseF" +
		"pVUaHh4WbWnweLC0ScPDw0JVV1dXV1dXV1eurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6u" +
		"rq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6uXF1dXV29S6o8HDwt0vLg8GhJi4eDpyVaHhye" +
		"LGnWLiklBQIKQkJCQkJCQkJCQkJCQkJCQoKEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISE" +
		"hISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISENWmzNGw8LNpS4fFgaZOGh4dFWxo8HiyJlnNxczHx8PKwsHBg" +
		"4GBkZGRkZGRkZGRkZGRkZGRkZGRkZGRkZMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjI" +
		"yMjIyMjIyMjIyMjIyMjIyMjIuDA8HZZmaeh4mLQlw+PB0ioNFw+LtjR4qLmVtZWRja2NiYkFAwtDQ0NDQ0NDQ0NDQ0ND" +
		"Q0NDQ0NDQ0NDQ0NDQ0NDQ0NDQ0NDQ0NDg4aGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaG" +
		"hoaGho/D82BplYaHh0VbGjweLG3S8PCwZEuDl55fXV8d3dzenJxcOLg4PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8" +
		"PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eFhMHg+Wtmh4eFiy" +
		"pcHjodIWDY8OS7M0FDU8KjoqKCY2JiQkQkBEYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBg" +
		"YGBgYGBgYGBgYGBgYGBgYGBgYGBgYGDAwMDAwMDAwMDAwMDAwMDAwMDAwEDl4GmJlgeHJ0tWORo2HiZtyfB4sLRJw6Hj" +
		"U9NTQzOzM0NGJgxMDA4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4O" +
		"Dg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4cHBwcHBx8DpuWZHlYeFqk5cLh0ZIqDwdPi6w6NOzKzsXNxcTDy8PCwoGBg5GR" +
		"kZGRkZGRkSEjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMj" +
		"IyMjIyMjIyMjIyMjIyNjxfB4WLKlwePB0hYNDw9LtTR0PEzakuGh5lbWVkY2tjYmJhYMLAwNDQ0NDQ0NDQ0NDQ0NDQ0N" +
		"DQ0NDQ0NDRoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoa" +
		"Pg5Ph6VaGjoeJm3J8LiwtErDxcOiLQ3uehijdtMSKVZ4UiKI05pP2D31z/4FaQM9FgArVNEKqwCrAKsA////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"//////////////////////////////8U////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"//////////////////////////////////////////////////////////////////////////8D////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"//////////////////////////8DQAgh/P//////////////////////////////////////////////////////////" +
		"////////////////////////////////////////////////////////////////////////////////////////////" +
		"/////////////////////////////////////////////////////////////////////////x+VPFgBABRAAAH4HwAQ" +
		"/R/shUyI",
	"empty-1": "KLUv/SQAAQAAmenYUQ==",
	"one-1":   "KLUv/SQBCQAAB7e7WOg=",
	"prose-1": "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",
	"prose-12": "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",
	"run-1":  "KLUv/WSIEk0AABBaWgEAg9MDLPZxz34=",
	"run-19": "KLUv/WSIEkUAAAhaAQCE0wMh9nHPfg==",
}
