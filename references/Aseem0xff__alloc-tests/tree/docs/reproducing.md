# Reproducing a result

⚠ **Two different things are called "reproducing", and this project separates
them.**

| | |
| --- | --- |
| **Reproducible** — and this project aims for it | the same sources, the same flags, the same corpus, the same checks, the same *shape* of result. Byte-identical inputs, verifiable outputs. |
| **Identical timings** — and this project does **not** claim it | the same numbers on your hardware. ⛔ A benchmark that claims this is claiming something about your CPU, your kernel, your storage and your neighbours that it cannot know. |

What travels between machines is the **ratio within a group** — an allocator
against the control in the same image, architecture, profile and toolchain — and
even that is a measurement, not a guarantee.

---

## From nothing to a result

```sh
git clone https://github.com/Aseem0xff/alloc-tests && cd alloc-tests
cargo build --release -p alloc-bench
./target/release/alloc-bench doctor
./target/release/alloc-bench run --suite core --arch x86_64
```

Prerequisites: Docker **or** Podman, `git`, `curl`, and a Rust toolchain for the
orchestrator. Nothing else. `doctor` checks all of it and the disk headroom, and
exits 2 rather than starting a run that will die half way.

Output lands in `results/local/<run-id>/`:

```
run.json          the conditions: host, CPU, kernel, runtime, CI ids, the lock
plan.json         every cell, including the unsupported ones and their reasons
results/*.json    one document per cell: identity, correctness, ASLR, samples
cells/<id>/       the raw artefacts: the binary's metadata, logs, measurements
logs/             the container's own output, per cell and per image
report.md         tables, rankings, and what the run did NOT establish
rankings.json     the same, machine-readable
*.svg             the graphs, generated from the dataset
```

## Reproducing a *published* result

Every published run carries the exact inputs. From
`results/published/<run>/run.json`:

- `lock` — the repository URL, tag and **commit** for every allocator and for
  ripgrep;
- `corpus_seed` — the corpus is a pure function of `(seed, profile)`;
- `git_commit` — the repository revision that ran it;
- `host` — the machine, so you know what you are comparing against.

To rebuild the same inputs, check out that `git_commit` and run the same suite:
the lock file at that revision pins every source.

```sh
git checkout <git_commit>
./target/release/alloc-bench run --suite core --arch x86_64 --seed <corpus_seed>
```

Your absolute times will differ. Compare the `rel` column.

## Reproducing one cell

⭐ **The unit of reproduction is one `docker run` of one script.** No CI-only
step exists. See `docs/containers.md` for the full command; the commits come
from `allocators/allocators.lock.json`.

## Checking a dataset you did not produce

```sh
alloc-bench validate --run path/to/dataset
```

It re-checks the whole dataset against the plan: every planned cell present,
every `ok` cell carrying identity and correctness evidence, no workload with a
failed run, no missing median silently read as zero, one corpus digest across
the run, and a control for every ratio.

⛔ **An `ERROR` means the ranking must not be trusted**, and the exit status says
so. The report is still written — the evidence of a broken run is the point.

```sh
alloc-bench report --run path/to/dataset
```

regenerates the tables and graphs from the raw samples. ⛔ **No graph in this
repository is drawn by hand or edited afterwards**; each is a pure function of
the dataset it names, so a stale graph is impossible rather than merely
discouraged.

## Why "latest" is still reproducible

The project is supposed to benchmark current upstream, not versions frozen years
ago. It is also supposed to be reproducible. Those are only compatible if
"latest" is resolved **at a known moment** and the answer is **committed**.

```sh
alloc-bench update --write
```

resolves each allocator's newest release (or branch head, where upstream
publishes no release) to an exact commit and writes
`allocators/allocators.lock.json`. That file is a normal reviewable diff:

```
mimalloc   release v3.5.0 -> 18b08671c930
mesh       branch  master -> 2987f883b869
```

Everything downstream reads the lock and **never asks a network what "latest"
means**. A run against a lock file that differs from the committed one is a
different experiment, and CI checks that the lock covers the manifest and that
every commit is a full SHA.

⚠ Two upstreams (Mesh, Google tcmalloc) publish no releases or tags at all, so
they are tracked by branch head. The lock records `kind: "branch"` for them, so
a reader can see that their pin is a moving target that was frozen, not a
release.

## What would make your numbers not comparable to a published run

Named explicitly, because these are the ways a reproduction quietly stops being
one:

- **A different corpus seed or profile.** The digest is recorded; the validator
  errors if cells within one profile disagree on the match-set digest.
- **A different lock file.** Different allocator commits are a different
  experiment.
- **Emulation.** `--allow-emulation` runs foreign-architecture cells under
  binfmt. They are recorded and **excluded from ranking**; the report marks the
  run.
- **A busy machine.** The report prints each cell's relative MAD. A run with a
  large spread cannot carry a fine comparison, and the ranking refuses to name a
  winner inside its own noise.
- **A modified recipe with a warm cache.** The allocator cache key covers
  allocator, commit, mode, PIC, libc, architecture, toolchain and variant — not
  the recipe's contents. Edit a recipe, delete `.cache/<distro>-<arch>`.
