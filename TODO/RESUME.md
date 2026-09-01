# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-01c, refresh 3
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

    DONE   ten references mined; docs/research/nix.md is the write-up
    DONE   scripts/common/nix-fetch.sh + nix-nar.py: a nixpkgs closure with
           NO nix, verified. experiments/80-, 16 assertions, 0 skips.
    DONE   tool/nix-drv.py: nix's ATerm derivation format, so PLANNING needs
           no nix either. experiments/83-, 7 assertions, 0 skips.
    DONE   pgb nix plan|fetch|build with a dependency walk and an adaptation
           loop. bash, gawk, jq, sqlite3, htop, tmux built static from
           nixpkgs plans; bash and htop verified 11/11, zero host objects.
    DONE   tool/nix-appimage.sh: a GTK app (galculator) bundled with
           uruntime + dwarfs + sharun, reaching GTK on musl and glibc.
    NEXT   the operator's three goals, in PROGRESS.md's work order. T-052
           (OpenGL) gates two of them and is the first thing to measure.

## ⚠ Where the artefacts are on this machine

    /var/tmp/pgb-nix/<attr>/out/       each ladder build's binaries
    /var/tmp/pgb-appimage/galculator/  the AppDir and the AppImage
    ~/.local/state/pgb/plans/          the plans, both routes
    ~/.local/state/pgb/nix-prefix/     the shared static dependency prefix
    /var/tmp/pgb-nix-cache/            the channel index and every narinfo

## ⛔ The branch situation

The harness named `claude/nix-mining-static-builds-q2ffvi`; `RULES.md` §Git and
the operator's own instruction say `main`. `main` was nine commits behind that
branch and was fast-forwarded onto it. ⚠ **Two remote `claude/*` branches are
on GitHub**, both at commits `main` contains; this environment's git proxy
refuses to delete a remote branch, so they need one click each in the web UI.
