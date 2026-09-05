# EXPECTED.md — what the 2026-09-05 corpus re-run should produce

⛔ **WRITTEN AND COMMITTED BEFORE THE SUBJECTS IT NAMES WERE MEASURED.**
`TODO/PROGRESS.md` delivery rule 1. At the moment this was written the run had
recorded exactly **two** rows — `gtk3-1` and `sdl-2` — and **none** of the
subjects named below. The git log is the proof that this was not written
afterwards.

⭐ **WHY THIS RUN EXISTS.** Two corrections owe it one re-run:

* **C49** — `exp_classify_trace`'s host test was a prefix list whose complement
  was scored `bundled`, so a host object outside `/lib` and `/usr/lib` read
  **clean**. Thirteen such files exist across the eleven rootfs and one of them
  is **`/usr/bin/ld.so`, the host loader**. Runs in the dangerous direction:
  **dirty → clean**.
* **C54** — a row counted **clean** when it loaded zero host objects, which is
  also what a subject that **never started** reports. The guard is now
  `bundled > 0 AND host == 0`.

⚠ And one measurement rides along: **T-094's spawn count**, which did not exist.

## ⭐ THE PRIOR RUN, 2026-09-04c — the numbers this replaces

`FULL 21 of 26`, `CLEANALL 24 of 26`, taken under the **old** predicate and the
**old** clean rule.

| the five that did not pass on all eleven | why, as recorded |
|---|---|
| `vulkan-3` vkmark | the **BED**: no `/dev/dri` anywhere here |
| `py-2` pdfarranger | **OURS, and a class**: `/usr/local/share/pdfarranger/…` asked of Python at run time, which `pgb-storefix.c` does not answer |
| `field-2` neovim | the **CLOSURE**: its own glibc 2.26 rejects the loader invocation |
| `field-3` flameshot | the **SUBJECT**: a tray application with no toplevel |
| `field-4` gearlever | a `RuntimeError` out of libadwaita |

| the two that were not clean on all eleven | why |
|---|---|
| `x11-3` xterm | **pre-registered as C5**: its job is to run the user's **shell**, a host program |
| `qt-1` qalculate-qt | **C55**: it probes for `gnuplot` through the host's `/bin/sh` |

## ⛔ THE PREDICTIONS, AND EACH ONE CAN FAIL

**P1 — C49 moves nothing.** No corpus row's clean count changes because of the
widened host predicate. ⚠ The argument (C49) is that all thirteen newly-host
files are reachable only by running `stdbuf`, `sudo`, or the host loader **by
that path**, and no corpus subject does any of those. ⛔ **It is an argument,
not a measurement, which is exactly why it is written down before the run.**
*Falsified if* any subject that was clean 11/11 comes back below 11.

**P2 — C54 demotes `field-2` (neovim), and only `field-2`.** Its artefact never
executes, so it loads nothing at all — host or bundled — and the new rule
refuses to count that as clean. Its clean count must fall **below 11/11** and a
`NOSTART` line must name it. ⛔ *Falsified if* it still reads 11/11, or if a
subject whose program **does** run is demoted.

⚠ **The other four zero-pass subjects should NOT be demoted**, and that is the
sharper half: `flameshot` puts a 3×3 selection owner on the server, `gearlever`
reaches a Python `RuntimeError`, `vkmark` runs and finds no `/dev/dri`, and
`pdfarranger` reaches a `FileNotFoundError` — **all four RAN**, so all four
loaded bundled objects and stay clean.

**P3 — `CLEANALL` therefore lands on 23 of 26**, not 24: the same two as before
plus `field-2`. **`FULL` stays 21 of 26.**

**P4 — T-094's count is between 2 and 10 of 26**, and `qt-1` and `x11-3` are in
it. ⛔ Those two are the spawn instrument's **positive control** (C9a/C9b): one
was read off a trace by `experiments/107-`, the other is predicted by C5's own
reasoning. If either reads zero the instrument is broken and no other subject's
zero means anything.

## ⚠ WHAT WOULD MAKE THIS RUN UNREADABLE

**C6.** `gtk3-1`, `gtk3-2` and `py-1` were each measured at 11 of 11 twice by
`experiments/64-`. A row below 11 for any of them is a disagreement with a
known-good measurement, and the instrument is the first suspect — not the
capability. ⭐ `gtk3-1` has already come back **11/11 pass, 11/11 clean**.
