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

1. ⛔ **There is no static nix to fetch** — measured 2026-09-03c; nixpkgs
   ships none, and seven of `nix`'s eleven `DT_NEEDED` are its own component
   libraries, which is T-060 rung 1. ⭐ **The operator's answer is to BUILD
   one**: *"implement a mix of existing techniques by iterating/improving
   them and publishing a 'static' nix ourself"*.
2. ⚠ **The `--store` half is unmeasured.** The fetched closure is
   self-contained (all 11 `DT_NEEDED` resolve inside its 64 paths); whether
   that nix will operate a store under `$HOME` needs a rootfs, because this
   build host has nix installed.
3. `nix-user-chroot` where namespaces are available, with the AppArmor case
   detected and named rather than hit.

⛔ **Not `curl | sh` as root.**

**Reading owed** — named by the operator 2026-09-03c, all three mined the same
day, three passes each and the two write-ups, ⛔ **not delegated to a
sub-agent** ([`../docs/methodology/references.md`](../docs/methodology/references.md)):

| reference | commit |
|---|---|
| [`DavHau/nix-portable`](../references/DavHau__nix-portable/PROVENANCE.md) | `91122e3d94ba51d7d83fe990fa81d3de0968fb32` |
| [`nixie-dev/nixie`](../references/nixie-dev__nixie/PROVENANCE.md) | `d14c6c370489ec13b24d65df569e7769444ebebf` |
| [`containerbase/nix-prebuild`](../references/containerbase__nix-prebuild/PROVENANCE.md) | `9302079d1cb625307f195273cee4632648ecbaec` |

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

⚠ **Named risks:** boost, libgit2, libarchive, lowdown, editline, sqlite,
curl+openssl, libsodium, brotli, toml11 and the AWS CRT. Any one can refuse
`-static` the way MLT's `add_library(mlt SHARED)` did.

⭐ **And the operator has said what the answer looks like** — not "find a static
nix" but publish one; see T-051's reading list, which is this entry's too.

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

**Prove.** A CLI subject bundled by one command, with startup and run time
**under** the field's on the same machine on the same day, and the eleven-row
coverage column unchanged.

📚 [detail](../HISTORY/entries/toolchain-open.md) — the AppDir-vs-artefact
ratio (7.5:1), the six runs, the debloat accounting and the contaminated-clock
correction all live there.
