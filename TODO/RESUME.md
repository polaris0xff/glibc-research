# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02c, refreshed after T-064 closed
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

    (nothing running)

## What this session has done so far

⭐ **T-064 is CLOSED and it was the first P0.** `tool/runtime/pgb-elfload.c` is
an ELF loader compiled into the binary; `pgb build --host-dlopen` turns it on;
`experiments/76-` measures it on all eleven, exit 0, four of four assertions:

    carried: nine assertions pass, every environment     = 11 of 11
    carried: loaded no host shared object, every one     = 11 of 11
    native:  loads a real host object on every glibc row  = 7 of 7
    native:  refuses CLEANLY on every musl row, no signal = 4 of 4
    control: ran                                          = 0 of 11

⭐ On the four musl rows that is a GLIBC `.so` dlopen'd on a machine with no
glibc. 1,093 code lines against solo's 2,332 for the loader alone. T-068 is
new and carries the residue (86 of 904 host objects) so it is not rounded off.

⛔ **Five defects found, each by something disagreeing, never by reading:**

    libm.a is a GNU ld SCRIPT, not an archive -- read as `ar` it is zero
      symbols in silence. Second time this trap has fired here.
    __tls_get_addr is in no archive; ld.so exports it. 398 of 492
      undefined-symbol failures were that one name.
    DT_RELR was ignored -- Fedora and Arch pack relative relocations into a
      bitmap, so the loader "succeeded" and left pointers unrelocated. A
      SILENT wrong answer, caught only because a constructor was called
      through one.
    make did not depend on the go:embed'd C, so editing the loader printed
      "Nothing to be done" and the next build used the PREVIOUS loader. It
      cost a full eleven-environment run.
    my own benchmark forked per sample and reported the loader 10x slower
      than ld.so; that was copy-on-write faults on a 4.4 MB static image.

## ⛔ WHAT IS LEFT, IN ORDER

    T-065   ⛔ NEXT. anylinux dlopens the HOST on purpose and this tree
            asserts that is always failure. Right for a static ELF, WRONG
            for a bundle. Restudy Anylinux-sharun, Anylinux-AppImages,
            VHSgunzo__sharun, runimage, nixGL and the trackers; write the
            policy up; implement the search order -- bundled-first with a
            documented lowest-priority host fallback and per-class opt-ins
            for mesa, Vulkan ICDs, NVIDIA and a newer host glibc.
    T-066   ⛔ the bundler is bloated and slow. ⭐ Iterate on a CLI (bash or
            7z), NOT kdenlive. The reachability sweep exists and NOTHING
            consumes it -- the largest unused lever in the tree.
    T-067   ⛔ is C enough for tool/runtime/? ⭐ A measured "C is adequate,
            here is why" CLOSES it. ⚠ pgb-elfload.c is 1,093 new lines of C
            written this session and is the natural subject.
    T-068   the --host-dlopen residue, classified in docs/limitations.md §1
    then    T-063, T-062, T-060, T-054, T-057, T-051, then P2
