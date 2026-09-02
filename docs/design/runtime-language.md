# Is C enough for `tool/runtime/`?

⭐ **Ruling: yes. C is adequate, and this document is the evidence so it is not
re-asked.** `TODO` T-067, which is explicit that *"a migration with no measured
limitation behind it is refused by this entry, not enabled by it"*.

⚠ **The operator's own expectation was the same** — *"Look into using zig if
existing c is limited/slow, thought that shouldn't be the case"* — so the
useful work here was not reaching the conclusion but **measuring it**, and in
particular finding out what would change the answer.

## The subject

`tool/runtime/` is 3,041 lines of C compiled into every artefact `pgb`
produces.

| file | lines | what it is |
|---|---|---|
| `pgb-elfload.c` | **1,578** | ⭐ the compiled-in ELF loader (T-064). Raw pointer arithmetic, `mmap`, relocation, TLS |
| `pgb-dlopen.c` | 301 | `--wrap-dlopen`, and the fall-through to the loader |
| `pgb-trace.c` | 244 | the carried `ptrace` tracer `pgb verify` uses |
| `pgb-cacert.c` | 237 | `--embed-cacert` |
| `pgb-locale.c` | 215 | `--embed-locale` |
| `pgb-terminfo.c` | 161 | `--embed-terminfo` |
| `pgb-apprun.c` | 135 | the bundle's `AppRun` |
| `pgb-nssfix.c` | 89 | the NSS override |
| `pgb-iconv.c` | 81 | the `iconv` wrappers |

⭐ **The loader is over half of it and it is the honest candidate**, which is
what makes the question answerable now rather than in the abstract: it was
written this session, it is the hairiest C in the tree, and it is the one piece
where a safer language could plausibly have paid.

## Question 1 — is any of it measurably slow?

⭐ **No, and for most of it there is nothing to be slow.** Six of the nine
files are one-shot: a constructor that calls `__nss_configure_lookup` once, or
a `--wrap` shim that adds a branch to a call the program was already making.
`evidence/40-overhead/RESULT.txt` measures the whole runtime's cost against
plain `gcc -static`:

| arm | size | per exec | peak RSS |
|---|---|---|---|
| plain `gcc -static` | 1,057,760 B | 1177 µs | 5380 KiB |
| `pgb` | 2,138,296 B | 1205 µs | 5352 KiB |

⛔ **Only the size column is a real difference**, and `docs/AGENTS.md` §10 says
why: two runs put the per-exec cost 42 µs then 28 µs above plain static and the
peak RSS 56 KiB above then 28 KiB *below* — a sign change. Startup and memory
are at or under this instrument's noise floor.

**The loader is the only piece with real work**, and it is measured in
`experiments/76-`, on debian-12, two objects in one process:

| | first load | second |
|---|---|---|
| the compiled-in loader | **147,543 ns** | 166,220 ns |
| the same static binary reaching the host `ld.so` | 711,066 ns | 41,430 ns |

⚠ The per-load figures are at the noise floor. What is not is the first
column, and the C loader is **4.8× faster** than the thing it replaces.

⭐ **So there is no slow component to point a new language at.** A language
choice that cannot name a number it would improve is a preference.

## Question 2 — is any of it hard to write correctly in C?

⛔ **This is the real question, and "no defects" would be a weak answer.** The
loader had plenty of defects. What matters is **which language layer each one
lived in**, and that is decidable.

### The instruments

| instrument | scope | result |
|---|---|---|
| `gcc -O2 -Wall -Wextra -fanalyzer` | all nine files, 3,041 lines | **5 warnings, 0 errors** |
| ⭐ **UBSan**, `-fsanitize=undefined`, running the loader over **904 real host shared objects**, one fork each | `pgb-elfload.c` | ⭐ **0 runtime errors** |
| ASan | the same | ⛔ **could not run** — see below |

⭐ **All five gcc warnings are the same false positive**, and it is a good one:

```
pgb-elfload.c:311: warning: the comparison will always evaluate as 'false'
  for the address of 'pgb_provider_syms' will never be NULL [-Waddress]
```

The symbol is **weak**, so its address *can* be NULL — that is the entire
mechanism by which a build without `--host-dlopen` still links. gcc's
`-Waddress` does not model weak linkage. ⚠ **So C's own static analysis is
wrong about the one construct this runtime most depends on, and it is wrong in
the direction that produces noise rather than silence** — which is the
survivable direction, but it is a real cost and it is why `-Waddress` cannot
simply be turned on in CI for these files.

⛔ **ASan could not run here and that is recorded as "could not run", not as a
pass.** It died in its own initialisation before any of our code executed —
`AsanInitInternal` → `AsanThread::Init` → `SetTLSFakeStack`, SEGV on the zero
page. ASan's fake-stack machinery is itself TLS-based and does not survive this
environment. An absence is not a zero: ASan found nothing because ASan did not
run.

### ⭐ The finding that actually decides it: where the defects lived

Five real defects were found while building the loader, every one by something
disagreeing rather than by reading. **Not one of them is a C-language defect**,
and a memory-safe language would have prevented none of them:

| defect | what layer it lived in | would zig have caught it? |
|---|---|---|
| `libm.a` is a **GNU ld script**, not an archive — read as `ar` it yields zero symbols in silence | a fact about the filesystem | ⛔ no |
| `__tls_get_addr` is in **no archive**; `ld.so` exports it. 398 of 492 undefined-symbol failures were that one name | a fact about the ELF ABI | ⛔ no |
| **`DT_RELR` was ignored**, so modern objects "loaded" with their pointers unrelocated — a *silent wrong answer* | a missing feature | ⛔ no |
| `make` did not depend on the **go:embed'd** C, so editing the loader used the previous one | a Makefile bug, in Make | ⛔ no |
| the **benchmark forked per sample**, reporting the loader 10× slower than `ld.so` when it was measuring copy-on-write faults | an instrument bug, in shell and C | ⛔ no |

⭐ **The one class where a safer language genuinely helps — out-of-bounds
access and integer overflow in the relocation loops — is the class UBSan
already covers, and it found zero over 904 real objects.** That is not proof
of absence; it is the strongest evidence available on this machine, and it is
the same evidence a zig build would have to produce to claim better.

## Question 3 — what would zig cost?

⛔ **The constraint T-067 names is the one that decides it.** `pgb` is one
static Go binary built `CGO_ENABLED=0` that **carries its C sources and
compiles them with the target's own toolchain**. That is why a distributed
`pgb` works on any machine with a `cc`, and it is the property a zig runtime
would give up.

| | measured |
|---|---|
| zig in the pinned Debian | ⛔ **not packaged.** `apt-cache policy zig` returns nothing in the pinned environment. ⚠ Measured when the pin was `debian:12`; T-070 has since moved it to `debian:13` and Debian still does not package zig |
| so it must be fetched | zig 0.15.2 x86_64-linux tarball **53,733,924 B**, from `ziglang.org/download/index.json` |
| against | the pinned environment's gcc, already there, already used for every other artefact — `12.2.0` when this was measured, `14.2.0` at the pin today |
| and against | `pgb` itself, 11,765,820 B, which would now be smaller than its own dependency |

⚠ **And it would be a second toolchain, not a replacement.** The application
being built is still compiled by the target's `cc` through `pgb`'s wrappers;
zig would compile only the runtime objects that get linked into it. Two
toolchains producing objects for one link is a new class of problem — ABI,
unwind tables, `--wrap` semantics — in exchange for a benefit nothing above
could name.

⭐ **`references/allyourcodebase__pipewire/src/wrap/dlfcn.zig` is real prior
art and it is worth reading, but it is not this case.** 358 lines answering
`dlopen`/`dlsym`/`dlclose` from a hard-coded table for **one** program, inside
a project that is already a zig build. `pgb-dlopen.c` is 301 lines doing the
same job from a table `pgb` **generates**, inside a project whose whole
delivery mechanism is "carry the source, use the target's compiler". The
comparison favours zig on ergonomics and C on the constraint that actually
binds.

## The ruling, and what would overturn it

⭐ **C is adequate for `tool/runtime/`. Do not migrate.**

⛔ **And "do not migrate" is not "never re-ask".** These are the named
conditions that would reopen it, so a future session has a test rather than a
prohibition:

1. **A defect that is a C-language defect.** A memory-safety or
   undefined-behaviour bug in `tool/runtime/` that reaches evidence. The list
   above is the standard: five defects, none of them C's. One that is would
   change the balance, and a second would settle it.
2. **A runtime piece that is measurably slow**, with the number and the
   workload — not a size or a startup figure at the noise floor.
3. **zig in the pinned environment.** If a future pinned image packages it, the
   53.7 MB fetch and the second-toolchain problem both disappear and only
   questions 1 and 2 remain.
4. ⭐ **A component that cannot be written correctly in C at all.** The loader
   was the candidate for this and it came out at 1,093 code lines with zero
   UBSan findings over 904 real objects — smaller than `pg83/solo`'s 2,332 for
   the same job in C++. If something harder than an ELF loader turns up, it
   gets to re-ask.

⚠ **What this document does NOT claim.** It does not claim the C is free of
defects — `TODO` T-068 carries 86 host objects that do not load, and one of
them, `libLLVM`, dies in its 605th static constructor for a reason nobody has
found yet. It claims that the defects found so far are not defects a language
change would have prevented, and that the cost of the change is measured and
the benefit is not.
