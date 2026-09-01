# mechanisms, alpine / x86_64 / static-pie, 2026-09-01

mimalloc reached three ways, against musl's own allocator. Same host, same
corpus, same ripgrep, 6 samples per workload.

## ⚠ TWO RUNS, AND THEY DISAGREE ON THE MAGNITUDE

`rankings.json` here is **run B**. `run-a-11member-splice/rankings.json` is an
earlier run, kept deliberately.

| mechanism | run A | run B | 
| --- | --- | --- |
| `libc-surgery` | 0.444× | **0.460×** |
| `rust-global` | 0.501× | **0.606×** |
| `link-override` | fails to link | fails to link |
| surgery advantage | 13.1% | **31.6%** |

Between the runs, `scripts/build/libc-surgery.sh` changed: run A displaced 11
`libc.a` members (the public allocation entry points), run B displaces 13 (the
same plus musl's mallocng implementation objects). See
`evidence/50-libc-surgery-verify.txt`.

⛔ **That change does not explain the gap.** It affects only the surgery cell,
and the surgery cell barely moved (0.444 → 0.460). The row that moved is
`rust-global` (0.501 → 0.606), which was built identically in both runs.

⭐ **So the honest reading is:**

- **The direction is robust.** `libc-surgery` beat `rust-global` in both runs,
  and both margins are far outside either run's internal spread.
- **The magnitude is not.** 13% and 32% are the same comparison run twice.
- ⚠ **Within-run MAD understates between-run variability on this host.** Every
  cell here reported 1.6–2.9% MAD, and the same cell moved ~20% between runs.
  A reader should treat the MAD as a floor on the uncertainty, not a bound.

This is why the project reports its own noise and refuses a winner inside it,
and it is why a single run is not enough to publish a magnitude.

## Files

| | |
| --- | --- |
| `report.md`, `rankings.json` | run B, generated from `results/` |
| `run-a-11member-splice/` | run A's rankings, kept so the disagreement is checkable |
| `evidence/libc-surgery.txt` | what the splice displaced, per `libc.a` copy |
| `evidence/link-override-failure.txt` | the linker's own message |
| `evidence/50-libc-surgery-verify.txt` | the standalone verification experiment |
