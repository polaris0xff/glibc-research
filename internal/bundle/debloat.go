// debloat.go — remove what a bundle cannot use, with a reason per rule.
//
// The number this exists for: on a GL application an undebloated mesa is 95
// MiB of a 163 MB bundle, and most of that is not the GL driver — it is Vulkan
// ICDs for GPUs that cannot exist on this architecture. panfrost is ARM Mali,
// freedreno is Adreno, broadcom is a Raspberry Pi, asahi is Apple silicon,
// powervr is Imagination, dzn is Direct3D 12 on Windows, gfxstream is an
// Android emulator transport. Dropping them is not a size/function trade.
//
// Nothing is dropped without its reference: a Vulkan driver is found through
// share/vulkan/icd.d/<name>.json, and removing the library while leaving the
// JSON gives a loader that opens a file that is gone. Both go.
//
// The control runs after every rule: every DT_NEEDED of every ELF left in the
// bundle must still resolve inside it.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"os"
	"path/filepath"
	"slices"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/logx"
)

// foreignVulkan lists the drivers with no possible GPU on an architecture.
func foreignVulkan(arch, level string) []string {
	var list []string
	switch arch {
	case "x86_64", "i686", "i386":
		list = []string{"panfrost", "freedreno", "broadcom", "asahi", "powervr",
			"dzn", "gfxstream", "v3dv", "imagination"}
	case "aarch64":
		list = []string{"dzn", "gfxstream", "intel_hasvk"}
	}
	if level == "aggressive" {
		list = append(list, "intel", "intel_hasvk", "radeon", "nouveau", "virtio", "swrast_no")
	}
	return list
}

// docDirs are what a bundle is not: a development environment.
var docDirs = []string{"doc", "man", "info", "gtk-doc", "devhelp",
	"bash-completion", "zsh", "fish"}

// buildOnlyDirs and buildOnlyExts are what only a compiler would open.
var (
	buildOnlyDirs = []string{"include", "pkgconfig", "cmake", "aclocal"}
	buildOnlyExts = []string{".a", ".la", ".pc"}
)

func (b *Builder) debloat() {
	before := dirSize(b.AppDir)
	logx.Say("")
	logx.Say("debloating (level: %s)", b.O.Debloat)

	share := filepath.Join(b.AppDir, "share")
	var docs []string
	for _, d := range docDirs {
		docs = append(docs, filepath.Join(share, d))
	}
	b.dropRule("documentation and shell completions", docs)

	var buildOnly []string
	_ = filepath.Walk(b.AppDir, func(p string, fi os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if fi.IsDir() {
			depth := strings.Count(strings.TrimPrefix(p, b.AppDir), string(os.PathSeparator))
			if depth <= 3 && containsString(buildOnlyDirs, fi.Name()) {
				buildOnly = append(buildOnly, p)
				return filepath.SkipDir
			}
			return nil
		}
		for _, ext := range buildOnlyExts {
			if strings.HasSuffix(fi.Name(), ext) {
				buildOnly = append(buildOnly, p)
				return nil
			}
		}
		return nil
	})
	b.dropRule("static archives, headers and build metadata", buildOnly)

	// Locales are a real trade and it is stated as one: the application's own
	// translations go with them. --keep-locales names the ones to keep.
	localeRoot := filepath.Join(share, "locale")
	if entries, err := os.ReadDir(localeRoot); err == nil {
		keep := map[string]bool{}
		for _, k := range b.O.KeepLocales {
			keep[k] = true
		}
		var drop []string
		for _, e := range entries {
			if e.IsDir() && !keep[e.Name()] {
				drop = append(drop, filepath.Join(localeRoot, e.Name()))
			}
		}
		kept := strings.Join(b.O.KeepLocales, ",")
		if kept == "" {
			kept = "none"
		}
		b.dropRule("locale catalogues (kept: "+kept+")", drop)
	}

	for _, f := range foreignVulkan(b.Arch, b.O.Debloat) {
		var drop []string
		libs, _ := filepath.Glob(filepath.Join(b.AppDir, "lib", "libvulkan_"+f+"*.so*"))
		drop = append(drop, libs...)
		_ = filepath.Walk(filepath.Join(b.AppDir, "share", "vulkan"), func(p string, fi os.FileInfo, err error) error {
			if err == nil && !fi.IsDir() && strings.Contains(fi.Name(), f) &&
				strings.HasSuffix(fi.Name(), ".json") {
				drop = append(drop, p)
			}
			return nil
		})
		b.dropRule("vulkan driver '"+f+"' (no such GPU on "+b.Arch+")", drop)
	}

	// An NPU delegate is not a graphics driver and nothing in a GL closure
	// references it.
	teflon, _ := filepath.Glob(filepath.Join(b.AppDir, "lib", "libteflon.so*"))
	b.dropRule("libteflon (an NPU delegate, not a GPU driver)", teflon)

	b.dropLocaleSources()

	after := dirSize(b.AppDir)
	if before > 0 {
		logx.Say("  debloat   %s -> %s (%.1f%% off)",
			humanBytes(before), humanBytes(after),
			float64(before-after)*100/float64(before))
	}
}

// dropRule removes a rule's paths and reports what it cost.
func (b *Builder) dropRule(label string, paths []string) {
	var n int
	var bytes int64
	for _, p := range paths {
		if p == "" {
			continue
		}
		size := dirSize(p)
		if err := os.RemoveAll(p); err != nil {
			continue
		}
		n++
		bytes += size
	}
	if n > 0 {
		logx.Say("  debloat   %10s  %3d  %s", humanBytes(bytes), n, label)
	}
}

func dirSize(path string) int64 {
	var total int64
	fi, err := os.Lstat(path)
	if err != nil {
		return 0
	}
	if !fi.IsDir() {
		return fi.Size()
	}
	_ = filepath.Walk(path, func(_ string, info os.FileInfo, err error) error {
		if err == nil && !info.IsDir() {
			total += info.Size()
		}
		return nil
	})
	return total
}

func containsString(list []string, want string) bool {
	return slices.Contains(list, want)
}

// DropUnreachable removes the shared objects nothing in the bundle can reach.
//
// ⛔ IT RUNS AFTER writeEnv(), NOT INSIDE debloat(), AND THAT ORDER IS THE
// WHOLE CORRECTNESS OF IT. The sweep learns which directories hold plugins
// partly from the bundle's own `.env` -- MLT_REPOSITORY, FREI0R_PATH,
// GST_PLUGIN_SYSTEM_PATH_1_0 and the rest name directories whose contents
// nothing links against and which are loaded by name at run time.
//
// ⚠ MEASURED, BY BREAKING IT. The first version called this from debloat(),
// which runs BEFORE writeEnv(), so `.env` did not exist yet, the sweep saw no
// plugin directories, and it deleted kdenlive's MLT modules as unreachable.
// experiments/90- caught it in one row: ours rendered 0 bytes of MP4 where the
// previous run rendered 4,149. ⭐ jq did not catch it and could not -- a CLI
// with no plugin directories has nothing at risk, which is the limit of
// iterating on a CLI and the reason the plugin-heavy case stays a control.
//
// ⛔ THE SWEEP EXISTED AND NOTHING CONSUMED IT. `internal/bundle/sweep.go`
// computes exactly this and had two callers: the `pgb bundle sweep`
// subcommand, which only prints, and its own selftest. The bundler's build
// path never called it, so every rule above is a rule about NAMES -- a list of
// Vulkan drivers, a list of documentation directories -- and the one
// structural answer in the tree was reporting to a human and being thrown
// away. `TODO` T-066 calls this the largest unused lever and it was right:
// codegraph callers Sweep is the one command that shows it.
//
// ⭐ WHY THIS IS SAFE TO DELETE ON, and it is the sweep's own design rather
// than optimism: reachability is the DT_NEEDED closure of every program in the
// bundle PLUS every object in any directory something could load a plugin out
// of, and ANYTHING A RULE CANNOT CLASSIFY COUNTS AS REACHABLE. It errs toward
// keeping. `b.integrity()` then re-checks that every DT_NEEDED of every ELF
// left still resolves inside the bundle, so a mistake here is a build failure
// rather than a bundle that dies on someone's machine.
//
// ⚠ It runs at `safe` as well as `aggressive`. A structural proof that nothing
// can reach an object is not a size/function trade of the kind `--debloat
// aggressive` exists to gate; the locale rule above is one of those and this
// is not.
func (b *Builder) DropUnreachable() {
	res, err := Sweep(SweepOptions{
		Dir:      b.AppDir,
		EnvFiles: []string{filepath.Join(b.AppDir, ".env")},
	})
	if err != nil {
		// ⚠ Reported, not fatal, and NOTHING is deleted. A sweep that could
		// not run is not a sweep that found nothing -- docs/AGENTS.md §0b.
		logx.Say("  debloat   reachability sweep could not run: %v", err)
		return
	}
	if len(res.Unresolved) > 0 {
		// ⛔ The bundle is already inconsistent, so its DT_NEEDED graph is not
		// a sound basis for deleting anything. Say so and keep everything.
		logx.Say("  debloat   %d unresolved DT_NEEDED name(s); sweep NOT used to delete",
			len(res.Unresolved))
		return
	}
	// ⚠ SweepResult.Unreachable is RELATIVE to the bundle root; dropRule takes
	// paths. Passing the relative ones deletes nothing and reports success,
	// which is the quiet no-op docs/AGENTS.md §0b calls the worst answer this
	// codebase can give.
	abs := make([]string, 0, len(res.Unreachable))
	for _, rel := range res.Unreachable {
		abs = append(abs, filepath.Join(b.AppDir, rel))
	}
	b.dropRule("unreachable by DT_NEEDED from any program or plugin dir", abs)
}

// dropLocaleSources removes share/i18n, which is glibc's locale SOURCE data.
//
// ⭐ THE NUMBER THAT FOUND THIS: on a `jq` bundle, share/ was 17 MiB of a
// 22 MiB AppDir and share/i18n was all of it -- cns11643_stroke alone is
// 4.31 MiB, iso14651_t1_common 3.23 MiB. `jq`. The bundle's entire lib/ was
// 4.8 MiB.
//
// ⛔ IT IS NOT RUNTIME DATA. share/i18n/locales and share/i18n/charmaps are
// the INPUT to `localedef`: text a locale is compiled FROM. What a running
// program reads is the COMPILED form -- a locale-archive, or lib/locale --
// and that is a different tree which this rule does not touch. glibc ships
// the sources so a system can build locales it was not given.
//
// ⚠ SO THE RULE IS CONDITIONAL, not a blanket delete. If the bundle ships a
// program that compiles locales, the sources are its input and it keeps them.
// A bundle carrying `localedef` and no i18n data would be a bundle whose
// localedef cannot do anything, which is the silent-wrong-answer shape this
// project keeps finding.
func (b *Builder) dropLocaleSources() {
	for _, prog := range []string{"localedef", "locale", "iconvconfig"} {
		for _, dir := range []string{"bin", filepath.Join("shared", "bin")} {
			if _, err := os.Stat(filepath.Join(b.AppDir, dir, prog)); err == nil {
				logx.Say("  debloat   share/i18n kept: the bundle ships %s", prog)
				return
			}
		}
	}
	i18n := filepath.Join(b.AppDir, "share", "i18n")
	if _, err := os.Stat(i18n); err != nil {
		return
	}
	b.dropRule("share/i18n (locale SOURCES; nothing here compiles locales)",
		[]string{i18n})
}
