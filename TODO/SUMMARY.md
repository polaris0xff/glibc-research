# SUMMARY.md — the session of 2026-09-03e

⛔ **Overwritten every session.** The work order is
[`PROGRESS.md`](PROGRESS.md); the closed entries are
[`../HISTORY/entries/`](../HISTORY/entries/).

    SCOPE     three entries and nothing else, set by the operator on
              2026-09-03d: T-078 the three-way parity matrix, T-079
              enumerate the remaining glibc-static edge cases BY SEARCH,
              T-080 the nix-bundle capability guarantee.
              ⭐ MID-SESSION the operator added two things: a correction
              to T-080's success criterion, and permission to take on
              T-081 because it turned out to be the blocker.
    RESULT    ⭐ ALL THREE CLOSED. Two of them came out AGAINST US, which
              is the outcome each entry's Prove line said to report
              rather than soften.

## ⭐ What moved

| | before | after |
|---|---|---|
| `musl-gcc` | ⛔ **absent** — the blocker the last session recorded | ✅ installed; `experiments/61-` arm A and `63-` arm M now RUN instead of skipping |
| the parity matrix | evidence spread over ten experiments, musl column mostly **inferred** | ⭐ one table, every cell run, `skip=0` |
| the glibc-static quirk list | 10 found, 9 closed | ⛔ **11 found, 9 closed** — `/etc/services` |
| GTK out of a nix closure | a hypothesis, graded *"Garbage"* by the field on a different pipeline | ⭐ **11 of 11 real windows on a real X server**, zero host objects |
| T-081's cost | a plan | ⭐ a measurement with a positive control |

## ⛔ The two rows that came out against us

⭐ **Both are in the shipped table. Neither axis was softened until it passed.**

1. **The environment-default codeset.** With no `LANG` set, native musl static
   answers **UTF-8 on 11 of 11** and every glibc arm — `pgb` included —
   answers `ANSI_X3.4-1968` on **11 of 11**. ⛔ **The prediction registered
   before the run said the opposite on both halves.** musl's minimal locale
   support does not mean a poor codeset: its default charset *is* UTF-8.
   Asked for `C.UTF-8` **by name**, `pgb` answers UTF-8 on 11 of 11 against
   vanilla's 7 — but that is a different question, and it is now a separate row.
2. **`/etc/services`** — the eleventh host-data dependency. All three columns
   fail it on the same three environments, so it is not a row `pgb` loses *to
   musl*; it is one nobody wins and `pgb` claims it should.

## ⭐ The operator corrected the instrument, and it cost eleven green rows

⛔ **`experiments/64-` first scored GTK 11 of 11 GREEN.** Its criterion was
that the program printed `Gtk-WARNING **: cannot open display:`, on the
reasoning that the message is emitted by the *bundled* libgtk-3 and therefore
proves it loaded.

> *"you may be measuring the wrong success criteria. Previously nixappimage
> bundled apps showed the same error on real hw with display, confirm it
> properly by feeding it a fake/emulated display"*

⭐ **Right, and decisive.** The message does not discriminate: it is identical
when there is no display and when the bundle's own X stack is broken, which is
the only distinction the experiment exists to make. Feeding it a real display
(`Xvfb`, socket bound into each rootfs) and asking the **X server** for a
window — from outside the process — turned **11 green rows into 0**.

⭐ **A second subject then separated the causes**, and that pair is the whole
T-080 deliverable:

| arm | subject | window on a real X server |
|---|---|---|
| G | `galculator` — UI is a **file** at a compiled-in store path | ⛔ **0 / 11** |
| X | `mousepad` — UI is a **GResource compiled into the binary** | ✅ **11 / 11** |
| ⭐ C | `galculator` **again**, with that store path made to resolve | ✅ **11 / 11** |

⭐ **Arm C is the argument**: identical artefact, one variable changed, and it
draws. So *"a hardcoded store path is what stops it"* is a **measurement**, not
a reading of an error message — which is what licenses the guarantee's
sentence, *the remaining gap is tooling, not capability*.

## The five defects this session found in its own instruments

⭐ **Every one was found by something disagreeing, none by reading.**

| what | how it was caught |
|---|---|
| `[ -e "$rootfs$path" ]` resolves an **absolute symlink against the HOST**, so Alpine's `/bin/sh -> /bin/busybox` read as absent on three rows | the file listing disagreed with a binary that ran fine |
| the probe **buffered stdout to a pipe**, so a crash discarded every answer already computed and a row read "no output" | Arch printed nothing while its neighbours printed everything |
| the crash counter read the **parent's** exit status, and the per-axis fork is what makes a crash survivable — it reported `crashed = 0` on a run whose rows read `SIG8` | the summary disagreed with the rows above it |
| the UTF-8 counter **globbed the whole line**, so adding a second axis containing `UTF-8` moved the glibc arms 0 → 7 between two runs | the two-runs rule |
| `tr -d '\r' < f1 f2` redirects `f1` and passes `f2` as an argument, so every row printed `<none>` | the trace beside it plainly showed GTK loading |

⚠ **And two in the write-up itself**, caught by checking it against the
evidence: `comparison.md` said vanilla iconv gives *"SIGABRT on 3"* when it is
SIGABRT on two and SIGFPE on one; and a throughput figure was quoted from a
first run whose `RESULT.txt` the second had already overwritten.

## What is NOT claimed

⛔ **Said in the sentence, not in a footnote**, because overclaiming is the
failure T-080 names first.

- **Vulkan and NVIDIA are not measured.** Every GL row in this tree is
  `swrast` and surfaceless. The supported sentence is *"the closure produces a
  working EGL display offscreen"*. **T-059** owns real hardware.
- **SDL was never run through `pgb bundle appimage`** and stays a hypothesis.
- **No Python GUI application bundles at all**, so the operator's own
  counter-example is still unreached — `resolveEntry` oscillates on the
  standard nixpkgs wrapper shape.
- **GTK is proved on one subject**, not on GTK in general.
- **The eleventh quirk has no mechanism yet**, only a measurement.

## ⭐ Next

⛔ **T-081, and its acceptance test already exists**: `experiments/64-` arm G
must go **0 of 11 → 11 of 11** without the bind arm C uses.
