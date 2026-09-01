# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-01c, session end
    TREE           clean, everything committed and pushed to main
    CHECKS         sh TODO/check.sh green
    BRANCH         main

---

| | |
|---|---|
| **the task** | ⭐ **The operator's three goals**, quoted in full in `PROGRESS.md`: (1) the universal **builder** via pgb + nix, (2) the universal **bundler** — a maintained nix-appimage descendant on the Anylinux mechanisms, debloated, with the OpenGL problem solved, (3) **kdenlive**: static if it can be reached, otherwise a bundle that beats `pkgforge-dev/kdenlive-AppImage-Enhanced` on size, load and run. |
| **the resume point** | `PROGRESS.md` "Work order". T-052's remaining half or T-057's debloat, whichever the operator prefers; both are one step from a measurable result. |
| **in flight** | **nothing half-written.** Every mechanism below is landed, committed and has a run behind it. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## What landed this session

    scripts/common/nix-fetch.sh + nix-nar.py   a nixpkgs closure with NO nix,
                                               signature and NarHash checked
    tool/nix-drv.py                            nix's ATerm .drv format, so
                                               PLANNING needs no nix either
    tool/nix-plan.py, tool/lib/nix.sh          pgb nix plan|fetch|build, a
                                               dependency walk, and the
                                               adaptation loop
    tool/nix-appimage.sh + tool/elf-needed.py  the bundler: uruntime + dwarfs
                                               + sharun, and the mesa half of
                                               the OpenGL problem
    experiments/80-, 83-                       16 and 7 assertions, 0 skips
    docs/research/nix.md                       the sweep write-up

Built static from nixpkgs plans: **bash, gawk, jq, sqlite3, htop, tmux**.
`bash` and `htop` verified on **11 of 11, zero host shared objects**.
Bundled: **galculator** (GTK3, reaches GTK on musl) and **mesa-demos**
(`EGL vendor string: Mesa Project`, `driver name: swrast`, no GPU present).

## ⛔ Start here, in this order

1. **`docs/research/nix.md`** — the findings, the instruments, and the
   known-weak claims. It opens with what was NOT established; read that part.
2. **`TODO/PROGRESS.md`** — the operator's three goals, quoted, and the work
   order.
3. **T-052** for the OpenGL half that is left (nvidia, and nothing has run on
   the eleven), **T-057** for the bundler's five gaps, **T-054** for kdenlive.

## ⚠ Where the artefacts are on this machine

    /var/tmp/pgb-nix/<attr>/out/       each ladder build's binaries
    /var/tmp/pgb-appimage/{galculator,eglinfo}/   AppDirs and AppImages
    /var/tmp/pgb-appimage/tools/       uruntime, sharun, mkdwarfs (pinned)
    ~/.local/state/pgb/plans/          the plans, both routes
    ~/.local/state/pgb/nix-prefix/     the shared static dependency prefix
    /var/tmp/pgb-nix-cache/            the channel index and every narinfo

⚠ **None of it survives the machine.** A fresh session pays `pgb env create`
(~5 min), `fetch-rootfs.sh` (~1.5 GiB), and the nix install again.

## ⛔ Machine notes a fresh session cannot infer

- **nix IS installed here** (Determinate Nix 3.22.2, via pkgforge's
  `install_nix.sh`, operator-authorised). ⚠ Its **flake route is broken in
  this environment**: the harness proxy answers `api.github.com` with 403, so
  `nixpkgs#attr` fails. **`nix-instantiate '<nixpkgs>' --attr X` is the route
  that works**, and everything in `tool/lib/nix.sh` uses it.
- **The pinned build environment gained `bison flex gettext texinfo`** this
  session, because nixpkgs said tmux needed bison and the environment could
  not run it. A rebuilt environment picks them up from `PGB_ENV_PACKAGES`.
- `PGB_NIX_FORCE_EVAL=1` skips the nix-free plan route, which is how
  `experiments/83-` compares the two.

## ⛔ The branch situation

The harness named `claude/nix-mining-static-builds-q2ffvi`; `RULES.md` §Git and
the operator's own instruction say `main`. `main` was **nine commits behind**
that branch at session start and was fast-forwarded onto it; every commit of
this session is on `main` and pushed. ⚠ **Three remote `claude/*` branches are
on GitHub**, all at commits `main` contains; this environment's git proxy
refuses to delete a remote branch, so they need one click each in the web UI.
