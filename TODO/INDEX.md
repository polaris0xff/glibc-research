# INDEX.md — every entry

Counts are derived. ⛔ Do not edit them by hand — `sh TODO/check.sh` re-derives
them from the rows and fails if they disagree.

    TOTAL 34  OPEN 16  DONE 18

| priority | means | total | open | done |
|---|---|---|---|---|
| P0 | breaks correctness, loses data, or takes the process down | 1 | 1 | 0 |
| P1 | a documented capability does not work, or a flag does nothing | 25 | 8 | 17 |
| P2 | worth doing; nothing is wrong without it | 8 | 7 | 1 |
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
| T-015 | P2 | S | open | toolchain | `oci-pull.sh` drops the image config |
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
| T-056 | P2 | L | open | toolchain | Port the python helpers to Rust |
| T-057 | P1 | L | ⚠started | research | The bundler: a maintained nix-appimage descendant on the Anylinux mechanisms |
| T-058 | P1 | S | done | toolchain | Two `pgb build`s at once share one wrapper directory |
| T-059 | P1 | M | open | research | GL on real hardware, and the NVIDIA case |
| T-060 | P1 | L | open | toolchain | ⭐ Static-glibc nix: no root, no docker, no nix |
| T-061 | P0 | XL | open | toolchain | ⛔ Port the whole toolchain to Go, and ship one static `pgb` |

## The argument behind the ordering

⭐ **Recorded so it can be re-derived rather than re-argued.**

0. ⛔ **T-061 IS THE ONLY P0 AND IT OUTRANKS EVERY ROW BELOW.** The operator,
   2026-09-02: the next session ports the tooling to Go and reaches parity, and
   *"the next session after that will return back to usual tasks."* An entry
   that changes shell or Python in `tool/` or `scripts/` before that port is
   work that will be thrown away.
1. **Harder POCs first (T-001..T-003).** The operator's instruction, and it is
   right for a reason worth writing down: every mechanism in `tool/runtime/`
   exists because something broke. Five passing POCs cannot tell you where the
   next defect is. ⛔ **A tree of only-passing POCs is a demo.**
2. **Then the split (T-010) and the language ruling (T-011).** Both are S, both
   block T-012, and T-010 gets harder every time `pgb` grows. ⚠ **This may be
   bumped above the POCs** — the operator flagged it — and the trigger is
   whether the POC work starts wanting planner code that has nowhere to live.
3. **T-030 before T-031.** Same goal, one is M and proven prior art, the other
   is L and a research port. Cheapest first.
4. **T-040 early despite being unglamorous.** It is S, and it is the only way
   the docker/podman engines and the workflow's own YAML get exercised at all.
5. **T-012 last of the P1s** because it is XL, which per `authoring.md` means
   it is not really one entry. Split it before starting.
