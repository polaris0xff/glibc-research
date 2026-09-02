# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-01e, session START
    TREE           main, clean at session start
    BRANCH         ⛔ main. The harness named `claude/glibc-nix-static-v2nttp`
                   and RULES.md §Git outranks it. ⚠ THE PREVIOUS SESSION'S TEN
                   COMMITS WERE ON THAT BRANCH AND NOT ON main -- main was at
                   b77e0333. Fast-forwarded main to 4745d267 and pushed; the
                   branch is deleted locally and pruned from the remote.
                   `git ls-remote --heads origin` lists refs/heads/main ONLY.

---

| | |
|---|---|
| **the task** | ⛔ **The operator re-opened the previous session's closures** as met by the narrowest reading: `poc/90-qt` built a Qt with no xcb/GL/network/sql and an offscreen QPA ("a Qt library that links, not a Qt application"), and `experiments/86-` compared bundlers on **jq**, which is two shared objects. **Priority: FINISH THE NIX WORK** — (1) T-050/T-051 the no-nix route *finished*, (2) ⭐ **static-glibc nix**, (3) nix+AppImage finished (debloat, wrapper scripts, lib32, then 86- against a real application), (4) T-055 and T-054 rungs 2–4 with Qt built properly. |
| **the resume point** | see "In flight" below |
| **⛔ do not parallelise** | `RULES.md` §"one thing at a time on the bed" and **T-058** (two `pgb build`s share one wrapper directory). T-058 is the first thing this session fixes so the 4 cores become usable. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## In flight

    CLOSED   T-058 (experiments/87-)  T-050 (experiments/88-)  T-053
    LANDED   T-057 items 1, 3, 4: experiments/89- (debloat, three arms, all
             identical on 11 of 11) and 86- re-run on mpv (2.71x, 11/11)
    MEASURED T-055: experiments/90-, kdenlive 26.08.0 both sides.
             ⛔ THE BAR IS NOT MET: 2.07x the size, 1.81x the render,
             2.5x cold / 4.1x warm start. 11 of 11 either way. The route to
             the bar is written in the entry, in the order the numbers say.
    RUNNING  poc/91-qt-xcb -- qtbase CONFIGURES with xcb (the static xcb link
             is proved first, then TEST_xcb_syslibs is overridden with that
             as the evidence). The build is in flight; watch
             /var/tmp/pgb-t054/poc91l.log
    RUNNING  T-060 rung 1, the nix closure: sh /var/tmp/pgb-t060/rung1.sh
             -> rung1b.log. 30 libraries built. Eleven pgb defects found and
             fixed on the way; the entry lists every one.

## ⛔ Machine notes a fresh session cannot infer

- **nix IS installed** here (Determinate Nix, pkgforge installer). ⚠ Its flake
  route is broken — the harness proxy answers `api.github.com` with 403, so
  `nixpkgs#attr` fails. `nix-instantiate '<nixpkgs>' --attr X` is the route
  that works. ⛔ **That is exactly the crutch this session is removing.**
- **`pgb env create` ignores a trailing `--engine`**; the global one works:
  `sh pgb --engine chroot env create`.
- **4 cores, ~15 GiB RAM.** ⛔ **DISK IS THE BINDING CONSTRAINT**: it reached
  **1.3 GiB free** with a Qt build and a kdenlive bundle in flight. A kdenlive
  AppDir is 1.5 GiB and its closure another 3; delete `AppDir` and `store`
  under `$PGB_APPIMAGE_CACHE` as soon as an artefact is measured.
- ⛔ **DO NOT EDIT A SHELL SCRIPT WHILE IT IS RUNNING.** `sh` re-reads from a
  byte offset, so an edit mid-run corrupts the running process: it cost a
  20-minute kdenlive pack with `Syntax error: end of file unexpected`. Copy
  the tree to `/var/tmp/frozen` (keeping the repo's directory layout, because
  every tool resolves its siblings from its own path) or wait.
