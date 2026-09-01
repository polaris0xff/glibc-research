# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-01c, refresh 2 (mid-session)
    TREE           committed through "pgb nix: nixpkgs plans, pgb builds"
    CHECKS         sh TODO/check.sh green
    BRANCH         main  (fast-forwarded onto the previous session's work)

---

| | |
|---|---|
| **the task** | ⭐ **The nix front end, end to end.** (1) mine the nix references, (2) POC fetching/using nixpkgs **without installing nix**, (3) `pgb` builds **static bash** from the soarpkgs manifest shape, (4) climb to harder CLI tools until `nix-build` refuses and `pgb` patches it through, (5) a **GUI app** built with the `Anylinux-AppImages` runtime rather than nix-appimage's outdated one. |
| **the resume point** | the ladder in "in flight" below, then the GUI app. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## In flight, right now

    DONE   six references mined (nix-ld, nput, nix-download, docker-nixuser,
           nix-user-chroot re-mined, soarpkgs AT THE OPERATOR'S PIN via a new
           mine-repo.sh --ref)
    DONE   scripts/common/nix-fetch.sh + nix-nar.py: resolve and fetch a
           nixpkgs closure with NO nix. 28-check selftest, real narinfo
           fixtures committed.
    DONE   experiments/80-: 16 assertions, 0 skips, oracle-checked against a
           real nix. Our closure == nix-store -qR; our extraction == nix's own
           /nix/store tree.
    DONE   `pgb nix plan|fetch|build`: nixpkgs is the planner, pgb builds
           static glibc. bash 5.3p15 built in 60s, 16 patches at -p0, no
           PT_INTERP, zero /nix/store strings. gawk built too.
    NOW    the ladder: jq, sqlite, htop, tmux. Two derivation shapes are
           handled (old flat `env`, new `__structuredAttrs`).
    NEXT   `pgb verify` the nix-built bash on all 11; write docs/research/nix.md
           (the sweep write-up methodology/references.md requires); then the
           GUI app on the Anylinux runtime.

## ⛔ Machine state a fresh session inherits nothing of

- **nix IS installed here** (Determinate Nix 3.22.2, via pkgforge's
  `install_nix.sh`, operator-authorised). `/nix` is ~2 GiB.
  ⚠ `nix registry`'s flake route is BROKEN in this environment: the harness
  proxy answers `api.github.com` with 403, so `nixpkgs#attr` fails and
  **`nix-instantiate '<nixpkgs>' --attr X` is the route that works** — the
  channel is fetched over plain HTTPS. Every command in `tool/lib/nix.sh` uses
  the channel form for that reason.
- **the 11-environment bed IS fetched** (`/var/lib/pgb-rootfs`), and
  `pgb env create` has run.
- `/var/tmp/pgb-nix/<attr>/` holds each ladder build; `~/.local/state/pgb/plans`
  holds the plans.

## ⛔ The branch situation

The harness named `claude/nix-mining-static-builds-q2ffvi`; `RULES.md` §Git and
the operator's own instruction say `main`. `main` was nine commits behind that
branch and was fast-forwarded onto it. ⚠ **Two remote `claude/*` branches are
on GitHub**, both at commits `main` contains; this environment's git proxy
refuses to delete a remote branch, so they need one click each in the web UI.
