# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log,
the entries, and [`../HISTORY/`](../HISTORY/).

    STATE     2026-09-03c  ⚠ IN PROGRESS — refreshed as work lands, not only
              at the end
    COUNTS    50 entries, 15 open, 35 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: GREEN on every push this session
              selftests 540 pass, 1 could not run (no zstd)
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ THE BAR MOVED. The operator struck SIZE from the bundler's
              acceptance axes and replaced it with SPEED plus one-command
              packaging. Every size lever this project has built is now
              UN-SCORED until it is re-measured on the clock, and the clock is
              the column we are furthest behind on.
              ⭐ TODO/ was stripped: 5,085 lines of entry text -> 545. The 34
              closed entries and the long-form findings behind the open ones
              are in HISTORY/entries/, and the gate now enforces the split
              rather than trusting it.
              ⛔ AND THE LIST OF NINE GLIBC QUIRKS WAS NOT COMPLETE. A TENTH
              was found by taking "are there still some?" as a question about
              completeness: static glibc reads the HOST's timezone database,
              4 of 11 environments have none -- including ubuntu-20.04, which
              is glibc -- and they do not say so. T-076, experiments/97-.

## ⛔ READ THIS FIRST

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers, the planner, the verifier and the bundler are the same executable,
built `CGO_ENABLED=0`, carrying the C runtime sources it compiles. The shell
and Python it replaced are under [`../HISTORY/`](../HISTORY/), unedited, and
are the oracle every gate is measured against.

Read [`../HISTORY/entries/toolchain.md`](../HISTORY/entries/toolchain.md)
§T-061 for what was required, then
[`../docs/design/toolchain.md`](../docs/design/toolchain.md) "Language and
structure" for the decision, the architecture and the six gates.

## ⭐ The operator's rulings in force

⚠ **Each is quoted verbatim where it lives, because the reasoning is the
load-bearing part.** This is the index.

| when | ruling | where it is recorded |
|---|---|---|
| 2026-09-01b | *"replace with per part claim, also anylinux is a bundle, our primary goal is still a static glibc binary that has none of the issues"* | [`../docs/REQUIREMENTS.md`](../docs/REQUIREMENTS.md) |
| 2026-09-01b | nixpkgs IS the planner | [`../docs/design/nix-front-end.md`](../docs/design/nix-front-end.md) |
| 2026-09-01c | the three goals, below | this page |
| 2026-09-02b | *"pgb bundle isn't good enough, it is bloated, slow and a complete failure"* | T-066 |
| ⭐ **2026-09-03c** | *"us having a bigger size than anylinux-appimages and onelf is acceptable as long as ours performs better and packaging is just one command not a multiline shell script"* | [`../docs/design/toolchain.md`](../docs/design/toolchain.md) "Static first, bundle last" |
| ⭐ **2026-09-03c** | *"we will have multiple backends, nix just being one of them"*, and a static nix is one we **publish** | [`../docs/design/nix-front-end.md`](../docs/design/nix-front-end.md) |
| ⭐ **2026-09-03c** | *"strip away the fat … the TODO/\* must be lean and contain only what's left"* | [`../HISTORY/README.md`](../HISTORY/README.md), and `check.sh` 4b/4c |

## ⭐ The operator's three goals

> *"1. Make the 'universal' builder true via pgb + nix. 2. make the 'universal'
> bundler true via a modern, updated, maintained 'nixappimage' descendant that
> uses or rather reimplements many of the anylinux tooling, iterating/improving
> them, and debloating nixappimages, correctly packing them, and also solving
> the opengl problem ... 3. poc a kdenlive static (exhaust all resources), if
> impossible, pivot to kdenlive.nixappimage, but it must be smaller, load
> faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."*

| goal | entries | where it stands |
|---|---|---|
| 1. the builder | T-051, T-060, T-012 | ⭐ Spec resolution, build-system detection and the dependency planner all EXIST and ran end to end on postgres. ⛔ Blocked on **building nix's own closure static**, not on a tool. ⚠ And "no nix" does not hold for a dotted attribute |
| 2. the bundler | T-057, T-066, T-055 | ⭐ One command from a package name, 11 of 11, zero host objects — ⭐ **and one-command packaging is now half the bar**. ⛔ **The other half is the clock and we lose it**: 4.92× cold start on kdenlive, ~1.9× on `jq` |
| 3. kdenlive | T-054, T-055 | ⭐ The ENGINE is static and renders on 11 of 11. Rungs 1 and 2 (Qt 6, a real window) closed. ⛔ Rung 3 is **two** direct inputs, not a framework set. ⛔ The bundle bar is not met on the three columns the operator named |

⛔ **Goal 3's operator sentence — *"smaller, load faster, run faster"* — is
NOT amended by the 2026-09-03c ruling.** The ruling sets the general bundler
bar; goal 3 names a specific competitor and three specific columns. ⚠ Where
the two disagree, say which one you are measuring against.

## ⭐ Work order — ⛔ REORDERED 2026-09-03c BY THE OPERATOR

> *"let's dedicate remaining session to reorder the leftover tasks and carry
> out 4 deep reviews, we can leave the build from git/url deferred for now,
> and fix all remaining GLIBC quirks if there still are some, else focus the
> next session entirely on optimizing the nix bundler as much as possible"*

    ---- 0. THIS SESSION ----

    A   ✅ the operator's rulings recorded verbatim, and every claim they
        re-score amended in place.
    B   ✅ TODO/ stripped; HISTORY/entries/ created; the gate taught the
        invariant and PROVED able to fail on it.
    C   ⚠ SIX deep reviews — the operator raised it from four on 2026-09-03c
        ("add 2 more deep reviews (thorough ones) before the kickoff prompt"):
          1 ✅ does every claim hold when the command is run
              -> C23: the bundler's MILLISECONDS do not re-derive, and the
                 ruling had just made them the bar
          2 ✅ what did the change stop measuring
              -> four open entries lost the pointer to their own detail;
                 fixed, and check.sh 4d now enforces it
          3 ✅ what was deferred — the list is below, and it is short because
                three of its items were converted into deliveries instead
          4 ✅ is the code right
              -> cxxCandidates skipped the SEPARATED `-l namespec`; R3's fix
                 went halfway. Proved red, fixed, both spellings on one
                 resolver, validated on 60-leveldb and 40-jq
          5 ✅ the instruments: does each still measure what its comment says
              -> exp_run_status collapsed "could not run" into "exited N";
                 the carried note about RESULT.txt was INVERTED for 19 of 32
                 experiments and cost a measurement the same day
          6 ✅ the claims nobody has attacked
              -> 7 of 32 experiments had evidence older than their own
                 script; gate 10 now catches the class; T-077 owns the four
                 that are too expensive to re-run now

    ---- ⛔ REVIEW 3: WHAT WAS DEFERRED, NAMED PLAINLY ----

    1 ✅ CLEARED. The four operator-named references were mined AND READ the
      same day: docs/research/portable-nix.md (findings) and
      portable-nix-mechanisms.md (the usable half, at file and line).
      ⚠ WHAT REMAINS DEFERRED IS THE MEASUREMENT: nothing in that sweep was
      RUN, and the write-up says so. Three probes are named, each one command.
    2 ⚠ T-012's git/URL route -- DEFERRED BY THE OPERATOR, not by this session.
    3 ⚠ 60-, 61-, 62- and 88- were NOT re-run on the current pin. Their
      committed numbers -- which include the whole head-to-head in
      docs/comparison.md -- were measured inside glibc 2.36. Pinned in
      evidence/STALE-EVIDENCE.txt, owned by T-077.
    4 ⚠ THE BUNDLER'S CLOCK INSTRUMENT IS DIAGNOSED, NOT BUILT. 90- still takes
      one sample per arm. That is N0 and it is the first thing next session.
    5 ⚠ lookPathIn's execute-bit test diverges from exec.LookPath for a
      non-root caller. Named in the code and deliberately NOT fixed: it cannot
      be shown to fail here, and a fix with no assertion behind it is a change.
    6 ⚠ --embed-tzdata carries TWENTY zones, not the database. A zone that is
      not carried behaves exactly as before. And it WRITES to $TMPDIR, which
      is a real cost against the "writes nothing" shape claim -- which is why
      it, like --embed-terminfo, is opt-in.
    7 ✅ CONVERTED RATHER THAN DEFERRED: the tenth glibc quirk was found AND
      closed; the POC suite was re-run against the separated-`-l` fix; the
      four stale-evidence experiments that COULD be re-run were.
    D   ✅ THE REMAINING GLIBC QUIRKS — ANSWERED, AND THE ANSWER IS "YES,
        THERE WAS ONE". REQUIREMENTS.md said of its nine: "there is no
        unenumerated remainder". FALSE. `grep -rn zoneinfo` over the whole
        tree returned NOTHING, and the row that came out of looking fails on
        FOUR environments, one of them glibc. T-076 is open; the list is TEN
        and is no longer described as closed.

    ---- 1. NEXT SESSION, and the operator scoped it: THE BUNDLER ----

    ⛔ "focus the next session entirely on optimizing the nix bundler as much
       as possible". The bar is now SPEED and ONE COMMAND. Size is struck.

    N0  ⛔ FIX THE INSTRUMENT FIRST — deep review 1 found the timing half of
        the record does not re-derive. experiments/90- takes ONE SAMPLE per
        arm; its quoted numbers are from a SUPERSEDED version of the evidence
        file it cites; four runs give cold-start ratios of 2.52×, 3.48×, 4.92×
        and 5.02×; and warm exceeds cold in two of them. ⭐ Every run agrees
        on the DIRECTION — we are slower — and none pins the magnitude.
        ⭐ experiments/86- is the shape to carry across: eleven environments,
        a mean of five per arm, cold obtained by a fresh copy. Its jq figures
        DO re-derive: 139 vs 67 ms cold (2.07×), 14.9 vs 10.8 warm (1.38×).
        ⚠ Under the new bar an unpinned millisecond is worth LESS than none,
        because it reads as measurement. docs/history/corrections.md C23.
    N1  ⛔ THEN re-measure the levers ON THE CLOCK. --cut, --fixpoint, the
        debloat rules, route A and route B were all costed in BYTES. Nothing
        was measured in milliseconds, so nothing is scored against the bar.
        ⚠ Not invalidated — un-scored.
    N2  ⭐ THE HYPOTHESIS TO TEST FIRST, because it is the one that makes the
        struck size work still count: on kdenlive, "start and render are
        dominated by mounting a 398 MB dwarfs image against a 192 MB one" —
        i.e. the size column IS the time column. ⛔ NOBODY HAS MEASURED THAT.
        If it holds, every byte lever is a millisecond lever and the ordering
        below is right. If it does not, the levers are worth nothing and the
        work is elsewhere: uruntime's mount path, dwarfs settings, or the
        selector shell.
    N3  route B, costed and NOT yet built: the -mini set forces 161 of
        kdenlive's 676 closure paths (23.8%) from source, and 8 of 111 on
        mesa-demos. A floor on the biggest single path is measured — Qt with
        xcb, TLS, network and SQL is under half an hour on four cores.
    N4  --fixpoint into the debloat path behind its own flag, then
        experiments/89- as its control. ⛔ Read T-066's detail first: 89- is
        NOT one command against it, and DISK IS BINDING.
    N5  ⛔ route A at PATH granularity is measured DEAD — 0 of jq's 7 store
        paths are entirely unreachable. Do not build a path-level allowlist.
        The FILE-level sweep is the lever that works and it exists.
    N6  ⭐ xplshn/pelf IS READ and it hands N2 a lever we do not have: mount
        below 350 MB, EXTRACT above it (appbundle-runtime.go:764). Ours is
        398 MB, the competitor's 192 MB -- either side of somebody else's
        production threshold. Plus two warm-start techniques: the parsed
        config cached in an xattr ON THE ARTEFACT, and live-mount reuse.
        docs/research/portable-nix-mechanisms.md §3-4.

    ---- 2. then the builder, by how foundational ----

    T-060  rungs 1→3, the static nix. ⭐ THE TARGET HAS A NAME NOW:
           `nix-cli-static`, an attribute of the NIX FLAKE (not nixpkgs), which
           is why `pgb nix cache attr nix-cli` found nothing. Rung 1 is its
           closure built glibc-static by pgb rather than musl-static by
           pkgsStatic.
    T-051  ⛔ NOT the same work as T-060, corrected 2026-09-03c: a published
           (musl) static nix already serves "enough nix on a minimal host".
           ⚠ And `nix --store` under $HOME is NOT the route -- nix-portable
           ships it as first choice and its tracker documents it failing on
           Arch, Debian 11 and Debian 12 with a probe that passes first.
    T-012  items 2, 3 and 4. ⛔ ITEM 1, the git/URL route, is DEFERRED by the
           operator: "we can leave the build from git/url deferred for now".

    ---- 3. then kdenlive ----

    T-054  rung 3 (kio-extras, qqc2-desktop-style) then rung 4.
    T-063  arm S: src/interfaces, and the generalised undefined-symbol fix
           that readline needs and ICU already got.

    ---- 4. P2 by category ----

    T-013  ⭐ PROMOTED by the ruling — it is the instrument that measures the
           one-command half of the bundler's bar, which is the half we win.
    T-015  T-021  T-031  T-041

⭐ **Two pieces of real work are NAMED and are not entries**, because each is
one clear fix inside T-063 arm S:

    the static link-order problem   ⚠ -Wl,--start-group fixes ORDER and
                                    CANNOT fix ABSENCE, measured. The real
                                    fix is the same shape as the C++ one:
                                    read the archives' undefined symbols and
                                    append what defines them. Not built.
    a C link that pulled a C++      ✅ FIXED, and ⭐ PROVED on the real
    archive                         subject 2026-09-03c: a static PostgreSQL
                                    18.6 WITH ICU answering on Alpine.

## Open questions for the operator

⭐ **None blocking.**

1. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`.
2. ⚠ **Docker Hub rate-limits anonymous pulls in this environment.**
   ⭐ `pgb rootfs pull` does the anonymous-token dance and succeeds where
   `docker pull` 429s.
3. ⚠ **`musl-gcc` is absent**, the one remaining blocker on `experiments/90-`
   arm O.
4. ⚠ **`--tls-reserve` costs ~1.15 MB on EVERY `--host-dlopen` build**
   (4,575,160 → 5,725,864 B), because the iconv wrapper must be forced out of
   its archive whether or not the program calls iconv. A narrower fix needs
   the provider table to reference the wrapper strongly for three names only;
   it is not obviously worth the machinery.

⚠ **The session narrative and the four reviews of 2026-09-03b/c are
[`../HISTORY/sessions/2026-09-03.md`](../HISTORY/sessions/2026-09-03.md)**,
with the annotated work order and every item that was completed.
