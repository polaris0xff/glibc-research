# SUMMARY.md — the session of 2026-09-03b

⛔ **Overwritten every session.** The history is the git log.

⭐ **The headline: the operator's `cross-libc-dlopen#28` review turned up a live
silent-wrong-answer defect in our own loader, and a check that could not fail on
the state it was written to catch.** ⛔ **We are not affected by upstream's
bug** — it is an `LD_PRELOAD` interposition defect, this tool ships no preload
shim and its output has no `PT_INTERP`. ⭐ **The defect CLASS is ours**, and it
is the one this tree keeps paying for: *a lookup that ANSWERS when it should
DEFER*.

⭐ **And T-066, the last open P0, has route A's ceiling MEASURED**: 218.5 MiB of
a 938.8 MiB mesa bundle is reachable only through edges two `-mini` rebuilds
delete, against **6.3%** the sweep can prove dead.

## Before and after

| | at start | at end |
|---|---|---|
| **the own-symbol table** | ⛔ ONE table, two entries with **opposite** requirements, checked first and unconditionally. Safe **by accident** | ⭐ **two tables at two points**, and `experiments/94-` asserts both directions, 11 of 11 |
| **that defect, live?** | unknown — nothing asserted it | ⭐ **yes, and silent**: reversed, the decoy `__tls_get_addr` answered **twice** while the value still read `0x5eeded` |
| **the host-policy selftest** | ⛔ five "is unset" assertions that were green on the **dangerous** state too | ⭐ presence-based, **and the instrument itself asserted** |
| **the vendored `cross-libc-dlopen`** | ⛔ pinned **pre-PR-30**; T-031 proposes porting from it | ⭐ re-mined at **`793f3f3f`**, PR 30's merge commit |
| **third-party agent files under `references/`** | ⛔ **three**, and a re-mine silently put one back | ⭐ **zero**, stripped at fetch time and **gated** |
| **T-066 route A** | ⛔ "measure the ceiling first" — no instrument | ⭐ **`pgb bundle sweep --cut`**, and the ceiling is **218.5 MiB / 23.3%** |
| **`DropUnreachable` on versioned libraries** | ⛔ could not drop them — every one was a **root of itself** through its own SONAME | ⭐ fixed; jq roots **40 → 28** |
| **the `jq` bundle vs the field** | ⛔ "1.22×" — from an evidence file predating the gating change | ⭐ **1.58×** at `aggressive`, re-measured |
| **`--tls-reserve`** | ⛔ build host only | ⭐ **11 of 11**: refuses at 0, loads at 65536, **88 bytes** on disk |
| **`pgb selftest`** | 307 cases | ⭐ **371** — `verifyx`, `fail` and `cxx-runtime` new |
| **a C link pulling a C++ archive** | ⛔ fails on `operator delete` | ⭐ **detected by reading the archives**, and it links |
| **`--start-group` as the readline fix** | asserted in an entry | ⚠ **measured false**: it fixes ORDER, not ABSENCE |
| **Entries** | 45 / 17 open / 28 done | ⭐ **48 / 16 open / 32 done** — T-072 closed, and T-073/T-074/T-075 opened and closed |

## ⛔ What was found, and again not one came from reading code

1. ⭐ **`__tls_get_addr` could lose to a loaded object, and the value would hide
   it.** The decoy returns its own slab, so a thread-local round trip through it
   is self-consistent: `tls=0x5eeded` in **both** columns. Only `decoy_calls` —
   0 fixed, 2 reversed — separates them. An experiment written around the value,
   which is the obvious way, would have measured nothing and reported a pass.
2. ⛔ **An assertion whose own label named a distinction its helper could not
   make.** `get()` returned `""` for a key that is absent **and** for one
   emitted as `KEY=`, and for `__EGL_VENDOR_LIBRARY_FILENAMES` those are the
   safe state and the dangerous one.
3. ⛔ **A re-mine undid a recorded deliberate deletion** — and sweeping all 34
   references found **two more** third-party agent-instruction files nobody had
   noticed.
4. ⛔ **A versioned library was a root of itself.** The soname scans excluded the
   object's own *filename*, but `libunistring.so.5.2.1` carries
   `DT_SONAME libunistring.so.5`. Nearly every ordinary shared library.
   ⚠ The selftest had a self-naming case and it passed throughout, because its
   fixture used a file whose name equals its SONAME.
5. ⛔ **A committed evidence file described a build configuration that no longer
   exists**, found with `git merge-base --is-ancestor` against the commit that
   changed the gating. It is where T-066's "1.22×" came from.
6. ⚠ **`LD_DEBUG_OUTPUT` writes more than one file** — `timeout` forks and is
   itself dynamic — and glob order yields the useless one.
7. ⚠ **`pgb bundle appimage --out` threw away a whole build** for a missing
   output directory, reporting `mkdwarfs failed` over a log saying it wrote
   **0 files**, which reads like the AppDir was empty.
8. ⚠ **`--start-group` cannot fix absence**, measured in three arms.
9. ⛔ **`$?` after a pipeline is the pipeline's status** — paid again:
   `make 2>&1 | tail -2 && echo BUILD_OK` printed BUILD_OK over a failed build.
10. ⛔ **`pkill -f` matched its own runner and killed two backgrounded jobs** —
    the trap `docs/AGENTS.md` §14 already records, walked into anyway.

## What is left

⛔ **T-066 is still the last open P0**, and its premise is significantly
advanced rather than met: the allowlist is bounded by a measured ceiling now,
route B is uncosted, and no kdenlive AppDir exists. Then T-062 (three packages),
T-063 (arm S re-run with `--without-icu` removed), and kdenlive.

`PROGRESS.md` carries the work order; `RESUME.md` carries what a fresh session
cannot infer.
