# HISTORY/entries/research.md — the CLOSED research entries

⛔ **Nothing here is work.** Every entry below is `done`. They were moved
out of `TODO/research.md` on 2026-09-03c so that `TODO/` carries only what is
left, at the operator's instruction:

> *"strip away the fat, things that are already resolved and fixed and just
> send them straight into /HISTORY/\*, the TODO/\* must be lean and contain
> only what's left"*

⭐ **They keep their `T-` ids and their rows in [`../../TODO/INDEX.md`](../../TODO/INDEX.md)**,
which is what stops any of this being rediscovered. `sh TODO/check.sh`
checks this file against those rows exactly as it checked `TODO/`.

⛔ Do not reopen an entry here. A defect that still matters is a NEW entry.

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

## ✅ Done — [`../docs/design/host-fallback.md`](../../docs/design/host-fallback.md), and the mechanism

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
[`../docs/research/one-libc.md`](../../docs/research/one-libc.md).


## T-071 — ⛔ EGL out of a nixpkgs closure: the vendor libraries are the fragile part

**Source** ⭐ **operator, 2026-09-02c**: *"add a dedicated task to solving the
egl issue with nix"*.
**Category** research · **Priority** P0 · **Effort** L · **Status** ✅ done

⛔ **This exists because EGL out of nixpkgs has failed three times, each for a
DIFFERENT reason, and two of the three were invisible to every check the tree
had.** `experiments/85-` gets `EGL vendor string: Mesa Project` on 11 of 11
today, so the entry is not "make EGL work" — it is that the way it works is
held together by three separate rewrites of third-party data, none of which has
a control.

**The three failures so far, in order.** `TODO/research.md` T-052 records the
first two; the third was found reviewing the sweep on 2026-09-02c.

| | what broke | why it was hard to see |
|---|---|---|
| 1 | `eglInitialize failed`, no vendor at all — **mesa was not in the closure** when libglvnd was present and no driver was | libglvnd loads fine on its own; the failure is one layer down |
| 2 | `lib/dri` and `lib/gbm` were **flattened**, and they are found through a variable naming a DIRECTORY | a flattened copy has every file, so nothing is missing |
| 3 | ⛔ **the ICD/vendor JSONs name an ABSOLUTE `/nix/store` path** — `"library_path": "/nix/store/4cvv9…-mesa-26.2.1/lib/libEGL_mesa.so.0"` — so libglvnd found the vendor file, opened a path that did not exist, and failed **with `libEGL_mesa.so.0` sitting in `lib/` beside it** | the library IS in the bundle. Every integrity check passes |

⭐ **And a fourth was caught before it shipped, on 2026-09-02c.** The
reachability sweep now deletes what nothing can reach, and **a vendor library
is unreachable by construction**: it lives in `lib/` itself, which the
plugin-directory rule deliberately excludes (`p == root`), and nothing carries
`DT_NEEDED libEGL_mesa.so.0` because libglvnd `dlopen`s it by name out of the
JSON. `internal/bundle/sweep.go` now takes manifest-named libraries as roots,
with three selftest cases including the negative arm. ⛔ **That is the fourth
distinct way this stack has broken, and the pattern is the same every time: EGL
is reached through DATA, and every check this tree owns follows CODE.**

**⛔ The problem, stated so it is not re-derived.** A nixpkgs closure is
**location-locked** (`docs/research/nix.md`), and the GL stack is the worst
case for that: libglvnd finds vendors through
`share/glvnd/egl_vendor.d/*.json`, the Vulkan loader through
`share/vulkan/icd.d/*.json`, and both files carry absolute store paths written
at build time. Bundling therefore means **rewriting third-party data files**,
and every rewrite is a place the format can change under us.

**What this entry owns.**

1. ⭐ **A control for the rewrites.** ✅ **DONE 2026-09-02d.**
   `CheckManifests` (`internal/bundle/assemble.go:631`) is the measurement — no manifest may name a
   path outside the bundle, and every library a manifest names must exist in
   it. It runs in the build as `manifestIntegrity()` (a report, like
   `integrity()`, because a closure may legitimately carry a manifest for a
   vendor it did not bundle) and is exposed as **`pgb bundle manifests
   APPDIR`**, which exits non-zero, so `experiments/85-` can assert on it.
   ⛔ **With the negative control the Prove asks for**: one vendor JSON is put
   back the way nixpkgs ships it and the check must FAIL, then the damage is
   restored and the restore itself is asserted.
   ⭐ **This is the first check in this tree that reads DATA rather than
   DT_NEEDED**, which is the whole shape of T-071's four failures.
2. ⛔ **The layer manifests are unhandled.** ✅ **DONE 2026-09-02d.** The
   rewrite now iterates the SWEEP's own `manifestGlobs` instead of a second
   hand-written list, so the two rules cannot disagree about which files
   matter, and it handles the OpenCL `.icd` form — one library per line, not
   JSON — which neither side read before. The selftest generates its fixture
   **from `manifestGlobs`**, so a glob added to the list gets an assertion
   automatically; a second hand-maintained list would be this same defect one
   level up.

   ⭐ **And the widening is measured, on the same subject, one run apart.**
   kdenlive's build log, `icd json N rewritten to bare sonames`:

       run 5, three globs   16 files
       run 6, five globs    20 files

   ⚠ **Four files that were being kept as reachability roots while their own
   `library_path` still pointed into `/nix/store`** — which is the half-fix
   stated as a count rather than as a worry.
3. **Size.** The GL stack is **95 MiB of a 163 MB bundle** (`experiments/85-`),
   because nixpkgs' mesa is 273 MB unstripped and the Anylinux flow ships a
   debloated one. Folds into T-066's "where the closure comes from".
4. **NVIDIA.** `docs/design/host-fallback.md` rules it host-always and never
   bundled. ⚠ Untested against nixpkgs' libglvnd, which is the dispatcher that
   would have to find a host vendor while its own vendors are bundled.
   ⭐ **SPLIT 2026-09-02d, because the two halves have different owners.** The
   half that is decidable here — that the opt-in *changes the environment
   correctly* — is now asserted offline: `PGB_HOST_MESA` drops
   `LIBGL_DRIVERS_PATH` **and** releases
   `__EGL_VENDOR_LIBRARY_FILENAMES`, which it previously would not have,
   because pinning the bundle's own vendor list under an opt-in that says "use
   the host's mesa" makes the opt-in do the opposite of what it says. ⛔ The
   half that needs a card — that a host NVIDIA vendor is then actually found
   and renders — is **T-059's**, and no assertion here can stand in for it.
5. ⛔ **`__EGL_VENDOR_LIBRARY_DIRS` vs `__EGL_VENDOR_LIBRARY_FILENAMES`.**
   ✅ **DONE 2026-09-02d, and the semantics were READ rather than assumed** —
   libglvnd v1.7.0 `src/EGL/libeglvendor.c`, `LoadVendors()`:

   ```c
   env = getenv("__EGL_VENDOR_LIBRARY_FILENAMES");
   if (env != NULL) { ...load each token as a FILE...; return; }
   ```

   ⛔ **The `return` is the finding.** FILENAMES does not add to DIRS, it
   REPLACES it: if it is set at all, `__EGL_VENDOR_LIBRARY_DIRS` is never
   read. So a host that exports it makes a bundle setting only DIRS load the
   **host's** vendor JSONs and `dlopen` the host's `libEGL_mesa` — the
   two-libc state this project exists to prevent, arrived at through a
   variable nobody in the bundle set.

   ⚠ **And "just set it empty" would have been worse, not safer**, which is
   exactly why this was worth reading instead of guessing: `getenv` returns a
   non-NULL empty string, the branch is taken, no vendor is loaded, and the
   bundle has no EGL at all.

   The bundle now sets it to its own vendor files — what scanning DIRS would
   have found anyway — and **releases it under `PGB_HOST_MESA`**, where
   pinning it would make the opt-in do the opposite of what it says. Four
   selftest assertions including two negatives: a non-JSON file in the same
   directory must not reach the list, and a bundle with no vendor directory
   must leave the variable unset rather than set-and-empty.

**⚠ What this entry CANNOT settle here, and it must not be closed by silence.**
Every GL row in this tree is `swrast` — this machine has no GPU and none of the
eleven has a display. **T-059** owns hardware. So T-071's assertions are about
the bundle's DATA being coherent and its libraries being reachable, which is
decidable without a GPU; anything about a driver actually rendering is T-059's.

**Prove.** `experiments/85-` extended with a data-coherence arm: for every
manifest in the bundle, the library it names resolves INSIDE the bundle, and no
manifest names a path outside it — asserted, on a bundle built by `pgb bundle
appimage`, with a negative control that a deliberately un-rewritten manifest
fails it.

## ✅ CLOSED 2026-09-03 — `experiments/85-` RUN, and the negative control fired

    sh experiments/85-opengl.sh
    ok  arm A built (bundled mesa)                              = yes
    ok  arm B built (--no-gl)                                   = yes
    ok  arm A: every manifest names a library present in the bundle = yes
        manifests read: 13
    ok  an un-rewritten manifest is CAUGHT (the control)        = caught
        OUTSIDE  share/glvnd/egl_vendor.d/50_mesa.json
                 -> /nix/store/00000000…-mesa/lib/libEGL_mesa.so.0
    ok  the control restored the manifest it damaged            = yes
    ok  arm A: surfaceless EGL reports Mesa on every environment = 11
    ok  arm A: a driver is named on every environment           = 11
    ok  arm A: every target agrees with the build host's exit    = 11
    ok  arm A loaded no host shared object                      = 11
    ok  arm B (no bundled mesa) reported NO vendor anywhere     = 0
    pass=10 fail=0 skip=0     VERDICT: matched expectation

⭐ **That is the Prove as written, on a bundle `pgb bundle appimage` built**,
with the negative control the Prove asks for and the restore asserted. ⛔ **It
had been "written and not run" for three sessions**; the bed was occupied each
time.

⚠ **What the run does NOT say, quoted from its own footer rather than
paraphrased:** every row is **swrast** — this machine has no GPU — and every
target is **surfaceless**, because none of the eleven has an X server, a
Wayland compositor or a GBM device. It measures the GL stack **loading and
initialising**, not anything drawn to a screen. ⚠ `A:exit3` on every row is
`eglinfo`'s own convention — the number of platforms that failed to initialise
(GBM, Wayland, X11), which is 3 on the build host too.

**Items 3 and 4 are not closed here and are not this entry's to close:** item 3
(the GL stack is 95 MiB of a 163 MB bundle) folds into **T-066**'s "where the
closure comes from", and item 4's untestable half — NVIDIA against nixpkgs'
libglvnd — is **T-059**'s and needs a GPU.

### The arm, as it was written

⭐ **Written 2026-09-02d** — four assertions including the negative control and
a check that the control restored what it damaged.

⭐ **But the Prove was carried out by hand on a REAL bundle — kdenlive's, the
one `experiments/90-` had just built — and it is stronger than a fixture:**

```
$ pgb bundle manifests .../kdenlive/AppDir
manifests read: 8
VERDICT: every manifest names a library present in the bundle.        exit 0

# the negative control: 50_mesa.json put back the way nixpkgs ships it
$ pgb bundle manifests .../kdenlive/AppDir
OUTSIDE  share/glvnd/egl_vendor.d/50_mesa.json
         -> /nix/store/000…-mesa/lib/libEGL_mesa.so.0                 exit 1

# restored, and the restore verified
$ pgb bundle manifests .../kdenlive/AppDir                            exit 0
```

⛔ **That un-rewritten path is failure 3 of the four above, reproduced
deliberately and caught.** It is the first time this class has been detectable
at all: the same bundle passes `integrity()` — every `DT_NEEDED` resolves —
in both states.

---


## T-074 — ⭐ the host-policy selftest could not fail on the state it was written to catch

**Source** operator, 2026-09-03, item 3 of the `cross-libc-dlopen#28` review:
*"verify that the release path is asserted and not merely implemented."*
**Category** research · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** `internal/bundle/hostpolicy_selftest.go` asked whether a variable
was unset by reading its VALUE:

    get := func(lines []string, key string) string {
        for _, l := range lines {
            if v, ok := strings.CutPrefix(l, key+"="); ok { return v }
        }
        return ""            // ⛔ and "" for a key that is ABSENT, too
    }

⛔ **`get` returns `""` for a key that is absent AND for a key emitted as
`KEY=`**, and for `__EGL_VENDOR_LIBRARY_FILENAMES` those two are the safe state
and the dangerous one. `EnvLines()`'s own comment says so at length — libglvnd's
`LoadVendors()` reads `getenv`, a non-NULL empty string takes the branch,
`return` follows, and **the bundle gets no EGL at all**. So the assertion
labelled *"FILENAMES is unset, not set-and-empty"* was green on both, and four
others with it.

⚠ **Not a paragraph — this is the same defect class as T-073 and as upstream's
#28**, one level up: a *check* that answers when it cannot know. `PROGRESS.md`
2026-09-02f finding 9 is the previous instance ("one of my own new checks was
theatre").

**What the product does.** ✅ **Correct, and it always was.** `EnvLines()` never
emits the variable under `PGB_HOST_MESA` or for a bundle with no vendor
directory — it is inside `if !p.Mesa` and guarded by `len(paths) > 0`. Nothing
about the bundler changed here.

⚠ **And `experiments/85-` does not cover this at all.** It measures arm A
(bundled mesa) against arm B (`--no-gl`) and never sets `PGB_HOST_MESA`. The
release path's only assertion was the one above, so the answer to the operator's
question is neither "asserted" nor "merely implemented": it was asserted by an
instrument blind to the distinction being asserted.

### ✅ CLOSED 2026-09-03 — the Prove, run, with the reversal planted

`present()` reports whether the key appears at all; the five "is unset"
assertions ask it instead of `get`. ⭐ **The instrument itself is now asserted**
— a constructed `__EGL_VENDOR_LIBRARY_FILENAMES=` line is put through both
helpers, plus a prefix-collision negative.

⛔ **THE COMPARISON THAT SETTLES IT.** One planted defect — `EnvLines()` made to
`add("__EGL_VENDOR_LIBRARY_FILENAMES=")` under the mesa opt-in — through both
instruments:

| instrument | verdict on the planted defect |
|---|---|
| old, by value (`get(...) == ""`) | ⛔ **`ok ... the bundle stops pinning the EGL vendor list =`** |
| new, by presence (`present(...) == false`) | ⭐ **`FAIL ... RELEASED, not emptied = yes, wanted no`** |

    ./pgb selftest      312 cases pass, 1 COULD NOT RUN (no zstd)   [was 307]

`internal/bundle/hostpolicy.go` is byte-identical to before this entry
(`git diff --quiet`), which is the point: the fix is entirely in what the
harness can see.

---


## T-075 — ⭐ LD_DEBUG=bindings on the control, because the subject cannot be asked

**Source** operator, 2026-09-03, item 4 of the `cross-libc-dlopen#28` review.
**Category** research · **Priority** P2 · **Effort** S · **Status** ✅ done

**Problem.** ⚠ *Which object won this symbol* is a question this tree answers
expensively. The session of 2026-09-03 spent **four probe builds and a SIGSEGV
handler** establishing that `__once_proxy` had resolved to the definition it
should have. Upstream settled the identical question on issue #28 in **one
command**:

    LD_DEBUG=bindings ./contour
    binding file .../libQt6Gui.so.6 ... to .../gles-fwd.so: normal symbol `glGetString'

**Premise.** ⛔ **It cannot be used on the subject, and that is not a limitation
of the flag.** `LD_DEBUG` is read by glibc's dynamic loader. `pgb`'s output is a
static binary with `tool/runtime/pgb-elfload.c` compiled in and **no
`PT_INTERP`**, so there is no `ld.so` in the process to read it and nothing
would be printed. ⭐ **But every control this tree runs against glibc's own
`ld.so` is dynamic** — that is what makes it a control — so the diagnostic
belongs there.

### ✅ CLOSED 2026-09-03 — placed, and exercised

`experiments/93-`'s control is an ordinary dynamic probe that answers *does
glibc's own loader crash on this object too*. It now captures
`LD_DEBUG=bindings` for **the objects where the two loaders DISAGREE** — the
rows the assertion fails on, and the rows whose first follow-up question is
which definition `ld.so` bound. Not for all 1,527: that is hundreds of
megabytes answering a question nobody is asking about the rows that agree.

⛔ **AND IT FOUND A DEFECT IN ITSELF BEFORE IT WAS COMMITTED.**
`LD_DEBUG_OUTPUT` appends `.<pid>`, and **more than one file appears**:
`timeout` forks and is itself dynamic, so it writes a log of its own bindings
beside the probe's. Measured on `libz.so.1`: two files, `.22171` with 186 lines
and **not one mention of libz**, `.22172` with 141 lines and the bindings
actually wanted. ⚠ Glob order yields the useless one first, and the first
version of this took it. The file is chosen by **content** now — the one
carrying a `binding file <object>` line — never by position.

⚠ **`DIFFER` is 0 today, so the new path would never have run.** It was
exercised in a scratch copy with the branch forced and a three-object
population; both objects produced a log naming their own bindings (214 and 212
lines), the `timeout` decoy was deleted, and an empty directory is removed
rather than left to suggest a capture happened. `experiments/93-` re-run
afterwards: `ok=882 refused=122 failed=478 crash=45 hang=0`, `pass=6 fail=0`,
and no `bindings/` directory left behind.

### ⚠ Where it was NOT placed, and why — measured, not assumed

| named by the operator | what was found |
|---|---|
| `experiments/93-`'s hostprobe | ✅ **placed and exercised.** An ordinary dynamic program; the diagnostic works and prints the expected line shape |
| `poc/10-gawk`'s host-extension observation | ⚠ **the subject is the STATIC gawk.** Where the extension loads, the host `ld.so` enters as a *library* rather than as the program interpreter, and `LD_DEBUG` is read at `ld.so` startup. Whether it prints anything there is **unmeasured**, and this tree does not write an unverified diagnostic into a harness |
| `experiments/62-`'s bundle arms | ⚠ **the question is already answered by `classify_trace`.** *Which objects were opened* is what decides the second-libc claim there; *which object won a symbol* is a different and less load-bearing question, and the arms run inside a rootfs under `strace` with timeouts by design, so the logs would have to be written inside the target and carried out |

⛔ Both remaining rows are **open work, not a verdict**: each needs one
measurement — does `LD_DEBUG` print anything in that configuration — before
anything is written into those harnesses.

### ✅ 2026-09-03c — BOTH REMAINING ROWS ARE MEASURED, AND THE ANSWER IS NO

`experiments/96-`, `evidence/96-ld-debug-as-library/RESULT.txt`, **pass=14
fail=0**. The measurement the two rows were waiting on has been taken, and one
experiment settles both.

⭐ **The subject and the control differ in ONE property**: the same source, the
same compiler, one linked `-static` and one not, both `dlopen`ing the same host
object and calling a symbol out of it. Both succeed.

| `LD_DEBUG` | dynamic (control) | ⭐ static (subject) |
|---|---|---|
| `bindings`, stderr lines | **143** | ⛔ **0** |
| `all`, stderr lines | **485** | ⛔ **0** |
| `bindings`, `LD_DEBUG_OUTPUT` files | **1** | ⛔ **0** |
| `help`, program still ran | 0 — the loader printed and exited | ⛔ **1 — it ran to completion** |

⛔ **AND `ld.so` IS IN THE STATIC PROCESS.** `strace` shows the static binary
opening `/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2` on its way through the
`dlopen`. So this is **not** *"there was no loader to read the variable"* — the
loader is present, mapped, and does the work, and reads `LD_DEBUG` never.
⭐ The `help` row is the cleanest single proof: `LD_DEBUG=help` makes `ld.so`
print and **exit before the program runs**, and the static arm ran to
completion.

⭐ **The cause: `LD_DEBUG` is parsed during `ld.so`'s OWN startup**, which the
static-`dlopen` path never executes. Arriving as a library is arriving after
the point at which the variable would have been read.

⛔ **So the diagnostic must NOT go into `poc/10-gawk`.** It would not *fail*
there — it would produce an **empty capture**, and an empty capture in an
observation table reads as *"no bindings"* rather than *"the instrument does
not work here"*. `docs/AGENTS.md` §0b: an absence is not a zero. Writing it in
would have manufactured one on every row.

⭐ **AND THE SAME MEASUREMENT CLOSES THE `experiments/62-` ROW, which is more
than that row asked for.** T-075's premise said `LD_DEBUG` cannot be asked of
our output because there is no `PT_INTERP`. 96- says something stronger: it
cannot be asked **even when glibc's loader is in the process**. 62-'s arms
divide exactly on that line — the anylinux/sharun arm runs a **bundled
`ld-linux`** explicitly and would answer, and **our own arm is static and
cannot**. ⛔ An instrument that can describe every competitor and not the
subject is the wrong instrument for a comparison about the subject, and
`classify_trace` — which reads the opens and attributes them to a pid — already
answers the question 62- is actually asking.

⚠ **Measured on the build host** (Ubuntu 24.04, glibc 2.39), not across the
eleven. The mechanism is a property of where glibc parses the variable rather
than of a distribution, and the experiment carries its own control so a host
where the instrument were simply broken would fail the control first — but the
eleven-row version has not been run and is not claimed.


## T-080 — ⛔ REOPENED: the capability guarantee, on THREE applications per category

**Source** ⭐ **operator, 2026-09-03d** for the guarantee itself; ⛔ **reopened
by the operator, 2026-09-03f**: *"every capability listed in
docs/research/bundle-capabilities.md including ones already measured, must be
remeasured with 3 applications per category in order of simple to complex
applications ... all capabilities are closed as 'MEASURED, AND IT WORKS'"*.
**Category** research · **Priority** P1 · **Effort** L · **Status** ✅ done

⚠ **The closed version and everything it established are
[`../HISTORY/entries/research-open.md`](research-open.md)
`T-080 · retired detail`.** It is not repeated here.

⛔ **WHY ONE SUBJECT WAS NOT ENOUGH, AND THE RECORD ALREADY SHOWS IT.**
`experiments/64-` scored GTK on ONE application, `galculator`, and got **0 of
11** — from which *"GTK does not work out of a nix closure"* would have been
the obvious and WRONG conclusion. A second subject, `mousepad`, drew **11 of
11** through the same bundler on the same day, and the boundary turned out to
be a compiled-in data path rather than GTK. ⭐ One subject measures a subject.

**What is left.** `experiments/65-` is the corpus: three applications per
category, ordered by how much of the stack they drag in, each scored by a
**window on a real X server** (or, for a CLI subject, its exit status AND a
required string) plus **zero host shared objects** on all eleven.

⛔ **The rows that must come out MEASURED rather than assumed** are the ones
§0 currently labels otherwise: **SDL** (never run through this pipeline),
**Vulkan** (the ICD mechanism is relocatable by design and nothing has made a
Vulkan call here), **Qt**, **Python GUI** (T-081 unblocked it) and **apps with
a compiled-in data path** (T-081's own subject).

⚠ **TWO ROWS CANNOT BE CLOSED ON THIS MACHINE AND MUST SAY SO IN THE
SENTENCE**: every GL and Vulkan row here is a SOFTWARE rasteriser — `llvmpipe`
and `lavapipe` — and **NVIDIA is not bundled by design**. `T-059` owns
hardware, and a green corpus is not a GPU claim.

**Prove.** Every row of
[`../docs/research/bundle-capabilities.md`](../../docs/research/bundle-capabilities.md)
§0 carrying a count out of eleven and the subject that produced it, with the
UNRESOLVED subjects listed by name and reason rather than dropped.

---

⭐ **CLOSED 2026-09-04c: the corpus ran to completion — 26 of 26 subjects, every row a count out of eleven, 0 UNRESOLVED and 0 INSTRUMENT.** Six categories closed at three subjects each. ⛔ Every remaining zero is NAMED: the bed (`vkmark`, no `/dev/dri`), the closure (`neovim`, glibc 2.26), the subject's shape (`flameshot`, a tray application) and one path class we do not reach (`pdfarranger`, `/usr/local`). ⚠ The clean counts predate **C49** and **C54** and one corpus re-run is owed — tracked in `../../evidence/STALE-EVIDENCE.txt`, not here.

## T-088 — ⛔ multi-entry dispatch is SHIPPED and has never been run

**Source** the operator's question, 2026-09-04: *"Can we handle and are we
handling apps that provide multiple binaries like busybox … does simply
renaming or symlinking the bundle with the app we want to enter work?"*
**Category** research · **Priority** P1 · **Effort** S · **Status** ✅ done

⭐ **The answer read off the source was YES, and that was the problem.**
`assemble.go` installs every non-dot program in the entry store path's `bin/`
automatically; `--with-program` adds one from anywhere in the closure; and
`tool/runtime/pgb-apprun.c` is a **static** selector.

## ⭐ MEASURED — `experiments/68-`, `pass=24 fail=0 skip=0`

**Arm S, the dispatch table** (a synthetic AppDir; no bed, no display) —
18 of 18, and it **corrected the source's own header comment**, which stated
the order wrongly and named `ARGV0` nowhere:

    $ARGV0's basename  →  argv[1] (and it is DROPPED)  →  argv[0]  →  default

with it: a selected `argv[1]` is consumed and later arguments survive; an
`argv[1]` that did not select is passed through; the child's `argv[0]` is the
**absolute** path; `ARGV0` is unset and `APPDIR` set in the child; ⛔ a name
containing `/` is **never** a program, so a path cannot be injected; and
`shared/bin` without `bin/` is not a program. ⭐ The negative control — the
same source with an empty default — exits **127** with `no default program`
while still dispatching when told a name.

**Arm B, a real closure on all eleven** — `mkvtoolnix`:

| | |
|---|---|
| the build's entry-point count | ⭐ **`programs mkvmerge + 4 more`** |
| the selector actually built | the **static** `pgb-apprun`, not a shell |
| ⭐ the SECOND program (`mkvextract`), by its own name | ✅ **11 / 11** |
| the entry (`mkvmerge`), same artefact — within-row control | ✅ **11 / 11** |
| the second program's host shared objects | ⭐ **0 on 11 / 11** |

⛔ **The assertion is each program's OWN identity**: `mkvextract --version`
says `mkvextract`, so a dispatch that quietly ran the default fails. That is
why `imagemagick` was rejected as the subject — its `convert` and `identify`
are symlinks to one binary printing one string.

⭐ **The operator's second question is answered by a number the tool emits**,
not by an inference: `programs <prog> + N more`.

## ⛔ WHAT THE PROVE LINE ASKS FOR AND THIS DID NOT DO

⚠ **The entry stays open, and the goalposts are not being moved.**

1. ⛔ **THIS ITEM SAID `--with-program` WAS NEVER EXERCISED AND THAT WAS
   FALSE — `experiments/90-` HAS BEEN EXERCISING IT SINCE IT WAS WRITTEN.**
   Found 2026-09-04c by reading the script rather than the entry, which is
   delivery rule 5 doing its job on this page:

       ./pgb bundle appimage kdenlive --with-program melt --with-program ffmpeg
       …
       /kd-arm melt -version
       /kd-arm melt color:blue out=12 -consumer avformat:/tmp/kd.mp4 …

   ⭐ **`melt` is in the `mlt` store path, not in `kdenlive`'s `bin/`**, so it
   is precisely the whole-closure branch this item is about — and `90-` does
   not merely start it, it makes it **render an MP4**, on **all eleven**
   environments, and reports `P=ok E=ok` per row. ⚠ It also knows the
   assertion discriminates: its own comment records that the competitor's
   artefact runs *kdenlive* with `melt` as an argument.

   ⭐ **A second, independent exercise, 2026-09-04c**, on a subject chosen for
   it rather than for something else: `flameshot`, whose own closure carries
   `dbus-daemon` and `dbus-run-session` in a **dependency** store path —
   exactly the case this branch exists for, because they are not in the
   entry's `bin/`:

       pgb bundle appimage flameshot --name flameshot \
           --with-program dbus-daemon --with-program dbus-run-session
       →  programs    flameshot + 2 more
          apprun      pgb-apprun, static, 785704 bytes (no host interpreter)

       ./flameshot.AppImage dbus-daemon --version    (debian-12, in the bed)
       →  D-Bus Message Bus Daemon 1.16.2

   ⭐ **The assertion discriminates**: `dbus-daemon` prints its OWN identity,
   so a dispatch that quietly ran the default would print flameshot's banner
   instead. ⚠ **One environment, by hand** — a second closure rather than a
   second eleven, and this entry does not count it as the Prove line.

   ⛔ **So what this item actually still owes is narrow**: the `flameshot`
   demonstration on the eleven. The whole-closure branch itself is measured
   across the bed by `90-`, twice over — a start and a render.
2. **Two names, not N.** The bundle carried five programs and the run asserted
   two of them. A third name costs one more row and would catch a selector
   that only ever gets the first two right.
3. **One closure.** A second multi-program subject — `rnote`/`rnote-cli` is
   the rung-1 example — is a different measurement, not a repeat.

**Prove.** One artefact, N names, N assertions, on all eleven — and the
`--with-program` path exercised at least once, because it is the branch that
searches the whole closure rather than one `bin/`.

**Study.** `tool/runtime/pgb-apprun.c` (130 lines, and it is the answer);
`internal/bundle/assemble.go` `installProgram` and the `bin/` walk above it.

---

⭐ **CLOSED 2026-09-04c: the premise was false.** `--with-program` was **already being exercised** by a shipped experiment — `experiments/90-` — which the entry had not checked (**C45**). It is exercised again by `107-` (`--with-program dbus-daemon dbus-launch`, both installed, verified by the build log) and by `108-`.

## T-089 — ⛔ the interposer row marked NOT MEASURED: a static or raw-syscall payload

**Source** [`../docs/design/store-paths.md`](../../docs/design/store-paths.md) §3,
which marks exactly one row of its own comparison table as **NOT MEASURED**.
**Category** research · **Priority** P1 · **Effort** S · **Status** ✅ done

The interposer wins calls that go **through the PLT**. A statically linked
program, or one issuing raw syscalls — a Go binary is the common case — has no
PLT, so the mechanism cannot see its `open`. ⛔ That row is reasoning about a
mechanism, not a result, and it sits in a shipped design document.

**The subject is named and it is in the corpus**: `syncthing`, a Go program
from the closure. `lilipod` and `powershell` are the same shape.

⛔ **PRE-REGISTER THE FAILURE.** The expected outcome is that the build
**reports** the compiled-in store path and the program cannot resolve it at run
time. If it passes, the reasoning was wrong and the record says so rather than
being quietly corrected.

**Prove.** One Go subject bundled and run on all eleven, with the build's own
`store paths … compiled in` line quoted beside the result, and
`docs/design/store-paths.md` §3's row changed from NOT MEASURED to whatever
came out.

## ⭐ MEASURED — `experiments/100-`, three arms, `pass=13 fail=3`

⛔ **The three failures are the findings and none of them is the one that was
pre-registered.**

**Arm P — the mechanism** (planted store path, real interposer, real
`.storemap`, same source built three ways; **two runs identical**). ⭐ **The one
row split into two, failing for different reasons:**

| probe | our object mapped | resolves |
|---|---|---|
| ⭐ dynamic through the PLT — the positive control | yes | ✅ yes |
| ⛔ `-static` | **no** | no |
| ⛔ `syscall(SYS_openat, …)` | **yes** | no |
| ⭐ dynamic, no preload — the negative control | no | no |

⛔ Only the first is about linking. **A subject can be fully dynamic and still
defeat the interposer.**

**Arm G — `syncthing`.** ⛔ **G3 falsified the entry's own premise**: nixpkgs'
build is **dynamic**, not static, so the subject named here for the static row
was the wrong shape. It ran **11 of 11**, clean **11 of 11**, 8 store paths
compiled in and 7 resolving.

**Arm L — `lilipod`, genuinely static.** ⛔ **`pgb bundle appimage` REFUSES a
loader-less closure**, so the interposer question never arises — there is no
artefact. The raw binary, no bundle at all:

| | |
|---|---|
| ⭐ **the static ELF EXECUTES** | ✅ **11 / 11** |
| ⛔ **the APPLICATION completes** | **2 / 11** |
| zero host shared objects | **1 / 11** |

⭐ **Nine failures, one cause, named by the program itself**: *"failed to find
dependency `getsubids`, can't recover"* — a **host program on `$PATH`**.
Exactly two rootfs carry it (`archlinux-latest`, `opensuse-leap-15.6`) and
those are exactly the two that completed. ⚠ One of them also needed the
**network**: openSUSE has no `tar`, so it downloaded busybox at run time.

⭐ **THE AXIS MOVED.** A statically linked **ELF** is portable — that is what
static linking buys, and it is 11 of 11. A statically linked **application**
need not be, and nothing a linker, loader or bundler does addresses a
dependency that is **another program**. ⚠ It also explains the host-object row:
when a static program execs host programs, the host's libraries enter the
**process tree** — the same mechanism `experiments/65-` pre-registers for
`xterm` (C5), reached independently on a different subject.

## ⛔ What is still owed

1. ⭐ **`powershell` WAS ATTEMPTED, 2026-09-04c, and it did NOT turn out to be
   this rung's shape at all — which is the useful part.** It bundles in one
   command (34 store paths, 95.4 MiB) and starts; `pwsh -NoProfile -Command`
   then exits **127**, and the trace says exactly where:

       2858 execve(".../store/powershell-7.6.5/share/powershell/
                    .pwsh-wrapped", …)                 ⭐ TRANSLATED, and it RAN
       2858 newfstatat(".../store/…/.pwsh-wrapped", {st_mode=…})  translated ×2
       2858 newfstatat("/nix/store/3sn7g1s…/.pwsh-wrapped", …)
                                        ⛔ UNTRANSLATED, ENOENT ×2
       2858 +++ exited with 127 +++

   ⛔ **Nothing here is static.** `.pwsh-wrapped` is a dynamic PIE, it imports
   `stat`/`lstat`/`faccessat`/`eaccess` — the **unsuffixed** names the
   interposer defines — and `strings` finds **no** `/nix/store/3sn7g1s…` in
   it. ⭐ It is the **.NET apphost**: its strings carry `/proc/self/exe`,
   `/proc/self/maps`, `DOTNET_ROOT` and `/usr/share/dotnet`.

   ⭐ **So the path is ASSEMBLED AT RUN TIME from something the interposer does
   not mediate**, and that is a different failure from the one this entry was
   written for. ⚠ Two candidates and neither is measured: `/proc/self/exe`,
   and `/proc/self/maps`. **The next step is one traced row that reads which
   of the two the apphost consults before it builds that string** — not
   another subject.

2. ⛔ **THE SEARCH FOR A STATIC SUBJECT IS NOW ENUMERATED, AND THE OBVIOUS
   ROUTE IS CLOSED.** Four candidates have been tried and every one failed for
   a different reason:

   | candidate | why it is not this rung |
   |---|---|
   | `syncthing` | ⛔ nixpkgs builds it **dynamic** — the entry's own premise, falsified by arm G3 |
   | `lilipod` | genuinely static, but ⛔ `pgb bundle appimage` **refuses** it |
   | `powershell` | ⛔ dynamic PIE; fails on a run-time-assembled path instead |
   | ⛔ `pkgsStatic.file` | **unreachable by construction** — `pgb` turns a name into a store path with a regex over what the channel already **built**, and `pkgsStatic.*` is an unbuilt attribute. `experiments/83-` says so in its own header |

   ⛔ **AND A FIFTH ROUTE WAS PROPOSED HERE AND WITHDRAWN THE SAME DAY, which
   belongs in the entry rather than in a deleted diff.** The proposal was
   `pgb build` a static `file`(1) from a stock tarball —
   **[`experiments/105-`](../../experiments/105-file-magic.sh)** has already
   measured the DYNAMIC half at 11/11 with a `--no-storefix` control failing
   11/11, so a static build of the same program looked like the same subject
   with the PLT removed.

   ⭐ **IT IS NOT, AND `internal/nixx/build.go` SAYS SO IN ITS OWN HEADER**:
   *"What comes out is an ordinary statically linked glibc ELF: **no store**,
   no …"*. A `pgb build` artefact carries a `--prefix` path, **not a
   `/nix/store` path**, and `pgb-storefix.c`'s `fix()` rewrites nothing that
   is not `/nix/store/`. ⛔ So that subject would exercise a static binary
   with a compiled-in path the interposer was never meant to touch — a
   different question wearing this one's clothes.

   ⭐ **WHAT THAT LEAVES IS A BUNDLER QUESTION, NOT A SUBJECT HUNT.** A real
   application on this rung must be *statically linked* **and** carry a
   `/nix/store` path, which means a static build **from the ordinary nixpkgs
   set** — and exactly one such subject is known: `lilipod`, which
   `pgb bundle appimage` **refuses**. ⚠ Meanwhile
   `internal/bundle/assemble.go` records that sharun *"skips the loader
   invocation entirely for a static or already-patched payload
   (`is_static_bin`, `is_patched_bin`)"*, i.e. the delivery path does handle
   static payloads. ⛔ **So the next step is to read WHY the bundler refuses
   `lilipod` and whether that refusal is correct** — not to look for a fifth
   subject.

   ⭐ **AND THAT POSSIBILITY IS THE ANSWER — the evidence was already in arm L
   and had not been read back against the row.** `pgb bundle appimage` refuses
   `lilipod` with one line, `pgb: the closure carries no dynamic loader`, and
   the refusal is **correct**: arm L ran the raw static binary with **no
   bundle at all** and it executed on **11 of 11** loading **0** host shared
   objects. A statically linked ELF is already portable, so bundling it adds
   nothing — and there is therefore **no artefact to ask the interposer
   question of**. ⛔ The `-static` row is not untested; it is one a `pgb`
   artefact **cannot reach**, because the loader that would carry the
   interposer is exactly what such a closure lacks.

   ⚠ **WHAT WOULD ACTUALLY MOVE THE ROW, stated precisely.** A **mixed**
   closure: a statically linked main program in a closure that *does* carry a
   loader because its sibling programs are dynamic. That gets bundled, and its
   entry point defeats the interposer exactly as arm P predicts. ⛔ No such
   subject is known here — `syncthing` was checked and nixpkgs builds it
   dynamic. `docs/design/store-paths.md` §3 now says this instead of "still
   owed".

   ⚠ **And one thing the run establishes on the way**: `shared/script/pwsh`
   carries its `/nix/store` paths **as text, unrewritten** —
   `exec -a "$0" "/nix/store/3sn7g1s…/.pwsh-wrapped"` and an
   `LD_LIBRARY_PATH='/nix/store/…-curl-8.21.0/lib'`. ⭐ The interposer caught
   the `exec` (the trace proves it) — which is the run-time-rewrite cell of
   the grid in [`../docs/research/bundle-capabilities.md`](../../docs/research/bundle-capabilities.md)
   working on a real subject.

   ⛔ **The `-static` row therefore STILL has no application behind it.**
   `syncthing` was dynamic, `lilipod` was refused, and `powershell` is dynamic
   too. A genuinely static subject that needs a compiled-in path has not been
   found.
2. ⚠ **An open question arm G raised rather than settled.** Go issues raw
   syscalls for much of its file I/O even when dynamic, so S3 predicts the
   interposer loses — yet 7 of 8 paths resolved. Build-time **text** rewriting,
   the program never opening them, or those opens routing through libc are all
   consistent. `LD_DEBUG` plus one traced row separates them.
3. ⭐ **The refusal message sends the reader to the wrong problem.** It names a
   missing **loader**; the useful sentence is that a payload which already
   starts everywhere does not need bundling.

---

⭐ **CLOSED 2026-09-04c: the `-static` row is answered, and the answer is a refusal.** A fully static closure carries no dynamic loader, so `pgb bundle appimage` refuses it (`pgb: the closure carries no dynamic loader`) and **there is no artefact to ask the interposer question of**. ⭐ The refusal is correct, measured: arm L ran `lilipod`'s raw static binary with no bundle at all — **11/11 executes, 0 host objects on 11/11**. ⛔ Four subjects were tried and all four failed differently (`syncthing` is dynamic, `lilipod` is refused, `powershell` is dynamic, `pkgsStatic.*` is unreachable by construction), and a fifth route (`pgb build` a static `file`) was proposed and withdrawn the same day — its output carries a `--prefix` path, not a `/nix/store` path. ⚠ **WHAT WOULD STILL MOVE IT**, stated precisely: a **mixed** closure — a static main program in a closure that carries a loader because its siblings are dynamic. No such subject is known. `../../docs/design/store-paths.md` §3 says this instead of "still owed".
