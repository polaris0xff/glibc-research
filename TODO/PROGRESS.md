# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-02
    COUNTS    34 entries, 15 open, 19 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: GREEN, 15 jobs, and it asserts criterion 2
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ T-061 IS SUBSTANTIALLY LANDED. The toolchain is Go. What is
              left of it is listed under "In progress" and is NOT the whole
              entry: gate 5 has rows still running.

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

## What the last session did

### 1. T-061 — the whole toolchain ported to Go

~9,300 lines of POSIX shell and Python became one static binary. Every retired
file was moved with `git mv` to `HISTORY/<commit>/<its original path>`, per the
operator's ruling — the shell driver, the six sourced shell modules, the seven
Python helpers, the bundler, the libiconv builder and the whole of the old
`scripts/common`. Two commits hold them, one per retirement wave; `HISTORY/`
lists both and `HISTORY/README.md` says why they are kept. ⭐ `tool/` is C
runtime sources and nothing else now.

⭐ **Gates 1, 2, 3, 4 and 6 are met with output**, and requirement 2's second
half with them: pgb built by pgb inside the pinned environment is
byte-identical to the host build. Gate 5 is nine POCs and eighteen
experiments; the rows are in the evidence file and the rest are named under
"In progress".

### 2. The eight defects the port found, in code that had been trusted

Each was found by a measurement disagreeing, not by reading:

1. ⛔ **`nix-plan.py` was not deterministic.** An output store path can be
   claimed by more than one derivation — 14 of them in git's graph — and the
   Python was last-writer-wins over document order. Ten shuffles of the same
   graph gave 4 of one plan and 6 of another; the Go planner sorts the
   claimants and gives one plan 10 times out of 10.
2. ⛔ **The shell bootstrap built the wrong environment.** It called
   `pgb env create` with no engine after starting dockerd, so the "chroot
   environment" it made was a second docker one. `pgb bootstrap` names the
   engine.
3. ⛔ **`pgb nix deps` did not converge from a cold prefix.** It stops
   recursing at `NIX_DEP_DEPTH`, so a dependency whose inputs sit below the
   cut fails until a sibling has built them. `poc/91-qt-xcb` was five X
   libraries short and a second run would have fixed it. The walk repeats
   passes now while one is still landing something.
4. ⛔ **Requirement 3 was written and never wired.** `internal/logx/stamp.go`
   had the columns, the parser and the heartbeat, and nothing called
   `NewStamper`: `pgb --ts build` printed no timestamps at all.
5. ⛔ **`pgb selftest <typo>` printed "0 cases, all pass".** A name that
   matches nothing selected nothing and reported success.
6. ⛔ **CI had been red for every commit of the port and nobody had looked.**
   `pgb` is a built binary now and is not committed, so three jobs did
   `actions/checkout` and then ran `./pgb` against nothing. A `toolchain` job
   builds one static pgb and hands it to the rest as an artefact.
7. ⛔ **A skip counted as a failure**, found by the repaired CI within
   minutes. The runner is not root, `rootfs-run` needs root, and the report
   returned 1 for it. It returns 0 / 1 / 2 now, like everything else here.
8. ⛔ **The libiconv build needed `msgfmt`.** The pinned image has no
   gettext; this machine had been given it by hand, which hid the problem.
   `--disable-nls` removes the dependency rather than adding one.

### 3. zstd, decoded in Go

cache.nixos.org serves NARs as `.nar.zst` and the pinned build environment
carries no `zstd` binary, so `pgb nix build` inside it stopped dead — the
retired Python reached `libzstd.so.1` through ctypes and `CGO_ENABLED=0` has
no equivalent. `internal/zstd` decodes RFC 8878 with nothing outside the
standard library, measured byte-identical against the reference encoder over
120 frames at levels 1 to 22. ⚠ **xz still shells out**; the build environment
has it and a target rootfs does not.

### 4. The documentation

`docs/design/toolchain.md` "Language and structure" was still asserting that
the driver stays POSIX `sh`, and listing a sourced-shell-module layout that no
longer exists. It records the Go decision now, with the ranked comparison,
what "single binary" can and cannot mean, the package layout and the six
gates folded in from the commissioned porting analysis, which is deleted as
the operator asked.

## In progress

⛔ **These were running when the session was checkpointed and their rows are
NOT in the evidence file.** Each is a shell script that can simply be re-run;
none of them holds state that must be preserved.

    poc/91-qt-xcb          from a COLD prefix, the proof of the fixed-point
                           dependency walk. Was at 1,256 of 1,644 Qt objects.
    experiments/89-        the debloater's three arms, Go bundler
    experiments/60-        versus the alternatives

⚠ **T-060 rung 1's `/var/tmp` build tree is gone with an old container.** The
run is idempotent; treat it as never started.

## ⭐ Work order

    T-061   ⚠ FINISH IT. Gate 5's remaining rows (91-qt-xcb, 86, 89, 90, 60,
            62), then the operator's post-port instruction: codegraph, the
            deprecation sweep, two deep reviews, and a docs/AGENTS.md a fresh
            session can start from alone.
    ---- and then ----
    T-055   the bundle size: 489 MB of lib/ is unreachable and that is the cut
    T-060   rungs 2 and 3, the static nix
    T-054   rungs 3 (KF6) and 4 (kdenlive static)
    T-057   item 2: a 32-bit application through the lib32 path
    T-051   the no-compiler host
    then P2 by category

## Open questions for the operator

⭐ **None blocking.**

1. ⭐ **No branch debt.** The harness named `claude/glibc-nix-static-v2nttp`;
   `RULES.md` §Git outranks it and every commit is on `main`.
   `git ls-remote --heads origin` lists `refs/heads/main` and nothing else.
2. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`.
3. ⭐ **The porting report is gone, as the operator asked.** Its content is
   in `docs/design/toolchain.md` "Language and structure" and T-061 deleted
   the file.
