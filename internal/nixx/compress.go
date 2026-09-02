// compress.go — decompression for the binary-cache protocol.
//
// gzip and bzip2 come from the standard library and zstd from internal/zstd,
// so the three schemes cache.nixos.org actually serves need nothing on the
// host. xz is decoded by the command-line tool; when it is absent the error
// names the missing decoder rather than reporting the archive as corrupt.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"compress/bzip2"
	"compress/gzip"
	"fmt"
	"io"
	"os"
	"os/exec"

	"github.com/polaris0xff/glibc-research/internal/zstd"
)

// Decompress wraps r according to a narinfo Compression field.
func Decompress(r io.Reader, compression string) (io.ReadCloser, error) {
	switch compression {
	case "", "none":
		return io.NopCloser(r), nil
	case "gzip":
		zr, err := gzip.NewReader(r)
		if err != nil {
			return nil, err
		}
		return zr, nil
	case "bzip2", "bz2":
		return io.NopCloser(bzip2.NewReader(r)), nil
	case "xz":
		return pipeThrough(r, "xz", []string{"xz", "unxz", "xzcat"},
			"xz-compressed and no xz decoder on PATH")
	case "zstd", "zst":
		return io.NopCloser(zstd.NewReader(r)), nil
	}
	return nil, fmt.Errorf("unsupported compression: %s", compression)
}

// pipeThrough streams the reader through the first available decoder.
func pipeThrough(r io.Reader, kind string, candidates []string, missing string) (io.ReadCloser, error) {
	tool := ""
	for _, c := range candidates {
		if _, err := exec.LookPath(c); err == nil {
			tool = c
			break
		}
	}
	if tool == "" {
		return nil, fmt.Errorf("%s (tried: %v)", missing, candidates)
	}
	args := []string{"-dc"}
	if tool == "xzcat" || tool == "unxz" {
		args = []string{"-c"}
	}
	cmd := exec.Command(tool, args...)
	cmd.Stdin = r
	cmd.Stderr = os.Stderr
	out, err := cmd.StdoutPipe()
	if err != nil {
		return nil, err
	}
	if err := cmd.Start(); err != nil {
		return nil, err
	}
	return &decoderPipe{ReadCloser: out, cmd: cmd, kind: kind}, nil
}

type decoderPipe struct {
	io.ReadCloser
	cmd  *exec.Cmd
	kind string
}

func (d *decoderPipe) Close() error {
	err := d.ReadCloser.Close()
	if werr := d.cmd.Wait(); err == nil && werr != nil {
		err = fmt.Errorf("%s decoder: %w", d.kind, werr)
	}
	return err
}

// HaveDecoder reports whether a compression scheme can be decoded here, for
// `pgb doctor`.
func HaveDecoder(compression string) bool {
	switch compression {
	case "", "none", "gzip", "bzip2", "bz2", "zstd", "zst":
		return true
	case "xz":
		return anyOnPath("xz", "unxz", "xzcat")
	}
	return false
}

func anyOnPath(names ...string) bool {
	for _, n := range names {
		if _, err := exec.LookPath(n); err == nil {
			return true
		}
	}
	return false
}
