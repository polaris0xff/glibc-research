// fetch.go — resolve and fetch nixpkgs store paths with no nix installed.
//
// Three plain HTTPS endpoints and nothing else:
//
//  1. channels.nixos.org/<channel>/store-paths.xz — a 302 to a release URL
//     that names the revision, so following it once gives a pin, and the body
//     is every store path that channel built.
//  2. cache.nixos.org/<hash>.narinfo — the metadata and the signature.
//  3. cache.nixos.org/<the narinfo's URL> — the NAR.
//
// This fetches what a channel ALREADY BUILT. It does not evaluate nix
// expressions, so it cannot reach an attribute nobody has built, cannot apply
// an overlay, and cannot build from source.
//
// Every fetch is verified and refusing is the point: the narinfo's ed25519
// signature against the pinned key, and the NAR's sha256 against the NarHash
// that signature covers.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"bufio"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"

	"github.com/polaris0xff/glibc-research/internal/fail"
)

// Client fetches from a channel and a substituter.
type Client struct {
	Cache       string
	Substituter string
	Channel     string
	System      string
	Jobset      string

	http *http.Client
}

// NewClient applies the defaults every subcommand shares.
func NewClient() *Client {
	c := &Client{
		Cache:       envOr("NIX_FETCH_CACHE", "/var/tmp/pgb-nix-cache"),
		Substituter: envOr("NIX_FETCH_SUBSTITUTER", "https://cache.nixos.org"),
		Channel:     "nixpkgs-unstable",
		// store-paths.xz is every system the channel built: resolving a name
		// without stating the system has returned an aarch64-darwin build.
		System: "x86_64-linux",
		Jobset: "nixpkgs/trunk",
	}
	c.http = &http.Client{Timeout: 30 * time.Minute}
	return c
}

func envOr(name, def string) string {
	if v := os.Getenv(name); v != "" {
		return v
	}
	return def
}

func (c *Client) ensureCache() error {
	if err := os.MkdirAll(c.Cache, 0o755); err != nil {
		return fail.Cannot("cannot create %s: %v", c.Cache, err)
	}
	return nil
}

// ReleaseURL follows the channel's one redirect. The redirect is the pin:
// it names the exact nixpkgs revision the channel points at.
func (c *Client) ReleaseURL() (string, error) {
	url := fmt.Sprintf("https://channels.nixos.org/%s/store-paths.xz", c.Channel)
	noRedirect := &http.Client{
		Timeout: 2 * time.Minute,
		CheckRedirect: func(*http.Request, []*http.Request) error {
			return http.ErrUseLastResponse
		},
	}
	req, err := http.NewRequest(http.MethodHead, url, nil)
	if err != nil {
		return "", err
	}
	resp, err := noRedirect.Do(req)
	if err != nil {
		return "", fail.Ran("cannot reach %s: %v", url, err)
	}
	defer resp.Body.Close()
	loc := resp.Header.Get("Location")
	if loc == "" {
		return "", fail.Ran("the channel did not redirect: is %s a channel name?", c.Channel)
	}
	return loc, nil
}

// Revision extracts the nixpkgs revision from a release URL.
func Revision(releaseURL string) string {
	m := regexp.MustCompile(`/nixpkgs-([^/]*)/`).FindStringSubmatch(releaseURL)
	if len(m) < 2 {
		return ""
	}
	return m[1]
}

// IndexFile materialises the channel's store-path list, decompressed.
func (c *Client) IndexFile() (string, error) {
	if err := c.ensureCache(); err != nil {
		return "", err
	}
	idx := filepath.Join(c.Cache, "store-paths."+c.Channel+".txt")
	if fi, err := os.Stat(idx); err == nil && fi.Size() > 0 {
		return idx, nil
	}
	url, err := c.ReleaseURL()
	if err != nil {
		return "", err
	}
	if err := os.WriteFile(filepath.Join(c.Cache, "store-paths."+c.Channel+".pin"),
		[]byte(url+"\n"), 0o644); err != nil {
		return "", err
	}
	log.Infof("index <- %s", url)
	body, err := c.get(url)
	if err != nil {
		return "", fail.Ran("could not fetch the index: %v", err)
	}
	defer body.Close()
	dec, err := Decompress(body, "xz")
	if err != nil {
		return "", fail.Cannot("%v", err)
	}
	defer dec.Close()
	part := idx + ".part"
	f, err := os.Create(part)
	if err != nil {
		return "", err
	}
	if _, err := io.Copy(f, dec); err != nil {
		f.Close()
		os.Remove(part)
		return "", fail.Ran("could not read the index: %v", err)
	}
	if err := f.Close(); err != nil {
		return "", err
	}
	return idx, os.Rename(part, idx)
}

// AttrsFile materialises the attribute index as a TSV.
//
// packages.json.br sits beside store-paths.xz in the same pinned release
// directory. It carries what store-paths.xz cannot: the attribute path, the
// derivation name that attribute produces, the default output, and the system.
func (c *Client) AttrsFile() (string, error) {
	if err := c.ensureCache(); err != nil {
		return "", err
	}
	at := filepath.Join(c.Cache, "attrs."+c.Channel+".tsv")
	if fi, err := os.Stat(at); err == nil && fi.Size() > 0 {
		return at, nil
	}
	url, err := c.ReleaseURL()
	if err != nil {
		return "", err
	}
	pj := strings.TrimSuffix(url, "/store-paths.xz") + "/packages.json.br"
	log.Infof("attribute index <- %s", pj)

	// The document is served with Content-Encoding: br, which is not in the
	// standard library, so this one fetch goes through curl --compressed. It is
	// streamed to disk and then streamed again: the decoded document is ~400 MB
	// and the index walk writes a ~10 MB TSV.
	raw := filepath.Join(c.Cache, "packages."+c.Channel+".json.part")
	if err := fetchDecoded(pj, raw); err != nil {
		return "", err
	}
	defer os.Remove(raw)
	if err := BuildIndex(raw, at); err != nil {
		return "", err
	}
	return at, nil
}

// fetchDecoded downloads a URL whose body carries a Content-Encoding the
// standard library does not decode.
func fetchDecoded(url, dst string) error {
	if _, err := exec.LookPath("curl"); err != nil {
		return fail.Cannot("%s is brotli-encoded and curl is not on PATH to decode it", url)
	}
	cmd := exec.Command("curl", "-sSfL", "--compressed", url, "-o", dst)
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		return fail.Ran("could not fetch %s: %v", url, err)
	}
	return nil
}

func (c *Client) get(url string) (io.ReadCloser, error) {
	var lastErr error
	for attempt := 0; attempt < 3; attempt++ {
		if attempt > 0 {
			time.Sleep(time.Duration(attempt) * 2 * time.Second)
		}
		req, err := http.NewRequest(http.MethodGet, url, nil)
		if err != nil {
			return nil, err
		}
		req.Header.Set("Accept", "*/*")
		resp, err := c.http.Do(req)
		if err != nil {
			lastErr = err
			continue
		}
		if resp.StatusCode == http.StatusOK {
			return resp.Body, nil
		}
		resp.Body.Close()
		lastErr = fmt.Errorf("%s: HTTP %d", url, resp.StatusCode)
		if resp.StatusCode < 500 {
			break
		}
	}
	return nil, lastErr
}

// HashOf reduces a store path, a base name or a bare hash to the hash part.
func HashOf(s string) string {
	s = strings.TrimPrefix(s, "/nix/store/")
	if i := strings.IndexByte(s, '-'); i == 32 {
		return s[:32]
	}
	if len(s) >= 32 {
		return s[:32]
	}
	return s
}

// NarinfoPath fetches and verifies one narinfo, caching it.
//
// It is verified BEFORE it is cached, never after: a narinfo that failed its
// signature must not be left on disk where the next run reads it as checked.
func (c *Client) NarinfoPath(hash string) (string, error) {
	dir := filepath.Join(c.Cache, "narinfo")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return "", err
	}
	f := filepath.Join(dir, hash+".narinfo")
	if fi, err := os.Stat(f); err == nil && fi.Size() > 0 {
		return f, nil
	}
	body, err := c.get(c.Substituter + "/" + hash + ".narinfo")
	if err != nil {
		return "", fail.Ran("no narinfo for %s: %v", hash, err)
	}
	defer body.Close()
	data, err := io.ReadAll(body)
	if err != nil {
		return "", err
	}
	info := ParseNarinfo(string(data))
	ok, why := info.Verify(DefaultKeys())
	if !ok {
		return "", fail.Ran("narinfo for %s did not verify: %s", hash, why)
	}
	if err := os.WriteFile(f+".part", data, 0o644); err != nil {
		return "", err
	}
	return f, os.Rename(f+".part", f)
}

// Narinfo reads a verified narinfo.
func (c *Client) Narinfo(pathOrHash string) (Narinfo, error) {
	f, err := c.NarinfoPath(HashOf(pathOrHash))
	if err != nil {
		return nil, err
	}
	b, err := os.ReadFile(f)
	if err != nil {
		return nil, err
	}
	return ParseNarinfo(string(b)), nil
}

// Closure walks References breadth-first. A store path references itself in
// its own narinfo, which is a fact to skip rather than a cycle to break.
func (c *Client) Closure(root string) ([]string, error) {
	seen := map[string]bool{}
	var out []string
	todo := []string{HashOf(root)}
	for len(todo) > 0 {
		var next []string
		for _, h := range todo {
			if seen[h] {
				continue
			}
			seen[h] = true
			info, err := c.Narinfo(h)
			if err != nil {
				return nil, err
			}
			out = append(out, info["StorePath"])
			for _, r := range strings.Fields(info["References"]) {
				rh := HashOf(r)
				if !seen[rh] {
					next = append(next, rh)
				}
			}
		}
		todo = next
	}
	return out, nil
}

// Unpack decompresses, hashes and extracts from one stream, refusing when the
// hash does not match what the signature covers.
func Unpack(r io.Reader, compression, wantHash, dest string) error {
	dec, err := Decompress(r, compression)
	if err != nil {
		return err
	}
	defer dec.Close()
	h := sha256.New()
	if err := NarExtract(io.TeeReader(dec, h), dest); err != nil {
		return err
	}
	got := "sha256:" + NixBase32(h.Sum(nil))
	if wantHash != "" && got != wantHash {
		return fmt.Errorf("NAR hash mismatch: got %s, the signed narinfo says %s", got, wantHash)
	}
	return nil
}

// FetchOptions controls a store-path fetch.
type FetchOptions struct {
	Out         string
	WithClosure bool
}

// Fetch downloads a store path, and by default its whole closure, into a
// directory.
func (c *Client) Fetch(root string, o FetchOptions) ([]string, error) {
	if o.Out == "" {
		return nil, fail.Cannot("fetch needs an output directory")
	}
	if err := os.MkdirAll(o.Out, 0o755); err != nil {
		return nil, fail.Cannot("cannot create %s: %v", o.Out, err)
	}
	var paths []string
	if o.WithClosure {
		var err error
		if paths, err = c.Closure(root); err != nil {
			return nil, err
		}
	} else {
		info, err := c.Narinfo(root)
		if err != nil {
			return nil, err
		}
		paths = []string{info["StorePath"]}
	}

	var written []string
	for _, p := range paths {
		base := strings.TrimPrefix(p, "/nix/store/")
		dest := filepath.Join(o.Out, base)
		if _, err := os.Lstat(dest); err == nil {
			continue
		}
		info, err := c.Narinfo(p)
		if err != nil {
			return written, err
		}
		body, err := c.get(c.Substituter + "/" + info["URL"])
		if err != nil {
			return written, fail.Ran("fetch failed for %s: %v", p, err)
		}
		err = Unpack(body, info["Compression"], info["NarHash"], dest+".part")
		body.Close()
		if err != nil {
			os.RemoveAll(dest + ".part")
			return written, fail.Ran("fetch failed for %s: %v", p, err)
		}
		if err := os.Rename(dest+".part", dest); err != nil {
			return written, err
		}
		written = append(written, dest)
	}
	log.Infof("%d path(s) fetched into %s", len(written), o.Out)
	return written, nil
}

// AttrMatch is what the attribute index resolved a name to.
type AttrMatch struct {
	Attr       string
	Name       string
	Pname      string
	Version    string
	System     string
	OutputName string
	Outputs    []string
	MatchedBy  string // attr | pname | name
	Candidates int
}

// Attr resolves a name against the attribute index.
//
// Three matches are tried in order — the attribute path, then pname, then the
// derivation name — and a tie is broken by depth, because the first row is the
// wrong answer: matching `sed` on pname alone returns `freebsd.sed`, a real
// package for the wrong userland and indistinguishable from a correct answer
// once it is one line of output. Only rows for the system asked for are
// eligible.
func (c *Client) Attr(want string) (*AttrMatch, error) {
	at, err := c.AttrsFile()
	if err != nil {
		return nil, err
	}
	rows, err := ReadIndex(at, "")
	if err != nil {
		return nil, err
	}
	best := -1
	bestRank, bestDepth := 0, 0
	counts := map[int]int{}
	for i, r := range rows {
		if r.System != c.System && r.System != "" {
			continue
		}
		rank := 0
		switch {
		case r.Attr == want:
			rank = 3
		case r.Pname == want:
			rank = 2
		case r.Name == want:
			rank = 1
		default:
			continue
		}
		counts[rank]++
		depth := strings.Count(r.Attr, ".")
		better := rank > bestRank ||
			(rank == bestRank && (depth < bestDepth ||
				(depth == bestDepth && best >= 0 && len(r.Attr) < len(rows[best].Attr))))
		if best < 0 || better {
			best, bestRank, bestDepth = i, rank, depth
		}
	}
	if best < 0 {
		return nil, fail.Ran("no attribute, pname or name %q for %s in the %s index",
			want, c.System, c.Channel)
	}
	r := rows[best]
	how := map[int]string{3: "attr", 2: "pname", 1: "name"}[bestRank]
	return &AttrMatch{
		Attr: r.Attr, Name: r.Name, Pname: r.Pname, Version: r.Version,
		System: r.System, OutputName: r.OutputName, Outputs: r.Outputs,
		MatchedBy: how, Candidates: counts[bestRank],
	}, nil
}

// Resolve greps the channel's store-path index.
func (c *Client) Resolve(pattern string, limit int) ([]string, error) {
	idx, err := c.IndexFile()
	if err != nil {
		return nil, err
	}
	re, err := regexp.Compile(pattern)
	if err != nil {
		return nil, fail.Cannot("resolve: %v", err)
	}
	f, err := os.Open(idx)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	var out []string
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<20)
	for sc.Scan() {
		if re.MatchString(sc.Text()) {
			out = append(out, sc.Text())
			if limit > 0 && len(out) >= limit {
				break
			}
		}
	}
	return out, sc.Err()
}

// IndexHas reports whether the channel index carries a store path exactly.
func (c *Client) IndexHas(paths []string) (present, missing int, err error) {
	idx, err := c.IndexFile()
	if err != nil {
		return 0, 0, err
	}
	f, err := os.Open(idx)
	if err != nil {
		return 0, 0, err
	}
	defer f.Close()
	want := map[string]bool{}
	for _, p := range paths {
		want[p] = true
	}
	found := map[string]bool{}
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<20)
	for sc.Scan() {
		if want[sc.Text()] {
			found[sc.Text()] = true
		}
	}
	if err := sc.Err(); err != nil {
		return 0, 0, err
	}
	return len(found), len(want) - len(found), nil
}

// HydraJob fetches a job's latest-finished document, caching it.
//
// hydra built the channel, so it knows the derivation for every job: drvpath,
// the system, and each output's store path. This route has no Deriver-field
// availability ceiling because it is an index of builds rather than a field
// somebody happened to upload.
func (c *Client) HydraJob(attr string) (string, error) {
	dir := filepath.Join(c.Cache, "hydra")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return "", err
	}
	key := strings.ReplaceAll(fmt.Sprintf("%s/%s.%s", c.Jobset, attr, c.System), "/", "_")
	f := filepath.Join(dir, key+".json")
	if fi, err := os.Stat(f); err == nil && fi.Size() > 0 {
		return f, nil
	}
	url := fmt.Sprintf("https://hydra.nixos.org/job/%s/%s.%s/latest-finished",
		c.Jobset, attr, c.System)
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return "", err
	}
	req.Header.Set("Accept", "application/json")
	resp, err := c.http.Do(req)
	if err != nil {
		// The error names the failure it had, not the one it expected: a TLS
		// failure reported as "hydra has no finished build" sends the reader to
		// look for a package that is plainly there.
		return "", fail.Ran("could not reach %s: %v", url, err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return "", fail.Ran("could not reach %s: HTTP %d", url, resp.StatusCode)
	}
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}
	if err := os.WriteFile(f+".part", data, 0o644); err != nil {
		return "", err
	}
	return f, os.Rename(f+".part", f)
}

// HydraEvalRevision names the nixpkgs revision an eval was of.
func (c *Client) HydraEvalRevision(eval int) string {
	f := filepath.Join(c.Cache, "hydra", fmt.Sprintf("eval-%d.json", eval))
	if fi, err := os.Stat(f); err != nil || fi.Size() == 0 {
		body, err := c.get(fmt.Sprintf("https://hydra.nixos.org/eval/%d", eval))
		if err != nil {
			return ""
		}
		defer body.Close()
		data, err := io.ReadAll(body)
		if err != nil {
			return ""
		}
		if err := os.WriteFile(f, data, 0o644); err != nil {
			return ""
		}
	}
	b, err := os.ReadFile(f)
	if err != nil {
		return ""
	}
	var doc struct {
		Inputs map[string]struct {
			Revision string `json:"revision"`
		} `json:"jobsetevalinputs"`
	}
	if err := json.Unmarshal(b, &doc); err != nil {
		return ""
	}
	return doc.Inputs["nixpkgs"].Revision
}

// SortedStrings is a small helper for reporting.
func SortedStrings(in []string) []string {
	out := append([]string(nil), in...)
	sort.Strings(out)
	return out
}
