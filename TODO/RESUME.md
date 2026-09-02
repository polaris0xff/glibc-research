# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02, checkpoint + docs review (operator interrupt)
    TREE           main, committing as the review lands
    BRANCH         ⛔ main. The harness named `claude/glibc-nix-static-v2nttp`
                   and RULES.md §Git outranks it. `git ls-remote --heads
                   origin` lists refs/heads/main ONLY.

---

| | |
|---|---|
| **the task** | Operator interrupt, 2026-09-02: *"stop, save current progress and checkpoint; our docs have been stale for many commits now — do a comprehensive docs review, bring everything to date, ensure consistent, correct, concise; update the tasks etc; ensure you do 3 mandated deep reviews as part of the end session protocols."* The measurement work is **paused, not abandoned** — its resume point is below. |
| **the resume point** | ⭐ **Shrink the kdenlive bundle.** The reachability sweep says **488,934,276 bytes of `AppDir/lib` (2,300 files, 39% of `lib/`) is not reachable** from the four programs or any plugin directory. That is the route to the operator's bar and it is measured, not guessed. Then re-run `experiments/90-` with all three arms. |
| **⛔ do not parallelise** | `RULES.md` §"one thing at a time on the bed". Two `pgb build`s may overlap (T-058 closed); two things touching one rootfs may not. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## In flight

    RUNNING  T-060 rung 1, the nix closure, from the FROZEN tool copy:
             sh /var/tmp/pgb-t060/rung1.sh -> /var/tmp/pgb-t060/rung1e.log
             31 of nix's external dependencies built into
             /var/tmp/pgb-t060/prefix; boost is the long pole (~25 min).
             ⚠ Idempotent: re-running skips what `.built/` already names.

    OPEN     T-060 rungs 2 (link nix against that prefix) and 3 (run it in a
             rootfs with no nix) are untouched.

    OPEN     T-055: ours 395,294,317 B vs the competitor's 191,900,604 B.
             The 489 MB unreachable-library figure above is the first cut.

## ⛔ Machine notes a fresh session cannot infer

- **nix IS installed** here (Determinate Nix, pkgforge installer). ⚠ Its flake
  route is broken — the harness proxy answers `api.github.com` with 403, so
  `nixpkgs#attr` fails. `nix-instantiate '<nixpkgs>' --attr X` is the route
  that works. ⛔ **That is exactly the crutch T-060 is removing.**
- **`pgb env create` ignores a trailing `--engine`**; the global one works:
  `sh pgb --engine chroot env create`.
- **4 cores, ~15 GiB RAM.** ⛔ **DISK IS THE BINDING CONSTRAINT**: 7–9 GiB
  free with one AppDir on disk. A kdenlive AppDir is 1.9 GiB apparent and its
  closure another 3; delete `AppDir` and `store` under `$PGB_APPIMAGE_CACHE`
  as soon as an artefact is measured.
- ⛔ **DO NOT EDIT A SHELL SCRIPT WHILE IT IS RUNNING.** `sh` re-reads from a
  byte offset, so an edit mid-run corrupts the running process: it cost a
  20-minute kdenlive pack with `Syntax error: end of file unexpected`. Copy
  the tree to `/var/tmp/frozen-<what>` keeping the repo's directory layout —
  every tool resolves its siblings from its own path — or wait.
