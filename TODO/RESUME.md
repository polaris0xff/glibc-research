# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02f/03, REFRESHED mid-session
    TREE           main, clean, at f4147fad
    BRANCH         ⛔ main. The harness named
                   `claude/glibc-research-foundations-7pjoqe`; RULES.md §Git
                   outranks it and THE OPERATOR SAID THE SAME in the work
                   order ("never work on any other branch than main"). The
                   local copy is deleted; the remote copy was already there at
                   main's commit and is left alone — the git proxy refuses
                   deletes.
    CI             pushed; not yet read this session.

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER, RE-CONFIRMED THIS SESSION

⚠ **The clone came up SHALLOW again, and `main` came up 18 commits BEHIND.**
`git fetch --unshallow` then `git merge --ff-only origin/main` — 173 commits
after unshallowing. ⛔ Check `git rev-list --count HEAD..origin/main` before
believing "up to date": the switch reported it before the fetch.

⚠ **The container is fresh: nothing was bootstrapped.** Again ~2 minutes, not
the ~25 the record claims.

    make                            builds ./pgb, ~15 s
    ./pgb bootstrap --detach        nix + env + bed, parallel
    sh scripts/common/install-codegraph.sh   v1.6.0, 93 files, 1,799 nodes

## ⛔ WHAT THE LAST SESSION LEFT STALE, VERIFIED BY READING THE COMMITS

⛔ **`PROGRESS.md` is 12 commits out of date** (last touched `a5619d8f`, HEAD
is `764c8544`) and `RESUME.md` was 18 behind. Both describe 2026-09-02d work
and neither carries 2026-09-02e's:

- **T-070 arm 5 is COMPLETE** — 10 of 10 POCs build and pass at glibc 2.41 —
  **and the ruling is MOVE THE PIN**, all four costs measured at zero
  (`9e0cc52e`, `7e01e0fd`). `PROGRESS.md`'s work order still says "ONE ROW
  LEFT".
- **`experiments/93-` HAS BEEN RUN** (`8266954e`, `764c8544`).
  `TODO/runtime.md` T-068 still says "93- has not been RUN yet".

## In flight right now

    ⛔ THE BED IS BUSY. A POC chain is re-running the ten POCs at the new pin,
    ONE AT A TIME, and its matrix phase uses the eleven rootfs. Do not start
    85-, 62-, 86- or a second POC beside it.
      driver  scratchpad/pocchain.sh   log  scratchpad/pocchain.log
      done    40-jq 20-nano 60-leveldb 30-curl 70-sqlite  all pass, at 2.41
      left    90-qt (running) 50-python 91-qt-xcb
      ⚠ 80-mlt and 10-gawk are NOT in the chain: 10-gawk was run first, by
        hand, and 80-mlt is hours. 80-mlt is what is owed.

    A host-side sweep of all 1527 host shared objects through the TLS-fixed
    loader is also running: scratchpad/tlsfix-sweep.sh, result lands in
    scratchpad/tlsfix-sweep.txt. It touches no rootfs.

## ⛔ WHAT IS LEFT, IN ORDER

    1  T-070 P0  steps 1 and 2 ARE LANDED (cfg.go is debian:13 / glibc 2.41 /
                 pgb-env-debian13, CI derives the image, check.sh gates all
                 three constants). Step 3 is the matrices: 73- and 21- re-run
                 and committed, six POCs re-run and committed. ⛔ 80-mlt at
                 2.41 through the normal POC path is what is left.
    2  T-068 P1  ⭐ TWO REAL LOADER DEFECTS FOUND AND FIXED this session (the
                 iconv weak-reference one and the general-dynamic TLS one).
                 ⛔ experiments/93- MUST BE RE-RUN with both -- the committed
                 RESULT.txt predates the TLS fix and its control fails at 10.
    3  T-072 P1  --tls-reserve IS implemented (the record said otherwise).
                 ⛔ Its outstanding item turned up an unexpected answer: see
                 the entry. experiments/76- with a non-zero reserve on the
                 eleven is still owed.
    4  T-071 P0  `sh experiments/85-opengl.sh` -- the data-coherence arm is
                 written and NOT RUN. That is the entry's Prove. Uses the bed.
    5  T-066 P0  the corpus is mined; measure the allowlist's ceiling first.

## ⛔ THE INCANTATION FOR A CANDIDATE ENVIRONMENT

⛔ **`PGB_ENV_NAME=... sh poc/<name>/run.sh` alone BUILDS AGAINST THE
INCUMBENT** on any machine running dockerd. All four are needed (`pgb` now
refuses without them, commit 333cb92f):

    PGB_ENGINE=chroot                 the only engine that READS a name
    PGB_ENV_NAME=pgb-env-debian-trixie
    PGB_ENV_IMAGE=debian:trixie       the stamp guard compares IMAGES, not
    PGB_ENV_DIGEST=sha256:6788062a…   names, so without these it sees a match

    debian:13 = debian:trixie   sha256:6788062a…  (COMMITTED in cfg.go now)

⭐ **That digest IS committed now** — `internal/cfg/cfg.go` is the pin, and
`debian:13` and `debian:trixie` resolved to the same manifest digest on
2026-09-02 with `experiments/91-`'s own control passing. ⚠ The incantation
above is now only for measuring a *candidate* pin; the default builds 2.41.

## ⛔ Machine notes

- 4 cores, uid 0, 15 GiB RAM, **30 GiB free at session start**. Go 1.24.7.
  Kernel `6.18.44-fc-v24` — ⚠ **`evidence/93-host-object-residue/RESULT.txt`
  was measured on `-fc-v22`, a different container with a different set of
  host objects.**
- ⛔ **`make` depends on `tool/runtime/*.c`.**
- ⛔ **DISK IS BINDING.** Delete the previous build tree before the next.
- **Absent on a fresh container:** nix→installed by bootstrap, zstd, musl-gcc,
  podman, gh, codegraph→`sh scripts/common/install-codegraph.sh`. `docker`,
  `strace`, `gcc`, `make`, `curl`, `tar`, `xz` present.
- ⚠ **Docker Hub rate-limits anonymous pulls.** `pgb rootfs pull` succeeds
  where `docker pull` 429s. `docker buildx imagetools inspect` (metadata only,
  no pull) also works.
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirect stdout elsewhere.
- ⛔ **`evidence/*/build/` is `.gitignore`d.** Copy out what you need.
- ⛔ **Never edit a shell script while it is running.**
- ⚠ **`pgrep -f "90-kdenlive"` MATCHES YOUR OWN WAITING LOOP.** Use
  `ps -eo pid,args | grep -v grep`.

## ⛔ THE RULE THE SESSION OF 2026-09-02d LEARNED THE EXPENSIVE WAY

`experiments/90-`'s render and startup arms are **wall-clock on the build
host**, so "it does not touch the bed" is **not** sufficient. ⭐ Before
starting anything, ask which shared resource it needs: counts and exit
statuses need the **bed** idle; **milliseconds need the whole machine** idle.
`RULES.md` §"the shared resource is sometimes the clock".
⛔ **A same-day `safe` vs `aggressive` kdenlive timing comparison is still
owed.**
