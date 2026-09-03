# The nixappimage sweep — what the field patches, and what our runtime already carries

⛔ **This is the usable half.** It exists for the session that does the work,
and every line reference is checkable against a commit in `references/`.

⚠ **Three passes and three reviews, 2026-09-03d.** The corpus is what the
operator named: `pkgforge/soarpkgs` at the exact commit that still has the
recipes, `Azathothas/Toolpacks` at the commit whose scripts still carry the
patching, and the **forked** runtimes `pkgforge-dev/Anylinux-uruntime` and
`pkgforge-dev/appimagetool`.

| repository | commit |
|---|---|
| `pkgforge/soarpkgs` | `55c774a5e24d9f17af69911a4d70884dfb566626` |
| `Azathothas/Toolpacks` | `7fe47ab34b2648a2873d112f4e6b7e6423721a13` |
| `pkgforge-dev/Anylinux-uruntime` | `5a0b4a336a89daa56902d95c328ff7a4ae673c66` |
| `pkgforge-dev/appimagetool` | `183c04927f4c4c5bd757e42d9f66b5a998b44838` |
| `Azathothas/bit-cli` | `cce8131231abe8b232054f3f27b3feeac19dd411` |

---

## ⭐ 1. THE HEADLINE: our own runtime's knobs are PATCHABLE, not just settable

⛔ **THIS CORRECTS SOMETHING THIS PROJECT PUBLISHED THE SAME DAY.**
`TODO/toolchain.md` T-066 item 6, `portable-nix-mechanisms.md` §3 and
`experiments/81-`'s header all said the mount-versus-extract selector is an
**environment variable read at run time**, so shipping a non-default would
need something beside the artefact and the brief refuses that.

⚠ **That is wrong, and the source says so plainly.**
`Anylinux-uruntime/tree/src/main.rs:29-33`:

```rust
const URUNTIME_EXTRACT:   &str = "URUNTIME_EXTRACT=3";
const URUNTIME_MOUNT:     &str = "URUNTIME_MOUNT=3";
const REUSE_CHECK_DELAY:  &str = "5s";
const MAX_EXTRACT_SELF_SIZE: u64 = 350 * 1024 * 1024; // 350 MB
```

and `main.rs:1427` reads its own constant back through a string replace:

```rust
match URUNTIME_EXTRACT.replace("URUNTIME_EXTRACT=", "=").as_str() {
    "=1" => { is_extract_run = true; 1 }
    ...
```

⭐ **That replace is the tell.** A plain integer constant would be folded away
by the compiler; a *string* of the form `NAME=value`, read back by stripping
its own prefix, survives into the binary as searchable ASCII **so it can be
patched in place**. It is the same trick AppImage uses for `.upd_info`.

⛔ **Confirmed in the artefact `pgb` ships**, not merely in upstream's source:

```sh
strings -a /var/tmp/pgb-appimage/tools/uruntime | grep -oE 'URUNTIME_(EXTRACT|MOUNT)=[0-9]'
# -> URUNTIME_EXTRACT=3
# -> URUNTIME_MOUNT=3
```

⭐ **So the lever is a ONE-BYTE, same-length overwrite** (`=3` → `=1`), applied
to the header before packing, and the output stays a single file with nothing
beside it. ⚠ **Nothing about it is measured yet** — `experiments/81-`'s
machinery builds artefacts of any size and `experiments/clock.sh` gives them a
control, so it is one experiment away.

### ⛔ AND OUR kdenlive BUNDLE IS ALREADY TAKING THE OTHER PATH

`URUNTIME_EXTRACT=3` is mode 3: **mount below `MAX_EXTRACT_SELF_SIZE`, extract
above it**, and that constant is **350 MB**.

| artefact | size | which path uruntime takes |
|---|---|---|
| `pgb bundle appimage jq` | 6,806,407 B | **mounts** |
| `pgb bundle appimage kdenlive` | 565,332,219 B | ⛔ **extracts** |

⭐ **Nothing in this project's record knew that**, and it changes how two
measurements must be read: `experiments/86-` (jq) and `experiments/90-`
(kdenlive) are exercising **different runtime paths**, so a lever measured on
one does not transfer to the other by default. ⚠ It is also the first
candidate explanation for `90-`'s unexplained warm row — 114 ms against `jq`'s
8 ms — which is otherwise a large number with no mechanism behind it.

## ⭐ 2. What `lite` actually drops — settled, from the build task list

⚠ **This project guessed "compression backends" in a code comment, could not
measure it, and withdrew the claim.** The fork's own README answers it:

```
appimage-lite-x86_64   build x86_64 AppImage uruntime (no dwarfsck, mkdwarfs)
```

and `src/main.rs:98-131` shows the mechanism — the lite build embeds a
**different, smaller dwarfs binary**:

| build | embedded asset | `dwarfsck` | `mkdwarfs` |
|---|---|---|---|
| full | `assets/dwarfs-universal-zst` | ✅ | ✅ |
| ⭐ **lite** | `assets/dwarfs-fuse-extract-zst` | ⛔ gated out | ⛔ gated out |

⭐ **It drops TOOLS, not codecs** — which is exactly why the measurement this
project *did* take came out the way it did: the same AppDir packed at
`zstd:level=19`, `lzma:level=6` and `null` runs under **both** headers, six for
six. `internal/bundle/appimage.go` can now say why rather than saying it does
not know.

## ⭐ 3. The 5-second reuse window is a source constant

`experiments/99-` measured uruntime's mount teardown at **between 4 s and 6 s**
by bisecting the gap between two runs, and made a 6.24× cold/warm difference
out of it. `main.rs:32`:

```rust
const REUSE_CHECK_DELAY: &str = "5s";
```

⭐ **The measurement and the source agree exactly**, which is the strongest
form this project's evidence takes: an independent instrument landing on a
constant it never saw.

## 4. ⛔ The patching the operator called messy, quoted so it can be beaten

`Toolpacks/tree/.github/scripts/x86_64_Linux/bins/gettext.sh:35-39` — **five
overlapping regexes applied in sequence, each catching what the last missed**,
ending in a blunt "any store path becomes `/`":

```sh
sed "s|^#!/nix/store/.*/bin/sh|#!/bin/sh|"                  # 1 the shebang
sed "s|/nix/store[^ ]*/bin/\([^ ]*\)|/usr/local/bin/\1|g"   # 2 store bin + program
sed "s|/nix/store/[^/]*/bin|/usr/local/bin|g"               # 3 store bin
sed "s|/nix/store[^/ ]*/bin|/usr/local/bin|g"               # 4 the same, differently
sed "s|/nix/store[^ \"']*|/|g"                              # 5 ⛔ everything else
```

⚠ **Applied to `*.sh` files OR files that are not executables** — the selector
is `-name "*.sh" -o -exec sh -c 'file -i "$1" | grep -qiv
"application/.*executable"'`, and that `-o` is an **or**, so an executable
`.sh` is still patched. The intent is: text-patch scripts, leave ELFs alone.

⭐ **WHY WE CAN DO BETTER, AND IT IS NOT A MATTER OF WRITING NICER REGEXES.**
Those patterns are guessing at the store-path *shape* because the script does
not know the closure. ⛔ **`pgb` does.** `internal/nixx` computes the exact set
of store paths in the bundle, so the rewrite can be an **exact match against a
known finite set** — every occurrence of a path that IS in the closure gets
rewritten, and a path that is not is left alone and *reported*. Regex 5 above
cannot make that distinction and destroys any store-shaped string it finds,
including ones inside data.

⚠ **And a path with no in-bundle target is a FINDING, not a substitution.** The
cascade above silently maps such paths to `/usr/local/bin`, which is a bet that
the host has the program. Naming them is the honest behaviour and is what
`docs/AGENTS.md` §0b's *"an absence is not a zero"* asks for.

## 5. The `.desktop` and icon rules, in one place

⭐ **This answers the operator's standing question** in
[`../design/nix-front-end.md`](../design/nix-front-end.md) — *"there must be a
way to automatically get these"*. From
`soarpkgs/.../ghostty/nixappimage.nixpkgs.stable.yaml`:

**Icon.** ⛔ **THE LOAD-BEARING PART IS THE EXCLUSION, AND AN EARLIER VERSION OF
THIS PAGE INVERTED IT.** It said "the smallest `.png`/`.svg`", which is wrong
twice — the operator caught it, and the source confirms both:

```sh
find -L "./squashfs-root/usr" -type f,l -regex '.*\.\(png\|svg\)' \
  -not -regex '.*\(favicon\|/\(16x16\|22x22\|24x24\|32x32\|36x36\|48x48\|64x64\|72x72\|96x96\)/\).*' \
  | awk '{print length, $0}' | sort -n | awk 'NR==1 {print $2}'
# fallback, when that produced nothing usable:
find -L "./squashfs-root/usr" -regex ".*\(128x128/apps\|256x256\)/.*${PKG}.*\.\(png\|svg\)" ...
```

1. ⚠ `awk '{print length, $0}' | sort -n` sorts by the length of the **path
   string**, not by file size. "Smallest file" was never what it did.
2. ⛔ **And the intent is the opposite of small.** The `-not -regex` throws away
   `16x16` through `96x96`, and the fallback targets **`128x128/apps` or
   `256x256`** explicitly. The rule is *at least 128×128*.

⭐ **What ours should do, and it is a preference order rather than a search
trick**: prefer **128×128**, then **512×512** or **1024×1024**; ⛔ never take a
bucket below 128 — a 48×48 icon scaled up is what a desktop entry looks bad
with, and the exclusion list above exists for exactly that reason. Copy the
result to `<pkg>.png` **and** `.DirIcon`.

⚠ `Toolpacks`' `mpv.sh` agrees and is blunter about it —
`find "." -path '*128x128/apps/*.png'`, nothing else considered.

**Desktop.** ⛔ **AND "THE SMALLEST" IS WRONG HERE TOO, FOR A SECOND REASON:**

```sh
find -L "./squashfs-root/usr" -name "*.desktop" -printf "%s %p\n" -quit | sort -n | awk 'NR==1 {print $2}'
```

⚠ `-quit` is a `find` **action** that terminates the search at the first match,
so `sort -n` receives exactly one line and sorts nothing. **It takes the first
`.desktop` the traversal encounters**, and the `-printf "%s %p"`/`sort -n`
around it is dead code that reads as a size policy. ⚠ The icon *fallback* above
carries the same `-quit` and the same dead sort.

⚠ `Toolpacks`' `mpv.sh` does something different again —
`... | awk '{print length, $0}' | sort -n | head -n 1` with **no** `-quit` —
which takes the **shortest path**, still not the smallest file. ⭐ Three
selection rules across two repositories, none of them the one the code appears
to state.

⛔ **So there is no upstream policy here to copy, only accidents.** Ours has to
choose one deliberately and say what it is.

Then three fixes to whatever was chosen:

```sh
sed '/.*DBusActivatable.*/I d'      # ⛔ D-Bus activation cannot work in a bundle
sed -E 's/\s+setup\s+/ /Ig'         # strip a " setup " token from Exec
sed "s/Icon=[^ ]*/Icon=${PKG}/"     # ⭐ rewrite Icon to the bundled name
```

⚠ **`DBusActivatable` is the interesting one** — it is not cosmetic. A desktop
entry claiming D-Bus activation asks the session bus to start the program by
name, which cannot resolve to a file inside somebody's bundle.

**The `usr/` symlink** — the whole de-Nix step in one line: resolve
`squashfs-root/entrypoint` to its `/nix/store/<hash>` directory and make
`squashfs-root/usr` a **relative** symlink to it. That is what makes a nix
closure look like an AppDir.

**Dangling and self-referential symlinks** are pruned by an awk block that
resolves each link and removes it when the target does not exist, when it
points at itself, or when it points at its own parent.

## 6. The debloat rules, as a list rather than as a script

⭐ **Independently arrived at, and worth comparing against `internal/bundle`'s
sweep.** From the same recipe:

| class | action |
|---|---|
| `share/{locale,locales,fonts,man}` | ⛔ **deleted and replaced with a symlink to the HOST's `/usr/share/...`** — ⚠ **and the first of the two passes is buggy**: it matches `locale`, `locales`, `font`, `fonts` and `man`, then symlinks **every** match to `/usr/share/locale`, so a `share/fonts` directory becomes a link to the locale tree. A second loop then does it correctly per directory. Do not copy the first pass |
| `*.a *.cmake *.jmod *.gz *.md *.mk *.prf *.rar *.tar *.xz *.zip` | deleted |
| `LICENSE`, `LICENSE.md`, `Makefile` | deleted |
| dirs `doc/share`, `include`, `nix-support`, `share/docs`, `share/man` | deleted |
| dirs `ensurepip example examples gcc i18n mkspecs __pycache__ __pyinstaller test tests translation translations unit_test unit_tests` | deleted |
| ⭐ `*llvm*`, `*perl*`, `*systemd*` | **keep only `*.so*`, delete every other file** |
| `*LC_MESSAGES*` | deleted |

⛔ **The locale row is a HOST FALLBACK and this project has a design for that
class already** — [`../design/host-fallback.md`](../design/host-fallback.md),
T-065, four permitted classes. Symlinking to `/usr/share/locale` is *tier 4*:
it works where the host has it and silently degrades where it does not, which
is the failure mode `experiments/97-` found for timezones. ⚠ Adopting it needs
the same treatment: look first, carry a fallback, never prefer the stale copy.

## ⭐ 7. The block size — independent corroboration

`experiments/81-` swept the dwarfs block size on two subjects and shipped
`-S18`, a **256 KiB** block, for a 0.66× cold start. The same recipe packs with:

```sh
--mksquashfs-opt -b --mksquashfs-opt "1M"
```

⭐ **A production packager choosing a 1 MiB block for squashfs, against
`mksquashfs`'s own 128 KiB default and a long way from dwarfs' 64 MiB.** It is
a different filesystem so the numbers do not transfer, but the direction does,
and it was arrived at independently.

⚠ Their other settings, for the record: `--comp zstd`,
`-Xcompression-level 22`, `-root-owned`, `-no-xattrs`, `-noappend`,
`-mkfs-time 0`, `--no-appstream`.

## 8. ⛔ What the field records as BROKEN, which is the honest baseline

Of the **16** `nixappimage` recipes in `soarpkgs` at the pinned commit, **three
are disabled**, and one names the class the operator asked about:

| package | reason recorded |
|---|---|
| `ghostty` | ⛔ **"Fails to create EGL Display"** — cites `NixOS/nixpkgs#9415` |
| `lazarus` | "Exec Format Error" |
| `wget2` | "Doesn't do anything" |

⭐ **So EGL out of a nixpkgs closure is a known, recorded failure for the
people who do this in production** — and `experiments/85-` already runs EGL out
of a `pgb` closure at `pass=10 fail=0`. ⚠ Every row there is `swrast` and
surfaceless (T-059 owns real hardware), so the claim that can be made today is
*"the closure can produce a working EGL display offscreen"*, which is more than
the recipe above achieved and less than "the GPU problem is solved".

⚠ **13 of 16 are ACTIVE**, including chromium, brave, discord and telegram —
so the format works for large GUI applications. That is the baseline any claim
about nix bundling has to beat, and it is higher than "nix cannot do GUI".

## ⭐ 9. The vendoring model to copy, and it is a working one

⛔ **The operator's future task F1 asks for "a script/tool/bot auto wired into
our dev cycle where upstream's new commits/changes auto detected and
auto-diffed".** `Azathothas/bit-cli` at `cce8131231abe8b232054f3f27b3feeac19dd411`
already runs it over three upstream repositories and thirteen crates, and its
`patches/README.md` states the model in one line:

> **The model: the tree is the truth.** The vendored tree is edited in place,
> like any other source in this repository. `patches/<upstream>/*.patch` is
> **derived** from it and is never applied to anything.

⭐ **And the alternative is refused with reasons, which is the part worth
copying**: a pristine tree plus patches applied by a setup step *"was
considered and rejected: every edit then needs a refresh, a dirty tree is easy
to lose, and `rust-analyzer` reads the applied tree while the truth lives
somewhere else."*

⚠ **What the derived series buys, given the tree is already the truth**, is two
things a working tree cannot say: **review** (somebody else's code on its own,
without the 389 files around it) and **attribution** (Apache-2.0 asks a
distributor to mark changed files as changed).

**Three files and four scripts, and nothing else binds:**

| | |
|---|---|
| `vendor/upstream.json` | what is vendored, from where, at which commit |
| `patches/UPSTREAM.md` | every change made, and why — *"the part a script cannot write"* |
| `patches/TASKS.md` | the work the fork exists to do, in order |
| its `vendor-sync` script | put a tree in, or **three-way merge a new release onto ours** |
| its `vendor-diff` script | regenerate the series; ⭐ **`-Check` fails when the series and the tree disagree** |
| ⭐ its `upstream-scan` script | **everything upstream has, ranked against our open entries** |
| its `vendor-status` script | one screen: is the fork healthy, is a merge due |

⭐ **`upstream-scan` is F1's drift detector, already specified.** And
`upstream.json` carries the field that makes reconciliation possible:

> `base` is the commit our tree was last reconciled against. **It is not
> necessarily what the tree contains**: the tree is ours and may carry patches.
> That is the whole point of recording it.

⚠ **One convention transfers directly and this project already has the rule.**
`UPSTREAM.md`'s `Upstream:` field does **not** track a submission — nothing is
sent upstream — it answers *"could a release retire this patch on its own?"*
`TODO/RULES.md` §6 says the same thing here.

⛔ **The methodology this is an instance of is already vendored in this tree**:
[`../methodology/vendoring.md`](../methodology/vendoring.md), from
`Azathothas/TEMPLATE`. It is binding, and F1 is an application of it rather
than a new invention.

## ⭐ 10. The sharun fork's delta — a list of fixes we would otherwise rediscover

`pkgforge-dev/Anylinux-sharun` at `9728d8d7` states its own delta against
`VHSgunzo/sharun`, and several rows are things this project has already met or
is about to.

| what the fork adds | why it matters here |
|---|---|
| ⭐ auto-sets `MLT_REPOSITORY`, `MLT_PROFILES_PATH`, `MLT_PRESETS_PATH`, `FREI0R_PATH`, `LADSPA_PATH` | **that is kdenlive's engine.** `poc/80-mlt` and `experiments/90-` drive `melt`; this is the field's list of what MLT needs to find its modules |
| ⭐ auto-sets `QT_XKB_CONFIG_ROOT` | the **xcb** half of the operator's "egl, sdl, xcb issues", named explicitly |
| ⭐ `AppRun.sh` support — *"removes hard `/bin/sh` dependency from `AppRun`"* | ⭐ **this is a row in our own [`../comparison.md`](../comparison.md)**: the AppImage's delivery path picks up the host `/bin/sh` on distributions where it is dynamic. The fork fixes it |
| `lib`/`lib32` directly instead of `shared/lib` — *"Fixes libraries that look for a relative `../share`"* | ✅ **our AppDir already does this**: `lib` at the top level with `shared/lib -> ../lib` |
| bwrap-wrapper — intercepts `bwrap` so self-sandboxing apps (WebKitGTK) work | a class `pgb` has not met yet |
| `SHARUN_MESA_PATH` | switch the mesa implementation at run time |
| Bun workaround | Bun breaks when run through the dynamic linker directly |

⛔ **AND THE FORK REMOVED THINGS**: `lib4bin`, `sharun-aio`, `sharun-lite`, the
`xdg-open` wrapper and `--with-wrappe`. A fork whose surface shrinks is exactly
the case an unpinned dependency cannot survive.

### ⛔ Which is a defect in this tree, found by reading that list

`internal/bundle/appimage.go`'s constant block opens with

> *"Pinned, and the pin is the point: `latest/download` moves under you, so two
> runs a week apart produce AppImages with different runtimes and nothing in
> either says so."*

and then:

```go
defaultSharunURL = "https://github.com/pkgforge-dev/Anylinux-sharun/releases/latest/download/sharun-%s"
```

⚠ **uruntime and dwarfs are pinned to a tag; sharun is pinned to `latest`.** The
comment and the code disagree, and the URL-keyed cache added on 2026-09-03d
does not help: the URL never changes, so a warm cache keeps whatever it fetched
and a cold one takes whatever is current. ⛔ **Not changed here** — choosing a
tag means re-running the eleven-environment matrix, which is the measurement
the operator deferred on 2026-09-03d. It is named in `TODO/toolchain.md`
instead.

## ⭐ 11. `archlinux-pkgs-debloated` is a DIFFERENT LEVER CLASS from our sweep

⛔ **They do not delete files. They rebuild the package without the
dependency**, and that is why T-066's route B is expensive.

| package | what the `-mini` build removes |
|---|---|
| `mesa-mini`, `vulkan-*-mini` | the link to `libLLVM.so` — *"making any hardware accelerated app tiny as result"* |
| `llvm-libs-mini` | 150+ MiB → 99 MiB; `-nano` limits the targets to one arch + AMDGPU → under 70 MiB |
| `qt6-base-mini`, `libxml2-mini` | the 30 MiB `libicudata` dependency |
| `ffmpeg-mini` | the 20 MiB `libx265.so`, and AV1 *encoding* (decoding stays) |
| `sdl2_image-mini` | AVIF and JPEG-XL — *"~23 MiB combined that most apps never use"* |
| `-nano` variants | additionally built `-Os`, *"~30% smaller"*, ⚠ with a stated performance and stability risk |

⭐ **The nixpkgs equivalent is an `override`**, which forces a source build —
which is exactly `experiments/95-`'s measured cost: the whole `-mini` set
forces **161 of kdenlive's 676 closure paths (23.8%)** from source. ⚠ The
number now has a reason behind it rather than being a bare measurement.

⛔ **And it is a SIZE lever**, so `experiments/84-` applies: it buys
0.024–0.031 ms/MiB and cannot move the clock. Its value is size, which the
2026-09-03c ruling struck and the 2026-09-03d ruling deprioritised further.

## ⭐ 12. Why the upstream attempt failed, in its own code

⛔ **The operator named `ralismark/nix-appimage` issue #18 and PR #26 and said
the fix "was too naive & generic, didn't work 90% of the time".** Both are in
`references/ralismark__nix-appimage/api/issues.json`, and the code PR #26
shipped is `extra-files.sh` in the tree.

**Issue #18** is Azathothas asking `nix-appimage` to conform to the AppImage
format by carrying `.desktop` and icons, and quoting the very `mpv.sh` bash
§4 above dissects — so the whole loop is one story: the bundler did not do it,
the packager did it with `sed` and `awk`, and asked for it upstream.

**PR #26** — *"Best-effort copying in of .desktop file and icons"* — closed it.
Its own summary already concedes a gap: *".DirIcon most likely won't be a
256x256 PNG, but that requirement's a SHOULD not a MUST."*

⭐ **And two lines of `extra-files.sh` explain the 90%:**

```sh
drv="/nix/store/$hash"                          # the ENTRYPOINT'S OWN path
for d in "$drv/share/applications/"*.desktop; do
```

⛔ **It searches ONE store path — the entrypoint's own derivation output.** In
nixpkgs a program's `bin/` and its `share/` routinely live in different
outputs or different derivations (multi-output `bin`/`out`/`man`, and wrapper
derivations whose `bin/x` is a wrapper around a binary in another path
entirely). ⚠ And the match is exact-or-bail:

```sh
# only a .desktop whose Exec= basename equals the entrypoint's basename,
# and: "multiple .desktop entries found; giving up"
```

⭐ **So it fails whenever the desktop entry is not in the entrypoint's own
output, whenever `Exec=` differs from the binary name, and whenever two
entries match.** That is a single-derivation assumption, an exact-string
match, and an abort on ambiguity — three ways to return nothing.

⭐ **What ours must do instead, and `pgb` is already holding the thing that
makes it possible**: search the **whole closure**, not one store path; **rank**
candidates rather than bailing on ambiguity (the field takes the smallest
match); and rewrite `Icon=` to the name actually shipped. §5 has the field's
rules and [`bundle-capabilities.md`](bundle-capabilities.md) §2 has what the
package managers then require of the result.

---

## ⛔ What this sweep did NOT establish

`methodology/references.md` §5: *every measured claim ships the thing that
measured it.* ⭐ Three findings here shipped one:

1. the constants are patchable strings **in our own artefact** —
   `strings -a`, quoted above, re-runnable;
2. `lite` drops `dwarfsck`/`mkdwarfs` — the fork's build task list and the
   `#[cfg]` gates, plus this project's own six-for-six compressor test;
3. the 5 s reuse window — `experiments/99-` measured it before the constant
   was read.

⚠ **Everything else on this page is a READING**, and the three probes that
would settle the rest are, each one command:

1. ⛔ **does patching `URUNTIME_EXTRACT=3` to `=1` change cold start** — one
   byte, `experiments/81-`'s artefacts, `experiments/clock.sh`'s control;
2. ⛔ **is kdenlive's 114 ms warm row the extract path** — the same patch, on
   an artefact either side of 350 MB;
3. **does the exact-closure rewrite beat the five-regex cascade** — the corpus
   is `soarpkgs`' 13 active recipes and the measure is how many store paths
   each leaves behind.
