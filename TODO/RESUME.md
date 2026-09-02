# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02d, refreshed at session END
    TREE           main, clean
    BRANCH         ⛔ main. The harness named
                   `claude/glibc-kdenlive-validation-2x7c3c`; RULES.md §Git
                   outranks it, as the operator has ruled twice. That branch
                   was already on the remote at main's commit when this
                   session started and is left alone — the git proxy refuses
                   deletes.
    CI             green on every commit of this session.

---

# ⛔ TWO THINGS A FRESH SESSION CANNOT INFER AND BOTH COST TIME

⚠ **The clone comes up SHALLOW.** `git rev-parse --is-shallow-repository`
returned `true` again this session, and `origin/main` came back as a **forced
update** from a stale commit. ⛔ Do not "recover" the orphan commits and do not
force-push. `git fetch --unshallow`, then `merge --ff-only`.

⚠ **The container is fresh: nothing is bootstrapped**, and it costs ~25
minutes. `./pgb bootstrap --detach` does all of it in parallel;
`./pgb bootstrap --check` says when it is ready.

    make                            builds ./pgb, ~15 s
    ./pgb selftest                  200 pass, 1 could not run (no zstd), exit 2
    make check                      selftests + both record gates, exits 0
    disk                            ~15 GiB free at session end

## In flight right now

    (nothing running)

⭐ **The loose end from earlier in this session is CLOSED.** The naive-vs-fast
sweep equivalence, on the real kdenlive AppDir — 1,633 libraries, 1.49 GiB,
2,586 roots:

    naive   838 s     exit 0, 47 lines
    fast      7.07 s  exit 0, 47 lines
    diff    IDENTICAL

Both outputs are kept:
`evidence/90-kdenlive-vs-enhanced/sweep-equivalence-naive.txt` and
`evidence/90-kdenlive-vs-enhanced/sweep-equivalence-fast.txt`.
⚠ The naive arm carried concurrent load, so "about 100×" is the honest claim
and 118× is the arithmetic.

## ⛔ WHAT IS LEFT, IN ORDER — and every one of these is READY TO RUN

    1  T-070 arm 5   the ten POCs at glibc 2.41. The environment EXISTS:
                     /var/lib/pgb-rootfs/pgb-env-debian-trixie, full package
                     list, gcc 14.2.0. `PGB_ENV_NAME=pgb-env-debian-trixie
                     sh poc/<name>/run.sh`. Uses the bed. ⛔ THIS IS THE ONE
                     ROW between "indicated" and "measured" for the pin move;
                     the other three costs are all ZERO.
    2  T-071 Prove   `sh experiments/85-opengl.sh`. The data-coherence arm is
                     written; the Prove was already carried out by hand on
                     kdenlive's AppDir, so this is confirmation on 85-'s own
                     subject. Uses the bed.
    3  T-068         `sh experiments/93-host-object-residue.sh`. Written, its
                     probe verified, NOT RUN. No bed; ~900 forks.
    4  T-066         mine pkgforge-dev/archlinux-pkgs-debloated (NOT in
                     references/), then the allowlist route. See PROGRESS.
    5  T-072         route D, designed and costed, not implemented.

## ⛔ Machine notes a fresh session cannot infer

- **Go 1.24.7 at `/usr/local/go/bin/go`.** `make` builds `./pgb`; `make check`
  runs the selftests and both record gates.
- ⛔ **`make` depends on `tool/runtime/*.c`.** It did not once, and that
  shipped a stale loader through a whole 11-environment run.
- ⛔ **DISK IS THE BINDING CONSTRAINT**, and the lesson is LEFTOVERS not
  allowance: delete the previous build tree before the next big one.
  `/var/tmp/pgb-appimage-kden` is **7 GB** and is now free to delete: the
  equivalence diff it was being kept for is done.
- **Absent on a fresh container:** nix, zstd, musl-gcc, podman, codegraph, gh.
  `docker` IS present. `sh scripts/common/install-codegraph.sh`.
- ⚠ **`musl-gcc` is the one remaining blocker on `experiments/90-`'s arm O.**
  The rust `x86_64-unknown-linux-musl` target was the first and is installed.
- ⚠ **Docker Hub rate-limits anonymous pulls here.** ⭐ `pgb rootfs pull` does
  the anonymous-token dance and succeeds where `docker pull` 429s.
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirect stdout to a
  separate log, never onto `RESULT.txt`.
- ⛔ **`evidence/*/build/` is `.gitignore`d**, so `debloat`, sweep and
  `icd json` totals are overwritten by the next run. Copy what you need out.
- ⛔ **Never edit a shell script while it is running.**
- ⚠ **`pgrep -f "90-kdenlive"` MATCHES YOUR OWN WAITING LOOP.** Use
  `ps -eo pid,args | grep -v grep`.
- ⚠ Use a heredoc for commit messages, never `git commit -m` with backticks.
- 4 cores, ~15 GiB RAM, uid 0.

## ⛔ THE RULE THIS SESSION LEARNED THE EXPENSIVE WAY

`experiments/90-`'s render and startup arms are **wall-clock on the build
host**, so "it does not touch the bed" is **not** sufficient. Running `go
build`, `pgb selftest` and the gates during them moved the **competitor's
fixed artefact** from 2,033 ms to 13,680 ms. ⭐ Before starting anything, ask
which shared resource the experiment needs: counts and exit statuses need the
**bed** idle; **milliseconds need the whole machine** idle.
`RULES.md` §"the shared resource is sometimes the clock".
