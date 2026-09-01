# poc — harder applications, until something breaks

⭐ **This is priority one.** The way static glibc gets pushed further is by
building progressively harder software with `pgb`, watching it fail, and fixing
`pgb` or `tool/runtime/*.c` until it does not. Five projects pass today
(`docs/AGENTS.md` §9); none of them is hard enough to find the next defect.

---

## T-001 — Build a C++ project with a real dependency tree

**Source** operator, session of 2026-09-01.
**Category** poc · **Priority** P1 · **Effort** M · **Status** ✅ done

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

**Closed with** `poc/60-leveldb/` — LevelDB 1.23 built with CMake, linked into
a C++ subject, `pass=12 fail=0 skip=0`, `ok` and `none` on all 11.
`evidence/poc/60-leveldb/RESULT.txt`.

⛔ **The premise was wrong and keeps its title.** It read "⚠ Read, not
measured: `pgb`'s wrappers pass `-shared` through untouched and inject only at
executable links, so a C++ build *should* work unchanged." Measured: **no C++
program linked at all**, and the reason is ordering, not `-shared`.

libstdc++ calls `iconv` itself from `std::__narrow_multibyte_chars`. `--wrap`
rewrites those references to `__wrap_iconv*` exactly as it does the
application's — and the wrappers append `pgb`'s flags to the end of the user's
argv, after which the compiler driver appends its own libraries. `gcc -###`
on this machine: `-lpgbruntime` at 178, `"-lstdc++"` at 180. An archive is
scanned where it appears, so:

```
undefined reference to `__wrap_iconv_open'
... in .text._ZSt24__narrow_multibyte_charsPKcP15__locale_struct
```

Fixed in `tool/lib/wrappers.sh` with `-Wl,-u,__wrap_iconv_open` for the **C++
drivers only** — the same forcing technique already used for
`pgb_runtime_anchor`. ⚠ The cost is stated rather than hidden: a C++ program
now links the iconv shim whether or not it calls `iconv`. The C property
`docs/AGENTS.md` §10 measures is kept — re-measured after the change, a C
program that never calls `iconv` is **1,008,152 bytes** and a C++ one is
**2,160,440**.

⭐ **Three more things the C++ arm found, none of them about C++:**

1. **The pinned environment had no CMake, meson or autoconf** — T-016. Every
   passing POC was an autotools tarball because that was the only build system
   the environment could run, and nothing said so.
2. ⛔ **`poc_matrix` passed with no functional test.** `poc_functional_test >
   script` writes an **empty file** when the function is undefined; `sh` on an
   empty script exits 0; every row read `ok` and the trace of that empty
   script found no objects. **Eleven green rows having executed nothing.**
   `poc_matrix` now refuses. ⚠ All five existing POCs define the function, so
   no committed result was affected — the harness could have certified a bad
   POC, and did not.
3. ⛔ **`poc_observe` had the same shape** — a missing probe printed `<none>`
   in every row, indistinguishable from eleven environments measured clean.
   It now says `NOT MEASURED` and counts a skip.

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

⚠ **Partly served already, and the entry stays OPEN because its subject has
not been attempted.** `evidence/72-static-host-plugin-abi/CPYTHON-FAILURE.txt`
is a committed record of a build that did not work — CPython 3.12.7 with a
static interpreter and shared stdlib modules — with the cause named at
`Makefile:1125` and diagnosed to a structural fact (`.dynsym` entries: 0), and
it produced both a fix in `--wrap-dlopen` and a new finding in
`docs/limitations.md` §0. ⛔ But this entry asks for a failure found by
building **above the current class** — the entry says a GTK or Qt application
— and that is a different question: what breaks when the *dependency graph*
gets large, rather than what breaks when a known-good project is deliberately
configured the hard way. The GTK/Qt attempt is still owed.
