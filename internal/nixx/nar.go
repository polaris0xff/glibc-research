// nar.go — the pieces of nix's binary-cache protocol that are not HTTP: the
// NAR archive format, nix's own base32, and narinfo signature verification.
//
// Nothing here trusts the server. A narinfo is signed, the signature is checked
// against a pinned public key, and the NAR is hashed and compared to the
// NarHash the signature covers. A fetch that skips either check is a download,
// not a substitution.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"bufio"
	"bytes"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/base64"
	"encoding/binary"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// CacheNixosOrgKeyName and CacheNixosOrgKey are the trust root: the public half
// of the key cache.nixos.org signs every narinfo with.
const (
	CacheNixosOrgKeyName = "cache.nixos.org-1"
	CacheNixosOrgKey     = "6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
)

// nix32 is nix's base32 alphabet. It is not RFC 4648: the alphabet omits e, o,
// u and t so a hash cannot spell a word, and the bits are consumed from the end
// of the digest backwards.
const nix32 = "0123456789abcdfghijklmnpqrsvwxyz"

// NixBase32 renders a digest the way nix writes hashes.
func NixBase32(digest []byte) string {
	n := (len(digest)*8-1)/5 + 1
	out := make([]byte, 0, n)
	for i := n - 1; i >= 0; i-- {
		b := i * 5
		byteIdx, bit := b/8, uint(b%8)
		var c uint32
		if byteIdx < len(digest) {
			c = uint32(digest[byteIdx]) >> bit
		}
		if byteIdx+1 < len(digest) {
			c |= uint32(digest[byteIdx+1]) << (8 - bit)
		}
		out = append(out, nix32[c&0x1f])
	}
	return string(out)
}

// maxString bounds a NAR token, so a corrupt length cannot ask for an
// allocation the size of the disk.
const maxString = 64 * 1024

// NarError is a malformed or unsafe archive.
type NarError struct{ msg string }

func (e *NarError) Error() string { return "nar: " + e.msg }

func narErr(format string, a ...any) error { return &NarError{msg: fmt.Sprintf(format, a...)} }

// narReader reads the NAR token grammar:
//
//	nar  = str("nix-archive-1") node
//	node = "(" "type" ( "regular" ["executable" ""] "contents" bytes
//	                  | "symlink"  "target" str
//	                  | "directory" ("entry" "(" "name" str "node" node ")")* ) ")"
//
// Every string is a little-endian u64 length, the bytes, then NUL padding to a
// multiple of 8. There is no file mode beyond the executable bit and no
// timestamp, which is why a NAR is reproducible and a tar is not.
type narReader struct {
	r      io.Reader
	pushed *string
}

func (r *narReader) readFull(n int) ([]byte, error) {
	buf := make([]byte, n)
	if _, err := io.ReadFull(r.r, buf); err != nil {
		return nil, narErr("short read: wanted %d bytes", n)
	}
	return buf, nil
}

func (r *narReader) u64() (uint64, error) {
	b, err := r.readFull(8)
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint64(b), nil
}

func (r *narReader) blob() ([]byte, error) {
	n, err := r.u64()
	if err != nil {
		return nil, err
	}
	if n > maxString {
		return nil, narErr("string of %d bytes exceeds the %d limit", n, maxString)
	}
	data, err := r.readFull(int(n))
	if err != nil {
		return nil, err
	}
	if err := r.skipPad(n); err != nil {
		return nil, err
	}
	return data, nil
}

func (r *narReader) skipPad(n uint64) error {
	pad := (8 - (n % 8)) % 8
	if pad == 0 {
		return nil
	}
	b, err := r.readFull(int(pad))
	if err != nil {
		return err
	}
	for _, c := range b {
		if c != 0 {
			return narErr("padding is not NUL")
		}
	}
	return nil
}

func (r *narReader) str() (string, error) {
	if r.pushed != nil {
		s := *r.pushed
		r.pushed = nil
		return s, nil
	}
	b, err := r.blob()
	if err != nil {
		return "", err
	}
	return string(b), nil
}

func (r *narReader) unread(s string) { r.pushed = &s }

func (r *narReader) expect(want string) error {
	got, err := r.str()
	if err != nil {
		return err
	}
	if got != want {
		return narErr("expected %q, got %q", want, got)
	}
	return nil
}

// NarExtract unpacks an archive to dest.
func NarExtract(in io.Reader, dest string) error {
	r := &narReader{r: bufio.NewReaderSize(in, 1<<20)}
	if err := r.expect("nix-archive-1"); err != nil {
		return err
	}
	abs, err := filepath.Abs(dest)
	if err != nil {
		return err
	}
	if parent := filepath.Dir(abs); parent != "" {
		if err := os.MkdirAll(parent, 0o755); err != nil {
			return err
		}
	}
	return narNode(r, abs)
}

func narNode(r *narReader, path string) error {
	if err := r.expect("("); err != nil {
		return err
	}
	if err := r.expect("type"); err != nil {
		return err
	}
	ty, err := r.str()
	if err != nil {
		return err
	}
	switch ty {
	case "regular":
		field, err := r.str()
		if err != nil {
			return err
		}
		mode := os.FileMode(0o644)
		if field == "executable" {
			if err := r.expect(""); err != nil {
				return err
			}
			if field, err = r.str(); err != nil {
				return err
			}
			mode = 0o755
		}
		if field != "contents" {
			return narErr("expected contents, got %q", field)
		}
		n, err := r.u64()
		if err != nil {
			return err
		}
		f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, mode)
		if err != nil {
			return err
		}
		if _, err := io.CopyN(f, r.r, int64(n)); err != nil {
			f.Close()
			return narErr("file contents truncated: %v", err)
		}
		if err := f.Close(); err != nil {
			return err
		}
		if err := r.skipPad(n); err != nil {
			return err
		}
	case "symlink":
		if err := r.expect("target"); err != nil {
			return err
		}
		target, err := r.str()
		if err != nil {
			return err
		}
		if err := os.Symlink(target, path); err != nil {
			return err
		}
	case "directory":
		if err := os.Mkdir(path, 0o755); err != nil {
			return err
		}
		prev := ""
		for {
			tok, err := r.str()
			if err != nil {
				return err
			}
			if tok != "entry" {
				r.unread(tok)
				break
			}
			if err := r.expect("("); err != nil {
				return err
			}
			if err := r.expect("name"); err != nil {
				return err
			}
			name, err := r.str()
			if err != nil {
				return err
			}
			// A NAR is remote input: a member called ".." escapes the
			// destination, and this check is what stops it.
			if name == "" || name == "." || name == ".." ||
				strings.ContainsAny(name, "/\x00") {
				return narErr("invalid path component: %q", name)
			}
			if prev != "" && name <= prev {
				return narErr("entries not sorted: %q >= %q", prev, name)
			}
			if err := r.expect("node"); err != nil {
				return err
			}
			if err := narNode(r, filepath.Join(path, name)); err != nil {
				return err
			}
			if err := r.expect(")"); err != nil {
				return err
			}
			prev = name
		}
	default:
		return narErr("unknown node type: %q", ty)
	}
	return r.expect(")")
}

// narWrite emits one length-prefixed, NUL-padded token.
func narWrite(w io.Writer, data []byte) error {
	var hdr [8]byte
	binary.LittleEndian.PutUint64(hdr[:], uint64(len(data)))
	if _, err := w.Write(hdr[:]); err != nil {
		return err
	}
	if _, err := w.Write(data); err != nil {
		return err
	}
	pad := (8 - (len(data) % 8)) % 8
	if pad > 0 {
		if _, err := w.Write(make([]byte, pad)); err != nil {
			return err
		}
	}
	return nil
}

func narWriteStr(w io.Writer, s string) error { return narWrite(w, []byte(s)) }

// NarDump serialises a path as a NAR.
func NarDump(src string, w io.Writer) error {
	if err := narWriteStr(w, "nix-archive-1"); err != nil {
		return err
	}
	return narDumpNode(src, w)
}

func narDumpNode(path string, w io.Writer) error {
	if err := narWriteStr(w, "("); err != nil {
		return err
	}
	if err := narWriteStr(w, "type"); err != nil {
		return err
	}
	fi, err := os.Lstat(path)
	if err != nil {
		return err
	}
	switch {
	case fi.Mode()&os.ModeSymlink != 0:
		target, err := os.Readlink(path)
		if err != nil {
			return err
		}
		if err := narWriteStr(w, "symlink"); err != nil {
			return err
		}
		if err := narWriteStr(w, "target"); err != nil {
			return err
		}
		if err := narWriteStr(w, target); err != nil {
			return err
		}
	case fi.IsDir():
		if err := narWriteStr(w, "directory"); err != nil {
			return err
		}
		entries, err := os.ReadDir(path)
		if err != nil {
			return err
		}
		names := make([]string, 0, len(entries))
		for _, e := range entries {
			names = append(names, e.Name())
		}
		// Sorted by bytes, not by locale: nix asserts this on read, and a
		// locale-aware sort produces an archive nix itself refuses.
		sort.Slice(names, func(i, j int) bool { return names[i] < names[j] })
		for _, name := range names {
			for _, tok := range []string{"entry", "(", "name", name, "node"} {
				if err := narWriteStr(w, tok); err != nil {
					return err
				}
			}
			if err := narDumpNode(filepath.Join(path, name), w); err != nil {
				return err
			}
			if err := narWriteStr(w, ")"); err != nil {
				return err
			}
		}
	default:
		if err := narWriteStr(w, "regular"); err != nil {
			return err
		}
		if fi.Mode()&0o111 != 0 {
			if err := narWriteStr(w, "executable"); err != nil {
				return err
			}
			if err := narWriteStr(w, ""); err != nil {
				return err
			}
		}
		if err := narWriteStr(w, "contents"); err != nil {
			return err
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		if err := narWrite(w, data); err != nil {
			return err
		}
	}
	return narWriteStr(w, ")")
}

// NarHash returns the sha256 of a NAR stream in nix-base32, and its size.
func NarHash(r io.Reader) (string, int64, error) {
	h := sha256.New()
	n, err := io.Copy(h, r)
	if err != nil {
		return "", 0, err
	}
	return "sha256:" + NixBase32(h.Sum(nil)), n, nil
}

// Narinfo is the parsed key: value document.
type Narinfo map[string]string

// ParseNarinfo reads the document.
func ParseNarinfo(text string) Narinfo {
	info := Narinfo{}
	for _, line := range strings.Split(text, "\n") {
		k, v, ok := strings.Cut(line, ": ")
		if ok {
			info[k] = v
		}
	}
	return info
}

// Fingerprint builds the message a narinfo's signature covers. Fields in
// another order, or references without their /nix/store/ prefix, verify
// against nothing and the failure looks like a bad key.
func (n Narinfo) Fingerprint() ([]byte, error) {
	for _, k := range []string{"StorePath", "NarHash", "NarSize"} {
		if n[k] == "" {
			return nil, fmt.Errorf("narinfo has no %s field", k)
		}
	}
	var refs []string
	for _, r := range strings.Fields(n["References"]) {
		refs = append(refs, "/nix/store/"+r)
	}
	return []byte(fmt.Sprintf("1;%s;%s;%s;%s",
		n["StorePath"], n["NarHash"], n["NarSize"], strings.Join(refs, ","))), nil
}

// Verify checks the Sig field against a set of trusted public keys, returning
// the key that matched.
func (n Narinfo) Verify(keys map[string]ed25519.PublicKey) (bool, string) {
	msg, err := n.Fingerprint()
	if err != nil {
		return false, err.Error()
	}
	sig, ok := n["Sig"]
	if !ok || sig == "" {
		return false, "no Sig field"
	}
	names := make([]string, 0, len(keys))
	for k := range keys {
		names = append(names, k)
	}
	sort.Strings(names)
	for _, one := range strings.Fields(sig) {
		name, b64, ok := strings.Cut(one, ":")
		if !ok {
			continue
		}
		key, known := keys[name]
		if !known {
			continue
		}
		raw, err := base64.StdEncoding.DecodeString(b64)
		if err != nil {
			continue
		}
		if ed25519.Verify(key, msg, raw) {
			return true, name
		}
	}
	return false, "no signature from a known key: " + strings.Join(names, ",")
}

// ParseKeySpec reads a NAME:BASE64 trusted-public-key specification.
func ParseKeySpec(spec string) (string, ed25519.PublicKey, error) {
	name, b64, ok := strings.Cut(spec, ":")
	if !ok {
		return "", nil, fmt.Errorf("a key spec is NAME:BASE64, got %q", spec)
	}
	raw, err := base64.StdEncoding.DecodeString(b64)
	if err != nil {
		return "", nil, fmt.Errorf("%s: key is not base64: %w", name, err)
	}
	if len(raw) != ed25519.PublicKeySize {
		return "", nil, fmt.Errorf("%s: key is %d bytes, wanted %d", name, len(raw), ed25519.PublicKeySize)
	}
	return name, ed25519.PublicKey(raw), nil
}

// DefaultKeys is the pinned trust root as a verifier set.
func DefaultKeys() map[string]ed25519.PublicKey {
	name, key, err := ParseKeySpec(CacheNixosOrgKeyName + ":" + CacheNixosOrgKey)
	if err != nil {
		panic(err)
	}
	return map[string]ed25519.PublicKey{name: key}
}

// storePathHashOf extracts the hash part of a /nix/store path.
func storePathHashOf(p string) string {
	base := filepath.Base(p)
	if i := strings.IndexByte(base, '-'); i > 0 {
		return base[:i]
	}
	return base
}

var _ = bytes.Equal
