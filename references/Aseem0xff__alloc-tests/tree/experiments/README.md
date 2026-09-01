# experiments

Every measured claim in this repository was taken by a script in this
directory. **The scripts are the deliverable; the numbers are what they printed
on one machine on one day.**

Numbered in the order they were first run. ⛔ **A number is never reused.** If a
script is replaced, the old one stays and the new one takes the next number,
because a citation of `40-` in a write-up has to keep meaning what it meant.

| script | question it answers | exit at last run |
| --- | --- | --- |
| `10-probe-host.sh` | can this host run the benchmark at all, and what is it | **1** on the machine of record: 19 GiB free, below the 20 GiB a full suite wants. Docker and every tool were present. |
| `20-base-image-arch-support.sh` | which architectures does each target distribution actually publish an image for | **1 on purpose** — Arch publishes no aarch64 image |
| `30-ripgrep-default-allocator.sh` | what allocator does an *unmodified* ripgrep build use on musl | **1 on purpose** — it is not the system allocator |
| `50-libc-surgery-verify.sh` | does splicing an allocator into `libc.a` really displace the libc's own | 0 |

⚠ **`40-` and `60-` do not exist.** They are named here because the numbering is
a sequence and reserving a number keeps a later citation meaningful. They are
**planned, not written**, and nothing in this repository cites a result from
them.

| planned | question it would answer | why it is not written yet |
| --- | --- | --- |
| `40-allocator-build-matrix.sh` | which allocator × mechanism × profile combinations actually build, as a standalone assertion | a normal `alloc-bench run` already produces this; the script would make it assertable without one |
| `60-static-pie-aslr.sh` | do the profiles that claim ASLR get a moving load address | `alloc-runner aslr-probe --expect randomised` already asserts it per cell, and every published run carries the observation |

## Exit codes

Uniform across every script here, and the same convention `alloc-runner` and
`alloc-bench` use.

| code | meaning |
| --- | --- |
| 0 | the measurement ran and the thing passed |
| 1 | the measurement ran and the thing FAILED |
| 2 | the measurement could not run (missing tool, no container runtime, no network) |

⛔ **`2` is never reported as a pass.** "I could not look" and "I looked and
found nothing" are different results and the scripts keep them apart.

⚠ `20-` and `30-` exit **1 on purpose**. They are assertions about facts that
are true today and that a future upstream could change. Re-run them against a
newer Arch image or a newer ripgrep and a `0` means the situation has changed —
which is the result you want and cannot get from prose.

## Output

Each script writes to `out/<name>.txt` and **does not clean up**. The evidence
is the point. `out/` is committed.

Conditions — host, tool versions, date, sample count — are printed at the top of
every output file, because a number that has lost its conditions cannot be
compared with anything.

## Running them

```sh
sh experiments/10-probe-host.sh
```

They resolve paths from their own location, so the working directory does not
matter. Those that need a container runtime say so and exit 2 without one.
