// plan.go — turn a nixpkgs derivation into a pgb build plan.
//
// The derivation is the source of truth, not the .nix expression: a derivation
// is what nix decided after every override, overlay and conditional has run.
//
// A plan carries the INPUTS — source, patches, flags, dependency names — and
// not nixpkgs' builder script. Carrying stdenv's setup hooks would mean
// carrying stdenv, and stdenv is where the /nix/store paths get baked in.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"math/big"
	"path"
	"sort"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/proc"
)

// ShowDrv is one derivation as `nix derivation show` prints it. env is
// map[string]any because the two derivation shapes disagree about types: the
// old one space-joins lists into a string, __structuredAttrs keeps them as
// JSON arrays.
type ShowDrv struct {
	Name            string                `json:"name"`
	Outputs         map[string]ShowOutput `json:"outputs"`
	System          string                `json:"system"`
	Env             map[string]any        `json:"env"`
	StructuredAttrs map[string]any        `json:"structuredAttrs"`
}

// ShowRecursive is the document `nix derivation show --recursive` writes, and
// the one pgb's own nix-free route emits.
type ShowRecursive struct {
	Derivations map[string]ShowDrv `json:"derivations"`
}

// Plan is the pgb build plan. Fields are in alphabetical order so the encoded
// document matches the sorted-key form every existing plan was written in.
type Plan struct {
	Attr                  string         `json:"attr"`
	BuildInputs           []string       `json:"buildInputs"`
	BuildSystemHooks      []string       `json:"buildSystemHooks"`
	CmakeFlags            []string       `json:"cmakeFlags"`
	ConfigureFlags        []string       `json:"configureFlags"`
	Deps                  []DepRecord    `json:"deps"`
	Drv                   string         `json:"drv"`
	MakeFlags             []string       `json:"makeFlags"`
	MesonFlags            []string       `json:"mesonFlags"`
	NativeBuildInputs     []string       `json:"nativeBuildInputs"`
	NixOnly               map[string]any `json:"nix_only"`
	Nixpkgs               string         `json:"nixpkgs"`
	Outputs               []string       `json:"outputs"`
	Patches               []Source       `json:"patches"`
	Pname                 string         `json:"pname"`
	PropagatedBuildInputs []string       `json:"propagatedBuildInputs"`
	Schema                string         `json:"schema"`
	Src                   *Source        `json:"src"`
	System                string         `json:"system"`
	Version               string         `json:"version"`
}

// Source is a store path with the upstream URLs and hash that produced it.
// Fields are alphabetical for the same reason.
type Source struct {
	OutputHash string   `json:"outputHash"`
	Store      string   `json:"store"`
	URLs       []string `json:"urls"`
}

// DepRecord names a dependency and the derivation that builds it, because a
// name alone is not actionable: `ncurses-6.6-dev` is not a nixpkgs attribute.
type DepRecord struct {
	Drv  string `json:"drv"`
	Name string `json:"name"`
	Out  string `json:"out"`
}

// hookMap reads the build system off nixpkgs' own setup hooks, which is
// strictly better than sniffing the unpacked tree: a source shipping both
// configure.ac and CMakeLists.txt is ambiguous to a sniffer and not to the
// derivation.
var hookMap = []struct{ Hook, Flag string }{
	{"autoreconf-hook", "autoreconf"},
	{"cmake-", "cmake"},
	{"meson-", "meson"},
	{"ninja-", "ninja"},
	{"pkg-config-wrapper", "pkg-config"},
	{"rustc", "cargo"},
	{"cargo-", "cargo"},
	{"go-", "go"},
}

// nixOnlyKeys are carried and deliberately not acted on: they are nixpkgs'
// own shell fragments, and running them needs stdenv's environment. They are
// in the plan so a human debugging a failed build can see what nixpkgs did.
var nixOnlyKeys = []string{
	"postPatch", "preConfigure", "postInstall", "prePatch",
	"preBuild", "postBuild", "patchFlags", "NIX_CFLAGS_COMPILE",
	"hardeningDisable", "dontDisableStatic", "env",
}

// BuildPlan turns a recursive derivation document into a plan. nixPrefix, when
// non-empty, is a nix installation whose `nix-store -q --deriver` is consulted
// for a store path the document does not index.
func BuildPlan(doc ShowRecursive, attr, drvPath, nixpkgsVersion, nixPrefix string) (*Plan, error) {
	drvs := doc.Derivations
	if _, ok := drvs[drvPath]; !ok {
		// `nix derivation show` keys by the full store path; a caller passing a
		// bare basename should still work rather than get an empty plan.
		base := path.Base(drvPath)
		found := ""
		var keys []string
		for k := range drvs {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			if strings.HasSuffix(k, base) {
				found = k
				break
			}
		}
		if found == "" {
			return nil, fmt.Errorf("%s is not in the document", drvPath)
		}
		drvPath = found
	}
	top := drvs[drvPath]
	env := attrsOf(top)

	// Index every derivation by the outputs it produces: the recursive document
	// already contains the whole graph, so an output path maps back to its
	// derivation without the local store holding anything.
	//
	// An output path can be claimed by more than one derivation — two .drv
	// files that build to the same output are equivalent, and a real graph has
	// several. The claimants are therefore sorted and the first taken, so the
	// plan is a property of the graph rather than of the order the document
	// happened to arrive in.
	claims := map[string][]string{}
	for p, d := range drvs {
		for _, o := range d.Outputs {
			if o.Path != "" {
				b := path.Base(o.Path)
				claims[b] = append(claims[b], fullStore(p))
			}
		}
	}
	outIndex := make(map[string]string, len(claims))
	ambiguous := 0
	for b, list := range claims {
		sort.Strings(list)
		if len(list) > 1 {
			ambiguous++
			log.Tracef("%s is produced by %d derivations; taking %s", b, len(list), list[0])
		}
		outIndex[b] = list[0]
	}
	if ambiguous > 0 {
		log.Debugf("%d output paths are produced by more than one derivation; "+
			"the lowest store path is taken for each", ambiguous)
	}

	// Index the fixed-output derivations too: those are the fetchurl calls, and
	// they are the only place the upstream URL and its hash exist.
	byName := map[string][]Source{}
	for p, d := range drvs {
		e := attrsOf(d)
		h := d.Outputs["out"].Hash
		if h == "" {
			h, _ = e["outputHash"].(string)
		}
		if h == "" {
			continue
		}
		urls := splitWS(e["urls"])
		if len(urls) == 0 {
			urls = splitWS(e["url"])
		}
		name := d.Name
		if name == "" {
			name, _ = e["name"].(string)
		}
		byName[name] = append(byName[name], Source{
			Store: fullStore(p), OutputHash: NormaliseHash(h), URLs: urls,
		})
	}
	for k := range byName {
		sort.Slice(byName[k], func(i, j int) bool { return byName[k][i].Store < byName[k][j].Store })
	}

	deriverOf := func(storePath string) string {
		if nixPrefix == "" {
			return ""
		}
		out, code := proc.CaptureAllowFail(path.Join(nixPrefix, "nix-store"), "-q", "--deriver", storePath)
		out = strings.TrimSpace(out)
		if code != 0 || strings.HasPrefix(path.Base(out), "unknown") {
			return ""
		}
		return out
	}

	resolve := func(storePath string) Source {
		rec := Source{Store: storePath, URLs: []string{}}
		base := path.Base(storePath)
		name := base
		if i := strings.Index(base, "-"); i >= 0 {
			name = base[i+1:]
		}
		drv := outIndex[base]
		if drv == "" {
			drv = deriverOf(storePath)
		}
		if drv != "" {
			if d, ok := drvs[path.Base(drv)]; ok {
				e := attrsOf(d)
				rec.URLs = splitWS(e["urls"])
				if len(rec.URLs) == 0 {
					rec.URLs = splitWS(e["url"])
				}
				h := d.Outputs["out"].Hash
				if h == "" {
					h, _ = e["outputHash"].(string)
				}
				rec.OutputHash = NormaliseHash(h)
				if len(rec.URLs) > 0 {
					return rec
				}
			}
		}
		// The name index is a fallback for a path nix does not have locally,
		// and it can be ambiguous: an ambiguous fallback keeps every candidate
		// URL rather than picking one, because a wrong URL that hashes wrong is
		// caught at fetch time and a silently dropped one is not.
		for _, c := range byName[name] {
			for _, u := range c.URLs {
				if !contains(rec.URLs, u) {
					rec.URLs = append(rec.URLs, u)
				}
			}
			if rec.OutputHash == "" {
				rec.OutputHash = NormaliseHash(c.OutputHash)
			}
		}
		return rec
	}

	depNames := func(key string) []string {
		out := []string{}
		for _, p := range splitWS(env[key]) {
			out = append(out, storeName(p))
		}
		return out
	}
	depRecords := func(key string) []DepRecord {
		out := []DepRecord{}
		for _, p := range splitWS(env[key]) {
			drv := outIndex[path.Base(p)]
			if drv == "" {
				// A dependency whose derivation could not be resolved is kept
				// with an empty drv rather than dropped: something this tool
				// cannot plan is something the caller has to be told about.
				drv = deriverOf(p)
			}
			out = append(out, DepRecord{Name: storeName(p), Out: p, Drv: drv})
		}
		return out
	}

	natives := strings.Join(depNames("nativeBuildInputs"), " ")
	hooks := []string{}
	for _, h := range hookMap {
		if strings.Contains(natives, h.Hook) && !contains(hooks, h.Flag) {
			hooks = append(hooks, h.Flag)
		}
	}

	// The VALUE is kept as it was: under __structuredAttrs `env` is an object,
	// and flattening it to a string would put an escaped document where a
	// nested one belongs.
	nixOnly := map[string]any{}
	for _, k := range nixOnlyKeys {
		v, ok := env[k]
		if !ok {
			continue
		}
		if s, isStr := v.(string); isStr && (s == "" || s == "1" || s == "0") {
			continue
		}
		nixOnly[k] = v
	}

	patches := []Source{}
	for _, p := range splitWS(env["patches"]) {
		patches = append(patches, resolve(p))
	}

	outputs := splitWS(env["outputs"])
	if len(outputs) == 0 {
		outputs = []string{"out"}
	}

	pname := firstString(env["pname"], env["name"], attr)
	version, _ := env["version"].(string)

	plan := &Plan{
		Attr:                  attr,
		BuildInputs:           depNames("buildInputs"),
		BuildSystemHooks:      hooks,
		CmakeFlags:            splitWS(env["cmakeFlags"]),
		ConfigureFlags:        splitWS(env["configureFlags"]),
		Deps:                  append(depRecords("buildInputs"), depRecords("propagatedBuildInputs")...),
		Drv:                   fullStore(drvPath),
		MakeFlags:             splitWS(env["makeFlags"]),
		MesonFlags:            splitWS(env["mesonFlags"]),
		NativeBuildInputs:     depNames("nativeBuildInputs"),
		NixOnly:               nixOnly,
		Nixpkgs:               nixpkgsVersion,
		Outputs:               outputs,
		Patches:               patches,
		Pname:                 pname,
		PropagatedBuildInputs: depNames("propagatedBuildInputs"),
		Schema:                "pgb-nix-plan/1",
		System:                top.System,
		Version:               version,
	}
	if src, ok := env["src"].(string); ok && src != "" {
		s := resolve(src)
		plan.Src = &s
	}
	return plan, nil
}

// WritePlan encodes a plan the way every existing plan was written.
func WritePlan(w io.Writer, p *Plan) error {
	enc := json.NewEncoder(w)
	enc.SetIndent("", " ")
	enc.SetEscapeHTML(false)
	return enc.Encode(p)
}

// ReadPlan decodes a plan file.
func ReadPlan(r io.Reader) (*Plan, error) {
	var p Plan
	if err := json.NewDecoder(r).Decode(&p); err != nil {
		return nil, err
	}
	return &p, nil
}

// attrsOf merges structuredAttrs over env. A dependency derivation has
// structuredAttrs too, and reading only env loses the urls a modern fetchurl
// keeps there.
func attrsOf(d ShowDrv) map[string]any {
	out := map[string]any{}
	for k, v := range d.Env {
		out[k] = v
	}
	if len(d.StructuredAttrs) > 0 {
		for k, v := range d.StructuredAttrs {
			out[k] = v
		}
	}
	return out
}

// splitWS normalises a derivation attribute to a list of strings, accepting
// both the space-joined string and the JSON array.
func splitWS(v any) []string {
	switch t := v.(type) {
	case nil:
		return []string{}
	case string:
		f := strings.Fields(t)
		if f == nil {
			return []string{}
		}
		return f
	case bool:
		return []string{}
	case []any:
		out := []string{}
		for _, e := range t {
			s := scalarString(e)
			if s != "" {
				out = append(out, s)
			}
		}
		return out
	case []string:
		return t
	}
	return []string{}
}

// scalarString renders a JSON scalar as the plan has always written it.
func scalarString(v any) string {
	switch t := v.(type) {
	case nil:
		return ""
	case string:
		return t
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
	default:
		b, err := json.Marshal(t)
		if err != nil {
			return ""
		}
		return string(b)
	}
}

func firstString(vals ...any) string {
	for _, v := range vals {
		if s, ok := v.(string); ok && s != "" {
			return s
		}
	}
	return ""
}

// fullStore makes a derivation key absolute: every nix command that takes a
// derivation wants a store path, and a basename resolves against the current
// directory instead.
func fullStore(p string) string {
	if p == "" || strings.HasPrefix(p, "/") {
		return p
	}
	return "/nix/store/" + p
}

// storeName strips the hash prefix from a store path's base name.
func storeName(p string) string {
	b := path.Base(p)
	if i := strings.Index(b, "-"); i >= 0 {
		return b[i+1:]
	}
	return b
}

func contains(list []string, want string) bool {
	for _, s := range list {
		if s == want {
			return true
		}
	}
	return false
}

const nixHashAlphabet = "0123456789abcdfghijklmnpqrsvwxyz"

// NormaliseHash reduces every encoding of one hash to lowercase hex. A raw
// .drv writes a fixed-output hash as hex, `nix derivation show` writes the same
// hash as SRI, and nix-base32 turns up in narinfos; left alone, two routes
// produce plans that differ in one field and look like a disagreement about
// the source. Anything unrecognised is returned unchanged, so an unexpected
// encoding stays visible instead of becoming an empty string.
func NormaliseHash(h string) string {
	if h == "" {
		return h
	}
	v := h
	for _, pfx := range []string{"sha256:", "sha256-", "sha512:", "sha512-",
		"sha1:", "sha1-", "md5:", "md5-"} {
		if strings.HasPrefix(v, pfx) {
			v = v[len(pfx):]
			break
		}
	}
	low := strings.ToLower(v)
	if isHexOfLen(low, 32, 40, 64, 128) {
		return low
	}
	if len(v) == 52 && onlyIn(v, nixHashAlphabet) {
		n := new(big.Int)
		base := big.NewInt(32)
		for i := len(v) - 1; i >= 0; i-- {
			n.Mul(n, base)
			n.Add(n, big.NewInt(int64(strings.IndexByte(nixHashAlphabet, v[i]))))
		}
		return fmt.Sprintf("%064x", n)
	}
	pad := (4 - len(v)%4) % 4
	if raw, err := base64.StdEncoding.DecodeString(v + strings.Repeat("=", pad)); err == nil {
		switch len(raw) {
		case 16, 20, 32, 64:
			return hex.EncodeToString(raw)
		}
	}
	return h
}

func isHexOfLen(s string, lens ...int) bool {
	ok := false
	for _, n := range lens {
		if len(s) == n {
			ok = true
			break
		}
	}
	if !ok {
		return false
	}
	for i := 0; i < len(s); i++ {
		c := s[i]
		if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}

func onlyIn(s, alphabet string) bool {
	for i := 0; i < len(s); i++ {
		if strings.IndexByte(alphabet, s[i]) < 0 {
			return false
		}
	}
	return true
}
