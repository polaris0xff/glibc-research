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

    LAST WRITTEN   2026-09-01T (session 2026-09-01b, start)
    TREE           clean at 86c40c8
    CHECKS         not yet re-run this session
    BRANCH         claude/glibc-research-session-17ku6v  (see note below)

---

| | |
|---|---|
| **the task** | Two operator rulings received at session start (below), then the work order from the top: T-002, T-017, T-003, T-012 (split first). Foundations before breadth. |
| **the resume point** | `TODO/PROGRESS.md` "Work order". |
| **in flight** | Machine bootstrap: `pgb env create` + `fetch-rootfs.sh`. **This machine started with 0 of 11 rootfs present and no static libiconv** — nothing in the bed is cached between sessions. |
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

Bootstrap only. Nothing half-written in the tree.
