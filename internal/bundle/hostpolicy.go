// hostpolicy.go — what a bundle may take from the HOST, and in what order.
//
// ⛔ THIS FILE OVERTURNS A RULE THIS REPOSITORY APPLIED EVERYWHERE.
// docs/AGENTS.md §3 criterion 2 -- "loads no host shared object" -- is the
// right acceptance test for a STATIC ELF and the wrong one for a bundle. A
// static binary that opens a host .so has put a second libc in the process
// (docs/limitations.md §1). A bundle carries its OWN glibc and its own ld.so,
// so a host object loaded into it binds against the BUNDLE's libc, and that is
// upstream's stated reason for bundling glibc rather than musl:
//
//	"With glibc, we are able to dlopen optional libraries on the host even
//	 when those link to musl."   -- Anylinux-AppImages/FAQ.md
//
// So for a bundle the question is not WHETHER a host object was loaded but
// WHICH. libGLX_nvidia.so.0 is correct; libcurl.so.4 is a bundling bug.
//
// The full argument, the quotations and the provenance are in
// docs/design/host-fallback.md. TODO T-065. This file is the mechanism.
//
// ⚠ NO GPU WAS INVOLVED IN ANY OF THIS. Every GL row in this repository is
// swrast (TODO T-059), so the driver classes below are a code and
// documentation read of the references, never an execution here. They are
// implemented and REPORTED; experiments/77- asserts the order and records the
// driver classes as unexercised rather than passing them.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"fmt"
	"os"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/logx"
)

// HostPolicy is the decision, per class, about where a bundle looks.
//
// ⭐ THE ZERO VALUE IS THE POLICY: bundled-first for everything, the host
// reachable only through the lowest-priority fallback. Each opt-in moves ONE
// class and says so out loud.
type HostPolicy struct {
	// Mesa lets the host's GL/VA drivers win over the bundled ones.
	//
	// ⚠ Upstream tried mixing bundled and host drivers and withdrew it --
	// "that ended up being a bad idea" -- so this is off by default and the
	// cost is stated when it is on: "not guaranteed to work forever due to
	// glibc symbol nonsense" (Anylinux-AppImages/FAQ.md).
	Mesa bool

	// Vulkan admits the host's ICDs and layers. Separate from Mesa because
	// the LAYERS are the user's rather than the application's: MangoHud and
	// lsfg-vk are installed by the person running the program and cannot be
	// anticipated by the bundle. sharun gates this on SHARUN_ALLOW_SYS_VKICD.
	Vulkan bool

	// Glibc puts the HOST's glibc ahead of the bundled one.
	//
	// ⛔ The narrowest opt-in and the most expensive: it gives up the property
	// the bundle exists for. experiments/73- measured the case it is for --
	// class B, 20 symbols, 14 of them __isoc23_*, where a host object was
	// built against a newer glibc than the bundle carries.
	Glibc bool

	// Extra is SHARUN_EXTRA_LIBRARY_PATH: highest priority, one run, the
	// caller's own. Never written into .env, only reported.
	Extra string
}

// ⭐ NVIDIA IS NOT AN OPT-IN AND HAS NO FIELD ABOVE, deliberately.
//
// It is the one class where the host's copy is unconditionally correct, and
// the reason is an ABI fact rather than a preference: the proprietary driver
// is built against a glibc old enough that any bundled glibc satisfies it.
//
//	"we never need to bundle the NVIDIA drivers, NVIDIA releases its driver
//	 linking to a +10yo version of glibc, that means we can use that driver
//	 without issue."   -- Anylinux-AppImages/HALL-OF-FAME.md
//
// It also CANNOT be bundled: it is the counterpart of a kernel module the user
// installed, and a stale copy is worse than no copy.
var nvidiaDirs = []string{
	"/etc/libnvidiacurrent",
}

// hostDirs are the ordinary system directories, in the order
// Anylinux-sharun/src/main.rs:296-325 uses them.
//
// ⚠ /run/opengl-driver/lib IS NOT DECORATION. pg83/solo's issue #2 was a NixOS
// VkResult -9 fixed by scanning /run/opengl-driver, and the same tracker notes
// that loading the system's libvulkan.so.1 dynamically does not save you
// there either. A search order missing this row fails on NixOS ONLY, which is
// the worst kind of missing row.
var hostDirs = []string{
	"/usr/local/lib", "/usr/lib", "/lib",
	"/usr/local/lib64", "/usr/lib64", "/lib64",
	"/usr/lib/x86_64-linux-gnu",
	"/run/opengl-driver/lib", "/run/current-system/sw/lib",
}

// LoadHostPolicy reads the opt-ins from the environment.
//
// ⛔ Every one is REPORTED, never silent. T-065's instruction is explicit that
// a deferral is reported rather than silent, and an opt-in IS a deferral to the
// host -- an opt-in that quietly changed which libc a process runs is the
// failure mode this whole project is about.
func LoadHostPolicy() HostPolicy {
	p := HostPolicy{
		Mesa:   logx.EnvBool("PGB_HOST_MESA", false),
		Vulkan: logx.EnvBool("PGB_HOST_VULKAN", false),
		Glibc:  logx.EnvBool("PGB_HOST_GLIBC", false),
		Extra:  os.Getenv("SHARUN_EXTRA_LIBRARY_PATH"),
	}
	p.Report()
	return p
}

// Report prints the policy that was applied, always, including the default.
func (p HostPolicy) Report() {
	logx.Say("host policy bundled-first; host reachable at lowest priority")
	say := func(class, state, why string) {
		logx.Say("  %-10s %-16s %s", class, state, why)
	}
	say("nvidia", "HOST always", "never bundled; its glibc floor is 10+ years old")
	if p.Mesa {
		say("mesa", "HOST (PGB_HOST_MESA)", "⚠ not guaranteed to keep working: glibc symbols")
	} else {
		say("mesa", "bundled", "set PGB_HOST_MESA=1 to prefer the host's")
	}
	if p.Vulkan {
		say("vulkan", "HOST (PGB_HOST_VULKAN)", "⚠ host ICDs and layers admitted")
	} else {
		say("vulkan", "bundled", "set PGB_HOST_VULKAN=1 for host ICDs and layers")
	}
	if p.Glibc {
		say("glibc", "HOST (PGB_HOST_GLIBC)", "⛔ gives up what the bundle exists for")
	} else {
		say("glibc", "bundled", "set PGB_HOST_GLIBC=1 only if the bundle cannot run")
	}
	if p.Extra != "" {
		say("extra", "caller", p.Extra)
	}
}

// EnvLines are the .env lines the policy contributes, appended after the
// bundle's own paths so the ordering in docs/design/host-fallback.md holds.
//
// `have` reports whether the bundle carries a given relative path, so a class
// the bundle does not have never claims to serve it.
func (p HostPolicy) EnvLines(have func(rel string) bool) []string {
	var out []string
	add := func(f string, a ...any) { out = append(out, fmt.Sprintf(f, a...)) }

	// ⛔ ROW 8 OF THE SEARCH ORDER, AND IT IS LAST ON PURPOSE. sharun's own
	// help calls it "Fallback library directories with lowest priority": the
	// host is reachable, in the one position where it can only ever answer a
	// question nothing bundled could.
	fallback := append([]string{}, nvidiaDirs...)
	fallback = append(fallback, hostDirs...)
	if p.Glibc {
		// ⛔ The one opt-in that reorders rather than appends. Reported by
		// Report() above; the .env carries no explanation, so this comment is
		// where the reader finds out why the order changed.
		add("SHARUN_EXTRA_LIBRARY_PATH=%s", strings.Join(hostDirs, ":"))
	}
	add("SHARUN_FALLBACK_LIBRARY_PATH=%s", strings.Join(fallback, ":"))

	// Drivers. Each is set only when the bundle actually has that tree, so a
	// bundle with no dri/ never claims to serve GL.
	if !p.Mesa {
		if have("lib/dri") {
			// nixGL is the control that shows bundling is a complete answer
			// here: it does not use the host's GL either -- it points
			// nixpkgs' own mesa at itself.
			add("LIBGL_DRIVERS_PATH=${SHARUN_DIR}/lib/dri")
			add("LIBVA_DRIVERS_PATH=${SHARUN_DIR}/lib/dri")
		}
		if have("share/glvnd/egl_vendor.d") {
			add("__EGL_VENDOR_LIBRARY_DIRS=${SHARUN_DIR}/share/glvnd/egl_vendor.d")
		}
	} else {
		// ⚠ Unset rather than pointed elsewhere. Naming a host directory here
		// would pin ONE host layout; leaving the variable unset lets the
		// host's own mesa find its drivers the way it normally does.
		add("PGB_HOST_MESA=1")
	}

	// ⭐ VULKAN: the bundle's ICDs, and the host's NVIDIA ones ALWAYS. The
	// second half is not the opt-in -- an NVIDIA ICD is the counterpart of the
	// user's kernel module and there is nothing to bundle.
	var icd []string
	if have("share/vulkan/icd.d") {
		icd = append(icd, "${SHARUN_DIR}/share/vulkan/icd.d")
	}
	icd = append(icd, "/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d")
	if len(icd) > 0 {
		add("VK_DRIVER_FILES=%s", strings.Join(icd, ":"))
	}
	if p.Vulkan {
		add("VK_LAYER_PATH=/usr/share/vulkan/explicit_layer.d:/etc/vulkan/explicit_layer.d")
		add("SHARUN_ALLOW_SYS_VKICD=1")
	}
	return out
}

// HostClass says which of the four permitted classes a loaded host object
// belongs to, or "" when it belongs to none.
//
// ⭐ THIS IS THE ACCEPTANCE TEST FOR A BUNDLE, and it is the change T-065
// asked for. For a static ELF the test stays "zero host shared objects"; for a
// bundle it becomes "no host shared object OUTSIDE these classes". A bundle
// loading the host's libcurl is still a defect and still fails; one loading
// libGLX_nvidia.so.0 has done the right thing.
func HostClass(path string) string {
	base := path
	if i := strings.LastIndex(base, "/"); i >= 0 {
		base = base[i+1:]
	}
	switch {
	case strings.Contains(base, "nvidia") || strings.Contains(base, "NVIDIA") ||
		strings.HasPrefix(base, "libcuda.") || strings.HasPrefix(base, "libnvcuvid.") ||
		strings.HasPrefix(base, "libGLX_nvidia.") || strings.HasPrefix(base, "libEGL_nvidia."):
		return "nvidia"
	case strings.HasPrefix(base, "libGL") || strings.HasPrefix(base, "libEGL") ||
		strings.HasPrefix(base, "libgbm") || strings.HasPrefix(base, "libdrm") ||
		strings.HasPrefix(base, "libva") || strings.Contains(path, "/dri/"):
		return "mesa"
	case strings.HasPrefix(base, "libvulkan") || strings.Contains(path, "vulkan"):
		return "vulkan"
	case strings.HasPrefix(base, "libc.so") || strings.HasPrefix(base, "ld-linux") ||
		strings.HasPrefix(base, "libm.so") || strings.HasPrefix(base, "libpthread.so") ||
		strings.HasPrefix(base, "libdl.so") || strings.HasPrefix(base, "librt.so"):
		return "glibc"
	}
	return ""
}
