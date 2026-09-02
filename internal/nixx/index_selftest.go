// index_selftest.go — the streaming package index, and the refusals that keep
// it honest.
//
// The document is shaped like the real one, including a value large enough to
// cross a read boundary, and it is walked through readers of several sizes: a
// walk that loses entries at one buffer size and not another is the failure
// this exists to catch, and the count from a whole-document decode is the
// control.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// IndexSelftest exercises the index and the hydra reader.
func IndexSelftest() *selftest.Report {
	r := selftest.New("nix-index")

	filler := strings.Repeat("x", 3000)
	doc := `{"packages":{` +
		`"jq":{"name":"jq-1.8.2","pname":"jq","version":"1.8.2","system":"x86_64-linux",` +
		`"outputName":"bin","outputs":{"bin":null,"out":null},"meta":{"description":"` + filler + `"}},` +
		`"bash":{"name":"bash-interactive-5.3p15","pname":"bash-interactive","version":"5.3p15",` +
		`"system":"x86_64-linux","outputName":"out","outputs":{"out":null},` +
		`"meta":{"description":"` + filler + `"}}},"version":2}`

	seen := map[string][2]string{}
	n, err := StreamPackages(strings.NewReader(doc), func(attr string, e pkgEntry) error {
		seen[attr] = [2]string{renderValue(e.Name), renderValue(e.OutputName)}
		return nil
	})
	if err != nil {
		r.Fail("stream the sample document", err.Error(), "no error")
	}
	r.Check("streamed attribute count", strconv.Itoa(n), "2")
	r.Check("bash resolves to its real name", seen["bash"][0], "bash-interactive-5.3p15")
	r.Check("jq's default output", seen["jq"][1], "bin")

	// A larger document read through readers of several sizes must produce the
	// same count as a whole-document decode.
	var big strings.Builder
	big.WriteString(`{"packages":{`)
	for k := 0; k < 500; k++ {
		if k > 0 {
			big.WriteString(",")
		}
		fmt.Fprintf(&big, `"p%04d":{"name":"p%04d-1.0","system":"x86_64-linux","outputName":"out","meta":{"d":"%s"}}`,
			k, k, strings.Repeat("y", 200))
	}
	big.WriteString(`},"version":2}`)

	var truth struct {
		Packages map[string]json.RawMessage `json:"packages"`
	}
	if err := json.Unmarshal([]byte(big.String()), &truth); err != nil {
		r.Fail("control decode", err.Error(), "no error")
	}
	want := strconv.Itoa(len(truth.Packages))
	for _, size := range []int{1, 7, 997, 65536} {
		count := 0
		_, err := StreamPackages(newChoppyReader(big.String(), size), func(string, pkgEntry) error {
			count++
			return nil
		})
		if err != nil {
			r.Fail(fmt.Sprintf("reader=%d loses nothing", size), err.Error(), "no error")
			continue
		}
		r.Check(fmt.Sprintf("reader=%-5d loses nothing against a whole decode", size),
			strconv.Itoa(count), want)
	}

	// A truncated document must raise, not return a short list quietly.
	_, err = StreamPackages(strings.NewReader(`{"packages":{"a":{"name":"x"`), func(string, pkgEntry) error { return nil })
	r.CheckBool("truncated document refused", err != nil, true)

	// A reply for the wrong system is a failure, not a result: this is the
	// defect that made a Mach-O binary look like a usable build.
	tmp, err := os.CreateTemp("", "pgb-hydra-*.json")
	if err != nil {
		r.Fail("tempfile", err.Error(), "created")
		return r
	}
	defer os.Remove(tmp.Name())
	_, _ = tmp.WriteString(`{"drvpath":"/nix/store/a-b.drv","system":"aarch64-darwin"}`)
	tmp.Close()
	r.CheckBool("hydra refuses a reply for another system",
		ReadHydra(tmp.Name(), "x86_64-linux") != nil, true)
	r.CheckBool("hydra accepts the system it was asked for",
		ReadHydra(tmp.Name(), "aarch64-darwin") == nil, true)

	// The written TSV must read back as the rows that went in.
	dir, err := os.MkdirTemp("", "pgb-index-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "created")
		return r
	}
	defer os.RemoveAll(dir)
	src := filepath.Join(dir, "packages.json")
	out := filepath.Join(dir, "packages.tsv")
	if err := os.WriteFile(src, []byte(doc), 0o644); err != nil {
		r.Fail("write fixture", err.Error(), "created")
		return r
	}
	if err := BuildIndex(src, out); err != nil {
		r.Fail("build the index", err.Error(), "no error")
		return r
	}
	rows, err := ReadIndex(out, "x86_64-linux")
	if err != nil {
		r.Fail("read the index", err.Error(), "no error")
		return r
	}
	r.Check("index round trip: row count", strconv.Itoa(len(rows)), "2")
	byAttr := map[string]IndexRow{}
	for _, row := range rows {
		byAttr[row.Attr] = row
	}
	r.Check("index round trip: jq outputs", strings.Join(byAttr["jq"].Outputs, ","), "bin,out")
	r.Check("index round trip: bash name", byAttr["bash"].Name, "bash-interactive-5.3p15")
	return r
}

// choppyReader hands out at most n bytes per Read, so a reader that depends on
// its buffering is caught.
type choppyReader struct {
	s string
	i int
	n int
}

func newChoppyReader(s string, n int) io.Reader { return &choppyReader{s: s, n: n} }

func (c *choppyReader) Read(p []byte) (int, error) {
	if c.i >= len(c.s) {
		return 0, io.EOF
	}
	n := c.n
	if n > len(p) {
		n = len(p)
	}
	if c.i+n > len(c.s) {
		n = len(c.s) - c.i
	}
	copy(p, c.s[c.i:c.i+n])
	c.i += n
	return n, nil
}
