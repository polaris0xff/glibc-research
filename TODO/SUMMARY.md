# SUMMARY.md — the session of 2026-09-02c

⛔ **Overwritten every session.** The history is the git log.

The four P0s the operator set on 2026-09-02b. ⭐ **Three closed, one advanced
and deliberately left open.**

## Before and after

| | at start | at end |
|---|---|---|
| **host `dlopen`** | ⛔ the project's one measured, unfixed failure. `limitations.md` §1 | ⭐ **SOLVED.** Our own ELF loader, `pgb build --host-dlopen`, **11 of 11**, zero host shared objects |
| **a glibc `.so` on a musl machine** | impossible | ⭐ **dlopen'd on Alpine ×3 and Void musl**, from one static ELF |
| **loader size vs `pg83/solo`** | solo: 2,332 code lines + 5,948 of glibc→musl shim | ⭐ **ours: 1,093 code lines, no shim** |
| **what a bundle may take from the host** | ⛔ never written down; "zero host objects" applied to bundles too | ⭐ **four classes, search order adopted, 29 offline assertions** |
| **the reachability sweep** | ⛔ existed; **nothing consumed it** | ⭐ consumed by debloat: 277 objects, 12.0 MiB on `jq` |
| **`jq` bundle vs the field** | 11,471,610 B — **2.86×** | ⭐ 4,890,913 B — **1.22×** |
| **debloat on the same closure** | 12.7% off | ⭐ **86.9% off** |
| **is C enough for `tool/runtime/`?** | an open P0 question | ⭐ **answered: yes**, 0 UBSan findings over 904 host objects |
| **Entries** | 40 / 19 open / 21 done | 42 / 17 open / 25 done |
| **`make` after editing the runtime C** | ⛔ "Nothing to be done" — shipped a stale loader | rebuilds |

## The four P0s

| | | |
|---|---|---|
| **T-064** | ✅ **closed** | `experiments/76-`, exit 0, four of four. Carried arm 11/11 with zero host objects; a **real host `.so`** on 7 of 7 glibc rows; a clean refusal on 4 of 4 musl rows; the control 0 of 11 |
| **T-065** | ✅ **closed** | `docs/design/host-fallback.md` + `internal/bundle/hostpolicy.go`. NVIDIA is host-always and not an opt-in |
| **T-066** | ⚠ **advanced, OPEN** | 2.86× → 1.22× on `jq`. ⛔ kdenlive ran and **failed to render** — my regression, fixed but **not re-measured** |
| **T-067** | ✅ **closed** | `docs/design/runtime-language.md`. C is adequate, with four named conditions that reopen it |

## ⛔ Eight defects, every one found by something disagreeing

1. **`libm.a` is a GNU ld script, not an archive** — zero symbols in silence.
   Second sighting here; a third arrived in the supplied paper.
2. **`__tls_get_addr` is in no archive** — 398 of 492 failures, one name.
3. ⛔ **`DT_RELR` ignored** — a **silent wrong answer**: the loader reported
   success and left pointers unrelocated. `init_array[0] 0x670`.
4. ⛔ **`make` did not depend on the `go:embed`'d C** — cost a full
   eleven-environment run measuring a fix that was never in the binary.
5. ⛔ **My benchmark forked per sample** — reported the loader 10× slower than
   `ld.so`; it was copy-on-write faults on a 4.4 MB static image.
6. ⛔ **The reachability sweep had no consumer** — `codegraph callers Sweep`.
7. ⛔ **The sweep then ran before `.env` existed** and deleted kdenlive's MLT
   modules. `jq` could not have caught it: a CLI has no plugin directories.
8. ⚠ **I pushed once with `TODO/check.sh` red** (stale index), having read the
   gate output after committing rather than before.

## The supplied working paper

⭐ **Useful, not slop.** Vendored at
`references/operator__one-libc-in-the-process/`, swept in
`docs/research/one-libc.md`, entry **T-069**. It supplied the source-level
cause of `experiments/72-` (glibc's dummy link map), killed the `-rdynamic`
export route twice at T1, and prompted a check on our provider table that could
have been a silent second libc — measured, and safe.

⭐ **Its own §10 limitation — *"no bridge of our own"* — is what
`experiments/76-` closed, at T1, on eleven environments, the same day.**

## ⛔ What the next session must do first

1. ⛔ **Re-measure `experiments/90-`.** The ordering fix is committed and
   unverified. Until it runs, **1.39× is a size for a bundle that did not
   render** and must not be quoted.
2. **T-066's remaining lever is not another debloat rule** — it is *where the
   closure comes from*. `Anylinux-AppImages/FAQ.md` names it: their packages
   are optimised for size, ours are nixpkgs.
3. **T-068**: `libLLVM` maps and relocates cleanly and dies in the **605th** of
   its C++ static constructors. It is the only ordinary library that does.
