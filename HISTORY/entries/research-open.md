# HISTORY/entries/research-open.md — retired DETAIL of research entries that are STILL OPEN

⚠ **These entries are open. This file is not the entry** — the entry is in
[`../../TODO/research.md`](../../TODO/research.md) and is deliberately short. What is
here is the long-form record each one accumulated: the measurements, the
corrections, the routes costed and the routes killed.

⛔ **Read the TODO entry first.** Come here when you need to know WHY it says
what it says, or before re-running something to check whether it was already
run. ⭐ A number quoted in the TODO entry was derived here.

⚠ The headings below deliberately do NOT use the `## T-NNN — ` form, because
that form is what `sh TODO/check.sh` treats as *the* entry, and there must be
exactly one of those per id.

---

## T-021 · retired detail — Build one nix-appimage and run it on the matrix

**Source** follow-on from T-020 · **Category** research · **Priority** P2 · **Effort** M · **Status** open

**Problem.** T-020's claims about what nix-appimage *costs* come from its
tracker, which is evidence of intent and never of behaviour.

**Approach.** `nix bundle` the same subject `experiments/60-` uses, run it on
all 11 with the `62-` instrument. ⚠ Needs nix on the build host — `pgb
bootstrap` installs it, so this is no longer the blocker it was.

**Prove.** `evidence/64-nix-appimage/RESULT.txt` with the coverage and
host-object columns filled, comparable to `60-` and `62-`.


## T-057 · retired detail — The bundler: a maintained nix-appimage descendant, on the Anylinux mechanisms

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
4. ⚠ **"No 32-bit path" — STALE, CORRECTED 2026-09-03c BY READING THE CODE
   RATHER THAN THE ENTRY.** `lib32` is implemented and has been for some time;
   this row was never updated. `internal/bundle/assemble.go` reads each
   object's `EI_CLASS`, routes the 32-bit half to `lib32` instead of `lib`,
   finds and copies a **32-bit loader** (`ld-linux.so.2` /
   `ld-linux-armhf.so.3`), ⭐ **warns by name when the closure has 32-bit
   objects and no 32-bit loader** rather than producing a bundle that cannot
   run them, and creates the `shared/lib32` symlink sharun expects.
   ⛔ **What is actually missing is the MEASUREMENT**: no 32-bit application
   has been put through it, so nothing says the layout works — only that it is
   produced. ⚠ And the whole path had **no carried coverage at all** until
   2026-09-03c: `elfClass` decides `lib32` versus `lib`, and its own comment
   names the cost of getting it wrong — *"a flat directory holding an i386 and
   an x86_64 `libfoo.so.1` gives the loader whichever landed first"*, which is
   a silent wrong-architecture load rather than a build failure. Seven
   hermetic cases now pin it in `bundle-appimage` (ELFCLASS32 → 32,
   ELFCLASS64 → 64, and five ways of being neither → 0); they need no compiler
   and no multilib because `elfClass` reads five bytes.
5. **Nothing is measured against a hand-built Anylinux AppImage.**
   `experiments/62-` compares `pgb` against one; nothing compares OUR bundle
   against one.
6. ⭐ **A reference named by the operator on 2026-09-03c is unread**, and it is
   the closest thing to our own bundler in the corpus:
   [`references/xplshn__pelf`](../../references/xplshn__pelf/PROVENANCE.md),
   commit `d3cb5c7be01ae6a672fe480a117bb84cc65fc438`, mined the same day —
   *"for our bundle related tooling, this reference is maybe worth a study"*.
   ⚠ Owed under [`../docs/methodology/references.md`](../../docs/methodology/references.md):
   three passes and the two write-up files, ⛔ **not delegated to a sub-agent**.

⭐ **The claim worth making, stated so it cannot drift:** not "as good as a
hand-crafted AppImage" but *"produced by one command from a package name, and
within measurable distance of one"*. `docs/AGENTS.md` §14 governs the
wording.

### ⭐ AMENDED BY OPERATOR RULING, 2026-09-03c — the claim's two halves split

⛔ **The ruling re-scores this entry's own summary**, and is quoted verbatim:

> *"us having a bigger size than anylinux-appimages and onelf is acceptable as
> long as ours performs better and packaging is just one command not a
> multiline shell script"*

⭐ **The first half of the claim above is now a REQUIREMENT that is already
met** — *produced by one command from a package name* is exactly the
"one command not a multiline shell script" the ruling asks for, and item 5's
own table shows the competitor's route is not one command. Publish it.

⛔ **The second half — "within measurable distance" — is no longer good
enough on the clock, and no longer required on size.** Of the three ratios
this entry quotes:

| | ratio | ⭐ under the ruling |
|---|---|---|
| size | 3.05× | ⭐ **acceptable — struck from the bar** |
| cold start | ~1.9× | ⛔ **binding: "performs better" means under 1.0×** |
| warm start | ~1.4× | ⛔ **binding** |

⚠ **So item 1 of this entry — "no debloating at all" — is demoted.** It was
the explanation for the size ratio and the size ratio no longer counts. It
stays open only insofar as fewer objects is less to mount, map and relocate,
⛔ **which nobody has measured.** `docs/design/toolchain.md` "Static first,
bundle last" carries the amendment in full.

**Prove.** `evidence/86-bundler-vs-anylinux/RESULT.jq.txt` and
`evidence/86-bundler-vs-anylinux/RESULT.mpv.txt`.

⚠ **Corrected 2026-09-03c**: this line used to cite a RESULT.txt in that
directory, which does not exist and never did — `86-` runs against two subjects
and writes one file each. The citation survived because the docs gate exempts
evidence "named by an open entry", which is the right exemption for work not
yet done and the wrong one here, where the work IS done under another name.
⭐ **The gate can tell those apart now and does**: an exempted citation whose
directory exists and is tracked is a FAILURE naming the files that are actually
there. It fired on this line before it was fixed.

The bar itself is unchanged: the same application as an Anylinux AppImage and
as ours, on all eleven, with size, startup, and host-object columns — the
instrument in `experiments/62-` already produces three of those four.

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


## T-059 · retired detail — GL on real hardware, and the NVIDIA case

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

