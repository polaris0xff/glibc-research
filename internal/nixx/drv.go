// drv.go — nix's own derivation format, so the planner needs no nix.
//
// A .drv is ATerm, one line, no whitespace:
//
//	Derive([(outName,outPath,hashAlgo,hash)...],
//	       [(inputDrvPath,[outNames])...],
//	       [inputSrcs...], system, builder, [args...], [(key,value)...])
//
// Strings are double-quoted with \" \\ \n \r \t escapes. The escapes are nix's,
// not JSON's: a derivation env value routinely contains newlines, and reading
// \n literally puts two characters where a newline belonged.
//
// The whole derivation graph is reachable over HTTPS from these: a narinfo
// names its Deriver, that .drv is itself a signed store path, and its
// References are the .drv paths of its inputs.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
)

// DrvError is a malformed derivation.
type DrvError struct{ msg string }

func (e *DrvError) Error() string { return "drv: " + e.msg }

func drvErr(format string, a ...any) error { return &DrvError{msg: fmt.Sprintf(format, a...)} }

// Output is one (name, path, hashAlgo, hash) tuple.
type Output struct {
	Name     string `json:"name"`
	Path     string `json:"path"`
	HashAlgo string `json:"hashAlgo"`
	Hash     string `json:"hash"`
}

// InputDrv is one (drvPath, outputNames) tuple.
type InputDrv struct {
	Path    string   `json:"path"`
	Outputs []string `json:"outputs"`
}

// Derivation is one parsed .drv.
type Derivation struct {
	Outputs   []Output          `json:"outputs"`
	InputDrvs []InputDrv        `json:"inputDrvs"`
	InputSrcs []string          `json:"inputSrcs"`
	System    string            `json:"system"`
	Builder   string            `json:"builder"`
	Args      []string          `json:"args"`
	Env       map[string]string `json:"env"`

	// Source is the path this was read from, for the show shape.
	Source string `json:"-"`
}

// aterm is the one-pass reader for the format above.
type aterm struct {
	s string
	i int
}

func (p *aterm) peek() byte {
	if p.i < len(p.s) {
		return p.s[p.i]
	}
	return 0
}

func (p *aterm) take(ch byte) error {
	if p.peek() != ch {
		return drvErr("expected %q at offset %d, got %q", string(ch), p.i, string(p.peek()))
	}
	p.i++
	return nil
}

func (p *aterm) str() (string, error) {
	if err := p.take('"'); err != nil {
		return "", err
	}
	var out strings.Builder
	for {
		if p.i >= len(p.s) {
			return "", drvErr("unterminated string")
		}
		c := p.s[p.i]
		p.i++
		if c == '"' {
			return out.String(), nil
		}
		if c != '\\' {
			out.WriteByte(c)
			continue
		}
		if p.i >= len(p.s) {
			return "", drvErr("unterminated escape")
		}
		e := p.s[p.i]
		p.i++
		switch e {
		case 'n':
			out.WriteByte('\n')
		case 'r':
			out.WriteByte('\r')
		case 't':
			out.WriteByte('\t')
		default:
			out.WriteByte(e)
		}
	}
}

// list reads [item, item, ...] with item supplied by the caller.
func (p *aterm) list(item func() error) error {
	if err := p.take('['); err != nil {
		return err
	}
	if p.peek() == ']' {
		p.i++
		return nil
	}
	for {
		if err := item(); err != nil {
			return err
		}
		c := p.peek()
		p.i++
		switch c {
		case ']':
			return nil
		case ',':
		default:
			return drvErr("expected , or ] at offset %d, got %q", p.i, string(c))
		}
	}
}

func (p *aterm) strList() ([]string, error) {
	out := []string{}
	err := p.list(func() error {
		s, err := p.str()
		if err != nil {
			return err
		}
		out = append(out, s)
		return nil
	})
	return out, err
}

// ParseDrv reads one derivation.
func ParseDrv(text string) (*Derivation, error) {
	if !strings.HasPrefix(text, "Derive(") {
		return nil, drvErr("not a derivation: does not begin with Derive(")
	}
	p := &aterm{s: text, i: len("Derive(")}
	d := &Derivation{
		Outputs:   []Output{},
		InputDrvs: []InputDrv{},
		InputSrcs: []string{},
		Args:      []string{},
		Env:       map[string]string{},
	}

	err := p.list(func() error {
		if err := p.take('('); err != nil {
			return err
		}
		var o Output
		fields := []*string{&o.Name, &o.Path, &o.HashAlgo, &o.Hash}
		for n, f := range fields {
			if n > 0 {
				if err := p.take(','); err != nil {
					return err
				}
			}
			v, err := p.str()
			if err != nil {
				return err
			}
			*f = v
		}
		if err := p.take(')'); err != nil {
			return err
		}
		d.Outputs = append(d.Outputs, o)
		return nil
	})
	if err != nil {
		return nil, err
	}
	if err := p.take(','); err != nil {
		return nil, err
	}

	err = p.list(func() error {
		if err := p.take('('); err != nil {
			return err
		}
		path, err := p.str()
		if err != nil {
			return err
		}
		if err := p.take(','); err != nil {
			return err
		}
		outs, err := p.strList()
		if err != nil {
			return err
		}
		if err := p.take(')'); err != nil {
			return err
		}
		d.InputDrvs = append(d.InputDrvs, InputDrv{Path: path, Outputs: outs})
		return nil
	})
	if err != nil {
		return nil, err
	}
	if err := p.take(','); err != nil {
		return nil, err
	}

	if d.InputSrcs, err = p.strList(); err != nil {
		return nil, err
	}
	if err := p.take(','); err != nil {
		return nil, err
	}
	if d.System, err = p.str(); err != nil {
		return nil, err
	}
	if err := p.take(','); err != nil {
		return nil, err
	}
	if d.Builder, err = p.str(); err != nil {
		return nil, err
	}
	if err := p.take(','); err != nil {
		return nil, err
	}
	if d.Args, err = p.strList(); err != nil {
		return nil, err
	}
	if err := p.take(','); err != nil {
		return nil, err
	}
	err = p.list(func() error {
		if err := p.take('('); err != nil {
			return err
		}
		k, err := p.str()
		if err != nil {
			return err
		}
		if err := p.take(','); err != nil {
			return err
		}
		v, err := p.str()
		if err != nil {
			return err
		}
		if err := p.take(')'); err != nil {
			return err
		}
		d.Env[k] = v
		return nil
	})
	if err != nil {
		return nil, err
	}
	if err := p.take(')'); err != nil {
		return nil, err
	}
	return d, nil
}

// ParseDrvFile reads a derivation from disk.
func ParseDrvFile(path string) (*Derivation, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	d, err := ParseDrv(strings.TrimRight(string(b), "\n"))
	if err != nil {
		return nil, fmt.Errorf("%s: %w", path, err)
	}
	d.Source = path
	return d, nil
}

// ShowEntry is the `nix derivation show` shape. One document format with two
// producers means the nix-free route and the nix route share every line of the
// planner below them.
type ShowEntry struct {
	Name            string                     `json:"name"`
	Outputs         map[string]ShowOutput      `json:"outputs"`
	Inputs          ShowInputs                 `json:"inputs"`
	System          string                     `json:"system"`
	Builder         string                     `json:"builder"`
	Args            []string                   `json:"args"`
	Env             map[string]string          `json:"env"`
	StructuredAttrs map[string]json.RawMessage `json:"structuredAttrs,omitempty"`
}

// ShowOutput is one output in that shape.
type ShowOutput struct {
	Path   string `json:"path,omitempty"`
	Hash   string `json:"hash,omitempty"`
	Method string `json:"method,omitempty"`
}

// ShowInputs names the inputs by base name.
type ShowInputs struct {
	Drvs []string `json:"drvs"`
	Srcs []string `json:"srcs"`
}

// ShowDocument is what `nix derivation show` prints.
type ShowDocument struct {
	Derivations map[string]ShowEntry `json:"derivations"`
	Version     int                  `json:"version"`
}

// Show reshapes a derivation into the show format.
//
// __json is structuredAttrs and `nix derivation show` decodes it: in the raw
// .drv the modern attribute set arrives as one env entry holding the whole
// document, while env itself carries only the output names. A reader that does
// not decode it sees a derivation with no src, no patches and no configure
// flags.
func (d *Derivation) Show() (string, ShowEntry) {
	base := filepath.Base(d.Source)
	outs := map[string]ShowOutput{}
	for _, o := range d.Outputs {
		e := ShowOutput{}
		if o.Path != "" {
			e.Path = filepath.Base(o.Path)
		}
		if o.Hash != "" {
			e.Hash = o.Hash
		}
		if o.HashAlgo != "" {
			e.Method = o.HashAlgo
		}
		outs[o.Name] = e
	}
	drvs := []string{}
	for _, in := range d.InputDrvs {
		drvs = append(drvs, filepath.Base(in.Path))
	}
	srcs := []string{}
	for _, s := range d.InputSrcs {
		srcs = append(srcs, filepath.Base(s))
	}
	name := base
	if i := strings.Index(name, "-"); i >= 0 {
		name = name[i+1:]
	}
	name = strings.TrimSuffix(name, ".drv")

	entry := ShowEntry{
		Name:    name,
		Outputs: outs,
		Inputs:  ShowInputs{Drvs: drvs, Srcs: srcs},
		System:  d.System,
		Builder: d.Builder,
		Args:    d.Args,
		Env:     d.Env,
	}
	if raw, ok := d.Env["__json"]; ok {
		var sattrs map[string]json.RawMessage
		if err := json.Unmarshal([]byte(raw), &sattrs); err == nil && len(sattrs) > 0 {
			entry.StructuredAttrs = sattrs
		}
	}
	return base, entry
}

// WriteJSON prints the parsed derivation.
func (d *Derivation) WriteJSON(w io.Writer) error {
	enc := json.NewEncoder(w)
	enc.SetIndent("", " ")
	return enc.Encode(d)
}

// WriteText prints a human summary.
func (d *Derivation) WriteText(w io.Writer) {
	fmt.Fprintf(w, "system   %s\n", d.System)
	fmt.Fprintf(w, "builder  %s\n", d.Builder)
	fmt.Fprintf(w, "args     %s\n", strings.Join(d.Args, " "))
	fmt.Fprintf(w, "outputs  %d\n", len(d.Outputs))
	for _, o := range d.Outputs {
		fmt.Fprintf(w, "  %-10s %s\n", o.Name, o.Path)
	}
	fmt.Fprintf(w, "inputDrvs %d, inputSrcs %d, env %d\n",
		len(d.InputDrvs), len(d.InputSrcs), len(d.Env))
}

// ShowFiles builds the show document for a set of .drv files.
func ShowFiles(paths []string) (ShowDocument, error) {
	doc := ShowDocument{Derivations: map[string]ShowEntry{}, Version: 3}
	for _, p := range paths {
		d, err := ParseDrvFile(p)
		if err != nil {
			return doc, err
		}
		k, e := d.Show()
		doc.Derivations[k] = e
	}
	return doc, nil
}
