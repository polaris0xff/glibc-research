# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02, at session start (recovery session)
    TREE           main, clean, at e44a6519
    BRANCH         ⛔ main. The harness named `claude/glibc-pgb-recovery-6dleai`;
                   RULES.md §Git outranks it, as the operator has ruled twice.
                   That branch already exists on the remote at the same commit
                   and is left alone — the git proxy refuses remote deletes.
    CI             ⭐ GREEN. Run 96 on e44a6519, workflow `portability`.

---

# ⛔ THE PREVIOUS SESSION DIED ON DISK. THIS ONE IS A RECOVERY.

⚠ The session before this one terminated itself after `poc/91-qt-xcb` filled
the disk at Qt object 1,538 of 1,644 (`cannot write PCH file: No space left on
device`). It had already pushed everything it did; **nothing was lost**.

⚠ **The clone came up SHALLOW**, grafted at `21e7dc06`, which makes local
`main` look like an unrelated history with 22 orphan commits. It is not:
`git fetch --unshallow` proves `b77e0333` is a plain ancestor of `origin/main`
and `git log origin/main..main` is empty. ⛔ **Do not "recover" those commits;
do not force-push.** Unshallow first, then fast-forward.

# ⛔ THE CONTAINER IS FRESH. NOTHING IS BOOTSTRAPPED.

⚠ **A fresh session cannot infer this and it costs ~25 minutes.** This
container came up with `/var/tmp` **empty**, `/var/lib/pgb-rootfs` absent, no
build environment, no `/nix`, and dockerd not running.

    make                            ✅ builds ./pgb, 15 s
    ./pgb selftest                  ✅ 123 pass, 1 could not run (no zstd), exit 0
    sh TODO/check.sh                ✅ exit 0 (codegraph absent — install it)
    sh scripts/common/check-docs.sh ✅ exit 0
    disk                            30 GiB available at session start

## ⛔ WHAT IS LEFT, IN ORDER

The work order is `PROGRESS.md`; this is only the short form.

1. **Gate 5's three missing rows** — `poc/91-qt-xcb`, `experiments/86-`,
   `experiments/90-`. All three need the bed and several GiB of free disk.
   ⛔ Never two Qt- or kdenlive-sized builds at once.
2. **`experiments/90-`'s onelf row is the wrong one.** Its argv[0] defect was
   fixed and the three-arm run has not been repeated since.
3. **The operator's post-port instruction.** Codegraph is wired into the gates
   and the rules (e44a6519) but is **not installed in this container**: run
   `sh scripts/common/install-codegraph.sh`. Then the deprecation sweep of the
   Go tree, two deep reviews of code and docs, unreferenced documents retired
   to `HISTORY/`, and a `docs/AGENTS.md` a session with no memory can start
   from alone.
4. **The miniflux proof** — miniflux plus an embedded PostgreSQL, its
   `dlopen`'d extensions and its share tree, against onelf's ~70 MB. It lands
   as POC 92 and a new `TODO/` entry.
5. Then the backlog: T-055, T-060 rungs 2–3, T-054 rungs 3–4, T-057 item 2,
   T-051, then P2 by category.

## ⛔ Machine notes a fresh session cannot infer

- **Go 1.24.7 is at `/usr/local/go/bin/go`.** `make` builds `./pgb`;
  `make check` runs the selftests and both record gates.
- ⛔ **DISK IS THE BINDING CONSTRAINT.** The allowance is per-session, so `df`
  reads a large filesystem with a small "Avail". Delete under
  `/var/tmp/pgb-poc`, `/var/tmp/pgb-appimage*` and `/var/tmp/pgb-nix-cache`
  before starting anything kdenlive- or Qt-sized.
- **nix, zstd, musl-gcc, podman and codegraph are ABSENT** on a fresh
  container. `docker` IS present. `gh` is absent — the harness's GitHub MCP
  tools are the authenticated route.
- ⚠ `xz` is still shelled out by pgb; the build environment has it and a
  target rootfs does not.
- 4 cores, ~15 GiB RAM, uid 0.
- ⛔ **Never edit a shell script while it is running** — `sh` re-reads from a
  byte offset. Copy the tree to `/var/tmp/frozen-<what>` or wait.

## In flight right now

    ./pgb bootstrap --detach     started at session start
                                 log: /var/tmp/pgb-bootstrap/bootstrap.log
                                 check: ./pgb bootstrap --check
