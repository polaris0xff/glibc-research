// libiconv.go — build GNU libiconv as a static archive, from inside a build
// environment.
//
// glibc's iconv reaches its encodings through dlopen'd gconv modules, and every
// one carries DT_NEEDED libc.so.6. In a static binary that means a second libc,
// and on a musl host nothing loads at all. GNU libiconv carries the same
// encodings as ordinary archive code, so a static link gets them with no dlopen
// and no data directory.
//
// The version is pinned: an unpinned dependency makes every number taken
// against it undated.
//
// SPDX-License-Identifier: MIT
package envx

import (
	"archive/tar"
	"compress/gzip"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"time"

	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

// LibiconvVersion is the pinned release.
const LibiconvVersion = "1.18"

// BuildLibiconv fetches, configures, builds and installs static libiconv into
// prefix. It is a no-op when the archive is already there unless force is set.
func BuildLibiconv(prefix, version string, force bool) error {
	if version == "" {
		version = LibiconvVersion
	}
	archive := filepath.Join(prefix, "lib", "libiconv.a")
	if !force {
		if _, err := os.Stat(archive); err == nil {
			logx.Say("libiconv already at %s (--force to rebuild)", prefix)
			return nil
		}
	}

	work, err := os.MkdirTemp("", "pgb-libiconv-")
	if err != nil {
		return fail.Cannot("%v", err)
	}
	defer os.RemoveAll(work)

	url := fmt.Sprintf("https://ftp.gnu.org/pub/gnu/libiconv/libiconv-%s.tar.gz", version)
	logx.Say("fetching %s", url)
	tarball := filepath.Join(work, "src.tar.gz")
	if err := download(url, tarball); err != nil {
		return fail.Ran("cannot fetch libiconv: %v", err)
	}
	if err := untarGz(tarball, work); err != nil {
		return fail.Ran("cannot unpack libiconv: %v", err)
	}
	srcDir := filepath.Join(work, "libiconv-"+version)
	if _, err := os.Stat(srcDir); err != nil {
		return fail.Ran("libiconv tarball did not contain %s", filepath.Base(srcDir))
	}

	steps := []struct {
		name string
		argv []string
		env  []string
	}{
		// --disable-nls drops the forty translation catalogues. pgb links
		// libiconv.a for its conversion tables and never shows its messages,
		// and building them needs msgfmt from gettext — a host dependency
		// that buys nothing and that the pinned build image does not carry.
		{"configure", []string{"./configure", "--prefix=" + prefix,
			"--enable-static", "--disable-shared", "--enable-extra-encodings",
			"--disable-nls"},
			[]string{"CFLAGS=-O2 -fPIC"}},
		{"make", []string{"make", "-j" + strconv.Itoa(runtime.NumCPU())}, nil},
		{"make install", []string{"make", "install"}, nil},
	}
	for _, s := range steps {
		logFile := filepath.Join(work, strings.ReplaceAll(s.name, " ", "-")+".log")
		lf, err := os.Create(logFile)
		if err != nil {
			return fail.Cannot("%v", err)
		}
		cmd := &proc.Cmd{Argv: s.argv, Dir: srcDir, Env: s.env, Stdout: lf, Stderr: lf, Subsys: "env"}
		r, err := cmd.Run()
		lf.Close()
		if err != nil {
			return fail.Cannot("libiconv %s: %v", s.name, err)
		}
		if r.Failed() {
			showTail(logFile)
			return fail.Ran("libiconv %s failed (exit %d)", s.name, r.Code)
		}
	}

	fi, err := os.Stat(archive)
	if err != nil {
		return fail.Ran("libiconv installed but %s is missing", archive)
	}
	logx.Say("libiconv %s -> %s (%d bytes)", version, archive, fi.Size())
	return nil
}

func download(url, dst string) error {
	client := &http.Client{Timeout: 30 * time.Minute}
	var lastErr error
	for attempt := range 4 {
		if attempt > 0 {
			time.Sleep(time.Duration(attempt) * 2 * time.Second)
		}
		resp, err := client.Get(url)
		if err != nil {
			lastErr = err
			continue
		}
		if resp.StatusCode != http.StatusOK {
			resp.Body.Close()
			lastErr = fmt.Errorf("%s: HTTP %d", url, resp.StatusCode)
			continue
		}
		f, err := os.Create(dst)
		if err != nil {
			resp.Body.Close()
			return err
		}
		_, err = io.Copy(f, resp.Body)
		resp.Body.Close()
		if cerr := f.Close(); err == nil {
			err = cerr
		}
		if err == nil {
			return nil
		}
		lastErr = err
	}
	return lastErr
}

func untarGz(path, dst string) error {
	f, err := os.Open(path)
	if err != nil {
		return err
	}
	defer f.Close()
	zr, err := gzip.NewReader(f)
	if err != nil {
		return err
	}
	defer zr.Close()
	tr := tar.NewReader(zr)
	for {
		h, err := tr.Next()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
		name := filepath.Clean("/" + h.Name)
		target := filepath.Join(dst, name)
		if !strings.HasPrefix(target, dst+string(os.PathSeparator)) {
			return fmt.Errorf("archive entry %q escapes %s", h.Name, dst)
		}
		switch h.Typeflag {
		case tar.TypeDir:
			if err := os.MkdirAll(target, h.FileInfo().Mode().Perm()); err != nil {
				return err
			}
		case tar.TypeReg:
			if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
				return err
			}
			out, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, h.FileInfo().Mode().Perm())
			if err != nil {
				return err
			}
			if _, err := io.Copy(out, tr); err != nil {
				out.Close()
				return err
			}
			if err := out.Close(); err != nil {
				return err
			}
		case tar.TypeSymlink:
			if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
				return err
			}
			_ = os.Remove(target)
			if err := os.Symlink(h.Linkname, target); err != nil {
				return err
			}
		}
	}
}
