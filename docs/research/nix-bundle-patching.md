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

⚠ **Applied only to files that are not ELF executables**, selected with
`file -i "$1" | grep -qiv "application/.*executable"`. So: text-patch scripts,
leave binaries alone.

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

**Icon** — the smallest `.png`/`.svg` under `usr/`, *excluding* `favicon` and
the small size buckets (`16x16` … `96x96`); if that fails, retry restricted to
`128x128/apps` or `256x256` and matching the package name. Copy to
`<pkg>.png` **and** `.DirIcon`.

**Desktop** — the smallest `*.desktop` under `usr/`, then three fixes:

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
| `share/{locale,locales,fonts,man}` | ⛔ **deleted and replaced with a symlink to the HOST's `/usr/share/...`** |
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
