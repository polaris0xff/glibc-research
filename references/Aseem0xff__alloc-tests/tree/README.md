# alloc-tests

**Does replacing the allocator in a statically linked container binary make it
materially faster, and what does that cost?**

This is a reproducible benchmark that answers that question with evidence rather
than folklore. The application under test is [ripgrep]; the environment is
Docker/Podman images built from `alpine:latest`, `debian:latest` and
`archlinux:latest`, on x86_64 and aarch64.

Clone it, install Docker or Podman, and run one command. Everything else —
allocator sources, the corpus, the toolchain — is fetched at a pinned revision
and recorded with the result.

```sh
cargo build --release -p alloc-bench
./target/release/alloc-bench doctor
./target/release/alloc-bench run --suite core --arch x86_64
```

---

## The result, so far

⛔ **The ranking depends on the machine.** `core` has been measured on three
CPUs and produced three different orderings — with *tight* noise on each. This
is the project's most important finding to date and it comes before the table.
`alpine / x86_64 / static-pie-lto`, workload `literal`, against musl's own
allocator:

| allocator | dev sandbox<br>Xeon @ 2.80GHz | dev sandbox<br>Xeon @ 2.10GHz | **GitHub runner**<br>EPYC 7763 |
| --- | --- | --- | --- |
| snmalloc | **0.595×** | **0.553×** | **1.138×** |
| mimalloc | 0.660× | 0.577× | 1.004× |
| jemalloc | 0.762× | 0.592× | **0.878×** |
| rpmalloc | 0.754× | 0.616× | 0.879× |
| **system (control)** | 1.000× | 1.000× | 1.000× |
| hardened_malloc | 1.075× | 0.890× | 1.225× |
| *within-run MAD* | *2.0–8.4%* | *2.0–6.4%* | ***0.5–1.4%*** |

⛔ **snmalloc is the fastest allocator measured on one machine and the second
slowest on another.** On the GitHub runner musl's own allocator beats snmalloc
outright (1.138×, well outside that run's 1.3% MAD) and mimalloc lands level
with it (1.004× — a tie, not a loss). ⚠ And the runner is the *quietest* of the
three, so this is not noise hiding a signal: each machine gives a tight,
self-consistent, different answer.

**What survives all three runs:**

- **jemalloc and rpmalloc beat musl's allocator on every machine measured** —
  by ~12% on the runner, ~25–40% on the sandbox. That is the only ordering
  claim this project currently supports.
- **Speed is not free**, and the memory cost also moves: mimalloc is 3.4× the
  control's peak RSS on the sandbox, and rpmalloc — the cheapest there at 1.14×
  — is **8.0×** on the runner.

**What it does not say:**

- **Which allocator is fastest.** It depends on the machine, and this project
  cannot yet tell you which property of the machine decides it.
- **That any magnitude here is reliable.** None survived repetition. Quote a
  direction, and name the machine.
- **Anything about your application.** ripgrep is not allocation-bound. An
  allocation-heavy service would rank differently.
- **That hardened_malloc is bad.** It spends performance on slab canaries, guard
  slabs, quarantines and zero-on-free. That is the product, not a defect. This
  project makes no claim about whether those mitigations work.

Datasets, all committed:
[sandbox A](results/published/2026-09-01-core-x86_64/) ·
[sandbox B](results/published/2026-09-01-core-x86_64-run-b/) ·
[GitHub runner](results/published/2026-09-01-core-x86_64-ci-runner/). Conditions:
Alpine musl 1.2.6, rustc 1.96.0, gcc 15.2.0, 8 samples on the sandbox and 12 in
CI. See [`docs/AGENTS.md` §11.1](docs/AGENTS.md).

### The mechanism changes the answer

Same allocator (mimalloc), same profile, reached three ways — measured three
times ([A and B](results/published/2026-09-01-mechanisms-x86_64/),
[C](results/published/2026-09-01-mechanisms-x86_64-run-c/)):

| mechanism | run A | run B | run C | outcome |
| --- | --- | --- | --- | --- |
| `libc-surgery` — rewrite `libc.a` | **0.444×** | **0.460×** | **0.523×** | ok |
| `rust-global` — `#[global_allocator]` | 0.501× | 0.606× | 0.597× | ok |
| `link-override` — archive ahead of libc | – | – | – | ⛔ fails to link |

⭐ Replacing `malloc` inside `libc.a` beat the `#[global_allocator]` in **all
three** runs — the shim catches Rust's allocations, the surgery catches
everything the process allocates, musl's own internal ones included. Peak RSS is
identical between them (3.465× vs 3.470×), as you would expect if both run the
same allocator and differ only in how much of the program reaches it.
**This is the only comparison in the project that has survived repetition**, and
it survived a change of CPU.

⚠ **The direction is robust; the magnitude is not** — 13%, 32%, 12%, against
internal MADs of 1.6–4.3%. Treat the MAD as a floor on the uncertainty, not a
bound. All three runs are published so the disagreement is checkable.

⛔ **But `libc-surgery` only works for mimalloc.** Run C was the first to try
every allocator, and jemalloc, snmalloc and hardened_malloc all fail to link:
deleting musl's malloc members leaves musl's own `dlerror.c` still referencing
`__libc_malloc` / `__libc_free` / `__libc_calloc`, and **mimalloc happens to
define those aliases while the others do not**. The technique is, as
implemented, mimalloc-specific.

The naive `link-override` fails for every allocator with `multiple definition of
__libc_malloc / calloc / free …` — which is precisely why the surgery *deletes*
the displaced members instead of merely linking ahead of libc.

---

## What makes this different from other allocator benchmarks

### It checks that the allocator is actually there

Upstream `mimalloc-bench` issues [245] and [247] — both still open — report that
it cannot detect a missing allocator library: the system allocator gets measured
and published under the other allocator's name, and the run is green.

Here, **identity is established by reading the ELF before anything is timed**.
The binary must contain the allocator's own symbols, and for a *replacement*
build the displaced libc allocator must be **absent**. A cell that fails is
never ranked.

### It does not put jemalloc in the control group

ripgrep selects `tikv-jemallocator` on musl. An unmodified
`cargo build --target x86_64-unknown-linux-musl` is therefore a **jemalloc**
binary, and calling it "the Alpine system allocator" would make every ratio
wrong. Every cell here — the baseline included — has that block stripped, and
the result is checked against the binary.

### It distinguishes four different things people call "using an allocator"

| mechanism | what it actually does |
| --- | --- |
| `rust-global` | `#[global_allocator]` → the allocator's prefixed API. libc's `malloc` untouched. |
| `libc-surgery` | rewrites **`libc.a` itself**, so every static binary built in the image gets it with **no build flags** |
| `link-override` | archive ahead of libc on the link line — the naive approach |
| `preload` | `LD_PRELOAD` into a dynamic binary |

"Linking an allocator into your app" and "replacing your image's allocator" are
different experiments, and they get different tables.

### A configuration that cannot exist is published, not dropped

Mesh and Google tcmalloc export no prefixed C API, so they cannot be used
through `#[global_allocator]`. That appears in the report as a row with the
technical reason, not as an absence. If a future upstream adds one, the row
becomes a measurement without anyone editing prose.

---

## How a number is defended

- **Corpus** — generated deterministically from a seed, byte-identical on every
  host. The generator *plants* each match, so it knows the right answers before
  ripgrep runs. The correctness gate asserts exact counts.
- **Correctness before speed** — every binary must find the exact expected
  number of matching lines and files, agree with itself across thread counts,
  handle non-ASCII, return exit 1 on no match, and produce identical results
  twice. A broken binary is never timed.
- **Measured from outside** — `fork`/`execve`/`wait4`. Wall time from
  `CLOCK_MONOTONIC` in the parent, peak RSS from the kernel's `rusage`. The
  subject never reports on itself.
- **Noise is stated** — median with a scaled MAD. A lead smaller than the run's
  own spread is reported as **no result**.
- **Missing is never zero** — a table cell with no measurement prints `–`. The
  validator makes an absent value an error rather than an implicit zero.
- **ASLR is observed** — read from `/proc/<pid>/maps` while the binary runs, not
  inferred from the ELF type.

---

## Documentation

| | |
| --- | --- |
| [`docs/AGENTS.md`](docs/AGENTS.md) | **Start here.** The complete handoff document: status, what works, what is untested, what to do next. |
| [`docs/methodology.md`](docs/methodology.md) | How measurements are taken and defended |
| [`docs/allocator-integration.md`](docs/allocator-integration.md) | Per-allocator build and integration recipes |
| [`docs/static-linking.md`](docs/static-linking.md) | Static, static-PIE, LTO, ASLR: flags and verification |
| [`docs/containers.md`](docs/containers.md) | Docker and Podman workflow, rebuilding images |
| [`docs/reproducing.md`](docs/reproducing.md) | Reproducing a published result exactly |
| [`docs/extending.md`](docs/extending.md) | Adding an allocator, application, distribution or suite |
| [`docs/results.md`](docs/results.md) | Where results live and how to read a report |
| [`docs/ci.md`](docs/ci.md) | What GitHub Actions runs |
| [`docs/troubleshooting.md`](docs/troubleshooting.md) | Failures seen and what they mean |
| [`experiments/`](experiments/) | Numbered measurement scripts and their committed output |
| [`references/`](references/) | The mined corpus: prior art and allocator trackers |

## Status

⚠ **This project is young and its status board is honest.** The pipeline has
been reproduced on three machines, and `core` and `mechanisms` have both been
run more than once. But of 122 planned cells **13 have been measured**, all of
them x86_64 on Alpine, and the honest summary is that the project has found more
limits than results:

- ⛔ **The ranking does not transfer between machines** (the table above). This
  is the open problem, and it is not a noise problem.
- ⛔ **`static-pie` does not exist on aarch64 musl**, so `core` cannot run
  there. Attempted 2026-09-01: all six cells built and all six were correctly
  rejected by the identity gate. **No aarch64 cell has been measured.**
- ⛔ **`libc-surgery` works for mimalloc and no other allocator** — the other
  four fail on musl's internal `__libc_*` aliases, which mimalloc happens to
  define and they do not.
- ⛔ **`preload` is not implemented**, so Mesh and Google tcmalloc have never
  been measured at all.

See [`docs/AGENTS.md` §13](docs/AGENTS.md#13-status-board) for exactly what has
been verified, what is written but unrun, and what is known broken.

## Licence

MIT, see [LICENSE](LICENSE). Allocator and application sources are fetched at
build time and are not redistributed here.

[ripgrep]: https://github.com/BurntSushi/ripgrep
[245]: https://github.com/daanx/mimalloc-bench/issues/245
[247]: https://github.com/daanx/mimalloc-bench/pull/247
