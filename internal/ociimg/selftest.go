// selftest.go — prove the whiteout pass offline.
//
// The failure it guards is silent: without the pass a pull still exits 0 and
// still produces a rootfs, just one carrying files the image deleted. The
// fixture therefore asserts both directions — a planted file that must
// disappear, and untouched files that must survive.
//
// SPDX-License-Identifier: MIT
package ociimg

import (
	"archive/tar"
	"bytes"
	"os"
	"path/filepath"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Selftest runs the whiteout cases with no network.
func Selftest() *selftest.Report {
	r := selftest.New("oci-pull")

	dir, err := os.MkdirTemp("", "pgb-oci-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "a writable temporary directory")
		return r
	}
	defer os.RemoveAll(dir)

	dst := filepath.Join(dir, "dst")
	mustMkdir(r, filepath.Join(dst, "usr", "lib"))
	mustMkdir(r, filepath.Join(dst, "etc"))
	mustMkdir(r, filepath.Join(dst, "keep"))
	mustTouch(r, filepath.Join(dst, "usr", "lib", "libnss_stale.so.2"))
	mustTouch(r, filepath.Join(dst, "etc", "nsswitch.conf"))
	mustTouch(r, filepath.Join(dst, "keep", "file"))

	var buf bytes.Buffer
	tw := tar.NewWriter(&buf)
	addDir(tw, "usr/")
	addDir(tw, "usr/lib/")
	addFile(tw, "usr/lib/.wh.libnss_stale.so.2", nil) // delete one file
	addDir(tw, "etc/")
	addFile(tw, "etc/.wh..wh..opq", nil)     // clear a whole directory
	addFile(tw, "etc/resolv.conf", []byte{}) // ... then re-add into it
	addFile(tw, "new", []byte("hello\n"))    // and add a new file
	_ = tw.Close()

	blob := filepath.Join(dir, "layer.tar")
	if err := os.WriteFile(blob, buf.Bytes(), 0o644); err != nil {
		r.Fail("write fixture", err.Error(), "a writable file")
		return r
	}
	if err := ApplyLayer(blob, "", dst); err != nil {
		r.Fail("apply layer", err.Error(), "no error")
		return r
	}

	r.Check("deleted-file", presence(filepath.Join(dst, "usr/lib/libnss_stale.so.2")), "gone")
	r.Check("opaque-dir-wiped", presence(filepath.Join(dst, "etc/nsswitch.conf")), "gone")
	r.Check("readd-after-opq", presence(filepath.Join(dst, "etc/resolv.conf")), "present")
	r.Check("untouched-kept", presence(filepath.Join(dst, "keep/file")), "present")
	r.Check("new-file-added", presence(filepath.Join(dst, "new")), "present")
	r.Check("markers-not-kept", countWhiteouts(dst), "0")
	return r
}

func presence(p string) string {
	if _, err := os.Lstat(p); err == nil {
		return "present"
	}
	return "gone"
}

func countWhiteouts(root string) string {
	n := 0
	_ = filepath.Walk(root, func(p string, fi os.FileInfo, err error) error {
		if err == nil && len(filepath.Base(p)) > 4 && filepath.Base(p)[:4] == ".wh." {
			n++
		}
		return nil
	})
	if n == 0 {
		return "0"
	}
	return string(rune('0' + n))
}

func mustMkdir(r *selftest.Report, p string) {
	if err := os.MkdirAll(p, 0o755); err != nil {
		r.Fail("mkdir "+p, err.Error(), "created")
	}
}

func mustTouch(r *selftest.Report, p string) {
	if err := os.WriteFile(p, nil, 0o644); err != nil {
		r.Fail("touch "+p, err.Error(), "created")
	}
}

func addDir(tw *tar.Writer, name string) {
	_ = tw.WriteHeader(&tar.Header{Name: name, Typeflag: tar.TypeDir, Mode: 0o755})
}

func addFile(tw *tar.Writer, name string, data []byte) {
	_ = tw.WriteHeader(&tar.Header{Name: name, Typeflag: tar.TypeReg, Mode: 0o644, Size: int64(len(data))})
	if len(data) > 0 {
		_, _ = tw.Write(data)
	}
}
