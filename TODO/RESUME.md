# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03, at session END
    TREE           main, clean
    BRANCH         ⛔ main. The harness named
                   `claude/glibc-research-foundations-7pjoqe`; RULES.md §Git
                   outranks it and THE OPERATOR SAID THE SAME ("never work on
                   any other branch than main"). The local copy is deleted; the
                   remote copy was already there and is left alone — the git
                   proxy refuses deletes.
    CI             ⭐ GREEN, 16 of 16, at the NEW pin (run 33699204833).
                   ⚠ It was RED for seven pushes before that — see below.

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` can come up BEHIND.** `git switch
main` printed *"Your branch is up to date"* and `git rev-list --count
HEAD..origin/main` then said **18**. Do this, in this order:

    git fetch --unshallow
    git rev-list --count HEAD..origin/main     ⛔ check it, do not assume
    git merge --ff-only origin/main

⚠ **The container is fresh: nothing is bootstrapped.** ~2 minutes, not the ~25
the record used to claim.

    make                                     builds ./pgb, ~15 s
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0

## In flight right now

    ⛔ THE BED IS BUSY, driven by scratchpad/bedqueue.sh (log:
    scratchpad/bedqueue.log). ONE AT A TIME:

      1  experiments/85-opengl.sh   T-071's Prove. STARTED 00:36, building two
                                    ~400 MB bundles. Log: scratchpad/exp85.log
      2  poc/80-mlt/run.sh          the tenth POC at glibc 2.41. Hours.

    ⛔ IF THE CONTAINER DIED, BOTH ARE LOST AND NEITHER IS COMMITTED. Re-run:
      sh experiments/85-opengl.sh
      sh poc/80-mlt/run.sh > evidence/poc/80-mlt/RESULT.txt 2>&1

    ⚠ Everything else this session did IS committed and pushed.

## ⛔ WHAT IS LEFT, IN ORDER

    1  T-070 P0  steps 1-3 landed and CI is green at debian:13. ⛔ ONLY
                 80-mlt is left; close the entry when it passes.
    2  T-071 P0  read scratchpad/exp85.log. That is the entry's Prove and it
                 has been "written and not run" for three sessions.
    3  T-072 P1  experiments/76- with a non-zero --tls-reserve on the eleven.
                 ⚠ Read the entry first: the object that motivated it is
                 refused for a DIFFERENT reason and the benefit measured on
                 real host objects is currently ZERO objects.
    4  T-066 P0  the corpus is mined; measure the allowlist's ceiling first.
                 Needs an AppDir, and a 7 GB one did not survive a container.
    5  T-062 P1  five packages still carry no selftest. verifyx and buildx
                 shell out to a bed — carry their parsing, not their run.

## ⭐ WHAT THIS SESSION CHANGED THAT YOU WILL TRIP OVER

- ⛔ **The glibc pin MOVED**: `debian:12`/2.36 → **`debian:13`/2.41**, gcc
  12.2.0 → **14.2.0**, environment `pgb-env-debian13`. Three constants in
  `internal/cfg/cfg.go` and **nowhere else** — `TODO/check.sh` fails on a copy.
  ⚠ A `pgb-env-debian12` on disk from an older container is stale; `pgb build`
  refuses against it by name and says so.
- ⭐ **`experiments/lib.sh` now defines `ENV_NAME` and `ENV_ROOT`** from
  `cfg.go`. Do not retype the name in an experiment.
- ⭐ **A POC's `RESULT.txt` now opens with the environment that built it**, and
  `poc_inspect` ASSERTS the binary's `.comment` gcc against it.
- ⭐ **`pgb selftest` is 307 cases** (was 200): `wrapper-flags` and `cfg` are new.

## ⛔ THE TRAPS THIS SESSION PAID FOR

- ⛔ **`pgb rootfs run` MOUNTS A FRESH TMPFS OVER `/tmp`.** A file copied there
  from outside is not there inside. Use `--bind` or `--copy`.
- ⛔ **`$?` after a pipeline is the PIPELINE's status.** A sweep reported
  `ok=1527 fail=0 crash=0` over a population containing 96 files that are not
  ELF, because `rc=$?` came after `| head -1`.
- ⛔ **`chmod 000` is not a control when you are root.** Move the file away.
- ⛔ **`check-docs.sh` used to ask the DISK.** A gitignored build product passed
  here and failed in CI. It asks the repository now — but the lesson stands:
  ⭐ **read the CI run, do not assume a local gate speaks for it.**
- ⚠ **A gate cannot catch a stale NUMBER.** Three documents quoted `ok=628`
  after the tree moved to 882; only re-reading the claims against the evidence
  file found it.

## ⛔ Machine notes

- 4 cores, uid 0, 15 GiB RAM, ~30 GiB free at session start. Go 1.24.7,
  kernel `6.18.44-fc-v24`.
- ⛔ **`make` depends on `tool/runtime/*.c`.** Rebuild after touching the loader.
- ⛔ **DISK IS BINDING.** Delete the previous build tree before the next; the
  POC chain does `rm -rf /var/tmp/pgb-poc/*` between subjects.
- **Absent on a fresh container:** nix→bootstrap installs it, zstd, musl-gcc,
  podman, gh, codegraph. `docker`, `strace`, `gcc`, `make`, `curl`, `tar`, `xz`
  present.
- ⚠ **Docker Hub rate-limits anonymous pulls.** `pgb rootfs pull` succeeds where
  `docker pull` 429s; `docker buildx imagetools inspect` reads metadata without
  pulling.
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirect stdout elsewhere.
  ⚠ **A POC does NOT** — `poc/common.sh` says so: redirect, or its RESULT.txt
  describes the previous run.
- ⛔ **`evidence/*/build/` is `.gitignore`d.** Copy out what you need.
- ⛔ **Never edit a shell script while it is running.**

## ⛔ THE RULE ABOUT THE SHARED RESOURCE

⭐ Before starting anything, ask which resource it needs: counts and exit
statuses need the **bed** (the eleven rootfs) idle; **milliseconds need the
whole machine** idle. `RULES.md` §"the shared resource is sometimes the clock".
⚠ A host-side sweep and a `pgb build` can overlap; two things touching one
rootfs cannot.
⛔ **A same-day `safe` vs `aggressive` kdenlive timing comparison is still
owed.**
