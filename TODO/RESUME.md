# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

⚠ **This project commits it.** `sessions.md` leaves that to the project; here
it is tracked, so it survives the machine going away, which is the failure it
exists for.

⛔ **Refresh it whenever "in flight" changes** — a rewrite of five lines.
⚠ It was written at the END of the session of 2026-09-01, not the start. That
is the wrong time and is recorded as the debt it is: had that session died, it
would have handed over nothing.

    LAST WRITTEN   2026-09-01T13:50Z
    TREE           clean, everything pushed to main
    CHECKS         sh TODO/check.sh green; CI run 19 green, 15 jobs

---

| | |
|---|---|
| **the task** | Work through the foundational TODO entries in an order where doing one makes the next possible, verifying every claim rather than trusting the record. |
| **the resume point** | `TODO/PROGRESS.md` "Work order". The head of it is **T-002** — a project that dlopens its own plugins at scale, which is also what T-030 now needs. |
| **in flight** | ⚠ **Nothing is half-written.** Every entry touched is either closed with its evidence or open with the blocker named. Two need an operator decision before an agent may proceed — see below. |
| **the state of the tree** | Clean. Working tree has no uncommitted changes; `main` is pushed; no `ephemeral-*` branches. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## ⛔ Two things an agent must not decide alone

1. **T-030's acceptance was disproved and a replacement is proposed, not
   adopted.** Its `Prove` named CPython, and `experiments/72-` showed the
   subject that acceptance needs cannot be built. The corrected acceptance is
   written in the entry and is **the operator's to accept or change**.
2. **`REQUIREMENTS.md` part 2 is still not met**, and `--wrap-dlopen` narrowed
   the gap without closing it. `PROGRESS.md` "Open questions" states it.

## What was in flight and is now not

Nothing. The last unit of work — `poc/60-leveldb`, the first C++ and first
CMake POC — closed with `pass=12 fail=0 skip=0` and CI green afterwards.
