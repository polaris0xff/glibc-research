# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-01d, three of the four required POCs closed
    TREE           committed and pushed to main after every step
    CHECKS         sh TODO/check.sh green
    BRANCH         main  ⭐ the harness named claude/glibc-research-poc-0mwrxy;
                   RULES.md §Git outranks it. Deleted locally;
                   `git ls-remote --heads origin` shows main and nothing else —
                   it never reached the remote, so there is no branch debt.

---

| | |
|---|---|
| **the task** | ⛔ **THE STOP CONDITION in `PROGRESS.md`: four required POCs, in order, then END THE SESSION.** |
| **the resume point** | **deliverable 3, `experiments/86-bundler-vs-anylinux`, is the only one left** — it was running when this was written. Deliverables 1, 2 and 4 are closed with evidence. |
| **in flight** | `sh experiments/86-bundler-vs-anylinux.sh > evidence/86-bundler-vs-anylinux/RESULT.txt 2>&1`. Both artefacts are already built and cached (`/var/tmp/pgb-appimage/jq/` and `evidence/86-.../build/jq-any-x86_64.AppImage`), so a re-run skips straight to the matrix. Budget **~40 minutes**: every AppImage start costs ~14 s of dwarfs mount and there are 14 starts per environment. |
| **⛔ do not parallelise** | `RULES.md` §"one thing at a time on the bed". Learned here: `poc/30-curl`'s voidlinux row came back `SIG9` because `experiments/85-` was reaping the same rootfs. And T-058: two `pgb build`s share one wrapper directory. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## What closed this session

| # | deliverable | evidence |
|---|---|---|
| 1 | **`poc/90-qt`** — Qt 6.11.1 static, 11 of 11, zero host objects | `evidence/poc/90-qt/RESULT.txt`, 19 assertions, 0 fail |
| 2 | **`experiments/85-opengl`** — bundled GL on all eleven | `evidence/85-opengl/RESULT.txt`, 7 assertions, 0 fail |
| 3 | `experiments/86-bundler-vs-anylinux` | ⏳ running |
| 4 | **`poc/20` + `poc/30` reruns** — T-032 | 12 assertions each, 0 fail |

**Four defects found and fixed, every one by building above the current class:**
`pgb`'s `-march` baseline overrode every project's own (C15); two `pgb build`s
share one wrapper directory (T-058, half fixed); `nix-appimage.sh` silently
packed the wrong program when `--name` missed; and it read nixpkgs' `out`
output when the executables are in `bin`.

## ⛔ Machine notes a fresh session cannot infer

- **nix IS installed** (Determinate Nix, pkgforge installer). ⚠ Its **flake
  route is broken here** — the harness proxy answers `api.github.com` with 403,
  so `nixpkgs#attr` fails. `nix-instantiate '<nixpkgs>' --attr X` is the route
  that works.
- **`pgb env create` ignores a trailing `--engine`**; the global one works:
  `sh pgb --engine chroot env create`. Without it, `pick_engine` chooses docker
  the moment dockerd is up, and `bootstrap.sh --check` then reports
  `build env (chroot) ABSENT` while everything actually works through docker.
- **4 cores, 15 GiB RAM.** Qt 6 qtbase (1283 targets) took about 25 minutes.
- Artefacts, none of which survive the machine:

```
/var/tmp/pgb-poc/90-qt/{qtbase,build,inst,app}   the Qt build, ~5 GiB
/var/tmp/pgb-appimage/{eglinfo,jq}/              bundles + their AppDirs
/var/tmp/pgb-appimage-nogl/eglinfo/              the 85- control arm
/var/tmp/pgb-nix-cache/                          channel index and narinfos
~/.local/state/pgb/                              wrappers, runtime objects, plans
```

## ⛔ Start here, in this order

```sh
sh scripts/common/bootstrap.sh --detach   # FIRST LINE. ~10 min, in parallel
sh scripts/common/bootstrap.sh --check    # when it is done
```

1. **`TODO/PROGRESS.md`** — the stop condition and the work order.
2. The entries: **T-054** (Qt rung 1 closed, rungs 2–4 open), **T-057**,
   **T-058**, **T-059**.
