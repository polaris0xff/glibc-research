# SUMMARY.md — the session of 2026-09-02f / 2026-09-03

⛔ **Overwritten every session.** The history is the git log.

⭐ **The headline: the glibc pin moved, and moving it turned over four real
loader defects.** Host shared objects that `--host-dlopen` can load went
**406 → 882 of 1,527** on one machine, one population, five builds — and the
assertion that was supposed to catch two of those defects had been passing at
zero *because of* a third.

## Before and after

| | at start | at end |
|---|---|---|
| **the glibc pin** | ⛔ 2.36, "the move is *indicated*, `cfg.go` untouched" | ⭐ **2.41, landed**, gcc 14.2.0, and **CI green at it, 16 of 16, on all eleven** |
| **copies of the pin in code** | ⛔ **nine** of the name, **two** of the digest — one an `env.BUILD_IMAGE` nothing had ever read | ⭐ **three constants in `cfg.go`**, and `TODO/check.sh` fails on a copy |
| **host objects `--host-dlopen` loads** | 406 of 1,527 | ⭐ **882** |
| **`experiments/93-`'s own control** | `DIFFER = 0` | ⭐ **`DIFFER = 0`, and this one is earned** — it read 10, then 1, then 0 as defects were fixed |
| **the residue that is actually ours** | ⛔ "86 of 904", from a sweep nobody committed | ⭐ **257 objects, SIX symbols; 254 of them load** |
| **`libLLVM`** | ⛔ "dies in its 605th static constructor for a reason nobody has found" | ⭐ **loads** |
| **`experiments/21-`** | labels typed as `"2.31"`/`"2.36"` | ⭐ a **third arm that follows `cfg.go`**, every label read from the environment |
| **a POC's `RESULT.txt`** | said nothing about what built it | ⭐ environment, image, digest, gcc, glibc — and the binary's `.comment` **asserted** against it |
| **`pgb selftest`** | 200 cases | ⭐ **307**, `wrapper` and `cfg` new |
| **CI** | green | ⛔ **red for seven pushes**, then green — on a gate that passed here and failed there |
| **Entries** | 45 / 20 open / 25 done | ⭐ **45 / 17 open / 28 done** — T-068, T-070 and T-071 closed. P0: 3 open → **1** |

## What was actually settled

| | | |
|---|---|---|
| **T-070** | ✅ **CLOSED** | the pin is `debian:13`/2.41. `73-` re-run independently reproduced the prediction, `21-` re-run with a pin-following arm, ⭐ **all TEN POCs** re-run through the normal path, CI green 16 of 16 |
| **T-068** | ✅ **CLOSED** | 93- green at `pass=6 fail=0`, with the arc that earned the zero |
| **T-071** | ✅ **CLOSED** | 85- run at last — the data-coherence arm's negative control fired on a real bundle |
| **T-072** | ⚠ **advanced, and its premise is dented** | the object it was opened on, `liblsan.so`, is refused as a **sanitizer interposer** before TLS is considered. Zero of 71 `PT_TLS` objects want more than the surplus |
| **T-062** | ⚠ **advanced** | `wrapper` (55 cases) and `cfg` (37). Five packages left |

## ⛔ Findings, and not one came from reading code

1. ⛔ **`main` came up 18 commits behind** and `git switch` said *"up to date"* —
   shallow clone, before the fetch.
2. ⛔ **The record was 12 commits stale.** `TODO/runtime.md` said *"93- has not
   been RUN yet"* twelve commits after it was run twice.
3. ⛔ **`--host-dlopen` could not load anything using iconv.** The provider
   table's **weak** `extern char iconv_open[]` was rewritten by `--wrap` to a
   name no archive member was pulled for. Found by running **real host
   objects**, which is what T-072's Prove asked for.
4. ⛔ **A general-dynamic TLS pair's two halves searched different sets of
   objects**, so a cross-module thread-local bound to *(right module, offset
   0)* — one module writing into another's thread storage. Found by a SIGSEGV
   handler, because `rip=0` names nothing.
5. ⛔ **Defect 3 was HIDING defect 4.** Ten objects the control should have
   caught were failing earlier, on iconv.
6. ⛔ **The symbol lookup stopped at the first NAME match** and checked the
   version after. A versioned table holds several entries per name.
7. ⛔ **`_dl_mcount_wrapper_check` is in `libc.so.6` and not in `libc.a`** —
   247 objects, refused for want of a symbol whose right implementation is to
   do nothing.
8. ⛔ **"Failed on 631 objects" was not a defect count.** glibc's own `ld.so`
   fails **374** of them: plugins of a host *program*.
9. ⛔ **CI was red for seven pushes** on a gate that asked the **disk** where
   CI asks a fresh clone.
10. ⛔ **`pgb rootfs run` mounts a tmpfs over `/tmp`** — a probe copied there
    from outside is not there inside.
11. ⛔ **A `$?` after a pipeline is the pipeline's status.** A sweep reported
    `ok=1527 fail=0 crash=0` over a population containing 96 non-ELF files.
12. ⛔ **`chmod 000` is not a control when you are root.**
13. ⛔ **`docs/research/solo.md`'s "5,807 objects" disagreed with the table
    printed beneath it**, by 200 — and its range missed its own maximum.

## The three deep reviews, three distinct findings

⭐ `docs/methodology/reviews.md`: *"A pass that reports nothing means that pass
was too shallow."*

**1. The door sweep — "what other door reaches this code?"** Ten call sites
reach `el_lookup_in`; I had enumerated three. ⛔ **Finding: `el_defver()` and
`el_refver()` read `o->versym[si]` with no bounds check.** `versym` is a
*separate array* from `symtab`, and the walk only validates `symtab`. It was
one call after the loop before; my own fix moved it **inside** the walk and
multiplied it. Both guarded now.

**2. The guard mutation — "can my new guard actually fail?"** ⛔ **Finding: my
own measurement is thinner than my own fix.** Two mutations:

    el_accept ignores the version entirely   the motivating objects STILL LOAD
    drop the unversioned fallback            254 of 257 -- IDENTICAL

The version rule has three branches and this host's 1,527 objects exercise
**one**. *A wrong version is never returned* holds by construction and is
asserted by nothing. ⭐ Recorded rather than rounded up to "validated"; a
synthetic versioned fixture would close it and is not written.

**3. The claim audit — "which sentence is not backed by an artefact?"**
⛔ **Finding: three documents quoted `ok=628` and `631 undefined` after the tree
had moved to `882` and `376`.** `AGENTS.md` §7, `limitations.md` and
`runtime.md`, all written a few commits before the last two fixes. ⚠ **No gate
catches this and none can** — `check-docs.sh` asserts that a cited *path*
exists, not that a quoted *number* is current. Corrected against the evidence
file.

⭐ **And a fourth, from the same lens applied to my own checks:** the `cfg`
suite's container-argument case iterated `OptVars` and asked whether each was
rendered — which `ContainerEnvArgs` also iterates. It was **theatre**, and
dropping a variable from `OptVars` left it green. The binding direction has to
be **derived**: diff the environment across `Export()`.

## ⛔ What the next session must not read as settled

- ⛔ **The version-matching fix is under-measured** — see review 2.
- ⛔ **`--tls-reserve`'s measured benefit on real host objects is zero
  objects.** The mechanism is sound; its justification is not.
- ⛔ **The operator's bar for kdenlive is still NOT met** (2.22× the size), and
  a same-day `safe` vs `aggressive` timing comparison is still owed.
- ⚠ **Three host objects still do not load**, each for its own reason:
  `libnsl.so.1` (class S, sunrpc), `libmvec.so.1` (class A, `_rtld_global_ro`),
  `libcuilo.so` (`more than 64 objects loaded` — a limit of this loader's
  object table).
