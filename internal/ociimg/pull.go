// Package ociimg fetches an OCI/Docker image and unpacks it to a root
// filesystem directory, without a container daemon.
//
// A registry is a plain HTTPS blob store: this does the anonymous-token dance,
// resolves a tag to a per-platform manifest digest, downloads the layers and
// merges them with the whiteout conventions honoured. Every pull writes
// .oci-provenance recording exactly what landed, so --digest reproduces the
// same filesystem.
//
// SPDX-License-Identifier: MIT
package ociimg

import (
	"archive/tar"
	"compress/gzip"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"slices"
	"strings"
	"time"

	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/zstd"
)

var log = logx.New("oci")

const acceptHeader = "application/vnd.oci.image.index.v1+json," +
	"application/vnd.docker.distribution.manifest.list.v2+json," +
	"application/vnd.oci.image.manifest.v1+json," +
	"application/vnd.docker.distribution.manifest.v2+json"

// Options controls one pull.
type Options struct {
	Ref    string // alpine:3.20, voidlinux/voidlinux-musl:latest, ghcr.io/x/y:z
	Out    string // the rootfs directory, replaced wholesale
	Arch   string // amd64 | arm64; empty means this machine's
	Digest string // pin the per-platform manifest; skips the index hop
	Quiet  bool
}

type reference struct {
	Registry string
	Repo     string
	Tag      string
}

// parseRef splits a reference into registry, repository and tag. A first
// component containing a dot or a colon, or literally "localhost", is a
// registry; anything else is a Docker Hub repository, and a bare name is under
// library/.
func parseRef(ref string) reference {
	registry := "registry-1.docker.io"
	rest := ref
	if before, after, ok := strings.Cut(ref, "/"); ok {
		head := before
		if strings.Count(ref, "/") >= 2 ||
			strings.ContainsAny(head, ".:") || head == "localhost" {
			registry, rest = head, after
		}
	} else {
		rest = "library/" + ref
	}
	if registry == "docker.io" {
		registry = "registry-1.docker.io"
	}
	repo, tag := rest, "latest"
	if i := strings.LastIndexByte(rest, ':'); i > strings.LastIndexByte(rest, '/') {
		repo, tag = rest[:i], rest[i+1:]
	}
	return reference{Registry: registry, Repo: repo, Tag: tag}
}

// tokenEndpoints are the registries that answer an anonymous pull-scope token
// request. A registry needing nothing simply gets no header.
var tokenEndpoints = map[string]string{
	"registry-1.docker.io": "https://auth.docker.io/token?service=registry.docker.io&scope=repository:%s:pull",
	"ghcr.io":              "https://ghcr.io/token?service=ghcr.io&scope=repository:%s:pull",
	"quay.io":              "https://quay.io/v2/auth?service=quay.io&scope=repository:%s:pull",
}

type client struct {
	ref   reference
	token string
	http  *http.Client
}

func newClient(ref reference) *client {
	c := &client{ref: ref, http: &http.Client{Timeout: 30 * time.Minute}}
	tmpl, ok := tokenEndpoints[ref.Registry]
	if !ok {
		return c
	}
	resp, err := c.http.Get(fmt.Sprintf(tmpl, ref.Repo))
	if err != nil {
		return c
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return c
	}
	var body struct {
		Token       string `json:"token"`
		AccessToken string `json:"access_token"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		return c
	}
	c.token = body.Token
	if c.token == "" {
		c.token = body.AccessToken
	}
	return c
}

// get fetches a registry path, following redirects and retrying transient
// failures.
func (c *client) get(path string) (*http.Response, error) {
	url := fmt.Sprintf("https://%s/v2/%s", c.ref.Registry, path)
	var lastErr error
	for attempt := range 4 {
		if attempt > 0 {
			time.Sleep(time.Duration(attempt) * 2 * time.Second)
		}
		req, err := http.NewRequest(http.MethodGet, url, nil)
		if err != nil {
			return nil, err
		}
		req.Header.Set("Accept", acceptHeader)
		if c.token != "" {
			req.Header.Set("Authorization", "Bearer "+c.token)
		}
		resp, err := c.http.Do(req)
		if err != nil {
			lastErr = err
			continue
		}
		if resp.StatusCode == http.StatusOK {
			return resp, nil
		}
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 2048))
		resp.Body.Close()
		lastErr = fmt.Errorf("%s: HTTP %d: %s", url, resp.StatusCode, strings.TrimSpace(string(body)))
		if resp.StatusCode < 500 && resp.StatusCode != http.StatusTooManyRequests {
			break
		}
	}
	return nil, lastErr
}

func (c *client) getBytes(path string) ([]byte, error) {
	resp, err := c.get(path)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	return io.ReadAll(resp.Body)
}

func (c *client) getFile(path, dst string) error {
	resp, err := c.get(path)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	f, err := os.Create(dst)
	if err != nil {
		return err
	}
	if _, err := io.Copy(f, resp.Body); err != nil {
		f.Close()
		return err
	}
	return f.Close()
}

type descriptor struct {
	MediaType string `json:"mediaType"`
	Digest    string `json:"digest"`
	Size      int64  `json:"size"`
	Platform  *struct {
		Architecture string `json:"architecture"`
		OS           string `json:"os"`
		Variant      string `json:"variant"`
	} `json:"platform,omitempty"`
}

type manifest struct {
	MediaType string       `json:"mediaType"`
	Manifests []descriptor `json:"manifests"`
	Layers    []descriptor `json:"layers"`
}

// Result describes what a pull landed.
type Result struct {
	Ref            string
	Registry       string
	Repo           string
	Tag            string
	Arch           string
	IndexDigest    string
	ManifestDigest string
	Layers         []descriptor
}

// Pull fetches and unpacks an image.
func Pull(o Options) (*Result, error) {
	if o.Ref == "" {
		return nil, fail.Cannot("oci: no image reference given")
	}
	if o.Out == "" {
		return nil, fail.Cannot("oci: --out DIR is required")
	}
	arch := o.Arch
	if arch == "" {
		arch = goArchToOCI(runtime.GOARCH)
	}
	ref := parseRef(o.Ref)
	if !o.Quiet {
		logx.Say("oci-pull: %s/%s:%s  arch=%s", ref.Registry, ref.Repo, ref.Tag, arch)
	}

	c := newClient(ref)
	res := &Result{Ref: o.Ref, Registry: ref.Registry, Repo: ref.Repo, Tag: ref.Tag, Arch: arch}

	var manifestBytes []byte
	if o.Digest != "" {
		res.ManifestDigest = o.Digest
		res.IndexDigest = "(pinned, index not consulted)"
		b, err := c.getBytes(ref.Repo + "/manifests/" + o.Digest)
		if err != nil {
			return nil, fail.Ran("cannot fetch %s: %v", o.Digest, err)
		}
		manifestBytes = b
	} else {
		idx, err := c.getBytes(ref.Repo + "/manifests/" + ref.Tag)
		if err != nil {
			return nil, fail.Ran("cannot fetch manifest for %s:%s: %v", ref.Repo, ref.Tag, err)
		}
		sum := sha256.Sum256(idx)
		res.IndexDigest = "sha256:" + hex.EncodeToString(sum[:])
		var m manifest
		if err := json.Unmarshal(idx, &m); err != nil {
			return nil, fail.Ran("manifest is not JSON: %v", err)
		}
		if len(m.Manifests) == 0 {
			// Already a per-platform manifest.
			res.ManifestDigest = res.IndexDigest
			manifestBytes = idx
		} else {
			d := pickPlatform(m.Manifests, arch)
			if d == "" {
				return nil, fail.Ran("no linux/%s manifest in %s:%s", arch, ref.Repo, ref.Tag)
			}
			res.ManifestDigest = d
			b, err := c.getBytes(ref.Repo + "/manifests/" + d)
			if err != nil {
				return nil, fail.Ran("cannot fetch %s: %v", d, err)
			}
			manifestBytes = b
		}
	}

	var m manifest
	if err := json.Unmarshal(manifestBytes, &m); err != nil {
		return nil, fail.Ran("cannot read layers: %v", err)
	}
	if len(m.Layers) == 0 {
		return nil, fail.Ran("manifest lists no layers")
	}
	res.Layers = m.Layers

	if err := os.MkdirAll(o.Out, 0o755); err != nil {
		return nil, fail.Cannot("cannot create %s: %v", o.Out, err)
	}
	// A rootfs is replaced wholesale, never merged onto a previous run: a
	// leftover file from an earlier image is exactly the contamination this bed
	// exists to rule out.
	if entries, err := os.ReadDir(o.Out); err == nil && len(entries) > 0 {
		if !o.Quiet {
			logx.Say("oci-pull: %s is not empty, clearing it", o.Out)
		}
		for _, e := range entries {
			if err := os.RemoveAll(filepath.Join(o.Out, e.Name())); err != nil {
				return nil, fail.Cannot("cannot clear %s: %v", o.Out, err)
			}
		}
	}

	work, err := os.MkdirTemp("", "pgb-oci-")
	if err != nil {
		return nil, fail.Cannot("%v", err)
	}
	defer os.RemoveAll(work)

	for i, l := range m.Layers {
		if !o.Quiet {
			logx.Say("  layer %d: %s", i+1, l.Digest)
		}
		blob := filepath.Join(work, "layer.blob")
		if err := c.getFile(ref.Repo+"/blobs/"+l.Digest, blob); err != nil {
			return nil, fail.Ran("cannot fetch layer %s: %v", l.Digest, err)
		}
		if err := ApplyLayer(blob, l.MediaType, o.Out); err != nil {
			return nil, fail.Ran("layer %s: %v", l.Digest, err)
		}
		_ = os.Remove(blob)
	}

	if err := writeProvenance(o.Out, res); err != nil {
		return nil, err
	}
	if !o.Quiet {
		logx.Say("oci-pull: unpacked to %s  (manifest %s)", o.Out, res.ManifestDigest)
	}
	return res, nil
}

// pickPlatform prefers an exact linux/arch with no variant, then any linux/arch.
func pickPlatform(list []descriptor, arch string) string {
	for _, m := range list {
		if m.Platform != nil && m.Platform.Architecture == arch &&
			m.Platform.OS == "linux" && m.Platform.Variant == "" {
			return m.Digest
		}
	}
	for _, m := range list {
		if m.Platform != nil && m.Platform.Architecture == arch && m.Platform.OS == "linux" {
			return m.Digest
		}
	}
	return ""
}

func writeProvenance(out string, r *Result) error {
	var b strings.Builder
	b.WriteString("# oci-pull provenance\n\n")
	fmt.Fprintf(&b, "reference:        %s\n", r.Ref)
	fmt.Fprintf(&b, "registry:         %s\n", r.Registry)
	fmt.Fprintf(&b, "repository:       %s\n", r.Repo)
	fmt.Fprintf(&b, "tag:              %s\n", r.Tag)
	fmt.Fprintf(&b, "architecture:     %s\n", r.Arch)
	fmt.Fprintf(&b, "index digest:     %s\n", r.IndexDigest)
	fmt.Fprintf(&b, "manifest digest:  %s\n", r.ManifestDigest)
	fmt.Fprintf(&b, "pulled (UTC):     %s\n", time.Now().UTC().Format("2006-01-02T15:04:05Z"))
	b.WriteString("\nlayers:\n")
	for _, l := range r.Layers {
		fmt.Fprintf(&b, "  %s %s\n", l.Digest, l.MediaType)
	}
	b.WriteString("\nreproduce exactly:\n")
	fmt.Fprintf(&b, "  pgb rootfs pull %s --arch %s --digest %s --out DIR\n",
		r.Ref, r.Arch, r.ManifestDigest)
	return os.WriteFile(filepath.Join(out, ".oci-provenance"), []byte(b.String()), 0o644)
}

func goArchToOCI(a string) string {
	switch a {
	case "amd64", "arm64", "386", "ppc64le", "s390x", "riscv64":
		return a
	}
	return a
}

// openLayer returns a reader over a layer blob, decompressing by media type
// and falling back to sniffing when the type is absent.
func openLayer(path, mediaType string) (io.ReadCloser, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	if strings.Contains(mediaType, "zstd") {
		return &closerPair{io.NopCloser(zstd.NewReader(f)), f}, nil
	}
	if strings.Contains(mediaType, "gzip") || mediaType == "" {
		var magic [4]byte
		if _, err := io.ReadFull(f, magic[:]); err != nil {
			f.Close()
			return nil, err
		}
		if _, err := f.Seek(0, io.SeekStart); err != nil {
			f.Close()
			return nil, err
		}
		switch {
		case magic[0] == 0x1f && magic[1] == 0x8b:
			zr, err := gzip.NewReader(f)
			if err != nil {
				f.Close()
				return nil, err
			}
			return &closerPair{zr, f}, nil
		case magic == [4]byte{0x28, 0xb5, 0x2f, 0xfd}:
			return &closerPair{io.NopCloser(zstd.NewReader(f)), f}, nil
		}
		return f, nil
	}
	return f, nil
}

type closerPair struct {
	io.ReadCloser
	under io.Closer
}

func (c *closerPair) Close() error {
	err := c.ReadCloser.Close()
	if e := c.under.Close(); err == nil {
		err = e
	}
	return err
}

// ApplyLayer merges one layer tarball into dst.
//
// Deletions are applied in a first pass so a layer that both deletes and
// re-adds a path ends with the re-add. Without the whiteout pass a sequential
// extraction silently resurrects files the image deleted, which for a libc
// test bed can mean a stale loader or an NSS module the image does not ship.
func ApplyLayer(blob, mediaType, dst string) error {
	if err := applyWhiteouts(blob, mediaType, dst); err != nil {
		return err
	}
	return extractLayer(blob, mediaType, dst)
}

func applyWhiteouts(blob, mediaType, dst string) error {
	r, err := openLayer(blob, mediaType)
	if err != nil {
		return err
	}
	defer r.Close()
	tr := tar.NewReader(r)
	for {
		h, err := tr.Next()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
		name := filepath.Clean(strings.TrimPrefix(h.Name, "./"))
		base := filepath.Base(name)
		if !strings.HasPrefix(base, ".wh.") {
			continue
		}
		dir := filepath.Dir(name)
		if dir == "." {
			dir = ""
		}
		if base == ".wh..wh..opq" {
			// Everything the lower layers put in this directory is gone, even
			// entries this layer does not mention.
			target, err := safeJoin(dst, dir)
			if err != nil {
				return err
			}
			entries, err := os.ReadDir(target)
			if err != nil {
				continue
			}
			for _, e := range entries {
				if err := os.RemoveAll(filepath.Join(target, e.Name())); err != nil {
					return err
				}
			}
			continue
		}
		target, err := safeJoin(dst, filepath.Join(dir, strings.TrimPrefix(base, ".wh.")))
		if err != nil {
			return err
		}
		if err := os.RemoveAll(target); err != nil {
			return err
		}
	}
}

func extractLayer(blob, mediaType, dst string) error {
	r, err := openLayer(blob, mediaType)
	if err != nil {
		return err
	}
	defer r.Close()
	tr := tar.NewReader(r)
	type deferredDir struct {
		path string
		mode os.FileMode
		mod  time.Time
	}
	var dirs []deferredDir
	for {
		h, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return err
		}
		name := filepath.Clean(strings.TrimPrefix(h.Name, "./"))
		if name == "." || name == "/" {
			continue
		}
		if strings.HasPrefix(filepath.Base(name), ".wh.") {
			continue
		}
		target, err := safeJoin(dst, name)
		if err != nil {
			return err
		}
		if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
			return err
		}
		mode := h.FileInfo().Mode()
		switch h.Typeflag {
		case tar.TypeDir:
			if err := os.MkdirAll(target, mode.Perm()); err != nil {
				return err
			}
			dirs = append(dirs, deferredDir{target, mode.Perm(), h.ModTime})
		case tar.TypeReg:
			if err := replacePath(target); err != nil {
				return err
			}
			f, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, mode.Perm())
			if err != nil {
				return err
			}
			if _, err := io.Copy(f, tr); err != nil {
				f.Close()
				return err
			}
			if err := f.Close(); err != nil {
				return err
			}
			_ = os.Chmod(target, mode.Perm())
			_ = os.Chtimes(target, h.ModTime, h.ModTime)
		case tar.TypeSymlink:
			if err := replacePath(target); err != nil {
				return err
			}
			if err := os.Symlink(h.Linkname, target); err != nil {
				return err
			}
		case tar.TypeLink:
			source, err := safeJoin(dst, filepath.Clean(strings.TrimPrefix(h.Linkname, "./")))
			if err != nil {
				return err
			}
			if err := replacePath(target); err != nil {
				return err
			}
			if err := os.Link(source, target); err != nil {
				// A hard link across a replaced file is not fatal for a rootfs;
				// fall back to copying the content.
				if err2 := copyPath(source, target, mode.Perm()); err2 != nil {
					return err
				}
			}
		case tar.TypeChar, tar.TypeBlock, tar.TypeFifo:
			if err := replacePath(target); err != nil {
				return err
			}
			if err := mknod(target, h.Typeflag, mode.Perm(), h.Devmajor, h.Devminor); err != nil {
				// Device nodes are cosmetic here: rootfs-run bind-mounts the
				// host /dev, so a refusal is a note rather than a failure.
				log.Debugf("skipping device node %s: %v", name, err)
			}
		default:
			log.Tracef("skipping %s (type %c)", name, h.Typeflag)
			continue
		}
		if h.Uid != 0 || h.Gid != 0 {
			_ = os.Lchown(target, h.Uid, h.Gid)
		}
	}
	// Directory modes are applied last so a read-only directory does not block
	// writing its own contents.
	for _, dir := range slices.Backward(dirs) {
		_ = os.Chmod(dir.path, dir.mode)
		_ = os.Chtimes(dir.path, dir.mod, dir.mod)
	}
	return nil
}

// replacePath removes whatever is at target so a file can replace a directory
// and vice versa, which plain extraction cannot do.
func replacePath(target string) error {
	fi, err := os.Lstat(target)
	if err != nil {
		return nil
	}
	if fi.IsDir() {
		return os.RemoveAll(target)
	}
	return os.Remove(target)
}

func copyPath(src, dst string, mode os.FileMode) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()
	out, err := os.OpenFile(dst, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, mode)
	if err != nil {
		return err
	}
	if _, err := io.Copy(out, in); err != nil {
		out.Close()
		return err
	}
	return out.Close()
}

// safeJoin refuses a path that would escape the destination directory.
func safeJoin(root, name string) (string, error) {
	clean := filepath.Clean("/" + name)
	joined := filepath.Join(root, clean)
	if joined != root && !strings.HasPrefix(joined, root+string(os.PathSeparator)) {
		return "", fmt.Errorf("archive entry %q escapes %s", name, root)
	}
	return joined, nil
}
