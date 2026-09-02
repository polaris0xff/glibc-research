# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02, at session start
    TREE           main, clean, at f6b5f600
    BRANCH         ⛔ main. The harness named `claude/glibc-research-pgb-cw5bix`;
                   RULES.md §Git outranks it, as the operator has ruled twice.
                   The harness branch exists on the remote at main's commit and
                   is left alone — the git proxy refuses remote deletes.

---

# ⛔ THE CONTAINER IS FRESH. NOTHING IS BOOTSTRAPPED.

⚠ **A fresh session cannot infer this and it costs ~25 minutes.** This
container came up with `/var/tmp` **empty**, `/var/lib/pgb-rootfs` absent, no
build environment, no `/nix`, and dockerd not running. Everything that needs
the bed — every POC, every experiment, `pgb verify` — is blocked until
`pgb bootstrap` finishes.

    make                            ✅ builds ./pgb, 12 s
    ./pgb selftest                  ✅ 123 pass, 1 could not run (no zstd), exit 0
    ./pgb doctor                    chroot engine usable; 0 of 11 rootfs present
    disk                            30 GiB available at session start

## ⛔ WHAT IS LEFT, IN ORDER

1. **Gate 5's three missing rows** — `poc/91-qt-xcb`, `experiments/86-`,
   `experiments/90-`. All three need the bed and several GiB of free disk.
2. **`experiments/90-`'s onelf row is the wrong one.** Its argv[0] defect was
   fixed and the three-arm run has not been repeated since.
3. **The operator's post-port instruction**: codegraph installed and wired
   into the gates and the rules, a deprecation sweep of the Go tree, two deep
   reviews of code and docs, unreferenced documents retired to `HISTORY/`, and
   a `docs/AGENTS.md` a session with no memory can start from alone.
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
- **nix, zstd, musl-gcc, podman and codegraph are ABSENT.** `gh` is absent
  too; the harness's GitHub MCP tools are the authenticated route.
- ⚠ `xz` is still shelled out by pgb; the build environment has it and a
  target rootfs does not.
- 4 cores, ~15 GiB RAM, uid 0.
- ⛔ **Never edit a shell script while it is running** — `sh` re-reads from a
  byte offset. Copy the tree to `/var/tmp/frozen-<what>` or wait.

## In flight right now

    (nothing yet — session just started)
