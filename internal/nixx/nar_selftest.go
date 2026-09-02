// nar_selftest.go — the NAR, base32, signature and decoder cases, offline.
//
// The oracles are external on purpose: the base32 vectors were printed by a
// real `nix-hash`, the ed25519 vectors come from RFC 8032, and the narinfo
// fixtures are signed by cache.nixos.org's own key. A self-generated fixture
// would test this code against itself.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"bytes"
	"compress/gzip"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Selftest runs every carried case for the binary-cache protocol.
func Selftest(fixtureDir string) *selftest.Report {
	r := selftest.New("nix-nar")

	empty := sha256.Sum256(nil)
	r.Check("nix-base32 of sha256('') matches nix-hash", NixBase32(empty[:]),
		"0mdqa9w1p6cmli6976v4wi0sw9r4p5prkj7lzfd1877wk11c9c73")
	hello := sha256.Sum256([]byte("hello"))
	r.Check("nix-base32 of sha256('hello') matches nix-hash", NixBase32(hello[:]),
		"094qif9n4cq4fdg459qzbhg1c6wywawwaaivx0k0x8xhbyx4vwic")
	r.Check("nix-base32 length for sha256", strconv.Itoa(len(NixBase32(make([]byte, 32)))), "52")
	r.CheckBool("nix-base32 alphabet excludes eotu", strings.ContainsAny(nix32, "eotu"), false)

	narRoundTrip(r)
	ed25519Vectors(r)
	fingerprints(r)
	fixtures(r, fixtureDir)
	decoders(r)
	return r
}

func narRoundTrip(r *selftest.Report) {
	tmp, err := os.MkdirTemp("", "pgb-nar-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "created")
		return
	}
	defer os.RemoveAll(tmp)

	src := filepath.Join(tmp, "src")
	if err := os.MkdirAll(filepath.Join(src, "bin"), 0o755); err != nil {
		r.Fail("fixture mkdir", err.Error(), "created")
		return
	}
	_ = os.WriteFile(filepath.Join(src, "bin", "prog"), []byte("#!/bin/sh\necho hi\n"), 0o755)
	_ = os.WriteFile(filepath.Join(src, "data.txt"), bytes.Repeat([]byte("x"), 1000), 0o644)
	_ = os.Symlink("bin/prog", filepath.Join(src, "link"))

	var buf bytes.Buffer
	if err := NarDump(src, &buf); err != nil {
		r.Fail("nar dump", err.Error(), "no error")
		return
	}
	blob := buf.Bytes()
	r.CheckBool("nar magic", len(blob) > 21 && string(blob[8:21]) == "nix-archive-1", true)
	r.CheckBool("nar is 8-byte aligned", len(blob)%8 == 0, true)

	dest := filepath.Join(tmp, "dest")
	if err := NarExtract(bytes.NewReader(blob), dest); err != nil {
		r.Fail("nar extract", err.Error(), "no error")
		return
	}
	fi, err := os.Stat(filepath.Join(dest, "bin", "prog"))
	r.CheckBool("round trip: executable bit", err == nil && fi.Mode()&0o111 != 0, true)
	target, err := os.Readlink(filepath.Join(dest, "link"))
	r.Check("round trip: symlink target", orErr(target, err), "bin/prog")
	content, err := os.ReadFile(filepath.Join(dest, "data.txt"))
	r.CheckBool("round trip: contents", err == nil && string(content) == strings.Repeat("x", 1000), true)

	// Hand-built, because the serialiser cannot produce it: an entry named
	// ".." must be refused, not written.
	var evil bytes.Buffer
	for _, tok := range []string{"nix-archive-1", "(", "type", "directory", "entry", "(", "name", ".."} {
		_ = narWriteStr(&evil, tok)
	}
	err = NarExtract(bytes.NewReader(evil.Bytes()), filepath.Join(tmp, "evil"))
	r.CheckBool("refuses an entry named ..",
		err != nil && strings.Contains(err.Error(), "invalid path component"), true)

	// Unsorted entries must be refused too: nix relies on the order for
	// canonicity, so accepting them would accept two NARs for one tree.
	var unsorted bytes.Buffer
	for _, tok := range []string{"nix-archive-1", "(", "type", "directory"} {
		_ = narWriteStr(&unsorted, tok)
	}
	for _, name := range []string{"b", "a"} {
		for _, tok := range []string{"entry", "(", "name", name, "node", "(", "type", "symlink", "target", "t", ")", ")"} {
			_ = narWriteStr(&unsorted, tok)
		}
	}
	_ = narWriteStr(&unsorted, ")")
	err = NarExtract(bytes.NewReader(unsorted.Bytes()), filepath.Join(tmp, "unsorted"))
	r.CheckBool("refuses unsorted entries",
		err != nil && strings.Contains(err.Error(), "not sorted"), true)
}

// rfc8032 vectors catch an implementation that is internally consistent and
// wrong, which a self-generated key pair cannot.
var rfc8032 = []struct{ label, pub, msg, sig string }{
	{
		"RFC 8032 test 1 (empty message)",
		"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
		"",
		"e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8" +
			"821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
	},
	{
		"RFC 8032 test 2 (one byte)",
		"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
		"72",
		"92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085a" +
			"c1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
	},
}

func ed25519Vectors(r *selftest.Report) {
	for _, v := range rfc8032 {
		pub, err1 := hex.DecodeString(v.pub)
		msg, err2 := hex.DecodeString(v.msg)
		sig, err3 := hex.DecodeString(v.sig)
		if err1 != nil || err2 != nil || err3 != nil {
			r.Fail(v.label, "vector is not hex", "hex")
			continue
		}
		r.CheckBool(v.label+": accepts", ed25519.Verify(pub, msg, sig), true)
		bad := append([]byte(nil), sig...)
		bad[0] ^= 1
		r.CheckBool(v.label+": refuses a flipped bit", ed25519.Verify(pub, msg, bad), false)
		r.CheckBool(v.label+": refuses a changed message",
			ed25519.Verify(pub, append(msg, '!'), sig), false)
	}
}

func fingerprints(r *selftest.Report) {
	info := ParseNarinfo("StorePath: /nix/store/aaa-x\nURL: nar/z.nar.xz\nCompression: xz\n" +
		"NarHash: sha256:1x\nNarSize: 42\nReferences: bbb-y ccc-z\n")
	fp, err := info.Fingerprint()
	r.Check("signature message shape", orErr(string(fp), err),
		"1;/nix/store/aaa-x;sha256:1x;42;/nix/store/bbb-y,/nix/store/ccc-z")

	info2 := ParseNarinfo("StorePath: /nix/store/aaa-x\nNarHash: h\nNarSize: 1\n")
	fp2, err := info2.Fingerprint()
	r.Check("signature message with no references", orErr(string(fp2), err),
		"1;/nix/store/aaa-x;h;1;")
}

// fixtures verifies real narinfo bodies signed by cache.nixos.org's key,
// exercising the message construction, the base64 decode, the pinned key and
// ed25519 together against a signature this project could not have produced.
func fixtures(r *selftest.Report, dir string) {
	if dir == "" {
		r.Skip("no narinfo fixture directory given")
		return
	}
	entries, err := os.ReadDir(dir)
	if err != nil {
		r.Skip("narinfo fixtures missing: " + dir)
		return
	}
	keys := DefaultKeys()
	found := 0
	for _, e := range entries {
		if !strings.HasSuffix(e.Name(), ".narinfo") {
			continue
		}
		found++
		b, err := os.ReadFile(filepath.Join(dir, e.Name()))
		if err != nil {
			r.Fail("fixture "+e.Name(), err.Error(), "readable")
			continue
		}
		info := ParseNarinfo(string(b))
		ok, why := info.Verify(keys)
		if !ok {
			r.Fail("fixture "+e.Name()+" verifies against the pinned cache key", why, "verified")
		} else {
			r.CheckBool("fixture "+e.Name()+" verifies against the pinned cache key", true, true)
		}
		// The negative case is the half that proves the positive one.
		tampered := Narinfo{}
		for k, v := range info {
			tampered[k] = v
		}
		if n, err := strconv.Atoi(info["NarSize"]); err == nil {
			tampered["NarSize"] = strconv.Itoa(n + 1)
		} else {
			tampered["NarSize"] = info["NarSize"] + "1"
		}
		bad, _ := tampered.Verify(keys)
		r.CheckBool("fixture "+e.Name()+" fails when NarSize is changed", bad, false)
	}
	if found == 0 {
		r.Skip("no .narinfo fixtures in " + dir)
	}
}

func decoders(r *selftest.Report) {
	for _, c := range []string{"gzip", "bzip2", "xz", "zstd"} {
		r.CheckBool("decoder available: "+c, HaveDecoder(c), true)
	}
	payload := bytes.Repeat([]byte("nix-archive-1"), 5000)

	var gz bytes.Buffer
	if err := gzipCompress(&gz, payload); err != nil {
		r.Fail("gzip round trip", err.Error(), "compressed")
	} else {
		r.CheckBool("gzip decoder round trip", roundTrips(gz.Bytes(), "gzip", payload), true)
	}

	// zstd is what cache.nixos.org serves today and xz is what it served
	// before, so both are exercised rather than reported present. Producing
	// the input needs the corresponding tool; when it is absent the case
	// reports skipped rather than passing vacuously. Decoding zstd does not:
	// `pgb selftest zstd` covers that offline against carried frames.
	for _, c := range []struct{ name, tool string }{{"zstd", "zstd"}, {"xz", "xz"}} {
		blob, err := compressWith(c.tool, payload)
		if err != nil {
			r.Skip(c.name + " round trip: " + err.Error())
			continue
		}
		r.CheckBool(c.name+" decoder round trip", roundTrips(blob, c.name, payload), true)
	}
}

func roundTrips(blob []byte, compression string, want []byte) bool {
	rc, err := Decompress(bytes.NewReader(blob), compression)
	if err != nil {
		return false
	}
	got, err := io.ReadAll(rc)
	closeErr := rc.Close()
	return err == nil && closeErr == nil && bytes.Equal(got, want)
}

// compressWith produces a fixture using the same tool the decoder uses, so a
// missing tool is reported once rather than looking like a decode failure.
func compressWith(tool string, data []byte) ([]byte, error) {
	if _, err := exec.LookPath(tool); err != nil {
		return nil, errors.New(tool + " is not on PATH")
	}
	cmd := exec.Command(tool, "-q", "-c")
	cmd.Stdin = bytes.NewReader(data)
	var out bytes.Buffer
	cmd.Stdout = &out
	if err := cmd.Run(); err != nil {
		return nil, err
	}
	return out.Bytes(), nil
}

func orErr(v string, err error) string {
	if err != nil {
		return "error: " + err.Error()
	}
	return v
}

func gzipCompress(w *bytes.Buffer, data []byte) error {
	zw := gzip.NewWriter(w)
	if _, err := zw.Write(data); err != nil {
		return err
	}
	return zw.Close()
}
