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

⛔ **FOUR P0s were set by the operator on 2026-09-02b and outrank everything.**
Each carries the same instruction: **work until it is met or the premise is
significantly advanced.** None is a spike. `PROGRESS.md` has the work order,
`INDEX.md` the argument for it.

1. **T-064 — static glibc's `dlopen`, REALLY solved.** Our own ELF loader,
   resolving against our own static glibc. ⭐ The evidence is already in hand:
   `experiments/73-` says 90.8–97.8% of host imports are definable by it with
   **zero** unexplained residue; `experiments/72-` says the host loader can
   never work because a static binary's dynamic symbol table is empty. Restudy
   `references/pg83__solo` — `elf_loader.cpp` is 2,707 lines and most of it is
   musl translation a glibc host does not need — and beat it.
2. **T-065 — anylinux dlopens the HOST on purpose.** This tree asserts that is
   always failure; it is right for a static ELF and **wrong for a bundle**.
   `Anylinux-sharun`'s `main.rs:45` documents `SHARUN_FALLBACK_LIBRARY_PATH` as
   "lowest priority", with opt-ins for mesa, Vulkan ICDs, NVIDIA and a newer
   host glibc. Restudy the family, write the policy up, adopt it.
3. **T-066 — the bundler is bloated and slow.** 2.86× on `jq`, 2.49× and ~3×
   slower on kdenlive. ⭐ **Iterate on a CLI — bash or 7z — not on kdenlive**:
   minutes instead of twenty, and it benchmarks cleanly. The reachability sweep
   exists and **nothing consumes it**: the largest unused lever in the tree.
4. **T-067 — is C enough for `tool/runtime/`?** ⭐ A measured "C is adequate,
   here is why" **closes** it. It is a question, not a migration.

Then: T-063 (arm S has a static postgres on Alpine; `src/interfaces` does not
build), T-062, T-060, T-054, T-057, T-051, then P2.

⭐ **Two pieces of real work are named and are not entries**, because each is
one fix inside T-063 arm S:

    static link order    AC_SEARCH_LIBS probes -lreadline alone, so
                         libreadline.a's ncurses references go unresolved.
                         poc/91-qt-xcb answered the same class with
                         -Wl,--start-group
    a C link that pulled libicuuc.a needs `operator delete` and the __cxxabiv1
    in a C++ archive     vtables. LinkFlags already takes a `cxx bool`

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
