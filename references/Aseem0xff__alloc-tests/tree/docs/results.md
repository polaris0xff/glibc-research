# Results: where they live and how to read them

---

## The shape of a dataset

Every run — local or CI — writes the same tree:

| path | what it is |
| --- | --- |
| `run.json` | **the conditions**: host, CPU, kernel, memory, container runtime, CI identifiers, the corpus seed, the repository commit, and the whole lock file |
| `plan.json` | every cell, **including the unsupported ones and their reasons** |
| `results/<cell>.json` | one document per cell: identity evidence, correctness checks, ASLR observation, build metadata, and every raw sample |
| `cells/<cell>/` | the artefacts the container wrote: logs, `build.json`, `measure-*.json` |
| `logs/` | the container's own output, per cell and per image |
| `report.md` | the human-readable report |
| `rankings.json` | the same, machine-readable |
| `*.svg` | the graphs |

⭐ **Every number in `report.md` is derived from `results/`. None was typed in,
and no graph was drawn by hand.** `alloc-bench report --run DIR` regenerates all
of it, so a stale graph is impossible rather than merely discouraged.

## Where results are published

| | |
| --- | --- |
| **CI artefacts** | every `bench.yml` run, 90 days. The full dataset. |
| **`results/published/`** | curated snapshots worth citing, committed |
| **`results/local/`** | your own runs. Gitignored. |

Committing every dataset would bloat the repository for numbers that are only
meaningful alongside their conditions, so only snapshots that a document cites
are committed.

## Reading `report.md`

It is deliberately ordered so that the caveats cannot be skipped.

### 1. "What this run does not establish" — first, not in an appendix

The machine, the cells that could not exist, the cells that failed, whether the
run was emulated, and how many cells were too noisy to carry a fine comparison.
⭐ A reader who reaches the recommendation first has already stopped reading, so
the limits come before the table.

### 2. Conditions, and exactly what produced the numbers

Host and runtime, then a table of every pinned component with its tag and
**commit**, linked. This is the answer to "what source and environment produced
this number?".

### 3. Rankings

One table per **group** — a group is one distribution, architecture, profile and
toolchain. ⚠ **No ratio crosses a group**, because two cells that differ in more
than the allocator are not comparable.

| column | meaning |
| --- | --- |
| `time (s)` | median wall clock over the run's samples, warm-ups discarded |
| `rel` | that time ÷ the control's, in the same group. **1.000 is the image's own allocator.** |
| `MAD` | relative median absolute deviation of *that cell's* samples |
| `peak RSS` | `ru_maxrss` from the kernel — what sets a container's memory limit, not the allocator's idea of heap size |
| `size` | ⚠ **unstripped**, with symbols kept so the identity oracle can read the binary. A shipped build strips and is smaller. |
| `startup` | the `startup` workload: process start plus allocator initialisation |
| `composite` | see below |

⛔ **`–` means the value is unknown.** It is never a zero.

Under each table is either a winner **or** a refusal:

> ⚠ **No winner is claimed here.** The lead of 3.1% is within this run's own
> spread (8.4% relative MAD), so the ordering between the top rows is not
> established here.

⭐ A lead smaller than the run's own noise is not a lead. Reporting it as one is
how a benchmark manufactures a winner.

### 4. Every workload

The ranking uses one workload. This table shows all of them, so a reader can see
whether the ordering **holds** or is an artefact of one shape of work.

### 5. ASLR, observed

Distinct load addresses out of N samples, read from `/proc/<pid>/maps` while the
binary ran — not inferred from the ELF type.

### 6. Configurations that could not exist

⭐ **These are results.** Each names a concrete technical reason. If a future
upstream release removes the reason, the row becomes a measurement without
anyone editing prose.

### 7. Configurations that failed, and 8. Validation

Every finding with its severity. ⛔ **An `ERROR` means the ranking above must not
be trusted**, and `alloc-bench` exits non-zero to say so.

## The composite score

```
composite = 0.60 × (time / baseline_time)
          + 0.30 × (peak_rss / baseline_peak_rss)
          + 0.10 × (binary_bytes / baseline_binary_bytes)
```

Lower is better; 1.000 is the control.

**Rationale, so it is arguable rather than arbitrary:** for a container binary
the dominant cost is how long the command takes, so time carries most of the
weight. Peak RSS is next, because it sets the container's memory limit and
therefore what it costs to run. Binary size is last and small: it is paid once
at pull time, not per invocation.

⛔ **These weights are a choice, and it is the wrong one for some readers.** A
memory-capped deployment should weigh RSS far higher — and would then prefer
rpmalloc over mimalloc in the published run, reversing the order. **Every
component is published separately** in the same table and in `rankings.json`, so
re-ranking needs no re-run.

⚠ The composite is a **secondary** column. The primary ranking is execution
time, which is the project's stated practical goal.

## Published runs

### `2026-09-01-core-x86_64`

Six allocators on Alpine musl, `static-pie-lto`, x86_64, 8 samples.
Headline: **snmalloc 0.595×, mimalloc 0.660×, rpmalloc 0.754×, jemalloc 0.762×,
hardened_malloc 1.075×** against musl's own allocator.

⚠ The top four are separated by less than the noisiest of them; the ordering
among *them* is not established. That musl is much slower than all of them is.

### `2026-09-01-mechanisms-x86_64`

The same allocator (mimalloc) reached three different ways, `static-pie`, x86_64.

| mechanism | run A | run B | outcome |
| --- | --- | --- | --- |
| `libc-surgery` | 0.444× | **0.460×** | ok |
| `rust-global` | 0.501× | **0.606×** | ok |
| `link-override` | – | – | ⛔ **fails to link** |

⭐ **The mechanism changes the answer.** Replacing `malloc` inside `libc.a` beat
the `#[global_allocator]` in both runs, by a margin far outside either run's
internal spread. That is consistent with the mechanisms doing different things:
the shim catches Rust's allocations, the surgery catches everything the process
allocates, musl's own internal allocations included.

⚠ Peak RSS is essentially identical between them (3.466× vs 3.471×), which is
what you would expect if both are running the same allocator and differ only in
how much of the program reaches it.

⛔ **The magnitude is NOT established.** 13% and 32% are the same comparison run
twice, while every cell reported a 1.6–2.9% MAD. **Within-run spread understates
between-run variability on this host.** Both runs are published so the
disagreement can be checked; see that directory's README for why the surgery
change between them does not explain it.

`link-override` fails with `multiple definition of __libc_malloc / calloc /
free …`. Evidence in that run's `evidence/` directory. ⭐ **That failure is why
the surgery deletes the displaced members** rather than merely linking ahead of
libc.
