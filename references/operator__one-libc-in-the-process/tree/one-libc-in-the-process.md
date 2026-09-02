# One Libc in the Process

## A chronological investigation of runtime shared-object loading from statically linked Linux binaries

**Draft status:** pre-print (working paper)
**Date of experiments:** this study was conducted in a single working session; every measurement reported as Tier 1 (below) was re-executed immediately before this document was written and its output is reproduced verbatim in Appendix B.
**Subject systems:** glibc 2.43 (static binaries and embedded loader), musl 1.2.5 (shared objects and toolchain), binutils 2.46.1, GCC 15.3.0, Linux 6.18.39 (x86-64).

---

## Abstract

A statically linked glibc binary carries its entire C runtime in its own image, including the machinery of a dynamic loader. This paper investigates a precise, four-constraint question: **can such a binary load, at runtime, a shared object it did not link — on every environment — with its own loader — and without a second libc entering the process?** We answer it chronologically, reporting not only the final result but every intermediate hypothesis, dead end, overturned expectation, and misread question that shaped it. On the stock embedded loader (rtld-static) we find a sharp dichotomy, verified by direct experiment: self-contained objects (zero unresolved external symbols) satisfy all four constraints, while any object with external libc references either fails to load or admits a second libc into the process. The folklore remedy of exporting symbols with `-rdynamic` is shown to be inoperative twice over on a modern toolchain. A detour through musl establishes the exact composition of musl 1.2.5's static archive (a weak `dlopen` stub; no embedded loader) and documents a working recipe for static-PIE musl linking. Loading musl-built objects into the static glibc binary is then measured in both directions of environment: failure via `invalid ELF header` on glibc hosts (where `libc.so` is a linker script) and a fully mapped second libc — a different libc implementation — on musl hosts. The naive conclusion, that the four constraints are satisfiable *only* by self-contained objects, is then corrected against the published state of the art: the loader itself is a choice. Architectures that replace the loader with a userspace implementation and answer the object's libc dependency *from inside the process* — an ABI bridge over the resident runtime, or a private-copy rewrite of version requirements — satisfy all four constraints for ordinary libc-linked objects, with one libc family in the process, and this is documented and tested at scale by independent projects. We formalize the taxonomy (native loader, bridged loader, rewritten requirement, split runtime), give a constraint scorecard for each, and record the complete experimental record.

**Keywords:** ELF, dlopen, static linking, glibc, musl, rtld, symbol versioning, plugin loading, cross-libc, NSS, portability

---

## 1. Introduction

### 1.1 The research question

Static linking appears to dissolve the dependency problem: one file, every library inside it, no host requirements. The appearance holds until the process must, at runtime, absorb code that was not present at link time — a plugin, a hardware driver, a name-service module, a platform toolkit. On Linux this means `dlopen(3)`, and `dlopen` was designed for dynamically linked processes. The question this paper pursues is the exact statement:

> **Q.** Can a static glibc binary load a shared object it did **not** link, on **every environment**, with its **own loader**, and **without a second libc entering the process**?

The four clauses are named for reference:

- **C1 — runtime loading:** the object is not among the binary's build-time inputs; it is absorbed after the process starts (the `dlopen` family, or an equivalent in-process mechanism).
- **C2 — environment independence:** no constraint on the host beyond what the static binary already requires; the construction must not silently depend on the host's libc matching, or even resembling, the binary's.
- **C3 — own loader:** the mapping, relocation, and symbol resolution are performed by machinery the process itself carries; no external `ld.so` is executed, no helper process, no host dynamic loader.
- **C4 — one libc:** no second libc implementation enters the address space; one allocator domain, one TLS regime, one set of runtime state.

C4 is the constraint that does all the discriminating work, and it is not aesthetic. Two libcs in one process means two allocators (a pointer from one must never reach the other), two stdio states interleaving on the same descriptors, two TLS layouts, and — when the two libcs are different implementations — struct layouts that merely happen to overlap. The official glibc position is a link-time warning, reproduced verbatim in §3.2; it describes exactly the coupling that C2 and C4 together forbid.

### 1.2 Motivation

Three families of real systems ask this question. First, **plugin systems in portable binaries**: an application shipped as one file that still wants to adopt optional host components — a platform theme, a codec, an input method. Second, **hardware interfaces**: GPU drivers and Vulkan ICDs are host-supplied shared objects and cannot practically be bundled [FAQ], [solo]. Third, and the original motivation for this line of work, **name service switching**: the classic glibc NSS architecture resolves `getpwnam`-family calls by loading `libnss_*.so.2` modules at runtime — modules that are themselves linked against the dynamic libc — which is precisely the second-libc hazard (§3.4); packaging tools that walk this boundary report NSS modules as a known trouble spot to this day [onelf]. 

### 1.3 Contributions

1. A direct experimental settlement, on a current toolchain, of what the stock glibc static loader can and cannot do (Phase I, §5): the dummy-main-map limitation, the non-operation of symbol exporting, the successful-but-hazardous loading of libc-linked objects, and the working self-contained construction.
2. The exact static-archive composition of musl 1.2.5 with respect to `dlopen` (Phase II, §6), including a reproducible static-PIE link recipe, established in the course of a tangent that the chronicle reports honestly.
3. The first direct measurements of the musl-object-into-static-glibc-binary direction in both environment flavors (Phase III, §7).
4. A correction of the premature conclusion those measurements invited, and a taxonomy of the loader-replacement architectures that satisfy all four constraints for libc-linked objects (Phase IV, §8), each scored against C1–C4 (§9).
5. The complete record: sources, commands, verbatim outputs, and the three documented peer-review passes over this document (Appendices A–C).

### 1.4 How to read this paper

It is a chronicle as much as a report. Phases I–III contain expectations that were wrong, questions that were misread, and results that were later reinterpreted; each is marked as such where it occurs, because the shape of the dead ends is part of the evidence for the final taxonomy. Readers wanting only the answer should read §2, §9, and §11.

---

## 2. Hypotheses and evidence policy

The investigation generated three hypotheses, stated here as they were finally fixed (their earlier, wrong forms are visible in the chronicle):

- **H1 (sufficiency of the self-contained class).** A statically linked glibc binary, using only its embedded loader, satisfies C1–C4 for exactly the class of shared objects with **no unresolved external symbols** — and for no larger class.
- **H2 (the native-loader dilemma).** Under the same embedded loader, any shared object that carries external libc references (a `DT_NEEDED` on a libc, or undefined symbols against it) either fails to load or causes a second libc to be mapped into the process. There is no third outcome.
- **H3 (loader replaceability).** C1–C4 are jointly satisfiable for ordinary libc-linked shared objects when loading is performed not by the embedded rtld but by a **userspace loader under the application's control that answers the object's libc dependency from the resident runtime** — by bridging the foreign ABI onto the in-process libc, or by rewriting the object so its requirements resolve there — so that no second libc is ever mapped.

Evidence tiers, used throughout:

| tier | meaning |
|---|---|
| **T1** | Measured in this study; command and verbatim output in Appendix B |
| **T2** | Property of a published artifact verified directly here (e.g., an ELF inspected with `readelf`) |
| **T3** | Documented claim of a cited project (their measurements, their CI) — reproduced or summarized from the cited document, not ours |

H1 and H2 are settled at T1. H3 is settled at T2/T3: the artifacts and documented test records of two independent projects, plus a mechanism analysis; we did not construct a bridge of our own, and we say so plainly in §10.

---

## 3. Background

### 3.1 The static glibc binary contains a loader — with a dummy where its symbols would be

Since glibc 2.34, the dynamic loading API lives in libc itself; the release notes record the merge:

> In order to support smoother in-place-upgrades and to simplify the implementation of the runtime all functionality formerly implemented in the libraries libpthread, libdl, libutil, libanl has been integrated into libc. [...] For backwards compatibility, empty static archives libpthread.a, libdl.a, libutil.a, libanl.a are provided [...] [NEWS 2.34]

A static binary therefore contains the dynamic linker's machinery (the rtld code paths compiled for static use). What it does **not** do is present the executable's own symbols to that machinery. The static-loader path constructs a placeholder for the main program, whose defining comment in the current source reads:

```c
/* A dummy link map for the executable, used by dlopen to access the global
   scope.  We don't export any symbols ourselves, so this can be minimal.  */
static struct link_map _dl_main_map = { ... };
```

— `elf/dl-support.c`, glibc master, fetched during this study

That comment, seventeen words of it, predicts most of Phase I. If the dummy map has no symbol table, then no matter what the executable's ELF headers contain, a loaded object can resolve nothing against it.

The historical record around this feature is itself a plot twist worth recording: glibc 2.31 *deprecated* static-`dlopen` outright —

> Support for statically linked applications which call dlopen is deprecated and will be removed in a future version of glibc. [NEWS 2.31]

— and 2.34 then reversed course by making the machinery permanent via the libdl merge above, adding one behavioral note for the static case (HWCAP subdirectory variants are not loaded) [NEWS 2.34]. Separately, `dlmopen` has never been available to static binaries [NEWS 2.27].

### 3.2 The double-libc hazard

When a shared object with `DT_NEEDED: libc.so.6` is loaded into a static glibc process, the embedded loader satisfies that dependency the ordinary way: it finds and maps a dynamic libc. The result is one process, two complete libc instances. GCC states the operating envelope at link time:

```
warning: Using 'dlopen' in statically linked applications requires at runtime
the shared libraries from the glibc version used for linking
```

(T1, E17). "Same version used for linking" is a best case, not a safety argument: even two identical copies maintain separate allocator, stdio, and TLS state. The pre-2.34 NSS architecture put this hazard on the critical path of every static binary that resolved a user or host name (§3.4).

### 3.3 Symbol versioning: which direction fails

glibc DSOs carry *versioned* symbol references (`malloc@GLIBC_2.2.5`, or newer versions for newer symbols). A versioned reference demands a provider that defines the version; pointing it at an older or unversioned provider fails with the well-known `version 'GLIBC_2.x' not found`. musl has no symbol versioning at all: references compiled against musl are *unversioned*, and an unversioned reference binds to whatever compatible definition the loader's scope offers. This asymmetry — **versioned requests are strict, unversioned requests are permissive** — is what makes the glibc-runtime-hosts-musl-objects direction tractable and the reverse expensive (§8.4).

### 3.4 NSS heritage

The historical glibc design for `getpwnam`/`gethostbyname`-family calls loads `libnss_<source>.so.2` modules at runtime; those modules link against the dynamic libc. In a static binary this is the double-libc hazard wearing a system costume — the module is exactly the kind of libc-referencing object Phase I measures. Whatever the current default module set does, non-default NSS backends and any tool that still walks this boundary remain exposed, and current packaging tooling still lists "nss / nsswitch libraries" among its cross-libc trouble spots [onelf].

### 3.5 musl's static posture

musl takes the opposite default: its static archive (1.2.5, measured in §6) contains **no embedded loader at all** — only a weak stub. The entire question for a static musl binary is therefore different, and shorter.

---

## 4. Environment and methodology

| component | version (E1, T1) |
|---|---|
| C library (host) | glibc 2.43-r2 (Gentoo patchset 3) |
| Compiler | GCC 15.3.0 (Gentoo p8) |
| Linker | GNU ld 2.46.1 |
| Kernel | 6.18.39 (x86-64) |
| Second libc | musl 1.2.5, built from source during the study |
| Debugging | `gdb` and `strace` present but **non-functional in this sandbox**: every `ptrace` operation is denied (`PTRACE_TRACEME: Operation not permitted`); all diagnosis was by marker instrumentation and ELF inspection |

Method: every claim tagged T1 was produced by a build-and-run cycle whose full source and command appear in Appendix A, and whose output appears verbatim in Appendix B. Where an expectation was falsified, the falsification is reported in place. Reference documents for T3 claims were fetched in full during the study and re-read completely twice before this document was drafted; they are listed in §References with the file names under which they were kept during the study.

---

## 5. Phase I — the static glibc binary and its stock loader

### 5.1 First construction: the folklore recipe, and two findings

*Expectation at outset:* the well-known recipe — link the static binary with `-rdynamic` (`--export-dynamic`) so its symbols, including the libc ones copied in from `libc.a`, appear in a dynamic symbol table; build plugins freestanding; let the plugin resolve host services against those exports. This recipe circulates in older plugin-loading documentation.

Two measurements broke it:

**F1 (T1, E2/E3).** On binutils 2.46.1, `gcc -static -Wl,--export-dynamic` produces a binary with **no dynamic section whatsoever**:

```
$ readelf -d host_static
There is no dynamic section in this file.
```

The flag is accepted and has no effect on a `-static` link. Raw `ld -static -E` on a trivial object behaves the same. `--export-dynamic-symbol-list=<file>`, by contrast, *does* create a `.dynsym` (with `GNU_HASH`, `STRTAB`, `SYMTAB`) — but only when the output is position-independent (`-static-pie`); on plain `-static` it too yields nothing. This is a toolchain fact, not a libc fact; it invalidates the folklore recipe at step one on current binutils.

The binary also states its kernel floor in its ELF notes (`for GNU/Linux 3.2.0`, E18) — the "every environment" of C2 bounded below only by the kernel, as expected for a static binary.

### 5.2 The load that should not happen

*Expectation at outset (wrong, and corrected by experiment):* that the embedded loader would *refuse* to load a DSO carrying `DT_NEEDED: libc.so.6` — that some check added in modern glibc turns the double-libc hazard into a clean error. This expectation appears in scattered community commentary; it is **false** on glibc 2.43.

**F2 (T1, E4/E5).** `dlopen("./plugin_libc.so", RTLD_NOW)` from the plain static binary **succeeds**. The plugin is mapped, its symbols resolve, its function runs. There is no rejection. (The instrumentation printed `LOADED (unexpected!)` — the marker of an overturned expectation kept in the final test source.)

### 5.3 The second libc, observed live

**F3 (T1, E6).** Immediately after that successful load, `/proc/self/maps` contains a full dynamic glibc — five segments of `libc.so.6` — alongside the static one in the binary's own image. `dlsym(handle, "malloc")` returns an address in the second libc's allocator domain; the probe labels it `TWO ALLOCATORS`. The plugin's `printf` executes through the second libc and works. A stress probe then frees a pointer allocated in one domain using the other domain's `free`, in both directions: **both survive**. Survival is not safety: both copies are the same glibc version's ptmalloc, so chunk metadata merely happens to be interpretable; the documented envelope (E17) requires exactly this version coincidence, and C2 forbids relying on it.

**F4 (T1, E6).** A control probe in the same binary: `dlsym(RTLD_DEFAULT, "malloc")` **returns NULL** before any `dlopen` — a static glibc binary cannot even look up *its own* libc symbols through the dl API. Consistent with §3.1: there is no dynsym to search.

### 5.4 Exports are dead letters: the dummy map confirmed

**F5 (T1, E5/E7).** With `-static-pie` and `--export-dynamic-symbol-list`, the binary *does* carry a well-formed dynamic symbol table (six entries on the test binary, including `malloc` and an application-defined `host_log`, each with a real definition and hash coverage). A freestanding plugin whose *only* external reference is `host_log` still fails:

```
./plugin_logonly.so FAIL: ./plugin_logonly.so: undefined symbol: host_log
```

Resolution against the main program never happens, because the loader's model of the main program is the dummy map of §3.1, which consults nothing. The dynamically linked control binary loads the same freestanding object without incident (E7 note) — and because both probes use `RTLD_NOW`, its successful `dlopen` *is* the proof that `host_log` resolved, since eager binding would otherwise have failed the load. This converts the source comment into a measured behavior: **no construction of the executable's symbol tables can make a static glibc binary export anything to a loaded object.**

**F6 (T1, E4).** Corollary, measured with a weak-reference probe: a plugin whose only external reference is a weak `dlvsym` finds it unresolved (NULL) — a loaded object **cannot call the dl API itself**, not even to load further objects.

### 5.5 What does work on the stock loader

**F7 (T1, E4/E8/E9).** Self-contained objects — zero named undefined symbols, zero `DT_NEEDED` (verified by `readelf`: the only UND entry is the mandatory null symbol) — load cleanly from any of the static test binaries, execute, and can be `dlclose`d. The workable interface is the **API-table handoff**: the host passes a struct of function pointers to a single `plugin_init` entry point; the plugin returns its own function-pointer table; no relocation ever points outward. `libc.so*` and `ld-linux` map counts remain zero throughout — C1, C3, C4 all hold, and C2 holds because a self-contained object is just ELF: it has no libc identity to mismatch.

**F8 (T1, E8).** Inter-plugin structure works: a freestanding plugin with `DT_NEEDED: ./plugin_min.so` resolves symbols in its dependency and the cross-plugin call succeeds (`dep_entry() = 43`). Dependency graphs among self-contained objects are fine; it is only the boundary back into the executable that is sealed.

Phase I result: **H1 and H2 confirmed at T1** (for this toolchain). The stock loader gives a dichotomy — self-contained (all four constraints) or libc-referencing (violates C2 and C4, or fails) — with nothing between.

---

## 6. Phase II — the musl tangent

*Status of this phase in the chronicle:* it began as a misreading of the research question — the author of the question meant "can the static glibc binary load objects *from a musl environment*"; the investigation instead asked "can a static *musl* binary load objects". The tangent is retained in full because it produced the musl-side facts that Phase IV's taxonomy needs, and because the misreading itself is instructive.

### 6.1 "Dynamic loading not supported"

musl 1.2.5 was built from the official release tarball and installed locally; all musl-side objects and binaries were produced with its `musl-gcc` wrapper. A statically linked musl binary calling `dlopen` receives, for every request:

```
[1] FAIL: Dynamic loading not supported
```

**F9 (T1, E12).** The string is exact and unconditional — the same failure for every object, including self-contained ones that would load fine under glibc's static loader.

### 6.2 Why: the archive contains a stub, and only a stub

**F10 (T1, E15).** musl's static `libc.a` was inspected directly:

```
$ ar t libc.a | grep -E "dlopen|dynlink"
dlopen.lo
$ nm libc.a | grep -cE ' T __dls2| T __dls3'
0
```

The real loader (`ldso/dynlink.c`, whose stage functions are `__dls2`/`__dls3`) is **not in the static archive at all**; the archive's `dlopen` is this file, quoted in full:

```c
/* musl 1.2.5, src/ldso/dlopen.c */
#include <dlfcn.h>
#include "dynlink.h"

static void *stub_dlopen(const char *file, int mode)
{
	__dl_seterr("Dynamic loading not supported");
	return 0;
}

weak_alias(stub_dlopen, dlopen);
```

The Makefile confirms the composition: static archive members are drawn from `obj/src/*` and `obj/compat/*`; the loader objects (`obj/ldso/*`, `.lo`) are linked only into `libc.so`. A static musl binary that wanted the real loader would need to link it explicitly — nothing in the default archive pulls it in.

### 6.3 The static-PIE saga

Chasing the (mistaken) idea that musl's static-PIE startup (`crt/rcrt1.c`, which includes `ldso/dlstart.c` and self-relocates via the executable's `.dynamic`) might initialize a real loader, the study constructed a static-PIE musl binary by hand and hit three successive toolchain traps, each worth recording:

1. **`gcc -static -pie` written as two flags silently produces ET_EXEC** (non-PIE) — and, in the same measurement, an output with no dynamic section and no relocations at all — which then crashes at startup in the PIE-only startup code.
2. **Raw `ld -static -pie` produces a correct ET_DYN with `R_X86_64_RELATIVE` relocations — and injects `PT_INTERP: /lib/ld64.so.1`**, a nonexistent interpreter, so `exec` returns ENOENT.
3. The fix, visible in `gcc -### -static-pie`: GCC's single-flag form passes `--no-dynamic-linker`, suppressing the injection.

**F11 (T1, E13).** The working recipe, verified to run (`static-pie musl alive`):

```sh
gcc -static-pie -nostdlib -nostartfiles \
    -Wl,--export-dynamic-symbol-list=exports.txt -o out \
    $MUSL/lib/rcrt1.o $MUSL/lib/crti.o \
    $(gcc -print-file-name=crtbeginS.o) prog.o \
    $MUSL/lib/libc.a $(gcc -print-libgcc-file-name) \
    $(gcc -print-file-name=crtendS.o) $MUSL/lib/crtn.o
```

Two further notes: the self-relocator needs the binary to *have* a `.dynamic` section with relocations to process (the ET_EXEC mis-build of trap 1 had none), which the export-list flag guarantees by forcing a dynamic section into the output; and even a correctly built static-PIE musl binary still answers `dlopen` with the stub of §6.1, because the archive composition (F10) is unchanged. musl's `musl-gcc` wrapper additionally cannot produce static-PIE directly on this toolchain: its specs unconditionally select `Scrt1.o` and the link acquires `PT_INTERP: /lib/ld-musl-x86_64.so.1` (ENOENT on a glibc host).

### 6.4 Outcome

The tangent closed with a negative and a positive: static musl binaries have no `dlopen` at all (any plugin architecture for them must replace the loader entirely — a fact that Phase IV's first case study exploits deliberately), and the static-PIE link recipe above is documented for reuse.

---

## 7. Phase III — the corrected question: musl objects into the static glibc binary

Restated correctly, the question assigns roles: the **static glibc binary is the constant**; the shared object comes from a **musl environment** (built against musl, `DT_NEEDED: libc.so`). Two test objects were used: a musl-toolchain-built freestanding plugin (zero unresolved symbols) and the same trivial plugin built normally against musl (`DT_NEEDED: libc.so`). Two environments were simulated: a glibc host (nothing named `libc.so` on the loader's search path except the system file) and a musl host (musl's `libc.so` discoverable via `LD_LIBRARY_PATH`).

### 7.1 glibc environment

**F12 (T1, E10).** The freestanding musl-built plugin loads and runs (`run() = 42`, `libc.so` maps = 0) — toolchain-independence of self-contained objects confirmed from the other side. The libc-linked plugin fails with:

```
[B] musl-libc-linked plugin FAIL: /lib64/libc.so: invalid ELF header
```

The mechanism is almost comic: on a glibc host, the name `libc.so` resolves to the **GNU ld linker script** (a text file, `INPUT(...)` directives), which the loader dutifully tries to parse as an ELF. The failure is environmental, not architectural — and therefore exactly the kind of thing C2 forbids depending on.

### 7.2 musl environment

**F13 (T1, E11).** With musl's `libc.so` discoverable, the load **succeeds** — and `/proc/self/maps` shows musl's libc mapped in five segments, alongside the static glibc baked into the binary. One process, **two different libc implementations**. The plugin's `printf` executes through musl's stdio, inside the static glibc process, and returns normally. Nothing coordinated the two runtimes; nothing failed this time. Every sentence of §3.2's hazard description applies, with the aggravation that the two libcs here do not even share an implementation lineage: allocator metadata formats, `FILE` layouts, TLS layout — agreement is coincidental, version-locked to this pair of builds, and unmaintained by any contract.

### 7.3 An honest anomaly

During this phase, one build of the test host (an early `-O2` revision) **segfaulted** immediately after its first `dlopen` returned, before any output could flush; a fully marker-instrumented rebuild of the same logic ran clean at `-O0` and `-O2` (eight consecutive runs). The crash was never reproduced and its cause was not identified; it is recorded here as an anomaly of the study (the sandbox's disabled `ptrace` prevented post-mortem), and as a standing caution that results in officially-unsupported territory are provisional even when they "work".

### 7.4 The premature conclusion

At the close of Phase III the study concluded: *the four constraints are satisfiable exactly and only by the self-contained construction; a libc-referencing object gets you failure on the wrong host or a second libc on the right one — there is no third outcome.* Relative to the stock loader this is correct, and H1/H2 stand. As a general claim it is **false**, and the falseness is the subject of Phase IV. The error was an implicit assumption smuggled in by the phrase *with its own loader*: that "its own loader" must mean *the loader glibc shipped in libc.a*. The assumption is natural — the loader is literally inside the binary — but nothing forces an application to use it.

---

## 8. Phase IV — the correction: the loader is a choice

Two published projects, working in mirrored directions, hold the one-libc line (C4) for ordinary libc-linked objects — and between them they cover both process shapes: "solo completes a **static** binary; this completes a **dynamic** one" [cld-alternatives]. Their architectures differ, and the difference maps exactly onto the two halves of H3.

### 8.1 Replace the loader, bridge the ABI: solo

solo [solo] links a userspace ELF loader into a fully static binary — musl-static, by design — and uses it to load **unmodified host glibc DSOs**. Its loader implements the full ld.so contract in miniature: segment mapping, `DT_NEEDED` walking, versioned symbol resolution with the unversioned-provider compatibility rule, x86-64 relocations, IFUNC materialization, all four TLS models (initial-exec served from a surplus arena that rides in the executable's own static TLS), RELRO, lazy PLT with argument-register preservation, `/etc/ld.so.cache`, `DT_SYMBOLIC`, `RTLD_DEEPBIND` [solo]. The decisive move for C4:

> glibc is deliberately *not* loaded. Imports such as `malloc@GLIBC_2.2.5` are resolved [...] to ABI-correct adapters over the process's existing musl runtime. [solo]

A glibc-import is answered **in-process**, by an adapter over the resident musl runtime (a hand-written bridge of ~6,000 lines [solo-findings]); the glibc the DSO was built against never enters. Unsupported glibc functions get per-symbol fail-loud stubs. One libc family, one TLS world, one unwinder (C++ exceptions cross the boundary in both directions through the single embedded unwinder).

Evidence tier: the project publishes a prebuilt proof binary; we verified directly (T2, E16) that `vulkan-x86_64` is a 2,300,272-byte statically linked ELF with **zero `PT_INTERP`** and **no dynamic section** — an artifact that is, by construction, carrying its own loader. Its documented test record (T3): CI loads the shared libraries of the most-installed Debian packages — over two thousand objects by the project's count [solo] — on x86-64 and aarch64, and the Vulkan proof runs against real host drivers (AMD, Intel, NVIDIA, Asahi, Termux, WSL). An independent code sweep by the second project below read solo's source at a pinned commit and confirmed the version-matching loader and the bridge structure, while correcting one numeric claim about corpus counts [solo-findings].

### 8.2 Keep the host loader, rewrite the object: cross-libc-dlopen

cross-libc-dlopen [cld] addresses the mirror case: a **dynamically linked** process that bundles its own glibc, wanting to load a host object built against musl, or against a newer glibc. It is an `LD_PRELOAD` interposer on `dlopen` that:

> rewrites the host object in a private copy so symbol version requirements stop mattering. [cld]

The private copy's version requirements are stripped (or, under `CROSS_LIBC_DLOPEN_NOSTRIP=1`, kept, to bisect rewrite-versus-path failures); symbol renaming and version-trap forwarders handle the remainder [cld]. Versioned references — the strict direction of §3.3 — stop gating the load, and the object's unversioned or resolvable references bind against the **in-process** libc. The project's first invariant, asserted by a test binary on every run, is the subject of this paper stated as law:

> **Exactly one libc family in the process.** The whole design is that a second libc never enters. [cld]

Its documented record (T3): tested across Ubuntu 12.04–22.04, Alpine, Arch, Artix, NixOS, Slackware, with a per-host measured report [cld]. The project frames the problem as two gaps, and this paper addresses only the first: the libc gap. The second gap — OpenGL's fragmented dispatcher conventions (glvnd vs. non-glvnd hosts, missing `libGLX_<vendor>` or GLES libraries) — is solved in that project by a family of forwarding shims built with the SONAME of the library they replace [cld], and is orthogonal to the libc question studied here. Its limits document contains a passage this study can close cleanly: it distinguishes **three static-binary cases** — static musl (`dlopen` "a stub: it fails, always"), static glibc (`dlopen` works, with the link-time warning), and mostly-static-dynamic-libc — and marks all three **UNVERIFIED** in that repository [cld-limits]. The present study measured exactly those three cases directly: E12 verifies the first (F9), E4–E6 the second (F2–F4); the third is ordinary dynamic behavior. Two independent projects arriving at the same three-way split, one by reasoning and one by experiment, is the kind of agreement worth writing down.

### 8.3 The split-runtime family: a fourth architecture that fails C4

A family of prior systems gives static binaries dynamic loading by **bootstrapping the system's dynamic linker in-process** and letting a second runtime coexist: detour drives the host `ld-linux` through a stub ELF and a `setjmp`/`longjmp` trampoline, explicitly advertising "multiple C runtimes in the same process", x86-64 only, with its helper pinned to 2002-era glibc symbol versions for maximum host compatibility [detour]; Cosmopolitan's `cosmo_dlopen` and an experimental ClickHouse userspace loader follow the same split-runtime scheme [solo]; a published experiment in the same family swaps TLS pointers in assembly trampolines around every foreign call [solo]. These are real, working systems — and by construction they violate C4: two libc states, two TLS worlds, and callbacks that cannot safely cross because the foreign side invokes them under the wrong TLS. solo's comparison names the boundary precisely and takes the opposite side of it [solo].

### 8.4 The direction asymmetry

Why does solo choose a musl-static carrier, and why does the bundled-glibc world find the musl-object direction the easy one? Per §3.3: musl-compiled references are unversioned and draw on a smaller API surface, so a resident **glibc** can satisfy a musl-built object with little more than dependency bookkeeping (Phase III's F13 shows the raw symbol-binding side working by accident); the reverse — a musl runtime satisfying glibc-built objects — requires a real bridge, because the glibc API surface is larger and its references are versioned. The AnyLinux FAQ states the asymmetry from the packaging side ("we are able to dlopen optional libraries on the host even when those link to musl. If we used musl the opposite is usually not possible" [FAQ]); the same FAQ adds two practical reasons the bundled-glibc world stays glibc — the proprietary NVIDIA driver's glibc linkage and allocator performance concerns with musl [FAQ] — and solo embodies the reverse direction as a ~6,000-line bridge. The bridge also repairs ABI *layout* divergences that no loader can reach from outside — measured examples include `regmatch_t` stride and `FTW_*` constant shifts, fixed by call-level trampolines that translate at the boundary [solo-findings] — a category of hazard invisible to symbol resolution and orthogonal to everything Phase I could measure.

### 8.5 The packaging boundary: onelf

Completing the landscape, onelf [onelf] attacks the problem at pack time rather than load time: it bundles an application's whole dependency closure — including, for a musl application, the musl libc under both names a host might request — and injects an `AT_EXECFN` bootstrap that maps the *bundled* loader at startup, so "the host's own loader is never consulted" [onelf-cross-libc]. The deployment history that made this the standard shape is itself part of the FAQ's record: a truly portable bundle needs to ship its own dynamic linker, but executables cannot carry a *relative* `PT_INTERP` path — a limitation once attacked by rewriting `PT_INTERP` into `PT_LOAD`, and settled instead by executing the bundled linker first and handing it the payload [FAQ]. onelf's own guide names the wall this paper's Phase III measured: host libraries that link against the other libc family, and NSS modules specifically [onelf-cross-libc] — the same wall cross-libc-dlopen exists to remove at runtime [cld].

---

## 9. Synthesis

### 9.1 Mechanism taxonomy and constraint scorecard

| architecture | exemplar | C1 runtime load | C2 every env | C3 own loader | C4 one libc | evidence |
|---|---|---|---|---|---|---|
| Stock embedded loader + self-contained object | this study, §5.5 | ✅ | ✅ (object is bare ELF) | ✅ | ✅ | T1 |
| Stock embedded loader + libc-referencing object | this study, §5.2–5.3, §7 | ✅ | ❌ (version-locked, or host-shape-locked) | ✅ | ❌ (second libc mapped, measured) | T1 |
| Userspace loader + ABI bridge over resident libc | solo | ✅ unmodified host DSOs | ✅ (binary static; host contributes only the object) | ✅ (loader linked into the binary) | ✅ (foreign libc "deliberately not loaded") | T2+T3 |
| Host loader + private-copy requirement rewrite | cross-libc-dlopen | ✅ | ✅ within documented host matrix | ⚠ (uses the process's own ld.so; the *rewrite* is its own) | ✅ (invariant asserted by test) | T3 |
| Split runtime (bootstrap foreign ld.so) | detour, cosmo_dlopen, ClickHouse loader | ✅ | ✅ | ⚠ (drives the host's loader in-process) | ❌ (second libc by design) | T3 |

### 9.2 Answers to the research question

**Under the stock loader (the literal reading):** yes, exactly for self-contained objects, and never for libc-referencing ones — H1 and H2, T1 throughout.

**Under the engineering reading ("its own loader" = a loader it carries):** yes, for libc-referencing host objects up to the loader's documented coverage — solo states its ABI coverage explicitly and makes unsupported calls fail loudly rather than silently, and cross-libc-dlopen states that "the host's graphics stack is the ceiling" — provided the loader answers the object's libc dependency from the resident runtime instead of loading it. H3, at T2/T3.

**The musl sub-question** ("can it load an object *from a musl environment* without a second libc"): on the stock loader, no — `invalid ELF header` on glibc hosts (F12) or a second, foreign-implementation libc on musl hosts (F13). Under a bridged or rewriting loader, yes — this is precisely the direction the bundled-glibc ecosystem demonstrates [FAQ], [cld]. And the freestanding sub-case is toolchain-blind in both directions (F7, F12): a zero-dependency object built by a musl toolchain loads into a static glibc binary, own loader, no second libc, T1.

### 9.3 Design guidance

1. If your plugin surface is under your control, the self-contained construction is unconditional, dependency-free, and tested here at T1: freestanding build (`-nostdlib -ffreestanding`), no TLS, no exceptions, no stack protector (each creates an unresolved external), one `init(const struct host_api *)` entry point, function-pointer tables in both directions. Audit with `readelf --dyn-syms` until the only `UND` entry is the mandatory null symbol.
2. If you must absorb arbitrary host objects, you must own the loader: either bridge (solo-style, works from a fully static binary) or rewrite (interposer-style, works from a dynamic one). Do not ship the middle row of §9.1 — it is the one combination that appears to work while violating the constraints.
3. Beware the version trap in both directions: a versioned reference never binds to an older provider, and an unversioned one binds almost anywhere — which is why Phase III's hazard "worked".
4. State struct-ABI divergences are a separate axis from symbol resolution (§8.4); any bridge must translate at call level, not just bind symbols.

---

## 10. Limitations

1. **Single toolchain.** All T1 measurements come from one environment (glibc 2.43-r2 / GCC 15.3.0 / binutils 2.46.1 / kernel 6.18.39, Gentoo). F1 in particular is a binutils behavior and may differ elsewhere; the study's NEWS citations bound the libc-side history, not the toolchain's.
2. **No bridge of our own.** H3 is supported at T2/T3 — a verified static artifact plus two projects' documented, reproducible test records — but this study did not construct, run, or independently re-measure a bridged loader end-to-end. The solo artifact was verified structurally (static, no interpreter, no dynamic section) and not executed.
3. **musl 1.2.5 only**, and only the default archive configuration; musl's loader-outside-the-archive composition is stated as measured, not as universal (the cited limits document itself warns to confirm the static-musl case per version [cld-limits]).
4. **The Phase III anomaly (§7.3)** remains unexplained; no `ptrace` was available for post-mortem.
5. **Vocabulary risk.** "Every environment" (C2) is interpreted as *no dependence on host libc shape or version*; it does not extend to missing files (a host with no driver at all), seccomp-confined mappings, or non-Linux kernels.

---

## 11. Conclusion

The four-constraint question has a two-part answer, and the parts are separated not by libc but by *who owns the loader*. The loader that a static glibc binary carries imposes a dichotomy measured end-to-end here: self-contained objects satisfy every constraint; libc-referencing objects violate the one-libc constraint or the environment-independence constraint, with no third outcome — the folklore export recipe being doubly dead on current toolchains (no dynsym emitted; and even a hand-built dynsym is a dead letter to the dummy main map). The moment the application ships its *own* loader in the meaningful sense — a userspace loader that answers foreign libc imports from the resident runtime, or an interposer that rewrites the requirement itself — the dichotomy dissolves: unmodified, libc-linked host objects load into a fully static process with one libc family inside, a result demonstrated at scale by independent projects in both directions of the libc divide. The chronicle's dead ends — the expected rejection that never came, the misread question, the toolchain traps, the crash that would not reproduce — are retained because they mark exactly where the real boundaries lie: not where the documentation implies, and not where the folklore says, but where a loader's model of the world stops matching the ELF in front of it.

---

## References

All reference documents were fetched in full during the study, stored under `/workspace/refs/` during the working session, and re-read completely twice before drafting.

- **[NEWS]** GNU C Library `NEWS`, release/2.34 branch (contains cumulative history for 2.27–2.34). Source: sourceware.org gitweb, `glibc.git`, `NEWS` at `release/2.34/master`. Quotes: 2.34 libdl merge; 2.31 static-dlopen deprecation; 2.27 dlmopen unavailability; 2.34 HWCAP note. (T3, quotes verified in-session.)
- **[dl-support]** glibc `elf/dl-support.c`, master, sourceware.org gitweb. Dummy main map comment and initializer. (T3.)
- **[solo]** pg83, *solo — a `.so` loader for static Linux binaries*, github.com/pg83/solo, README (release binary verified structurally: T2, E16). Claims of bridge size, CI corpus, and GPU matrix: T3.
- **[solo-findings]** pkgforge-dev/cross-libc-dlopen, *Reference sweep: pg83/solo*, `docs/history/references/solo-findings.md`. Independent source-level audit at pinned commit; corrections to corpus counts; ABI-layout hazard inventory. (T3.)
- **[cld]** pkgforge-dev, *cross-libc dlopen*, github.com/pkgforge-dev/cross-libc-dlopen, README: interposer design, private-copy rewrite, one-libc invariant and `tests/invariants.c`, runtime switches, host matrix. (T3.)
- **[cld-limits]** pkgforge-dev/cross-libc-dlopen, `docs/limits.md`: "Static binaries: three cases, not one" (marked unverified there; verified by this study's E4–E6, E12). (T3.)
- **[cld-alternatives]** pkgforge-dev/cross-libc-dlopen, `docs/alternatives.md`: four-position comparison, "solo completes a static binary; this completes a dynamic one". (T3.)
- **[FAQ]** pkgforge-dev, *Anylinux-AppImages FAQ*, `FAQ.md`: relative-interpreter impossibility, sharun/AT_EXECFN lineage, glibc-vs-musl bundling rationale, direction asymmetry, Qt6/Alpine-GTK3 demonstration, solo/detour assessment, cross-libc-dlopen integration. (T3.)
- **[detour]** graphitemaster, *Detour*, github.com/graphitemaster/detour, README: in-process `ld-linux` bootstrap via stub ELF and `setjmp`/`longjmp` trampoline; multiple C runtimes by design. (T3.)
- **[onelf]** QaidVoid, *onelf*, github.com/QaidVoid/onelf. (T3, referenced via [cld] and [onelf-cross-libc].)
- **[onelf-cross-libc]** QaidVoid/onelf, `docs/guide/cross-libc.md`: pack-time bundling, `AT_EXECFN` bootstrap, `--strict-libc`, NSS-modules limitation. (T3.)

---

## Appendix A — Complete sources and build commands

All programs below are verbatim as built during the study. Common plugin build flags: `gcc -shared -fPIC -nostdlib -fno-stack-protector -fno-builtin` (freestanding) or plain `gcc -shared -fPIC` (libc-linked). Host builds: `gcc -static -O2 [-Wl,--export-dynamic-symbol-list=exports.txt] ... [-ldl]`, plus the static-PIE musl recipe of §6.3 where noted. `musl-gcc` refers to the locally built musl 1.2.5 wrapper.

### A.1 `host.c` — first construction (Phase I, §5.1)

```c
/* host.c - statically linked glibc binary that dlopens a DSO it did not link.
 * Built: gcc -static -O2 -Wl,--export-dynamic host.c -o host_static -ldl
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void *host_alloc(unsigned long n) { return malloc(n); }
void   host_free(void *p)         { free(p); }
int    host_log(const char *s)    { return puts(s); }

static int maps_count(const char *needle)
{
    FILE *f = fopen("/proc/self/maps", "r");
    char line[512];
    int n = 0;
    if (!f) return -1;
    while (fgets(line, sizeof line, f))
        if (strstr(line, needle)) n++;
    fclose(f);
    return n;
}

static void probe(const char *tag)
{
    printf("[%s] libc.so.6 maps: %d, ld-linux maps: %d\n",
           tag, maps_count("libc.so.6"), maps_count("ld-linux"));
}

typedef long (*fn_t)(void);

int main(void)
{
    probe("start (static, before any dlopen)");
    void *host_malloc = dlsym(RTLD_DEFAULT, "malloc");

    void *h1 = dlopen("./plugin_free.so", RTLD_NOW);
    if (!h1) { printf("free plugin FAILED: %s\n", dlerror()); return 1; }
    probe("after dlopen(plugin_free.so)");

    fn_t probe_fn = (fn_t)dlsym(h1, "plugin_probe");
    fn_t run_fn   = (fn_t)dlsym(h1, "plugin_run");
    printf("plugin_probe(): &malloc resolved inside plugin = %p\n"
           "  dlsym(RTLD_DEFAULT,\"malloc\") in host        = %p\n"
           "  -> %s allocator\n",
           (void *)probe_fn(), host_malloc,
           (void *)probe_fn() == host_malloc ? "SAME (one libc)" : "DIFFERENT (two libcs!)");

    printf("plugin_run() returned: %ld\n", run_fn());
    dlclose(h1);
    probe("after dlclose");

    void *h2 = dlopen("./plugin_libc.so", RTLD_NOW);
    if (!h2)
        printf("libc-linked plugin: dlopen REJECTED, dlerror:\n  %s\n", dlerror());
    else
        printf("libc-linked plugin: LOADED (unexpected on glibc >= 2.34)\n");
    probe("after dlopen attempt (plugin_libc.so)");

    return 0;
}
```

### A.2 `plugin_free.c` — freestanding plugin with libc references (dead-letter probe)

```c
/* Built: gcc -shared -fPIC -nostdlib -fno-stack-protector -fno-builtin
 *        plugin_free.c -o plugin_free.so
 * readelf: zero DT_NEEDED; UND = { free, host_alloc, host_free, host_log, malloc }
 */
extern void *malloc(unsigned long);
extern void  free(void *);
extern void *host_alloc(unsigned long);
extern void   host_free(void *);
extern int    host_log(const char *);

long plugin_probe(void) { return (long)(void *)&malloc; }

long plugin_run(void)
{
    char *a = malloc(64);
    char *b = host_alloc(64);
    for (int i = 0; i < 63; i++) { a[i] = 'a'; b[i] = 'b'; }
    a[63] = 0; b[63] = 0;
    host_log("plugin_run: allocated via malloc() and host_alloc()");
    free(a);
    host_free(b);
    return 42;
}
```

### A.3 `plugin_libc.c` — ordinary plugin (`DT_NEEDED: libc.so.6`)

```c
/* Built: gcc -shared -fPIC plugin_libc.c -o plugin_libc.so */
#include <stdio.h>
long plugin_run(void) { printf("hello from the libc-linked plugin\n"); return 7; }
```

### A.4 `plugin_min.c` — self-contained plugin (the working construction)

```c
/* Built: gcc -shared -fPIC -nostdlib plugin_min.c -o plugin_min.so
 *        (and, toolchain-independence probe, with musl-gcc:
 *         musl-gcc -shared -fPIC -nostdlib plugin_min.c -o plugin_min_musl.so)
 * readelf: zero DT_NEEDED; the only UND entry is the mandatory null symbol.
 */
struct host_api {
    void *(*alloc)(unsigned long);
    void  (*dealloc)(void *);
    int   (*log)(const char *);
};
struct plugin_api {
    long (*run)(void);
    const char *name;
};
static struct host_api *host;
long plugin_run(void) { return 42; }
struct plugin_api *plugin_init(struct host_api *api)
{
    host = api;
    return &(struct plugin_api){ .run = plugin_run, .name = "plugin_min" };
}
```

### A.5 `plugin_dlref.c` — weak dl-API probe

```c
/* Built: gcc -shared -fPIC -nostdlib plugin_dlref.c -o plugin_dlref.so */
extern long dlvsym(void *handle, const char *name, const char *version)
    __attribute__((weak));
long plugin_dlself(void)
{
    if (dlvsym == 0) return -1;
    return (long)(void *)dlvsym;
}
```

### A.6 `plugin_dep.c` — inter-plugin dependency probe

```c
/* Built: gcc -shared -fPIC -nostdlib plugin_dep.c ./plugin_min.so -o plugin_dep.so
 * readelf: DT_NEEDED: ./plugin_min.so
 */
extern long plugin_run(void);
long dep_entry(void) { return plugin_run() + 1; }
```

### A.7 `plugin_tls.c` — TLS probe

```c
/* Built: musl-gcc -shared -fPIC -nostdlib plugin_tls.c -o plugin_tls.so */
static __thread long counter;
long t_tls(void) { counter += 1; return counter; }
```

### A.8 `exports.txt` — export list (final form used in §6–7 builds)

```
{
  host_alloc;
  host_free;
  host_log;
  malloc;
  free;
  __tls_get_addr;
  __tls_get_new;
};
```

### A.9 `host3.c` — the double-libc live demonstration

```c
/* Built: gcc -static -O2 host3.c -o host3 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef long (*fn_t)(void);
typedef void *(*malloc_t)(unsigned long);
typedef void  (*free_t)(void *);

static int maps_count(const char *needle)
{
    FILE *f = fopen("/proc/self/maps", "r");
    char line[512]; int n = 0;
    if (!f) return -1;
    while (fgets(line, sizeof line, f)) if (strstr(line, needle)) n++;
    fclose(f); return n;
}

int main(void)
{
    printf("before: libc.so.6 maps=%d\n", maps_count("libc.so.6"));
    void *host_malloc = dlsym(RTLD_DEFAULT, "malloc");

    void *h = dlopen("./plugin_libc.so", RTLD_NOW);
    if (!h) { printf("rejected: %s\n", dlerror()); return 0; }
    printf("loaded. libc.so.6 maps=%d  (>=1 means a SECOND libc entered)\n",
           maps_count("libc.so.6"));

    malloc_t p_malloc = (malloc_t)dlsym(h, "malloc");
    free_t   p_free   = (free_t)dlsym(h, "free");
    printf("host malloc  = %p\nplugin malloc = %p  -> %s\n",
           host_malloc, (void *)p_malloc,
           host_malloc == (void *)p_malloc ? "same" : "TWO ALLOCATORS");

    void *a = malloc(100);
    void *b = p_malloc(100);
    memset(a, 1, 100); memset(b, 2, 100);
    printf("cross-free: host-alloc block freed by plugin libc...\n");
    fflush(stdout);
    p_free(a);
    printf("...survived host->plugin free\n");
    free(b);
    printf("...survived plugin->host free\n");

    fn_t run = (fn_t)dlsym(h, "plugin_run");
    printf("plugin_run() = %ld\n", run());
    return 0;
}
```

### A.10 `host5.c` — dead-letter isolation (host symbol vs libc symbol)

```c
/* Built (three variants): gcc -static-pie -O2 -Wl,--export-dynamic-symbol-list=exports.txt host5.c -o host5_elist
 *                         gcc -static-pie -O2 -rdynamic host5.c -o host5_rdyn
 *                         gcc -rdynamic -O2 host5.c -o host5_dyn        (dynamic control)
 * Plugin sources: plugin_logonly.c (extern int host_log(const char*); t_log calls it)
 *                 plugin_malonly.c (extern void *malloc(unsigned long); t_malloc calls it)
 *                 both built freestanding.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
int host_log(const char *s) { return puts(s); }
typedef long (*fn_t)(void);
static void try(const char *so) {
    dlerror();
    void *h = dlopen(so, RTLD_NOW);
    if (!h) { printf("%-22s FAIL: %s\n", so, dlerror()); return; }
    char sym[64]; snprintf(sym, sizeof sym, "t_%s", so[7]=='l' ? "log" : "malloc");
    void *p = dlsym(h, sym);
    printf("%-22s dlopen OK, dlsym(%s)=%s\n", so, sym, p ? "found" : dlerror());
    if (p) ((fn_t)p)();
}
int main(void) { try("./plugin_logonly.so"); try("./plugin_malonly.so"); return 0; }
```

### A.11 `host6.c` — inter-plugin dependency host

```c
/* Built: gcc -static -O2 host6.c -o host6 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
typedef long (*fn_t)(void);
int main(void) {
    void *h = dlopen("./plugin_dep.so", RTLD_NOW | RTLD_GLOBAL);
    if (!h) { printf("dep FAIL: %s\n", dlerror()); return 1; }
    fn_t f = (fn_t)dlsym(h, "dep_entry");
    printf("inter-plugin call: dep_entry() = %ld (expected 43)\n", f());
    return 0;
}
```

### A.12 `host_dc.c` — minimal dlopen/dlsym/dlclose probe

```c
/* Built: gcc -static -O0 -g host_dc.c -o host_dc */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
typedef long (*fn_t)(void);
int main(void) {
    fprintf(stderr, "dlopen...\n");
    void *h = dlopen("./plugin_min_musl.so", RTLD_NOW);
    if (!h) { fprintf(stderr, "FAIL %s\n", dlerror()); return 1; }
    fprintf(stderr, "dlsym...\n");
    fn_t f = (fn_t)dlsym(h, "plugin_run");
    fprintf(stderr, "call -> %ld\n", f());
    fprintf(stderr, "dlclose...\n");
    dlclose(h);
    fprintf(stderr, "survived dlclose\n");
    return 0;
}
```

### A.13 `host_g_m.c` — Phase III: musl objects into the static glibc binary

```c
/* Built: gcc -static -O0 -g host_g_m.c -o host_gm_dbg  (marker-instrumented form)
 *   [A] dlopens plugin_min_musl.so (musl-toolchain freestanding plugin)
 *   [B] dlopens plugin_mlibc.so    (musl-gcc -shared -fPIC plugin_libc.c,
 *        DT_NEEDED: libc.so)
 * Environment A: plain run (glibc host).
 * Environment B: LD_LIBRARY_PATH=/workspace/musl/lib (musl libc.so discoverable).
 * The instrumented build prints [dbg] markers to stderr before/after each step;
 * output in Appendix B has the [dbg] lines filtered for legibility, except where
 * the anomaly of §7.3 is discussed.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef long (*fn_t)(void);

static int maps_count(const char *needle)
{
    FILE *f = fopen("/proc/self/maps", "r"); char l[512]; int n = 0;
    if (!f) return -1;
    while (fgets(l, sizeof l, f)) if (strstr(l, needle)) n++;
    fclose(f); return n;
}

static void show_libc_maps(void)
{
    FILE *f = fopen("/proc/self/maps", "r"); char l[512];
    while (fgets(l, sizeof l, f)) if (strstr(l, "libc.so")) fputs(l, stdout);
    fclose(f);
}

struct host_api { void *(*alloc)(unsigned long); void (*dealloc)(void *); int (*log)(const char *); };
struct plugin_api { long (*run)(void); const char *name; };
typedef struct plugin_api *(*init_t)(struct host_api *);
static void *h_alloc(unsigned long n) { return malloc(n); }
static void   h_dealloc(void *p)      { free(p); }
static int    h_log(const char *s)    { return puts(s); }

int main(int argc, char **argv)
{
    void *h0 = dlopen("./plugin_min_musl.so", RTLD_NOW);
    if (!h0) printf("[A] musl-toolchain freestanding plugin FAIL: %s\n", dlerror());
    else {
        init_t init = (init_t)dlsym(h0, "plugin_init");
        struct plugin_api *a = init(&(struct host_api){h_alloc, h_dealloc, h_log});
        printf("[A] musl-toolchain freestanding plugin: run()=%ld, libc.so maps=%d\n",
               a->run(), maps_count("libc.so"));
        dlclose(h0);
    }

    dlerror();
    void *h = dlopen("./plugin_mlibc.so", RTLD_NOW);
    if (!h) { printf("[B] musl-libc-linked plugin FAIL: %s\n", dlerror()); return 0; }
    printf("[B] loaded. mappings containing 'libc.so' (second libc if >0):\n");
    show_libc_maps();
    fn_t run = (fn_t)dlsym(h, "plugin_run");
    fflush(stdout);
    printf("[B] plugin_run() = %ld   <- printf executed through MUSL libc inside a static glibc process\n",
           run());
    return 0;
}
```

### A.14 `host2.c` — the four-test matrix host (E4/E5)

```c
/* Built: gcc -static -O2 host2.c -o host2_plain
 *        gcc -static -O2 -Wl,--export-dynamic-symbol-list=exports.txt host2.c -o host2_spie
 *        gcc -static -O2 -Wl,-E host2.c -o host2_E
 * Tests: [1] plugin_min.so (self-contained)  [2] plugin_free.so (libc refs)
 *        [3] plugin_dlref.so (weak dl-API ref) [4] plugin_libc.so (DT_NEEDED libc.so.6)
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

struct host_api { void *(*alloc)(unsigned long); void (*dealloc)(void *); int (*log)(const char *); };
struct plugin_api { long (*run)(void); const char *name; };

static void *h_alloc(unsigned long n) { return malloc(n); }
static void   h_dealloc(void *p)      { free(p); }
static int    h_log(const char *s)    { return puts(s); }
void *host_alloc(unsigned long n) { return malloc(n); }
void   host_free(void *p)         { free(p); }
int    host_log(const char *s)    { return puts(s); }

typedef struct plugin_api *(*init_t)(struct host_api *);
typedef long (*fn_t)(void);

int main(void)
{
    void *h = dlopen("./plugin_min.so", RTLD_NOW);
    if (!h) { printf("[1] FAIL: %s\n", dlerror()); return 1; }
    init_t init = (init_t)dlsym(h, "plugin_init");
    struct plugin_api *api = init(&(struct host_api){ h_alloc, h_dealloc, h_log });
    printf("[1] %s run() = %ld\n", api->name, api->run());

    void *h2 = dlopen("./plugin_free.so", RTLD_NOW);
    if (!h2) printf("[2] FAIL: %s\n", dlerror());
    else { fn_t f = (fn_t)dlsym(h2, "plugin_run"); printf("[2] run() = %ld\n", f()); }

    void *h3 = dlopen("./plugin_dlref.so", RTLD_NOW);
    if (!h3) printf("[3] FAIL: %s\n", dlerror());
    else { fn_t f = (fn_t)dlsym(h3, "plugin_dlself"); printf("[3] = %ld\n", f()); }

    dlerror();
    void *h4 = dlopen("./plugin_libc.so", RTLD_NOW);
    if (!h4) printf("[4] rejected: %s\n", dlerror());
    else     printf("[4] LOADED (unexpected!)\n");
    return 0;
}
```

### A.15 `plugin_logonly.c`, `plugin_malonly.c`, `plugin_syms.c`, `t_spie.c`

```c
/* plugin_logonly.c — freestanding; sole external ref is the host function.
 * Built: gcc -shared -fPIC -nostdlib plugin_logonly.c -o plugin_logonly.so */
extern int host_log(const char *);
long t_log(void) { return host_log("hi from plugin"); }
```

```c
/* plugin_malonly.c — freestanding; sole external ref is a libc symbol.
 * Built: gcc -shared -fPIC -nostdlib plugin_malonly.c -o plugin_malonly.so */
extern void *malloc(unsigned long);
long t_malloc(void) { return (long)malloc(8); }
```

```c
/* plugin_syms.c — freestanding; refs a host function and a libc symbol.
 * Built: gcc -shared -fPIC -nostdlib plugin_syms.c -o plugin_syms.so
 * (used in the RTLD_NOW/RTLD_LAZY isolation run: both modes fail on malloc) */
extern int host_log(const char *);
extern void *malloc(unsigned long);
long t_hostlog(void) { return host_log("resolved host_log"); }
long t_malloc(void)  { return (long)malloc(8); }
```

```c
/* t_spie.c — static-PIE musl smoke test (§6.3, E13).
 * Object compiled with plain gcc; final link per the F11 recipe. */
#include <stdio.h>
int main(void) { puts("static-pie musl alive"); return 0; }
```

### A.16 `host_m.c` — Phase II: static musl host matrix (E12)

```c
/* Built: musl-gcc -static -O2 host_m.c -o hostm_static
 *   [1] plugin_min.so  [2] plugin_logonly.so  [3] plugin_tls.so  [4] plugin_mlibc.so
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct host_api { void *(*alloc)(unsigned long); void (*dealloc)(void *); int (*log)(const char *); };
struct plugin_api { long (*run)(void); const char *name; };

static void *h_alloc(unsigned long n) { return malloc(n); }
static void   h_dealloc(void *p)      { free(p); }
static int    h_log(const char *s)    { return puts(s); }
int host_log(const char *s)           { return puts(s); }

typedef struct plugin_api *(*init_t)(struct host_api *);
typedef long (*fn_t)(void);
typedef void *(*malloc_t)(unsigned long);
typedef void  (*free_t)(void *);

static int maps_count(const char *needle) {
    FILE *f = fopen("/proc/self/maps", "r"); char l[512]; int n = 0;
    if (!f) return -1;
    while (fgets(l, sizeof l, f)) if (strstr(l, needle)) n++;
    fclose(f); return n;
}

int main(void)
{
    printf("start: libc.so maps=%d\n", maps_count("libc.so"));

    void *h1 = dlopen("./plugin_min.so", RTLD_NOW);
    if (!h1) { printf("[1] FAIL: %s\n", dlerror()); }
    else {
        init_t init = (init_t)dlsym(h1, "plugin_init");
        struct plugin_api *a = init(&(struct host_api){h_alloc, h_dealloc, h_log});
        printf("[1] %s run()=%ld\n", a->name, a->run());
        dlclose(h1);
    }

    void *h2 = dlopen("./plugin_logonly.so", RTLD_NOW);
    if (!h2) printf("[2] FAIL: %s\n", dlerror());
    else {
        fn_t f = (fn_t)dlsym(h2, "t_log");
        printf("[2] t_log()=%ld\n", f());
        dlclose(h2);
    }

    void *h3 = dlopen("./plugin_tls.so", RTLD_NOW);
    if (!h3) printf("[3] FAIL: %s\n", dlerror());
    else {
        fn_t f = (fn_t)dlsym(h3, "t_tls");
        printf("[3] t_tls()=%ld\n", f());
        dlclose(h3);
    }

    void *host_malloc = dlsym(RTLD_DEFAULT, "malloc");
    void *h4 = dlopen("./plugin_mlibc.so", RTLD_NOW);
    if (!h4) printf("[4] FAIL: %s\n", dlerror());
    else {
        printf("[4] loaded; libc.so maps=%d\n", maps_count("libc.so"));
        malloc_t pm = (malloc_t)dlsym(h4, "malloc");
        printf("[4] host=%p plugin=%p\n", host_malloc, (void *)pm);
        fn_t run = (fn_t)dlsym(h4, "plugin_run");
        printf("[4] plugin_run()=%ld\n", run());
    }
    return 0;
}
```

---

## Appendix B — Evidence log (verbatim)

Re-executed in full immediately before drafting; commands as given in Appendix A. `#` comments are annotations added at assembly time; all other lines are program output.

### E1 — Toolchain

```
ldd (Gentoo 2.43-r2 (patchset 3)) 2.43
gcc (Gentoo 15.3.0 p8) 15.3.0
GNU ld (Gentoo 2.46.1 p1) 2.46.1
6.18.39-gentoo-dist-bin
```

### E2 — First construction run (`host_static`, glibc static + `--export-dynamic`)

```
[start (static, before any dlopen)] libc.so.6 maps: 0, ld-linux maps: 0
free plugin FAILED: ./plugin_free.so: undefined symbol: malloc
exit=1
```

### E3 — `host_static` dynamic section

```
$ readelf -d host_static
There is no dynamic section in this file.
```

### E4 — Plain static binary matrix (`host2_plain`)

```
[1] plugin_min run() = 42
[2] plugin_free FAILED: ./plugin_free.so: undefined symbol: malloc
[3] plugin_dlref() = -1 (dlvsym from inside plugin worked)
[4] plugin_libc LOADED (unexpected!)
exit=0
```

### E5 — Static-PIE + export list (`host2_spie`, 6 dynsym entries incl. defined `malloc`)

```
6
[1] plugin_min run() = 42
[2] plugin_free FAILED: ./plugin_free.so: undefined symbol: malloc
[3] plugin_dlref() = -1 (dlvsym from inside plugin worked)
[4] plugin_libc LOADED (unexpected!)
exit=0
```

### E6 — Double-libc live (`host3`)

```
before: libc.so.6 maps=0
loaded. libc.so.6 maps=5  (>=1 means a SECOND libc entered)
host malloc  = (nil)
plugin malloc = 0x7fd0375ff530  -> TWO ALLOCATORS
cross-free: host-alloc block freed by plugin libc...
...survived host->plugin free
...survived plugin->host free
plugin_run() = 7
exit=0
```

### E7 — Dead-letter isolation (`host5_elist`, `host5_rdyn`, then dynamic control `host5_dyn`)

```
./plugin_logonly.so    FAIL: ./plugin_logonly.so: undefined symbol: host_log
./plugin_malonly.so    FAIL: ./plugin_malonly.so: undefined symbol: malloc
./plugin_logonly.so    FAIL: ./plugin_logonly.so: undefined symbol: host_log
./plugin_malonly.so    FAIL: ./plugin_malonly.so: undefined symbol: malloc
--- host5_dyn (dynamically linked control) ---
./plugin_logonly.so    dlopen OK, dlsym(t_malloc)=./plugin_logonly.so: undefined symbol: t_malloc
./plugin_malonly.so    dlopen OK, dlsym(t_malloc)=found
```

*(The control's probe code reversed the two lookup labels — it asks `plugin_logonly.so` for `t_malloc` — so its first `dlsym` line reports that label error; the load-bearing observation is `dlopen OK` on both objects in the dynamic binary — under `RTLD_NOW`, a successful load proves `host_log` and `malloc` resolved — versus `undefined symbol: host_log` for the same freestanding object in both static ones.)*

### E8 — Inter-plugin dependency (`host6`)

```
inter-plugin call: dep_entry() = 43 (expected 43)
```

### E9 — Minimal probe against musl-built freestanding plugin (`host_dc`)

```
dlopen...
dlsym...
call -> 42
dlclose...
survived dlclose
```

### E10 — Phase III, environment A (`host_gm_dbg`, glibc host)

```
[A] musl-toolchain freestanding plugin: run()=42, libc.so maps=0
[B] musl-libc-linked plugin FAIL: /lib64/libc.so: invalid ELF header
```

### E11 — Phase III, environment B (`LD_LIBRARY_PATH=/workspace/musl/lib ./host_gm_dbg`)

```
[A] musl-toolchain freestanding plugin: run()=42, libc.so maps=0
[B] loaded. mappings containing 'libc.so' (second libc if >0):
7f8a224c9000-7f8a224dd000 r--p 00000000 00:26 16402060  /workspace/musl/lib/libc.so
7f8a224dd000-7f8a22537000 r-xp 00014000 00:26 16402060  /workspace/musl/lib/libc.so
7f8a22537000-7f8a2256d000 r--p 0006e000 00:26 16402060  /workspace/musl/lib/libc.so
7f8a2256d000-7f8a2256e000 r--p 000a3000 00:26 16402060  /workspace/musl/lib/libc.so
7f8a2256e000-7f8a2256f000 rw-p 000a4000 00:26 16402060  /workspace/musl/lib/libc.so
7f8a22572000-7f8a22573000 r--p 00000000 00:26 16401582  /workspace/dlo_static/plugin_mlibc.so
7f8a22573000-7f8a22574000 r-xp 00001000 00:26 16401582  /workspace/dlo_static/plugin_mlibc.so
7f8a22574000-7f8a22575000 r--p 00002000 00:26 16401582  /workspace/dlo_static/plugin_mlibc.so
7f8a22575000-7f8a22576000 r--p 00002000 00:26 16401582  /workspace/dlo_static/plugin_mlibc.so
7f8a22576000-7f8a22577000 rw-p 00003000 00:26 16401582  /workspace/dlo_static/plugin_mlibc.so
hello from the libc-linked plugin
[B] plugin_run() = 7   <- printf executed through MUSL libc inside a static glibc process
```

### E12 — Static musl binary (`hostm_static`)

```
start: libc.so maps=0
[1] FAIL: Dynamic loading not supported
[2] FAIL: Dynamic loading not supported
[3] FAIL: Dynamic loading not supported
[4] FAIL: Dynamic loading not supported
```

### E13 — Static-PIE musl recipe (`t_spie4`)

```
$ readelf -h t_spie4 | grep Type
  Type: DYN (Position-Independent Executable file)
$ readelf -lW t_spie4 | grep -c INTERP
0
$ ./t_spie4
static-pie musl alive
```

### E14 — Plugin ELF facts

```
plugin_min.so:        named UND symbols = 0; DT_NEEDED count = 0
plugin_min_musl.so:   named UND symbols = 0
plugin_mlibc.so:      NEEDED Shared library: [libc.so]
plugin_free.so:       named UND symbols = free host_alloc host_free host_log malloc
```

### E15 — musl 1.2.5 static archive composition

```
$ ar t /workspace/musl/lib/libc.a | grep -E "dlopen|dynlink"
dlopen.lo
$ nm /workspace/musl/lib/libc.a | grep -cE ' T __dls2| T __dls3'
0
```

### E16 — solo release artifact (structural verification)

```
$ file solo-vulkan
ELF 64-bit LSB executable, x86-64, version 1 (SYSV), statically linked, stripped
$ readelf -lW solo-vulkan | grep -c INTERP
0
$ readelf -dW solo-vulkan
There is no dynamic section in this file.
(size: 2,300,272 bytes)
```

### E17 — Linker warning, verbatim

```
warning: Using 'dlopen' in statically linked applications requires at runtime the shared libraries from the glibc version used for linking
```

### E18 — ELF object descriptors

```
host_static: ELF 64-bit LSB executable, x86-64, version 1 (GNU/Linux), statically
linked, BuildID[sha1]=38f67ed..., for GNU/Linux 3.2.0, not stripped
```

---

## Appendix C — Peer review record

This document was drafted, then reviewed in three passes with distinct mandates. Each pass lists its findings and the edit applied. No pass was allowed to only approve: every pass was required to find and fix at least one defect, and the findings below are the ones actually acted on.

### Pass 1 — Factual verification against sources

*Mandate: every number, verbatim string, and quotation checked against the captured evidence log (Appendix B), the in-session source extracts (`quotes.log`), and the fetched reference documents.*

Findings and fixes:
1. §4 mischaracterized the debugging environment with a dangling internal reference ("E-permit below") — reworded to state the `ptrace` denial plainly.
2. §6.3 trap 1 conflated two observations (ET_EXEC output; absence of relocations) — restated as the single measurement it was: the two-flag link produced an ET_EXEC with no dynamic section and no relocations.
3. §5.4's dynamic-control argument was incomplete: it asserted resolution "without incident" without noting *why* a successful `dlopen` proves it — added the `RTLD_NOW` eager-binding justification.
4. §6.3's `.dynamic`-section note attributed no-relocation output to the wrong build — re-anchored to the ET_EXEC mis-build, with the export-list flag's role (forcing a dynamic section) stated separately.
5. Appendix A deferred several sources to "the study workspace" — a standalone-document violation; `host2.c`, `host_m.c`, `plugin_syms.c`, `plugin_logonly.c`, `plugin_malonly.c`, and `t_spie.c` were added in full (A.14–A.16).

Verified without change: all verbatim error strings (E2–E17) against the log; the glibc source comment; the musl stub source; the NEWS quotes; the solo artifact size and ELF properties; host-matrix and CI claims as attributed (T3).

### Pass 2 — Logical consistency and claim strength

*Mandate: hunt internal contradictions, overclaims, undefined references, and claims not supported at their stated tier.*

Findings and fixes:
1. §1.1 pointed to the linker warning "below (§5.1)"; the warning lives in §3.2 — cross-reference corrected.
2. §8's opening claimed both highlighted projects "satisfy C1–C4", contradicting this paper's own scorecard (row 4 marks cross-libc-dlopen's C3 as not-applicable-to-static) — restated as holding the one-libc line across the two process shapes, with the alternatives document's own phrasing quoted.
3. §9.2 claimed H3 holds for "arbitrary libc-referencing objects" — both cited projects state ceilings explicitly (solo: fail-loud stubs for uncovered ABI; cross-libc-dlopen: "the host's graphics stack is the ceiling") — the claim was qualified accordingly.
4. §3.4 asserted a version-specific NSS integration fact not verifiable from any in-session source — generalized to what the record supports (the classic module-loading design is itself the measured hazard class; packaging tooling still flags NSS) after a targeted NEWS search returned no such statement.
5. §9.2 cited an undefined finding label ("F12-A") — collapsed to F12, whose text already covers the freestanding-plugin measurement.
6. F3's "address distinct from (and unavailable to) the host side" was vague — tightened to the measured fact (an address in the second libc's allocator domain).

### Pass 3 — Structure, chronology completeness, and citations

*Mandate: verify every tangent of the investigation appears; every citation resolves; formatting is uniform; appendices are complete and verbatim where claimed.*

Findings and fixes:
1. Chronology gaps against the session record: the FAQ's deployment history (relative-`PT_INTERP` impossibility, the `PT_INTERP`→`PT_LOAD` rewrite attempt, execute-the-linker-first settlement) was absent from §8.5 — added with citation; cross-libc-dlopen's second gap (GL dispatcher fragmentation and the forwarding shims) was absent from §8.2 — added with an explicit scope note that this paper addresses only the libc gap; detour's compatibility envelope (x86-64-only, 2002-era pinned symbol versions) was absent from §8.3 — added; the FAQ's practical bundling rationale (NVIDIA driver linkage, allocator performance) was absent from §8.4 — added.
2. Appendix B's dynamic-control block paraphrased one output line with an ellipsis — restored to the exact bytes, with the label-reversal explanation moved to the annotation below it.
3. Citation sweep: every bracketed reference in the body now resolves to an entry in §References; `[cld-alternatives]`, previously listed but uncited, is now cited in §8.
4. Appendix C itself was a placeholder — this record replaces it.

Residual, accepted as-is: §9.1's scorecard condenses judgment calls (the ⚠ markings) that §8 justifies in prose; the abstract compresses the four phases and omits the anomaly of §7.3, which the chronicle reports in place.
