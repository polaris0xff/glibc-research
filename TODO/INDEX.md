# INDEX.md — every entry

Counts are derived. ⛔ Do not edit them by hand — `sh TODO/check.sh` re-derives
them from the rows and fails if they disagree.

    TOTAL 36  OPEN 17  DONE 19

| priority | means | total | open | done |
|---|---|---|---|---|
| P0 | breaks correctness, loses data, or takes the process down | 1 | 1 | 0 |
| P1 | a documented capability does not work, or a flag does nothing | 27 | 10 | 17 |
| P2 | worth doing; nothing is wrong without it | 8 | 6 | 2 |
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
| T-022 | P2 | M | open | research | Spike a nixpkgs front end for the planner |
| T-030 | P1 | M | done | runtime | `--wrap-dlopen` against a compiled-in table |
| T-031 | P2 | L | open | runtime | Port cross-libc-dlopen's full rewrite, not one function |
| T-032 | P1 | S | done | runtime | `--embed-terminfo` and a CA-bundle answer |
| T-033 | P1 | L | open | runtime | Route D: compile an ELF loader in, resolve against our own static glibc |
| T-040 | P1 | S | done | ci | Run CI once |
| T-041 | P2 | M | open | ci | aarch64 |
| T-050 | P1 | M | done | toolchain | Plan a nixpkgs package with NO nix, from the `.drv` in the cache |
| T-051 | P1 | M | open | toolchain | Enough nix for a host with no root, no docker and no nix |
| T-052 | P1 | M | done | research | The libGL problem, and whether a bundle can claim "universal" |
| T-053 | P2 | S | done | research | patchelf and patsh: use them, or say why not |
| T-054 | P1 | L | open | poc | kdenlive, static: exhaust it |
| T-055 | P1 | L | open | poc | If static will not reach it, a kdenlive bundle that BEATS the field |
| T-056 | P2 | L | done | toolchain | Port the python helpers to Rust — superseded by T-061 |
| T-057 | P1 | L | ⚠started | research | The bundler: a maintained nix-appimage descendant on the Anylinux mechanisms |
| T-058 | P1 | S | done | toolchain | Two `pgb build`s at once share one wrapper directory |
| T-059 | P1 | M | open | research | GL on real hardware, and the NVIDIA case |
| T-060 | P1 | L | open | toolchain | ⭐ Static-glibc nix: no root, no docker, no nix |
| T-061 | P0 | XL | open | toolchain | ⛔ Port the whole toolchain to Go, and ship one static `pgb` |
| T-062 | P1 | M | open | toolchain | Eight packages carry no selftest, `internal/wrapper` among them |
| T-063 | P1 | L | open | poc | miniflux + an embedded PostgreSQL, against onelf's ~70 MB |

## The argument behind the ordering

⭐ **Recorded so it can be re-derived rather than re-argued.** The order itself
is in [`PROGRESS.md`](PROGRESS.md) "Work order"; this is why it is that order.

0. ⭐ **T-061 is landed and no longer outranks anything.** The toolchain is one
   static Go binary, the shell and Python it replaced are the oracle under
   `../HISTORY/`, and ⭐ **all six workload gates are met** — gate 5 completed
   in the session of 2026-09-02b with ten of ten POCs and twenty-three
   experiments. The entry stays open only for what the operator adds to it.
1. **T-063 first, because it is closest to done and it is the operator's
   newest instruction.** Arm S already has a static PostgreSQL 18.6 running on
   Alpine; what is missing is `src/interfaces`, so `initdb`/`pg_ctl`/`psql` do
   not exist yet and nothing yet claims the miniflux stack runs. ⚠ An entry
   that is one build away from an answer is worth more than one that is ten.
2. **T-062 next, and it is the cheapest insurance in the tree.** Eight
   packages carry no selftest and `internal/wrapper` is one of them — it
   composes every flag `pgb build` injects, and its only acceptance is gate 4,
   which needs a bed, a network and half an hour. ⛔ **A change to the product
   cannot currently be checked while it is being made.**
3. **T-055 before the remaining rungs.** `experiments/90-` measured the gap and
   it is size-dominated: the artefact is 2.49x the competitor and start and
   render both track size. The reachability sweep exists and ⛔ **nothing
   consumes it**, which is the single largest lever and is already written.
4. **T-060, T-054, T-057 and T-051 by goal.** Each is a rung on one of the
   operator's three goals and each has evidence per rung; take the goal that
   is furthest from its bar.
5. **P2 by category last.** Nothing is wrong without them.

⚠ **Two pieces of real work are named in `PROGRESS.md` and are deliberately
NOT entries**, because each is one clear fix inside T-063 arm S: the static
**link-order** problem (`AC_SEARCH_LIBS` probes `-lreadline` alone;
`poc/91-qt-xcb` answered the same class with `-Wl,--start-group`) and **a C
link that pulled in a C++ archive** (`libicuuc.a` needs `operator delete`;
`LinkFlags` already takes a `cxx bool`). File them if they outgrow that.
