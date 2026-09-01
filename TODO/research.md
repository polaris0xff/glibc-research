# research — sweeps and prior art

Binding: [`../docs/methodology/references.md`](../docs/methodology/references.md).

---

## T-020 — Sweep the nix-appimage family

**Source** operator, session of 2026-09-01.
**Category** research · **Priority** P1 · **Effort** M · **Status** ✅ done

**Problem.** Nix has the largest package database and `nix bundle` is the
closest existing thing to `pgb build <spec>`. What that family hits is the most
direct evidence available about what T-012 will hit.

**Premise.** Operator's, and ⭐ **confirmed**: these projects get abandoned
because the bundle ends up containing a portable container, or is too large and
too slow without one.

**Closed with.** `docs/research/nix-appimage.md`. Five repositories mined and
kept under `references/`, pinned; plus `pkgforge/soarpkgs`' chromium recipe at
`6f1cbb9b`. The container is **forced** by nix's absolute-path store — the
maintainer says so in `ralismark/nix-appimage` issue #10 — and it costs every
application that sandboxes itself, because `unshare(2)` returns `EPERM` for
`CLONE_NEWUSER` inside a chroot. The chromium recipe's `#Purge Bloatware`
section deletes locales and then **symlinks them back to the host**.

⛔ **Gap, recorded:** discussions were not fetched for any of them — GraphQL
only, no credential-free route. Nothing was executed; these are code and
tracker reads.

## T-021 — Build one nix-appimage and run it on the matrix

**Source** follow-on from T-020 · **Category** research · **Priority** P2 · **Effort** M · **Status** open

**Problem.** T-020's claims about what nix-appimage *costs* come from its
tracker, which is evidence of intent and never of behaviour.

**Approach.** `nix bundle` the same subject `experiments/60-` uses, run it on
all 11 with the `62-` instrument. ⚠ Needs nix on the build host, which this
environment does not have.

**Prove.** `evidence/64-nix-appimage/RESULT.txt` with the coverage and
host-object columns filled, comparable to `60-` and `62-`.

## T-022 — Spike a nixpkgs front end for the planner

⭐ **RULED ON BY THE OPERATOR, 2026-09-01b: IN SCOPE.** The open question this
entry existed to settle — *"does depending on nix defeat the point?"* — is
answered. `../docs/design/nix-front-end.md` records the ruling verbatim, the
two reference recipe shapes at their pinned commit, the "use nix without
installing nix" reading list, and what the next session owes before writing
anything. ⛔ **The mining is the FIRST task of the next session** and this
entry should be re-scoped from it, not from the text below.

**Source** follow-on from T-020 · **Category** research · **Priority** P2 · **Effort** M · **Status** open

**Problem.** T-012's planner needs a dependency graph. Building one from distro
metadata is strictly harder than reading one that already exists.

**Premise.** ⭐ Every nix derivation names its inputs exactly. Taking the graph
while refusing the store layout is the interesting move.

**Prove.** For one package, `pgb` prints the transitive input list obtained
from nix, and each entry marked link-statically / build-static / bundle.

## T-052 — The libGL problem, and whether a bundle can honestly claim "universal"

**Source** ⭐ **operator, 2026-09-01c**: *"I also remember an infamous 'libGl'
problem with the nixappimages created, which further killed the dream of a
'universal' builder as many graphical apps didn't work. See:
https://github.com/nix-community/nixgl"*
**Category** research · **Priority** P1 · **Effort** M · **Status** ✅ done

## ⭐ THE MESA HALF IS SOLVED, AND MEASURED

⛔ **The mechanism, which is not what the name suggests.** A nixpkgs GL
program does not depend on mesa at all. It depends on **libglvnd**, the
vendor-neutral dispatch layer, and libglvnd finds the implementation by reading
`share/glvnd/egl_vendor.d/*.json` and `dlopen`ing what they name. That file is
HOST configuration, so mesa is not in the closure — mesa-demos' 111-path
closure carries libglvnd and mesa-libgbm and **not one mesa driver**.

⭐ **That makes the libGL problem this project's own `docs/limitations.md` §1
arriving from the GL side: a dlopen of an object the host is supposed to
provide.** And `nix-community/nixGL` (commit `b6105297`, `nixGL.nix:54-62`)
answers it the same way this project now does: for the mesa case it does NOT
use the host's GL, it points nixpkgs' own mesa at itself with
`LIBGL_DRIVERS_PATH`, `GBM_BACKENDS_PATH` and `__EGL_VENDOR_LIBRARY_*`.

**Landed in `tool/nix-appimage.sh`**, in three parts, each because a real run
failed:

1. **mesa is pulled into the closure** when libglvnd is present and no driver
   is. Before: `eglinfo: eglInitialize failed`, no vendor at all.
2. **`lib/dri`, `lib/gbm` and the other module trees are copied as
   DIRECTORIES.** They are found through a variable naming a directory, so a
   flattened copy is invisible.
3. ⛔ **The ICD JSONs name an absolute store path** —
   `"library_path": "/nix/store/4cvv9...-mesa-26.2.1/lib/libEGL_mesa.so.0"` —
   so libglvnd found the vendor file, opened a path that was not there, and
   failed on a bundle with `libEGL_mesa.so.0` sitting in `lib/` beside it.
   Rewritten to the bare soname.

**Measured, on a machine with NO GPU:**

```
EGL vendor string: Mesa Project
EGL driver name:   swrast
EGL client APIs:   OpenGL OpenGL_ES
```

from `eglinfo` in a 163 MB bundle, surfaceless platform, no host GL involved.

## ⛔ What this entry did NOT settle, and who owns each part

- **NVIDIA proprietary**, and **GL against a real DRM device** — **T-059**,
  which exists so neither is closed by silence.
- **163 MB**, because mesa is 273 MB unstripped and nothing debloats it. The
  Anylinux flow ships a debloated mesa; **T-057** owns that, and `85-` gives
  it the number to beat: the GL stack is 95 MiB of the bundle.

**The problem, as it was stated before it was measured.** OpenGL and Vulkan are the one
class of library that **must** come from the host, because it is the host's
kernel driver that the userspace half has to match. Bundling a `libGL.so` from
nixpkgs and running it against another machine's GPU driver is not the same
kind of portability as bundling `libpng`. ⚠ Everything in this paragraph is a
statement of the problem, not a measurement, and this entry exists to replace
it with one.

**What the mining already gives us.**
- `nix-community/nixGL` exists for exactly this and wraps a program so that
  the host's GL implementation is found. It is the inverse shape to ours.
- The `Anylinux-AppImages` flow bundles a **debloated mesa** and has
  `DEPLOY_OPENGL`/`DEPLOY_VULKAN` on by default, `SHARUN_MESA_PATH` to point
  at an external mesa at run time, and `SHARUN_NO_NVIDIA_EGL_PRIME` /
  `SHARUN_ALLOW_SYS_VKICD` for the nvidia and ICD cases
  (`references/pkgforge-dev__Anylinux-sharun`, README). ⭐ **So the project the
  operator points at as the quality bar has already decided this is bundled
  mesa plus escape hatches**, not host GL.

**Approach**, as it was written before the run. All three steps were taken;
step 3's answer is bundled mesa, which is what `85-` measures and what the
Anylinux flow had already chosen.

**Prove.** `evidence/85-opengl/RESULT.txt` with a row per environment and per
strategy, and the GPU-less caveat stated in the file rather than inferred.

## ✅ CLOSED on that acceptance, 2026-09-01d — `experiments/85-opengl.sh`

**7 assertions, 0 failures, 0 skips.** Two strategies, eleven environments:

| arm | what it is | result |
|---|---|---|
| **A** bundled mesa | the closure augmented with nixpkgs' own mesa, `LIBGL_DRIVERS_PATH`/`GBM_BACKENDS_PATH`/`__EGL_VENDOR_LIBRARY_DIRS` pointed at itself, ICD JSONs rewritten off their absolute store paths | ⭐ **`EGL vendor string: Mesa Project`, `EGL driver name: swrast`, on 11 of 11** |
| **B** `--no-gl` | the identical closure with mesa left out — which is what a nix-appimage of a GL program is today | ⛔ **no vendor on any of the eleven** |

```
  ok    arm A: surfaceless EGL reports Mesa on every environment = 11
  ok    arm A: a driver is named on every environment            = 11
  ok    arm A: every target agrees with the build host's exit    = 11
  ok    arm A loaded no host shared object                       = 11
  ok    arm B (no bundled mesa) reported NO vendor anywhere      = 0
```

⭐ **Arm B is what makes arm A a measurement.** Alone, `EGL vendor string:
Mesa Project` is a program printing a string. Measured: arm B's **EGL client
extensions string is empty** and it reports one `Default display platform:
eglInitialize failed` — libglvnd with no vendor does not know the surfaceless
platform extension exists, so it cannot even enumerate the platform that works
in arm A. The vendor came from the bundle.

⚠ **The exit status is not the measurement, and reading it as one would have
failed a correct result.** `eglinfo` walks every platform it was built for and
returns the **number that failed to initialise** — 3 here (GBM, Wayland, X11),
on every row *and on the build host*, because none of the eleven has a display
or a DRM node. The assertion is therefore that every target agrees with the
build host, measured rather than assumed.

**The cost column, for T-057:** the GL stack is **95 MiB** of the 163 MB
bundle (arm A 170,610,343 B, arm B 70,568,698 B).

⛔ **What this does NOT establish is in the evidence file itself, not in a
footnote:** no GPU is present, so every row is software rasterisation; nothing
is drawn to a screen; NVIDIA is untouched. **T-059 carries that half** — the
entry does not close it by silence.

## T-053 — patchelf and patsh: use them, or say why not

**Source** operator, 2026-09-01c: *"did we have no need to use tools like
patchelf & patsh here?"*
**Category** research · **Priority** P2 · **Effort** S · **Status** open

**Where it stands.** `tool/elf-needed.py` does ONE edit — rewrite an absolute
`DT_NEEDED` to its basename, in place, at the same `.dynstr` offset — because
that is the single edit `tool/nix-appimage.sh` needed and patchelf is not on
this machine. That is a reason for the sixty lines, not an argument against
patchelf.

⭐ **And the ground has moved since: `scripts/common/nix-fetch.sh` can now
fetch patchelf's own closure from cache.nixos.org**, so "not installed" is no
longer a blocker for either tool.

**The two questions, and they are different.**
- **patchelf** does what `elf-needed.py` does and much more (interpreter,
  RPATH, soname, shrinking). ⚠ It is also famous for producing binaries that
  break in subtle ways when it has to grow a section, which is exactly why
  `elf-needed.py` refuses to move anything. Decide per edit, not per tool.
- **patsh** patches store paths in **shell scripts**, which is the gap
  `tool/nix-appimage.sh` currently *names and does not fill*: a nixpkgs
  `bin/x` that is a wrapper script is followed to its ELF and the wrapper's
  environment is dropped. ⭐ That is a real hole in the bundler and patsh is
  aimed straight at it.

**Prove.** A written comparison at file and line, plus either a patsh-shaped
step in the bundler with a wrapper-using application working through it, or
the measurement that says why it is not needed.

## T-057 — The bundler: a maintained nix-appimage descendant, on the Anylinux mechanisms

**Source** ⭐ **operator, 2026-09-01c**, and it is the second of three goals:
*"make the 'universal' bundler true via a modern, updated, maintained
'nixappimage' descendant that uses or rather reimplements many of the anylinux
tooling, iterating/improving them, and debloating nixappimages, correctly
packing them, and also solving the opengl problem"*.
**Category** research · **Priority** P1 · **Effort** L · **Status** ⚠started

**Landed already.** `tool/nix-appimage.sh` builds one: uruntime + dwarfs +
sharun instead of appimage-type2-runtime + mksquashfs + a bwrap AppRun, with
the nixpkgs **closure** replacing sharun's ldd-and-strace library discovery.
galculator 2.1.4 reaches GTK's own "cannot open display" on alpine 3.22,
voidlinux (both musl), debian 11 and arch.

⛔ **What it does NOT do yet, each of which is between it and the bar:**

1. **No debloating at all.** galculator is 64 MB from a 315 MB closure and the
   Anylinux flow would strip locales, docs, static archives and unused
   drivers. ⚠ Their chromium recipe deletes locale directories and then
   **symlinks them back to the host**, which is a trade to copy deliberately
   or reject deliberately, not to miss.
2. **OpenGL** — T-052, and for a video editor it decides whether it runs.
3. **Wrapper scripts.** A nixpkgs `bin/x` that is a shell wrapper is followed
   to its ELF and the wrapper's ENVIRONMENT is dropped. T-053; `patsh` is
   aimed at exactly this.
4. **No 32-bit path.** `lib32` exists in the Anylinux layout and not here.
5. **Nothing is measured against a hand-built Anylinux AppImage.**
   `experiments/62-` compares `pgb` against one; nothing compares OUR bundle
   against one.

⭐ **The claim worth making, stated so it cannot drift:** not "as good as a
hand-crafted AppImage" but *"produced by one command from a package name, and
within measurable distance of one"*. `docs/AGENTS.md` §14 governs the
wording.

**Prove.** `evidence/86-bundler-vs-anylinux/RESULT.txt`: the same application
as an Anylinux AppImage and as ours, on all eleven, with size, startup, and
host-object columns — the instrument in `experiments/62-` already produces
three of those four.

## ⭐ THE DISTANCE IS MEASURED — `experiments/86-bundler-vs-anylinux.sh`

**7 assertions, 0 failures, 0 skips**, 2026-09-01d. Subject `jq 1.8.2`, the
same release on both sides by luck rather than design.

| | arm P — **ours, one command** | arm A — hand-built Anylinux |
|---|---|---|
| how | `sh tool/nix-appimage.sh jq` | install the distro package on Arch, `quick-sharun.sh`, `--make-appimage` |
| size | **12,230,824 B** (7 store paths) | **4,006,946 B** (68 libraries) |
| cold start | 162–198 ms | 79–107 ms |
| warm start | 11–22 ms | 9–16 ms |
| runs on the eleven | **11 of 11** | **11 of 11** |
| host shared objects | **0 on every row** | **0 on every row** |

⭐ **So the claim T-057 set out to make is supported and quantified**: not
*"as good as a hand-crafted AppImage"* but *"produced by one command from a
package name, and within measurable distance of one"* — **3.05× the size,
about 1.9× the cold start, about 1.4× the warm start**, and identical on the
two columns that decide whether it works at all.

⛔ **And the size ratio is item 1 of this entry, not a mystery.** Nothing is
debloated. The Anylinux flow strips locales, docs, static archives and unused
drivers; ours ships the closure as nixpkgs built it. `experiments/85-` gives
the other half of the number: on a GL application the undebloated mesa is
**95 MiB** of a 163 MB bundle.

⛔ **The startup instrument had to be rewritten, and the first version measured
itself.** It timed one chroot enter per invocation and reaped the rootfs after
each — which kills uruntime's dwarfs FUSE daemon, so **every** run paid a cold
mount and both arms came out at ~14,500 ms. The same artefact starts in 162 ms
cold and **17 ms** warm on the build host, where nothing reaps. A "warm" column
850× the real figure is not a slow measurement, it is the wrong one. Cold is
now one enter with the mount reaped *before*; warm is `(six invocations −
cold) / 5` inside **one** enter with the mount left alive, which is what a real
user gets.

**What the entry does NOT claim**, stated in the evidence file rather than
here: the two arms are different distributions' builds; jq is a CLI, so this
says nothing about a GUI (that is `85-`); nothing is debloated; one machine,
one day.

**What is left of this entry** — it stays **open**, with items 1, 3 and 4
untouched: debloating (item 1, now with a number to beat), wrapper scripts and
their environment (item 3, T-053, `patsh`), and the 32-bit path (item 4).
Item 5 — *"nothing is measured against a hand-built Anylinux AppImage"* — is
what closed here.

## T-059 — GL on real hardware, and the NVIDIA case

**Source** split out of T-052 when it closed on its own acceptance,
2026-09-01d. ⭐ It is the operator's own open question 3 from the previous
session — *"a GPU. T-052 cannot be honestly closed on a machine with no
graphics hardware"* — given an entry of its own rather than left inside a
closed one.
**Category** research · **Priority** P1 · **Effort** M · **Status** open

**Blocked on hardware, and that is stated rather than worked around.**
`experiments/85-` shows that a bundle carries a complete GL stack that
initialises and names its driver on eleven distributions with none of their
own. It shows nothing about **iris, radeonsi, amdvlk or NVIDIA**, because this
machine has no GPU and `swrast` is what it can reach.

**Two questions, and they are different.**

1. **Bundled mesa against a real DRM device.** The mechanism is the same one
   85- proves; what is untested is whether a bundled mesa of one version
   drives a kernel DRM driver of another. ⚠ mesa's userspace/kernel contract
   is far looser than NVIDIA's, so the expectation is that it works — but an
   expectation is not `evidence/`.
2. ⛔ **NVIDIA proprietary, where the userspace half MUST match the running
   kernel module.** `nix-community/nixGL` reads `/proc/driver/nvidia/version`
   and **fetches** a matching driver (`nixGL.nix:69`). A bundle cannot fetch
   at run time, so the honest options are: detect the host's NVIDIA userspace
   and use it (which reintroduces `docs/limitations.md` §1 deliberately),
   carry several and pick, or say the case is unserved. ⭐ The Anylinux flow
   has already chosen: `SHARUN_NO_NVIDIA_EGL_PRIME` and
   `SHARUN_ALLOW_SYS_VKICD` are escape hatches to the host's, which is the
   first option with a switch on it.

**What can be done here without hardware**, and should be, before anyone waits
for a GPU: implement the host-NVIDIA detection path and assert that it finds
**nothing** on all eleven — the negative half of the measurement is available
now and it is the half that says the detection code runs at all.

**Prove.** A row per environment for a machine that has a GPU, with the vendor
and renderer strings, plus the detection path exercised on the eleven that do
not.
