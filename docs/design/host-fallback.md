# What a bundle may take from the host

⛔ **This document overturns a rule this repository applied everywhere.**
`docs/AGENTS.md` §3 criterion 2 — *loads no host shared object* — is the right
acceptance test for a **static ELF** and the **wrong one for a bundle**. The
anylinux stack reaches the host deliberately, for a named and bounded set of
things, and it is right to. `TODO` T-065.

⚠ **What this document does NOT establish, first.**

- ⛔ **No GPU was involved.** Every GL row in this repository is `swrast`
  (`TODO` T-059). Everything below about NVIDIA and Vulkan ICDs is a **code and
  documentation read** of the references, never an execution here.
- ⚠ **The upstream reasoning is quoted rather than re-derived.** These are
  decisions the anylinux maintainers made against real hardware this project
  does not have, and the honest thing is to say whose measurement it is.
- ⚠ **The search order below is implemented and asserted; the per-class
  opt-ins are implemented and only partly asserted** — the classes needing a
  driver cannot be exercised on a machine with none. The assertions are 29
  offline cases in `internal/bundle/hostpolicy_selftest.go`, run by
  `pgb selftest` under the subject `bundle-hostpolicy`; they assert what the
  policy EMITS, which is decidable without hardware. Whether a real driver
  then loads is `TODO` T-059.

## Provenance

| | |
|---|---|
| `pkgforge-dev/Anylinux-sharun` | `src/main.rs`, the library-path assembly at `:230-340` |
| `VHSgunzo/sharun` | `src/main.rs:380-400` (the environment contract), `:1062` (the fallback) |
| `pkgforge-dev/Anylinux-AppImages` | `FAQ.md`, `HALL-OF-FAME.md`, `HOW-TO-MAKE-THESE.md` |
| `pkgforge-dev/cross-libc-dlopen` | `docs/traps.md`, `docs/report/07-closed-source-driver-and-abi.md` |
| `nix-community/nixGL` | the counter-example: it does **not** use the host's GL |

All under `references/`, at the commits their `PROVENANCE.md` files name.

## The rule, in one line

⭐ **Bundled first, host last, and the host is reached only through a
lowest-priority fallback plus a small number of named, per-class opt-ins.**

That is not a compromise between two designs. It is what makes both properties
true at once: the bundle is self-contained by default, and the two things a
bundle genuinely cannot carry — the machine's GPU driver and the kernel's
notion of it — still work.

## Why "zero host objects" is the wrong test for a bundle

A static ELF and a bundle are answering different questions, and the same
measurement means opposite things in each.

| | static ELF (`pgb build`) | bundle (`pgb bundle`) |
|---|---|---|
| what a host `.so` in the trace means | ⛔ **failure.** A second libc entered the process; `docs/limitations.md` §1 | ⚠ **depends entirely on WHICH object.** `libGLX_nvidia.so.0` is correct; `libcurl.so.4` is a bundling bug |
| what the bundle carries | nothing beside the binary | its own glibc, its own `ld.so`, its own gconv tree |
| so a host object binds against | the host's libc — the broken pairing | ⭐ **the bundle's own glibc**, which is the whole point of bundling glibc rather than musl |

⭐ **The second row of the last column is the load-bearing one and it is
upstream's own reason for choosing glibc**, `Anylinux-AppImages/FAQ.md`:

> *"With glibc, we are able to dlopen optional libraries on the host **even
> when those link to musl**. If we used musl the opposite is usually not
> possible as musl lacks a lot of symbols that libraries expect from glibc."*

So the edge resolves **inside** the bundle. That is why `docs/AGENTS.md` §14's
"do not bundle glibc's gconv modules into a STATIC binary" already carries the
exception it does: it does not apply to a bundle carrying its own libc.

## The four things a bundle must take from the host, and why each

⛔ **The list is closed. Anything not on it is a bundling defect**, and the
sweep in `internal/bundle/sweep.go` is what should have caught it.

### 1. ⭐ NVIDIA — always the host's, never bundled

`Anylinux-AppImages/HALL-OF-FAME.md`:

> *"we never need to bundle the NVIDIA drivers, NVIDIA releases its driver
> linking to a +10yo version of glibc, that means we can use that driver
> without issue."*

⭐ **This is the one class where the host copy is unconditionally correct**, and
the reason is an ABI fact rather than a preference: the proprietary driver is
built against a glibc old enough that any bundled glibc satisfies it. It also
cannot be bundled — it is the counterpart of a kernel module the user has
installed, and a stale copy is worse than no copy.

Mechanism: the driver's directories join the search path, and its ICDs are
named in `VK_DRIVER_FILES`. `Anylinux-sharun/src/main.rs` adds
`/etc/libnvidiacurrent` when it exists.

### 2. Mesa and the GL/VA drivers — bundled by default, host by opt-in

⛔ **Bundled is the default and upstream is emphatic about why.** They tried
the other way:

> *"we used to allow our AppImage to use the host vulkan drivers along with the
> bundled drivers, that ended up being a bad idea."*

And the opt-in exists anyway, with its cost stated:

> *"you are not forced to use our bundled drivers always, you can always set
> `USE_HOST_MESA_DRIVERS=1`, this will help if you plan to use the same
> AppImage several years into the future, but it is not guaranteed to work
> forever due to glibc symbol nonsense."*

⭐ **`nixGL` is the control that shows bundling is a complete answer here**: it
does not use the host's GL either — it points nixpkgs' own mesa at itself.

### 3. Vulkan ICDs and layers — bundled by default, host by opt-in

Separate from mesa because the *layers* are the user's, not the
application's: MangoHud and `lsfg-vk` are installed by the person running the
program and cannot be anticipated by the bundle. `sharun` gates this on
`SHARUN_ALLOW_SYS_VKICD=1`.

⚠ **This is also upstream's strongest argument against a fully static build**,
and it is worth quoting because it is an argument this project has to answer
rather than dismiss:

> *"Statically linking everything means we are not able to dlopen any library
> from the host … It means goodbye to the proprietary nvidia driver. It means
> you are no longer able to use vulkan layers like mangohud or lsfg-vk. It
> means you are forever stuck with the version of MESA that was statically
> linked."*

⭐ **T-064 answers the first sentence and not the other three.**
`pgb build --host-dlopen` loads a host shared object from a fully static binary
on 11 of 11 (`experiments/76-`), so "not able to dlopen any library from the
host" is no longer true of this project's static output. ⛔ **But the driver
arguments stand**, and they are not about `dlopen`: a bundled mesa is a
*version* commitment, and a static one is a permanent version commitment. That
is a packaging question, and it is why a bundle exists beside the static ELF
rather than instead of it.

⚠ **Upstream names `solo` — the reference T-064 built from — and rejects it**,
`FAQ.md` "Why not use solo or detour?". Their three objections, and what this
tree can say about each:

| their objection | measured here |
|---|---|
| the host mesa may be older than the app needs | ⭐ **agreed, and not a loader problem.** Nothing in T-064 changes which mesa is installed |
| a statically linked LLVM against a host mesa's different LLVM | ⭐ **real, and `experiments/76-`'s residue shows the shape of it**: `libLLVM` was the one ordinary library that mapped and relocated cleanly and then died in its 605th static constructor. ⭐ **It LOADS now** — the cause was a general-dynamic TLS pair whose two halves searched different sets of objects, so a cross-module thread-local bound to offset 0. `TODO` T-068, closed |
| *"none of the solutions implement `dlmopen`, so you are likely to run into symbol collisions"* | ⛔ **true of `pgb-elfload.c` too and it is stated**: there is one namespace. `--wrap-dlopen` already gives each of a program's own plugins a private namespace with `objcopy --redefine-syms`; the loader does not do the equivalent for host objects |

⭐ **And upstream has since shipped the same idea**: `USE_HOST_DRIVERS_EXPERIMENTAL=1`
in `quick-sharun`, via `cross-libc-dlopen`. So the disagreement is about
defaults and about drivers, not about whether the mechanism is sound.

### 4. A newer host glibc — opt-in, and the narrowest case

`experiments/73-` measured the shape of this from the other side: **class B**,
20 symbols, 14 of them the `__isoc23_*` family, where a host object was built
against a newer glibc than the bundle carries. A bundle pinning an old glibc
and then loading a host object built against a new one hits exactly that.

⛔ **Off by default**, because preferring the host's glibc gives up the property
the bundle exists for. On by name when the alternative is not running at all.

## The search order

⭐ **Highest priority first. This is `Anylinux-sharun/src/main.rs:230-340`'s
order, adopted rather than invented**, with the names `pgb` uses.

| | source | who sets it |
|---|---|---|
| 1 | `SHARUN_EXTRA_LIBRARY_PATH` | the caller, explicitly, for this one run |
| 2 | the mesa path, when `PGB_HOST_MESA=1` selected one | the opt-in |
| 3 | ⭐ **the bundle's own `lib/`** | `pgb bundle`. **This is the default answer for everything.** |
| 4 | `LD_LIBRARY_PATH` | the caller's environment |
| 5 | `/etc/libnvidiacurrent`, when it exists | the host's NVIDIA install |
| 6 | the ordinary host directories, and the dirs named in `/etc/ld.so.cache` | the host |
| 7 | `/run/opengl-driver/lib`, `/run/current-system/sw/lib` | NixOS, which puts drivers nowhere else |
| 8 | ⭐ **`SHARUN_FALLBACK_LIBRARY_PATH` — LAST, and documented as last** | the harness, for a driver directory nothing else found |

⚠ **Row 8 is the one the entry was opened about.** `sharun`'s own help text
calls it *"Fallback library directories with lowest priority"* — the host is
reachable, and it is reachable in the position where it can only ever answer a
question nothing bundled could.

⚠ **Row 7 is not decoration.** `pg83/solo`'s issue #2 was a NixOS
`VkResult -9` fixed by scanning `/run/opengl-driver/share`; the same tracker
notes that *"loading the system's `libvulkan.so.1` dynamically does not save
you either"* there. A search order missing that row fails on NixOS only, which
is the worst kind of missing row.

## What `pgb` does with this

⭐ **Implemented in `internal/bundle/hostpolicy.go`**, emitted into the
bundle's `.env`, and asserted by `internal/bundle/hostpolicy_selftest.go` —
29 cases, offline, run by `pgb selftest`.

| class | default | opt-in | variable |
|---|---|---|---|
| ordinary libraries | ⭐ bundled, host last | always on, lowest priority | `SHARUN_FALLBACK_LIBRARY_PATH` |
| NVIDIA | ⭐ **host, always** | — it is never bundled | `VK_DRIVER_FILES`, `/etc/libnvidiacurrent` |
| mesa / GL / VA | bundled | `PGB_HOST_MESA=1` | `LIBGL_DRIVERS_PATH`, `LIBVA_DRIVERS_PATH`, `__EGL_VENDOR_LIBRARY_DIRS` |
| Vulkan ICDs and layers | bundled | `PGB_HOST_VULKAN=1` | `VK_DRIVER_FILES`, `VK_LAYER_PATH` |
| glibc | bundled | `PGB_HOST_GLIBC=1` | the search order, and it moves the host ahead of the bundle |

⛔ **Every one of those opt-ins is reported, never silent.** `pgb bundle`
prints the policy it applied and `pgb explain` prints it for a build, because
an opt-in that quietly changed which libc a process is running is the failure
mode this whole project is about. T-065's instruction is explicit that a
deferral is reported rather than silent, and an opt-in is a deferral to the
host.

## What this changes about the acceptance test

⛔ **`docs/AGENTS.md` §3 criterion 2 stands unchanged for `pgb build`.** A
static ELF that loads a host shared object has failed, and `experiments/76-`
asserts zero on all eleven.

⭐ **For a bundle the criterion becomes: no host shared object OUTSIDE the four
classes above.** `experiments/86-` and `62-`'s `classify_trace` is the working
instrument; what changes is that its output is read against this list rather
than against zero. A bundle that loads the host's `libcurl` is still a defect
and still fails; one that loads `libGLX_nvidia.so.0` has done the right thing.

⚠ **On this machine that distinction cannot be exercised**, because there is no
GPU and every GL row is `swrast` — so the selftest asserts the ORDER, the
per-class opt-ins and the classifier, all of which are decidable offline, and
the driver behaviour is recorded as unexercised rather than passed. ⛔ The
classifier is asserted in BOTH directions: one that answered "nvidia" for
everything would make every bundle pass, so `libcurl.so.4`, `libssl.so.3` and
`libQt6Core.so.6` are asserted to classify as nothing.
