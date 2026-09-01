# ci — the workflow that has never run

## T-040 — Run CI once

**Source** `docs/AGENTS.md` §9 · **Category** ci · **Priority** P1 · **Effort** S · **Status** ✅ done

⛔ **The title's premise was false and the title keeps it**, per
`../docs/methodology/authoring.md`. CI had not "never run": it had run **ten
times and been red ten times** before this entry was written. The entry is now
*get it green*.

**Problem.** Runs 1–10 (`79bbfa33` … `b77e0333`) were all red, on the same two
rows, and on neither did a probe ever execute:

```
voidlinux    exec /__e/node24/bin/node: no such file or directory
alpine-3.10  Error relocating /__e/node24_alpine/bin/node:
               pthread_getname_np: symbol not found
               secure_getenv: symbol not found
```

A job using GitHub's `container:` has the runner inject its own dynamically
linked Node.js to execute JavaScript actions. It cannot start on Void's musl
(the runner picks the glibc build unless `ID=alpine`) or on Alpine 3.10's musl
1.1.22. `actions/download-artifact` died before the binary was fetched.

**Premise.** ⭐ **Measured, not predicted, and it holds.** The nine other rows
were green every run — `probe-portable` printed `PASSED: 0 failure(s)` — and
the plain control segfaulted on Arch, which is the positive control. This
session then ran the *whole* matrix locally under `docker run --entrypoint`
against the digest-pinned images: **11 of 11 portable ok, 11 of 11 plain
failed**, including both rows CI could not reach. The chroot bed agreed:
11 of 11 ok, zero host shared objects.

**Approach.** Done in this session; what remains is the green run itself.

1. every job on the `ubuntu-latest` host; targets entered with
   `docker run --entrypoint`, so the only process in the target image is the
   probe — no shell, no Node, no runner;
2. the matrix **generated** from `scripts/common/rootfs-images.txt` and
   asserted to be 11 rows. ⛔ Runs 1–10 hand-wrote tags (`archlinux:latest` is
   rolling), so CI and the local bed were two different beds reporting as one.
   Measured consequence: CI's Arch killed the control with **SIGSEGV**; the
   digest-pinned Arch kills it with **SIGFPE**;
3. an assertion that the two arms are different binaries, because every other
   assertion is made against the pgb arm and a no-op `pgb` would otherwise go
   green;
4. `TODO/check.sh` and `sh -n` over every script, as CI steps.

`../docs/history/corrections.md` C8.

**Prove.** A green run on a runner, with its URL recorded in this entry.

**Closed with** run 11, the first green run this workflow has ever had:
<https://github.com/polaris0xff/glibc-research/actions/runs/33506148035>
(`a1d30d3`, 2026-09-01). ⛔ **The rollup is not the evidence** — a run can be
green because it did less. 14 jobs, all `success`, and the matrix job names
carry the digest each row resolved to:

```
matrix                                          success   parsed 11 targets
build                                           success   incl. "Assert the two arms are actually different binaries"
probe-host                                      success   incl. TODO/check.sh and sh -n over every script
run-matrix (alpine-3.22,  musl,  alpine@sha256:7c8cb692…)          success
run-matrix (alpine-3.20,  musl,  alpine@sha256:c64c687c…)          success
run-matrix (alpine-3.10,  musl,  alpine@sha256:e515aad2…)          success   <- red in runs 1-10
run-matrix (voidlinux-musl, musl, voidlinux/…@sha256:d5c970d0…)    success   <- red in runs 1-10
run-matrix (debian-11,    glibc, debian@sha256:c0a2ad73…)          success
run-matrix (debian-12,    glibc, debian@sha256:2f65600e…)          success
run-matrix (ubuntu-20.04, glibc, ubuntu@sha256:c664f8f8…)          success
run-matrix (rockylinux-8, glibc, rockylinux@sha256:2d05a926…)      success
run-matrix (opensuse-leap-15.6, glibc, opensuse/leap@sha256:ca2942f9…) success
run-matrix (fedora-42,    glibc, fedora@sha256:7c63468d…)          success
run-matrix (archlinux-latest, glibc, archlinux@sha256:818793c8…)   success
```

⭐ **Both rows that had never executed a binary now execute one and it passes.**

⚠ **What this run does NOT establish**, so it is not read as more than it is:
the eleven rows assert the program's own exit status. They do **not** assert
"loaded no host shared object" — that is criterion 2 of `docs/AGENTS.md` §3
and it needs the trace instrument, which needs `pgb verify` to have a docker
engine. Carried as **T-014**, and until it lands CI is a weaker check than the
local bed. `podman` is still unexercised.

## T-041 — aarch64

**Source** `docs/AGENTS.md` §13 · **Category** ci · **Priority** P2 · **Effort** M · **Status** open

**Problem.** Every number in this repository is x86_64, one machine, one day.
`--arch arm64` exists in `oci-pull.sh` and `fetch-rootfs.sh` and re-resolves by
tag, trading the digest pin away.

**Premise.** ⚠ Expect IFUNC and CPU-baseline questions x86_64 did not raise.
`experiments/61-` shows glibc's advantage is largely IFUNC-dispatched routines,
so the throughput result may not carry.

**Prove.** `experiments/61-` and `62-` run on an aarch64 runner with their
tables filled.
