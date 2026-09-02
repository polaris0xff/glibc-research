// Package nixx uses nixpkgs as the planner: the indexes that turn a package
// name into a derivation, the ATerm derivation format, the NAR archive format,
// and the plan a pgb build runs from.
//
// index.go carries the two indexes that resolve a name with no nix and no
// evaluation:
//
//  1. releases.nixos.org/nixpkgs/<pin>/packages.json.br — 10 MB on the wire,
//     ~400 MB of JSON, giving name, pname, version, system and outputName per
//     attribute. It is streamed, never loaded.
//  2. hydra.nixos.org/job/.../latest-finished — the build that produced the
//     channel, with drvpath, system and every output's store path.
//
// A reply for a system other than the one asked for is a failure, not a
// result: store-paths.xz covers every system the channel built, and a name
// match there has returned an aarch64-darwin binary.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
)

var log = logx.New("nix")

// IndexHeader is the TSV header the index writes.
const IndexHeader = "# attr\tname\tpname\tversion\tsystem\toutputName\toutputs"

// pkgEntry keeps each field raw so a non-string value renders the way the
// index has always rendered it.
type pkgEntry struct {
	Name       json.RawMessage            `json:"name"`
	Pname      json.RawMessage            `json:"pname"`
	Version    json.RawMessage            `json:"version"`
	System     json.RawMessage            `json:"system"`
	OutputName json.RawMessage            `json:"outputName"`
	Outputs    map[string]json.RawMessage `json:"outputs"`
}

// StreamPackages walks the top-level {"packages": {ATTR: {...}}} object,
// decoding one package value at a time.
func StreamPackages(r io.Reader, emit func(attr string, e pkgEntry) error) (int, error) {
	dec := json.NewDecoder(r)
	tok, err := dec.Token()
	if err != nil {
		return 0, fmt.Errorf("not a JSON document: %w", err)
	}
	if d, ok := tok.(json.Delim); !ok || d != '{' {
		return 0, fmt.Errorf("the document is not a JSON object")
	}
	for dec.More() {
		keyTok, err := dec.Token()
		if err != nil {
			return 0, err
		}
		key, _ := keyTok.(string)
		if key != "packages" {
			var skip json.RawMessage
			if err := dec.Decode(&skip); err != nil {
				return 0, err
			}
			continue
		}
		open, err := dec.Token()
		if err != nil {
			return 0, err
		}
		if d, ok := open.(json.Delim); !ok || d != '{' {
			return 0, fmt.Errorf(`"packages" is not an object`)
		}
		n := 0
		for dec.More() {
			attrTok, err := dec.Token()
			if err != nil {
				return n, err
			}
			attr, _ := attrTok.(string)
			var e pkgEntry
			if err := dec.Decode(&e); err != nil {
				return n, fmt.Errorf("%s: %w", attr, err)
			}
			if err := emit(attr, e); err != nil {
				return n, err
			}
			n++
		}
		if _, err := dec.Token(); err != nil {
			return n, fmt.Errorf("truncated packages object: %w", err)
		}
		return n, nil
	}
	return 0, fmt.Errorf(`no "packages" key in this document`)
}

// BuildIndex streams packages.json into a TSV every later lookup reads.
func BuildIndex(src, out string) error {
	in, err := os.Open(src)
	if err != nil {
		return fail.Cannot("cannot read %s: %v", src, err)
	}
	defer in.Close()

	tmp := out + ".part"
	w, err := os.Create(tmp)
	if err != nil {
		return fail.Cannot("cannot write %s: %v", tmp, err)
	}
	bw := newBufWriter(w)
	fmt.Fprintln(bw, IndexHeader)

	n, err := StreamPackages(in, func(attr string, e pkgEntry) error {
		outs := make([]string, 0, len(e.Outputs))
		for k := range e.Outputs {
			outs = append(outs, k)
		}
		sort.Strings(outs)
		_, werr := fmt.Fprintf(bw, "%s\t%s\t%s\t%s\t%s\t%s\t%s\n",
			attr, renderValue(e.Name), renderValue(e.Pname), renderValue(e.Version),
			renderValue(e.System), renderValue(e.OutputName), strings.Join(outs, ","))
		return werr
	})
	if err != nil {
		bw.Flush()
		w.Close()
		os.Remove(tmp)
		return fail.Ran("%s: %v", src, err)
	}
	if err := bw.Flush(); err != nil {
		w.Close()
		return err
	}
	if err := w.Close(); err != nil {
		return err
	}
	if err := os.Rename(tmp, out); err != nil {
		return err
	}
	log.Infof("%d attributes -> %s", n, out)
	fmt.Fprintf(os.Stderr, "nix-index: %d attributes -> %s\n", n, out)
	return nil
}

// renderValue turns a raw JSON value into the index's text form: a string
// unquoted, an absent field empty, and anything else its literal JSON.
func renderValue(raw json.RawMessage) string {
	if len(raw) == 0 {
		return ""
	}
	if raw[0] == '"' {
		var s string
		if err := json.Unmarshal(raw, &s); err == nil {
			return s
		}
	}
	switch string(raw) {
	case "null":
		return "None"
	case "true":
		return "True"
	case "false":
		return "False"
	}
	return string(raw)
}

// IndexRow is one line of the TSV.
type IndexRow struct {
	Attr       string
	Name       string
	Pname      string
	Version    string
	System     string
	OutputName string
	Outputs    []string
}

// ReadIndex parses the TSV, optionally filtering by system.
func ReadIndex(path string, system string) ([]IndexRow, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, fail.Cannot("cannot read %s: %v", path, err)
	}
	var out []IndexRow
	for line := range strings.SplitSeq(string(b), "\n") {
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		f := strings.Split(line, "\t")
		if len(f) < 7 {
			continue
		}
		if system != "" && f[4] != system {
			continue
		}
		row := IndexRow{Attr: f[0], Name: f[1], Pname: f[2], Version: f[3],
			System: f[4], OutputName: f[5]}
		if f[6] != "" {
			row.Outputs = strings.Split(f[6], ",")
		}
		out = append(out, row)
	}
	return out, nil
}

// HydraReply is the part of a latest-finished document pgb reads.
type HydraReply struct {
	DrvPath      string         `json:"drvpath"`
	System       string         `json:"system"`
	NixName      string         `json:"nixname"`
	Job          string         `json:"job"`
	Finished     any            `json:"finished"`
	BuildStatus  any            `json:"buildstatus"`
	JobsetEvals  []any          `json:"jobsetevals"`
	BuildOutputs map[string]any `json:"buildoutputs"`
}

// ReadHydra renders a latest-finished reply as key: value lines, refusing a
// reply for a system other than the one asked for.
func ReadHydra(path, wantSystem string) error {
	if wantSystem == "" {
		wantSystem = "x86_64-linux"
	}
	b, err := os.ReadFile(path)
	if err != nil {
		return fail.Cannot("hydra: cannot read %s: %v", path, err)
	}
	var d HydraReply
	if err := json.Unmarshal(b, &d); err != nil {
		return fail.Ran("hydra: not JSON: %v", err)
	}
	if d.DrvPath == "" {
		return fail.Ran("hydra: the reply names no drvpath")
	}
	if d.System != wantSystem {
		return fail.Ran("hydra: reply is for %s, wanted %s", d.System, wantSystem)
	}
	logx.Say("Drv: %s", d.DrvPath)
	logx.Say("System: %s", d.System)
	logx.Say("Nixname: %s", d.NixName)
	logx.Say("Job: %s", d.Job)
	logx.Say("Finished: %s", pyValue(d.Finished))
	logx.Say("Buildstatus: %s", pyValue(d.BuildStatus))
	var evals []int
	for _, e := range d.JobsetEvals {
		if f, ok := e.(float64); ok && f == float64(int(f)) {
			evals = append(evals, int(f))
		}
	}
	if len(evals) > 0 {
		// One line, not one per eval: a popular job is in dozens and the list
		// buries the fields a caller reads.
		latest := evals[0]
		for _, e := range evals {
			if e > latest {
				latest = e
			}
		}
		logx.Say("Evals: %d", len(evals))
		logx.Say("EvalLatest: %d", latest)
	}
	names := make([]string, 0, len(d.BuildOutputs))
	for k := range d.BuildOutputs {
		names = append(names, k)
	}
	sort.Strings(names)
	for _, name := range names {
		m, ok := d.BuildOutputs[name].(map[string]any)
		if !ok {
			continue
		}
		if p, ok := m["path"].(string); ok && p != "" {
			logx.Say("Out.%s: %s", name, p)
		}
	}
	return nil
}

// pyValue renders a decoded JSON scalar the way the index has always printed
// it, so a caller parsing these lines sees no change.
func pyValue(v any) string {
	switch t := v.(type) {
	case nil:
		return "None"
	case bool:
		if t {
			return "True"
		}
		return "False"
	case float64:
		if t == float64(int64(t)) {
			return fmt.Sprintf("%d", int64(t))
		}
		return fmt.Sprintf("%v", t)
	case string:
		return t
	default:
		return fmt.Sprintf("%v", t)
	}
}

// IndexPath is where a built index lives for a channel pin.
func IndexPath(cacheDir, pin string) string {
	return filepath.Join(cacheDir, "packages-"+pin+".tsv")
}
