# HISTORY/entries/ci-open.md — retired DETAIL of ci entries that are STILL OPEN

⚠ **These entries are open. This file is not the entry** — the entry is in
[`../../TODO/ci.md`](../../TODO/ci.md) and is deliberately short. What is
here is the long-form record each one accumulated: the measurements, the
corrections, the routes costed and the routes killed.

⛔ **Read the TODO entry first.** Come here when you need to know WHY it says
what it says, or before re-running something to check whether it was already
run. ⭐ A number quoted in the TODO entry was derived here.

⚠ The headings below deliberately do NOT use the `## T-NNN — ` form, because
that form is what `sh TODO/check.sh` treats as *the* entry, and there must be
exactly one of those per id.

---

## T-041 · retired detail — aarch64

**Source** `docs/AGENTS.md` §13 · **Category** ci · **Priority** P2 · **Effort** M · **Status** open

**Problem.** Every number in this repository is x86_64, one machine, one day.
`--arch arm64` exists in `pgb rootfs pull` and `pgb rootfs fetch` and re-resolves by
tag, trading the digest pin away.

**Premise.** ⚠ Expect IFUNC and CPU-baseline questions x86_64 did not raise.
`experiments/61-` shows glibc's advantage is largely IFUNC-dispatched routines,
so the throughput result may not carry.

**Prove.** `experiments/61-` and `62-` run on an aarch64 runner with their
tables filled.
