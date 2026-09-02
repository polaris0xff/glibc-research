# SUMMARY.md — the session of 2026-09-02d

⛔ **Overwritten every session.** The history is the git log.

⭐ **The headline: kdenlive is validated, five runs after it was first asked
for, and the three sweep fixes that had never been exercised now have their
measurement.** Everything else in this session came out of getting there.

## Before and after

| | at start | at end |
|---|---|---|
| **kdenlive rendering** | ⛔ `ours rendered on 0 of 11`, four runs, four different causes | ⭐ **11 of 11, twice** — at `safe` and at `aggressive` |
| **the three sweep fixes** | committed, **never exercised end to end** | ⭐ **proved**: 1,712 objects and 227.4 MiB deleted, and `melt` still rendered 4,149 bytes |
| **the kdenlive size figure** | 1.39×, quoted from a bundle that **did not render** | ⭐ **2.45× at `safe`, 2.22× at `aggressive`**, both from runs that rendered |
| **the artefact cache** | keyed on the bundler's mtime only | ⭐ keyed on the **build options** too — it caught its own case on the next run |
| **the soname scan** | ⛔ quadratic: the bundle advanced at **2.8 MiB/s**, ~12 min on kdenlive | ⭐ single pass, **exactly** equivalent by construction, with the original kept as its control |
| **the glibc pin** | ⛔ 2.36, a floor set two releases above its own floor, ceiling widening yearly | ⭐ **three of four costs measured at zero**; the move is *indicated*, `cfg.go` untouched |
| **class B** | 20 distinct symbols, 14 at `GLIBC_2.38` | ⭐ **5 at glibc 2.41**, all on the one rolling distribution |
| **class C (what a move COSTS)** | unknown | ⭐ **empty on all 11 rows at both pins** |
| **the manifest rewrite** | ⛔ half-fixed: the sweep knew 5 globs, the rewrite 3 | ⭐ one list, and a **build-time check that reads DATA**, passing on the real bundle |
| **`__EGL_VENDOR_LIBRARY_FILENAMES`** | ⛔ never set; a host value silently wins | ⭐ set from the bundle's own vendors, released under `PGB_HOST_MESA` |
| **T-068's 904-object sweep** | ⛔ ad-hoc, never committed, quoted in four places | ⭐ `experiments/93-`, written and its probe verified; **not yet run** |
| **selftests** | 138 pass | **200 pass**, 1 could not run (no zstd) |
| **Entries** | 45 / 20 open / 25 done | 45 / 20 open / 25 done |

## What was actually settled

| | | |
|---|---|---|
| **T-055 / goal 3** | ⭐ **the comparison exists and is honest** | ours renders on 11 of 11 with **zero host shared objects**; the competitor renders on 11 of 11 with **4 of 11 clean**. ⛔ **The operator's bar — smaller, loads faster, runs faster — is NOT met**: 2.22× the size, and the timing columns of the run that would have settled them are contaminated |
| **T-070** | ⚠ **advanced, open** | kernel floor unchanged (3.2.0 → 3.2.0 at glibc 2.41, two instruments); class B 20 → 5; class C empty at both pins; NSS floor holds with its below-floor control firing. ⛔ The ten POCs at 2.41 are the one row left |
| **T-071** | ⚠ **advanced, open** | items 1, 2 and 5 done. Items 3 and 4 remain, and 4's untestable half is T-059's |
| **T-066** | ⚠ **advanced, open** | ⭐ **the gap is the direction the pipeline runs in** — theirs is additive from an allowlist, ours subtractive from a closure — and deletion is worth ~1/6 on the packed artefact |
| **T-072** | ⚠ **advanced, open** | route B **refuted** by measurement, route D opened and costed, neither implemented |
| **T-068** | ⚠ **advanced, open** | the harness exists; the sweep it was built on turned out to be unreproducible |

## ⛔ Findings, and not one came from reading code

1. ⛔ **The artefact cache ignored the build options.** It rebuilt when the
   *bundler* changed and not when the *invocation* did, and nothing in the
   cache path mentions one — so the `aggressive` run would have re-measured the
   `safe` bytes, silently, to the digit. That is run 2's defect in a new
   costume. Found by asking what the next run would actually do.
2. ⛔ **The soname scan was quadratic.** Found by watching `/proc/<pid>/io`
   while it ran: `rchar` advancing 14 MiB per 5 s at 101% CPU.
3. ⛔ **`__EGL_VENDOR_LIBRARY_FILENAMES` replaces `_DIRS` rather than adding to
   it**, and setting it *empty* would leave the bundle with no EGL at all.
   Found by reading libglvnd's own `LoadVendors()` instead of guessing.
4. ⛔ **The pinned digest is the per-platform manifest digest, not the OCI index
   digest**, and `docker pull` records the latter. Found by a control that
   required the method to reproduce a digest this tree already pins.
5. ⛔ **A registry 429 was being reported as "unresolved"** — indistinguishable
   in the table from a tag that does not exist.
6. ⛔ **My own NSS measurement printed `none` for every arm from an empty
   pipeline**, because of an unquoted shell variable. Caught because
   `experiments/21-` supplies an arm that must fail, and it fires.
7. ⛔ **I contaminated run 6's clock** by running builds and selftests during a
   wall-clock arm. Caught because the *competitor's fixed artefact* moved 6.7×.
8. ⛔ **`_dl_tls_static_size` was being quoted as the static TLS surplus** in
   four places. It includes the program's own `PT_TLS`, so it moves with the
   binary — implying, backwards, that a bigger binary has more room.
9. ⛔ **T-068's 904-object sweep was never committed**, so the entry's own
   Prove could not be carried out.
10. ⛔ **A `while read` loop whose child reads stdin truncates itself** —
    demonstrated at 1 of 5 rather than asserted.

## ⛔ What the next session must not read as settled

- **The operator's bar for kdenlive is not met**, and the run that would give
  a same-day `safe` vs `aggressive` timing comparison has not been done.
- **The pin has not moved.** `cfg.go` is untouched and the POC row is open.
- **`experiments/91-`, `93-` and `85-`'s new arm are written and NOT RUN.**
