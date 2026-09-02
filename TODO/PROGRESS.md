# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-02b (recovery session)
    COUNTS    41 entries, 17 open, 24 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: GREEN; selftests 138 pass, 1 could not run (no zstd)
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ GATE 5 IS COMPLETE — ten of ten POCs and twenty-three
              experiments, every row measured. T-061's last gap is closed.

## ⛔ READ THIS FIRST: the toolchain is Go now, and the shell is the oracle

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers, the planner, the verifier and the bundler are the same executable,
built `CGO_ENABLED=0`, carrying the C runtime sources it compiles. The shell
and Python it replaced are under `HISTORY/<commit>/`, unedited, and are the
oracle every gate is measured against.

**Read [`toolchain.md`](toolchain.md) §T-061 for what was required, then
[`../docs/design/toolchain.md`](../docs/design/toolchain.md) "Language and
structure" for the decision, the architecture and the six gates.** The
measurements are in `evidence/92-go-port/RESULT.txt`, per row, as they landed.

⛔ **The experiments and the POCs stay shell** and are the acceptance harness.
An entry below that proposes editing shell under `tool/` or `scripts/` is
describing files that no longer exist; check `HISTORY/` before acting on it.

## ⭐ The operator's three goals

Quoted, because the framing is load-bearing:

> *"1. Make the 'universal' builder true via pgb + nix. 2. make the 'universal'
> bundler true via a modern, updated, maintained 'nixappimage' descendant that
> uses or rather reimplements many of the anylinux tooling, iterating/improving
> them, and debloating nixappimages, correctly packing them, and also solving
> the opengl problem ... 3. poc a kdenlive static (exhaust all resources), if
> impossible, pivot to kdenlive.nixappimage, but it must be smaller, load
> faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."*

| goal | entries | where it stands |
|---|---|---|
| 1. the builder | T-050 ✅, T-051, T-060, T-012 | ⭐ **T-050 CLOSED**: `experiments/88-` plans, fetches **and builds** a nixpkgs package with no nix and no root, 25 assertions. ⚠ It still needs a C toolchain on the host, which is T-051; **T-060** is the static-glibc nix that removes the last crutch and is **rung 1 of 3**, 31 of nix's dependencies built |
| 2. the bundler | T-057 ⚠started, T-052 ✅, T-053 ✅ | ⭐ **items 1, 3 and 4 landed**: debloating with a three-arm control (`experiments/89-`), wrapper environments lifted into `.env`, and the lib32 path. ⛔ **item 2, a 32-bit application, is still untried** |
| 3. kdenlive | T-054, T-055 | ⭐ **rung 2 climbed**: `poc/91-qt-xcb` — a static Qt 6 opening a **real xcb window**, 26 assertions, 11/11, zero host objects. ⛔ **T-055's bar is NOT met**: ours 395,294,317 B against 191,900,604 B |

## What the last session did (2026-09-02b, a recovery session)

The session before this one terminated itself when `poc/91-qt-xcb` filled the
disk. It had pushed everything; nothing was lost. ⚠ The clone comes up SHALLOW,
grafted at `21e7dc06`, which makes local `main` look like an unrelated history
with 22 orphan commits — it is not, and the recovery is `fetch --unshallow`
then a fast-forward, never a force-push.

### 1. ⭐ GATE 5 IS COMPLETE — T-061's last gap

    poc/91-qt-xcb            rc=0  1,429 s  27 assertions, 11 of 11
    experiments/86-          rc=0           7 cases, both arms 11 of 11
    experiments/90-          rc=0           10 cases, ⭐ 0 SKIPPED

⚠ **Qt cleared the wall for a reason worth carrying**: not more disk. The
failing session had run nine POCs and twenty-one experiments into
`/var/tmp/pgb-poc` before starting Qt; this one started clean. Peak was 6.5 GiB
for the qtbase build tree alone.

⭐ **`experiments/90-`'s onelf arm ran for the first time**, and the defect
blocking it was ours for the second time — the staging step copied
`shared/bin/*`, a shell glob never matches a leading dot, and a nixpkgs wrapper
leaves the payload ELF beside itself as `.NAME-wrapped`.
`../docs/history/corrections.md` C16.

### 2. The operator's post-port instruction

Codegraph installed and the gate reports the index current. The deprecation
sweep ran with tools rather than by reading — **six import-silencing hacks**
(`var _ = pkg.Symbol`) that neither `go build` nor staticcheck can flag by
construction, two dead functions, and **71 modernize rewrites** across 38
files. Documents to retire: **answered — nothing qualifies**, and
`../tmp/README.md` says why so it is not re-litigated a third time.
`../docs/AGENTS.md` §0b is the cold start, and it now links
`methodology/sessions.md`, which the entry point had never referenced.

### 3. Defects found, each by something disagreeing

1. ⛔ **`make check` never reached either record gate** on a machine without
   zstd. `pgb selftest` exits 2 for "could not run"; make treats non-zero as
   failure. The documented command claimed to run gates it never reached.
2. ⛔ **`README.md`'s first code block could not run.** `sh pgb env create` is
   a syntax error — pgb has been a Go binary since `4ef2acc7`. `AGENTS.md` §1
   still described it as "a POSIX-sh driver plus four small C runtime pieces".
3. ⛔ **`pgb nix build --configure` reached every dependency**, not the package
   named; numactl's configure said `unrecognized options: --without-icu`.
4. ⛔ **Five defects in the adaptation loop**, all found by T-063's arm S — it
   could remove NONE of the fifteen optional features nixpkgs' postgres plan
   enables, and removes thirteen now. Its default round budget of 8 reports
   `gave up after 8 rounds`, which reads as "cannot be built" and is not that.
5. ⛔ **`internal/nixx`'s adaptation logic had no selftest at all**, which is
   T-062: `pgb selftest` prints 138 cases and reaches 7 of 17 packages.

### 4. T-063 — a static PostgreSQL that runs on Alpine

  `pgb rootfs run alpine-3.22 -- /postgres --version` → `PostgreSQL 18.6`,
  63,889,168 B, no `PT_INTERP`, no `DT_NEEDED`. ⚠ `src/interfaces` (libpq,
  ecpg) does not build yet, so initdb/pg_ctl/psql do not exist and nothing
  claims the miniflux stack runs.

⭐ **Gate 4 was re-measured** after the modernisation touched
`internal/wrapper` — byte-identical, sha256 `251cec64…`.

## In progress

⛔ **Nothing is running.** Every job this session started finished and its row
is in the evidence file.

⚠ **T-060 rung 1's `/var/tmp` build tree is gone with an old container.** The
run is idempotent; treat it as never started.

## ⭐ Work order

⛔ **FOUR NEW P0s, set by the operator on 2026-09-02b, and each carries the
same instruction: work until it is met or the premise is significantly
advanced.** They outrank everything below them.

    T-064   ⛔ static glibc's dlopen, REALLY solved — our own ELF loader,
            resolving against our own static glibc. experiments/73- already
            measured that 90.8–97.8% of host imports are definable by it and
            the unexplained residue is ZERO. Restudy references/pg83__solo
            (elf_loader.cpp is 2,707 lines and most of that is musl
            translation a glibc host does not need) and beat it
    T-065   ⛔ anylinux dlopens the HOST on purpose, and this project asserts
            that is always failure. It is right for a static ELF and WRONG for
            a bundle: sharun searches bundled-first with a documented
            lowest-priority host fallback and per-class opt-ins for mesa,
            Vulkan ICDs, NVIDIA and even a newer host glibc. Restudy the whole
            family, write the policy up, adopt it
    T-066   ⛔ the bundler is bloated and slow — 2.86x on jq, 2.49x and ~3x
            slower on kdenlive. ⭐ Iterate on a CLI (bash or 7z), NOT on
            kdenlive: minutes not 20, and it benchmarks cleanly. The
            reachability sweep exists and NOTHING consumes it, which is the
            largest unused lever in the tree
    T-067   ⛔ does zig buy anything tool/runtime/'s C cannot? ⭐ A measured
            "C is adequate, here is why" CLOSES this entry — it is a question,
            not a migration

    ---- and then ----

    T-063   the miniflux proof: arm S has a static postgres running on Alpine;
            src/interfaces does not build, so initdb/pg_ctl/psql do not exist
    T-062   eight packages carry no selftest, internal/wrapper first
    T-055   the bundle size cut — folded into T-066's iteration
    T-060   rungs 2 and 3, the static nix
    T-054   rungs 3 (KF6) and 4 (kdenlive static)
    T-057   item 2: a 32-bit application through the lib32 path
    T-051   the no-compiler host
    then P2 by category

⭐ **Two pieces of real work are NAMED and are not entries**, because each is
one clear fix inside T-063 arm S:

    the static link-order problem   AC_SEARCH_LIBS probes -lreadline alone and
                                    libreadline.a's ncurses references go
                                    unresolved. poc/91-qt-xcb answered the same
                                    class with -Wl,--start-group
    a C link that pulled a C++      libicuuc.a needs operator delete and the
    archive                         __cxxabiv1 vtables; LinkFlags already takes
                                    a `cxx bool` and does not notice this case

## Open questions for the operator

⭐ **None blocking.**

1. ⚠ **One branch exists on the remote and this session did not create it.**
   The harness named `claude/glibc-pgb-recovery-6dleai`; `RULES.md` §Git
   outranks it and every commit is on `main`. The branch was already on the
   remote at `main`'s commit when this session started and the git proxy
   refuses remote deletes, so it is left for a human to remove in the web UI.
2. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`.
3. ⭐ **The porting report is gone, as the operator asked.** Its content is
   in `docs/design/toolchain.md` "Language and structure" and T-061 deleted
   the file.
