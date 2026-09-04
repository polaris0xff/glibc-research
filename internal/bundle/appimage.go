// appimage.go — a nixpkgs closure, packed the Anylinux way.
//
// Three things differ from `nix bundle --bundler nix-appimage`, and each is a
// mechanism this file adopts:
//
//  1. uruntime + dwarfs instead of appimage-type2-runtime + mksquashfs;
//  2. sharun — run the bundled loader with --library-path — instead of a bwrap
//     AppRun that bind-mounts /nix/store. Item 2 is the one that matters: bwrap
//     needs unprivileged user namespaces, which cannot be relied on; sharun
//     needs no namespace at all;
//  3. shared/{bin,lib} instead of the store layout verbatim.
//
// And one thing neither upstream does: sharun's own library discovery walks
// ldd and then straces the program to catch dlopen'd libraries. A nixpkgs
// closure is not a heuristic — it is the exact set the derivation declared,
// including the dlopen'd ones — so the closure REPLACES the discovery step.
//
// What this does not do: it does not make the application static, and it is
// not trying to. This is a bundle, built because the GUI case needs one.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"time"

	assets "github.com/polaris0xff/glibc-research"
	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/elfx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/nixx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

// Pinned, and the pin is the point: `latest/download` moves under you, so two
// runs a week apart produce AppImages with different runtimes and nothing in
// either says so.
//
// ⭐ THE RUNTIME IS THE COLD-START COST AND `lite` IS 24% OF IT. experiments/77-
// packed one AppDir five ways, one component at a time, median of 21: only
// `full` -> `lite` moves the clock (0.76x, -1,552,384 B, exactly the two
// headers' difference). Neither the uruntime version bump nor the dwarfs
// upgrade is distinguishable from 1.00x. dwarfs 0.15.6 is taken anyway for
// COMPARABILITY -- it is what experiments/86- stages for the competitor arm,
// so the head-to-head now differs in the closure and nothing else.
//
// ⭐ `lite` DROPS TOOLS, NOT CODECS: it embeds `dwarfs-fuse-extract-*` where
// full embeds `dwarfs-universal-*` and gates out `dwarfsck`/`mkdwarfs`
// (`src/main.rs:98-131`). The same AppDir runs under both headers at
// `-C zstd:level=19`, `-C lzma:level=6` and `-C null`, six for six.
//
// ⛔ AND THE MODE SELECTOR IS PATCHABLE. `URUNTIME_EXTRACT=3` and
// `URUNTIME_MOUNT=3` are compile-time constants laid out as ASCII, so
// `strings -a` finds them in the artefact. Mode 3 is "mount below 350 MB,
// EXTRACT above" -- so a jq bundle mounts and a kdenlive bundle does not, and
// a lever measured on one does not transfer to the other. Nothing here sets
// them yet; docs/research/nix-bundle-patching.md §1 names the probe.
const (
	defaultURuntimeURL = "https://github.com/VHSgunzo/uruntime/releases/download/v0.5.9/uruntime-appimage-dwarfs-lite-%s"
	defaultSharunURL   = "https://github.com/pkgforge-dev/Anylinux-sharun/releases/latest/download/sharun-%s"
	defaultDwarfsURL   = "https://github.com/mhx/dwarfs/releases/download/v0.15.6/dwarfs-universal-0.15.6-Linux-%s"
)

// AppImageOptions describes one bundle.
type AppImageOptions struct {
	Target       string   // a nixpkgs attribute or a store path
	Out          string   // the AppImage to write
	Name         string   // the program to run; empty means derived
	Debloat      string   // none | safe | aggressive
	KeepLocales  []string //
	WithPrograms []string // extra programs, found anywhere in the closure
	Extra        []string // extra attributes or store paths to pull in
	NoGL         bool
	// NoStorefix builds without the compiled-in-store-path mechanism. ⭐ It
	// is the NEGATIVE CONTROL for T-081 and it is a shipped flag rather than
	// a comment: a criterion that cannot be made to fail is not an
	// instrument. storefix.go, and PROGRESS.md delivery rule 6.
	NoStorefix bool
	// NoPluginEnv builds without the variables that point a plugin host at
	// the bundled plugin tree — GStreamer's four, MLT's, frei0r, LADSPA, babl,
	// GEGL. ⭐ It is the NEGATIVE CONTROL for T-091, and it is a shipped flag
	// for the same reason `--no-storefix` is: a criterion that cannot be made
	// to fail is not an instrument. sharun.go `bakedOverride`.
	NoPluginEnv bool
	Cache       string
}

// Builder assembles one AppImage.
type Builder struct {
	C   *cfg.Config
	Nix *nixx.Client
	O   AppImageOptions

	Arch    string
	Work    string
	Root    string // the unpacked closure
	AppDir  string
	Prog    string
	Base    string // the entry store path's base name
	wrapEnv []WrapRecord

	// farmNames memoises farmDirName: store base -> the directory name the
	// bundle answers that store path under. Computed once from the closure, so
	// writeEnv and storefix cannot disagree about it. T-092.
	farmNames map[string]string

	// Host is what this bundle may take from the machine it runs on.
	// docs/design/host-fallback.md; the zero value is bundled-first.
	Host HostPolicy
}

// NewAppImageBuilder applies the defaults.
func NewAppImageBuilder(c *cfg.Config, o AppImageOptions) *Builder {
	if o.Debloat == "" {
		o.Debloat = envOr("PGB_APPIMAGE_DEBLOAT", "safe")
	}
	if o.Cache == "" {
		o.Cache = envOr("PGB_APPIMAGE_CACHE", "/var/tmp/pgb-appimage")
	}
	return &Builder{C: c, Nix: nixx.NewClient(), O: o, Arch: machine(),
		Host: LoadHostPolicy()}
}

func envOr(name, def string) string {
	if v := os.Getenv(name); v != "" {
		return v
	}
	return def
}

func machine() string {
	if out, err := exec.Command("uname", "-m").Output(); err == nil {
		return strings.TrimSpace(string(out))
	}
	switch runtime.GOARCH {
	case "amd64":
		return "x86_64"
	case "arm64":
		return "aarch64"
	}
	return runtime.GOARCH
}

// storePathRe matches a store path: a 32-character hash and a name, not
// "anything with a dash" — an attribute like `mesa-demos` is not one.
var storePathRe = regexp.MustCompile(`^(?:/nix/store/)?[a-z0-9]{32}-.+$`)

// Build produces the AppImage.
func (b *Builder) Build() error {
	if b.O.Target == "" {
		return fail.Cannot("give a nixpkgs attribute or a store path")
	}
	// ⛔ THE OUTPUT DIRECTORY IS CHECKED FIRST, BECAUSE THE FAILURE ARRIVES
	// LAST. `--out /var/tmp/t066/jq.AppImage` with no `/var/tmp/t066` fetched
	// the whole closure, assembled the AppDir, debloated it and swept it, and
	// only then handed mkdwarfs a path it could not open:
	//
	//     E cannot open output file '"/var/tmp/t066/jq.AppImage"': No such file
	//     0 dirs, 0/0 soft/hard links, 0/0 files, 0 other
	//
	// ⚠ And the message a reader sees is `mkdwarfs failed` over a log saying
	// it wrote 0 files, which reads like the AppDir was empty. Minutes of work
	// thrown away for a missing directory nothing had looked at.
	if b.O.Out != "" {
		if dir := filepath.Dir(b.O.Out); dir != "" && dir != "." {
			if err := os.MkdirAll(dir, 0o755); err != nil {
				return fail.Cannot("cannot create the output directory %s: %v", dir, err)
			}
		}
	}
	outPath, err := b.resolveTarget()
	if err != nil {
		return err
	}
	b.Base = strings.TrimPrefix(outPath, "/nix/store/")
	b.Prog = b.O.Name
	if b.Prog == "" {
		b.Prog = deriveProgramName(b.Base)
	}
	logx.Say("attribute   %s", b.O.Target)
	logx.Say("store path  /nix/store/%s", b.Base)
	logx.Say("program     %s", b.Prog)

	b.Work = filepath.Join(b.O.Cache, b.Prog)
	b.Root = filepath.Join(b.Work, "store")
	b.AppDir = filepath.Join(b.Work, "AppDir")
	if err := os.MkdirAll(filepath.Join(b.O.Cache, "tools"), 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(b.Root, 0o755); err != nil {
		return err
	}

	logx.Say("fetching the closure (signature and NarHash checked, no nix involved)")
	if _, err := b.Nix.Fetch("/nix/store/"+b.Base, nixx.FetchOptions{Out: b.Root, WithClosure: true}); err != nil {
		return fail.Ran("could not fetch the closure: %v", err)
	}
	logx.Say("closure     %d store paths", countEntries(b.Root))

	if err := b.augment(); err != nil {
		return err
	}
	if err := b.assemble(); err != nil {
		return err
	}
	if err := b.desktopAndIcon(); err != nil {
		return err
	}
	if b.O.Debloat != "none" {
		b.debloat()
	}
	if err := b.installSharun(); err != nil {
		return err
	}
	if err := b.writeEnv(); err != nil {
		return err
	}
	// ⛔ AFTER writeEnv, because carryBakedPaths has by then materialised the
	// subtrees an override variable redirects, and those real directories must
	// win over a symlink into the merged tree. BEFORE the sweep, so the
	// interposer is present when reachability is computed. storefix.go.
	if err := b.storefix(); err != nil {
		return err
	}
	// ⛔ AFTER writeEnv, because the sweep reads `.env` to find the plugin
	// directories nothing links against -- and BEFORE integrity, so the
	// control runs on what is actually shipped. See DropUnreachable.
	//
	// ⛔ AND AT `aggressive` ONLY, WHICH IS A DOWNGRADE MADE ON EVIDENCE.
	// It ran at `safe` for one afternoon and three separate classes of
	// runtime-loaded library turned out to be invisible to it: MLT's modules,
	// libglvnd's vendor libraries, and a libSDL3.so.0 dlopen'd by name from
	// inside an MLT module with no data file naming it anywhere. Each was
	// fixed, and finding three in one day is the measurement -- the sweep's
	// model is a good approximation of reachability and it is not a proof, so
	// `safe` must not mean it. What `safe` means is rules that cannot be wrong
	// about a name; `aggressive` means this.
	if b.O.Debloat == "aggressive" {
		b.DropUnreachable()
	}
	b.integrity()
	// ⛔ THE THIRD INTEGRITY CHECK, AND IT IS THE ONE THAT DID NOT EXIST.
	// integrity() walks DT_NEEDED and manifestIntegrity() reads ICD manifests;
	// NEITHER READS A `.env` VALUE BACK AGAINST THE TREE, which is how
	// `${SHARUN_DIR}` expanded to nothing for a whole session and how a
	// short-name collision could point every override in a package at a
	// directory that was never created. T-092, corrections.md C28.
	b.envIntegrity()
	// ⛔ AFTER the sweep, for the same reason integrity() is: the manifests
	// have to be checked against what actually ships, not against what the
	// AppDir held before anything was deleted. It reads the half of the
	// bundle that is DATA, which is where all four of T-071's failures were.
	b.manifestIntegrity()
	return b.pack()
}

// resolveTarget turns an attribute into an output store path, preferring the
// nix-free route: hydra names the derivation and every output's store path,
// and the package index names which output nixpkgs installs by default.
func (b *Builder) resolveTarget() (string, error) {
	if storePathRe.MatchString(b.O.Target) {
		p := b.O.Target
		if !strings.HasPrefix(p, "/nix/store/") {
			p = "/nix/store/" + p
		}
		return p, nil
	}
	jobFile, err := b.Nix.HydraJob(b.hydraAttr())
	if err == nil {
		raw, rerr := os.ReadFile(jobFile)
		if rerr == nil {
			var reply nixx.HydraReply
			if json.Unmarshal(raw, &reply) == nil {
				want := []string{"out"}
				if m, aerr := b.Nix.Attr(b.O.Target); aerr == nil && m.OutputName != "" {
					want = append([]string{m.OutputName, "bin"}, want...)
				} else {
					want = append([]string{"bin"}, want...)
				}
				for _, name := range want {
					if v, ok := reply.BuildOutputs[name].(map[string]any); ok {
						if p, ok := v["path"].(string); ok && p != "" {
							logx.Say("resolved    %s -> %s  (hydra, no nix)", b.O.Target, p)
							return p, nil
						}
					}
				}
			}
		}
	}
	// The evaluated route. `out` is not where the program is for a
	// multi-output package: nixpkgs splits an output off exactly when it wants
	// the executables separated, and the `bin` output references `out`.
	pfx, ok := nixx.NixPrefix()
	if !ok {
		return "", fail.Cannot("no nix here, and no hydra job for %q; pass a store path instead", b.O.Target)
	}
	out, code := proc.CaptureAllowFail(filepath.Join(pfx, "nix-instantiate"), "<nixpkgs>", "--attr", b.O.Target)
	drv := ""
	for line := range strings.SplitSeq(out, "\n") {
		if strings.HasPrefix(line, "/nix/store/") {
			drv, _, _ = strings.Cut(line, "!")
			break
		}
	}
	if code != 0 || drv == "" {
		return "", fail.Ran("nixpkgs has no attribute %q", b.O.Target)
	}
	showJSON, code := proc.CaptureAllowFail(filepath.Join(pfx, "nix"),
		"--extra-experimental-features", "nix-command", "derivation", "show", drv)
	if code != 0 {
		return "", fail.Ran("could not read the derivation %s", drv)
	}
	var doc nixx.ShowRecursive
	if err := json.Unmarshal([]byte(showJSON), &doc); err != nil {
		return "", fail.Ran("the derivation document is not JSON: %v", err)
	}
	for _, d := range doc.Derivations {
		for _, name := range []string{"bin", "out"} {
			if o, ok := d.Outputs[name]; ok && o.Path != "" {
				if name != "out" {
					logx.Say("output      %q (nixpkgs put the programs there, not in 'out')", name)
				}
				return o.Path, nil
			}
		}
		for _, o := range d.Outputs {
			if o.Path != "" {
				return o.Path, nil
			}
		}
	}
	return "", fail.Ran("could not find an output path of %s", drv)
}

func (b *Builder) hydraAttr() string {
	if m, err := b.Nix.Attr(b.O.Target); err == nil {
		return m.Attr
	}
	return b.O.Target
}

var versionSuffix = regexp.MustCompile(`-[0-9].*$`)

func deriveProgramName(base string) string {
	// ⚠ shortStoreName, NOT farmDirName: this is the PROGRAM's name, and it
	// must not pick up the farm's collision fallback -- a closure with two
	// builds of one package would otherwise name the program after a hash.
	return versionSuffix.ReplaceAllString(shortStoreName(base), "")
}

// augment pulls mesa in when the closure has libglvnd and no driver.
//
// A nixpkgs GL program does not depend on mesa: it depends on libglvnd, the
// vendor-neutral dispatch layer, which finds the implementation by reading
// share/glvnd/egl_vendor.d/*.json and dlopen'ing whatever they name. That file
// is host configuration, so mesa is not in the closure at all and the bundle
// gets as far as "eglInitialize failed".
func (b *Builder) augment() error {
	extra := append([]string(nil), b.O.Extra...)
	if !b.O.NoGL && b.hasGLDispatch() && !b.hasMesaDriver() {
		logx.Say("opengl      libglvnd is in the closure and no mesa driver is: pulling mesa in")
		extra = append(extra, "mesa")
	}
	for _, x := range extra {
		p := x
		if !storePathRe.MatchString(x) {
			resolved, err := b.resolveExtra(x)
			if err != nil {
				logx.Warnf("could not resolve --extra %s: %v", x, err)
				continue
			}
			p = resolved
		}
		if !strings.HasPrefix(p, "/nix/store/") {
			p = "/nix/store/" + p
		}
		logx.Say("extra       %s -> %s", x, p)
		if _, err := b.Nix.Fetch(p, nixx.FetchOptions{Out: b.Root, WithClosure: true}); err != nil {
			logx.Warnf("could not fetch the closure of %s: %v", x, err)
		}
	}
	if len(extra) > 0 {
		logx.Say("closure     %d store paths after augmentation", countEntries(b.Root))
	}
	return nil
}

func (b *Builder) resolveExtra(attr string) (string, error) {
	if jobFile, err := b.Nix.HydraJob(attr); err == nil {
		if raw, rerr := os.ReadFile(jobFile); rerr == nil {
			var reply nixx.HydraReply
			if json.Unmarshal(raw, &reply) == nil {
				if v, ok := reply.BuildOutputs["out"].(map[string]any); ok {
					if p, ok := v["path"].(string); ok && p != "" {
						return p, nil
					}
				}
			}
		}
	}
	return "", fmt.Errorf("no hydra job")
}

func (b *Builder) hasGLDispatch() bool {
	found := false
	_ = filepath.Walk(b.Root, func(p string, fi os.FileInfo, err error) error {
		if err != nil || fi.IsDir() || found {
			return nil
		}
		n := fi.Name()
		if strings.HasPrefix(n, "libGLdispatch.so") || strings.HasPrefix(n, "libEGL.so") ||
			strings.HasPrefix(n, "libGL.so") {
			found = true
		}
		return nil
	})
	return found
}

func (b *Builder) hasMesaDriver() bool {
	matches, _ := filepath.Glob(filepath.Join(b.Root, "*mesa-[0-9]*", "lib", "dri"))
	for _, m := range matches {
		if fi, err := os.Stat(m); err == nil && fi.IsDir() {
			return true
		}
	}
	return false
}

// storeResolve follows a symlink chain, mapping any absolute /nix/store target
// back into the unpacked closure. A closure's share/ trees are farms of
// symlinks pointing at store paths that are not here, and copying one without
// resolving it fails in a way the caller then reports as a success.
func (b *Builder) storeResolve(p string) string {
	for range 10 {
		fi, err := os.Lstat(p)
		if err != nil {
			return ""
		}
		if fi.Mode()&os.ModeSymlink == 0 {
			return p
		}
		t, err := os.Readlink(p)
		if err != nil {
			return ""
		}
		switch {
		case strings.HasPrefix(t, "/nix/store/"):
			p = filepath.Join(b.Root, strings.TrimPrefix(t, "/nix/store/"))
		case filepath.IsAbs(t):
			p = t
		default:
			p = filepath.Join(filepath.Dir(p), t)
		}
	}
	return ""
}

// Entry is what a program name resolves to.
//
// ⭐ IT IS A PAIR, NOT A PATH, AND THAT IS THE WHOLE OF T-081's SECOND BLOCKER.
// A nixpkgs Python program is `bin/<name>`, a makeBinaryWrapper ELF, whose
// target `bin/.<name>-wrapped` is a SCRIPT. A script entry point is
// interpreter + script argument; asking for one path made `resolveEntry`
// oscillate between the two shapes for five hops and then report
// `no entry point`, so NO Python GUI application bundled at all.
type Entry struct {
	ELF    string // the executable to run: the program, or its interpreter
	Script string // non-empty when ELF is an interpreter and this is its argument
}

// resolveEntry finds what a program name means, following a nixpkgs wrapper to
// the real binary and keeping the environment it set.
//
// The wrapper test comes BEFORE the ELF test: nixpkgs' makeBinaryWrapper
// output is a compiled C program, so asking "is this an ELF" first declares
// the wrapper to be the program and packs a binary that then execs an absolute
// store path the bundle does not have.
func (b *Builder) resolveEntry(storeDir, prog string, nameWasAsked bool) (Entry, error) {
	bin := filepath.Join(storeDir, "bin", prog)
	if _, err := os.Lstat(bin); err != nil {
		if nameWasAsked {
			var have []string
			if entries, err := os.ReadDir(filepath.Join(storeDir, "bin")); err == nil {
				for _, e := range entries {
					have = append(have, e.Name())
				}
			}
			sort.Strings(have)
			logx.Warnf("--name %q names no program in %s/bin. What is there:\n             %s",
				prog, filepath.Base(storeDir), strings.Join(have, " "))
			return Entry{}, fmt.Errorf("no such program")
		}
		entries, err := os.ReadDir(filepath.Join(storeDir, "bin"))
		if err != nil {
			return Entry{}, err
		}
		bin = ""
		for _, e := range entries {
			if !e.IsDir() {
				bin = filepath.Join(storeDir, "bin", e.Name())
				break
			}
		}
		if bin == "" {
			return Entry{}, fmt.Errorf("no program in %s/bin", storeDir)
		}
		logx.Warnf("bin/%s does not exist; falling back to bin/%s", prog, filepath.Base(bin))
	}

	for range 5 {
		if recs := ReadWrapper(bin); len(recs) > 0 {
			target := WrapperTarget(recs)
			for _, r := range recs {
				if r.Op != OpTarget {
					b.wrapEnv = append(b.wrapEnv, r)
				}
			}
			if target != "" {
				inside := filepath.Join(b.Root, strings.TrimPrefix(target, "/nix/store/"))
				if _, err := os.Stat(inside); err == nil {
					logx.Warnf("bin/%s is a nixpkgs wrapper -> %s", prog, filepath.Base(target))
					logx.Warnf("   its environment is read out of it: %d record(s)", len(recs))
					bin = inside
					continue
				}
			}
		}
		if elfx.IsELF(bin) {
			return Entry{ELF: bin}, nil
		}
		// ⭐ A SHEBANG IS AN ANSWER, NOT A HOP. The interpreter it names is a
		// store path, so it is in this closure and it is an ELF; the file
		// itself is the argument. This is checked BEFORE the store-path scan
		// below, which is what used to walk the script's text and resolve
		// straight back to the wrapper that pointed here.
		if interp := b.shebangInterpreter(bin); interp != "" {
			logx.Warnf("bin/%s is a SCRIPT; its entry point is %s + the script itself",
				prog, filepath.Base(interp))
			return Entry{ELF: interp, Script: bin}, nil
		}
		// A wrapper shape the reader does not recognise: take the last store
		// path it names that exists here, and say the environment was not read.
		next := lastExistingStorePath(bin, b.Root)
		if next == "" {
			logx.Warnf("bin/%s is a script and no ELF in it could be resolved", prog)
			return Entry{}, fmt.Errorf("unresolvable wrapper")
		}
		logx.Warnf("bin/%s is a wrapper of a shape the reader does not know -> %s", prog, filepath.Base(next))
		logx.Warnf("its ENVIRONMENT is NOT reproduced.")
		bin = next
	}
	return Entry{}, fmt.Errorf("too many wrapper hops")
}

// shebangInterpreter reads a script's first line and resolves the interpreter
// it names inside this closure, returning "" when the file is not a script or
// the interpreter is not an ELF that is here.
//
// ⚠ `#!/nix/store/…/bin/env python3` IS HANDLED, because nixpkgs writes it:
// the interpreter is then the second word, looked up in the closure's own
// bin/ directories rather than on the host's PATH.
func (b *Builder) shebangInterpreter(script string) string {
	f, err := os.Open(script)
	if err != nil {
		return ""
	}
	defer f.Close()
	var head [512]byte
	n, _ := f.Read(head[:])
	if n < 3 || head[0] != '#' || head[1] != '!' {
		return ""
	}
	line := string(head[2:n])
	if i := strings.IndexAny(line, "\n\r"); i >= 0 {
		line = line[:i]
	}
	words := strings.Fields(line)
	if len(words) == 0 {
		return ""
	}
	cand := words[0]
	if filepath.Base(cand) == "env" && len(words) > 1 {
		// `env python3` — the name is a program, not a path.
		if p := b.findProgram(words[1]); p != "" && elfx.IsELF(p) {
			return p
		}
		return ""
	}
	if strings.HasPrefix(cand, "/nix/store/") {
		inside := filepath.Join(b.Root, strings.TrimPrefix(cand, "/nix/store/"))
		if elfx.IsELF(inside) {
			return inside
		}
		// A wrapper as the interpreter: follow it one hop.
		if e, err := b.resolveEntry(filepath.Dir(filepath.Dir(inside)), filepath.Base(inside), false); err == nil && e.Script == "" {
			return e.ELF
		}
		return ""
	}
	// An absolute host path (`#!/bin/sh`) is NOT resolved: running the host's
	// interpreter puts the host's libc in the process, which is the one thing
	// a bundle may not do. Reported by the caller as an unresolved entry.
	return ""
}

// storeRefRe finds a candidate store reference. ⛔ IT DOES NOT DECIDE ANYTHING:
// every caller cuts the match at the first `/`, looks the base up in the
// closure, and leaves the text alone when it is not there. The regex only says
// where a candidate STARTS and ENDS.
//
// ⭐ THE CLASS IS A BLACKLIST AND THAT DIRECTION IS DELIBERATE. Nix does not
// constrain the names of files INSIDE a store path, so a whitelist would
// TRUNCATE a legitimate tail -- the dangerous failure, because
// rewriteTextStorePaths substitutes the prefix and copies the tail verbatim, so
// a short tail is written back and corrupts the file. Over-capture fails safe:
// the base matches nothing, nothing is rewritten, and the path is REPORTED.
// So exclude only what cannot be in a path here -- control characters and space
// (a C string ends at NUL), quotes, and the XML and shell delimiters that end a
// path in the formats a bundle carries.
// ⚠ It excluded three characters until docs/history/corrections.md C27, and
// stopped at neither NUL nor `<`. The selftest carries both cases.
// storeRefStop is the ONE definition of where a store reference ends, shared by
// every pattern in this package. ⛔ Four patterns with four character classes
// is the "regex cascade" T-081 said it was replacing. corrections.md C27, C28.
const storeRefStop = `\x00-\x20"'` + "`" + `<>|;,()\\`

var storeRefRe = regexp.MustCompile(`/nix/store/[a-z0-9]{32}-[^` + storeRefStop + `]*`)

func lastExistingStorePath(file, root string) string {
	b, err := os.ReadFile(file)
	if err != nil {
		return ""
	}
	found := ""
	for _, m := range storeRefRe.FindAllString(string(b), -1) {
		p := filepath.Join(root, strings.TrimPrefix(m, "/nix/store/"))
		if fi, err := os.Stat(p); err == nil && !fi.IsDir() && fi.Mode()&0o111 != 0 {
			found = p
		}
	}
	return found
}

func countEntries(dir string) int {
	entries, err := os.ReadDir(dir)
	if err != nil {
		return 0
	}
	return len(entries)
}

// download fetches a file, falling back to the reverse proxy with the whole
// original URL, scheme included, when the direct route refuses.
func download(url, dst string) error {
	// ⛔ THE CACHE IS KEYED ON THE URL, NOT ON THE PATH, AND IT HAS TO BE.
	// This returned early whenever `dst` existed and was executable, whatever
	// URL was asked for. So moving a pin in the const block above changed
	// nothing on any machine that had ever built a bundle: the old tool sat in
	// the cache and every later AppImage carried it, silently, while the source
	// said otherwise. Found when the uruntime pin moved to the lite build on
	// 2026-09-03d and the artefact did not change size.
	//
	// ⚠ The sidecar records the URL the cached file actually came from. A
	// mismatch -- or an absent sidecar, which is every cache written before
	// this -- re-fetches. That makes the pin binding rather than decorative,
	// which is the whole reason the comment on the const block calls it "the
	// point".
	stamp := dst + ".url"
	if fi, err := os.Stat(dst); err == nil && fi.Mode()&0o111 != 0 {
		if got, err := os.ReadFile(stamp); err == nil && strings.TrimSpace(string(got)) == url {
			return nil
		}
		log.Debugf("%s: cached copy is from a different URL, re-fetching", dst)
	}
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return err
	}
	logx.Say("fetching %s", filepath.Base(dst))
	for _, u := range []string{url, "https://api.rv.pkgforge.dev/" + url} {
		if err := fetchTo(u, dst+".part"); err != nil {
			log.Debugf("%s: %v", u, err)
			continue
		}
		if err := os.Chmod(dst+".part", 0o755); err != nil {
			return err
		}
		if err := os.Rename(dst+".part", dst); err != nil {
			return err
		}
		// ⛔ Written AFTER the rename. A stamp that lands first would claim a
		// tool that is not there yet, and the next run would trust it.
		return os.WriteFile(stamp, []byte(url+"\n"), 0o644)
	}
	return fmt.Errorf("could not fetch %s", url)
}

func fetchTo(url, dst string) error {
	client := &http.Client{Timeout: 30 * time.Minute}
	resp, err := client.Get(url)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}
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

func toolURL(tmpl, arch string) string { return fmt.Sprintf(tmpl, arch) }

// pack writes the AppImage with uruntime as the header and dwarfs as the
// filesystem.
func (b *Builder) pack() error {
	tools := filepath.Join(b.O.Cache, "tools")
	uruntime := filepath.Join(tools, "uruntime")
	mkdwarfs := filepath.Join(tools, "mkdwarfs")
	if err := download(envOr("URUNTIME_URL", toolURL(defaultURuntimeURL, b.Arch)), uruntime); err != nil {
		return fail.Ran("could not fetch uruntime: %v", err)
	}
	if err := download(envOr("DWARFS_URL", toolURL(defaultDwarfsURL, b.Arch)), mkdwarfs); err != nil {
		return fail.Ran("could not fetch mkdwarfs: %v", err)
	}
	out := b.O.Out
	if out == "" {
		out = filepath.Join(b.Work, fmt.Sprintf("%s-anylinux-%s.AppImage", b.Prog, b.Arch))
	}
	logx.Say("")
	// ⛔ THE RUNTIME IS NAMED, NOT JUST MENTIONED, AND THAT IS A GATE THIS
	// PROJECT DOES NOT OTHERWISE HAVE. Moving the pins above silently
	// invalidates the committed evidence of every bundle experiment -- 78-,
	// 85-, 86-, 89-, 90- -- and NO gate can see it: `check-docs.sh` compares an
	// experiment's script against its evidence, and the script did not change.
	// It is `docs/history/corrections.md` C5's shape, "a committed evidence
	// file described a build configuration that no longer exists", reached
	// through the Go source instead of through the environment.
	// ⭐ Printing the two versions puts them in every experiment's `run.log`,
	// so a stale result says which runtime it describes instead of looking
	// current.
	logx.Say("packing with %s + %s",
		filepath.Base(toolURL(defaultURuntimeURL, b.Arch)),
		filepath.Base(toolURL(defaultDwarfsURL, b.Arch)))
	logFile := filepath.Join(b.Work, "mkdwarfs.log")
	lf, err := os.Create(logFile)
	if err != nil {
		return err
	}
	defer lf.Close()
	r, err := (&proc.Cmd{Argv: []string{mkdwarfs,
		"--force", "--set-owner", "0", "--set-group", "0",
		"--no-history", "--no-create-timestamp",
		"--header", uruntime,
		"--input", b.AppDir, "--output", out,
		// ⭐ -S18 IS A 256 KiB DWARFS BLOCK, AND IT IS THE SECOND-LARGEST
		// LEVER IN THE BUNDLER'S COLD START. dwarfs decompresses a whole
		// block to serve any byte in it, so the first read through a cold
		// mount pays for one block whatever it asked for. This read -S26 --
		// a 64 MiB block -- until 2026-09-03d, and nothing in the record
		// said where that came from.
		//
		// `experiments/81-` swept it on two subjects, a 5.8 MB `jq` bundle
		// and the same bundle plus 200 MiB, median of 15, interleaved, with
		// an A/A control. Cold start, and the size a real (compressible)
		// payload pays:
		//
		//	block     small     large     size
		//	64 MiB    1.00x     1.00x        —   what this was
		//	16 MiB    1.01x     1.00x    -0.0%
		//	 4 MiB    0.97x     0.80x    +6.9%
		//	 1 MiB    0.88x     0.73x   +11.0%
		//	256 KiB   0.76x     0.66x   +17.8%   ⭐ here
		//	 64 KiB   0.73x     0.64x   +36.9%
		//	 16 KiB   0.84x     0.78x   +54.6%   the curve turns
		//
		// ⛔ 64 KiB is the minimum of the curve and is NOT taken. It buys
		// 0.02x more and costs another 19% of the artefact; the operator's
		// 2026-09-03c ruling makes size acceptable, not free, and goal 3
		// still names "smaller" for kdenlive. ⚠ The two earlier sweeps
		// stopped at 1 MiB and at 256 KiB and were monotonic to their own
		// floor both times -- a sweep that has not found its minimum has
		// measured its range, not its knob, which is why 81- goes to 16 KiB.
		"-C", "zstd:level=19", "-S18"},
		Stdout: lf, Stderr: lf, Subsys: "bundle"}).Run()
	if err != nil {
		return fail.Cannot("mkdwarfs: %v", err)
	}
	if r.Failed() {
		// Its own error, not ours: "mkdwarfs failed" with the output discarded
		// is a message with no information in it.
		tailFile(logFile, 12)
		return fail.Ran("mkdwarfs failed (its output is above, and in %s)", logFile)
	}
	if err := os.Chmod(out, 0o755); err != nil {
		return err
	}
	fi, err := os.Stat(out)
	if err != nil {
		return err
	}
	logx.Say("")
	logx.Say("built  %s  (%s)", out, humanBytes(fi.Size()))
	logx.Say("AppDir kept at %s", b.AppDir)
	return nil
}

func tailFile(path string, n int) {
	b, err := os.ReadFile(path)
	if err != nil {
		return
	}
	lines := strings.Split(strings.TrimRight(string(b), "\n"), "\n")
	if len(lines) > n {
		lines = lines[len(lines)-n:]
	}
	for _, l := range lines {
		fmt.Fprintln(os.Stderr, l)
	}
}

// materialiseAppRunSource writes the carried AppRun source so it can be
// compiled into the bundle.
func materialiseAppRunSource(dir string) (string, error) {
	if err := assets.Materialise(dir); err != nil {
		return "", err
	}
	return filepath.Join(dir, "pgb-apprun.c"), nil
}

func itoa(n int) string { return strconv.Itoa(n) }
