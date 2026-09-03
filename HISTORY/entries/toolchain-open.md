# HISTORY/entries/toolchain-open.md — retired DETAIL of toolchain entries that are STILL OPEN

⚠ **These entries are open. This file is not the entry** — the entry is in
[`../../TODO/toolchain.md`](../../TODO/toolchain.md) and is deliberately short. What is
here is the long-form record each one accumulated: the measurements, the
corrections, the routes costed and the routes killed.

⛔ **Read the TODO entry first.** Come here when you need to know WHY it says
what it says, or before re-running something to check whether it was already
run. ⭐ A number quoted in the TODO entry was derived here.

⚠ The headings below deliberately do NOT use the `## T-NNN — ` form, because
that form is what `sh TODO/check.sh` treats as *the* entry, and there must be
exactly one of those per id.

---

## T-012 · retired detail — `pgb build <url-or-package>`

**Source** operator · **Category** toolchain · **Priority** P1 · **Effort** XL · **Status** open

**Problem.** A developer still has to know how to build the project.
`pgb build -- make` is a toolchain injector, not a toolchain.

**Premise.** ⭐ The interface is achievable over a large package database:
`nix bundle nixpkgs#chromium` does it today
(`docs/research/nix-appimage.md`). ⛔ Its store model must not be copied.

**Approach.** ⚠ **XL — this is two or more entries pretending to be one.**
Split before starting: spec resolution (URL or package name → source tree),
build-system detection, and the dependency planner are separate.

### ⭐ 2026-09-03c — ALL THREE OF THOSE PIECES EXIST, AND R3 RAN THEM END TO END

⚠ **The entry says "split before starting" as though none of the three were
built. They are, and the postgres run of T-063 arm S exercised all three in one
command.** `NIX_MAX_ROUNDS=24 pgb nix build --plan pg.plan`:

| the split this entry names | what ran |
|---|---|
| **spec resolution** | `pgb nix cache attr postgresql` → the attribute; `pgb nix plan postgresql --out pg.plan` → source, patches, buildInputs, configureFlags. ⭐ Resolved through hydra with **no nix used** |
| **build-system detection** | ⭐ **it prints what it decided, per dependency.** Over one postgres build: `pkg-config` ×11, `cmake (cmakeFlags)` ×9, `autoreconf (configureFlags)` ×8, `meson ninja pkg-config (mesonFlags)` ×2, `cmake ninja` ×2, `autoreconf pkg-config` ×2 |
| **the dependency planner** | **41** `dep ok` / `dep have` lines — it walked postgres's inputs, built each, and installed them into one static prefix |

⭐ **So `pgb nix build <package>` already IS `pgb build <package>` for the
nixpkgs half**, and it produced a statically linked PostgreSQL 18.6 with ICU
that runs on Alpine. T-063 carries that result.

⛔ **What is actually left, then, is narrower than "XL":**

1. ⛔ **The URL route.** Nothing resolves a git URL to a source tree; every
   route above goes through nixpkgs. This is the half with no code at all.
2. ⚠ **The report.** `design/toolchain.md` requires the tool to *"name every
   component it could not link statically, and why"*. The adaptation loop
   already knows — it prints `round N: FAILED -> drop:--with-llvm` — but
   nothing collects those into an answer for the developer.
3. ⚠ **Joining static-first to bundle-last.** `pgb nix build` and
   `pgb bundle appimage` are two commands with no path between them, and the
   brief's preference order is exactly that path.
4. ⛔ **AND THE "NO NIX" PROPERTY DOES NOT HOLD FOR EVERY SPEC**, which T-060
   owns but T-012 inherits: `pgb nix plan kdePackages.kdenlive` reports
   *"no nix-free route resolved … falling back to evaluation"*. The nix-free
   index and hydra routes reach a top-level attribute; a dotted one they do
   not.

**Prove.** `pgb build <a git URL>` produces a binary that `pgb verify`
passes on 11 of 11, with no other input from the operator.


## T-013 · retired detail — Measure developer friction

**Source** operator · **Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

**Problem.** `docs/comparison.md` states the friction axis from one session's
record. Nothing re-runs it, so it goes stale silently.

**Approach.** `experiments/63-developer-friction.sh`: count external artefacts
fetched, files authored, environment variables required, and whether each route
completes unattended.

**Prove.** `sh experiments/63-developer-friction.sh` exits 0 and
`evidence/63-developer-friction/RESULT.txt` carries the counts.


## T-015 · retired detail — the OCI pull unpacks the filesystem and drops the image config

**Source** found while cross-checking the two `pgb verify` arms (T-014),
session of 2026-09-01.
**Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

**Problem.** An OCI image is a filesystem **and** a configuration.
`internal/ociimg/pull.go` unpacks the layers and ignores the config, so the
chroot bed and a `docker run` of the same digest are not the same environment.

**Premise.** ⭐ **Measured**, `../docs/history/corrections.md` C10. The
`archlinux` image config carries `Env: LANG=C.UTF-8`; `docker run` applies it,
the chroot bed does not, and the same binary therefore takes a different
`setlocale` path under the two beds:

```
docker run --rm --entrypoint /usr/bin/env archlinux@sha256:818793c8… | grep LANG
  -> LANG=C.UTF-8
the chroot bed sets no LANG at all
  -> 0
```

⚠ **This invalidates no committed number.** The difference showed up only in
the **host data** column, which `../docs/AGENTS.md` §3 reports and never
asserts, and the two arms agree on all eleven rows for both asserted columns.
What was wrong was the *claim* that the beds are the same environment.

**Approach.** `internal/ociimg` already parses the manifest; the config blob it
points at carries `Env`, `Cmd`, `Entrypoint` and `WorkingDir`. Record them
into `.oci-provenance`, and have `internal/rootfs` (`pgb rootfs run`) apply
`Env` unless the caller overrides it.

⛔ **Applying it must be a decision, not a default that appears silently.** A
bed that starts exporting `LANG` changes what every locale-sensitive result
describes, so the change lands with the affected experiments re-run, or it
lands behind a flag.

**Prove.** `pgb verify --engine chroot` and `--engine docker` on the same
binary produce the same HOST DATA column on all eleven rows, and
`experiments/30-gconv-and-locale.sh` is re-run and its result compared before
and after.


## T-051 · retired detail — Enough nix for a host with no root, no docker and no nix

**Source** operator, 2026-09-01c: *"find the least invasive way to 'install'
enough nix so pgb is usable even on the minimal of hosts like containers that
can't run docker images or install nix because no root"*.
**Category** toolchain · **Priority** P1 · **Effort** M · **Status** open

**Problem.** T-050 removes nix from the *planning* step for packages whose
`.drv` is cached. It does not remove it from the cases T-050 lists as out of
reach, and `pgb nix build` still wants a build environment.

**What the mining already says.** `references/nix-community__nix-user-chroot`
(commit `987302aef4e3aa267355cfad00027b730bcb389b`) runs nix as an ordinary
user in a user namespace — and its own README says Ubuntu 23.10+ gates
unprivileged user namespaces behind AppArmor and RHEL/CentOS 7 ship them off,
so it is not a universal answer either. `grigio/docker-nixuser` is the
container form of the same and needs a container.

**Approach, cheapest first.**
1. Push T-050 as far as it goes and measure exactly what is left.
2. For what is left, a static `nix` binary from the cache, run against a store
   under `$HOME` with `--store`. ⚠ Unverified; it is the obvious first probe.
   ⛔ **CORRECTED 2026-09-03c — THERE IS NO STATIC `nix` IN THE CACHE TO
   FETCH.** See below.
3. `nix-user-chroot` where namespaces are available, with the AppArmor case
   detected and named rather than hit.

### ⛔ 2026-09-03c — STEP 2's PREMISE IS WRONG, AND IT MAKES THIS ENTRY T-060's OTHER HALF

⚠ **Step 2 says "a static `nix` binary from the cache".** Fetched and looked
at — `pgb nix cache fetch /nix/store/7vb637v8…-nix-2.34.8`, **64 store paths,
163 MB, no nix and no root involved**:

    bin/nix   ELF 64-bit LSB pie executable, dynamically linked
              PT_INTERP 1        DT_NEEDED 11

    libnixutil.so.2.34.8   libnixstore.so.2.34.8   libnixexpr.so.2.34.8
    libnixfetchers.so.2.34.8   libnixflake.so.2.34.8   libnixmain.so.2.34.8
    libnixcmd.so.2.34.8    libgc.so.1   libstdc++.so.6   libgcc_s.so.1
    libc.so.6

⛔ **nixpkgs ships no static nix**, so step 2 cannot begin with a fetch. ⭐ And
**seven of those eleven are nix's own component libraries** — the very
derivations T-060 rung 1 has to build. **So step 2 as written requires T-060
rung 2**, and these two entries are the same work seen from two sides rather
than two independent probes. Whichever is taken first should be taken knowing
that.

⭐ **What DOES hold is the entry's own cheaper reading.** The fetched closure is
**self-contained**: all 11 `DT_NEEDED` resolve inside the 64 paths, and
`ld-linux-x86-64.so.2` is among them — which is the configuration
`experiments/80-` arm 5 already ran in a rootfs with no `/nix`. ⚠ **What is
still unmeasured is the `--store` half**: whether that nix, so carried, will
operate a store under `$HOME`. That needs a rootfs, because this build host has
nix installed and testing it here would prove nothing.

### ⭐ THE OPERATOR ANSWERED THIS ON 2026-09-03c: BUILD ONE, DO NOT LOOK FOR ONE

⛔ **Step 2 is not "fetch a static nix". It is "publish a static nix."** The
note is quoted because it also names the method:

> *"if we do end up needing to bundle nix, we will probably implement a mix of
> existing techniques by iterating/improving them and publishing a 'static'
> nix ourself"*

⭐ **Three references were named for it and all three are MINED**, 2026-09-03c,
`sh scripts/common/mine-repo.sh`, commits in each `PROVENANCE.md`:

| reference | commit | what it is for |
|---|---|---|
| [`references/DavHau__nix-portable`](../../references/DavHau__nix-portable/PROVENANCE.md) | `91122e3d94ba51d7d83fe990fa81d3de0968fb32` | nix as one file on a host with no root, no nix and no namespaces — the entry's own problem statement, solved by somebody |
| [`references/nixie-dev__nixie`](../../references/nixie-dev__nixie/PROVENANCE.md) | `d14c6c370489ec13b24d65df569e7769444ebebf` | a from-scratch reimplementation of the store side |
| [`references/containerbase__nix-prebuild`](../../references/containerbase__nix-prebuild/PROVENANCE.md) | `9302079d1cb625307f195273cee4632648ecbaec` | prebuilt nix artefacts, i.e. what publishing one looks like in practice |

⚠ **Reading them is owed, and it is owed under
[`../docs/methodology/references.md`](../../docs/methodology/references.md)** —
three passes each and the two write-up files, ⛔ **not delegated to a
sub-agent**, which that page rules out by name.

⛔ **Not `curl | sh` as root.** That is what this environment did once, on the
operator's explicit authorisation, and it is not the shape the entry is for.

## ⭐ STEP 1 IS DONE AND IT WENT FURTHER THAN THE ENTRY EXPECTED

`experiments/88-nonix-end-to-end.sh` arm 5, **8 assertions, 0 fail**. A rootfs
with a C toolchain and nothing else — and jq **planned, fetched, its
dependency planned and built, and itself built**, inside it:

    uid=12000            not root
    nix on PATH          no
    /nix                 absent
    a C compiler         yes
    build exit status    0
    the binary           4,129,368 B, no PT_INTERP, and `.a[1]` on
                         {"a":["x","é中"]} answers é中

⛔ **One defect had to be fixed for the dependency to build**, and it is the
one that would have stopped this entry at the first real package.
`nix_build_dep` opened with `nix_prefix() || warn "no nix, so a dependency
cannot be planned"` — so the nix-free route reached exactly as far as a package
with no dependencies. **The dependency's own `.drv` is already in the parent's
plan**, so planning it needs no evaluation: `nix_plan_from_drv` now serves it
and the evaluated route is the fallback. jq's oniguruma is planned from
5 derivations fetched over HTTPS and built into the shared prefix, with no nix
anywhere.

⚠ **What arm 5 does NOT show, stated rather than implied:**

- **The chroot is the harness, not the claim.** It is how a host with no `nix`
  and no `/nix` is produced on a machine that has both. Everything asserted is
  about what the process inside could see and do, and it drops to uid 12000
  before `pgb` is reached.
- ⛔ **The host still needs a C toolchain.** `pgb` is a build tool; the rootfs
  used is `pgb-env-debian12`, which has gcc and the static libiconv. A host
  with *no compiler* is not served by this and is what step 2 (a carried,
  relocatable toolchain) is for.
- **This environment's CA bundle lives under `/root`**, which uid 12000 cannot
  read; a readable copy travels in with the harness. The first run failed on
  exactly that and `nix-fetch` reported it as *"hydra has no finished build for
  jq"* — an error naming the failure it expected instead of the one it had.
  Fixed: curl's own message is printed.

**What is left on this entry** is steps 2 and 3 — a host with **no compiler**
— which is `T-060`, the static-glibc nix.


## T-060 · retired detail — ⭐ STATIC-GLIBC nix: the entry that makes "no root, no docker, no nix" true

**Source** ⭐ **operator, session of 2026-09-01e**, quoted because the framing
is the entry: *"nixpkgs' pkgsStatic is musl and this project is the glibc half.
Produce a static-glibc nix toolchain path end to end: pgb builds nix's own
dependency closure static, or carries enough of one to plan and fetch on a host
with nothing. `nixie-dev/nixie` is the shape the operator named. This is the
entry that makes 'no root, no docker, no nix' true rather than aspirational."*
**Category** toolchain · **Priority** P1 · **Effort** L · **Status** open

**Why it exists, given T-050 closed.** `experiments/88-` arm 5 plans, fetches
and builds a nixpkgs package on a host with **no nix and no root** — and it
still needs **a C toolchain on that host**. Two cases are left:

1. a host with no compiler at all;
2. the cases the index route cannot reach by construction — an override, an
   overlay, a `pkgsStatic.*` attribute, anything hydra never built — which need
   **evaluation**, and evaluation needs a nix binary.

⛔ **And nixpkgs has no static-glibc nix to fetch.** `pkgsStatic` is musl,
measured (`docs/research/nix.md` finding 1), and this project is the glibc half.

**The three rungs, and each is recorded whether it is reached or not** — the
shape `poc/90-qt` used:

| rung | what |
|---|---|
| 1 | nix's dependency closure built **static-glibc by pgb** — how far the dependency walk gets, dependency by dependency, with what stopped each one it could not |
| 2 | nix itself linked against them: a `nix-instantiate` with no `PT_INTERP` |
| 3 | that binary evaluating a nixpkgs attribute **inside a rootfs with no nix, no `/nix` and no root**, on the eleven |

⚠ **The honest risks, named now.** nixpkgs builds nix as **eight component
derivations** (`nix-util`, `nix-store`, `nix-expr`, `nix-fetchers`,
`nix-flake`, `nix-main`, `nix-cmd`, `nix-cli`) under meson, not one autotools
tree; its closure carries **boost, libgit2, libarchive, lowdown, editline,
sqlite, curl+openssl, libsodium, brotli, toml11 and the AWS CRT**, and the AWS
half is optional while boost is not. Any one of those can refuse `-static` the
way MLT's `add_library(mlt SHARED)` did.

### ⭐ 2026-09-03c — RUNG 1's PREMISE, MEASURED BEFORE THE ARM IS ATTEMPTED

⛔ **THE COMPONENTS ARE NOT INDEX ATTRIBUTES, and that is the first thing rung
1 walks into.** The nixpkgs index this tool resolves through holds **149,813**
attributes and none of them is a nix component:

    pgb nix cache attr nix-cli
      -> no attribute, pname or name "nix-cli" for x86_64-linux
    attributes matching ^nixVersions   -> 6, every one the AGGREGATOR
    attributes named nix-{cli,store,expr,util,main,cmd,fetchers,flake}-N
      -> 0   (the only `nix-store*` hits are unrelated packages)

⚠ **And `nix` itself is an aggregator, which the plan says plainly:**
`pgb nix plan nix` gives `nix 2.34.8` with **7 buildInputs — five test-runs,
the functional tests and `nix-perl` — and one nativeBuildInput, `lndir`.**
Planning the top-level attribute and building it would build no nix at all.

⭐ **BUT THEY ARE REACHABLE, BY A ROUTE THE TOOL ALREADY HAS.** `pgb nix plan
nix` reports `derivations 27 fetched and verified over HTTPS`, and
`/root/.local/state/pgb/drv/` then holds **24 nix component derivations** —
the eight the paragraph above names, plus their `-c` (C API) variants, the
`-tests-run` ones, `nix-manual`, `nix-nswrapper` and `nix-perl`.
`pgb nix drv <file>` reads any of them:

    nix-2.34.8.drv        inputDrvs 26, inputSrcs 2, env 36, outputs 4
    nix-store-2.34.8.drv  inputDrvs 15, inputSrcs 3, env 39, outputs 3

⛔ **AND THE TRANSITIVE WALK IS NOT MISSING EITHER — I SAID IT WAS, AND THEN
RAN IT.** The first version of this section concluded *"rung 1's missing piece
is a transitive `.drv` walk"*, from the observation that one `plan` leaves only
the top derivation's inputs on disk. ⚠ That inferred a gap in the tool from a
gap in what one command happened to cache. `pgb nix cache closure` takes a
`.drv` store path and does the whole walk:

    pgb nix cache closure /nix/store/nxwqpdnn…-nix-2.34.8.drv
      ⭐ 2,000 paths — 1,665 .drv and 335 sources

⭐ **AND EVERY DEPENDENCY THIS ENTRY NAMED AS A RISK IS IN IT**, so the list
above is confirmed rather than feared:

    boost 6   libgit2 1   libarchive 1   lowdown 2   editline 1
    sqlite 5  libsodium 1  brotli 1  toml11 1  aws-c-* 10  curl 5  openssl 5

⚠ **2,000 is the real count, not a cap** — checked three ways, because a round
number in a measurement deserves it: `Closure` has no limit (it is a BFS over
`References` until nothing new), the list is 2,000 **unique** paths, the count
is stable on a re-run, and a different nix derivation in the same closure gives
**1,248**, so the number tracks its subject.

⛔ **SO RUNG 1 IS NOT BLOCKED ON A TOOL. It is blocked on BUILDING 1,665
derivations static-glibc**, which is the work the entry describes and the risk
it names — *"any one of those can refuse `-static`"* — with boost, the AWS CRT
and libgit2 as the ones to expect trouble from. ⭐ The dependency-by-dependency
table rung 1 asks for can now be produced by walking that list; nothing has
walked it **building**, and that is the arm.

⭐ **A cheaper second reading of the same goal, which the operator allowed:**
*"or carries enough of one"*. `internal/nixx/fetch.go` already fetches
nix's own closure from `cache.nixos.org` with no nix and no root — 57 store
paths, 142 MB, signature and NarHash checked — and `experiments/80-` arm 5
already showed that a nixpkgs binary handed **the loader fetched beside it**
runs in a rootfs with no `/nix`. That is a relocatable nix without a single
line of C, and it is the fallback rung if the static build stops.


## ⭐ RUNG 1 MEASURED: `pgb nix deps`, and eleven defects it found

⛔ **`pgb nix deps` is new and it is what rung 1 needed.** nixpkgs' `nix` is
**seven component derivations over ONE source tree**, so no single plan
describes the build — but the union of those plans is exactly the library
closure a static nix needs. `pgb nix deps --plan X` builds a plan's
dependencies into the shared static prefix and stops.

**First pass: 24 built, 32 failed.** The failures were specific enough to fix
rather than to report, and fixing them is the "universal builder" work goal 1
names:

| what stopped | why | fixed by |
|---|---|---|
| boost | `bootstrap.sh` + `b2` is a build system pgb did not know; it said *"No targets specified and no makefile found"* | a b2 branch |
| zstd, libblake3, icu4c | the build file is in `build/cmake/`, `c/`, `icu4c/source/` | `pgb_build_root()` finds the shallowest one and says which it chose |
| lowdown | `oconfigure` takes `PREFIX=` and prints its key list when handed `--prefix` | an oconfigure branch |
| libseccomp | `configure: error: please install gperf` | `gperf` in the environment |
| **all six nix components** | `Meson version is 1.0.1 but project requires >= 1.1` | `meson==1.9.1` pip-installed into the environment, **with the pip set in the environment stamp** |
| doctest | its CMake config looks for MPI | test-only dependencies skipped by default |

### ⛔ And five defects in pgb itself, each of which produced a plausible result

1. **`pgb nix` ran every inner `pgb build` through `pick_engine`**, so
   `--engine chroot` was used for nothing and a freshly rebuilt chroot
   environment was ignored for a stale docker one. T-017 one layer in.
2. **The environment stamp's `packages=[\(.*\)]$` is greedy**, so adding a
   second bracketed field made it capture `a b] pip=[c` — and pgb refused a
   just-created environment for missing a package that was in it.
3. **The Dockerfile put the pip step above the trust anchor**, so pip died
   with *"certificate verify failed"* five retries deep: a message about pypi
   that is really about the order of two lines.
4. ⛔ **The `cpp` wrapper appended LINK flags and the runtime OBJECT to the
   preprocessor.** `cpp foo.c` has no `-c`/`-E`/`-S`, so the wrapper chose
   `mode=link`. On libX11 that produced `pgb-nssfix.o:4:457: warning: null
   character(s) ignored` and `configure: error: .../cpp defines unix with or
   without -undef`. `cpp` is a query tool and is passed through untouched now.
5. ⛔ **`PKG_CONFIG_PATH` had `lib/pkgconfig` and not `share/pkgconfig`**,
   where every architecture-independent package puts its `.pc`. libxcb's
   configure said *"Package 'xcb-proto', required by 'virtual:world', not
   found"* about a package built one directory over.

### ⛔ Three more, and the last one is the worst kind

6. **`pgb nix deps` keyed its per-dependency directories on the NAME alone**,
   so two runs with different `$NIX_PREFIX` shared
   `$PGB_STATE/nix-deps/openssl`; one run's tree was rebuilt under the other's
   feet and the loser failed compiling a demo. T-058 arriving in `pgb nix`.
7. **nixpkgs writes its output paths as placeholders** — `--libdir=$(out)/lib`
   and `--libdir=/02qcpld…52chars/lib` — and passed through, **libxml2
   installed itself into `/02qcpld1y6xhs5gz9bchpxaw0xdhmsp5dv88lh25r2ss44kh8dxz/lib`**.
   `.built/libxml2` was written, the prefix had nothing in it, and libxkbcommon
   then failed for want of a library that had just "built".
8. ⛔ **meson and openssl install into `lib64` while everything else looks in
   `lib`.** `dep ok libxkbcommon` was printed, `.built/libxkbcommon` was
   written, `xkbcommon.pc` was sitting in `lib64/pkgconfig`, and qtbase's
   configure said `XKB_FOUND = "FALSE"`. openssl's `libcrypto.a` was there too,
   in **both** prefixes. ⭐ **A dependency that builds, installs and is
   invisible is the worst of the three outcomes**, and pgb produced it twice.

### ⭐ And one adaptation rule worth having

meson prints the option that turns a feature off, in the error:
*"You can disable the Wayland xkbcli programs with `-Denable-wayland=false`."*
`nix_diagnose` takes that literally — only the disable form, only when the flag
is not already set. Measured on libxkbcommon: two rounds, two of meson's own
suggestions, built on the third.

⚠ **And the plan carries three flag lists while only `configureFlags` was ever
read**, so every cmake and meson package got none of the options nixpkgs chose
for it — including nix's own `-Dgc=enabled`, `-Dcpuid=enabled` and
`-Dseccomp-sandboxing=enabled`.

**Where rung 1 stands:** the closure is being rebuilt with all of the above in
place. ⛔ **Rungs 2 and 3 are not reached and this entry stays open.**

**Prove.** `evidence/89-static-nix/RESULT.txt`: the rung reached, with the
error and the file it came from for the rung that stopped — plus, for any rung
reached, `nix-instantiate` naming a derivation inside a rootfs that has no nix.

---


## T-066 · retired detail — ⛔ P0: the bundler is bloated and slow. Rebuild it against a CLI benchmark

**Source** ⭐ **operator, 2026-09-02b**: *"pgb bundle isn't good enough, it is
bloated, slow and a complete failure. Restudy what all nixappimage related
references and fix/patch/reimplement/iterate+improve everything needed to fix
our bundles. best place is to bundle a cli first, bundle something like bash or
maybe 7z which can be benchmarked better, and smaller and less time to compare
after each iteration/improvements."*
**Category** toolchain · **Priority** P0 · **Effort** XL · **Status** open

⛔ **WORK UNTIL IT IS MET OR THE PREMISE IS SIGNIFICANTLY ADVANCED.**

## ⭐ AMENDMENT — "bloated" is struck, "slow" is the whole entry, operator ruling, 2026-09-03c

⛔ **Half of this entry's premise was withdrawn by the operator** and the
withdrawal is quoted verbatim, because it re-scores every measurement below:

> *"us having a bigger size than anylinux-appimages and onelf is acceptable as
> long as ours performs better and packaging is just one command not a
> multiline shell script"*

⭐ **So the `2.86×` and `2.45×` rows are no longer failures.** They are
context. What is left of "bloated, slow and a complete failure" is **slow** —
and slow is now the entire bar, together with one-command packaging, which
this entry's subject already meets. `docs/design/toolchain.md`
"Static first, bundle last" carries the full amendment and the new axes table.

⚠ **Read the ROUTE work below with that in mind.** Route A, route B, `--cut`,
`--fixpoint` and the debloat rules were all costed in **bytes**. None of them
was measured on the **clock**. That does not invalidate any of the numbers; it
means none of them is scored against the bar until someone re-runs it with
startup and run time as the columns. ⛔ **That re-measurement is the first
thing the next session on this entry should do**, because it decides which of
the levers is worth building out and the byte figures cannot answer it.

**Problem, measured, and it is not close.**

| subject | ours | the field | | ⭐ under the ruling |
|---|---|---|---|---|
| `jq` (`experiments/86-`) | **11,471,610 B**, 7 store paths | **4,006,916 B**, 68 libraries | ⛔ **2.86×** | ⭐ acceptable |
| kdenlive (`experiments/90-`) | **471,033,944 B** | 191,900,604 B | ⛔ **2.45×** | ⭐ acceptable |
| kdenlive render | 4,947 ms | 2,033 ms | ⛔ **2.43×** | ⛔ **binding** |
| kdenlive cold start | 300 ms | 61 ms | ⛔ **4.92×** | ⛔ **binding, and the worst column** |

⭐ **AND `aggressive` MOVES IT — run 6, same day.** With sweep deletion on,
`DropUnreachable` removed **1,712 objects, 227.4 MiB**, and the artefact came
out at **426,528,098 B = 2.22×**, against `safe`'s 2.45×. ⛔ **It still
rendered**, 4,149 bytes of MP4, byte-for-byte what `safe` produced — which is
the measurement runs 1 and 3 failed at, and the first evidence that the three
sweep fixes hold on a plugin-heavy subject.

⚠ **Run 6's render and startup MILLISECONDS are contaminated and are not
quoted here.** The competitor's fixed artefact moved 2,033 → 13,680 ms in the
same window, which is the control saying the machine was loaded rather than
the bundle slower. `RULES.md` §"the shared resource is sometimes the clock".

### ⛔ AppDir bytes are NOT artefact bytes, and the ratio is about 6 to 1

    safe        AppDir 2.53 GiB -> 2.16 GiB      artefact 471,033,944 B
    aggressive  AppDir 2.53 GiB -> 2.07 GiB      artefact 426,528,098 B
                then the sweep: -227.4 MiB       delta      44,505,846 B

    AppDir removed, extra   92.2 MiB (debloat) + 227.4 MiB (sweep) = 319.6 MiB
    artefact removed                                              =  42.4 MiB
    ⭐ ratio                                                       = 7.5 : 1

⭐ **Corroborated by a second route:** `aggressive`'s extra rules are the
Vulkan drivers `intel` 47.8, `radeon` 20.0, `nouveau` 21.4 and `virtio` 1.9
MiB — **91.1 MiB**, against the 92.2 MiB the two totals imply. The logs round
to two decimals, so the AppDir delta is 319.6 MiB ± ~20; the ratio is 7 to 8,
not a constant.

⭐ **So a debloat rule is worth about an EIGHTH of its raw size on the thing a
user downloads**, because dwarfs at `zstd:level=19` was already compressing
what got deleted. ⚠ **This reframes the lever this entry opened on**: "489 MB
of kdenlive's `lib/` is unreachable" is worth **~65 MB** of artefact, not 489.

⛔ **And it makes the arithmetic decisive.** Closing 426,528,098 → 191,900,604
means removing **223.8 MiB of artefact**, which at 7.5:1 is **~1.65 GiB of
AppDir** — out of the ~1.85 GiB that remains after the sweep. **You would have
to prove 89% of what is left is dead.** Deletion is not the route; where the
closure comes from is.

⚠ **A correction, and lens 3 of the review is what caught it.** This paragraph
first said "about a sixth" and "~1.4 GiB", computed from an AppDir delta of
250 MiB that omitted `aggressive`'s extra Vulkan rules. The conclusion did not
change; the denominator did.

⛔ **AND THE NUMBERS ABOVE LIVE IN A LOG `.gitignore` DISCARDS.** `debloat`,
the sweep total and `icd json N rewritten` are printed to
`evidence/*/build/build-ours.log`, which `.gitignore:15` excludes — so the next
run overwrites the only copy. `evidence/90-kdenlive-vs-enhanced/run6-build-summary.txt`
is that copy, kept deliberately; ⚠ **run 5's equivalent is already gone**, which
is why its `2.53 GiB -> 2.16 GiB` is cited from a transcript rather than from
the tree.

⛔ **THE kdenlive ROWS ARE RUN 5's (2026-09-02d, `safe`) AND THEY SUPERSEDE
WHAT WAS HERE.** The previous figures — 477,191,058 B, 3,559 ms, 181 ms — came
from before the five-run sequence in which runs 1 through 4 were each invalid
for a different reason, and no run before 5 both rendered and completed.
⭐ Run 5 is the first that did: `ours rendered on every environment = 11 of 11`.
⚠ Timings move with the machine, so the render and start rows are a same-day
comparison against the competitor and not a claim about last week's numbers.

⭐ The one column ours wins is host-object cleanliness — **11/11 against 4/11**,
and on `rockylinux-8` the competitor loads **10** host shared objects to our
zero — and T-065 is about whether that is even the right assertion for a
bundle.

**Why a CLI is the subject, and the operator named it.** kdenlive is a
20-minute build and a 477 MB artefact: an iteration loop nobody can run. `bash`
or `7z` is minutes and megabytes, benchmarks cleanly (startup dominated, no
display, a real workload in `7z b`), and every improvement shows up
immediately. ⛔ **Do not iterate on kdenlive.** Land the CLI numbers first,
then re-measure the big subjects once.

**Premise — the levers already measured and not pulled.**

1. ⛔ **The reachability sweep exists and NOTHING consumes it.** Confirmed with
   codegraph: `Sweep` has exactly two callers, `bundleSweep` and its own
   selftest. `--debloat` is pattern rules only. 489 MB of kdenlive's `lib/` is
   unreachable. **This is the single largest unused lever.**
   ⭐ **PULLED 2026-09-02c** — `DropUnreachable` consumes it — and then
   **gated to `aggressive`** on the same day, after three classes of
   runtime-loaded library turned out to be invisible to it.
   ⛔ **AND IT WAS QUADRATIC, which is a second finding inside the first.** The
   soname scan did one `bytes.Contains` per needle per object, so it re-read
   every byte of the bundle once for every library in it. On kdenlive that is
   ~1,000 objects against ~1,000 names over a 2 GiB tree.

   ⭐ **Measured while it ran, rather than argued from the shape of the loop:**
   `/proc/<pid>/io` showed `rchar` advancing **14 MiB per 5 s — 2.8 MiB/s** —
   at 101% CPU, sleeping on nothing. The bundler reads each object once and
   then scans that buffer a thousand times, so the *bundle* advances at
   2.8 MiB/s while the *scanner* runs at gigabytes per second. On this AppDir
   that is **~12 minutes for the sweep alone**, against ~8 minutes for the
   whole of the rest of the build — fetch, debloat, wrapper lifting and all.

   ⭐ Replaced with a single-pass scan that is **exactly** equivalent by
   construction — the splitting alphabet is derived from the needles
   themselves, so a needle occurrence cannot straddle a split, and only runs
   containing `.so` are kept because `IsSharedObject` requires it — with the
   original kept as `sonamesMentionedNaive`, the control its selftest compares
   against on fixtures built for the four ways the two could differ.

   ⭐ **MEASURED ON THE REAL kdenlive AppDir — 1,633 library files, 1.49 GiB,
   2,586 roots, 33 plugin directories:**

   ```
   pgb bundle sweep AppDir --env AppDir/.env --list
     naive   838 s      exit 0, 47 lines
     fast      7.07 s   exit 0, 47 lines
     diff    IDENTICAL
   ```

   ⭐ **118×, and the outputs are byte-for-byte identical on a real bundle** —
   which is a far stronger control than the fixture selftest, because it is
   1,633 real libraries and 2,586 real roots rather than five files written to
   have a known answer.

   ⚠ **The ratio is approximate in one direction and it is worth saying which.**
   The naive arm ran with other work on the box (a `go build`, two `pgb
   selftest`s, the gates), so 838 s is if anything generous to the fast path;
   the fast arm was measured on an idle machine. An earlier reading during run
   6's own build put the sweep at ~12 minutes, also under load. ⛔ So "about
   100×" is the honest claim and 118× is the arithmetic.
2. `store/` is 405 MB of the kdenlive bundle and duplicates what is already in
   `lib/`.
3. `share/` is 368 MB, most of it one icon theme shipping every size.
4. Start and render both track artefact size — the dwarfs image is mounted at
   launch — so 1–3 move all three columns at once.

### ⭐ THE COMPETITOR'S WHOLE PIPELINE IS 89 LINES, AND IT ANSWERS THE ENTRY

Read 2026-09-02d out of `references/pkgforge-dev__kdenlive-AppImage-Enhanced/tree/`
— `get-dependencies.sh` (37 lines) and `make-appimage.sh` (52). ⛔ **The gap is
not a debloat rule we have not written. It is the direction the two pipelines
run in.**

| | ours | theirs |
|---|---|---|
| starting set | nixpkgs' **complete closure** — every path every derivation declared, **2.53 GiB** | `pacman -Syu` of a **hand-picked list of 12 packages** |
| then | **subtract**: delete what can be *proved* unreachable | **add**: `quick-sharun` walks the DT_NEEDED closure of ~20 named paths |
| the heavy packages | whatever nixpkgs built | ⭐ `get-debloated-pkgs --add-common` — size-optimised **rebuilds**, which is `pkgforge-dev/archlinux-pkgs-debloated`, the corpus this entry already names |
| what must not come | nothing; the sweep has to *discover* it | ⭐ `pacman -Rsndd --noconfirm qt6-webengine` — an explicit **removal**, one line |
| subsystems | inferred — our soname-string rule scans every ELF for `libSDL3.so.0` at 2.8 MiB/s | ⭐ **declared**: `DEPLOY_OPENGL=1 DEPLOY_SDL=1 DEPLOY_PIPEWIRE=1` |

⛔ **SUBTRACTIVE CANNOT WIN AGAINST ADDITIVE HERE, AND IT IS STRUCTURAL RATHER
THAN A MATTER OF EFFORT.** The sweep is deliberately conservative — sweep.go's
own rule is *"anything a rule cannot classify counts as REACHABLE"* — so
everything it cannot **prove** unnecessary stays. An allowlist keeps only what
was **named**. Starting from a superset and deleting provable dead weight can
approach the allowlist's result only if the proof is complete, and it is not:
three classes of runtime-loaded library were invisible to it in a single
afternoon.

⭐ **And the arithmetic already said so.** `aggressive` deletes 250 MiB of
AppDir for 42.4 MiB of artefact — about **6 to 1**, because dwarfs was already
compressing what got deleted. Closing 426 MB → 192 MB by deletion alone would
need roughly **1.65 GiB** more of provably-dead AppDir, out of the ~1.85 GiB
that remains after the sweep — **89% of what is left**. There is not that much
left to prove.

⭐ **And the corpus names exactly the packages that dominate OUR bundle.**
`Anylinux-AppImages/HOW-TO-MAKE-THESE.md`: *"Installs a debloated MESA, Vulkan,
Qt, GTK, libicudata, and more"*, with `--prefer-nano`, `ffmpeg-mini` and
`intel-media-driver-mini`. ⚠ Compare what `experiments/85-` measured on our
side — the GL stack alone is **95 MiB of a 163 MB bundle** — and what run 6's
debloat log lists: mesa's Vulkan drivers, `libteflon`, the locale catalogues.
**The overlap is not partial; it is the same list.**

⚠ **What this does NOT say.** It is a reading of somebody else's build script,
not a measurement of ours-rebuilt-additively; the measurement backing it is the
6:1 ratio and the 2.22× gap, both from run 6. ⭐ **The route it indicates** —
`pgb bundle appimage` taking an allowlist of paths rather than a closure, and
sourcing heavy packages from a debloated corpus — is the next thing to build
and it is not yet built.

### ⭐ MINED 2026-09-02e — and an allowlist is NOT enough, which the corpus says in its own recipes

`references/pkgforge-dev__archlinux-pkgs-debloated`, commit
`f29738934d003731a37bb1ca191030927fd3fa1b`, route proxy, 24 recipes.
⛔ **Reading them changes this entry's named lever, because a "debloated
package" is not a package with files deleted. It is a package REBUILT with
different build options**, and what that removes is a `DT_NEEDED` **edge**:

| recipe | what it actually does | the edge it removes |
|---|---|---|
| `qt6-base-mini.sh` | inserts `-DFEATURE_icu=OFF`, `-DCMAKE_BUILD_TYPE=MinSizeRel`, `-O2`→`-Os` | ⭐ `libQt6Core.so` → `libicuuc/libicudata`, ~30 MiB |
| `mesa-mini.sh` | deletes `llvm-libs` from `depends`, `-D amd-use-llvm=false -D draw-use-llvm=false` | ⭐ `libgallium.so` → `libLLVM.so`, **150+ MiB** |
| `ffmpeg-mini.sh` | deletes `--enable-libsvtav1`, `--enable-vapoursynth`→`--enable-small` | `libavcodec` → `libx265.so`, ~20 MiB |
| `icu-mini.sh` | rebuilds `libicudata.so` | 30 MiB → **<3 MiB** |
| `opus-mini.sh` | Arch's own options | 5 MiB → **<500 KiB** |
| `libxml2-mini`, `gdk-pixbuf2-mini`, `librsvg-mini`, `glycin-mini` | drop icu / drop glycin | ~20 MiB of glycin |

And the set kdenlive gets is not a guess — `get-debloated-pkgs.sh:210`,
reached by `get-dependencies.sh`'s one call to `--add-common`:

    icu-mini  opus-mini  libxml2-mini  qt6-base-mini  gtk3-mini  gtk4-mini
    glycin-mini   + mesa-mini (ADD_MESA)  + vulkan-intel-mini  (+ intel-media-driver-mini)

⛔ **SO AN ALLOWLIST CANNOT REACH THIS, AND THAT IS THE CORRECTION.** The lever
this entry named is *"take an allowlist of paths rather than a closure"*. That
is necessary and it is still worth building — but it is **bounded**, and the
bound is not effort. An allowlist chooses which *paths* to carry; it cannot
remove a dependency a library **declares**. A perfect allowlist naming only
kdenlive's true dependencies still carries `libicudata.so`, because the
`libQt6Core.so` in the closure has a `DT_NEEDED` on it and deleting it breaks
the binary — which is exactly the assertion `b.integrity()` already makes
("every DT_NEEDED in the bundle resolves inside it"). Only a **rebuild**
removes the edge.

### ⛔ AND WHY THEIR SWAP IS CHEAP AND OURS IS NOT — the packaging models differ

⭐ **This is the structural reason, and it is about content addressing rather
than about bundlers.** Arch swaps `qt6-base` for `qt6-base-mini` and **nothing
downstream rebuilds**: the soname `libQt6Core.so.6` is unchanged, so kdenlive's
existing binary keeps resolving against it. One rebuild of one package, done
once in their CI and published.

Ours cannot do that. `internal/bundle/appimage.go:154` fetches the closure by
**exact store path** — `b.Nix.Fetch("/nix/store/"+b.Base, WithClosure: true)`
— and a nixpkgs store path is the hash of the derivation's inputs. Changing
qtbase's build options changes qtbase's hash, which changes the hash of
everything that depends on it. ⛔ **So a `qt6-base-mini` equivalent invalidates
the binary cache for kdenlive's entire KDE/Qt subtree, and every one of those
paths would have to be built from source** — the thing `pgb nix` exists to
avoid, and the reason `experiments/88-` is a *fetch* story.

⚠ **`pgb nix build` CAN already express the option change** — `nixArgs.Configure`
→ `Builder.ConfigureExtra` → `internal/nixx/tree.go:99` — so the mechanism is
present. What is absent is a costing of the rebuild it forces.

⭐ **Three routes, and the entry now carries the argument rather than one name:**

| | route | what it costs, and what would settle it |
|---|---|---|
| **A** | the **allowlist**, as already named | still worth building; bounded by the edges above. ⛔ Measure the bound: sum the sizes of the closure paths that are reachable ONLY through an edge a `-mini` rebuild would delete. That number is the ceiling an allowlist cannot pass, and it is cheap — it needs the AppDir and `pgb bundle sweep`, no rebuild |
| **B** | build our own `-mini` derivations through `pgb nix build --configure` | ⚠ forces a from-source build of every dependent path. Cost unknown; the first measurement is how many store paths kdenlive's closure has downstream of `qtbase` and `mesa` |
| **C** | splice a smaller library into the fetched closure post hoc | ⛔ breaks the closure's own guarantee — the NarHash no longer matches what was signed — and `b.integrity()` would have to be re-satisfied by hand. Cheapest to try, weakest to defend |

⛔ **Nothing here is measured yet on our own bundle**, and it must not be
written up as though it were. ⚠ **The AppDir this would be measured against —
`/var/tmp/pgb-appimage-kden`, 7 GB — did not survive the container**, so route
A's ceiling is a rebuild away and is the first thing to run when a bundle
exists again. ⭐ The claim carried here is only what the corpus's own recipes
say, at file and line, plus the store-path property read out of
`appimage.go:154`.

**Approach.** Restudy the family first, then iterate against the CLI:
`pkgforge__nix-appimage`, `ralismark__nix-appimage`, `of-the-stars__nix-appimage`,
`logos-co__nix-bundle-appimage`, `VHSgunzo__sharun`, `VHSgunzo__runimage`,
`pkgforge-dev__Anylinux-sharun`, `nix-community__patsh`, `leleliu008__elftool`.
⭐ **Iterate, patch and reimplement — the brief says reuse and improve before
reinventing.** Each change lands with the CLI numbers before and after.

**Prove.** ⛔ Not "it is smaller". A table with a row per iteration for the CLI
subject — bytes, cold start, warm start, and the workload's own time — showing
what each change bought; then the same three columns re-measured for `jq` and
for kdenlive. ⭐ **The bar is the field**: `experiments/86-`'s hand-built
Anylinux arm for the CLI, and `kdenlive-AppImage-Enhanced` for the big one.
`docs/AGENTS.md` §14 forbids "better" without the measurement.

### ⛔ 2026-09-03: ROUTE A HAS AN INSTRUMENT NOW, AND MEASURING IT FOUND A DEFECT IN THE SWEEP

⭐ **`pgb bundle sweep --cut FROM=>TO` is route A's measuring device.** It
treats one `DT_NEEDED` edge as absent and reports what becomes unreachable
without it; the delta against the uncut sweep is the size of the subtree
reachable ONLY through that edge — the bytes an allowlist cannot reach, because
only a rebuild removes a declared dependency. ⚠ **Nothing is modified**: the cut
is applied to the graph walk, so one AppDir answers for every edge in turn.

⛔ **THE ENTRY SAID THIS MEASUREMENT WAS CHEAP — "it needs the AppDir and `pgb
bundle sweep`, no rebuild". THAT WAS WRONG, and finding out why is the result.**
Three things had to be fixed before a cut could move a single byte, and each was
found by the number refusing to move:

**1. ⛔ A versioned library was a ROOT OF ITSELF, and that disabled the largest
lever in this entry.** Both soname scans excluded the scanned object's own base
name — `n != self` with `self = filepath.Base(o)` — so a library mentioning its
own name did not become a root. ⚠ **For a versioned library that check can never
fire.** `libunistring.so.5.2.1` carries `DT_SONAME libunistring.so.5`; the index
holds `libunistring.so.5`, because the symlink beside it is an index key; and
the two strings are not equal. So the needle matched the SONAME in the object's
own `.dynstr`, the self-check compared it against the FILENAME, and the library
became its own root. ⭐ **Nearly every ordinary shared library has a SONAME that
differs from its filename**, so `DropUnreachable` could not drop a versioned
library whatever the graph said.

⭐ **Measured on the `jq` AppDir**: roots **40 → 28**, unreachable **252 files /
7,698,280 B → 262 files / 8,012,232 B**. The self-check is by REAL FILE now, so
the group for `libunistring.so.5.2.1` is `{libunistring.so, libunistring.so.5,
libunistring.so.5.2.1}` — filename, SONAME symlink and development symlink at
once, with no rule about version suffixes.

⚠ **The selftest had a self-naming case and it passed throughout**, because its
fixture used `libself.so.3`, a file whose name equals its SONAME. A versioned
fixture with a SONAME symlink is added; it FAILS against the old one-name check
and passes against the new one.

**2. The cut has to reach the soname STRING scan, not just the DT_NEEDED walk.**
A `-mini` rebuild does not merely stop linking: it removes the `DT_NEEDED`
entry, and that entry IS a string in the object's `.dynstr`. A cut that
suppressed the edge and left the string made every edge measure zero.

**3. The cut is by target FILE, not by name.** Cutting
`libidn2=>libunistring.so.5` suppressed that needle and left `libunistring.so`
— a substring of the same string — to make the library a root anyway. The roots
count fell by one and not a byte moved.

⭐ **VALIDATED AGAINST A KNOWN ANSWER.** With all three fixed, cutting `jq`'s
only interesting edge gives a delta of **2,076,792 bytes, which is exactly the
size of `libunistring.so.5.2.1`** — the one library that edge uniquely reaches.

    unreachable  baseline  262 files,  8,012,232 B
    unreachable  --cut     263 files, 10,089,024 B     delta 2,076,792 B

⚠ **And the instrument says when it measured nothing.** A `--cut` naming an edge
the bundle does not have prints `cut edges hit 0  ⛔ NOTHING MATCHED`, because a
zero delta otherwise reads exactly like "that edge costs nothing to carry" and
means the opposite.

### ⛔ AND THE `jq` RATIO IN THIS ENTRY IS STALE — 1.22× DESCRIBES A BUILD THAT NO LONGER EXISTS

⚠ **`evidence/78-bundle-cli-bench/RESULT.txt` predated the commit that gated
`DropUnreachable` to `aggressive`** (`5fbf7ad0`, "The sweep ran before .env
existed and deleted kdenlive's MLT modules"), checked with
`git merge-base --is-ancestor`. In that file `safe` and `aggressive` are the
SAME number, 4,890,913 — the sweep was feeding both. After the gating, `safe`
correctly stops dropping unreachable objects and grows back.

⭐ **Re-run 2026-09-03, same subject, against `experiments/86-`'s hand-built
Anylinux arm at 4,006,916 B:**

| arm | bytes | vs the field |
|---|---|---|
| `none` | 12,261,839 | 3.06× |
| `safe` | 7,332,001 | 1.83× |
| ⭐ `aggressive` | **6,326,245** | ⭐ **1.58×** |
| ⚠ the 1.22× this entry quoted | 4,890,913 | ⚠ a pre-gating build where `safe` also swept |

⛔ **So the honest current figure is 1.58× at `aggressive`, not 1.22×**, and the
improvement this entry claimed was partly the gating being absent rather than
the bundler being better. `experiments/78-` re-run: `pass=1 fail=0`, all three
arms build and answer the workload identically.

⚠ **What the self-exclusion fix bought on the artefact, measured both ways on
today's tree** — the same subject built with the pre-fix check planted:

    pre-fix   aggressive   252 objects, 7.3 MiB dropped   artefact 6,389,462 B
    post-fix  aggressive   262 objects, 7.6 MiB dropped   artefact 6,326,245 B
                                                          delta       63,217 B

⭐ **~62 KiB, about 1%**, and consistent with this entry's own ~7.5:1
AppDir-to-artefact ratio. ⛔ **It must not be written up as a size win.** `jq`'s
bundle has few versioned libraries with subtrees of their own; the fix is
structural — it restores a lever that was silently disabled — and what it is
worth on a Qt/mesa bundle, where versioned libraries dominate, is **unmeasured**.

### ⭐ AND THE CEILING IS MEASURED — 218.5 MiB of 938.8, on a mesa bundle

`evidence/t066-allowlist-ceiling/RESULT.txt`. ⚠ **Not on kdenlive**: the
subject is `mesa-demos` at `--debloat none`, whose 1.2 GB AppDir carries the
single biggest `-mini` edge (`mesa-mini.sh` deletes `llvm-libs` from `depends`)
and the icu one, and builds in minutes rather than twenty. **806 library files,
938.8 MiB.**

| arm | unreachable | delta |
|---|---|---|
| baseline | 61,882,072 B (59.0 MiB) | — |
| `--cut '*=>libLLVM.so.21.1'` | 250,081,656 B (238.5 MiB) | ⭐ **188,199,584 B** |
| ⭐ **+ `--cut` the three icu names** | **291,032,720 B (277.6 MiB)** | **40,951,064 B** |

⭐ **THE CEILING FOR THESE TWO RECIPES IS 229,150,648 B = 218.5 MiB — 23.3% of
the library tree**, and at this entry's own ~7.5:1 AppDir-to-artefact ratio
that is **about 29 MiB of artefact** an allowlist can never reach.
⚠ 8 edges were hit for `libLLVM` and 13 with icu; the counts are printed
because a `--cut` that matches nothing gives a zero delta that reads exactly
like "this edge is free to carry".

⛔ **AND IT CORROBORATES "SUBTRACTIVE CANNOT WIN" WITH A FRESH NUMBER.** On this
bundle the sweep can prove only **6.3%** dead. The two rebuild edges above are
worth **23.3%** — nearly four times what deletion can reach here, from two
recipes out of the corpus's 24. ⭐ That is the entry's structural argument
measured on our own bundle rather than read out of somebody's build script,
which is what the previous revision said was missing.

⚠ **What it does not say.** It is `mesa-demos`, not kdenlive — a different
closure with a different ceiling, and the Qt and `ffmpeg→x265` edges are not in
it and are not counted. The 7.5:1 ratio is carried from `experiments/90-` run 6
and was **not** re-measured here, so the ~29 MiB is an estimate and the
218.5 MiB is the measurement. ⚠ The kdenlive AppDir the entry wanted
(`/var/tmp/pgb-appimage-kden`, 7 GB) still does not exist.

⭐ **And one more thing the sweep does that bounds any ceiling measured with
it**: the soname string scan takes mentions from EVERY object in the bundle,
including objects that are themselves unreachable. So an unreachable
`libicui18n` mentioning `libicuuc` keeps `libicuuc` reachable. Making that a
fixpoint — only mentions from reachable objects count — is a real further lever
and a real safety question, and it is named here rather than taken.

### ⭐ 2026-09-03c: LEVER B3 IS TAKEN — AS A MEASURING DEVICE, AND IT IS WORTH 0.93 MiB ON `jq`

⭐ **`pgb bundle sweep --fixpoint`.** Soname mentions are counted only from
objects that are themselves reachable, iterated until the scan set stops
shrinking. ⛔ **Off by default and NOT wired into debloat** — `Fixpoint` appears
in `cmd/pgb/bundle.go` and nowhere else, and `internal/bundle/appimage.go` never
sets it. This measures the lever; it does not act on it, exactly as `--cut`
measured route A's ceiling without modifying a bundle.

⭐ **WHY IT TERMINATES, rather than "it seems to settle".** The scan set starts
as every object and each round keeps only what the previous round found
reachable, so it is monotonically decreasing over a finite set; fewer objects
scanned gives fewer or equal mention-roots, hence a smaller or equal closure.
⚠ The **seed** — the program roots and everything in a plugin directory — is
outside the loop, because those are roots by structure rather than by mention
and the fixpoint must never remove one.

**Measured on the `jq` AppDir at `--debloat none`, 284 library files, 16.7 MiB:**

| arm | roots | unreachable | |
|---|---|---|---|
| baseline | 28 | 262 files, 8,012,232 B (45.8%) | |
| ⭐ `--fixpoint` | 20 | **269 files, 8,990,808 B (51.4%)** | ⭐ **+7 files, +978,576 B** |

    fixpoint rounds  3
    objects scanned  14 of 283 for soname mentions

⭐ **THE BASELINE ROW IS BYTE-IDENTICAL TO THE ONE THIS ENTRY ALREADY
RECORDED** — 262 files, 8,012,232 B — which is the regression control saying
the loop this change wraps around the walk is a no-op when the flag is off.

⭐ **AND THE SEVEN FILES ARE NAMED, because that is what makes the safety
question answerable instead of abstract:**

    lib/libCNS.so  lib/libGB.so  lib/libISOIR165.so
    lib/libJIS.so  lib/libJISX0213.so  lib/libKSC.so      lib/libresolv.so.2

Six are glibc's **CJK gconv helper libraries**, and the objects mentioning them
are `EUC-TW.so`, `ISO-2022-CN.so` and `ISO-2022-CN-EXT.so` — gconv modules the
**baseline already classifies unreachable**. So the pattern this lever was
named for is exactly what it found: an unreachable object holding up a library
only it names. ⚠ Nothing goes the other way — the set of files the baseline
drops and the fixpoint keeps is **empty**.

⛔ **THE SAFETY QUESTION IS STILL OPEN AND `experiments/89-` IS STILL THE
CONTROL.** It has **not** been run against the fixpoint, and until it is,
nothing may act on this. ⚠ And the seven names sharpen the question rather
than settling it: a bundle whose application converts a CJK encoding at run
time reaches those helpers through `iconv_open`, which leaves no `DT_NEEDED`
and no mention — the same shape as the `libSDL3` miss this sweep already paid
for once. ⭐ The gconv tree is how a *bundle* solves the gconv problem
(`docs/AGENTS.md` §14), so this lever aims straight at it.

#### ⭐ BUT THE LEVER INHERITS THE BASELINE'S RISK RATHER THAN ADDING ONE, and that is measured

⭐ **The invariant: the fixpoint can only drop a library whose every supporter
— `DT_NEEDED` or mention — is itself dropped.** It follows from the
construction (a library survives if any *reachable* object needs or names it),
and it was checked on the real bundle rather than argued: for each of the seven
files, against every object still reachable under the fixpoint, resolved to
real files — ⭐ **0 reachable objects mention any of the seven.**

⭐ **`libresolv.so.2` is the case that makes the point.** Its only mentions in
the bundle come from `libnss_dns.so.2` and `libnss_hesiod.so.2`, and those are
`DT_NEEDED` edges — and ⛔ **the BASELINE already classifies both unreachable**.
So the baseline was deleting the NSS modules and keeping the library they exist
to use. ⚠ **That is an incoherence in the sweep, not a safety margin**, and the
fixpoint removes it. The same holds for the six gconv helpers: their only
mentions are `EUC-TW.so`, `ISO-2022-CN.so` and `ISO-2022-CN-EXT.so`, which the
baseline drops.

⛔ **So what `experiments/89-` has to clear is smaller than it looked.** The
question is not *"is it safe to drop `libresolv`"* — it is *"was it safe to drop
`libnss_dns`"*, which the baseline already decided and 89- already covers. The
fixpoint takes no independent judgement. ⚠ The residual risk is unchanged and
is the baseline's: a library reached only through `iconv_open` or a `dlopen` by
a name nothing spells out is invisible to both.

⚠ **And the check itself found the defect class this entry is about, in a
throwaway script.** The first version subtracted the unreachable list from the
directory listing **by name**, so `lib/libresolv.so` — a symlink to
`libresolv.so.2` — counted as a surviving object that mentions a dropped one,
and reported two violations. Both vanished when the sets were resolved to real
files. ⭐ Name-based set arithmetic over a library directory is wrong in exactly
the way `selfKeys` is written to prevent, and it is still easy to write.

**Carried offline** in `bundle-sweep`: `libghost.so.1` is unreachable and its
`.rodata` names `libhaunted.so.1`, which nothing else references. Without the
fixpoint `libhaunted` is a root; with it, it is unreachable. ⛔ Plus the three
negatives that stop the lever being "delete more": a library the program
NEEDS, a plugin's own dependency, and the env-named library are all still
reachable under it. 506 → **516 cases**.

### ⛔ 2026-09-03c: ROUTE A AS AN ALLOWLIST OF **PATHS** IS THE WRONG GRANULARITY, and two cheap measurements say so

⭐ **The lever this entry names for route A is *"take an allowlist of paths
rather than a closure"*. Both halves of that were measured, and neither
supports it.**

**1. On `jq`, not one store path is entirely unreachable.** Per store path, how
many of its shared objects survive the sweep:

| store path | kept | dropped | bytes dropped |
|---|---|---|---|
| `oniguruma-6.9.10-lib` | 1 | 0 | 0 |
| `jq-1.8.2` | 1 | 0 | 0 |
| ⛔ **`glibc-2.42-84`** | 8 | **269** | **8,990,808** |
| `libidn2-2.3.8` | 1 | 0 | 0 |
| `xgcc-15.3.0-libgcc` | 2 | 0 | 0 |
| `jq-1.8.2-bin` | 0 | 0 | 0 |
| `libunistring-1.4.2` | 1 | 0 | 0 |

⛔ **Six of seven are 100% kept, and ONE path carries every deletable byte.**
An allowlist choosing which store paths to fetch would have saved **nothing**
here: all the weight is *inside* glibc — the gconv modules, the NSS modules and
their helpers — which is a **file-level** problem the sweep already solves and a
path-level allowlist cannot touch at all.

**2. On kdenlive, the `-dev` outputs bound what path-level exclusion can
reach.** From the same closure `experiments/95-` walked, 676 paths /
2,941,485,288 B:

    -dev outputs        80 paths (11.8%)     70,581,624 B (2.4% of bytes)
    -debug/-doc/-man     0 paths                       0

⚠ **11.8% of the paths and 2.4% of the bytes.** So even excluding every
development output — the most obviously droppable class, and the only one
present — moves a fortieth of the closure. ⛔ *And these are in the RUNTIME
closure*, which means something references them at run time; that is the
wrapper-script problem T-053 and `patsh` are aimed at, not dead weight to
delete unexamined.

⭐ **What this reorders.** B2 was *"then the allowlist, now bounded and worth
building anyway"*. It is bounded much harder than the ceiling measurement
implied: the ceiling said an allowlist tops out near a quarter of the tree,
and these two say the *path*-level form of it reaches ~2% on one subject and
**0%** on the other. ⛔ The lever that works at this granularity is the
file-level sweep, which exists. ⚠ Neither number is kdenlive's AppDir, which
still does not exist.

#### ⭐ THE TWO LEVERS ARE ADDITIVE, and the published delta REPRODUCES

Same fresh `jq` AppDir, four sweeps:

| arm | unreachable | delta on baseline |
|---|---|---|
| baseline | 262 files, 8,012,232 B | — |
| `--cut '*=>libunistring.so.5'` | 263 files, 10,089,024 B | **2,076,792 B** |
| `--fixpoint` | 269 files, 8,990,808 B | **978,576 B** |
| ⭐ **both** | **270 files, 11,067,600 B** | ⭐ **3,055,368 B** |

⭐ **2,076,792 + 978,576 = 3,055,368 exactly.** The rebuild-edge lever and the
fixpoint lever touch **disjoint** sets on this bundle, so they add rather than
overlap — which is what makes it worth having both.

⭐ **AND THE 2,076,792 B IS A RE-DERIVATION, not a copy.** This AppDir was
built today from a different nixpkgs revision than the one that produced the
number above, and the cut delta came out byte-identical — still exactly the
size of `libunistring.so.5.2.1`.

⚠ **A trap worth writing down, because it cost a measurement here.** The `FROM`
half of a cut is matched against the depending file's **exact base name**.
`--cut 'libidn2=>libunistring.so.5'` — the spelling this entry uses in prose —
reports `cut edges hit 0  ⛔ NOTHING MATCHED`, because the file is
`libidn2.so.0.4.0`. ⭐ The instrument said so rather than returning a
misleading zero delta, which is the guard working. Use `*` as FROM, or the
full versioned file name.

### ⭐ 2026-09-03c: ROUTE B IS COSTED, AND IT IS FOUR TIMES CHEAPER THAN THIS ENTRY ASSUMED

`experiments/95-`, `evidence/95-route-b-cost/RESULT.kdenlive.txt`. ⛔ **This entry's own
words were *"forces a from-source build of every dependent path — the thing
`pgb nix` exists to avoid"*, and *"cost unknown"*. The cost is now known and
"every dependent path" is wrong.**

**Subject:** `kdePackages.kdenlive` → `/nix/store/rybc03ip…-kdenlive-26.08.0`,
resolved through hydra exactly as `appimage.go:resolveTarget` resolves it, so
this is the closure the bundler actually builds. **676 store paths,
2,941,485,288 B.** No rebuild, no AppDir — one hydra job and one closure walk.

| a `-mini` rebuild of | paths forced from source | bytes | of the closure |
|---|---|---|---|
| `qtbase` (3 outputs) | **78** | 739,766,064 | 11.5% |
| `mesa` (1 output) | **85** | 811,935,576 | 12.6% |
| ⭐ **`qtbase` + `mesa`** | ⭐ **85** | 811,935,576 | ⭐ **12.6%** |
| `icu` | 100 | 1,038,769,104 | 14.8% |
| `libxml2` | 136 | 1,504,152,912 | 20.1% |
| `opus` | 62 | 792,942,664 | 9.2% |
| `gtk4` | 11 | 239,430,608 | 1.6% |
| `gtk3`, `glycin` | ⚠ **NOT IN THIS CLOSURE** | — | — |
| ⭐ **the whole `--add-common` set** | ⭐ **161** | 1,675,619,424 | ⭐ **23.8%** |

⭐ **SO ROUTE B'S WORST CASE IS 161 OF 676 PATHS, AND 515 STILL COME FROM THE
BINARY CACHE.** Applying *every* `-mini` recipe the corpus applies to kdenlive
leaves **76.2% of the closure** fetched rather than built. The fear this entry
recorded — that changing qtbase's options invalidates "kdenlive's entire KDE/Qt
subtree" — is measured at **11.5%**.

⭐ **AND `qtbase` IS FREE ONCE `mesa` IS PAID FOR.** Every path downstream of
`qtbase` is also downstream of `mesa-libgbm` — `downstream of qtbase but NOT of
mesa: 0` — so the two biggest recipes together cost exactly what `mesa` costs
alone. ⚠ That is a property of *this* closure, not a law.

⛔ **THE CONTROLS, because reverse reachability computed the wrong way round
produces a perfectly plausible table.** They swap on an inverted graph:

    downstream of kdenlive itself        1 of 676   (nothing is above the top)
    downstream of glibc                635 of 676   (libc is under everything)

⛔ **AND THE NUMBERS ARE A FLOOR, WHICH THE EXPERIMENT SAYS OUT LOUD.** The
graph is the **runtime reference** graph, read from each narinfo's
`References:`. nixpkgs propagates a rebuild along **build inputs**, of which
runtime references are a subset. So the true rebuild set within this closure is
**at least** these counts. ⭐ It is a floor on the right population, though: the
bundle carries exactly these 676 paths.

⚠ **AND `mesa` HERE IS ONLY `mesa-libgbm-26.1.3`.** kdenlive's nixpkgs closure
carries no mesa driver stack at all — GL arrives through `libglvnd`, and the
`libLLVM.so.21.1` that dominated route A's ceiling is in `mesa-demos`' closure,
not this one. ⛔ **So the 218.5 MiB ceiling and these 161 paths are measured on
two different closures and must not be subtracted from one another.**

**What this changes, and what it does not.** Route B is affordable in *paths*.
⚠ It is **not costed in WALL CLOCK**, which is the number that decides whether
`pgb bundle appimage kdenlive` stays a one-command operation: one of those 161
is qtbase itself, and Qt does not build in a minute. That measurement needs a
rebuild and is the next rung, not this one.

#### ⭐ B1b's FIRST BOUND, AND IT COST NOTHING: THE POC SUITE ALREADY BUILDS QT

⚠ **Qtbase is the biggest single path of the 161, and this tree builds it twice
every acceptance run.** Two clean-rebuild runs on 2026-09-03c, four cores,
`PGB_ENGINE=chroot` — **whole-POC** wall clock, which includes fetching,
configuring, the application build AND the eleven-environment matrix, so the
qtbase build alone is strictly less:

| | Qt configuration | run 1 | run 2 |
|---|---|---|---|
| `poc/90-qt` | `-static -no-opengl -no-icu -no-openssl -no-xcb -no-feature-network -no-feature-sql -qpa offscreen` | 868 s | **888 s** |
| ⭐ `poc/91-qt-xcb` | the above **plus** xcb, OpenSSL linked into QtNetwork, QtSql | 1,545 s | **1,681 s** |

⭐ **So a Qt 6.11.1 with xcb, TLS, network and SQL is under half an hour on
four cores, twice measured.** That is the right order of magnitude for the
biggest path route B forces, and it is a long way from prohibitive.

⛔ **AND IT CORRECTS THE PROSE IN `poc/90-qt/run.sh`**, which prints
*"building qtbase (this is the long pole: hours, not minutes)"*. ⚠ Its own POC
completes in under fifteen minutes including the matrix, and has twice. The
line is stale and is owed a fix.

⚠ **What these numbers are NOT.** Neither is nixpkgs' qtbase: kdenlive's plan
wants the full derivation plus `qtsvg`, `qtmultimedia`, `qtnetworkauth` and
`qtimageformats`, and poc/90's configuration in particular disables most of
Qt. They are a **floor on one path**, not route B's wall clock — which still
needs the 161 built. ⭐ But a floor measured twice on the real toolchain beats
the "Qt does not build in a minute" this entry had.

#### ⭐ AND THE SAME MEASUREMENT ON `mesa-demos` — THE SUBJECT ROUTE A's CEILING USED

⛔ **The section above ends by saying the ceiling and these counts are measured
on two different closures and must not be subtracted from one another. So the
experiment takes the subject as a parameter now** (`PGB_EXP95_ATTR`, one
`RESULT.<subject>.txt` each, the way `experiments/86-` does), and the same
question was put to `mesa-demos`. `evidence/95-route-b-cost/RESULT.mesa-demos.txt`:

    closure   111 paths, 386,416,368 B

| a `-mini` rebuild of | paths from source | bytes | of paths | of bytes |
|---|---|---|---|---|
| `mesa` | **2** | 63,939,384 | 1.8% | 16.5% |
| `icu` | 5 | 154,079,456 | 4.5% | 39.9% |
| `libxml2` | 6 | 115,215,856 | 5.4% | 29.8% |
| ⭐ **the whole `--add-common` set** | ⭐ **8** | **156,713,352** | **7.2%** | ⭐ **40.6%** |
| `qtbase`, `opus`, `gtk3`, `gtk4`, `glycin` | ⚠ **NOT IN THIS CLOSURE** | — | — | — |

⭐ **EIGHT PATHS OF 111, AND THEY CARRY 40.6% OF THE CLOSURE'S BYTES.** That
asymmetry is the finding: route B's cost is counted in paths and its reach is
counted in bytes, and on this subject the two are an order of magnitude apart.

⛔ **DO NOT SUBTRACT THE PERCENTAGES FROM ROUTE A's, AND THE DENOMINATORS ARE
WHY.** Route A's ceiling — 229,150,648 B, 23.3% — is of the **AppDir library
tree after assembly** (938.8 MiB of files). Route B's 40.6% is of the
**closure's NarSize** (386.4 MB). Different populations, measured at different
stages. ⭐ What IS comparable is the shape of each lever on one subject:
**route B costs 8 of 111 store paths from source**, and route A can only ever
delete what nothing reaches, which is where its 23.3% ceiling comes from.

⚠ **And a control fired while parameterising this**, which is the reason to
record it: the "nothing is downstream of the top but itself" case still seeded
on the literal string `kdenlive`, so against `mesa-demos` it found 0 seeds and
reported **`= 0, expected 1`**. The seed is the store path's own name now. A
control that only ever ran against the subject it was written for would have
gone green and the whole table would have been built on a graph nobody had
checked the orientation of.

#### ⚠ AND THE `kdenlive` NUMBERS ARE A RE-DERIVATION ON A DIFFERENT BUILD

Re-running the parameterised experiment resolved kdenlive to a **different
store path** — hydra's `latest-finished` had advanced from
`rybc03ipfn2fdncwhlp1awh4q56wjd0i` to `syyn0lv03zc71c6z6k12mivaz7qkmhc8`, the
same version from a later evaluation. ⭐ **Every path count came out
identical**: 676 in the closure, 78 downstream of qtbase, 85 of mesa. The byte
totals moved by about 13 KB in 2.9 GB, consistent with a few paths rebuilt.
⭐ So the structural answer does not depend on which evaluation was current,
and the counts quoted above are the same on two of them.

### ⛔ 2026-09-03c: THE CONTROL WAS SHARING CODE WITH THE SUBJECT, AND FIXING THAT FOUND A SECOND ROOT-OF-ITSELF

⚠ **The `selfKeys` fix above was applied to the fast scan AND to
`sonamesMentionedNaive`, the oracle that exists to check it.** The two went on
agreeing exactly — by calling the same function. ⛔ **A control that shares the
code under test proves the wrapper and nothing else**, and the previous session
recorded that reduction here rather than leaving it to be discovered.
`PROGRESS.md` R2.

⭐ **The control computes the self-set itself now, and with a different
instrument.** `selfSetNaiveFor` asks `os.SameFile`, which compares the **device
and inode** a `stat` returned; `selfKeys` resolved **path strings** with
`filepath.EvalSymlinks`. Same question, two instruments, no shared code on
either half of the job.

⛔ **THEY DISAGREED ON THE FIRST NEW CASE, AND IT IS A REAL DEFECT.** A hardlink
is not a symlink — it **is** the file, so `EvalSymlinks` returns it unchanged.
`libhard.so.9` and `libhard.so.9.0.1`, one inode under two names, landed in two
different groups; the object's own SONAME was then not in its self-set, and the
library became a **root of itself** — the identical defect the section above
records, in the one shape its fixture lacked.

    before (selfKeys keyed on the resolved PATH)
      FAIL  the fast scan agrees with the naive one, exactly
            = libhard.so.9 libinlist.so.1 libinpath.so.2 libplain.so.0 libtight.so
              wanted libinlist.so.1 libinpath.so.2 libplain.so.0 libtight.so
      FAIL  selfKeys groups a versioned file with its SONAME HARDLINK
            = libhard.so.9.0.1, wanted libhard.so.9 libhard.so.9.0.1

    after  (selfKeys keyed on dev:ino, via fileIdentity)
      pgb --selftest: 25 cases, all pass          (bundle-sweep + this one)
      pgb --selftest: 375 cases pass, 1 COULD NOT RUN here   (no zstd)

⛔ **CORRECTED THE SAME DAY, BY MEASURING WHAT I HAD ASSERTED.** The first
version of this section read *"REACHABLE, not hypothetical: nix optimises its
store by hardlinking … and an AppDir assembled with `cp -al` hardlinks the whole
tree. Both are inputs to this sweep."* ⚠ **That overstates it for THIS TOOL'S
OUTPUT — and an AppDir was on disk to check it against.**

    jq AppDir built by `pgb bundle appimage`: files in lib/ with nlink > 1
      ⛔ 0 of 284
    the fetched nix store beside it
      ⛔ 0

**And the reason is structural rather than luck.** NAR extraction *cannot*
produce a hardlink — the format has only regular, executable, symlink and
directory nodes — and the single `os.Link` in `internal/bundle` is sharun's
per-binary trick in `bin/`, which is not a shared object. ⭐ **So the fix moves
zero bytes on anything `pgb` currently assembles**, and it must not be written
up as a size win.

⭐ **It is still the right fix, and the reachability claim survives in a
narrower and checkable form.** `pgb bundle sweep` takes **any directory**: a
tree assembled with `cp -al` — which `experiments/89-`'s own `seed_cache()`
does — or a real nix store under `nix-store --optimise`. The scan has to be
right on the input it is *handed*, not only on the output we happen to produce.

⭐ **`os.Stat` follows symlinks, so `dev:ino` subsumes the symlink rule** — one
key now covers the filename, the SONAME symlink, the development symlink AND a
hardlinked SONAME, with no rule about version suffixes and no second code path.

⚠ **And the equivalence assertion is now belt AND braces.** Three cases pin
`selfKeys()` directly against the fixture, because an equivalence is only as
strong as the independence of its two sides — and those are the cases that
would still fire if both scans were changed the same wrong way at once.

### ⚠ A preflight, found by walking into it

`pgb bundle appimage --out DIR/x.AppImage` with no `DIR` fetched the whole
closure, assembled the AppDir, debloated and swept it, and only then handed
mkdwarfs a path it could not open — reporting `mkdwarfs failed` over a log
saying it wrote **0 files**, which reads like the AppDir was empty. The output
directory is created before the closure is fetched now.

## ⚠ Significantly advanced, and STILL OPEN — `experiments/78-`

⭐ **2.86× the field → 1.22×, on `jq`.** ⛔ Not parity, so this entry stays
open with the remaining lever named.

`evidence/78-bundle-cli-bench/RESULT.txt`. ⭐ **The subject is a CLI and that is
what made it possible**: a `jq` bundle builds in about a minute, so this was
four measured iterations in the time one kdenlive build takes.

    ARM          BYTES     COLD_MS  WARM_MS  WORKLOAD_MS  OUTPUT
    none      12261750         126        9           13  6
    safe       4890913          60       10           13  6
    aggressive 4890913          68       11           14  6

    field (Anylinux-AppImages, experiments/86-)   4,006,916 B
    ours, was                                    11,471,610 B   2.86×
    ours, now                                     4,890,913 B   1.22×

⛔ **A correctness column, because a smaller bundle that answers differently is
not a smaller bundle.** All three arms run the same job and their output is
compared byte for byte; that is the experiment's only assertion, because a
wall-clock figure from one machine on one day is not a threshold anything
should fail on.

**Two levers, both structural rather than a list of names.**

| | |
|---|---|
| ⭐ **the reachability sweep, which NOTHING consumed** | `sweep.go` computes the DT_NEEDED closure of every program plus every plugin directory, and had two callers: the `pgb bundle sweep` subcommand, which only prints, and its own selftest. The build path never called it, so every debloat rule was a rule about NAMES and the one structural answer in the tree was shown to a human and thrown away. ⭐ `codegraph callers Sweep` is the one command that shows it — `RULES.md`'s own example of what codegraph is for. **On `jq`: 277 objects, 12.0 MiB.** Safe to delete on because the sweep counts anything it cannot classify as REACHABLE, and `b.integrity()` re-checks every DT_NEEDED afterwards |
| ⭐ **`share/i18n` is 17 MiB of a 22 MiB `jq` bundle** | glibc's locale **SOURCE** data — the text `localedef` compiles FROM, not what a program reads. `cns11643_stroke` alone is 4.31 MiB in a bundle whose entire `lib/` is 4.8 MiB. The rule is **conditional**: a bundle shipping `localedef`, `locale` or `iconvconfig` keeps them. **On `jq`: 15.0 MiB** |

Debloat went from **12.7% off to 86.9% off** on the same closure.

⛔ **What is left, and it is why this stays open.**

1. ⛔ **kdenlive RAN AND FAILED, and the failure was mine.** `experiments/90-`,
   `pass=6 fail=2`:

       ARTEFACT                          BYTES
       P  ours (one command, nixpkgs)  267390365
       E  kdenlive-AppImage-Enhanced   191900604
          ratio P/E                        1.39x     (was 2.49x)

       render:  ours 0 bytes of MP4        enhanced 4162 bytes
       on 11:   ours rendered 0 of 11      enhanced 11 of 11
       clean:   ours 11 of 11              enhanced 4 of 11

   ⭐ The size moved the right way — 477,191,058 → 267,390,365 B, **2.49× →
   1.39×** — and ours is still the only arm that loads no host object on every
   row. ⛔ **But it could not render**, because the sweep ran before `.env`
   existed and deleted the MLT modules. `docs/history/corrections.md` C20; the
   ordering is fixed in `5fbf7ad0` and **the re-measurement has not been made**.
   ⚠ Until it is, the 1.39× is a size for a bundle that did not work, and it
   must not be quoted as a result.
2. ⚠ **The remaining 1.22× is a PACKAGE-SIZE gap, not a bundler one**, and
   `Anylinux-AppImages/FAQ.md` names it: their libraries come from packages
   optimised for size, ours from nixpkgs, and their own example is a
   `libicudata.so` that is *"less than 1 MiB"* in one and *"30 MiB"* in the
   other. ⭐ The next lever is therefore **where the closure comes from**
   (`pkgforge-dev/archlinux-pkgs-debloated` is the named corpus), not another
   debloat rule.
3. `--debloat aggressive` now buys **nothing** over `safe` on `jq` —
   4,890,913 B both. The sweep made the aggressive name-rules redundant for a
   CLI; whether that holds for a GL application is unmeasured.

---

