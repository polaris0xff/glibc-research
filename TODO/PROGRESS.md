# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log,
the entries, and [`../HISTORY/`](../HISTORY/).

    STATE     2026-09-03d  ✅ COMPLETE
    COUNTS    56 entries, 21 open, 35 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: GREEN (one red push, caught and fixed — see below)
              selftests 546 pass, 1 could not run (no zstd)
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ THE BUNDLER IS LEVEL ON SPEED. `jq` cold start went
              2.07× → **1.00×** against the field over eleven environments,
              and kdenlive's cold row went **4.92× against us → 0.74× FOR
              us**. The closure did not change; two constants in
              `internal/bundle/appimage.go` did — uruntime `full` → `lite`
              and the dwarfs block `-S26` → `-S18`.
              ⛔ AND THE INSTRUMENT THAT SAID 2.07× WAS PART OF THE PROBLEM:
              `90-`'s cold column was measuring a WARM start. uruntime keys
              its mount on CONTENT and holds it for 5 s, so "cold by a fresh
              copy" reused the live mount. `corrections.md` C24.

## ⛔ READ THIS FIRST

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers, the planner, the verifier and the bundler are the same executable,
built `CGO_ENABLED=0`, carrying the C runtime sources it compiles. The shell
and Python it replaced are under [`../HISTORY/`](../HISTORY/), unedited, and
are the oracle every gate is measured against.

⭐ **Every millisecond in this tree now goes through
[`../experiments/clock.sh`](../experiments/clock.sh)** — median of N, arms
interleaved with a rotating start, and an **A/A control** whose ratio is the
floor below which no row may be believed. ⛔ Do not add a timing column that
does not use it.

## ⭐ The operator's rulings in force

⚠ **Each is quoted verbatim where it lives, because the reasoning is the
load-bearing part.** This is the index.

| when | ruling | where it is recorded |
|---|---|---|
| 2026-09-01b | *"replace with per part claim, also anylinux is a bundle, our primary goal is still a static glibc binary that has none of the issues"* | [`../docs/REQUIREMENTS.md`](../docs/REQUIREMENTS.md) |
| 2026-09-01b | nixpkgs IS the planner | [`../docs/design/nix-front-end.md`](../docs/design/nix-front-end.md) |
| 2026-09-01c | the three goals, below | this page |
| 2026-09-02b | *"pgb bundle isn't good enough, it is bloated, slow and a complete failure"* | T-066 |
| 2026-09-03c | *"us having a bigger size than anylinux-appimages and onelf is acceptable as long as ours performs better and packaging is just one command not a multiline shell script"* | [`../docs/design/toolchain.md`](../docs/design/toolchain.md) |
| 2026-09-03c | *"we will have multiple backends, nix just being one of them"* | [`../docs/design/nix-front-end.md`](../docs/design/nix-front-end.md) |
| ⭐ **2026-09-03d** | *"Defer comparing speed/startup/performance with anylinux-appimages & onelf for now"* — and **reprioritise the remaining glibc-static and nix-bundle quirks** | this page, §"Work order" |

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
| 1. the builder | T-051, T-060, T-012 | ⭐ Spec resolution, build-system detection and the dependency planner all EXIST and ran end to end on postgres. ⛔ Blocked on **building nix's own closure static**, not on a tool |
| 2. the bundler | T-057, T-066, T-055 | ⭐ **One command, 11 of 11, zero host objects, and now LEVEL ON SPEED** — `jq` 1.00× cold, kdenlive 0.74× cold. ⛔ Size is 1.70× (`jq`) and 2.95× (kdenlive) |
| 3. kdenlive | T-054, T-055 | ⭐ **Two of the three columns are now ours**: cold start 0.74×, host objects 0 of 11 against the competitor's 4 of 11. ⛔ *smaller* is not met (2.95×) and *run faster* is **unresolved** — two runs disagree in direction |

## ⭐ Work order — ⛔ REORDERED 2026-09-03d BY THE OPERATOR

> *"1. Defer comparing speed/startup/performance with anylinux-appimages &
> onelf for now. 2. Reprioritize fixing all remaining quirks with GLIBC Static
> binaries and Nix Bundles."*

⛔ **THE SPEED COMPARISON IS DEFERRED, NOT ABANDONED.** It was the whole of the
previous work order and it is now parked: `jq` is level and kdenlive's cold row
is ours, which is far enough for now. ⚠ Do not spend the next session on
another millisecond.

    ---- ⭐ THE NEXT SESSION'S TASK LIST, AND IT IS TWO GOALS ----

    ⛔ EVERY ITEM BELOW IS A NUMBERED ENTRY. Read the entry, not this
       summary: it carries the sources, the measurements and the traps.

         G1.1 → T-078   the three-way parity matrix (runtime, P1, L)
         G1.2 → T-079   enumerate the remainder BY SEARCH (runtime, P1, M)
         G2.1 → T-080   the capability guarantee (research, P1, L)
         G2.2 → T-081   the debloater/patcher (toolchain, P1, L)
         F1   → T-082   vendor + patch + drift detection (toolchain, P2, XL)
         F2   → T-083   desktop integration (toolchain, P2, M)

    G1  ⛔ GLIBC STATIC IS TRULY COMPLETE. "No edge cases exist, and our
        static glibc binary and a native musl static binary are at
        feature/standalone parity. No buts and no ifs." — operator.

        G1.1  ⭐ THE DELIVERABLE IS A TABLE, and it is named:
              **"vanilla" gcc -static  vs  OURS  vs  native musl static**,
              compared on EVERY axis they can be compared on. Not three
              columns of prose — a matrix where every cell is a measurement
              or a dash, `docs/comparison.md`'s rule.
              ⚠ Candidate axes, from what this tree already measures:
                runs / payload clean on the eleven; NSS; iconv/gconv; locale;
                terminfo; CA bundle; timezone; dlopen of own plugins; dlopen
                of host objects; throughput (malloc/qsort/str/snprintf/math/
                memcpy); startup; peak RSS; artefact size; what each writes to
                the filesystem; PT_INTERP/DT_NEEDED; what breaks it.
              ⛔ Every row needs the MUSL column actually run, not inferred.
              `experiments/60-` and `61-` already build musl arms — start there.
        G1.2  ⛔ ENUMERATE THE REMAINDER RATHER THAN ASSERTING THERE IS NONE.
              `REQUIREMENTS.md` said of its list of nine "there is no
              unenumerated remainder" and a TENTH was found the next day by
              asking the question properly (`grep -rn zoneinfo` returned
              nothing, and 4 of 11 environments fail it). ⭐ The list is TEN
              and nine are closed. **Ask again, and this time the answer has
              to be a search, not a sentence.**
        G1.3  ⚠ The known-open one is host `dlopen` beyond what
              `--host-dlopen` covers, plus `--tls-reserve`'s cost. T-072.

    G2  ⛔ NIX'S EGL / SDL / XCB ISSUES ARE SOLVED OR PATCHED, and the only
        things left deferred for nix bundling are the two named below.

        G2.1  ⭐ THE DELIVERABLE IS A WRITE-UP WITH A GUARANTEE IN IT: that
              everything left unsolved in our nix bundle is **tooling, size
              or performance** — NOT that nix cannot do EGL/SDL/XCB, and NOT
              that it cannot load vulkan or nvidia.
              ⛔ The anylinux references must be studied extensively for this.
              ⚠ What we have: `experiments/85-` runs EGL out of a closure at
              pass=10 fail=0, every row `swrast` and surfaceless.
              ⚠ What the FIELD records: of 16 `nixappimage` recipes in
              `soarpkgs`, three are disabled and one is *"Fails to create EGL
              Display"* (ghostty, citing NixOS/nixpkgs#9415). 13 of 16 are
              ACTIVE including chromium, brave, discord, telegram — so the
              baseline to beat is higher than "nix cannot do GUI".
              ⛔ **AND THE FIELD'S GRADES ARE NOT OUR GRADES — OPERATOR,
              2026-09-03d.** `HALL-OF-FAME.md`'s verdicts are subjective and
              were earned deploying ARCH PACKAGES through quick-sharun, where
              paths and plugin dirs must be discovered by hand. A nix closure
              is the opposite. The operator's counter-example: *"in
              nixappimage for instance, python is easy and works, choose any
              python gui app and it works"* — against a **"Utter garbage"**
              grade. ⛔ Re-derive every row against `pgb bundle appimage`
              before quoting any of it.
              ⭐ **AND THE GOAL IS TO MOVE THEM**: every "Garbage",
              "Horrible" and "Utter garbage" row — GTK, Wayland, Python,
              glibc, WebKit, p11kit, JACK2 — should come out **Excellent or
              close** through a nix closure, or this project must name the
              exact mechanism that stops it.
              📚 [`../docs/research/bundle-capabilities.md`](../docs/research/bundle-capabilities.md),
              [`../docs/research/nix-bundle-patching.md`](../docs/research/nix-bundle-patching.md) §8.
        G2.2  ⛔ THE DEBLOATER/PATCHER COVERS EVERY CASE — shebang lines,
              hardcoded paths, `.desktop` files, anything else in a bundle.
              ⭐ **The corpus is mined and read** (`nix-bundle-patching.md`):
              the field does it with a FIVE-REGEX sed cascade ending in
              "replace any store path with /", and the operator's instruction
              is *"our debloater must find a way to get better results
              without being so messy"*.
              ⭐ **The route is named and it is not "nicer regexes"**: `pgb`
              has the CLOSURE, so the rewrite can be an exact match against a
              known finite set, and a store path with no in-bundle target is
              a FINDING rather than a silent substitution.
        G2.3  ⚠ ONLY THESE TWO MAY BE DEFERRED, per the operator:
                (a) our own "static" nix, embedded in pgb or shipped beside it
                    — T-060, T-051;
                (b) nothing else. G2.2 is not a place to leave a remainder.

    ---- ⛔ DEFERRED BY THE OPERATOR, 2026-09-03d ----

    ⛔ Speed / startup / performance against anylinux-appimages and onelf.
       ⚠ The instruments stay: `experiments/clock.sh`, `77-`, `81-`, `84-`,
       `86-`, `90-`, `99-` all work and all assert their own A/A control.
       Re-running one is cheap; the DEFERRAL is on making it the work.

    ---- ⭐ FUTURE, AFTER THE NEXT SESSION — recorded so it is never lost ----

    F1  ⛔ VENDOR AND PATCH THE THIRD-PARTY RUNTIME AND TOOLING.
        ⭐ **THE REASON IS MEASURED, NOT SPECULATIVE.** This project spent a
        whole session discovering that the field runs a `lite` uruntime and a
        different block size, because nothing tracked upstream's choices.
        Both were free wins sitting in somebody else's build flags.
        ⚠ And the pkgforge builds are ALREADY forks of upstream, so we are
        two levels behind, not one.
        ⭐ The things to vendor, all now in `references/`:
          pkgforge-dev/Anylinux-AppImages     docs, guides, faqs, the index
          pkgforge-dev/Anylinux-uruntime      ✅ MINED 5a0b4a33
          pkgforge-dev/Anylinux-sharun        ✅ already vendored
          pkgforge-dev/appimagetool           ✅ MINED 183c0492
          pkgforge-dev/archlinux-pkgs-debloated  ✅ already vendored
          pkgforge-dev/userland-execve-rust   ✅ already vendored
        ⭐ THE METHODOLOGY IS PRESCRIBED: `Azathothas/TEMPLATE`
        `docs/methodology/vendoring.md` (already vendored under
        `docs/methodology/`), and the worked example is
        `Azathothas/bit-cli` `{patches,vendor}` ✅ MINED cce81312.
        ⛔ **And it must be WIRED INTO THE DEV CYCLE**: a script/tool/bot that
        detects upstream's new commits and auto-diffs them. A vendored tree
        with no drift detector is how this session's finding was possible.

    F2  ⭐ NATIVE DESKTOP INTEGRATION AND AppImage COMPATIBILITY, so third-party
        package managers can consume our bundles as ordinary AppImages.
        ⛔ Requires every nixappimage quirk in G2 solved first.
        The managers to be compatible with — each must be mined and studied
        for what it expects, then extracted and patched against:
          mijorus/gearlever      kem-a/AppManager
          ivan-hc/AM             pkgforge/soar
        ⚠ `.DirIcon` and a top-level `*.desktop` are the two things every one
        of them looks for; `nix-bundle-patching.md` §5 has the rules the field
        uses to produce them.

    ---- then the builder, by how foundational ----

    T-060  rungs 1→3, the static nix. ⭐ THE TARGET HAS A NAME:
           `nix-cli-static`, an attribute of the NIX FLAKE (not nixpkgs).
    T-051  ⛔ NOT the same work as T-060: a published (musl) static nix already
           serves "enough nix on a minimal host".
    T-012  items 2, 3 and 4. ⛔ ITEM 1, the git/URL route, is DEFERRED.

    ---- then kdenlive ----

    T-054  rung 3 (kio-extras, qqc2-desktop-style) then rung 4.
    T-063  arm S: src/interfaces, and the generalised undefined-symbol fix.

    ---- P2 by category ----

    T-013  T-015  T-021  T-031  T-041

## Open questions for the operator

⭐ **None blocking.**

1. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`, and G2.1
   cannot claim "vulkan/nvidia work" without one. It can claim what the
   closure does offscreen, and must say which it is claiming.
2. ⚠ **`musl-gcc` is absent**, which skips `experiments/90-` arm O (onelf) and
   will bite **G1.1**: the musl column of the required table needs a musl
   toolchain, and `experiments/60-`/`61-` build their musl arms with one.
   ⛔ Install it before starting G1, or the table has a hole in the column the
   operator named.
3. ⚠ **kdenlive's warm row is 114 ms against `jq`'s 8**, unexplained. ⭐ The
   first candidate is in `nix-bundle-patching.md` §1: at 565 MB our bundle is
   over uruntime's 350 MB `MAX_EXTRACT_SELF_SIZE`, so it **extracts** where
   `jq` **mounts**. Different runtime path, different warm behaviour.
4. ⚠ **Docker Hub rate-limits anonymous pulls here.** `pgb rootfs pull` does
   the anonymous-token dance and succeeds where `docker pull` 429s.

⚠ **The session narrative, the six reviews and every defect found in this
session's own work are
[`../HISTORY/sessions/2026-09-03d.md`](../HISTORY/sessions/2026-09-03d.md).**
