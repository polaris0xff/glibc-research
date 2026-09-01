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

    T-058 CLOSED   experiments/87-, 8 assertions, the control reproduces the
                   old behaviour 5 of 5. pgb build is concurrency-safe now.
    T-050 CLOSED   experiments/88-, 25 assertions. hydra's job API is the
                   name->derivation index; 19 of 20 against Deriver's 9 of 20,
                   drvpaths byte-identical to nix-instantiate's.
    T-051 step 1   jq planned, fetched AND BUILT at uid 12000 with no nix and
                   no /nix. What is left is a host with NO COMPILER = T-060.
    T-060 NEXT     ⭐ static-glibc nix, three rungs. This is the flagship.

## ⛔ Machine notes a fresh session cannot infer

- **nix IS installed** here (Determinate Nix, pkgforge installer). ⚠ Its flake
  route is broken — the harness proxy answers `api.github.com` with 403, so
  `nixpkgs#attr` fails. `nix-instantiate '<nixpkgs>' --attr X` is the route
  that works. ⛔ **That is exactly the crutch this session is removing.**
- **`pgb env create` ignores a trailing `--engine`**; the global one works:
  `sh pgb --engine chroot env create`.
- **4 cores, ~15 GiB RAM, ~25 GiB free disk.** Watch the disk: a qtbase tree
  is several GiB and nothing under `/var/tmp` survives the machine.
