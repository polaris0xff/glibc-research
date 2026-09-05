# INDEX.md — every entry

Counts are derived. ⛔ Do not edit them by hand — `sh TODO/check.sh` re-derives
them from the rows and fails if they disagree.

    TOTAL 69  OPEN 25  DONE 44

| priority | means | total | open | done |
|---|---|---|---|---|
| P0 | breaks correctness, loses data, or takes the process down | 7 | 1 | 6 |
| P1 | a documented capability does not work, or a flag does nothing | 47 | 13 | 34 |
| P2 | worth doing; nothing is wrong without it | 15 | 11 | 4 |
| P3 | worth recording so it is not rediscovered | 0 | 0 | 0 |

Effort: S under a day · M a few days · L a week · XL almost always two entries
pretending to be one.

| id | pri | eff | status | category | title |
|---|---|---|---|---|---|
| T-001 | P1 | M | done | poc | Build a C++ project with a real dependency tree |
| T-002 | P1 | M | done | poc | Build something that dlopens its own plugins at scale |
| T-003 | P1 | S | done | poc | Build a project that fails, and write down why |
| T-010 | P1 | S | done | toolchain | Split `pgb` into `tool/lib/*.sh` |
| T-011 | P1 | S | done | toolchain | Confirm or overturn the language decision |
| T-012 | P1 | XL | open | toolchain | `pgb build <url-or-package>` |
| T-013 | P2 | S | open | toolchain | Measure developer friction |
| T-014 | P1 | M | done | toolchain | `pgb verify` ignores `--engine` |
| T-015 | P2 | S | open | toolchain | the OCI pull drops the image config |
| T-016 | P1 | S | done | toolchain | Pinned env cannot run CMake or meson |
| T-017 | P1 | S | done | toolchain | `env create` builds one engine; `pick_engine` may choose another |
| T-018 | P1 | S | done | toolchain | A `pgb` binary has no `PT_GNU_EH_FRAME` |
| T-019 | P1 | S | done | toolchain | The docker engine drops every build option |
| T-020 | P1 | M | done | research | Sweep the nix-appimage family |
| T-021 | P2 | M | open | research | Build one nix-appimage and run it on the matrix |
| T-022 | P2 | M | done | research | Spike a nixpkgs front end for the planner |
| T-030 | P1 | M | done | runtime | `--wrap-dlopen` against a compiled-in table |
| T-031 | P2 | L | open | runtime | Port cross-libc-dlopen's full rewrite, not one function |
| T-032 | P1 | S | done | runtime | `--embed-terminfo` and a CA-bundle answer |
| T-033 | P1 | L | done | runtime | ⚠ Route D — SUPERSEDED BY T-064, which shipped it; closed as a duplicate |
| T-040 | P1 | S | done | ci | Run CI once |
| T-041 | P2 | M | open | ci | aarch64 |
| T-050 | P1 | M | done | toolchain | Plan a nixpkgs package with NO nix, from the `.drv` in the cache |
| T-051 | P1 | M | open | toolchain | Enough nix for a host with no root, no docker and no nix |
| T-052 | P1 | M | done | research | The libGL problem, and whether a bundle can claim "universal" |
| T-053 | P2 | S | done | research | patchelf and patsh: use them, or say why not |
| T-054 | P1 | L | open | poc | kdenlive, static: exhaust it |
| T-055 | P1 | L | open | poc | If static will not reach it, a kdenlive bundle that BEATS the field |
| T-056 | P2 | L | done | toolchain | Port the python helpers to Rust — superseded by T-061 |
| T-057 | P1 | L | open | research | The bundler: a maintained nix-appimage descendant on the Anylinux mechanisms |
| T-058 | P1 | S | done | toolchain | Two `pgb build`s at once share one wrapper directory |
| T-059 | P1 | M | open | research | GL on real hardware, and the NVIDIA case |
| T-060 | P1 | L | open | toolchain | ⭐ Static-glibc nix: no root, no docker, no nix |
| T-061 | P0 | XL | done | toolchain | Port the whole toolchain to Go, and ship one static `pgb` |
| T-062 | P1 | M | done | toolchain | ⭐ Every package carries a selftest now: 375 → 540 cases, each proved able to fail |
| T-063 | P1 | L | open | poc | miniflux + an embedded PostgreSQL, against onelf's ~70 MB |
| T-064 | P0 | XL | done | runtime | ⭐ Static glibc's `dlopen`, really solved: our own ELF loader, 11 of 11 |
| T-065 | P0 | L | done | research | ⭐ What a bundle may take from the host: four classes, the order adopted |
| T-066 | P0 | XL | open | toolchain | ⛔ The bundler is bloated and slow — rebuild it against a CLI benchmark |
| T-067 | P0 | M | done | toolchain | ⭐ C is adequate for `tool/runtime/`, measured: 0 UBSan findings over 904 objects |
| T-068 | P1 | M | done | runtime | ⭐ The residue `--host-dlopen` does not load — two real defects fixed, nothing crashes us that glibc loads |
| T-069 | P1 | S | done | research | ⭐ The supplied working paper, swept: useful, and it names the gap `experiments/76-` closed |
| T-070 | P0 | M | done | toolchain | ⭐ The glibc pin MOVED, 2.36 → 2.41: four costs measured at zero, class B 20 → 5 |
| T-071 | P0 | L | done | research | ⭐ EGL from a nixpkgs closure: four failures, all in DATA rather than code |
| T-072 | P1 | M | done | toolchain | ⭐ `--tls-reserve` on the eleven: reserve 0 refuses and 65536 loads, 11 of 11 |
| T-073 | P1 | S | done | runtime | ⭐ The own-symbol table answered where one of its two names had to defer |
| T-074 | P1 | S | done | research | ⭐ The host-policy selftest could not fail on the state it was written to catch |
| T-075 | P2 | S | done | research | ⭐ LD_DEBUG=bindings on the control, because the subject cannot be asked |
| T-076 | P1 | M | done | runtime | ⭐ The TENTH quirk, found and CLOSED the same day: `--embed-tzdata`, 11 of 11 |
| T-077 | P1 | M | open | ci | ⛔ The head-to-head was measured on the RETIRED glibc pin, and nobody re-ran it |
| T-078 | P1 | L | done | runtime | ⭐ The three-way parity matrix, and TWO of its seven predictions were wrong |
| T-079 | P1 | M | done | runtime | ⭐ The list was TEN and it is ELEVEN: `/etc/services`, found by a search |
| T-080 | P1 | L | done | research | ⛔ REOPENED: the capability guarantee, on THREE applications per category |
| T-081 | P1 | L | done | toolchain | ⭐ Every store path, without the regex cascade: the bundle DRAWS, and Python bundles |
| T-082 | P2 | XL | open | toolchain | Vendor and patch the third-party runtime and tooling, with drift detection |
| T-083 | P2 | M | open | toolchain | Native desktop integration: our bundles as ordinary AppImages |
| T-084 | P1 | M | done | ci | ⛔ The trace classifier is SIX hand copies — measured: 3 texts, **2 behaviours**, and they differ from the shared one in **two** ways, not one |
| T-085 | P1 | S | done | runtime | ⭐ The ELEVENTH quirk CLOSED: `--embed-netdb`, and the boundary `--wrap` cannot cross |
| T-086 | P1 | S | done | runtime | ⭐ The one axis where native musl beat both glibc columns, closed: `--utf8-default` |
| T-087 | P1 | XL | open | research | ⭐ The battle-test corpus: 40+ applications, ordered by MECHANISM |
| T-088 | P1 | S | done | research | ⛔ Multi-entry dispatch is SHIPPED and has never been run |
| T-089 | P1 | S | done | research | ⛔ The interposer row marked NOT MEASURED: a static or raw-syscall payload |
| T-090 | P1 | M | open | research | ⛔ The sandbox rung needs a BED change, not a bundler change |
| T-091 | P2 | S | open | research | GStreamer needs four variables and a scanner; NOTHING was setting them |
| T-093 | P2 | M | open | research | "no more Vulkan layers like mangohud" is the one field objection still marked NOT MEASURED |
| T-094 | P1 | M | open | research | an application that shells out to the HOST loads the host libc, and no path rewriting prevents it |
| T-095 | P2 | S | open | toolchain | CI's libiconv fetch is one host with no mirror and no retry, and its timeout skips the matrix |
| T-092 | P2 | S | open | toolchain | The `.env` names a farm directory the farm may not have created |
| T-096 | P1 | S | open | ci | ⛔ gate 10 keyed on the evidence DIRECTORY, so eight stale pairs were invisible |

## The argument behind the ordering

⭐ **Recorded so it can be re-derived rather than re-argued.** The order itself
is in [`PROGRESS.md`](PROGRESS.md) "Work order"; this is why it is that order.

0. ⛔ **FOUR P0s, set by the operator on 2026-09-02b, outrank everything.**
   Each carries the same instruction — *work until it is met or the premise is
   significantly advanced* — so none of them is a spike to be timeboxed.
   - **T-064** is first because it is the project's thesis. `limitations.md` §1
     is the one measured, unfixed failure and the reason `REQUIREMENTS.md`
     part 1 is not met. ⭐ The evidence is already in hand: `experiments/73-`
     says 90.8–99.3% of host imports are definable by our own static glibc and
     the unexplained residue is **zero**, and `experiments/72-` says the host
     loader can never be the answer because a static binary's dynamic symbol
     table is empty. The loader has to be ours.
   - **T-065** is second because it changes what the other entries are allowed
     to assert. This tree treats every host `.so` as contamination; anylinux
     defers to the host **deliberately** for drivers. Until that policy is
     written down, T-066's bundle and T-059's GPU work are measured against
     the wrong bar.
   - **T-066** is third and is the operator's harshest verdict — *"bloated,
     slow and a complete failure"*. ⭐ It is ordered after T-065 because the
     size lever and the host-deferral policy interact: what may be dropped
     depends on what may be deferred.
   - **T-067** is last of the four because it is a **question**, and a measured
     "C is adequate" closes it. ⛔ It must not become a migration without a
     named, measured limitation behind it.
1. **T-063 next**, because it is closest to done: arm S already has a static
   PostgreSQL running on Alpine and what is missing is `src/interfaces`.
2. **T-062 after it** — eight packages carry no selftest and `internal/wrapper`
   is one of them. It composes every flag `pgb build` injects and its only
   acceptance is gate 4, which needs a bed and half an hour, so ⛔ a change to
   the product cannot currently be checked while it is being made.
3. **T-055 folds into T-066.** Same lever, same measurement; do not run them as
   two efforts.
4. **T-060, T-054, T-057 and T-051 by goal.** Each is a rung on one of the
   operator's three goals; take the goal furthest from its bar.
5. ⭐ **THEN THE 2026-09-03d ENTRIES, and the operator ordered them**: the
   speed comparison is deferred and these are what replaces it.
   - **T-078 and T-079 together** — they are one question asked two ways.
     T-078 is the table the operator named; T-079 is what has to be true for
     any cell in it to read "complete". ⛔ T-079 first if only one fits: a
     table that omits an unenumerated row is worse than no table, and that
     exact failure has happened once (nine host-data dependencies declared
     complete; a tenth found the next day).
   - **T-081 before T-080.** The guarantee is about capability; the patcher is
     about whether a bundle produced today actually launches. ⚠ A capability
     write-up standing on bundles whose `.desktop` still names store paths is
     a claim about a thing nobody can run.
   - **T-083 depends on T-081** and says so in the entry.
   - **T-082 is XL and is the one to start EARLY and finish LATE.** Its value
     is a drift detector running in the dev cycle, and a detector that starts
     reporting in three sessions' time is worth more than one that lands
     perfect in one.
6. **P2 by category last.** Nothing is wrong without them.
