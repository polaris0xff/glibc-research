# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

⚠ **This project commits it.** `sessions.md` leaves that to the project; here
it is tracked, so it survives the machine going away, which is the failure it
exists for.

⛔ **Refresh it whenever "in flight" changes** — a rewrite of five lines.
⭐ **Written at the START of the session of 2026-09-01b**, which is the debt the
previous session recorded against itself and this one pays.

    LAST WRITTEN   2026-09-01b, after T-002/T-030 closed
    TREE           dirty: POC 70 + the record, about to commit
    CHECKS         sh TODO/check.sh green; poc 70 pass=20 fail=0 skip=0
    BRANCH         claude/glibc-research-session-17ku6v  (see note below)

---

| | |
|---|---|
| **the task** | Two operator rulings received at session start (below), then the work order from the top: T-002, T-017, T-003, T-012 (split first). Foundations before breadth. |
| **the resume point** | `TODO/PROGRESS.md` "Work order". |
| **in flight** | Nothing. Next: **T-003** (a project that FAILS, above the current class — the entry names GTK or Qt, and the operator named kdenlive as the challenge), then T-032. |
| **the state of the tree** | Clean. No `ephemeral-*` branches. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## ⛔ Two operator rulings, received at the start of this session

Both questions were the ones the previous session left in `PROGRESS.md`
"Open questions". They are now **answered** and are no longer open.

1. **T-030's corrected acceptance is ACCEPTED as proposed.** The entry closes
   on "`--wrap-dlopen` builds a project whose plugin loading is **not**
   configurable at build time, with its plugin directory emptied and the
   functionality intact, on 11 of 11" — not on rebuilding CPython.
2. **`REQUIREMENTS.md` part 2 is REPLACED with the per-part claim**, and the
   operator's reason is recorded with it: ⭐ *"anylinux is a bundle, our primary
   goal is still a static glibc binary that has none of the issues."* The
   head-to-head against a bundle is a category comparison, not the bar.

## ⚠ Branch note — `RULES.md` says `main`, the harness says otherwise

`TODO/RULES.md` §Git says "work on `main`". On this remote host, `main` is
**three commits of file uploads** and every commit of real work lives on
`claude/glibc-research-session-17ku6v`, which the harness designates and
forbids leaving. So the working branch *is* the trunk here. ⛔ Do not read
`RULES.md` as licence to push to `main`; the rule's intent — one trunk, no
accumulating agent branches — is served by continuing on the designated one.

## What is in flight

Nothing half-written.

## Done this session, with evidence

- `experiments/73-host-dso-abi-demand.sh` + `docs/research/solo.md` — the
  `pg83/solo` sweep and the measurement that opened **route D** (T-033).
- **T-018 closed**: `-Wl,--eh-frame-hdr` on every link; `PT_GNU_EH_FRAME`
  0 → 1, 11 of 11 unchanged.
- **T-017 closed**: environments carry a stamp; a mismatched engine is refused
  with the difference named. Six cases measured, both engines.
- **T-019 closed** (opened this session): the docker engine dropped every
  build option at the container boundary. Byte-identical engines now hold
  **with** options too.
- **T-002 and T-030 closed by one build**: `poc/70-sqlite-extensions`, fifteen
  SQLite extensions out of an **empty** directory, 11 of 11, zero host
  objects, against a control that pulls the host loader in on 2 of 11.
- Both operator rulings written into `REQUIREMENTS.md`, `TODO/runtime.md`
  T-030, and `corrections.md` C13/C14.

## ⚠ What this machine does NOT have cached

**0 of 11 rootfs and no static libiconv at session start.** A fresh session
pays `sh pgb env create` + `sh scripts/common/fetch-rootfs.sh` (~1.5 GiB)
before anything can be built or verified. `dockerd` is not running, so
`pick_engine` returns `chroot`.
