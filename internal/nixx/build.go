// build.go — `pgb nix`: nixpkgs plans, pgb builds.
//
// The split is the design. `plan` turns an attribute into a small JSON file
// naming the source, its upstream URL and hash, the patches, the flags and the
// dependency derivations; `build` fetches those content-addressed paths from
// cache.nixos.org over plain HTTPS, signature- and hash-checked, unpacks,
// patches and builds through pgb's static-glibc toolchain. Only the first step
// can want a nix, and two nix-free routes are tried before it.
//
// What comes out is an ordinary statically linked glibc ELF: no store, no
// loader, no wrapper script.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/elfx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

// Builder holds the settings a plan, a dependency walk and a build share.
type Builder struct {
	C      *cfg.Config
	Client *Client

	Prefix    string // the shared static install prefix
	DepDepth  int
	MaxRounds int
	DepSkip   []string

	ConfigureExtra []string
	WithDeps       bool
	ForceEval      bool
	NoHydra        bool
	System         string
}

// depSkipDefault names what will not be built, each for a reason. A skipped
// dependency is reported, and the adaptation loop then drops the flag that
// wanted it, which is a different outcome from pretending it is present.
//
//	glibc, gcc, binutils   the pinned build environment already is these
//	systemd, dbus, udev    a static binary linking systemd links a dlopen host
//	bison, flex, pkg-config, hooks   build-time tools already in the environment
//	doctest, gtest, ...    test-only, needed by a flag pgb does not pass
var depSkipDefault = strings.Fields(`glibc gcc binutils systemd systemd-minimal
systemd-minimal-libs dbus udev bison flex pkg-config perl python3 hook stdenv
bash coreutils doctest rapidcheck gtest gbenchmark catch2`)

// NewBuilder applies the defaults and the environment overrides.
func NewBuilder(c *cfg.Config) *Builder {
	b := &Builder{
		C:         c,
		Client:    NewClient(),
		Prefix:    envOr("NIX_PREFIX", filepath.Join(c.State, "nix-prefix")),
		DepDepth:  envInt("NIX_DEP_DEPTH", 2),
		MaxRounds: envInt("NIX_MAX_ROUNDS", 8),
		DepSkip:   depSkipDefault,
		WithDeps:  true,
		System:    envOr("PGB_NIX_SYSTEM", "x86_64-linux"),
	}
	if v, ok := os.LookupEnv("NIX_DEP_SKIP"); ok {
		b.DepSkip = strings.Fields(v)
	}
	b.Client.System = b.System
	b.ForceEval = logx.EnvBool("PGB_NIX_FORCE_EVAL", false)
	b.NoHydra = logx.EnvBool("PGB_NIX_NO_HYDRA", false)
	return b
}

func envInt(name string, def int) int {
	if v := os.Getenv(name); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return n
		}
	}
	return def
}

// ---------------------------------------------------------------------------
// plan
// ---------------------------------------------------------------------------

// NixBin finds nix, probing the profile paths as well as PATH: a non-login
// shell on a machine where nix was just installed has the binary on disk and
// not on PATH.
func NixBin() (string, bool) {
	home, _ := os.UserHomeDir()
	for _, c := range []string{
		"nix",
		"/nix/var/nix/profiles/default/bin/nix",
		filepath.Join(home, ".nix-profile/bin/nix"),
	} {
		if p, err := proc.LookPath(c); err == nil {
			return p, true
		}
		if fi, err := os.Stat(c); err == nil && fi.Mode()&0o111 != 0 {
			return c, true
		}
	}
	return "", false
}

// NixPrefix is the directory nix's own tools live in.
func NixPrefix() (string, bool) {
	n, ok := NixBin()
	if !ok {
		return "", false
	}
	return filepath.Dir(n), true
}

// Plan resolves an attribute to a build plan, writing it to out when out is
// not empty.
//
// The nix-free routes are tried first because they are the ones that work on a
// host with no root, no docker and no nix. hydra comes before the channel
// index because it answers for every job it built rather than for the fraction
// of paths whose narinfo carries a Deriver.
func (b *Builder) Plan(attr, out string) (*Plan, error) {
	if !b.ForceEval && out != "" {
		if !b.NoHydra {
			if p, err := b.planHydra(attr, out); err == nil {
				logx.Say("plan: %s  (no nix was used -- hydra route)", out)
				return p, nil
			} else {
				log.Debugf("hydra route did not resolve %s: %v", attr, err)
			}
		}
		if p, err := b.planChannel(attr, out); err == nil {
			logx.Say("plan: %s  (no nix was used -- channel index route)", out)
			return p, nil
		} else {
			log.Debugf("channel route did not resolve %s: %v", attr, err)
		}
		logx.Warnf("no nix-free route resolved %q; falling back to evaluation", attr)
	}
	return b.planEval(attr, out)
}

// planChannel resolves a name through the channel index and the narinfo's own
// Deriver field.
func (b *Builder) planChannel(query, out string) (*Plan, error) {
	storePath := ""
	switch {
	case strings.HasPrefix(query, "/nix/store/"), storePathLike(query):
		storePath = query
	default:
		// Anchored at both ends: `resolve bash` unanchored also matches
		// bash-completion, bashdb and bash-5.3p15-doc, and the first line of
		// that is whichever sorted first.
		for _, pat := range []string{
			`/nix/store/[a-z0-9]{32}-` + regexp.QuoteMeta(query) + `-[0-9][^/]*$`,
			`/nix/store/[a-z0-9]{32}-` + regexp.QuoteMeta(query) + `$`,
		} {
			hits, err := b.Client.Resolve(pat, 5)
			if err == nil && len(hits) > 0 {
				storePath = hits[0]
				break
			}
		}
	}
	if storePath == "" {
		return nil, fail.Ran("no store path in the channel index matches %q", query)
	}
	logx.Say("resolved    %s  (channel index, no nix)", storePath)

	info, err := b.Client.Narinfo(storePath)
	if err != nil {
		return nil, err
	}
	deriver := info["Deriver"]
	if deriver == "" {
		return nil, fail.Ran("the narinfo for %s names no Deriver", storePath)
	}
	if !strings.HasPrefix(deriver, "/nix/store/") {
		deriver = "/nix/store/" + deriver
	}
	logx.Say("deriver     %s  (from the signed narinfo)", deriver)
	return b.planFromDrv(deriver, query, out)
}

// planHydra resolves an attribute through hydra's index of builds.
func (b *Builder) planHydra(query, out string) (*Plan, error) {
	// A .drv path already names the derivation and needs no index at all.
	if strings.HasPrefix(query, "/nix/store/") {
		if strings.HasSuffix(query, ".drv") {
			return b.planFromDrv(query, query, out)
		}
		return nil, fail.Ran("a store path that is not a .drv has no hydra job")
	}

	attr := query
	if m, err := b.Client.Attr(query); err == nil {
		if m.Attr != query {
			// A match that was not an exact attribute is said out loud: `sed`
			// resolves through pname to `freebsd.sed`, a real package for the
			// wrong userland that reads exactly like a correct answer.
			logx.Warnf("%s -> attribute %s (matched by %s)", query, m.Attr, m.MatchedBy)
		}
		attr = m.Attr
	}

	jobFile, err := b.Client.HydraJob(attr)
	if err != nil {
		return nil, err
	}
	raw, err := os.ReadFile(jobFile)
	if err != nil {
		return nil, err
	}
	var reply HydraReply
	if err := json.Unmarshal(raw, &reply); err != nil {
		return nil, fail.Ran("hydra: not JSON: %v", err)
	}
	if reply.DrvPath == "" {
		return nil, fail.Ran("hydra names no drvpath for %s", attr)
	}
	if reply.System != b.System {
		return nil, fail.Ran("hydra reply is for %s, wanted %s", reply.System, b.System)
	}
	logx.Say("resolved    %s  (hydra %s, no nix)", reply.DrvPath, reply.Job)

	// hydra answers for its latest FINISHED eval, which is not necessarily the
	// revision the channel pinned. That is a reason to check it, not to skip
	// the route: every output hydra names is looked for in the channel index.
	revision := ""
	if latest := latestEval(reply); latest > 0 {
		revision = b.Client.HydraEvalRevision(latest)
	}
	var outs []string
	for _, v := range reply.BuildOutputs {
		if m, ok := v.(map[string]any); ok {
			if p, ok := m["path"].(string); ok && p != "" {
				outs = append(outs, p)
			}
		}
	}
	present, missing, err := b.Client.IndexHas(outs)
	agrees := "no"
	if err == nil && present > 0 && missing == 0 {
		agrees = "yes"
	}
	logx.Say("revision    %s  channel pin agrees: %s", revision, agrees)

	return b.planFromDrv(reply.DrvPath, query, out)
}

func latestEval(r HydraReply) int {
	best := 0
	for _, e := range r.JobsetEvals {
		if f, ok := e.(float64); ok && int(f) > best {
			best = int(f)
		}
	}
	return best
}

// planFromDrv is the shared tail of every nix-free route: a .drv store path,
// over HTTPS, into a build plan. A fix to the plan pipeline therefore cannot
// land in one route and miss the other.
//
// Depth 1 is enough: the source, the patches and the buildInputs are all
// DIRECT inputs, and their own inputs are only needed when the dependency walk
// plans them, which it does one at a time.
func (b *Builder) planFromDrv(drvPath, query, out string) (*Plan, error) {
	dir := filepath.Join(b.C.State, "drv")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return nil, err
	}
	if _, err := b.Client.Fetch(drvPath, FetchOptions{Out: dir}); err != nil {
		return nil, fail.Ran("could not fetch %s: %v", drvPath, err)
	}
	self := filepath.Join(dir, strings.TrimPrefix(drvPath, "/nix/store/"))

	info, err := b.Client.Narinfo(drvPath)
	if err != nil {
		return nil, err
	}
	fetched := 1
	for _, ref := range strings.Fields(info["References"]) {
		if !strings.HasSuffix(ref, ".drv") {
			continue
		}
		p := ref
		if !strings.HasPrefix(p, "/nix/store/") {
			p = "/nix/store/" + p
		}
		if _, err := b.Client.Fetch(p, FetchOptions{Out: dir}); err == nil {
			fetched++
		}
	}
	logx.Say("derivations %d fetched and verified over HTTPS", fetched)

	drvFiles, err := filepath.Glob(filepath.Join(dir, "*.drv"))
	if err != nil {
		return nil, err
	}
	doc, err := showDocument(append([]string{self}, drvFiles...))
	if err != nil {
		return nil, err
	}
	plan, err := BuildPlan(doc, query, filepath.Base(self), "", "")
	if err != nil {
		return nil, err
	}
	if out != "" {
		if err := writePlanFile(out, plan); err != nil {
			return nil, err
		}
	}
	return plan, nil
}

// showDocument reads .drv files into the `nix derivation show` shape the
// planner consumes.
func showDocument(paths []string) (ShowRecursive, error) {
	seen := map[string]bool{}
	doc := ShowRecursive{Derivations: map[string]ShowDrv{}}
	for _, p := range paths {
		if seen[p] {
			continue
		}
		seen[p] = true
		d, err := ParseDrvFile(p)
		if err != nil {
			continue
		}
		key, entry := d.Show()
		env := map[string]any{}
		for k, v := range entry.Env {
			env[k] = v
		}
		sa := map[string]any{}
		for k, v := range entry.StructuredAttrs {
			var decoded any
			if err := json.Unmarshal(v, &decoded); err == nil {
				sa[k] = decoded
			}
		}
		doc.Derivations[key] = ShowDrv{
			Name:            entry.Name,
			Outputs:         entry.Outputs,
			System:          entry.System,
			Env:             env,
			StructuredAttrs: sa,
		}
	}
	if len(doc.Derivations) == 0 {
		return doc, fail.Ran("no derivation could be read")
	}
	return doc, nil
}

// planEval is the fallback: evaluate the attribute with a real nix.
func (b *Builder) planEval(attr, out string) (*Plan, error) {
	pfx, ok := NixPrefix()
	if !ok {
		return nil, fail.Cannot("pgb nix plan needs nix, or a name the channel " +
			"index knows. Install it, or use --plan with a plan another machine made.")
	}
	// A multi-output attribute prints `<drv>!bin`, not `<drv>`: jq, sqlite and
	// curl all do. Which output the caller wanted does not change the plan,
	// because the plan is the inputs.
	outText, code := proc.CaptureAllowFail(filepath.Join(pfx, "nix-instantiate"),
		"<nixpkgs>", "--attr", attr)
	drv := ""
	for _, line := range strings.Split(outText, "\n") {
		if strings.HasPrefix(line, "/nix/store/") {
			drv, _, _ = strings.Cut(line, "!")
			break
		}
	}
	if code != 0 || !strings.HasSuffix(drv, ".drv") {
		return nil, fail.Ran("nixpkgs has no attribute %q, or it does not evaluate", attr)
	}

	// --recursive so the fetchurl derivations that produce the source and the
	// patches are in the same document: their urls and outputHash are what make
	// a plan usable without nix or without the binary cache.
	showJSON, code := proc.CaptureAllowFail(filepath.Join(pfx, "nix"),
		"--extra-experimental-features", "nix-command",
		"derivation", "show", drv, "--recursive")
	if code != 0 || showJSON == "" {
		return nil, fail.Ran("could not turn %s into a plan", drv)
	}
	var doc ShowRecursive
	if err := json.Unmarshal([]byte(showJSON), &doc); err != nil {
		return nil, fail.Ran("the derivation document is not JSON: %v", err)
	}
	version := ""
	if v, code := proc.CaptureAllowFail(filepath.Join(pfx, "nix-instantiate"),
		"--eval", "--expr", "(import <nixpkgs> {}).lib.version"); code == 0 {
		version = strings.Trim(strings.TrimSpace(v), `"`)
	}
	plan, err := BuildPlan(doc, attr, drv, version, pfx)
	if err != nil {
		return nil, fail.Ran("%v", err)
	}
	if out == "" {
		return plan, WritePlan(os.Stdout, plan)
	}
	if err := writePlanFile(out, plan); err != nil {
		return nil, err
	}
	logx.Say("plan: %s", out)
	return plan, nil
}

func writePlanFile(path string, p *Plan) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	f, err := os.Create(path + ".part")
	if err != nil {
		return err
	}
	if err := WritePlan(f, p); err != nil {
		f.Close()
		os.Remove(path + ".part")
		return err
	}
	if err := f.Close(); err != nil {
		return err
	}
	return os.Rename(path+".part", path)
}

func storePathLike(s string) bool {
	if len(s) < 34 {
		return false
	}
	i := strings.IndexByte(s, '-')
	return i == 32
}

// ResolvePlan returns a plan file path, making one if needed.
func (b *Builder) ResolvePlan(attr, planFile string) (string, *Plan, error) {
	if planFile != "" {
		f, err := os.Open(planFile)
		if err != nil {
			return "", nil, fail.Cannot("no such plan: %s", planFile)
		}
		defer f.Close()
		p, err := ReadPlan(f)
		if err != nil {
			return "", nil, fail.Cannot("%s: %v", planFile, err)
		}
		return planFile, p, nil
	}
	if attr == "" {
		return "", nil, fail.Cannot("give an attribute or --plan FILE")
	}
	pf := filepath.Join(b.C.State, "plans", attr+".json")
	if fi, err := os.Stat(pf); err == nil && fi.Size() > 0 {
		f, err := os.Open(pf)
		if err == nil {
			defer f.Close()
			if p, err := ReadPlan(f); err == nil {
				return pf, p, nil
			}
		}
	}
	p, err := b.Plan(attr, pf)
	if err != nil {
		return "", nil, err
	}
	return pf, p, nil
}

// ---------------------------------------------------------------------------
// fetch
// ---------------------------------------------------------------------------

// FetchPlan fetches a plan's source and patches into dest, returning the
// source path and the patch paths in plan order — patches applied out of order
// fail in ways that read like a corrupt source tree.
func (b *Builder) FetchPlan(p *Plan, dest string) (string, []string, error) {
	if err := os.MkdirAll(dest, 0o755); err != nil {
		return "", nil, err
	}
	if p.Src == nil || p.Src.Store == "" {
		return "", nil, fail.Ran("the plan has no source")
	}
	src, err := b.fetchOne(*p.Src, dest)
	if err != nil {
		return "", nil, fail.Ran("could not fetch the source: %s", p.Src.Store)
	}
	var patches []string
	for i, patch := range p.Patches {
		got, err := b.fetchOne(patch, dest)
		if err != nil {
			return "", nil, fail.Ran("could not fetch patch %d: %s", i, patch.Store)
		}
		patches = append(patches, got)
	}
	return src, patches, nil
}

// fetchOne tries the binary cache first — content-addressed, signed and
// NarHash-checked — and the upstream URL second. The second route is not a
// convenience: it is the one that still works when a path was never uploaded
// or has been garbage-collected. Both verify.
func (b *Builder) fetchOne(s Source, dest string) (string, error) {
	base := strings.TrimPrefix(s.Store, "/nix/store/")
	target := filepath.Join(dest, base)
	if _, err := os.Lstat(target); err == nil {
		return target, nil
	}
	if _, err := b.Client.Fetch(s.Store, FetchOptions{Out: dest}); err == nil {
		return target, nil
	} else {
		log.Debugf("cache route failed for %s: %v", s.Store, err)
	}
	for _, u := range s.URLs {
		if err := downloadFile(u, target+".part"); err != nil {
			log.Debugf("%s: %v", u, err)
			continue
		}
		if ok, err := checkFileHash(target+".part", s.OutputHash); err != nil || !ok {
			logx.Warnf("hash mismatch from %s", u)
			os.Remove(target + ".part")
			continue
		}
		return target, os.Rename(target+".part", target)
	}
	os.Remove(target + ".part")
	return "", fail.Ran("no route fetched %s", s.Store)
}

// ---------------------------------------------------------------------------
// dependencies
// ---------------------------------------------------------------------------

// depSkipped reports whether a name is in the skip list, matching the name or
// a versioned form of it.
func (b *Builder) depSkipped(name string) bool {
	for _, s := range b.DepSkip {
		if name == s || strings.HasPrefix(name, s+"-") {
			return true
		}
	}
	return false
}

// shortDepName strips the version and the -dev suffix.
var depVersionRe = regexp.MustCompile(`-[0-9].*$`)

func shortDepName(name string) string {
	s := depVersionRe.ReplaceAllString(name, "")
	return strings.TrimSuffix(s, "-dev")
}

// depKey keys the per-dependency directories on the prefix, so two runs with
// different prefixes cannot rebuild each other's trees underneath them.
func (b *Builder) depKey() string {
	return fmt.Sprintf("%08x", crc32String(b.Prefix))
}

// BuildDeps walks the plan's dependency graph, building each into the shared
// static prefix depth first: a dependency's own dependencies go in before it
// does, or it configures against a prefix that does not have them yet.
//
// The walk stops at DepDepth, so a dependency whose own inputs live below that
// cut can only build once a sibling has put them in the prefix. Passes are
// therefore repeated while one is still making progress: a cold prefix
// converges within a single invocation instead of needing the caller to run
// the command again.
func (b *Builder) BuildDeps(p *Plan, depth int) error {
	if depth > b.DepDepth {
		return nil
	}
	for _, d := range []string{"lib", "include", ".built"} {
		if err := os.MkdirAll(filepath.Join(b.Prefix, d), 0o755); err != nil {
			return err
		}
	}
	for pass := 0; ; pass++ {
		built, missing := b.depPass(p, depth, pass)
		if missing == 0 || built == 0 || depth > 1 {
			return nil
		}
		logx.Say("dep retry   %d still missing, %d landed this pass", missing, built)
	}
}

// depPass builds every dependency it can, and reports how many landed and how
// many are still absent from the prefix.
func (b *Builder) depPass(p *Plan, depth, pass int) (built, missing int) {
	for _, dep := range p.Deps {
		if dep.Name == "" {
			continue
		}
		short := shortDepName(dep.Name)
		if b.depSkipped(short) {
			if pass == 0 {
				logx.Say("dep skip    %s (in the skip list)", dep.Name)
			}
			continue
		}
		if _, err := os.Stat(filepath.Join(b.Prefix, ".built", short)); err == nil {
			if pass == 0 {
				logx.Say("dep have    %s", short)
			}
			continue
		}
		if dep.Drv == "" {
			logx.Warnf("dep %s has no derivation in the plan; it cannot be built", dep.Name)
			continue
		}
		logx.Say("dep build   %s  (depth %d)", short, depth)
		if err := b.buildDep(dep.Drv, short, depth); err != nil {
			logx.Warnf("dep FAILED  %s -- %v", short, err)
			missing++
			continue
		}
		built++
	}
	return built, missing
}

func (b *Builder) buildDep(drv, short string, depth int) error {
	key := b.depKey()
	planPath := filepath.Join(b.C.State, "plans", key, "dep-"+short+".json")
	if err := os.MkdirAll(filepath.Dir(planPath), 0o755); err != nil {
		return err
	}
	var plan *Plan
	if fi, err := os.Stat(planPath); err == nil && fi.Size() > 0 {
		f, err := os.Open(planPath)
		if err == nil {
			plan, _ = ReadPlan(f)
			f.Close()
		}
	}
	if plan == nil {
		// The dependency's own .drv is already in the parent's plan, so
		// planning it needs no evaluation and therefore no nix.
		p, err := b.planFromDrv(drv, short, planPath)
		if err != nil {
			// The evaluated route stays as the fallback for a .drv the cache
			// does not have: anything built locally, or garbage-collected.
			pfx, ok := NixPrefix()
			if !ok {
				return fmt.Errorf("could not plan %s from %s without nix, and there is no nix", short, drv)
			}
			showJSON, code := proc.CaptureAllowFail(filepath.Join(pfx, "nix"),
				"--extra-experimental-features", "nix-command",
				"derivation", "show", drv, "--recursive")
			if code != 0 {
				return fmt.Errorf("could not plan %s from %s", short, drv)
			}
			var doc ShowRecursive
			if err := json.Unmarshal([]byte(showJSON), &doc); err != nil {
				return fmt.Errorf("could not plan %s: %v", short, err)
			}
			p, err = BuildPlan(doc, short, drv, "", pfx)
			if err != nil {
				return fmt.Errorf("could not plan %s: %v", short, err)
			}
			if err := writePlanFile(planPath, p); err != nil {
				return err
			}
		}
		plan = p
	}

	if err := b.BuildDeps(plan, depth+1); err != nil {
		return err
	}

	work := filepath.Join(b.C.State, "nix-deps", key, short)
	if err := os.MkdirAll(filepath.Join(work, "dl"), 0o755); err != nil {
		return err
	}
	src, patches, err := b.FetchPlan(plan, filepath.Join(work, "dl"))
	if err != nil {
		return fmt.Errorf("its source could not be fetched")
	}
	top, err := b.PrepareSource(plan, src, patches, work)
	if err != nil {
		return fmt.Errorf("its source could not be unpacked")
	}
	if err := b.BuildTree(top, plan, work, true); err != nil {
		return fmt.Errorf("the parent will have to do without it")
	}
	if err := os.WriteFile(filepath.Join(b.Prefix, ".built", short), nil, 0o644); err != nil {
		return err
	}
	logx.Say("dep ok      %s -> %s", short, b.Prefix)
	return nil
}

// ---------------------------------------------------------------------------
// source preparation
// ---------------------------------------------------------------------------

// PrepareSource unpacks a source into the work directory and applies the
// plan's patches, returning the top-level directory.
func (b *Builder) PrepareSource(p *Plan, src string, patches []string, work string) (string, error) {
	build := filepath.Join(work, "build")
	if err := os.RemoveAll(build); err != nil {
		return "", err
	}
	if err := os.MkdirAll(build, 0o755); err != nil {
		return "", err
	}

	fi, err := os.Stat(src)
	switch {
	case err == nil && fi.IsDir():
		// Copied, not used in place: a store path is read-only, and patches
		// and a build tree both need to write.
		if r, err := proc.Run("cp", "-a", src, filepath.Join(build, filepath.Base(src))); err != nil || r.Failed() {
			return "", fail.Ran("could not copy %s", src)
		}
		if r, err := proc.Run("chmod", "-R", "u+w", build); err != nil || r.Failed() {
			return "", fail.Ran("could not make %s writable", build)
		}
	case isZip(src):
		if r, err := (&proc.Cmd{Argv: []string{"unzip", "-q", src}, Dir: build}).Run(); err != nil || r.Failed() {
			return "", fail.Ran("could not unpack %s", src)
		}
	default:
		if r, err := proc.Run("tar", "-xf", src, "-C", build); err != nil || r.Failed() {
			return "", fail.Ran("could not unpack %s", src)
		}
	}

	entries, err := os.ReadDir(build)
	if err != nil {
		return "", err
	}
	top := ""
	for _, e := range entries {
		if e.IsDir() {
			top = filepath.Join(build, e.Name())
			break
		}
	}
	if top == "" {
		return "", fail.Ran("the archive unpacked to no directory")
	}

	// nixpkgs' own patchFlags when it set any. bash is the case that proves it
	// matters: its upstream patches are -p0 and the default -p1 fails on all.
	flags := []string{"-p1"}
	if v, ok := p.NixOnly["patchFlags"]; ok {
		if f := splitWS(v); len(f) > 0 {
			flags = f
		}
	}
	applied := 0
	for _, patch := range patches {
		argv := append([]string{"patch"}, flags...)
		argv = append(argv, "-s", "-f", "-i", patch)
		r, err := (&proc.Cmd{Argv: argv, Dir: top, Subsys: "nix"}).Output()
		if err == nil && !r.Failed() {
			applied++
			continue
		}
		logx.Warnf("patch did not apply: %s", filepath.Base(patch))
	}
	logx.Say("patches     %d applied (%s)", applied, strings.Join(flags, " "))
	return top, nil
}

func isZip(p string) bool { return strings.HasSuffix(p, ".zip") }

// ---------------------------------------------------------------------------
// collect
// ---------------------------------------------------------------------------

// Collect copies the ELF executables out of a build tree, by what the file is
// rather than by where it sits or what it is called: make leaves libtool
// wrapper scripts named exactly like the program beside the real binary.
func Collect(tree, dest string) (int, error) {
	if err := os.MkdirAll(dest, 0o755); err != nil {
		return 0, err
	}
	n := 0
	err := filepath.Walk(tree, func(p string, fi os.FileInfo, err error) error {
		if err != nil || !fi.Mode().IsRegular() || fi.Mode()&0o100 == 0 {
			return nil
		}
		if !elfx.IsELF(p) {
			return nil
		}
		data, err := os.ReadFile(p)
		if err != nil {
			return nil
		}
		if err := os.WriteFile(filepath.Join(dest, filepath.Base(p)), data, 0o755); err == nil {
			n++
		}
		return nil
	})
	return n, err
}

// crc32String is a small stable hash for directory keys.
func crc32String(s string) uint32 {
	var crc uint32 = 0xffffffff
	for i := 0; i < len(s); i++ {
		crc ^= uint32(s[i])
		for j := 0; j < 8; j++ {
			if crc&1 != 0 {
				crc = (crc >> 1) ^ 0xedb88320
			} else {
				crc >>= 1
			}
		}
	}
	return ^crc
}
