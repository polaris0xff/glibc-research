# `core` on alpine/x86_64 — run B

The `core` suite measured a second time on the dev sandbox, from an image built
at `HEAD` = `194ed63`. Run A is the sibling directory
[`2026-09-01-core-x86_64/`](../2026-09-01-core-x86_64/); a third measurement, on
a GitHub-hosted runner, is in
[`2026-09-01-core-x86_64-ci-runner/`](../2026-09-01-core-x86_64-ci-runner/).

⭐ **This directory exists because run B disagrees with run A.** The newer
numbers do not replace the older ones; both are published, for the same reason
the `mechanisms` directory publishes two runs.

| | run A | run B |
| --- | --- | --- |
| run id | `20260901-035224` | `20260901-110231` |
| started | 03:52 UTC | 11:02 UTC |
| image | built before the clippy/dead-code cleanup (`f7a46b5`) | built at `194ed63` |
| samples | 8 | 8 |
| kernel / cores / RAM | `6.18.44-fc-v22` / 4 / 16461068 kB | **identical** |
| **CPU** | `Intel(R) Xeon(R) Processor @ 2.80GHz` | **`Intel(R) Xeon(R) Processor @ 2.10GHz`** |

⛔ **These are not the same machine, and that is the first thing to know.** The
sandbox reports an identical kernel, core count and byte-exact RAM, so it looks
like one host — but `run.json` recorded a **different CPU model** for each run.
The VM was re-hosted between them. Anything below that is called "run-to-run
variation" is at least partly *machine-to-machine* variation, and the two cannot
be separated from these two runs alone.

⭐ This is precisely why `run.json` records `cpu_model`. Nothing in the two
reports would have revealed it; the conditions block did.

## What changed

Workload `literal`, relative to musl's own allocator in the same run:

| allocator | A | B | move | A MAD | B MAD | move exceeds both MADs? |
| --- | --- | --- | --- | --- | --- | --- |
| snmalloc | 0.595× | 0.553× | −7.0% | 8.4% | 4.6% | no |
| mimalloc | 0.660× | 0.577× | −12.6% | 2.0% | 3.5% | **yes** |
| jemalloc | 0.762× | 0.592× | −22.3% | 2.5% | 4.2% | **yes** |
| rpmalloc | 0.754× | 0.616× | −18.3% | 5.5% | 2.0% | **yes** |
| hardened_malloc | 1.075× | 0.890× | −17.2% | 3.3% | 6.4% | **yes** |
| system (control) | 1.000× | 1.000× | — | 2.9% | 4.1% | — |

```
order A:  snmalloc < mimalloc < rpmalloc < jemalloc < system < hardened_malloc
order B:  snmalloc < mimalloc < jemalloc < rpmalloc < hardened_malloc < system
```

⛔ **The ordering did not hold.** Two changes, and the second is qualitative:

1. `jemalloc` and `rpmalloc` swapped places.
2. **`hardened_malloc` crossed the control.** In run A it was *slower* than
   musl's allocator (1.075×); in run B it is *faster* (0.890×). A reader of run
   A alone would conclude hardened_malloc costs you performance against the
   baseline. Run B says the opposite.

## ⭐ It is not the build — that part was checked

"The newer image is different" is the first explanation to reach for, and it is
**wrong** here. Five of the six ripgrep binaries are **byte-identical** between
the pre-cleanup image and the `194ed63` image:

| allocator | run A bytes | run B bytes | |
| --- | --- | --- | --- |
| hardened_malloc | 6958256 | 6958256 | identical |
| mimalloc | 7178808 | 7178808 | identical |
| rpmalloc | 7313976 | 7313976 | identical |
| snmalloc | 7513200 | 7513200 | identical |
| system | 6950832 | 6950832 | identical |
| jemalloc | 13287816 | 13286320 | **differs by 1496 bytes** |

⚠ Only jemalloc moved, by 1496 bytes; its autotools build is the one that
embeds build-time strings. Whatever differs between the two images, it did not
change what five of the six cells actually measured. ⭐ This also retires the
old worry that the published datasets came from an image that could no longer
be rebuilt: it was rebuilt, and it produces the same binaries.

**The control moved with the hardware.** The baseline's own absolute time on
`literal`:

| | CPU | median | MAD | n |
| --- | --- | --- | --- | --- |
| run A | Xeon @ 2.80GHz | 61.46 ms | 1.78 ms | 8 |
| run B | Xeon @ 2.10GHz | 49.87 ms | 2.07 ms | 8 |

The control is **19% faster in run B** while each run reports a within-run MAD
of 3–4%. Since every figure here is a ratio to the control *in the same run*, a
control that moves that far — with the candidates not all moving with it — is
enough on its own to reshuffle the table.

## What these runs establish, and what they do not

✅ **The build is reproducible.** 5 of 6 binaries byte-identical across images.

✅ **On this sandbox, in both runs, musl's allocator is the slow one** and
snmalloc and mimalloc are the fastest two, at 0.55–0.66×.

⛔ **No magnitude is established.** Four of six cells moved by more than either
run's internal MAD.

⛔ **The ordering below the top two is not established.** jemalloc/rpmalloc
swapped and hardened_malloc changed sides of the control.

⛔ **And none of it transfers off this sandbox.** The GitHub-runner measurement
in the sibling directory puts snmalloc at **1.138×** — second-slowest, where
here it is the fastest — with *lower* within-run noise than either run above.
See `docs/AGENTS.md` §11.

⚠ **Do not quote a single figure from any one run as "the" ratio**, and do not
assume a within-run MAD bounds anything. Quote the direction, name the machine,
or quote every run.
