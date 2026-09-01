# ci — the workflow that has never run

## T-040 — Run CI once

**Source** `docs/AGENTS.md` §9 · **Category** ci · **Priority** P1 · **Effort** S · **Status** open

**Problem.** `.github/workflows/portability.yml` and `ci/probe.c` are written
and have **never executed on a runner**. The workflow has not proved its own
YAML, and the docker/podman engines have never been exercised.

**Premise.** ⚠ Locally the probe passes on all 11 and the plain control fails
on all 11, so the portable arm should be green. That is a prediction.

**Prove.** A green run on a runner, with its URL recorded in this entry.

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
