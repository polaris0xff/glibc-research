// hostpolicy_selftest.go — the host-fallback policy, asserted offline.
//
// ⛔ THE ORDER IS THE MECHANISM, so the order is what this asserts. A policy
// that names all the right directories in the wrong sequence is a bundle that
// silently prefers the host's copy of a library it carries, which is the
// failure docs/design/host-fallback.md exists to prevent — and it would pass
// any test that only checked which paths were mentioned.
//
// ⚠ NO GPU IS INVOLVED and none is needed here: this asserts what the policy
// EMITS, which is decidable offline. Whether a real NVIDIA driver then loads
// is TODO T-059 and is not claimed by anything in this file.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// HostPolicySelftest checks the emitted environment and the classifier.
func HostPolicySelftest() *selftest.Report {
	r := selftest.New("bundle-hostpolicy")

	// A bundle that carries everything, so every "only if present" branch is
	// exercised rather than skipped by an absent directory.
	full := func(string) bool { return true }
	// And one that carries nothing, which is the case that must not claim to
	// serve GL or Vulkan out of directories it does not have.
	empty := func(string) bool { return false }

	get := func(lines []string, key string) string {
		for _, l := range lines {
			if v, ok := strings.CutPrefix(l, key+"="); ok {
				return v
			}
		}
		return ""
	}

	// ---- the default policy: bundled first, host last ----------------------
	def := HostPolicy{}.EnvLines(full)

	fb := get(def, "SHARUN_FALLBACK_LIBRARY_PATH")
	r.CheckBool("default: a host fallback exists at all", fb != "", true)

	// ⭐ NVIDIA IS FIRST INSIDE THE FALLBACK and is never an opt-in: its
	// driver is built against a glibc old enough that any bundled one
	// satisfies it, and there is nothing to bundle in its place.
	r.CheckBool("default: /etc/libnvidiacurrent is in the fallback",
		strings.Contains(fb, "/etc/libnvidiacurrent"), true)
	r.CheckBool("default: nvidia precedes the ordinary host dirs",
		strings.Index(fb, "/etc/libnvidiacurrent") < strings.Index(fb, "/usr/lib"), true)

	// ⚠ The NixOS row. A search order missing this fails on NixOS ONLY, which
	// is the worst kind of missing row — pg83/solo issue #2.
	r.CheckBool("default: /run/opengl-driver/lib is in the fallback",
		strings.Contains(fb, "/run/opengl-driver/lib"), true)

	// ⭐ The bundle's own drivers win by default.
	r.Check("default: LIBGL_DRIVERS_PATH is the bundle's",
		get(def, "LIBGL_DRIVERS_PATH"), "${SHARUN_DIR}/lib/dri")
	r.CheckBool("default: the bundle's ICDs come before the host's",
		strings.HasPrefix(get(def, "VK_DRIVER_FILES"), "${SHARUN_DIR}/"), true)

	// ⭐ ...and the host's NVIDIA ICDs are admitted even so. This is the half
	// that is NOT an opt-in, and a test that only checked "bundled wins"
	// would pass while the driver was unreachable.
	r.CheckBool("default: the host ICD dirs are still reachable",
		strings.Contains(get(def, "VK_DRIVER_FILES"), "/usr/share/vulkan/icd.d"), true)

	// ⛔ NEGATIVE: with nothing bundled, nothing may claim to serve it.
	bare := HostPolicy{}.EnvLines(empty)
	r.Check("empty bundle: no LIBGL_DRIVERS_PATH is claimed",
		get(bare, "LIBGL_DRIVERS_PATH"), "")
	r.CheckBool("empty bundle: VK_DRIVER_FILES names no ${SHARUN_DIR}",
		strings.Contains(get(bare, "VK_DRIVER_FILES"), "${SHARUN_DIR}"), false)
	r.CheckBool("empty bundle: the host fallback still exists",
		get(bare, "SHARUN_FALLBACK_LIBRARY_PATH") != "", true)

	// ---- the opt-ins each move exactly one class ---------------------------
	mesa := HostPolicy{Mesa: true}.EnvLines(full)
	r.Check("PGB_HOST_MESA: the bundle stops claiming LIBGL_DRIVERS_PATH",
		get(mesa, "LIBGL_DRIVERS_PATH"), "")
	r.CheckBool("PGB_HOST_MESA: it is recorded in the environment",
		get(mesa, "PGB_HOST_MESA") == "1", true)
	// ⛔ and it must NOT have moved Vulkan or glibc.
	r.CheckBool("PGB_HOST_MESA: vulkan is untouched",
		strings.HasPrefix(get(mesa, "VK_DRIVER_FILES"), "${SHARUN_DIR}/"), true)
	r.Check("PGB_HOST_MESA: glibc is untouched",
		get(mesa, "SHARUN_EXTRA_LIBRARY_PATH"), "")

	vk := HostPolicy{Vulkan: true}.EnvLines(full)
	r.CheckBool("PGB_HOST_VULKAN: host layers are admitted",
		get(vk, "VK_LAYER_PATH") != "", true)
	r.Check("PGB_HOST_VULKAN: sharun's own gate is set",
		get(vk, "SHARUN_ALLOW_SYS_VKICD"), "1")
	r.Check("PGB_HOST_VULKAN: mesa is untouched",
		get(vk, "LIBGL_DRIVERS_PATH"), "${SHARUN_DIR}/lib/dri")

	// ⛔ THE EXPENSIVE ONE. It is the only opt-in that REORDERS rather than
	// appends: the host's directories arrive at SHARUN_EXTRA_LIBRARY_PATH,
	// which sharun documents as highest priority.
	gl := HostPolicy{Glibc: true}.EnvLines(full)
	r.CheckBool("PGB_HOST_GLIBC: host dirs reach the highest-priority slot",
		strings.Contains(get(gl, "SHARUN_EXTRA_LIBRARY_PATH"), "/usr/lib"), true)
	r.Check("PGB_HOST_GLIBC: mesa is untouched",
		get(gl, "LIBGL_DRIVERS_PATH"), "${SHARUN_DIR}/lib/dri")

	// ---- the classifier: the acceptance test for a bundle ------------------
	//
	// ⭐ For a static ELF the test is "zero host shared objects". For a bundle
	// it is "no host shared object OUTSIDE the four classes", so the
	// classifier has to be right in BOTH directions — a classifier that
	// answered "nvidia" for everything would make every bundle pass.
	for _, c := range []struct{ path, want string }{
		{"/usr/lib/x86_64-linux-gnu/libGLX_nvidia.so.0", "nvidia"},
		{"/usr/lib/libcuda.so.1", "nvidia"},
		{"/usr/lib/x86_64-linux-gnu/libGL.so.1", "mesa"},
		{"/usr/lib/dri/iris_dri.so", "mesa"},
		{"/usr/lib/libvulkan.so.1", "vulkan"},
		{"/lib/x86_64-linux-gnu/libc.so.6", "glibc"},
		{"/lib64/ld-linux-x86-64.so.2", "glibc"},
		// ⛔ THE NEGATIVES ARE THE POINT. Each of these in a bundle's trace is
		// a bundling defect and must still fail the bundle.
		{"/usr/lib/x86_64-linux-gnu/libcurl.so.4", ""},
		{"/usr/lib/x86_64-linux-gnu/libssl.so.3", ""},
		{"/usr/lib/x86_64-linux-gnu/libQt6Core.so.6", ""},
	} {
		r.Check("class "+c.path, HostClass(c.path), c.want)
	}
	return r
}
