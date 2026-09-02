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
all 11 with the `62-` instrument. ⚠ Needs nix on the build host — `pgb
bootstrap` installs it, so this is no longer the blocker it was.

**Prove.** `evidence/64-nix-appimage/RESULT.txt` with the coverage and
host-object columns filled, comparable to `60-` and `62-`.

## T-022 — Spike a nixpkgs front end for the planner

**Source** follow-on from T-020 · **Category** research · **Priority** P2 · **Effort** M · **Status** ✅ done

⭐ **CLOSED: the spike shipped.** The question it existed to settle — *"does
depending on nix defeat the point?"* — was ruled in scope by the operator
(2026-09-01b) and the front end is now `internal/nixx`: `pgb nix plan`, `deps`,
`fetch` and `build`, designed in `../docs/design/nix-front-end.md` and proven
end to end by `experiments/88-`, which plans, fetches **and builds** a nixpkgs
package with no nix and no root, 25 assertions.

⚠ **One nuance of the Prove line is NOT met and moves to T-012**, which owns the
planner: `pgb nix deps` prints the transitive input list and marks each entry
with its build **outcome** (ok / skipped / failed), not with the disposition
the line asked for (link-statically / build-static / bundle). The
static-first/bundle-last rule those three words come from is in
`../docs/design/toolchain.md`.

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

**Landed in `internal/bundle/appimage.go`**, in three parts, each because a real run
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
**Category** research · **Priority** P2 · **Effort** S · **Status** done

**Where it stands.** `internal/elfx/needed.go` does ONE edit — rewrite an absolute
`DT_NEEDED` to its basename, in place, at the same `.dynstr` offset — because
that is the single edit `internal/bundle/appimage.go` needed and patchelf is not on
this machine. That is a reason for the sixty lines, not an argument against
patchelf.

⭐ **And the ground has moved since: `internal/nixx/fetch.go` can now
fetch patchelf's own closure from cache.nixos.org**, so "not installed" is no
longer a blocker for either tool.

**The two questions, and they are different.**
- **patchelf** does what `internal/elfx/needed.go` does and much more
  (interpreter, RPATH, soname, shrinking). ⚠ It is also famous for producing
  binaries that break in subtle ways when it has to grow a section, which is
  exactly why `internal/elfx/needed.go` refuses to move anything. Decide per
  edit, not per tool.
- **patsh** patches store paths in **shell scripts**, which is the gap
  `internal/bundle/appimage.go` currently *names and does not fill*: a nixpkgs
  `bin/x` that is a wrapper script is followed to its ELF and the wrapper's
  environment is dropped. ⭐ That is a real hole in the bundler and patsh is
  aimed straight at it.

**Prove.** A written comparison at file and line, plus either a patsh-shaped
step in the bundler with a wrapper-using application working through it, or
the measurement that says why it is not needed.

## ⭐ CLOSED — the hole is filled, and patsh is not what fills it

**patchelf**: `internal/elfx/needed.go` keeps its sixty lines and the reason is now
measured rather than asserted. It does ONE edit — rewrite an absolute
`DT_NEEDED` to its basename **at the same `.dynstr` offset** — and refuses to
move anything, which is exactly the case patchelf is famous for getting wrong.
`internal/nixx/fetch.go` can fetch patchelf's closure, so availability is
no longer the argument; the argument is that the bundler's only ELF edit is the
one edit that needs no section growth. ⚠ **Revisit per edit, not per tool**: an
RPATH or interpreter rewrite is patchelf's job and `internal/elfx/needed.go`
must not grow one.

⛔ **patsh: no, and the reason is that nixpkgs no longer produces the thing it
patches.** patsh rewrites `/nix/store` paths **in shell scripts**. Two
measurements against that:

1. **The dominant wrapper is a compiled C program.** `mpv-with-scripts-0.41.0`'s
   `bin/mpv` is a **16,560-byte ELF** — nixpkgs' current `makeWrapper` emits
   `makeBinaryWrapper` output. There is no script to patch.
2. **A bundle does not run the wrapper at all.** sharun runs the real ELF and
   reads `.env`. Keeping a script working is the wrong end of the problem; the
   job is to **lift the assignments out** and re-express them against the
   bundle.

⭐ **And the binary wrapper makes that exact rather than heuristic**, because
nixpkgs embeds the generating command in the binary's own data:

    makeCWrapper '/nix/store/...-mpv-0.41.0/bin/mpv' \
        --inherit-argv0 \
        --prefix 'LUA_CPATH' ';' '/nix/store/...-lua-5.2.4-env/lib/lua/5.2/?.so' \
        --suffix 'PATH'     ':' '/nix/store/...-yt-dlp-2026.08.19/bin'

`internal/bundle/wrapper.go` reads both shapes (11-check selftest, including two
refusal cases), and `internal/bundle/appimage.go` copies each referenced store path
into `AppDir/store/<name>/` — keeping its internal layout, because `LUA_CPATH`
names a sub-path — and writes `.env` lines with the same prefix/suffix/set
semantics.

⛔ **THE DEFECT THIS FOUND, and it shipped a broken bundle.** `resolve_entry`
tested the **ELF magic first**, so a binary wrapper was declared to be the
program and packed. The AppImage then failed with

    Failed to run App: /tmp/.mount_mpv-.../bin/: Permission denied (os error 13)

— which reads like a bug in sharun. It asks whether the file is a *wrapper*
before asking whether it is an ELF now.

**Measured** with the shell bundler this predates, now `pgb bundle appimage`
and retired to `HISTORY/`: `sh tool/nix-appimage.sh mpv` reports
`bin/mpv is a nixpkgs wrapper -> mpv` and `4 variable(s) lifted out of the
wrapper into .env`, and the artefact runs: `mpv v0.41.0`, decoding an ffmpeg
lavfi test pattern with `VO: [null] 64x48 rgb24`.

## T-057 — The bundler: a maintained nix-appimage descendant, on the Anylinux mechanisms

**Source** ⭐ **operator, 2026-09-01c**, and it is the second of three goals:
*"make the 'universal' bundler true via a modern, updated, maintained
'nixappimage' descendant that uses or rather reimplements many of the anylinux
tooling, iterating/improving them, and debloating nixappimages, correctly
packing them, and also solving the opengl problem"*.
**Category** research · **Priority** P1 · **Effort** L · **Status** open

**Landed already.** `internal/bundle/appimage.go` builds one: uruntime + dwarfs +
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
| how | `sh tool/nix-appimage.sh jq` (the retired shell bundler; `pgb bundle appimage` now) | install the distro package on Arch, `quick-sharun.sh`, `--make-appimage` |
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

## ⭐ ITEMS 1, 3 AND 4 ARE DONE — `experiments/89-debloat.sh`, 10 assertions, 0 fail

### Item 1, debloating: three arms and a control on all eleven

⛔ **What the 95 MiB actually is**, measured on `mesa-26.2.1` rather than
assumed. Its `lib` is **273 MB** uncompressed and the GL driver is a minority
of it: **twelve Vulkan ICDs are ~194 MiB**, and `libteflon.so` (12.1 MiB) is an
NPU delegate rather than a GPU driver at all.

⭐ **Seven of the twelve are for GPUs that cannot exist on x86_64 Linux** —
panfrost is ARM Mali, freedreno is Adreno, broadcom is a Raspberry Pi, asahi is
Apple silicon, powervr is Imagination, dzn is Direct3D 12 on Windows, gfxstream
is an Android emulator transport. Dropping those is not a size/function trade.

| arm | AppDir | AppImage | |
|---|---|---|---|
| `--debloat none` | 878,044,217 | 170,610,180 | what 85- and 86- measured |
| ⭐ `--debloat safe` (default) | 676,429,749 | **147,196,846** | 0.77× / **0.86×** |
| `--debloat aggressive` | 580,947,048 | **132,874,316** | 0.66× / **0.78×** |

The rules that fired on the safe arm, each with its reason:

    1.7 MiB   5  documentation and shell completions
    0.0 MiB   7  static archives, headers and build metadata
   87.5 MiB 188  locale catalogues (kept: none)
   17.7 MiB   2  vulkan driver 'panfrost'   (no such GPU on x86_64)
   16.3 MiB   2  vulkan driver 'freedreno'  (no such GPU on x86_64)
   15.8 MiB   2  vulkan driver 'asahi'      (no such GPU on x86_64)
   13.0 MiB   2  vulkan driver 'powervr'    (no such GPU on x86_64)
   12.6 MiB   2  vulkan driver 'broadcom'   (no such GPU on x86_64)
   12.6 MiB   2  vulkan driver 'dzn'        (no such GPU on x86_64)
    3.0 MiB   2  vulkan driver 'gfxstream'  (no such GPU on x86_64)
   12.1 MiB   1  libteflon (an NPU delegate, not a GPU driver)

⭐ **AND THE CONTROL IS WHAT MAKES THE SIZE COLUMN REPORTABLE.** All three arms
run on all eleven environments, and the debloated ones must reach **the same
EGL vendor and the same driver as the undebloated one on every row**: 11 of 11
`Mesa Project` / `swrast`, **zero host shared objects**, and 11 of 11 agreement
for *both* the safe and the aggressive arm. A smaller bundle that stopped
working is the obvious way to get a good size number, so the number is only
printed next to that check.

⚠ **The aggressive arm is a real trade and is not the default.** It also drops
the Vulkan ICDs for GPUs this architecture *does* have. `eglinfo` is an
OpenGL/EGL program and never touches them, so eleven green rows say the trade
cost nothing **here** — they say nothing about a Vulkan application.

⚠ **The compressed saving is much smaller than the uncompressed one** — 23% off
the AppDir but 14% off the AppImage — because locales and driver blobs are
exactly what zstd is best at. Quoting the AppDir figure alone would overstate
what a user downloads.

⛔ **And a nix-appimage-scale caveat: two more rules were considered and not
taken.** The Anylinux chromium recipe deletes locale directories and then
**symlinks them back to the host**; that is a deliberate reintroduction of a
host dependency and is refused here — `--keep-locales` is the knob instead.
Stripping symbols from every shared object was not attempted and is the next
thing to measure.

### Item 3, wrapper scripts: **T-053**, and patsh is not the answer

nixpkgs' current `makeWrapper` emits a **compiled C program** — mpv's `bin/mpv`
is a 16,560-byte ELF — so patsh has no script to patch, and a bundle does not
run the wrapper anyway. `internal/bundle/wrapper.go` reads the environment out of both
wrapper shapes and `internal/bundle/appimage.go` re-expresses it against the bundle.
Full write-up in T-053.

### Item 4, lib32

Shared objects are placed by **ELF class** (byte 5 of the header) — 32-bit into
`lib32`, with its own loader and a `shared/lib32` symlink. Before this, an
i386 and an x86_64 `libfoo.so.1` landed in one flat directory where the second
`cp -n` silently lost. ⚠ **Not exercised by a 32-bit application**: nothing in
the eleven-environment bed carries one, so what is proved is the classification
and the layout, not a running 32-bit program. That is the next thing to try.

### ⛔ Two defects the debloat pass itself required

- **The symlink pass was order-dependent.** A closure has symlinks to symlinks
  — `libvulkan.so -> libvulkan.so.1 -> libvulkan.so.1.4.357` — and one pass
  left a `DT_NEEDED` on `libvulkan.so` resolving nowhere. It repeats until it
  creates nothing.
- ⭐ **An integrity pass now runs after the debloat**: every `DT_NEEDED` of
  every ELF left in the bundle must resolve inside it. That check is what
  caught the one above, and without it a debloat is a size number with no
  safety property behind it.

## ⭐ AND THE COMPARISON IS RE-RUN AGAINST A REAL APPLICATION

⛔ **The operator's ruling on 2026-09-01e:** *"experiments/86- compared jq. jq
is two shared libraries. Comparing bundlers on jq measures nothing about
bundling."* Correct. `PGB_VS_APP=mpv` re-runs it against a subject whose
closure is **297 store paths and 1.2 GB** — ffmpeg, libplacebo, libass, mesa,
lua — and whose `bin/mpv` is a **nixpkgs wrapper** rather than the program.

**7 assertions, 0 failures**, `evidence/86-bundler-vs-anylinux/RESULT.mpv.txt`:

| | arm P — ours, one command | arm A — hand-built Anylinux |
|---|---|---|
| size | **221,623,798** B (297 paths, `--debloat safe`) | **81,849,864** B (475 libraries) |
| ratio | **2.71×** (jq, undebloated, was 3.05×) | |
| cold start | 1,239–1,549 ms | 631–735 ms |
| warm start | **70–210 ms** | 92–119 ms |
| runs on the eleven | **11 of 11** | **11 of 11** |
| host shared objects | **0 on every row** | **0 on every row** |

⭐ **The functional test is a real decode**, not `--version`: ffmpeg's lavfi
generates a test pattern, libavcodec decodes it and the null video output
reports `64x48 rgb24`. A bundle that starts and cannot reach its own
libavcodec fails that and passes `--version`.

⭐ **And the warm-start column has changed shape.** On jq ours was ~1.4× the
hand-built one; on mpv the two are **within each other's spread** — 70 ms
against 97 ms on alpine 3.22, 112 against 114 on Arch. ⚠ `experiments/40-`'s
noise floor applies and the honest reading is *"no difference measurable on
this subject"*, not that ours is faster.

⛔ **Cold start is still about 2× and that is not noise.** 297 store paths of
dwarfs to mount against 475 flat libraries; it is the size row arriving in the
time column.

⛔ **A record defect this run caused and fixed.** The experiment is
parameterised by `$PGB_VS_APP` and its evidence directory was not, so running
it against mpv **overwrote the jq run's `per-environment.txt`**. Evidence files
are per subject now.


## ⭐ AND THREE MORE MECHANISMS THE kdenlive CASE REQUIRED

⛔ **A wrapper is not the only way a program learns where its plugins are.**
MLT bakes its module directory into `libmlt` at build time and nixpkgs does not
wrap it, because in nixpkgs the store is there. In a bundle it is not, and
`melt` **started, answered `-version`, and could not render**:

    mlt_repository_init: no plugins found in
      "/nix/store/2zh…-mlt-7.40.0/lib/mlt-7"

⭐ So the bundler now **scans every packed program and library for embedded
`/nix/store/<hash>-<name>/…` strings** — 735 of them for kdenlive, 532 naming a
real directory — and carries the ones a documented variable can redirect
(`MLT_REPOSITORY`, `MLT_DATA`, `FREI0R_PATH`, `LADSPA_PATH`,
`GST_PLUGIN_SYSTEM_PATH_1_0`, `BABL_PATH`, `GEGL_PATH`). ⚠ **Detection is
cheap; carrying is not**: the first version copied every baked directory that
existed and the artefact went from 354 MB to **539 MB** for two variables'
worth of benefit, because those directories are `lib/` trees already flattened
into the bundle's own `lib/`. What is not carried is counted, so the gap is
visible without being paid for.

⛔ **The store shard copied whole packages when a variable named one
directory.** `QT_PLUGIN_PATH=…/qtdeclarative-6.11.1/lib/qt-6/plugins` pulled in
all **190 MB** of qtdeclarative. The shard was **947 MB of a 1.2 GB `lib/`** —
most of it a second copy of the same files. Copying only the named
subdirectory: **539 MB → 398 MB**.

⭐ **`--with-program NAME` carries a helper from anywhere in the CLOSURE**, not
just the entry package's own `bin/`. kdenlive renders by running `melt`, which
is mlt's binary; a bundle that cannot render is not comparable to one that can.

⛔ **AND A MULTI-PROGRAM BUNDLE COSTS THE HOST'S `/bin/sh`.** Carrying several
programs needs a selector, a selector is a shell script, and a script is run by
the **host's** interpreter, which loads the host's libc. Measured in
`experiments/90-`: 1–4 host shared objects on every glibc row, none on the four
musl ones. ⭐ `pkgforge-dev/kdenlive-AppImage-Enhanced` pays the same price —
its `AppRun.sh` is a shell script too, and it opened **10** on Rocky 8 where
ours opened 3. ⚠ The shebang cannot point into the bundle: the mount path is
not known until run time. So the shell is taken **only when there is more than
one program**, and a single-program bundle keeps `AppRun` as a hardlink of
sharun — which is why 85-, 86- and 89-'s zero-host-object rows are unaffected.

⛔ **And `mkdwarfs` failures now print `mkdwarfs`'s own error.**
`die "mkdwarfs failed"` with the output on `/dev/null` is four words for twenty
minutes of work; measured when the disk filled during a kdenlive pack.

**What is left of this entry** — it stays **open** for one thing only: the
32-bit path has no 32-bit application behind it.

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

---

## T-065 — ⛔ P0: anylinux dlopens the HOST on purpose. Restudy why, and adopt the policy

**Source** ⭐ **operator, 2026-09-02b**: *"anylinux deliberately dlopens on
hosts and uses their libs because that's the way it is allowed to use stuff
like nvidia drivers, it only dlopens when it needs to and host has what it
needs, so restudy all of anylinux reference materials"*.
**Category** research · **Priority** P0 · **Effort** L · **Status** done

⛔ **WORK UNTIL IT IS MET OR THE PREMISE IS SIGNIFICANTLY ADVANCED.**

**Problem.** This project treats **any** host `.so` entering a process as the
failure it exists to prevent, and asserts on it in `poc_matrix` and
`pgb verify`. ⛔ **That is right for a static ELF and wrong for a bundle**, and
the difference has never been written down. A bundle that refuses the host's
GL, Vulkan or NVIDIA userspace cannot run on real hardware at all — which is
why every GL row in this tree is `swrast`.

**Premise, read out of the vendored source rather than assumed.** anylinux's
sharun (`references/pkgforge-dev__Anylinux-sharun/tree/src/main.rs`) implements
a **priority-ordered library search**, not a wall:

    SHARUN_EXTRA_LIBRARY_PATH      highest priority
    (the bundle's own lib/)
    SHARUN_FALLBACK_LIBRARY_PATH   "lowest priority" -- main.rs:45

plus explicit opt-ins for the cases where the host **must** win:
`SHARUN_USE_HOST_GLIBC`, `SHARUN_MESA_PATH`, `SHARUN_ALLOW_SYS_VKICD`,
`SHARUN_NO_NVIDIA_EGL_PRIME`, `SHARUN_ALLOW_LD_PRELOAD`,
`SHARUN_ALLOW_QT_PLUGIN_PATH`.

⭐ **And the maintainer states the rule in their own tracker**
(`references/VHSgunzo__sharun/api/issues.json`): sharun compares the host's
`libc.so.6 --version` against the bundled one — *"300 microseconds"* — and
uses the host's loader and libc **only when the host's is newer**, by
symlinking them into a temp dir and putting it first on `--library-path`.
⛔ **So the policy is: bundle by default, defer to the host when the host has
something the bundle cannot carry, and never silently.**

**Approach.**
1. **Read the family properly**: `pkgforge-dev__Anylinux-sharun`,
   `pkgforge-dev__Anylinux-AppImages`, `VHSgunzo__sharun`,
   `VHSgunzo__runimage`, `nix-community__nixGL`, and the trackers. Write the
   policy up as a table of *what defers to the host, when, and how it is
   detected* — this is a `docs/research/` page, not a paragraph.
2. **Separate the two claims in this tree's own instruments.** "Zero host
   objects" stays the assertion for a **static ELF**; for a **bundle** the
   assertion becomes "no host object the bundle could have carried", with the
   driver classes named and allowed.
3. **Implement the search order** in `internal/bundle`: bundled first, host
   fallback last, per-class opt-ins, and ⛔ **every deferral reported**, never
   silent.

**Prove.** A bundle that runs GL against a **real** driver rather than
`swrast` — the T-059 hardware case — while still loading nothing from the host
it could have carried; the policy table in `docs/research/`; and
`experiments/85-`/`89-` re-run so the eleven rows distinguish "carried" from
"deferred by policy" instead of counting both as contamination.

## ✅ Done — [`../docs/design/host-fallback.md`](../docs/design/host-fallback.md), and the mechanism

⭐ **The policy is written up and implemented.** `internal/bundle/hostpolicy.go`
emits it into the bundle's `.env`; 29 offline selftest cases assert it
(`pgb selftest`, subject `bundle-hostpolicy`).

⭐ **The search order is ADOPTED, not invented** —
`Anylinux-sharun/src/main.rs:230-340`, highest priority first:

    1  SHARUN_EXTRA_LIBRARY_PATH        the caller, this run
    2  the mesa path, if PGB_HOST_MESA  the opt-in
    3  ⭐ THE BUNDLE'S OWN lib/          the default answer for everything
    4  LD_LIBRARY_PATH                  the caller's environment
    5  /etc/libnvidiacurrent            the host's NVIDIA install
    6  the ordinary host dirs + /etc/ld.so.cache
    7  /run/opengl-driver/lib, /run/current-system/sw/lib   NixOS
    8  ⭐ SHARUN_FALLBACK_LIBRARY_PATH   LAST, and documented as last

⚠ Row 7 is not decoration: `pg83/solo`'s issue #2 was a NixOS `VkResult -9`
fixed by scanning `/run/opengl-driver`. A search order missing it fails on
NixOS **only**, which is the worst kind of missing row.

**Four classes, and the list is closed** — anything else in a bundle's trace is
still a bundling defect and still fails:

| class | default | opt-in |
|---|---|---|
| **nvidia** | ⭐ **HOST ALWAYS**, and not an opt-in | — it cannot be bundled: it is the counterpart of a kernel module the user installed, and its driver links a 10+ year old glibc so any bundled glibc satisfies it |
| mesa / GL / VA | bundled | `PGB_HOST_MESA=1`. Upstream tried mixing bundled and host drivers and **withdrew it** |
| vulkan ICDs and layers | bundled | `PGB_HOST_VULKAN=1`. Separate from mesa because the LAYERS are the user's — mangohud, lsfg-vk |
| glibc | bundled | `PGB_HOST_GLIBC=1`. The only opt-in that REORDERS rather than appends |

⛔ **Every opt-in is reported, never silent**, including the default, because an
opt-in that quietly changed which libc a process runs is this project's whole
subject.

⭐ **What changes about the acceptance test.** `docs/AGENTS.md` §3 criterion 2
stands unchanged for `pgb build` — a static ELF loading a host object has
failed, and `experiments/76-` asserts zero on all eleven. For a **bundle** it
becomes *no host shared object OUTSIDE the four classes*, and `HostClass()` is
the classifier, asserted in both directions: `libGLX_nvidia.so.0` is correct,
`libcurl.so.4` is a defect, and a classifier answering "nvidia" for everything
would make every bundle pass.

⭐ **One thing that fell out and is worth carrying.** Upstream's own FAQ names
`pg83/solo` — the reference T-064 built from — and rejects it. Their first
objection, *"not able to dlopen any library from the host"*, is now **false of
this project's static output**: `experiments/76-` does it on 11 of 11. Their
other three are about DRIVER VERSIONS, not about `dlopen`, and they stand. That
is why a bundle exists **beside** the static ELF rather than instead of it.

⚠ **NO GPU WAS INVOLVED.** Every GL row here is `swrast` (`TODO` T-059), so the
driver classes are implemented and REPORTED, not measured on hardware. The
order and the reporting are asserted; the driver behaviour is T-059's.

## T-069 — the supplied working paper, swept

**Source** ⭐ **operator, 2026-09-02c**: a 1,148-line working paper,
*One Libc in the Process*, supplied as an upload with the instruction to
*"check if this is actually useful and something that would have helped us
before or can still help us or is just low quality slop"*.
**Category** research · **Priority** P1 · **Effort** S · **Status** done

**Answer: useful, and not slop.** It states evidence tiers and keeps to them,
reports its dead ends and one unexplained anomaly, and names its own
limitations — including the one that matters most.

⭐ **Three things it gave this tree that it did not have**, and the first two
would have helped earlier:

| | |
|---|---|
| the **source-level cause** of `experiments/72-` | `elf/dl-support.c`'s dummy link map: *"We don't export any symbols ourselves."* We had the symptom (`DYNSYM 0`); this is glibc's own statement of the mechanism |
| the **folklore export route killed twice**, at T1 | `--export-dynamic` emits **no dynamic section at all** on a `-static` link (binutils 2.46.1), and a hand-built `.dynsym` under `-static-pie` is a dead letter anyway. ⛔ A route this project had never closed, and it looks plausible right up to the second measurement. Now in `docs/AGENTS.md` §14 |
| a **third sighting of the linker-script trap** | its `libc.so: invalid ELF header` is our `libm.a` is a `GROUP(...)` script, in a different reader. Three sightings makes it a platform property, not a recurring mistake |

⭐ **And the thing worth recording most: its own §10 limitation 2 is
*"No bridge of our own [...] this study did not construct, run, or
independently re-measure a bridged loader end-to-end."* `experiments/76-` is
that measurement, at T1, on eleven environments, made the same day.** It is
also cheaper than the paper's taxonomy predicts, for a reason the paper itself
supplies (§8.4's direction asymmetry): a static **glibc** carrier needs no ABI
bridge, so `pgb-elfload.c` is 1,093 code lines against `solo`'s 2,332 plus
5,948 of glibc→musl shim.

⛔ **One check it prompted, and it could have been a silent second libc.** Its
F6 observes that a loaded object cannot call the dl API. Ours is the opposite
case: the generated provider table is built from `libc.a` and carries rows for
`dlopen`/`dlsym`/`dlclose`/`dlerror`. Had those held *glibc's* `dlopen`, a
loaded host object calling it — GTK, Qt and mesa all do — would have reached
the **host** loader. Measured rather than reasoned about:

    provider table 'dlopen' = 0x405780
    __wrap_dlopen           = 0x405780
    VERDICT: table points at OUR wrapper -- safe

⚠ Safe **because `--wrap=dlopen` is on the link line and rewrites the table's
own undefined reference**, not because of anything in the generator. A build
producing the table without the wrap would reopen it; `internal/wrapper/flags.go`
adds both together and cannot add one without the other.

⚠ **One generality to distrust.** Its F2 reports that `dlopen` of a
libc-linked object from a plain static binary *succeeds* — "There is no
rejection". On the eleven pinned environments it succeeds on **two** and dies
on nine (`docs/limitations.md` §1). Its own §10 says "single toolchain", so
this is a caution about how F2 is read, not a contradiction of its evidence.

**Prove.** ⭐ **Done**: vendored at
`references/operator__one-libc-in-the-process/` with a `PROVENANCE.md` that
names every gap (no upstream, no author, no licence, no independent
reproduction), and swept in
[`../docs/research/one-libc.md`](../docs/research/one-libc.md).
