# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

⚠ **This project commits it.** `sessions.md` leaves that to the project; here
it is tracked, so it survives the machine going away, which is the failure it
exists for.

    LAST WRITTEN   2026-09-01c, session START (refreshed as things move)
    TREE           clean at session start
    CHECKS         sh TODO/check.sh green at session start
    BRANCH         main  ⭐ and the harness named a `claude/*` one again --
                   see below; `main` was fast-forwarded onto the work instead

---

| | |
|---|---|
| **the task** | ⭐ **The nix front end, end to end.** Mine the nix references, then build POCs in this order: (1) use/fetch nixpkgs **without installing nix**, (2) `pgb` builds **static bash** from the soarpkgs manifest shape, (3) climb to harder CLI tools until `nix-build` refuses and `pgb` patches it through, (4) a **GUI app** built with the `Anylinux-AppImages` runtime rather than nix-appimage's outdated one. Operator instruction, this session. |
| **the resume point** | `docs/design/nix-front-end.md` §"What the next session owes". |
| **in flight** | see the block below, refreshed as work moves |
| **the state of the tree** | see below |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## In flight, right now

    STEP    mining the six nix references (methodology/references.md)
    DONE    nix-ld, nput, nix-download, docker-nixuser mined
    NEXT    soarpkgs at the operator's pin, then read all six

## ⛔ The branch situation, again

The harness named `claude/nix-mining-static-builds-q2ffvi`, and both
`RULES.md` §Git and the operator's session instruction say `main`. ⭐ **The
previous session's nine commits were sitting only on that branch and `main`
was nine behind**; `main` was fast-forwarded onto them and pushed, and the work
of this session is committed straight to `main`.

⚠ **Two remote `claude/*` branches now exist on GitHub** and this
environment's git proxy refuses to delete a remote branch (measured last
session, both `--delete` and the colon form disconnect). Both point at commits
`main` contains. They need one click each in the web UI.

## ⚠ What this machine does NOT have cached

**0 of 11 rootfs and no static libiconv at session start**, and no nix. A
session that needs the bed pays `sh pgb env create` +
`sh scripts/common/fetch-rootfs.sh` (~1.5 GiB) first. `/var/tmp/pgb-poc` is
empty on this machine, so POC 80 is a 30–60 minute build rather than a re-run.

Disk at session start: **30 GiB free**, 4 cores, 15 GiB RAM, running as root.
