# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02b, at session end
    TREE           main, clean
    BRANCH         ⛔ main. The harness named `claude/glibc-pgb-recovery-6dleai`;
                   RULES.md §Git outranks it, as the operator has ruled twice.
                   That branch was already on the remote when this session
                   started and is left alone — the git proxy refuses deletes.
    CI             green at session start (run 96); re-check the run for
                   whatever commit you start from.

---

# ⛔ TWO THINGS A FRESH SESSION CANNOT INFER AND BOTH COST TIME

⚠ **The clone comes up SHALLOW**, grafted at `21e7dc06`, which makes local
`main` look like an unrelated history with 22 orphan commits. It is not:
`git fetch --unshallow` proves `b77e0333` is a plain ancestor of `origin/main`
and `git log origin/main..main` is empty. ⛔ **Do not "recover" those commits
and do not force-push.** Unshallow, then fast-forward.

⚠ **The container is fresh: nothing is bootstrapped**, and it costs ~25
minutes. `/var/tmp` empty, `/var/lib/pgb-rootfs` absent, no `/nix`, no build
environment, dockerd not running. `./pgb bootstrap --detach` does all of it in
parallel; `./pgb bootstrap --check` says when it is ready.

    make                            builds ./pgb, ~15 s
    ./pgb selftest                  138 pass, 1 could not run (no zstd), exit 2
    make check                      selftests + both record gates, exits 0
    disk                            ~30 GiB at session start

## ⛔ WHAT IS LEFT, IN ORDER

The work order is `PROGRESS.md`; this is the short form.

1. **T-063, and it is closest to done.** ⭐ Arm S has a **static PostgreSQL
   18.6 that runs on Alpine** (no `PT_INTERP`, no `DT_NEEDED`). ⚠ What is
   missing: `src/interfaces` (libpq, ecpg) does not build, so `initdb`,
   `pg_ctl` and `psql` do not exist yet. Then arm B, then the stack actually
   serving HTTP on Debian 12 and Alpine 3.22.
   Full record: `../evidence/poc/92-miniflux/ARM-S-FINDINGS.txt`.
2. **T-062** — eight packages carry no carried selftest, `internal/wrapper`
   first. It is the product, and gate 4 is its only acceptance.
3. Then T-055, T-060 rungs 2–3, T-054 rungs 3–4, T-057 item 2, T-051.

⭐ **Two pieces of real work are named but are not entries**, because both are
one clear fix inside T-063 arm S:

    static link order    AC_SEARCH_LIBS probes -lreadline alone, so
                         libreadline.a's ncurses references go unresolved and
                         configure calls the library absent. poc/91-qt-xcb
                         answered the same class with -Wl,--start-group
    a C link that pulled libicuuc.a needs `operator delete` and the __cxxabiv1
    in a C++ archive     vtables. LinkFlags already takes a `cxx bool`; what it
                         does not do is notice this case

## ⛔ Machine notes a fresh session cannot infer

- **Go 1.24.7 at `/usr/local/go/bin/go`.** `make` builds `./pgb`; `make check`
  runs the selftests and both record gates.
- ⛔ **DISK IS THE BINDING CONSTRAINT.** ⚠ And the lesson from the session that
  died on it: what mattered was not the allowance but the leftovers. Qt needs
  6.5 GiB for its build tree alone, so **delete the previous build tree before
  starting the next big one** — `/var/tmp/pgb-poc`, `/var/tmp/pgb-appimage*`,
  `/var/tmp/t055`, `/var/tmp/pgb-nix-cache`.
- ⛔ **Never two Qt- or kdenlive-sized builds at once.**
- **Absent on a fresh container:** nix, zstd, musl-gcc, podman, codegraph, gh.
  `docker` IS present. Install codegraph with
  `sh scripts/common/install-codegraph.sh` or `TODO/check.sh` reports it absent.
- ⚠ **`experiments/90-`'s onelf arm needs a toolchain this container lacked**:
  `rustup target add x86_64-unknown-linux-musl` and `apt-get install
  musl-tools`. Without both, arm O skips.
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirecting stdout onto the
  same path collides with it and loses the run — save stdout as `run.log`
  (or `run.<app>.log`), which is what the tracked evidence already does.
- ⚠ `xz` is still shelled out by pgb; the build environment has it, a target
  rootfs does not.
- 4 cores, ~15 GiB RAM, uid 0.
- ⛔ **Never edit a shell script while it is running** — `sh` re-reads from a
  byte offset. Copy the tree aside or wait.

## In flight right now

    (nothing — every job this session started finished and its row is
     in the evidence file)
