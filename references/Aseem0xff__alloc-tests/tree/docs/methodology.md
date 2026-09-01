# Methodology

How a number here is produced, and what would have to be true for it to be
wrong. This page follows `docs/methodology/experiments.md`; the rules quoted as
⛔ are that document's, not this project's inventions.

---

## The three traps this project is built against

### ⛔ 1. The baseline trap

**ripgrep does not use the system allocator on musl.** At tag `15.2.0`
(`e89fff89ac9a`), `crates/core/main.rs` selects `tikv_jemallocator::Jemalloc`
whenever `target_env = "musl"` and the pointer width is 64.

So an unmodified `cargo build --target x86_64-unknown-linux-musl` produces a
**jemalloc** binary. Publishing that as "the Alpine system allocator baseline"
puts jemalloc in the control group, and every ratio in the report is then wrong
by whatever jemalloc beats musl by — the largest single effect this project
measures.

**Every cell, the baseline included,** has that block removed by
`alloc-runner patch-rg`, which then asserts the tree contains exactly 0 or
exactly 1 `#[global_allocator]` attributes. A text transform over somebody
else's source that reports success without checking is how a baseline silently
keeps its allocator.

### ⛔ 2. Measuring an allocator that is not there

Upstream `mimalloc-bench` issues 245 and 247 (both open, corpus at
`references/daanx__mimalloc-bench`) report that a missing allocator library is
undetected: the system allocator is measured and filed under another name.

**Identity is established by reading the artefact**, before any timing.
`alloc-runner identify` requires:

- the allocator's own symbols present; **and**
- no *other* candidate allocator present; **and**
- the link kind to match the profile (static / static-PIE / dynamic); **and**
- for a **replacement** build, the displaced libc allocator to be **absent**.

⚠ **A stripped binary is reported as UNPROVEN, not as clean.** No `.symtab`
means the instrument could not look, which is a different result from looking
and finding nothing. This is why the build sets `strip=none`.

⚠ **What symbol evidence can and cannot prove.** Finding an allocator's symbols
proves its code was linked in. It does not by itself prove every allocation
flows through it. The negative control — the displaced allocator being absent —
is the other half. Both together are strong; either alone is not.

### ⛔ 3. Absence read as zero

A workload whose runs all failed has no median. If that absence became a `0.0`,
it would sort as the fastest row in the table.

Every ratio in `rank.rs` is `Option` all the way down. A missing value renders
as `–`, the validator makes it an **error**, and a cell with any failed run is
not `ok`.

---

## Measuring from outside

⭐ **The subject is never asked how it did.**

`crates/alloc-runner/src/measure.rs` forks, `execve`s, and waits. Wall time is
`CLOCK_MONOTONIC` in the parent around the child's whole life. Peak RSS, user
and system time, and fault counts come from the kernel's `rusage`.

Three specifics that are load-bearing:

- **`wait4`, not `getrusage(RUSAGE_CHILDREN)`.** The latter reports the maximum
  over *all* reaped children, so after the first sample every later `ru_maxrss`
  is that running maximum rather than this run's. The series would be flat,
  plausible and wrong.
- **`CLOCK_MONOTONIC`, not `CLOCK_REALTIME`.** An NTP step mid-run must not
  become a measurement.
- **Output goes to `/dev/null`, not a pipe.** A pipe makes the parent a
  participant: a full buffer stalls the child and is recorded as slowness.

⚠ **Does the instrument perturb what it measures?** The parent polls `wait4`
every 200 µs. Against runs measured in tens of milliseconds that is well under
one part in a hundred, and it is the same for every cell, so it cannot reorder
them. The ASLR probe is more invasive — it reads `/proc/<pid>/maps` while the
child runs — which is why it runs **after** the timed workloads and its runs are
not timing samples.

## The corpus

Generated from `(seed, profile)` by `alloc-runner gen-corpus`. Byte-identical on
every host, so two machines are compared on the same work and a corpus
difference can never be mistaken for an allocator difference.

⭐ **The generator plants every match, so it knows the answers before ripgrep
runs.** That is an oracle independent of the thing being tested. The correctness
gate asserts *exact* counts — matching lines and matching files — not "it printed
something".

⚠ At most one planted token per line, deliberately: ripgrep counts a line once
however many times a pattern hits it, so a line with two needles would break the
identity between "tokens planted" and "matching lines".

⛔ The searchable data lives in `<corpus>/data/`. The manifest that records the
patterns necessarily *contains* the needle strings; written beside the data,
ripgrep searches it too and every count comes back one too high. That was
observed on the first end-to-end run.

## The correctness gate

Nothing is timed until it passes. A configuration that crashes, or that finds
the wrong number of lines, has no interesting performance.

| check | what it would catch |
| --- | --- |
| `starts` | a binary that segfaults on `--version` |
| `literal` | exact matching lines **and** exact matching files |
| `icase` | a build that ignores `-i` — the expected count differs from the case-sensitive one on purpose |
| `unicode` | different UTF-8 handling |
| `regex` | a literal fast path taken where the engine should run |
| `nomatch` | zero matches **and exit 1**; the negative control |
| `threads` | `-j1` and `-j4` must list identical files — a race in a thread-caching allocator shows as a wrong count long before it shows as a crash |
| `repeat` | an intermittent fault; a control run once is a coincidence nobody has noticed yet |
| `json` | a different buffering path from the plain printer |

## Statistics, and what is deliberately absent

Median and a scaled median absolute deviation. Warm-up runs are **discarded**,
not averaged in: the first run over a 65 MB corpus pays for a cold page cache,
which is a filesystem measurement.

⛔ **There is no confidence interval, on purpose.** Repeated runs on a shared
runner are not independent draws from a stationary process; an interval computed
as though they were would be a precise-looking claim about nothing. The MAD says
what it says and no more.

⭐ **A lead smaller than the run's own MAD is reported as no result.** The report
prints "no winner is claimed here" and says why. Manufacturing an ordering out
of noise is the easiest way for a benchmark to be confidently wrong.

### ⛔ The MAD is a floor on the uncertainty, not a bound

⚠ **Measured here, and it is the most important caveat on this page.** The
`mechanisms` comparison was run twice on the same host, same corpus, same
sources. Every cell in both runs reported an internal relative MAD of 1.6–2.9%.
The `rust-global` cell — built identically both times — measured **0.501×** in
one run and **0.606×** in the other.

So a cell's own samples cluster tightly and the cell's *position* still moves
about twenty per cent between runs. Whatever causes it (a shared VM's
neighbours, page-cache layout, the scheduler) is common to a run and therefore
invisible to a within-run statistic.

⭐ **What follows from it, and what the project does:**

- **A direction that survives two runs is a finding. A magnitude from one run is
  not.** Both runs are published side by side rather than the newer one
  replacing the older, precisely so the disagreement is visible.
- The MAD-based refusal above is still worth having — it catches the *smallest*
  claims — but passing it is **not** sufficient to publish a magnitude.
- ⛔ It would have been easy to publish "13.1% faster, outside the run's 2.7%
  spread" and be wrong about the number while right about the ordering. That is
  what running the control twice is for.

## Holding everything else still

⚠ Comparisons are only made within a **group**: the same distribution, the same
architecture, the same build profile, the same toolchain. No ratio crosses a
group.

Constant across every cell:

- one pinned Rust toolchain (`toolchains/pins.env`), not the distribution's;
- one pinned corpus, shared through the cache so every cell reads the same bytes
  from the same place;
- identical Cargo profile keys, set by environment rather than by editing
  ripgrep's `Cargo.toml` — ripgrep's own `release-lto` profile differs from
  `release` in **four** ways at once (`lto`, `panic`, `strip`, `debug`) and would
  confound the LTO comparison;
- `-Wl,-z,stack-size=8388608` on every static cell including the baseline, so it
  is a constant rather than an advantage given to one row;
- allocator-internal LTO **off** for every allocator, so that dimension is
  constant. LTO is measured as an *application* profile instead.

Recorded per cell rather than held constant, because they are the variables:
allocator, mechanism, build profile, distribution, architecture, toolchain.

### The distribution axis needs a control

A fully static binary built with the distribution's own gcc embeds that
distribution's libc and toolchain. Comparing three distributions therefore
compares three gccs as much as two libcs.

The `toolchain-control` suite rebuilds the same cells with one pinned `zig cc`,
identical everywhere. ⚠ **A difference that survives that control is
attributable to the distribution. A difference that disappears was the
compiler**, and the `distros` table has to be read with that in mind.

## Emulation

⛔ **Emulated timings are recorded and excluded from ranking.** User-mode
emulation changes the instruction mix and the memory behaviour, so an allocator
comparison under it measures the emulator too. `alloc-bench run` skips
non-native cells unless `--allow-emulation` is passed, and the report marks the
run.

## What an experiment here cannot tell you

- **That it generalises.** One machine on one day is one machine on one day. The
  report prints the machine in the same section as the numbers.
- **That the allocator is why.** Where a difference is smaller than the spread,
  it says so.
- **That ripgrep predicts your workload.** ripgrep is not allocation-bound; an
  allocation-heavy service would rank differently.
- **That an absence is a zero.** A configuration that could not be built is
  published as unsupported with its reason, never as a missing row.

## Hardening is not a defect

hardened_malloc trades performance for slab canaries, guard slabs, quarantines,
slot randomisation and zero-on-free. Reporting it as "slow" without naming what
it bought would be the wrong reading, and the report says so beside the table.

⚠ Equally, this project makes **no claim** that those mitigations work. It
measures performance. Security properties are not established here.
