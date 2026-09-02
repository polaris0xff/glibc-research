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
    ./pgb selftest                  200 pass, 1 could not run (no zstd), exit 2
    make check                      selftests + both record gates, exits 0
    disk                            30 GiB free at session start

## In flight right now

    experiments/90- RUN 6, `aggressive`   log /var/tmp/exp90-run6-aggressive.log
                                      ⛔ DO NOT KILL IT. Run 4 died that way.
                                      ⚠ The SWEEP alone takes ~12 min of it.

    the trixie build environment      READY, glibc 2.41, full package list, at
                                      /var/lib/pgb-rootfs/pgb-env-debian-trixie
    a snapshot of the NAIVE-sweep pgb /var/tmp/pgb-naive-sweep, kept so the
                                      sweep speedup has a before/after and an
                                      equivalence check on a REAL bundle
    written but NOT RUN               experiments/91- (glibc pin candidates),
                                      experiments/93- (host-object residue),
                                      experiments/85-'s new data-coherence arm

## ⭐ KDENLIVE IS VALIDATED — RUN 5, `safe`, exit 0, pass=8 fail=0 skip=1

⛔ **The first bundle in five runs that renders.** `evidence/90-.../RESULT.txt`:

    ours rendered on every environment    11 of 11
    the competitor did too                11 of 11
    zero host shared objects              ours 11/11, the competitor 4/11

    ours   471,033,944 B   render 4,947 ms   startup 300 ms cold / 239 warm
    enh    191,900,604 B   render 2,033 ms   startup  61 ms cold /  82 warm
    ratio          2.45x          2.43x slower

⛔ **So the operator's bar is NOT met on any of the three columns**, and the
1.39× that was quoted before was a size for a bundle that did not render.
⭐ What ours does win: **0 host shared objects on every row**, against the
competitor's 1 on each glibc row and **10 on rockylinux-8**.

⚠ **Run 5 is `safe`, so it did NOT exercise the sweep** — `DropUnreachable` is
gated on `aggressive` (`internal/bundle/appimage.go`). Run 6 must set
`PGB_APPIMAGE_DEBLOAT=aggressive`.

⭐ **THE ARTEFACT CACHE NOW KEYS ON THE BUILD OPTIONS TOO** — it was keyed on
OPTIONS** — so a re-run at `aggressive` would silently re-measure the `safe`
artefact. That is the run-2 defect in a new costume. Fix it in
`experiments/90-` before run 6.

## T-070: the veto is measured and it CLEARS

    debian:12  glibc 2.36  gcc 12.2.0  kernel floor 3.2.0   (incumbent)
    debian:13  glibc 2.41  gcc 14.2.0  kernel floor 3.2.0   ⭐ SAME
    ubuntu:24.04                        could not run: registry 429

Both instruments agree on both rows — `readelf -n` `.note.ABI-tag` and
`file(1)`. ⭐ **So moving the pin past GLIBC_2.38 costs nothing on the one
property the entry named as the thing a move could take away.**

**And the ceiling collapses** (`experiments/73-`, once per pin, same day):

    ENVIRONMENT            B@2.36   B@2.41   SERVED
    opensuse-leap-15.6     13       0        993  -> 1005
    fedora-42              15       0        961  -> 976
    archlinux-latest       20       5        1198 -> 1213
    debian-12              0        0        851  -> 849     ⚠ the one cost
    the other four         0        0        unchanged

    class B distinct   20 -> 5     class C   empty at BOTH pins, all 11 rows

⚠ The five left are at `GLIBC_2.42`/`2.43` on `archlinux-latest` alone — a
rolling distribution is ahead of any pin, which is the residue that regrows.

⚠ **`debian:13` 429s at the registry while `debian:trixie` resolves**, and
`pgb rootfs pull` succeeds where `docker pull` is rate-limited — pgb does the
anonymous-token dance, dockerd's HEAD does not.

## T-072: route B is refuted and route D is opened, by one measurement

    no pad     : size=3264  used=96    headroom=3168
    64 KiB pad : size=68864 used=65648 headroom=3216   pad at tp-65616

Padding the binary's own `PT_TLS` raises `_dl_tls_static_size` **and**
`_dl_tls_static_used` together: +48 bytes of headroom, which is alignment
noise. ⭐ But the pad IS allocated in every thread at a stable offset, so a
loader handing out slices of **its own** `__thread` array gets what it reserved
— 65,536 against 3,168 — and needs none of glibc's internals to place them.

## ⛔ WHY RUNS 1–4 FAILED, KEPT SO IT IS NOT RE-DERIVED

    run 1  the sweep ran BEFORE writeEnv wrote .env, so MLT's modules were
           deleted. `ours rendered 0 of 11`. Fixed by ordering.
    run 2  the experiment REUSED the cached artefact, so the fix was never
           exercised -- it reported 267,390,365 B to the digit. Fixed: it
           rebuilds when ./pgb is newer than the artefact.
    run 3  `melt`: "Failed loading SDL3 library." libSDL3.so.0 is dlopen'd
           BY NAME from inside an MLT module. Fixed: soname-string roots,
           and sweep deletion moved to `aggressive`.
    run 4  ⛔ INVALID, AND THE CAUSE WAS THE AGENT'S OWN KILL, mid-pack.
    run 5  ⭐ VALID. 11/11 render, 11/11 zero host objects.

⛔ **Do not kill a run.** If you must stop one, delete
`/var/tmp/pgb-appimage-kden/kdenlive/*.AppImage` afterwards — the staleness
rule reuses a non-empty artefact that is older than `./pgb`.

## ⛔ WHAT IS LEFT, IN ORDER (reordered 2026-09-02c by the operator)

    T-070  P0  ⭐ VETO CLEARED and class B measured at both pins (above).
               ⛔ NOT YET: the NSS floor at 2.41 (probes built, bed was busy)
               and the ten POCs at 2.41. cfg.go is UNTOUCHED.
    T-071  P0  ⭐ items 1, 2 and 5 DONE. The rewrite iterates the sweep's own
               manifestGlobs; manifestIntegrity() is the first check here that
               reads DATA rather than DT_NEEDED and it PASSED on the real
               kdenlive bundle (`manifests 8 name only libraries present`);
               __EGL_VENDOR_LIBRARY_FILENAMES is set, because libglvnd's own
               source shows it REPLACES _DIRS. Items 3 and 4 remain, and
               experiments/85-'s new arm is written but NOT RUN.
    T-068  P1  the loader residue, 86 of 904; crashes 30 -> 10 and the 10 are
               ONE family, large C++ libraries. libLLVM dies in the 605th
               static constructor.
    T-072  P1  ⭐ route B REFUTED, route D opened and costed (above).
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
