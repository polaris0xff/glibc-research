# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02e, at session START
    TREE           main, clean, at 72effbe5
    BRANCH         ⛔ main. The harness named
                   `claude/glibc-research-work-order-o8vn6f`; RULES.md §Git
                   outranks it and the OPERATOR SAID THE SAME in the work
                   order ("use the default main branch"). The local copy is
                   deleted; the remote copy was already there at main's commit
                   and is left alone — the git proxy refuses deletes.
    CI             not yet run this session.

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER, RE-CONFIRMED THIS SESSION

⚠ **The clone came up SHALLOW again.** `git fetch --unshallow` then
`merge --ff-only` — 174 commits after unshallowing.

⚠ **The container is fresh: nothing was bootstrapped.** ⭐ But it is FASTER
than the ~25 min the record claims on this machine: `./pgb bootstrap --detach`
had nix, the chroot build env and all 11 rootfs on disk in **~2 minutes**.
Verified by `du`, not by a marker.

    make                            builds ./pgb, ~15 s
    ./pgb bootstrap --detach        nix + env + bed, parallel
    sh scripts/common/install-codegraph.sh   v1.6.0, 93 files, 1,799 nodes

## ⛔ THE ONE THING THAT WAS LOST WITH THE CONTAINER

⛔ **`pgb-env-debian-trixie` DID NOT SURVIVE**, and the record said it
"exists". It was never reproducible from the tree: **the trixie digest was
never committed anywhere**, only re-resolved at run time by `experiments/91-`
arm 1 from a rolling registry lookup.

Re-resolved this session, with 91-'s own control passing (the method
reproduces the pinned `debian:12` digest `sha256:2f65600e…`):

    debian:trixie   sha256:6788062a1b42ac281f053ac876170b79a3eaed5d61383b8ed7eaca6c6965f3b1

⚠ **This is not provably the same image the last session measured arms 2–4
against**, because that digest was not recorded. Arm 5 must therefore be read
against a re-measured arm 2/3/4 on THIS digest, or the digest committed and
both re-run.

## In flight right now

    experiments/91-  ⛔ RUNNING, arm 5, the ten POCs against
                     pgb-env-debian-trixie. It OWNS THE BED -- do not start
                     85-, 93- or any POC beside it. Log:
                     scratchpad/exp91-full.log; per-POC logs under
                     evidence/91-glibc-pin-candidates/build/poc-*.log.
                     Green so far: 10-gawk, 20-nano, both at gcc 14.2.0.

    bootstrap is COMPLETE: nix, chroot env, docker env, 11 of 11 rootfs.
    pgb-env-debian-trixie is built: glibc 2.41, gcc 14.2.0.

## ⛔ THE INCANTATION FOR A CANDIDATE ENVIRONMENT, AND THE OLD ONE WAS WRONG

⛔ **`PGB_ENV_NAME=... sh poc/<name>/run.sh` -- what the last RESUME and the
work order both prescribed -- BUILDS AGAINST THE INCUMBENT** on any machine
running dockerd, and nothing in the POC's output says so. Caught by reading
`.comment` out of the binary: `GCC: (Debian 12.2.0-14+deb12u1)` where the
candidate carries 14.2.0. Fixed in commit 333cb92f; `pgb` now refuses it. All
four of these are needed:

    PGB_ENGINE=chroot                 the only engine that READS a name
    PGB_ENV_NAME=pgb-env-debian-trixie
    PGB_ENV_IMAGE=debian:trixie       the stamp guard compares IMAGES, not
    PGB_ENV_DIGEST=sha256:6788062a…   names, so without these it sees a match

⭐ **The loose end the work order named is ALREADY CLOSED** — commit 72effbe5,
which landed after the work order was written. The naive-vs-fast sweep
equivalence on the real kdenlive AppDir: 838 s vs 7.07 s, both exit 0, both 47
lines, `diff` exit 0. Re-verified this session against the committed files
`evidence/90-kdenlive-vs-enhanced/sweep-equivalence-{naive,fast}.txt`.
⛔ `/var/tmp/pgb-appimage-kden` and `/var/tmp/pgb-naive-sweep` are both GONE
with the container, and nothing is owed from them.

## ⛔ WHAT IS LEFT, IN ORDER

    1  T-070 arm 5   the ten POCs at glibc 2.41. ⛔ THE ENVIRONMENT HAD TO BE
                     REBUILT (see above). `PGB_PIN_POCS` + `PGB_PIN_POC_ENV`
                     turn arm 5 of experiments/91- on. Uses the bed.
    2  T-071 Prove   `sh experiments/85-opengl.sh`, data-coherence arm written
                     and unrun. Uses the bed.
    3  T-068         `sh experiments/93-host-object-residue.sh`. ⚠ It DOES
                     need the chroot build env — it builds its probe with
                     `pgb --engine chroot build --host-dlopen`. The ~900-fork
                     sweep itself is host-side.
    4  T-066         mine pkgforge-dev/archlinux-pkgs-debloated, then the
                     allowlist route.
    5  T-072         route D, designed and costed, not implemented.

## ⛔ Machine notes

- 4 cores, uid 0, **30 GiB free at session start**. Go 1.24.7.
- ⛔ **`make` depends on `tool/runtime/*.c`.**
- ⛔ **DISK IS BINDING.** Delete the previous build tree before the next.
- **Absent on a fresh container:** nix→installed by bootstrap, zstd, musl-gcc,
  podman, gh. `docker`, `strace`, `gcc`, `make`, `curl`, `tar`, `xz` present.
- ⚠ **Docker Hub rate-limits anonymous pulls.** `pgb rootfs pull` succeeds
  where `docker pull` 429s. ⭐ `docker buildx imagetools inspect` (metadata
  only, no pull) answered fine this session for both `debian:12` and
  `debian:trixie`.
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirect stdout elsewhere.
- ⛔ **`evidence/*/build/` is `.gitignore`d.** Copy out what you need.
- ⛔ **Never edit a shell script while it is running.**
- ⚠ **`pgrep -f "90-kdenlive"` MATCHES YOUR OWN WAITING LOOP.** Use
  `ps -eo pid,args | grep -v grep`.

## ⛔ THE RULE THE LAST SESSION LEARNED THE EXPENSIVE WAY

`experiments/90-`'s render and startup arms are **wall-clock on the build
host**, so "it does not touch the bed" is **not** sufficient. ⭐ Before
starting anything, ask which shared resource it needs: counts and exit
statuses need the **bed** idle; **milliseconds need the whole machine** idle.
`RULES.md` §"the shared resource is sometimes the clock".
⛔ **A same-day `safe` vs `aggressive` kdenlive timing comparison is still
owed.**
