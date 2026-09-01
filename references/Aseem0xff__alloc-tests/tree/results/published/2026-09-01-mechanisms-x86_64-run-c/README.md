# `mechanisms` on alpine/x86_64 — run C, the first across all allocators

Runs A and B are in [`../2026-09-01-mechanisms-x86_64/`](../2026-09-01-mechanisms-x86_64/)
and covered **mimalloc only**. This run covers **every allocator in the suite**:
16 cells, `alpine` / `x86_64` / `static-pie`, 8 samples, image built at
`194ed63`, host `Intel(R) Xeon(R) Processor @ 2.10GHz`.

**Outcome: 7 `ok`, 9 `build_failed`, 0 errors from the validator.**
⛔ The nine failures are the result, not a broken run.

## Which mechanisms actually work

| allocator | `rust-global` | `libc-surgery` | `link-override` |
| --- | --- | --- | --- |
| mimalloc | ✅ 0.597× | ✅ **0.523×** | ⛔ multiple definition |
| jemalloc | ✅ 0.591× | ⛔ undefined `__libc_malloc/free/calloc` | ⛔ |
| snmalloc | ✅ 0.608× | ⛔ undefined `__libc_malloc/free` | ⛔ multiple definition |
| rpmalloc | ✅ 0.613× | ⛔ allocator build fails | ⛔ allocator build fails |
| hardened_malloc | ✅ 0.951× | ⛔ undefined `__libc_malloc/free` | ⛔ |
| system | ✅ 1.000× (control) | — | — |

## ⭐ The headline: `libc-surgery` works for mimalloc and no one else

The `mechanisms` suite existed to test "replace the distribution's allocator" as
a general technique. Run across five allocators for the first time, it replaces
exactly one.

The reason is specific and it is in `evidence/`. Deleting musl's malloc members
from `libc.a` leaves musl's **own** remaining objects — `src/ldso/dlerror.c` —
still referencing the musl-internal aliases `__libc_malloc`, `__libc_free` and
`__libc_calloc`. The replacement archive has to supply them:

| allocator | undefined after the surgery |
| --- | --- |
| mimalloc | *none — links* |
| jemalloc | `__libc_calloc`, `__libc_free`, `__libc_malloc` |
| snmalloc | `__libc_free`, `__libc_malloc` |
| hardened_malloc | `__libc_free`, `__libc_malloc` |

⭐ **mimalloc happens to define those aliases; the others do not.** This is the
same alias recorded in `docs/AGENTS.md` §12 item 12, where mimalloc defining
`__libc_malloc` made a correctly-displaced musl binary look like it still
contained glibc's allocator and failed the identity gate. **The quirk that broke
the negative control is the quirk that makes the surgery link.**

⚠ So `libc-surgery` is not a general technique that was merely tested on
mimalloc first. As implemented it depends on a mimalloc-specific accident, and
the prior art it came from
(`references/haskell-wasm__rust-alpine-mimalloc`) is a mimalloc project — quite
possibly for this reason.

⚠ **rpmalloc fails earlier and differently**, and is the one entry here that
looks fixable: its own build breaks in override mode with
`'_ZdaPv' aliased to undefined symbol 'rpfree'`, a C++ operator-delete alias, so
no archive is produced at all. That is a recipe bug, not a musl one.

## The mechanism comparison, now three runs deep

mimalloc, the only allocator that can be compared across mechanisms:

| mechanism | run A | run B | run C | rel peak RSS (C) |
| --- | --- | --- | --- | --- |
| `libc-surgery` | **0.444×** | **0.460×** | **0.523×** | 3.465× |
| `rust-global` | 0.501× | 0.606× | 0.597× | 3.470× |

⭐ **The surgery beat the shim in all three runs** — by 13%, 32% and 12%. This is
the only comparison in this project that has survived repetition, and it did so
across a change of CPU (see `docs/AGENTS.md` §11.1). Peak RSS is identical
between the two mechanisms, as expected when both run the same allocator and
differ only in how much of the program reaches it.

⚠ **The magnitude is still not established** — 13%, 32%, 12%, against internal
MADs of 1.6–4.3%. Publish the direction.

## Reading the evidence files

`evidence/libc-surgery-<allocator>.txt` is the tail of each cell's real
`build.log`. ⚠ Use those, not `logs/<cell>.log` from a run directory: the latter
is an 11-line tail whose last line is an incidental
`undefined reference to __stack_chk_fail`, which is not the cause and will
mislead you if you take it for one.
