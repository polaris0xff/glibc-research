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
   the boundary in production code. ⚠ **It is built through `pkgsStatic`,
   which is musl** — inferred, not measured, and ⛔ **checking it is the first
   thing this entry should do** (`readelf -lW`, three commands, in
   [`../docs/research/portable-nix-mechanisms.md`](../docs/research/portable-nix-mechanisms.md) §1).
   ⭐ **So T-051 and T-060 are NOT the same work**: a musl static nix serves
   "enough nix on a minimal host"; only T-060 needs a glibc one.
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
| `jq` size (`experiments/86-`) | 11,471,610 B | 4,006,916 B | 2.86× | ⭐ acceptable |
| kdenlive size (`experiments/90-`) | 471,033,944 B | 191,900,604 B | 2.45× | ⭐ acceptable |
| kdenlive render | 4,947 ms | 2,033 ms | ⛔ 2.43× | ⛔ **binding** |
| kdenlive cold start | 300 ms | 61 ms | ⛔ 4.92× | ⛔ **binding, worst column** |

⛔ **THE TWO MILLISECOND ROWS ARE NOT RE-DERIVABLE FROM THE TREE — deep review
1, 2026-09-03c.** They are from `git show
68be1bcd:evidence/90-kdenlive-vs-enhanced/RESULT.txt`; the file at that path
today is run 6, which says 24,074 / 13,680 and 5,941 / 1,183, and whose
milliseconds this entry itself disclaims as contaminated. ⚠ **Four runs of the
same comparison give cold-start ratios of 2.52×, 3.48×, 4.92× and 5.02×, and
in two of them WARM IS SLOWER THAN COLD.** The direction survives — ours is
slower on every run — the magnitude does not.
`../docs/history/corrections.md` C23.

**What is left, in the order the ruling puts it.**

0. ⛔ **FIX THE INSTRUMENT BEFORE MEASURING ANYTHING WITH IT.** `90-` takes
   **one sample per arm**; `86-` takes eleven environments × a mean of five,
   with cold obtained by a fresh copy. Carry `86-`'s method into `90-`.
   ⚠ Under the new bar an unpinned millisecond is worth less than none,
   because it reads as measurement.
1. ⛔ **RE-MEASURE THE LEVERS ON THE CLOCK.** `--cut`, `--fixpoint`, the
   debloat rules, route A and route B were all costed in **bytes**. None was
   measured in **milliseconds**, which is now the only thing that scores.
   ⚠ Nothing is invalidated; everything is un-scored.
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
6. ⭐ **A LEVER WE DO NOT HAVE AT ALL, from the 2026-09-03c sweep**:
   `xplshn/pelf` chooses **extraction over FUSE mounting above 350 MB**
   (`appbundle-runtime.go:764`, mode 3). ⛔ **Our kdenlive bundle is 398 MB and
   the competitor's is 192 MB** — one either side of somebody else's
   production threshold, which is independent corroboration of the hypothesis
   in step 0's N2. ⚠ It is a default in one project, not a benchmark.
   [`../docs/research/portable-nix-mechanisms.md`](../docs/research/portable-nix-mechanisms.md) §3.

**Prove.** A CLI subject bundled by one command, with startup and run time
**under** the field's on the same machine on the same day, and the eleven-row
coverage column unchanged.

📚 [detail](../HISTORY/entries/toolchain-open.md) — the AppDir-vs-artefact
ratio (7.5:1), the six runs, the debloat accounting and the contaminated-clock
correction all live there.
