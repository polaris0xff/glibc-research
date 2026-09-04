# SUMMARY.md — the session of 2026-09-04c

⛔ **Overwritten every session.** The work order is
[`PROGRESS.md`](PROGRESS.md); the closed entries are
[`../HISTORY/entries/`](../HISTORY/entries/).

    SCOPE     Fix C39 and the defect class behind it, finish experiments/65-
              (T-080), then BY MECHANISM: T-084 step 2, T-091, the three
              unexplained rows, T-088, T-089.
    RESULT    ⭐ ALL OF IT LANDED, and then the session turned into something
              else: ⭐ FOUR BUNDLER DEFECTS in the morning (C41-C44, each
              found by running a recorded zero down), and ⛔ EIGHT INSTRUMENT
              CORRECTIONS in the afternoon (C49-C56), of which TWO can move a
              committed number and ONE turned a claim that had never been
              measured into a measurement.
    ⛔ THE    Five separate criteria in this tree COULD NOT FIRE, and no gate
    PATTERN   could see any of them, because a SKIP is not a failure and a
              ZERO looks like a result. C48, C50, C52, C54, C56.

## ⭐ What moved

| | before | after |
|---|---|---|
| **T-080** the corpus | 20 of 26 rows, five zeros unexplained | ✅ **26 of 26**, every zero named, six categories closed. **Retired** |
| **T-084** the classifier | six hand copies | ✅ **zero**; `102-` reads them back out of git. **Retired** |
| **T-088** `--with-program` | *"never been run"* | ✅ **false** — `90-` was exercising it all along (C45). **Retired** |
| **T-089** the `-static` row | *"a static application is still owed"* | ✅ **answered by a refusal** — a fully static closure has no loader, so there is no artefact to ask. **Retired** |
| ⭐ **`qt-1` qalculate-qt** | pass 11/11, clean 4/11, unexplained | ⭐ **explained**: it spawns `gnuplot` through the host's `/bin/sh`. **C55**, and **two of my own predictions fell** |
| ⭐ **the `--wrap=iconv` claim** | *"structurally better than theirs"* — reasoning | ⭐ **MEASURED**: 1-of-12 encodings and a crash → **12 of 12 with a byte-exact round trip on all eleven**. **C56** |
| ⭐ **rung 3's locale criterion** | *"not measurable in this bed"* | ⭐ **11/11**, after **two** recorded causes that were both wrong. **C48** |
| ⛔ **"clean on all eleven"** | 24 of 26 | ⛔ **counted `neovim`, whose program never starts**. Guard added; the number owes a re-run. **C54** |
| ⛔ **"host" objects** | a prefix list | ⛔ **missed `/usr/bin/ld.so` — the host LOADER**, on four of eleven. **C49** |
| ⛔ **the interposer** | works or does not | ⛔ had a **LOADED AND INERT** state that looked exactly like working, and said nothing. **C53** |

## ⭐ The measurements, each with its verdict line

| | verdict | note |
|---|---|---|
| `107-` qalculate-qt's 4-of-11 | ⭐ `pass=9 fail=1` | the **fail is the finding**: arm B carries `dbus-launch` and changes nothing |
| `30-` gconv and locale | ⭐ `pass=24 fail=0 skip=0` | arm B ran **for the first time**; zero skips now remain in the whole tree |
| `102-` classifier equivalence | ⭐ `pass=20 fail=0 skip=0` | R1 fires for the first time (**C50**) |
| `70-`, `83-` | ⭐ green | re-run rather than pinned after the `0\n0` sweep |
| `lib.sh --selftest` | ⭐ 14 pass 0 fail | three new rows for `exp_count` |
| `pgb selftest` | ⭐ 601 pass | with the C53 interposer embedded |
| `101-` rung 3 | ⛔ **re-run in flight** | its committed `RESULT.txt` is a truncated transcript |
| `108-` flameshot capture | ⛔ **pre-registered, never run** | the last *Untried* in the record |

## ⛔ THE MECHANISM BEHIND THE PATTERN, because a comment is not one

The `$(grep -c … || echo 0)` defect had been **diagnosed three times in
comments** (`79-`, `91-`, `95-`) and reintroduced in **five more files**. It
now has `exp_count` in `lib.sh` and a **gate** in `TODO/check.sh`. ⭐ That is
the shape every one of this session's instrument findings wants: not a note
beside the call site, but something that fails.
