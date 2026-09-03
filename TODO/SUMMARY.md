# SUMMARY.md — the session of 2026-09-03c

⛔ **Overwritten every session.** The history is the git log, and the previous
session's summary is
[`../HISTORY/sessions/2026-09-03.md`](../HISTORY/sessions/2026-09-03.md).

⭐ **The headline: the list of nine glibc quirks was not complete, and the
tenth was found and closed the same day.** `docs/REQUIREMENTS.md` said *"there
is no unenumerated remainder"* — the sentence that made the operator's part 2
countable. It was false. `grep -rn zoneinfo` over the whole tree returned
**nothing**, and static glibc turns out to read the host's **timezone
database**, which **four of eleven** environments do not have — including
`ubuntu-20.04`, which is glibc. `--embed-tzdata` closes it on 11 of 11.

⭐ **And the bundler's acceptance bar changed shape**: the operator struck
**size** and put **speed** and **one-command packaging** in its place. That is
harder, not softer — the conditions are conjunctive, one-command packaging is a
win `pgb` can already publish, and the clock is the column it is furthest
behind on.

## Before and after

| | at start | at end |
|---|---|---|
| **the bundler's bar** | size, shape, friction, honesty | ⭐ **speed + one command**; size struck. Every byte lever is **un-scored** until re-measured in milliseconds |
| **the bundler's timing numbers** | published as fact | ⛔ **do not re-derive** — from a superseded evidence file, one sample per arm, ratios of 2.52×/3.48×/4.92×/5.02× across four runs, warm above cold in two. ⭐ Direction survives; magnitude does not. C23 |
| **the glibc quirks** | "nine, no unenumerated remainder" | ⛔ **TEN**, and the claim of completeness is withdrawn. ⭐ Nine closed, one open |
| **timezone** | not on the list, never measured | ⭐ **closed**: `--embed-tzdata`, 11 of 11, 193,208 B for 20 zones |
| **`TODO/*.md` entry text** | 5,085 lines | ⭐ **545**. 34 closed entries + the long-form findings behind the open ones in `HISTORY/entries/` |
| **that split** | a convention | ⭐ **gated** — `check.sh` 4b/4c/4d, each proved able to fail |
| **`cxxCandidates`** | fixed for `-lNAME` | ⛔ still skipped the **separated** `-l NAME`. R3's fix went halfway; both spellings on one resolver now |
| **`exp_run_status`** | "could not run" and "exited N" both `2` | ⭐ a non-numeric token, so a comparison fails loudly |
| **the carried note on evidence** | "an experiment writes its own RESULT.txt" | ⛔ **inverted** — 19 of 32 do, 13 do not. `scripts/common/run-experiment.sh` makes one command right for all of them |
| **stale evidence** | invisible | ⭐ **gate 10**: 7 of 32 experiments had evidence older than their own script. Three re-run, four pinned with reasons, T-077 owns them |
| **the four new references** | mined, unread | ⭐ **read**, four passes + tracker, two write-up files. Two findings correct the record |
| **selftests** | 540 | ⭐ **546**, 1 could not run (no zstd) |
| **Entries** | 48 / 14 open / 34 done | **50 / 15 open / 35 done** |

## ⛔ What was found, and not one came from reading code

1. ⛔ **The tenth glibc quirk**, found by taking *"are there still some?"* as a
   question about **completeness**. And its failure is worse than "returns
   UTC": glibc re-reads `TZ=Europe/Berlin` as a POSIX spec and prints
   `Europe +0000` — **the zone name you asked for, at a UTC offset**, so the
   field that looks like a confirmation is an echo of the input.
2. ⛔ **The bundler's milliseconds do not re-derive**, found by opening the
   file the entries cite, on the day those milliseconds became the whole bar.
3. ⛔ **`-l NAME` with a space was still skipped** — found by reading the
   session's own fix adversarially, proved red before being fixed.
4. ⛔ **A helper collapsed this project's own exit convention**, found by
   running it three ways and getting `2` from all of them.
5. ⛔ **A carried machine note was backwards**, found because acting on it
   re-ran an experiment that refreshed nothing and reported success.
6. ⛔ **Seven experiments' evidence predates their own script**, found by
   comparing two `git log` timestamps — including one whose committed table
   names the environment T-070 retired.
7. ⛔ **Four open entries lost the pointer to their own detail** within an hour
   of the strip that created them. Now gated.
8. ⭐ **A carried selftest caught a live defect while it was being written**:
   `PGB_OPT_EMBED_TZDATA` was exported without being added to `cfg.OptVars`,
   which is T-019's class, caught in seconds.
9. ⛔ **Two of this project's written-down claims were corrected by somebody
   else's repository** — "there is no static nix to fetch" (the nix flake
   ships one) and T-051's `--store` route (it fails on Debian stable, with a
   probe that passes first).

## What is left

⛔ **The next session is scoped by the operator: the bundler, on the clock.**
`PROGRESS.md` N0–N6, and **N0 is not optional** — fix the instrument before
measuring anything with it. ⭐ N2 names the hypothesis that decides whether any
of the size work counts, and `pelf`'s 350 MB mount-versus-extract threshold is
independent corroboration for it.

⚠ **Deferred, plainly**: nothing from the reference sweep was **run**;
`60-`, `61-`, `62-` and `88-` were not re-run on the current pin (T-077);
T-012's git/URL route, by the operator's own instruction.
