# toolchain — pgb itself

Design: [`../docs/design/toolchain.md`](../docs/design/toolchain.md).

⚠ **Open entries only.** The 15 closed ones are
[`../HISTORY/entries/toolchain.md`](../HISTORY/entries/toolchain.md); the
long-form findings behind the entries below are
[`../HISTORY/entries/toolchain-open.md`](../HISTORY/entries/toolchain-open.md).
⛔ Read the detail before re-running anything — most of it has been run once.

---

## T-012 — `pgb build <url-or-package>`

**Source** operator · **Category** toolchain · **Priority** P1 · **Effort** XL · **Status** open

**Problem.** A developer still has to know how to build the project.
`pgb build -- make` is a toolchain injector, not a toolchain.

⭐ **Narrower than "XL" — the entry said "split into three before starting"
and all three exist.** `pgb nix build --plan` resolved the spec, detected the
build system per dependency and walked 41 dependencies in one command,
producing a static PostgreSQL 18.6 (T-063). What is left:

1. ⛔ **The URL route.** Nothing resolves a git URL to a source tree. The only
   half with no code at all. ⚠ **DEFERRED by the operator, 2026-09-03c** —
   *"we can leave the build from git/url deferred for now"*.
2. ⚠ **The report.** The adaptation loop knows what it dropped
   (`round N: FAILED -> drop:--with-llvm`); nothing collects it for the
   developer, which `design/toolchain.md` requires.
3. ⚠ **Joining static-first to bundle-last.** `pgb nix build` and
   `pgb bundle appimage` are two commands with no path between them.
4. ⛔ **"No nix" does not hold for a dotted attribute** — `pgb nix plan
   kdePackages.kdenlive` falls back to evaluation. T-060 owns it.

**Prove.** `pgb build <a git URL>` produces a binary `pgb verify` passes on
11 of 11, with no other input from the operator.

📚 [detail](../HISTORY/entries/toolchain-open.md)

## T-013 — Measure developer friction

**Source** operator · **Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

**Problem.** `docs/comparison.md` states the friction axis from one session's
record. Nothing re-runs it, so it goes stale silently.

⭐ **Promoted by the operator's ruling of 2026-09-03c**, which makes
*"packaging is just one command not a multiline shell script"* half of the
bundler's bar. This entry is the instrument that measures that half.

**What is left.** `experiments/63-developer-friction.sh`: count external
artefacts fetched, files authored, environment variables required, and whether
each route completes unattended.

**Prove.** `sh experiments/63-developer-friction.sh` exits 0 and
`evidence/63-developer-friction/RESULT.txt` carries the counts.

📚 [detail](../HISTORY/entries/toolchain-open.md)

## T-015 — the OCI pull unpacks the filesystem and drops the image config

**Source** found while cross-checking the two `pgb verify` arms (T-014).
**Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

**Problem.** An OCI image is a filesystem **and** a configuration.
`internal/ociimg/pull.go` unpacks the layers and ignores the config, so the
chroot bed and a `docker run` of the same digest are not the same environment.
⭐ Measured: the `archlinux` config carries `Env: LANG=C.UTF-8`, `docker run`
applies it, the chroot bed sets no `LANG` at all.

**What is left.** Record `Env`/`Cmd`/`Entrypoint`/`WorkingDir` into
`.oci-provenance` and have `pgb rootfs run` apply `Env`. ⛔ **Applying it must
be a decision, not a silent default** — a bed that starts exporting `LANG`
changes what every locale-sensitive result describes.

**Prove.** `pgb verify --engine chroot` and `--engine docker` on the same
binary produce the same HOST DATA column on all eleven, and
`experiments/30-gconv-and-locale.sh` is re-run and compared.

📚 [detail](../HISTORY/entries/toolchain-open.md)

## T-051 — Enough nix for a host with no root, no docker and no nix

**Source** operator, 2026-09-01c: *"find the least invasive way to 'install'
enough nix so pgb is usable even on the minimal of hosts like containers that
can't run docker images or install nix because no root"*.
**Category** toolchain · **Priority** P1 · **Effort** M · **Status** open

**Problem.** T-050 removed nix from *planning* for packages whose `.drv` is
cached. It did not remove it from the cases T-050 lists as out of reach.

⭐ **Step 1 is done and went further than the entry expected**:
`experiments/88-` arm 5 planned, fetched and built jq in a rootfs with uid
12000, no nix, no `/nix` and no root — 8 assertions, 0 fail.

**What is left.**

1. ⛔ **"There is no static nix to fetch" — CORRECTED 2026-09-03c BY READING
   THE REFERENCES.** nixpkgs ships none; ⭐ **the NIX FLAKE DOES**, and three
   projects consume it. The attribute is `nix-static` before nix 2.26.0 and
   **`nix-cli-static`** from 2.26.0 on — `containerbase/nix-prebuild` carries
   the boundary in production code. ⭐ **AND IT IS MEASURED**, not inferred:
   `experiments/98-`, pass=7 fail=0 — PT_INTERP 0, DT_NEEDED 0, target triple
   `x86_64-unknown-linux-**musl**`, and ⭐ **`nix --version` on 11 of 11**.
   ⛔ **The obvious probe lies**: `strings | grep 'GNU C Library'` returns 1
   and that 1 is a licence sentence; the store path is the discriminator.
   ⭐ **So T-051 and T-060 are NOT the same work**: this binary already serves
   "enough nix on a minimal host" and is one pinned fetch; only T-060 needs a
   glibc one.
2. ⛔ **THE `--store` HALF IS NO LONGER UNMEASURED — SOMEBODY ELSE MEASURED IT
   AND IT FAILS.** `nix-portable` has shipped `nix --store` as its
   **first-choice** runtime since 2021. Its issue **#98**, nine comments,
   three independent reporters: `error: setting up a private mount namespace:
   Operation not permitted` on **Arch, Debian 11 and Debian 12**, on systems
   where user namespaces demonstrably work and `bwrap` succeeds.
   ⛔ **And its probe is a FALSE POSITIVE** — it builds a trivial derivation,
   passes, and the real workload then fails. ⭐ **A probe must use the
   workload, not a token.** So step 2 as written is not a route to "works on
   a minimal host"; the FALLBACK CHAIN is — `nix --store → bwrap → proot`,
   each probed, the answer cached.
3. `nix-user-chroot` where namespaces are available, with the AppArmor case
   detected and named rather than hit.

⛔ **Not `curl | sh` as root.**

⭐ **THE READING IS DONE, 2026-09-03c.** All three references named by the
operator were mined and read — findings in
[`../docs/research/portable-nix.md`](../docs/research/portable-nix.md), the
mechanisms with their file-and-line citations in
[`../docs/research/portable-nix-mechanisms.md`](../docs/research/portable-nix-mechanisms.md).
⚠ **Nothing in that sweep was RUN**, and it says so: items 1 and 2 above are
readings, and each names the probe that would settle it.

**Prove.** `pgb nix plan` and `pgb nix build` on a rootfs with no nix, no
root and no compiler.

📚 [detail](../HISTORY/entries/toolchain-open.md)

## T-060 — ⭐ STATIC-GLIBC nix: the entry that makes "no root, no docker, no nix" true

**Source** ⭐ **operator, 2026-09-01e**: *"nixpkgs' pkgsStatic is musl and this
project is the glibc half. Produce a static-glibc nix toolchain path end to
end … `nixie-dev/nixie` is the shape the operator named."*
**Category** toolchain · **Priority** P1 · **Effort** L · **Status** open

**Problem.** `experiments/88-` arm 5 needs **a C toolchain on the host**. Two
cases are left: a host with no compiler at all, and the attributes the index
route cannot reach by construction (an override, an overlay, `pkgsStatic.*`),
which need **evaluation**, and evaluation needs a nix binary. ⛔ nixpkgs has no
static-glibc nix to fetch — `pkgsStatic` is musl.

**The three rungs.**

| rung | what | where it stands |
|---|---|---|
| 1 | nix's closure built static-glibc by `pgb` | ⚠ **in progress** — `pgb nix deps` walks it; first pass **24 built, 32 failed**, and it found **eleven defects**, five of them in `pgb` itself |
| 2 | nix itself linked against them: a `nix-instantiate` with no `PT_INTERP` | not reached |
| 3 | that binary evaluating a nixpkgs attribute in a rootfs with no nix, no `/nix` and no root, on the eleven | not reached |

⭐ **Rung 1's premise is measured**: the eight component derivations are **not**
index attributes (`nix-cli` → no attribute; `nix` is an aggregator whose 7
buildInputs are test-runs), but they ARE reachable — `pgb nix plan nix`
fetches 27 derivations and 24 nix components land in the `.drv` store.

⭐ **AND THE SWEEP OF 2026-09-03c EXPLAINS WHY, WHICH NARROWS THE WORK.**
`nix-cli` is not a nixpkgs attribute because it is an attribute of the **nix
flake**, and that flake publishes **`nix-cli-static`** (`nix-static` before nix
2.26.0) — consumed by `nix-portable`, `nixie` and `containerbase/nix-prebuild`
alike. ⚠ Built through `pkgsStatic`, i.e. **musl**, which is exactly the half
this entry does not want; ⛔ **that is inferred and unmeasured, and checking it
is three commands** ([`../docs/research/portable-nix-mechanisms.md`](../docs/research/portable-nix-mechanisms.md) §1).
⭐ So the target has a NAME and a BUILD RECIPE now: rung 1 is
`nix-cli-static`'s dependency closure, built glibc-static by `pgb` rather than
musl-static by `pkgsStatic`.

⚠ **AND THE OPERATOR'S NAMED SHAPE HAS A PROBLEM WORTH KNOWING.** `nixie` is
"the shape the operator named", and its entire premise is nix's own
local-store sandbox — the mechanism `nix-portable`'s issue #98 documents
FAILING on Arch, Debian 11 and Debian 12, with a probe that passes first.
`nixie` is also alpha by its own README and its Linux patch sets are empty.
⭐ **What it IS still worth**: it is the only reference in the corpus that
builds nix from source with patches rather than consuming a release, which is
what rung 1 has to do. `docs/research/portable-nix.md` finding 3.

⚠ **Named risks:** boost, libgit2, libarchive, lowdown, editline, sqlite,
curl+openssl, libsodium, brotli, toml11 and the AWS CRT. Any one can refuse
`-static` the way MLT's `add_library(mlt SHARED)` did.

⭐ **And the operator has said what the answer looks like** — not "find a static
nix" but publish one. ⭐ **The reading is DONE** (2026-09-03c):
[`../docs/research/portable-nix.md`](../docs/research/portable-nix.md) and its
[mechanisms page](../docs/research/portable-nix-mechanisms.md). ⚠ Nothing in it
was run.

**Prove.** `evidence/89-static-nix/RESULT.txt`: the rung reached, with the
dependency-by-dependency table and what stopped each one it could not build.

📚 [detail](../HISTORY/entries/toolchain-open.md)

## T-066 — ⛔ P0: the bundler is SLOW. Rebuild it against a CLI benchmark

**Source** ⭐ **operator, 2026-09-02b**: *"pgb bundle isn't good enough, it is
bloated, slow and a complete failure. Restudy what all nixappimage related
references and fix/patch/reimplement/iterate+improve everything needed to fix
our bundles. best place is to bundle a cli first, bundle something like bash or
maybe 7z which can be benchmarked better, and smaller and less time to compare
after each iteration/improvements."*
**Category** toolchain · **Priority** P0 · **Effort** XL · **Status** open

⛔ **WORK UNTIL IT IS MET OR THE PREMISE IS SIGNIFICANTLY ADVANCED.**

⭐ **"Bloated" was WITHDRAWN by the operator on 2026-09-03c** — *"us having a
bigger size than anylinux-appimages and onelf is acceptable as long as ours
performs better and packaging is just one command not a multiline shell
script"*. `../docs/design/toolchain.md` carries the amendment.

| subject | ours | the field | | under the ruling |
|---|---|---|---|---|
| ⭐ **`jq` cold start** (`experiments/86-`, 11 environments) | **58.3 ms** | 58.4 ms | ⭐ **1.00×**, ours faster on 6 of 11 rows | ⭐ **MET** |
| `jq` warm start | 8.5 ms | 9.3 ms | ⚠ **no difference measurable** — medians equal, and the column's arithmetic is unverified (T-057) | ⚠ not against us |
| `jq` size | 6,806,407 B | 4,006,949 B | 1.70× | ⭐ struck from the bar |
| kdenlive size (`experiments/90-`) | 471,033,944 B | 191,900,604 B | 2.45× | ⭐ acceptable |
| kdenlive render | 4,947 ms | 2,033 ms | ⛔ 2.43× | ⛔ **binding, UNMEASURED since the levers** |
| kdenlive cold start | 300 ms | 61 ms | ⛔ 4.92× | ⛔ **binding, and the number is not trustworthy** |

⭐ **THE SPEED HALF OF THE BAR IS MET ON A CLI, 2026-09-03d, and the closure
did not change.** Two constants in `internal/bundle/appimage.go` did:

| jq cold, ours vs the field | what changed |
|---|---|
| 2.07× | — |
| **1.28×** | uruntime v0.5.6 **full** → v0.5.9 **lite** — `experiments/77-`. ⚠ It is `lite` that pays; the *version* bump alone is 1.00× and does not resolve |
| ⭐ **1.00×** | dwarfs block `-S26` (64 MiB) → `-S18` (256 KiB) — `experiments/81-`. dwarfs decompresses a whole block to serve any byte in it |

⛔ **The bundler was being compared against a runtime it did not ship.**
`experiments/86-` stages the competitor's toolchain from `references/`, and
that toolchain is uruntime **lite**. Nothing in the record had noticed.

⛔ **THE KDENLIVE ROWS ARE NOT RE-DERIVABLE, AND NOW FOR THREE REASONS.**
They are from `git show 68be1bcd:evidence/90-kdenlive-vs-enhanced/RESULT.txt`;
the file at that path is a different run, which this entry itself disclaims as
contaminated. ⚠ Four runs of the same comparison give cold-start ratios of
2.52×, 3.48×, 4.92× and 5.02×, with warm above cold in two.
⭐ **C24 found why**: `90-`'s cold column obtained "cold" by COPYING the file,
and uruntime keys its mount on CONTENT, so the copy reused the live mount and
the column reported a warm start — 1.02× of warm, measured.
⛔ **And all four rows predate both levers above.**
`../docs/history/corrections.md` C23 and C24.

**What is left, in the order the ruling puts it.**

0. ✅ **THE INSTRUMENT IS FIXED.** `experiments/clock.sh` — median of N, arms
   interleaved with a rotating start, and an **A/A control** (one artefact
   under two names) whose ratio is the floor below which no row may be
   believed. `experiments/99-` stands it up and asserts it. ⭐ `90-` now uses
   it: cold is obtained by REAPING the live mount, not by copying the file.
1. ⛔ **RE-MEASURE kdenlive.** ⚠ This item used to read *"re-measure the levers
   on the clock"* — that premise is **gone**: `experiments/84-` measured image
   size at **0.024–0.031 ms/MiB**, so the byte levers (`--cut`, `--fixpoint`,
   the debloat rules) cannot move the clock and re-measuring them is not
   promising. ⭐ What IS left is the SUBJECT: the two levers that worked are
   properties of the runtime and the packer, so they should carry to kdenlive
   — and *should carry* is a reason to measure, not a substitute for it.
2. ⭐ **Route B is costed and it overturns the argument against it**: the whole
   `-mini` set forces **161 of kdenlive's 676 closure paths (23.8%)** from
   source — not "the entire KDE/Qt subtree". On mesa-demos it is 8 of 111.
   `experiments/95-`, pass=3 fail=0.
3. ⛔ **Route A is DEAD at path granularity**, measured: 0 of jq's 7 store
   paths are entirely unreachable, 6 of 7 are 100% kept, and glibc carries
   every deletable byte.
4. ⚠ **The two size levers are additive** and the fixpoint inherits the
   baseline's risk rather than adding one — 0 reachable objects mention the
   seven it drops.
5. ⭐ **The `--fixpoint` lever landed as a measuring device**, not yet as a
   default.
6. ⛔ **"A LEVER WE DO NOT HAVE AT ALL" WAS WRONG, corrected 2026-09-03d.**
   This cell said `xplshn/pelf`'s extract-over-mount above 350 MB
   (`appbundle-runtime.go:764`, mode 3) was a lever `pgb` lacks. ⭐ **uruntime,
   which `pgb` already ships, exposes the same selector** — `URUNTIME_EXTRACT`,
   `URUNTIME_MOUNT`, `URUNTIME_CLEANUP` and `REUSE_CHECK_DELAY` are in the
   strings of the artefact `pgb bundle appimage` produces, with `=0`, `=2` and
   `=3` among them. It is a lever `pgb` does not **set**.
   ⛔ **AND "ENVIRONMENT VARIABLES ONLY" WAS WRONG TOO — corrected the same
   day by reading the fork's source.** They are **compile-time constants laid
   out as patchable ASCII**: `const URUNTIME_EXTRACT: &str =
   "URUNTIME_EXTRACT=3"`, read back through `.replace("URUNTIME_EXTRACT=",
   "=")`, and `strings -a` finds them **in the artefact `pgb` ships**. A
   one-byte same-length overwrite before packing changes the mode with nothing
   beside the artefact. ⭐ **And mode 3 means mount below 350 MB, extract
   above — so our 565 MB kdenlive bundle is ALREADY EXTRACTING and the 6.8 MB
   `jq` one is mounting**, which is why a lever measured on one does not
   transfer to the other.
   [`../docs/research/nix-bundle-patching.md`](../docs/research/nix-bundle-patching.md) §1.
   [`../docs/research/portable-nix-mechanisms.md`](../docs/research/portable-nix-mechanisms.md) §3.
7. ⭐ **WHAT NOBODY HAS TRIED, and it is now the top of the list**: the two
   levers on **kdenlive**. Both are properties of the runtime and the packer,
   not of the payload, so the 4.92× cold row should move the way `jq`'s did.
   `experiments/90-` carries the corrected protocol; it needs a run.

⛔ **A DEFECT FOUND BY READING THE FORK'S CHANGELOG, 2026-09-03d, NOT FIXED.**
`internal/bundle/appimage.go`'s constant block says *"Pinned, and the pin is
the point: `latest/download` moves under you"* — and then pins **sharun** to
`releases/latest/download`. uruntime and dwarfs carry tags; sharun does not.
⚠ The URL-keyed cache added the same day does not help: the URL never changes,
so a warm cache keeps whatever it has and a cold one takes whatever is current.
⛔ **`Anylinux-sharun` is a fork whose surface SHRINKS** — it has already
removed `lib4bin`, `sharun-aio`, `sharun-lite`, the `xdg-open` wrapper and
`--with-wrappe`. ⚠ Not changed here because choosing a tag means re-running the
eleven-environment matrix, which the operator deferred on 2026-09-03d.
[`../docs/research/nix-bundle-patching.md`](../docs/research/nix-bundle-patching.md) §10.

**Prove.** A CLI subject bundled by one command, with startup and run time
**under** the field's on the same machine on the same day, and the eleven-row
coverage column unchanged.
⭐ **MET on `jq`, 2026-09-03d** — cold 1.00×, warm 0.92×, 11 of 11 both arms,
zero host shared objects both arms (`experiments/86-`, `evidence/`). ⚠ Parity
is not "under"; the honest sentence is *no difference measurable on cold,
possibly ahead on warm*.
⭐ **AND MET ON kdenlive TOO, the same day** — `experiments/90-` on the
corrected protocol: cold **380.2 vs 513.9 ms, 0.74×, ours faster**, host
objects **0 of 11 against the competitor's 4 of 11**. ⚠ Its warm row is 3.45×
against us and unexplained, and its render direction is unresolved (two runs
disagree). T-055 carries the table.

⛔ **THE ENTRY STAYS OPEN ON ONE COLUMN ONLY: SIZE.** 1.70× on `jq` and 2.95×
on kdenlive, and both moved the WRONG way to buy the speed — `-S18` costs
+17.8% of a real payload. Goal 3 names *smaller* for kdenlive specifically, so
this is not nothing.

⛔ **BUT DO NOT START HERE, AND THE PRIORITY SAYS OTHERWISE — read this
first.** This entry is **P0**, so the ordering in
[`INDEX.md`](INDEX.md) puts it above everything; its remaining column is
**size**, which the operator **struck** on 2026-09-03c and **deferred** on
2026-09-03d (*"Defer comparing speed/startup/performance with
anylinux-appimages & onelf for now"*). ⚠ A P0 whose only open column is a
deferred axis will route a session into work it was told not to do.
⭐ **[`PROGRESS.md`](PROGRESS.md)'s work order is what decides**, and it names
T-078 to T-081. The priority is kept at P0 because the operator set the
premise and has not retired it — ⛔ **not** because this is what to work on
next.
⚠ And `experiments/84-` measured that the debloat levers cannot buy the size
back on the clock anyway: 0.024–0.031 ms/MiB.

📚 [detail](../HISTORY/entries/toolchain-open.md) — the AppDir-vs-artefact
ratio (7.5:1), the six runs, the debloat accounting and the contaminated-clock
correction all live there.

## T-082 — Vendor and patch the third-party runtime and tooling, with drift detection

**Source** ⭐ **operator, 2026-09-03d**: *"Vendor & Patch as many third party
runtime or tooling we use, because upstream has bugs/feature requests still
sitting/stale, and also because we can now implement a much better & focused
version only useful for us ... we must have a script/tool/bot auto wired into
our dev cycle where upstream's new commits/changes auto detected and
auto-diffed."*
**Category** toolchain · **Priority** P2 · **Effort** XL · **Status** open

⛔ **NOT THE NEXT SESSION — operator, 2026-09-03d.** That session is scoped
to **T-078, T-079 and T-080** and nothing else. This entry is recorded now so
it is not rediscovered; ⚠ it is not to be started, and not to be "just
quickly" looked at. [`PROGRESS.md`](PROGRESS.md) carries the scope.

**Problem.** ⛔ **The cost of not having this is measured, not hypothetical.**
This project spent a session discovering that the field runs a `lite` uruntime
and a different block size. Both were free wins sitting in somebody else's
build flags, worth **0.69×** and **0.66×** of cold start, and nothing tracked
them. ⚠ And the pkgforge builds are themselves forks, so we are two levels
behind upstream, not one.

⚠ **A second instance, same week:** `defaultSharunURL` is pinned to
`releases/latest/download` in a constant block whose own comment says
`latest/download` is the thing not to do — and `Anylinux-sharun` is a fork whose
surface has already SHRUNK (it removed `lib4bin`, `sharun-aio`, `sharun-lite`,
the `xdg-open` wrapper and `--with-wrappe`).

**What is left.** All of these are already vendored and pinned in
`references/`: `pkgforge-dev/Anylinux-AppImages`, `Anylinux-uruntime`,
`Anylinux-sharun`, `appimagetool`, `archlinux-pkgs-debloated`,
`userland-execve-rust`.

⭐ **THE MODEL IS SPECIFIED AND HAS A WORKING INSTANCE.** The methodology is
[`../docs/methodology/vendoring.md`](../docs/methodology/vendoring.md), already
vendored and binding; the worked example is `references/Azathothas__bit-cli`,
which runs it over three repositories and thirteen crates. Its shape, verbatim:

> **The model: the tree is the truth.** The vendored tree is edited in place.
> `patches/<upstream>/*.patch` is **derived** from it and is never applied.

with a manifest recording the commit each tree was last **reconciled** against,
a written record of every change made, and four scripts — sync (three-way merge
a new release onto ours), diff (regenerate the series; **`-Check` fails when
the series and the tree disagree**), ⭐ **scan (everything upstream has, ranked
against our open entries)**, and status.

**Prove.** A drift detector that runs in the dev cycle and reports what
upstream has that we do not — and a first report from it.

## T-083 — Native desktop integration: our bundles as ordinary AppImages

**Source** ⭐ **operator, 2026-09-03d**: *"'Native' desktop integration &
appimage 'compatibility' for our nix bundled packages, so third party package
managers can use/integrate our bundles as native appimages."*
**Category** toolchain · **Priority** P2 · **Effort** M · **Status** open

⛔ **NOT THE NEXT SESSION — operator, 2026-09-03d.** That session is scoped
to **T-078, T-079 and T-080** and nothing else. This entry is recorded now so
it is not rediscovered; ⚠ it is not to be started, and not to be "just
quickly" looked at. [`PROGRESS.md`](PROGRESS.md) carries the scope.

**Problem.** ⭐ **The contract is measured, and we already meet most of it** —
[`../docs/research/bundle-capabilities.md`](../docs/research/bundle-capabilities.md) §2.
`gio info` reports our artefact as `application/vnd.appimage`, which is
`gearlever`'s actual gate; `--appimage-mount` prints and holds a mountpoint,
which is `AM`'s; `--appimage-extract` works; and `soar` reads dwarfs directly.

⛔ **Two real gaps, both one line in `internal/bundle/appimage.go`:**

1. **`X-AppImage-Version=`** is absent from our desktop entries and is read by
   **two** of the four managers (`gearlever`, `AppManager`). The version is
   already known — it is in the derivation.
2. **`.DirIcon` is absent when the closure carries no icon**, and the generated
   entry still says `Icon=<pkg>` — a dangling reference, which is worse than an
   absent one. Either emit a fallback or omit the key.

⚠ **And a third file the field treats as part of the contract**: AppStream
metadata under `usr/share/metainfo/`, which `soar` looks for (`find_appstream`)
and the `soarpkgs` recipe creates explicitly.

**What is left.** Close the two gaps, decide on AppStream, then mine each
manager for what it does with the result: `mijorus/gearlever`,
`kem-a/AppManager`, `ivan-hc/AM`, `pkgforge/soar` — all four vendored.
⚠ **Depends on T-081**: a bundle whose `.desktop` still names store paths
integrates a broken launcher.

**Prove.** Each of the four managers installs and launches a `pgb` bundle, with
an icon and a working launcher, and the failure of any one named with its
reason.

---

## T-092 — the `.env` names a farm directory the farm may not have created

**Source** a deep review of the store-path patterns, 2026-09-04.
**Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

`StoreRefToBundle` writes `${SHARUN_DIR}/store/<short name>` into a lifted
`.env` value. `buildStoreFarm` creates `store/<shortStoreName(base)>` — **unless
two closure entries share that short name**, in which case it falls back to the
full `<hash>-<name>` to disambiguate. ⛔ The `.env` side has no such fallback,
so on a closure with a short-name collision the value points at a directory
that does not exist.

⚠ **Not observed in a bundle**, and it is cheap to observe: a closure carrying
two revisions of one package. ⭐ **And nothing would catch it** — `integrity()`
walks `DT_NEEDED` and `manifestIntegrity()` reads the ICD manifests; **no check
reads a `.env` value back against the tree.** That absence is the finding, and
it is the same absence that let `${SHARUN_DIR}` expand to nothing for a whole
session.

**Prove.** A check that resolves every `${SHARUN_DIR}`-relative path in `.env`
against the AppDir and fails on a miss, plus a selftest closure with a
deliberate short-name collision.

## ⭐ BOTH HALVES LANDED, and the third slice was found by the check itself

| | |
|---|---|
| **one naming rule** | `farmDirName` — the collision-aware farm directory name, memoised from the **closure**, so `writeEnv` (before the farm exists) and `storefix` (after) cannot disagree. `carryBakedPaths` sliced `base[33:]` inline with no fallback; it calls `farmDirName` now |
| **the missing check** | `envIntegrity()` reads every `${SHARUN_DIR}/store/…` value in `.env` back against the tree and names the variable that does not resolve. ⚠ It **warns** rather than failing: `.env` legitimately names paths the debloater removed, and a dangling override is ENOENT either way — what must not happen is that nobody is told |
| **a structural gate** | `TODO/check.sh` check 9: the `[33:]` slice may appear **only** inside `shortStoreName` |

⭐ **The gate found a defect on its first run, which is the argument for having
it.** Two more hand-slices existed — `deriveProgramName` and `storeVersion` —
that no value test could have seen, because a third caller that never calls the
shared function is invisible to a test of the shared function. ⚠ Both wanted
the **pure** slice rather than the farm rule, and now call `shortStoreName`:
giving a *program* the farm's collision fallback would name it after a hash.

⛔ **AND ONE OF THE NEW SELFTEST CASES COULD NOT FAIL**, in the commit citing
C28 for exactly that. "The farm and the `.env` agree" compared `farmDirName`
with `buildStoreFarmNames` — both of which *call* `farmDirName` — so it passed
under a planted regression that made the other two fail. ⭐ It now compares the
**strings the two sides construct**, and was checked against two plants: a
changed format string in `bakedOverride` (it fails) and the naming regression
(the collision cases fail). `bundle-appimage --selftest`: **53 cases**.

⚠ **Still not observed in a real bundle**, and that is unchanged: no closure
here carries two builds of one package. What changed is that the divergence is
now impossible to reintroduce silently — one rule, a gate on the slice, and a
check that reads `.env` back.
