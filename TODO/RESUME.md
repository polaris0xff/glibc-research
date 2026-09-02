# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02c, at session START (refreshed as work lands)
    TREE           main, clean, fast-forwarded to 7b6fe6e0
    BRANCH         ⛔ main. The harness named `claude/glibc-static-dlopen-kyqd5n`;
                   RULES.md §Git outranks it, as the operator has ruled twice.
                   That branch was already on the remote at main's commit when
                   this session started and is left alone — the git proxy
                   refuses deletes.
    CI             re-check the run for whatever commit you start from.

---

# ⛔ TWO THINGS A FRESH SESSION CANNOT INFER AND BOTH COST TIME

⚠ **The clone comes up SHALLOW.** Local `main` looked like an unrelated history
(`e32a50b9`, "Add files via upload") until `git fetch --unshallow`, after which
`origin/main` is a plain fast-forward, 137 commits ahead. ⛔ **Do not "recover"
the orphan commits and do not force-push.** Unshallow, then `merge --ff-only`.

⚠ **The container is fresh: nothing is bootstrapped**, and it costs ~25
minutes. `./pgb bootstrap --detach` does all of it in parallel;
`./pgb bootstrap --check` says when it is ready.

    make                            builds ./pgb, ~15 s
    ./pgb selftest                  138 pass, 1 could not run (no zstd), exit 2
    make check                      selftests + both record gates, exits 0
    disk                            29 GiB free at session start

## ⛔ WHAT IS LEFT, IN ORDER

⛔ **FOUR P0s were set by the operator on 2026-09-02b and outrank everything.**
Each carries the same instruction: **work until it is met or the premise is
significantly advanced.** None is a spike.

1. **T-064 — static glibc's `dlopen`, REALLY solved.** Our own ELF loader,
   resolving against our own static glibc. Evidence already in hand:
   `experiments/73-` (90.8–97.8% of host imports definable, residue zero),
   `experiments/72-` (the host loader can never work — a static binary's
   dynamic symbol table is empty). Restudy `references/pg83__solo` and beat it.
2. **T-065 — anylinux dlopens the HOST on purpose**, and it is right for a
   bundle. Restudy the family, write the policy up, implement the search order.
3. **T-066 — the bundler is bloated and slow.** ⭐ Iterate on a **CLI**, not
   kdenlive. The reachability sweep exists and nothing consumes it.
4. **T-067 — is C enough for `tool/runtime/`?** A measured "C is adequate"
   closes it.

Then: T-063, T-062, T-060, T-054, T-057, T-051, then P2.

## ⛔ Machine notes a fresh session cannot infer

- **Go 1.24.7 at `/usr/local/go/bin/go`.** `make` builds `./pgb`; `make check`
  runs the selftests and both record gates.
- ⛔ **DISK IS THE BINDING CONSTRAINT**, and the lesson is LEFTOVERS not
  allowance: delete the previous build tree before the next big one —
  `/var/tmp/pgb-poc`, `/var/tmp/pgb-appimage*`, `/var/tmp/t055`,
  `/var/tmp/pgb-nix-cache`. ⛔ Never two Qt- or kdenlive-sized builds at once.
- **Absent on a fresh container:** nix, zstd, musl-gcc, podman, codegraph, gh.
  `docker` IS present. `sh scripts/common/install-codegraph.sh`.
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirect stdout to
  `run.log`, never onto `RESULT.txt` — they collide and the run is lost.
- ⛔ **Never edit a shell script while it is running** — `sh` re-reads from a
  byte offset.
- ⚠ Use a heredoc for commit messages, never `git commit -m` with backticks.
- 4 cores, ~15 GiB RAM, uid 0.

## In flight right now

    ./pgb bootstrap --detach   started at session open, ~25 min, log at
                               /var/tmp/pgb-bootstrap/bootstrap.log.
                               `./pgb bootstrap --check` reports readiness.
    T-064                      reading the evidence and references/pg83__solo
                               before writing the loader. Nothing built yet.
