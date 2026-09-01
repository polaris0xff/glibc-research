# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-01d, session end -- all four required POCs closed
    TREE           clean, everything committed and pushed to main
    CHECKS         sh TODO/check.sh green; 9 selftests re-run, all pass
    BRANCH         main. ⭐ NO BRANCH DEBT: the harness named
                   claude/glibc-research-poc-0mwrxy, RULES.md §Git outranks it,
                   the branch was deleted locally and never reached the remote.
                   `git ls-remote --heads origin` lists refs/heads/main only.

---

| | |
|---|---|
| **the task** | ⭐ **The operator's three goals**, quoted in full in `PROGRESS.md`: (1) the universal **builder** via pgb + nix, (2) the universal **bundler**, (3) **kdenlive**: static if it can be reached, otherwise a bundle that beats `pkgforge-dev/kdenlive-AppImage-Enhanced`. |
| **the resume point** | ⛔ **The previous stop condition is DISCHARGED**, so `PROGRESS.md` "Work order" is a real work order again. It starts at **T-055** (the kdenlive bundle comparison, now unblocked — T-054 rung 1 and T-052 are exactly what it was waiting on) and **T-054 rungs 2–4**. |
| **in flight** | **nothing half-written.** Every mechanism below is landed, committed, and has a run behind it. |
| **⛔ do not parallelise** | `RULES.md` §"one thing at a time on the bed": the reaper kills by `/proc/PID/root` and cannot tell one run's process from another's — a `SIG9` row in an otherwise clean table is what that looks like. And **T-058**: two `pgb build`s share one wrapper directory and one set of option-dependent flags. ⭐ A bundle build (`tool/nix-appimage.sh`) touches neither and **can** run beside a compile. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## What landed this session

    poc/90-qt/run.sh          Qt 6.11.1 static, 11 of 11, zero host objects
    experiments/85-opengl.sh  the bundled GL stack on the eleven, with a
                              --no-gl control that produces no vendor anywhere
    experiments/86-...sh      our one-command bundle vs a hand-built Anylinux
                              AppImage: size, cold/warm startup, host objects
    poc/20, poc/30 reruns     --embed-terminfo and --embed-cacert, 11/11 each
    tool/lib/wrappers.sh      compile flags LEAD the command line; -march=native
                              rewritten in the caller's argv; atomic wrapper
                              writes instead of rm -rf
    tool/nix-appimage.sh      --name that names no program is refused; the bin
                              output is preferred over out
    tool/runtime/pgb-cacert.c the missing <stdio.h>

⭐ **`docs/REQUIREMENTS.md` part 2 now reads 8 of 9 issues closed**, up from
6 of 9. The one left is **host plugins**.

## ⛔ Machine notes a fresh session cannot infer

- **nix IS installed** (Determinate Nix, pkgforge installer). ⚠ Its **flake
  route is broken here** — the harness proxy answers `api.github.com` with 403,
  so `nixpkgs#attr` fails. `nix-instantiate '<nixpkgs>' --attr X` is the route
  that works, and everything in `tool/lib/nix.sh` uses it.
- **`pgb env create` ignores a trailing `--engine`**; the global one works:
  `sh pgb --engine chroot env create`. Without it `pick_engine` chooses docker
  the moment dockerd is up, and `bootstrap.sh --check` then reports
  `build env (chroot) ABSENT` while every build actually works through docker.
- **4 cores, 15 GiB RAM.** qtbase (1283 targets) took about 25 minutes; a
  mesa-demos bundle about 6; the 86- matrix about 7.
- Artefacts, **none of which survive the machine**:

```
/var/tmp/pgb-poc/90-qt/{qtbase,build,inst,app}   the Qt build, several GiB
/var/tmp/pgb-appimage/{eglinfo,jq}/              bundles and their AppDirs
/var/tmp/pgb-appimage-nogl/eglinfo/              85-'s control arm
/var/tmp/pgb-nix-cache/                          channel index and narinfos
~/.local/state/pgb/                              wrappers, runtime objects, plans
```

## ⛔ Start here, in this order

```sh
sh scripts/common/bootstrap.sh --detach   # FIRST LINE. ~10 min, in parallel
sh scripts/common/bootstrap.sh --check    # when it is done
```

1. **`TODO/SUMMARY.md`** — one table, what the last session actually did.
2. **`TODO/PROGRESS.md`** — the three goals and the work order.
3. The entries the work order names: **T-055**, **T-054** (rung 1 closed,
   rungs 2–4 open), **T-050/T-051**, **T-058**, **T-057**.
