# spawns — which HOST programs each corpus subject ran

⭐ **THIS ANSWERS A QUESTION THE HOST-OBJECT COUNT CANNOT.**
`experiments/lib.sh`'s `exp_classify_trace` says *how many* host shared objects
a subject loaded. It cannot say **where they came from**, and
[`../../../docs/history/corrections.md`](../../../docs/history/corrections.md)
**C55** found that for at least one subject they came from the application
itself:

    execve("/bin/sh", ["sh", "-c", "--", "/nix/store/…-gnuplot-6.0.5/…"])

`qalculate-qt` probes for gnuplot through the **host's** shell. The shell loads
the host's libc, and those objects land in the `nhost` column looking exactly
like a bundler leak. ⛔ **No path rewriting prevents it** — the bundle would
have to carry a shell — so the first thing anybody needs is the count, and
**T-094** is that entry.

One file per subject, `<id>.tsv`, TAB-separated, one line per distinct host
program per environment:

    <environment> TAB <ok|fail> TAB <program path>

`ok` is an `execve` that succeeded; `fail` is one that did not. ⚠ **Both are
kept.** A failed exec still means the application *asked for* a host program,
and on the four musl rows C55 measured that exec never completing at all.

## ⛔ AN EMPTY FILE AND A MISSING FILE ARE DIFFERENT, AND THAT IS THE POINT

| | means |
|---|---|
| the file exists and is **empty** | ⭐ measured, and the subject spawned **no** host program |
| the file is **absent** | ⚠ **not measured** — the row predates this instrument |

⛔ The table in `experiments/65-` prints `-` for the second case and never `0`.
Five separate criteria in this tree have been found reporting an **absence** as
a **zero** — C48, C50, C52, C54, C56 — and this directory is shaped to make
that impossible for T-094's count.

⚠ A subject whose criterion failed its own sanity check writes no row, and its
spawns file is deleted with it: a half-measured subject must not read back as
`0/0` on the next run.

## What produces this

`exp_host_spawns` in [`../../../experiments/lib.sh`](../../../experiments/lib.sh),
covered by `sh experiments/lib.sh --selftest`. ⛔ Its host test is the
**complement** of the artefact's own locations — deliberately the opposite of
the prefix list C49 corrected — so it can only over-report, and every path is
named rather than counted.

## What was measured before the 2026-09-05 run was stopped

⛔ **The run was stopped by the operator at 4 of 26 subjects** (a refactor
pivot), so this store is **partial** and the corpus verdict was never written.

| subject | rows | spawns |
|---|---|---|
| `gtk3-1` galculator | 11/11 pass, 11/11 clean | **0** |
| `gtk3-2` mousepad | 11/11 pass, 11/11 clean | **0** |
| `sdl-2` stella | 11/11 pass, 11/11 clean | **0** |
| `sdl-3` scummvm | 11/11 pass, 11/11 clean | **0** |
| ⭐ `qt-1` qalculate-qt | **no row — 2 of 11 environments only** | ⭐ `ok /bin/sh` on both |

⭐ **C9a, the instrument's positive control, FIRED.** `experiments/107-` read
`execve("/bin/sh", ["sh", "-c", "--", "/nix/store/…-gnuplot-…"])` off
`qalculate-qt`'s trace (**C55**); an instrument written from scratch this
session reproduced it on the two environments it reached. That is the
validation the count depends on.

⚠ **`gtk3-1`, `gtk3-2`, `sdl-2` and `sdl-3` are the only complete rows**, and
all four are `0` — measured, not absent. ⛔ `gtk3-3` and `qt-1` carry files
with **no row**, so they are PARTIAL: the analysis keys off `rows/`, not off
this directory, and a resumed run truncates the file before re-measuring.

## ⚠ One known limit of the reader, found while reading this output

`exp_host_spawns` labels a spawn `fail` when the `execve` line carries `= -1`.
⛔ **A SPLIT `execve` whose failure lands on the `<... execve resumed>` line is
therefore labelled `ok`.** The real-trace validation covered a split execve
that *succeeded*, never one that failed.

⚠ **It does not move T-094's count**, which is *how many subjects ask for a
host program at all* — an attempted exec is still asking. It can only mislabel
the `ok`/`fail` column. Worth fixing before that column is quoted.
