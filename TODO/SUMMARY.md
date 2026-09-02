# SUMMARY.md — the session of 2026-09-02b

⛔ **Overwritten every session.** The history is the git log.

A recovery session. The one before it terminated itself when `poc/91-qt-xcb`
filled the disk; it had pushed everything and nothing was lost.

## Before and after

| | at start | at end |
|---|---|---|
| **Gate 5** | ⛔ INCOMPLETE — 9 of 10 POCs, 21 of 23 experiments | ⭐ **COMPLETE** — 10 of 10 and 23 of 23, every row measured |
| **`experiments/90-` onelf arm** | skipped for two sessions, blamed on onelf | ⭐ **ran**, 0 skips, and the defect was ours |
| **Entries** | 34 / 15 open / 19 done | 36 / 17 open / 19 done |
| **Carried selftests** | 124 cases, 7 of 17 packages | 139 cases; `internal/nixx`'s adaptation logic now covered |
| **`make check`** | ⛔ aborted before **both** record gates on any machine without zstd | green end to end |
| **`README.md` first command** | ⛔ `sh pgb env create` — a syntax error | runs |
| **Static postgres** | unknown, untried | ⭐ **PostgreSQL 18.6 running on Alpine**, no `PT_INTERP`, no `DT_NEEDED` |
| **CI** | green (run 96) | green |

## What was asked, and where each task stands

| # | task | state |
|---|---|---|
| 1 | Gate 5's three missing rows | ⭐ **done** — all three, with evidence |
| 2 | `experiments/90-`'s corrected onelf row | ⭐ **done** — three arms, 0 skips |
| 3 | codegraph installed and wired | ⭐ **done** (landed `e44a6519`; installed and gate-verified here) |
| 4 | deprecation / modernity sweep | ⭐ **done** — 6 hacks, 2 dead functions, 71 rewrites |
| 5 | two deep reviews, code and docs | ⭐ **done** — findings below; T-062 filed |
| 6 | retire unused documents | ⭐ **answered: nothing qualifies**, and `tmp/README.md` records why |
| 7 | `docs/AGENTS.md` a complete cold start | ⭐ **done** — §0b, and `sessions.md` is linked at last |
| 8 | the miniflux proof | ⚠ **arm S substantially done, entry open.** T-063 |
| 9–14 | the backlog | not started |

## The defects, and not one was found by reading

1. ⛔ **`make check` never reached either record gate.** `pgb selftest` exits 2
   for "a case could not run"; make treats non-zero as failure, so on any
   machine without `zstd` it stopped at the selftest line and `TODO/check.sh`
   and `check-docs.sh` never ran. The documented command claimed to run them.
2. ⛔ **Six `var _ = pkg.Symbol` import-silencing hacks.** Neither `go build`
   nor staticcheck can see these *by construction* — the hack uses the import,
   and it is an assignment rather than an unused declaration. Every one kept a
   zero-use import alive.
3. ⛔ **`README.md`'s first code block could not run.** `sh pgb env create` →
   `pgb: 1: Syntax error: ";" unexpected`. `AGENTS.md` §1 still called pgb "a
   POSIX-sh driver plus four small C runtime pieces".
4. ⛔ **`docs/methodology/sessions.md` was never linked from `docs/AGENTS.md`** —
   a cold-start agent following the entry point never learned the ending
   protocol existed. That was task 7's actual content.
5. ⛔ **`experiments/90-` dropped two payload ELFs.** `cp .../shared/bin/*` —
   a shell glob never matches a leading dot, and a nixpkgs wrapper leaves the
   real ELF as `.NAME-wrapped`. The recipe (written from a readdir, which sees
   them) named an entrypoint the packed directory did not contain. ⭐ The line
   below it already used the correct `lib/.` form. Correction **C16**.
6. ⛔ **`pgb nix build --configure` reached every dependency**, not the package
   named. `numactl`: `configure: WARNING: unrecognized options: --without-icu`.
7. ⛔ **Five defects in the adaptation loop**, all surfaced by T-063's arm S.
   It could remove **none** of the fifteen optional features nixpkgs' postgres
   plan enables; it removes thirteen now.
8. ⛔ **The adaptation loop's round budget is 8** and reported `gave up after 8
   rounds` with nine flags still to remove — which reads as "this package
   cannot be built" and is not that.
9. ⛔ **`internal/nixx`'s `diagnose` had no selftest**, and it is the only pure
   part of `pgb nix build` and the part that decides whether a package builds.

## What is left, in order

1. **T-063.** Arm S has a static postgres that runs on Alpine; `src/interfaces`
   (libpq, ecpg) does not build, so `initdb`/`pg_ctl`/`psql` do not exist yet.
   Then arm B, then the stack serving HTTP on Debian 12 and Alpine 3.22.
2. **T-062.** Eight packages carry no selftest, `internal/wrapper` first — it
   is the product, and gate 4 is its only acceptance.
3. Then T-055, T-060 rungs 2–3, T-054 rungs 3–4, T-057 item 2, T-051.

⭐ **Two pieces of real work were named and are not entries yet**, because both
are one clear fix inside T-063 arm S: the **static link-order** problem
(`AC_SEARCH_LIBS` probes `-lreadline` alone — `poc/91-qt-xcb` answered the same
class with `-Wl,--start-group`), and **a C link that pulled in a C++ archive**
(`libicuuc.a` needs `operator delete`; `LinkFlags` already takes a `cxx bool`
and does not notice this case).

## Open questions for the operator

⭐ **None blocking.**

⚠ **One branch exists on the remote and this session did not create it.** The
harness named `claude/glibc-pgb-recovery-6dleai`; `RULES.md` §Git outranks it
and every commit is on `main`. It was already on the remote at `main`'s commit
when this session started, and the git proxy refuses remote deletes, so it is
left for a human to remove in the web UI.

⚠ **This machine was extended and a fresh container will not have it**, which
is what the onelf arm needs: `rustup target add x86_64-unknown-linux-musl` and
`apt-get install musl-tools`. Without both, arm O skips.
