# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-01d, session START (refreshed as work moves)
    TREE           clean at 544bfa61, on main
    CHECKS         not yet run this session
    BRANCH         main  ⭐ the harness named claude/glibc-research-poc-0mwrxy;
                   RULES.md §Git outranks it. The branch was deleted locally
                   and `git ls-remote --heads origin` shows main and nothing
                   else — it never reached the remote.

---

| | |
|---|---|
| **the task** | ⛔ **THE STOP CONDITION in `PROGRESS.md`: four required POCs, in order.** (1) `poc/90-qt` a static Qt 6 widget program through the pgb toolchain, 11/11 or the rung that stopped it at file and line; (2) `experiments/85-opengl` the bundled GL stack on all eleven; (3) `experiments/86-bundler-vs-anylinux`; (4) `poc/20` + `poc/30` reruns for `--embed-terminfo`/`--embed-cacert`. **End the session when those four are done.** |
| **the resume point** | whichever of the four has no `evidence/.../RESULT.txt` yet. |
| **in flight** | nothing yet — session just started. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## Machine state at session start

    bootstrap.sh --detach   started as the first command
    nix                     present (Determinate Nix, pkgforge installer)
    test bed                11 of 11 rootfs under /var/lib/pgb-rootfs
    dockerd                 running; pgb-env:0.1.0 built for docker
    build env (chroot)      ABSENT at start — `pgb env create` picked docker
                            because dockerd was already up. Started by hand:
                            `sh pgb env create --engine chroot`
    cores / RAM / disk      4 / 15 GiB / 26 GiB free

⚠ **The 4 cores are the schedule.** Qt 6 is the long pole; it is started first
and everything else runs beside it.

## ⛔ Start here, in this order

```sh
sh scripts/common/bootstrap.sh --detach   # FIRST LINE. ~10 min, in parallel
sh scripts/common/bootstrap.sh --check    # when it is done
```

1. **`TODO/PROGRESS.md`** — the stop condition table. Read it before choosing
   anything.
2. The four entries it names: **T-054** (Qt — and read why "impossible" is not
   what the record says), **T-052**, **T-057**, **T-032**.
