# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02d, at session START
    TREE           main, clean, fast-forwarded to 590ed8a1
    BRANCH         ⛔ main. The harness named
                   `claude/glibc-kdenlive-validation-2x7c3c`; RULES.md §Git
                   outranks it, as the operator has ruled twice. That branch
                   was already on the remote at main's commit when this
                   session started and is left alone — the git proxy refuses
                   deletes.
    CI             re-check the run for whatever commit you start from.

---

# ⛔ TWO THINGS A FRESH SESSION CANNOT INFER AND BOTH COST TIME

⚠ **The clone comes up SHALLOW.** `git rev-parse --is-shallow-repository`
returned `true` again this session, and `origin/main` came back as a **forced
update** from a stale `b77e0333`. ⛔ Do not "recover" the orphan commits and do
not force-push. `git fetch --unshallow`, then `merge --ff-only`.

⚠ **The container is fresh: nothing is bootstrapped**, and it costs ~25
minutes. `./pgb bootstrap --detach` does all of it in parallel;
`./pgb bootstrap --check` says when it is ready.

    make                            builds ./pgb, ~15 s
    ./pgb selftest                  138 pass, 1 could not run (no zstd), exit 2
    make check                      selftests + both record gates, exits 0
    disk                            30 GiB free at session start

## In flight right now

    (session start — bootstrap and codegraph install running in background)

## ⛔ FIRST: kdenlive is STILL UNVALIDATED, FOUR RUNS IN

`sh experiments/90-kdenlive-vs-enhanced.sh` (~25 min, needs the bed to itself).

    run 1  the sweep ran BEFORE writeEnv wrote .env, so MLT's modules were
           deleted. `ours rendered 0 of 11`. Fixed by ordering.
    run 2  the experiment REUSED the cached artefact, so the fix was never
           exercised -- it reported 267,390,365 B to the digit. Fixed: it
           now rebuilds when ./pgb is newer than the artefact.
    run 3  `melt`: "Failed loading SDL3 library." libSDL3.so.0 is dlopen'd
           BY NAME from inside an MLT module. Fixed: soname-string roots,
           and sweep deletion moved to `aggressive`.
    run 4  ⛔ INVALID, AND THE CAUSE WAS THE AGENT'S OWN KILL. The run was
           stopped mid-pack; build-ours.log ends "packing with uruntime +
           dwarfs" / "Terminated", so the 471,020,146-byte artefact was a
           TRUNCATED AppImage. It says NOTHING about the code.

⭐ **Run 3's fixes have never been tested end to end.** ⛔ Do not kill the run;
if you must stop, delete `/var/tmp/pgb-appimage-kden/kdenlive/*.AppImage`
afterwards, because the staleness rule reuses a non-empty artefact older than
`./pgb`.

## ⛔ WHAT IS LEFT, IN ORDER (reordered 2026-09-02c by the operator)

    T-070  P0  ⛔ THE GLIBC PIN. The only thing here that gets WORSE by being
               left alone: class B is 20 symbols, 14 at exactly GLIBC_2.38,
               and it widens each release. Pin 2.36, floor 2.34, output
               STATIC so there is no upward pressure. ⛔ Measure the kernel
               floor a newer glibc declares BEFORE moving.
    T-071  P0  EGL from a nixpkgs closure. Four failures, every one in DATA.
               ⛔ Known half-fix: implicit_layer.d/explicit_layer.d are in
               the SWEEP's globs but not the REWRITE's.
    T-068  P1  the loader residue, 86 of 904; crashes 30 -> 10 and the 10 are
               ONE family, large C++ libraries. libLLVM dies in the 605th
               static constructor.
    T-072  P1  static TLS surplus: 3,176 bytes of headroom, one real library
               wants 56,248. Three untried routes.
    T-066  P0  the bundler. Remaining gap is WHERE THE CLOSURE COMES FROM.
    then   T-063, T-062, T-060, T-054, T-057, T-051, T-012, then P2.

## ⛔ Machine notes a fresh session cannot infer

- **Go 1.24.7 at `/usr/local/go/bin/go`.** `make` builds `./pgb`; `make check`
  runs the selftests and both record gates.
- ⛔ **`make` NOW depends on `tool/runtime/*.c`.** It did not, and that shipped
  a stale loader through a whole 11-environment run.
- ⛔ **DISK IS THE BINDING CONSTRAINT**, and the lesson is LEFTOVERS not
  allowance: delete the previous build tree before the next big one —
  `/var/tmp/pgb-poc`, `/var/tmp/pgb-appimage*`, `/var/tmp/t055`,
  `/var/tmp/pgb-nix-cache`. `/var/tmp/pgb-appimage-kden` is 6 GB.
  ⛔ Never two Qt- or kdenlive-sized builds at once.
- **Absent on a fresh container:** nix, zstd, musl-gcc, podman, codegraph, gh.
  `docker` IS present. `sh scripts/common/install-codegraph.sh`.
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirect stdout to
  `run.log`, never onto `RESULT.txt` — they collide and the run is lost.
- ⛔ **Never edit a shell script while it is running** — `sh` re-reads from a
  byte offset.
- ⚠ **`pgrep -f "90-kdenlive"` MATCHES YOUR OWN WAITING LOOP.** Use
  `ps -eo pid,args | grep -v grep`.
- ⚠ Use a heredoc for commit messages, never `git commit -m` with backticks.
- 4 cores, ~15 GiB RAM, uid 0.

## ⚠ The number to correct if you see it quoted

jq's 1.22× was produced by the UNSAFE sweep. Honest, re-measured:

    was                    11,471,610 B  2.86x
    safe (name rules)       7,331,882 B  1.83x
    aggressive (+ sweep)    6,389,461 B  1.59x
    field                   4,006,916 B  1.00x
