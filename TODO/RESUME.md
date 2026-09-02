# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02c, at session END
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

    experiments/90- kdenlive, log /var/tmp/exp90-final.log, started with all
    three sweep fixes in. ⛔ IT WILL NOT HAVE FINISHED. Check the log, then
    re-run: `sh experiments/90-kdenlive-vs-enhanced.sh` (~25 min, needs the
    bed to itself; it now rebuilds when ./pgb is newer than the artefact).

⛔ **THE SWEEP MISSED THREE CLASSES OF RUNTIME-LOADED LIBRARY IN ONE DAY** and
that is the headline of the 2026-09-02c reviews:

    MLT modules      loaded from a dir named in .env -- the sweep ran BEFORE
                     writeEnv wrote it. Fixed by ordering.
    libEGL_mesa.so.0 named in a vendor JSON, living in lib/ itself, which the
                     plugin-dir rule excludes (p == root). Fixed: manifest
                     roots, 3 selftest cases incl. the negative arm.
    libSDL3.so.0     ⛔ dlopen'd BY NAME from inside an MLT module, with NO
                     data file naming it anywhere. `melt` said "Failed loading
                     SDL3 library." Fixed: any soname spelled out inside any
                     ELF is a root.

⛔ **AND SWEEP DELETION MOVED FROM `safe` TO `aggressive`**, on that evidence.
Three misses in a day means the model is a good approximation of reachability
and not a proof, so `safe` must not mean it.

⚠ **CORRECT THE NUMBER I PUBLISHED.** The 4,890,913 B / 1.22x for jq was
produced by the UNSAFE sweep. Honest, re-measured:

    was                    11,471,610 B  2.86x
    safe (name rules)       7,331,882 B  1.83x
    aggressive (+ sweep)    6,389,461 B  1.59x
    field                   4,006,916 B  1.00x

## What this session has done

⭐ **THREE OF THE FOUR P0s ARE CLOSED; T-066 is advanced and stays open.**

    T-064  ✅ static glibc's dlopen, REALLY solved. Our own ELF loader,
              tool/runtime/pgb-elfload.c, `pgb build --host-dlopen`.
              experiments/76-, exit 0, four of four:
                carried: nine assertions, every environment  = 11 of 11
                carried: loaded no host shared object         = 11 of 11
                native:  a REAL host .so on every glibc row   =  7 of 7
                native:  refuses CLEANLY on musl, no signal   =  4 of 4
                control: ran                                  =  0 of 11
              ⭐ On the four musl rows that is a GLIBC .so dlopen'd on a
              machine with no glibc. 1,093 code lines against solo's 2,332.
    T-065  ✅ what a bundle may take from the HOST: docs/design/host-fallback.md
              plus internal/bundle/hostpolicy.go, 29 offline assertions.
              Four classes, search order adopted from Anylinux-sharun.
    T-066  ⚠ ADVANCED, NOT MET. 2.86x the field -> 1.22x on jq. Two levers:
              the reachability sweep NOTHING consumed (277 objects, 12.0 MiB)
              and share/i18n, glibc's locale SOURCES, 17 MiB of a 22 MiB
              bundle. Debloat 12.7% -> 86.9% off.
    T-067  ✅ C is adequate: docs/design/runtime-language.md. 0 UBSan findings
              over 904 host objects; zig is not in the pinned debian:12 and
              would be a 53,733,924 B fetch.
    T-068  NEW, P1: the 86 of 904 host objects --host-dlopen does not load,
              each classified rather than summarised.

⛔ **Seven defects found, each by something disagreeing, never by reading:**

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
      cost a full eleven-environment run. FIXED in the Makefile.
    my own benchmark forked per sample and reported the loader 10x slower
      than ld.so; that was copy-on-write faults on a 4.4 MB static image.
    the reachability sweep had NO consumer -- `codegraph callers Sweep`.
    the docs gate caught host-fallback.md citing an experiment I never wrote.

## ⛔ WHAT IS LEFT, IN ORDER

    T-066   ⛔ STILL OPEN, and the remaining gap is NOT a bundler one:
            Anylinux's libraries come from packages optimised for size and
            ours from nixpkgs (their example: a libicudata.so under 1 MiB
            vs 30 MiB). ⭐ The next lever is WHERE THE CLOSURE COMES FROM --
            pkgforge-dev/archlinux-pkgs-debloated is the named corpus --
            not another debloat rule. Also: kdenlive is not re-measured yet
            (experiments/90- was running when this was written), and
            --debloat aggressive now buys NOTHING over safe on jq.
    T-068   the --host-dlopen residue; docs/limitations.md §1 classifies it.
            30 of the 86 are objects no static image should load (NSS,
            sanitizer and allocator interposers) and refusing them by class
            is most of the entry. libLLVM is the one that is really about
            the loader: it maps and relocates cleanly and dies in the 605th
            of its C++ static constructors.
    then    T-063, T-062, T-060, T-054, T-057, T-051, then P2
