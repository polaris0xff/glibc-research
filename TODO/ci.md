# ci

⚠ **Open entries only.** T-040, which ran CI for the first time, is
[`../HISTORY/entries/ci.md`](../HISTORY/entries/ci.md); the long-form detail
behind the entry below is
[`../HISTORY/entries/ci-open.md`](../HISTORY/entries/ci-open.md).

---

## T-041 — aarch64

**Source** [`../docs/AGENTS.md`](../docs/AGENTS.md) §13 · **Category** ci · **Priority** P2 · **Effort** M · **Status** open

**Problem.** Every number in this repository is x86_64, one machine, one day.
`--arch arm64` exists in `pgb rootfs pull` and `pgb rootfs fetch` and
re-resolves by tag, trading the digest pin away.

**Premise.** ⚠ Expect IFUNC and CPU-baseline questions x86_64 did not raise.
`experiments/61-` shows glibc's advantage is largely IFUNC-dispatched
routines, so the throughput result may not carry.

**Prove.** `experiments/61-` and `62-` run on an aarch64 runner with their
tables filled.
