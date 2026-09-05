# SUMMARY.md — the session of 2026-09-05

⛔ **Overwritten every session.** The work order is
[`PROGRESS.md`](PROGRESS.md); the closed entries are
[`../HISTORY/entries/`](../HISTORY/entries/).

    SCOPE     Re-run the corpus (C49 + C54), 108-, T-094's count, the POC
              suite, 105-, 103-, T-093.
    RESULT    ⛔ THE CORPUS RE-RUN IS STILL IN FLIGHT — 2 of 26 subjects at
              the checkpoint, started 03:14Z, ~28 min per subject. Everything
              else in the scope is blocked behind it (the machine cannot host
              a third GUI instance, and `make` is forbidden mid-run).
              ⭐ THE SESSION'S YIELD IS FOUR DEFECTS THAT WOULD HAVE MADE THAT
              RUN WORTHLESS, plus two found by review afterwards. Every one
              was caught by RUNNING something, never by reading alone.
    ⛔ THE    An instrument validated only against its own FIXTURE. C57 and
    PATTERN   C60 are both "the code was checked against what the author
              believed the input looks like". A real trace and a real build
              disagreed with both.

## ⭐ What moved

| | before | after |
|---|---|---|
| **T-094's count** | *"that count does not exist"* | ⭐ **instrumented and in flight**: `exp_host_spawns`, a spawns store beside the rows, and C9a/C9b as its positive control |
| ⛔ **the spawn instrument** | — | **C57**: one pass MISSED the spawn it exists to find (`vfork` writes the child's `execve` before the line naming its pid); the obvious fix then counted the LAUNCHER, which would have read 26 of 26 |
| ⛔ **`65-`'s resume** | resumable by design | **C58**: the reuse guard is `[ -s ]` — non-empty, not COMPLETE — so a killed run left a fragment the next run ran on all eleven, scoring the **C6 control 0/11** |
| ⛔ **the bundler's `--out`** | written in place | ⭐ **atomic**: `<out>.part` + rename, fixing the C58 class at the producer for ~20 call sites |
| ⛔ **gate 10** | green | **T-096**: it keyed on the evidence **DIRECTORY**, so a README beside a result silenced it. Re-keyed: **8 stale pairs where it reported 0**, four of them evidence produced by the **shell predecessor** |
| ⛔ **`108-`'s criterion** | *"a PNG matching the server"* in the header | the code accepted any PNG with `w>0,h>0`; now it compares against the size read back from `xdpyinfo` |
| ⛔ **the interposer** | *"`open`, `stat`, `execve` and friends"* | **C60**: `execvp`, `execl`, `posix_spawn` are **not rewritten**, and `libglib-2.0.so.0` imports all three. **T-097** |
| ⛔ **the loader's size** | *"1,093 code lines"* in five documents | **C59**: **1,398**, measured. It had drifted 28% in three days and no gate could see it |
| **`pgb selftest`** | 601 pass, 1 skip | ⭐ **602, zero skips** — `zstd` was simply not installed |

## ⭐ The measurements, each with its verdict line

| | verdict | note |
|---|---|---|
| `lib.sh --selftest` | ⭐ **29 pass, 0 fail** | 15 new rows |
| `criteria-audit.sh --selftest` | ⭐ 4 pass, 0 fail | and it found `108-` on the real tree |
| `pgb selftest` | ⭐ **602 cases, all pass** | zero skips for the first time here |
| `65-` `gtk3-1` galculator | ⭐ **11/11 pass, 11/11 clean, 0 spawns** | the C6 positive control, agreeing with `64-` |
| `65-` `sdl-2` stella | ⭐ 11/11 pass, 11/11 clean, 0 spawns | |
| gate 10, re-keyed | ⛔ **8 disagreements** | was 0 |
| the interposer probe | ⛔ **3 of 7 exec paths not rewritten** | with a control that reads 0 of 7 |

## ⛔ WHAT IS OWED, AND IT IS ALL BLOCKED ON ONE RUN

`RESUME.md` carries the ordered list. In short: the corpus finishes → the
unfiltered read-back writes the verdict → `make` → the POC suite → the bed
fixtures → `108-` → `105-` → `103-` → T-096's four re-runs.
