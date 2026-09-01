# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-01, session end
    COUNTS    19 entries, 12 open, 7 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, five POCs
              CI: GREEN, 15 jobs, and it now asserts criterion 2
              chroot and docker engines produce BYTE-IDENTICAL binaries
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
              pgb over plain gcc -static, same workloads: 0.99x-1.05x

## What this session did

⛔ **It started by finding that three tracked files were wrong about
observable facts**, and `docs/AGENTS.md` — the first file a new session is
told to read — was one of them.

- **T-040 done. CI was not unrun; it had run 10 times and been red 10 times.**
  Nine of eleven rows were green the whole time and nobody had collected the
  result. The two red rows never executed a binary: GitHub's own dynamically
  linked Node.js cannot start in a musl container, which is this project's
  thesis observed on the CI provider. Rebuilt to run every job on the host and
  enter targets with `docker run --entrypoint`, and to **generate** its matrix
  from `scripts/common/rootfs-images.txt` — runs 1–10 hand-wrote tags, so CI
  and the local bed were two different beds reporting as one.
  `history/corrections.md` C8.
- **T-010 done. `pgb` split 813 → 141 lines** plus five sourced libraries.
  Proved by eight commands with byte-identical output and equal exit codes,
  and by running all three engines, which a byte comparison cannot reach.
  ⭐ **chroot and docker produce byte-identical binaries** — the engines are
  interchangeable, not merely both working.
- **T-011 done, by measurement rather than argument.** The entry's own premise
  was flagged untested; `experiments/70-` tested it. A carried-in static Rust
  helper runs on **12 of 12** targets, exactly where `sh` does. The driver
  stays POSIX `sh`, but "the alternative loses on bootstrap" is **withdrawn**
  and the planner is no longer blocked from being a real language.
- **T-014 done.** It was opened this session on finding `pgb verify` ignored
  `--engine` entirely, so the tool's own verification command could not run on
  a runner at all. Now green on one, criterion 2 included, via a `ptrace`
  tracer **carried into** the container — the thing `experiments/70-` had just
  measured was possible.
- **T-030's mechanism landed, entry still open.** `--wrap-dlopen` answers a
  program's own `dlopen`/`dlsym`/`dlclose`/`dlerror` from a table `pgb`
  generates with `nm`. 11 of 11, zero host objects, +544 bytes. ⛔ Open
  because its `Prove` names CPython and CPython is not rebuilt yet.
- **The docker engine was never "untested" for the recorded reason.** This
  machine has docker and no init; nothing had started `dockerd`. One line did,
  and the first ten minutes found three defects — including a `pgb build` that
  produced no output and **exited 0**. `history/corrections.md` C9.
- **Two new entries from what the work turned up:** T-015 (`oci-pull.sh` drops
  the image config, so the two beds are not the same environment) and T-014
  above.
- **Rules:** the two fetch routes (`api.gh.pkgforge.dev`,
  `api.rv.pkgforge.dev`) are written down and verified rather than left as
  folklore in a provenance file. `RULES.md`.
- **Vendored** `Aseem0xff/alloc-tests` for its container operations. Its
  `docs/AGENTS.md` and its own 61 MiB nested corpus were deleted, both
  recorded. ⚠ The branch it was read from was **gone within the hour** — the
  pinned copy is what survived.

⭐ **Six defects in this tree were found and fixed, and every one of them was
the kind that reads as success**: a build that produced nothing and exited 0;
a tracer that reported a clean binary because it had failed to attach; a
tracer that counted paths merely probed for; a tracer that hung forever on
exactly the binaries `verify` exists to catch; `die()` printing its exit code
into its own message; a backtick in an unquoted heredoc executing `nm` during
`pgb explain`.

## In progress

⚠ **T-030's CPython arm.** The mechanism is landed and measured; the entry's
acceptance is POC 50's CPython rebuilt on it. The entry names exactly what is
left and the two unknowns, neither ruled out.

## Work order

    T-030 (finish)             CPython on --wrap-dlopen; the entry has the plan
    T-001  T-002  T-003        harder POCs, until something breaks
    T-012                      pgb build <spec> -- split it first, it is XL
    T-041                      aarch64
    then P2 by category

⭐ **T-010 and T-011 are done, so the argument in `INDEX.md` for putting them
before the POCs is spent.** What replaces it: T-030's remaining arm is the
cheapest thing that also serves T-002, because "a program that dlopens its own
plugins at scale" and "CPython on `--wrap-dlopen`" are the same build.

## Open questions for the operator

1. ⛔ **`REQUIREMENTS.md` part 2 is not met and the reason has not changed.**
   `pgb` is not beaten on portability or throughput by anything measured — it
   ties the anylinux AppImage — but it does not *beat* it, and it is behind on
   the class of software each can serve. ⚠ `--wrap-dlopen` narrowed that gap
   this session and did not close it: it serves a program's **own** plugins,
   not host plugins. Either `pgb` grows to reach that class, or "strictly
   better than every existing format" is replaced. ⛔ **That is the operator's
   call and an agent must not make it.**
2. **Is a nixpkgs front end (T-022) in scope**, or does depending on nix defeat
   the point? T-020 argues the graph is worth taking and the store layout is
   not.
3. **T-015 changes what the bed is.** Applying an image's `Env` would make the
   chroot bed match `docker run`, and would also change what every
   locale-sensitive result describes. The entry says it lands with those
   experiments re-run or behind a flag; which one is a judgement about how much
   the existing numbers are worth.
