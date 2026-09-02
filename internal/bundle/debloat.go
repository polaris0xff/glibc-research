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
