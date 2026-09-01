# GitHub Actions

⚠ **Status: neither workflow has ever executed.** The push to this repository is
refused (see `docs/AGENTS.md` §13), so everything below describes what the
workflow files say, not what has been observed. Treat it as a specification
until a run exists.

---

## `ci.yml` — every push and pull request

Three jobs, minutes not hours.

### `instrument` — the tools check themselves first

| step | why it is not optional |
| --- | --- |
| `cargo test -p rgalloc-shim` | the shim decides the alignment of every allocation in every benchmarked binary. A bug here surfaces as an *allocator* crashing and gets written down as that allocator's fault. |
| `alloc-runner selftest` | 9 checks proving the instrument sees a wrong exit code, a `SIGSEGV`, and a timeout **as what they are** — before it is trusted to report a number. It also proves the corpus generator is deterministic and seed-sensitive. |
| `mine-repo.sh --selftest` | the vendored fetcher's own offline check, so a broken re-vendor is caught here rather than discovered mid-sweep |
| `cargo fmt` + `clippy -D warnings` | |

### `configuration` — the config must be coherent

- every suite expands, and cell ids are unique;
- ⛔ **no unsupported cell may lack a reason.** The job fails on one. An
  unexplained absence is the exact failure this project is built against;
- the lock file covers every allocator in the manifest and every commit is a
  full 40-character SHA;
- `sh -n` and `shellcheck -S warning` over every shell script.

### `smoke` — the whole pipeline, end to end

Runs the `smoke` suite with `--strict`: image build, allocator build, ripgrep
patch and build, identity, correctness gate, ASLR probe, measurement,
validation, report. On x86_64 only, on the smallest corpus.

⚠ **The smoke suite proves the pipeline works. It ranks nothing.** Its corpus is
small enough that a search takes milliseconds, so the noise swamps any real
difference. The job summary says so.

⭐ **Evidence is uploaded whether the run passed or failed.** A failed run's logs
and identity documents are exactly what makes the failure diagnosable; dropping
them turns a CI failure into a mystery.

## `bench.yml` — the benchmark

Weekly (`17 3 * * 1`) and on `workflow_dispatch` with `suite`, `repeat` and
`arches` inputs.

### Both architectures, both native

| architecture | runner |
| --- | --- |
| x86_64 | `ubuntu-24.04` |
| aarch64 | `ubuntu-24.04-arm` |

⛔ **Each architecture runs on its own native runner, and neither is emulated.**
User-mode emulation changes the instruction mix and the memory behaviour, so an
allocator comparison under it measures the emulator too. `alloc-bench` refuses
to rank an emulated run and this workflow never asks it to.

⚠ **If `ubuntu-24.04-arm` is unavailable** to a fork or a private repository, the
aarch64 job **fails to schedule** rather than silently running on x86_64 and
being labelled aarch64. That is the intended behaviour. To run aarch64 locally
under emulation instead — accepting that the result is excluded from ranking:

```sh
docker run --privileged --rm tonistiigi/binfmt --install arm64
alloc-bench run --suite core --arch aarch64 --allow-emulation
```

### The steps that matter

1. **Record the conditions** — `uname`, `lscpu`, memory, disk, docker version,
   and the cpufreq governor if the runner exposes one — **before** anything is
   measured, so a number cannot lose them.
2. **Free disk** — several images plus eight source trees plus a 65 MB corpus
   does not fit beside the runner's preinstalled toolchains. A run that dies on
   `ENOSPC` half way leaves a dataset that looks *partial* rather than *failed*.
3. **Run** — ⚠ deliberately **without `--strict`**. A cell that fails to build is
   a result this project publishes, not a reason to discard the run. The dataset
   validator decides whether the numbers are trustworthy.
4. **Re-validate as a separate step**, so the gate is visible on its own and so
   an artefact can be re-checked later without re-running anything.
5. **Upload the dataset** — 90 days, on success and on failure.
6. **Job summary** — leads with *what the run does not establish*, then the
   rankings, with the unsupported configurations behind a `<details>`.

### `combine` job

Downloads both architectures' artefacts and runs `scripts/report/combine.py`,
which answers the question neither single artefact can: ⭐ **does the ordering
hold across architectures?** An ordering that reverses between x86_64 and
aarch64 is the most interesting thing a two-architecture run can find, and
nobody sees it if the results stay in separate artefacts.

## Where results go

**GitHub Actions artefacts**, not the repository. Committing a full dataset on
every run would bloat the repository for numbers that are only meaningful
alongside their conditions.

Curated snapshots are committed under `results/published/<date>-<suite>-<arch>/`
when a run is worth citing. Those carry `run.json`, `plan.json`, the per-cell
result documents, the report and the graphs.

## Updating allocator versions

⚠ **There is no scheduled workflow for this**, deliberately: an automated
version bump that nobody reads is a change to what every future result means.

```sh
alloc-bench update --write
git diff allocators/allocators.lock.json
```

The diff is the reviewable artefact:

```
mimalloc   release v3.5.0 -> 18b08671c930
mesh       branch  master -> 2987f883b869
```

CI checks that the lock covers the manifest and that every commit is a full SHA,
so a hand-edited or truncated pin fails.

## Secrets

**None.** No workflow here needs a token beyond the default `GITHUB_TOKEN`, and
both declare `permissions: contents: read`. Allocator sources are public and
fetched over HTTPS.
