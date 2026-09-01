# poc — harder applications, until something breaks

⭐ **This is priority one.** The way static glibc gets pushed further is by
building progressively harder software with `pgb`, watching it fail, and fixing
`pgb` or `tool/runtime/*.c` until it does not. Five projects pass today
(`docs/AGENTS.md` §9); none of them is hard enough to find the next defect.

---

## T-001 — Build a C++ project with a real dependency tree

**Source** operator, session of 2026-09-01.
**Category** poc · **Priority** P1 · **Effort** M · **Status** open

**Problem.** Every passing POC is C. C++ brings static initialisation order,
`libstdc++` exception tables across a static link, RTTI across translation
units, and `--wrap` interacting with mangled symbols. None is exercised.

**Premise.** ⚠ Read, not measured: `pgb`'s wrappers pass `-shared` through
untouched and inject only at executable links, so a C++ build *should* work
unchanged. No C++ project has been built.

**Approach.** Pick something with a dependency chain that autotools/CMake
resolves — a candidate list is in the entry, not here — build under
`pgb build`, run `pgb verify` on the matrix.

**Prove.** `sh poc/60-<name>/run.sh` exits 0 with the matrix table showing
`ok` and `none` on all 11.

## T-002 — Build something that dlopens its own plugins at scale

**Source** operator · **Category** poc · **Priority** P1 · **Effort** M · **Status** open

**Problem.** POC 50 links CPython's 49 extension modules in by hand, using
CPython's own `Modules/Setup.local` mechanism. Nothing generalises that.

**Premise.** ⭐ Measured: a program loading its *own* plugins is servable —
POC 50 does it. What is unmeasured is whether it generalises without the
project cooperating.

**Approach.** A project whose plugin loading is not configurable at build time.
This is the entry that most likely produces T-010 (`--wrap-dlopen`).

**Prove.** `sh poc/<n>-<name>/run.sh` exits 0, and `evidence/.../RESULT.txt`
shows an empty plugin directory with the functionality still working.

## T-003 — Build a project that fails, and write down why

**Source** operator · **Category** poc · **Priority** P1 · **Effort** S · **Status** open

**Problem.** Every POC in the tree passes. A tree of only-passing POCs is a
demo, not a test bed, and says nothing about where the edge is.

**Premise.** ⭐ Certain: something will fail. `docs/limitations.md` §4 already
records CPython's `nis` module failing to link and that finding came from a
failure, not a success.

**Approach.** Deliberately pick above the current class — a GTK or Qt
application. Record the failure per `docs/methodology/experiments.md`: what was
attempted, expected, actual, failure mode, suspected cause, whether it is
fundamental.

**Prove.** A committed `evidence/` record of a build that did not work, with
the cause named at file and line, and either a fix or a new entry.
