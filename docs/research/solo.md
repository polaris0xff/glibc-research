# pg83/solo — the sweep, and what it changes

⚠ **What this sweep did NOT establish, first, because a reader skimming for
the recommendation will not come back for it.**

- ⛔ **Nothing in solo was successfully run here.** Its documented default
  target, `./build`, **fails from a clean tree on this machine** with both
  gcc 13 and clang 18 — reproduced three times, §"Building it". So every
  claim below about solo's *behaviour* is a code read plus its own CI
  configuration, never an execution. Its Vulkan demo was not run: this
  machine has no GPU and the prebuilt release binary was not fetched.
- ⛔ **The load-bearing measurement in `experiments/73-` is about THIS
  project, not about solo.** It measures what a *pgb* binary's own static
  glibc could define for a host object. It does not measure solo.
- ⚠ **Symbol availability is not a working `dlopen`.** 73- counts names. It
  says nothing about whether bound code behaves: TLS layout, the stdio ABI,
  pthread object sizes, IFUNC resolution and RELRO are untouched by counting.
  ⭐ solo's own README says the same about its CI corpus — "loading is the
  floor, not the claim".
- ⚠ **The tracker is nearly empty.** 2 issues and 3 pull requests, total, and
  3 comments. Discussions are GraphQL-only and were not fetched. For a
  project this young the tracker is not the usual source of decisions; two of
  the five items still paid, §"What the tracker had".
- ⚠ **One machine, one day, x86_64.** aarch64 untested here, as everywhere in
  this repository.

## Route the reader

| you have | read |
|---|---|
| two minutes | §"The verdict" and the table in §"What 73- measured" |
| ten minutes | those, then §"What this changes about the plan" |
| implementation to do | §"Mechanisms worth taking", at file and line |
| a reason to distrust this | `references/pg83__solo/PROVENANCE.md`, then §"Building it" |

## Provenance

| | |
|---|---|
| repository | `pg83/solo` |
| commit | `79451211e2b7833f423b07bdb8a6c5584abf5822` |
| fetched | 2026-09-01, `scripts/common/mine-repo.sh`, proxy route |
| licence | **MIT** — `references/pg83__solo/tree/LICENSE` |
| corpus | **tracked, in this tree**, `references/pg83__solo/` |
| gap | discussions not fetched (GraphQL only) |
| depth | four passes: README; `lib/dlfcn.cpp` in full; `lib/elf_loader.h` in full and `elf_loader.cpp` at the resolver, provider and TLS sites; `lib/musl_tls.c`, `lib/musl_provider.cpp`, `lib/musl_symbols.*`, the symbol inventories, `dev/`, `.github/workflows/ci.yml`; plus three build attempts |

⛔ **Cite the commit beside every line reference.** All line numbers below are
at `79451211`.

## The verdict

⭐ **adopt the mechanism; do not port the implementation.**

solo is a `.so` loader compiled **into** a fully static executable. It maps
host shared objects itself — `mmap` of `PT_LOAD`, `DT_NEEDED` walk, versioned
symbol resolution, x86-64 and aarch64 relocations, ELF TLS and TLSDESC, IFUNC
materialisation, RELRO, initialisers — and never asks the host's `ld.so` for
anything. Its host is **musl**, so it also carries a 5,948-line glibc→musl ABI
bridge to satisfy the guest's `malloc@GLIBC_2.2.5` and its 5,075 siblings.

⭐ **The reason this matters here is that the bridge is the part this project
would not need.** A `pgb` binary's libc *is* glibc. If a host object's glibc
imports can bind straight to the glibc already statically linked into the
executable, the expensive half of solo does not have to be written.

⛔ **That was the claim to verify, and it is the one this sweep tested.**

## What `73-` measured, and it is the reason to take this seriously

`experiments/73-host-dso-abi-demand.sh`,
`evidence/73-host-dso-abi-demand/RESULT.txt`. Every ELF shared object in every
one of the eleven pinned environments — **5,807 objects** across the seven
glibc ones — parsed byte-wise, no `readelf`, no binutils inside the target.
Every `GLIBC_`/`GCC_`-versioned undefined symbol checked against what the
pinned build environment's static glibc (`libc.a`, `libm-2.36.a`, `libmvec.a`,
`libpthread.a`, `librt.a`, `libdl.a`, `libutil.a`, `libresolv.a`, `libanl.a`,
`libcrypt.a`, `libc_nonshared.a`, `libgcc.a`) can define: **7,074 symbols**.

| environment | objects | demand | served | | A | B | C | S | D | **E** |
|---|---|---|---|---|---|---|---|---|---|---|
| alpine 3.22 / 3.20 / 3.10 | 28 | 0 | 0 | n/a | 0 | 0 | 0 | 0 | 0 | **0** |
| voidlinux-musl | 357 | 0 | 0 | n/a | 0 | 0 | 0 | 0 | 0 | **0** |
| debian-11 | 391 | 969 | 905 | 93.4% | 4 | 0 | 0 | 42 | 18 | **0** |
| debian-12 | 759 | 900 | 851 | 94.6% | 5 | 0 | 0 | 41 | 3 | **0** |
| ubuntu-20.04 | 782 | 983 | 893 | 90.8% | 4 | 0 | 0 | 44 | 42 | **0** |
| rockylinux-8 | 623 | 1084 | 1049 | 96.8% | 7 | 0 | 0 | 6 | 22 | **0** |
| opensuse-leap-15.6 | 209 | 1014 | 993 | 97.9% | 5 | 13 | 0 | 2 | 1 | **0** |
| fedora-42 | 463 | 983 | 961 | 97.8% | 6 | 15 | 0 | 0 | 1 | **0** |
| archlinux-latest | 2780 | 1270 | 1198 | 94.3% | 7 | 20 | 0 | 41 | 4 | **0** |

⭐ **Class E is empty on every environment.** Every symbol the pinned static
glibc cannot define falls into a class with a measured reason, and the classes
are decided by the target environment's own files rather than by judgement:

| | class | decided by | what it means |
|---|---|---|---|
| **A** | loader-owned | the host's own `ld.so` exports it | `__tls_get_addr`, `_rtld_global`, `_rtld_global_ro`, `__pointer_chk_guard`. **Not a gap** — a compiled-in loader provides these itself, exactly as `ld.so` does. solo does. |
| **B** | version **ceiling** | host `libc.so.6` has it at a `GLIBC_` version newer than the pin | 20 symbols. `__isoc23_strtol` and friends plus `strlcpy`/`strlcat`, all `GLIBC_2.38`; `__memset_explicit_chk`, `GLIBC_2.43`. The object was built against a newer glibc than ours. |
| **C** | version floor | host has it, the pinned `libc.so.6` does not | **empty everywhere.** |
| **S** | static/shared split | both `libc.so.6` have it, `libc.a` does not | 49 symbols. See below — this is the one nobody would have predicted. |
| **D** | not the host libc's either | host `libc.so.6` does not export it | the demand is met by another library there, or not at all. |
| **E** | unexplained | none of the above | **0**. |

### ⭐ Class S: the static libc is not a smaller shared libc

49 symbols that Debian 12's `libc.so.6` **exports** and that the *same
package's* `libc.a` **does not contain**. Verified directly rather than
inferred:

```
$ nm -D --defined-only .../debian-12/usr/lib/x86_64-linux-gnu/libc.so.6
  0000000000147000 T xdr_void@GLIBC_2.2.5
  0000000000109680 T __xmknod@@GLIBC_2.2.5
  00000000001d1e20 D _sys_siglist@GLIBC_2.3.3
  00000000001da460 B __malloc_initialize_hook@GLIBC_2.2.5
$ nm --defined-only .../pgb-env-debian12/usr/lib/x86_64-linux-gnu/libc.a | grep -w xdr_void
  (nothing)
```

The bulk of them are sunrpc — `xdr_*`, `svc_*`, `clnt_*`, `auth*`,
`key_*`, `host2netname`, `xdecrypt` — kept in the shared library as
compatibility symbols after the interface moved to `libtirpc`, and never
present in the archive. The rest are removed-interface compat: `__xmknod`,
`__xmknodat`, `_sys_siglist`, `__malloc_initialize_hook`.

⛔ **So "pin a different glibc" does not reach them at any version.** The
static libc is not a subset-by-version of the shared one; at one and the same
version it is a *different symbol set*. `libtirpc.a` is present in the pinned
environment and defines the sunrpc half — measured — so the answer is a link
choice, not a pin choice.

### ⭐ B and C together: two constraints pointing opposite ways

`docs/AGENTS.md` §14 already carries a **floor**: *do not build below glibc
2.34*, because the NSS override needs it (`experiments/21-`). Class B is a
**ceiling** on the same axis: build too old and a modern host object wants
`__isoc23_strtol`.

⛔ **No single pin satisfies both ends**, so a loader that serves host objects
everywhere needs compat definitions rather than a better-chosen pin. Class B
is 20 symbols, and 14 of them are the `__isoc23_*` family, which glibc
introduced as *aliases* for existing functions with C23 `strtol` semantics —
a small, bounded, writable compatibility layer, not an open-ended one.

### The version-resolution rule, measured in both directions

The whole approach binds a table of **bare names** from `nm` to a host
object's **versioned** references. That rule is real, and it is narrower than
it first looks. Both cases are asserted by `73-`:

| case | result |
|---|---|
| an unversioned definition in **another** object satisfies `foo@PGBTEST_1.0` | ⭐ **yes** — this is where a compiled-in provider table sits |
| the object **named in `DT_VERNEED`**, rebuilt without versions | ⛔ **the loader asserts**: `dl-lookup.c:106: check_match: Assertion 'version->filename == NULL \|\| ! _dl_name_match_p (version->filename, map)' failed` |

⭐ **The second row explains an existing result in this tree.**
`experiments/50-` ported `cross-libc-dlopen`'s `cld_strip_versions()` and
found no effect. Row 2 is what stripping versions off a *named* provider does
when it is reached: glibc treats it as a bug in that library, not as a
compatibility case. `docs/limitations.md` §1, `TODO` T-031.

solo relies on row 1 and implements it deliberately —
`lib/elf_loader.cpp:2070`, at `79451211`:

```
// ld.so's compatibility rule: a provider built without any version
// information satisfies a versioned reference. Only a genuinely
// unversioned definition qualifies -- a wrong-version one stays a
// loud failure.
```

## Mechanisms worth taking, at file and line

All at `79451211`, under `references/pg83__solo/tree/`.

| | mechanism | where | why it transfers |
|---|---|---|---|
| 1 | **A provider table generated from a symbol-name list, linked into the executable** | `lib/musl_symbols.cpp:1-12` reads `musl_symbols.json.h`, generated by `dev/generate_symbol_headers.py` from `lib/musl_symbols_x86_64.json` (a plain name list) | ⭐ This is **exactly** `pgb --wrap-dlopen`'s generated table, one layer up: applied to the **libc** instead of to the application's plugins. Taking the address of each name is also what forces those archive members into the link. `tool/runtime/pgb-dlopen.h` already has the layout. |
| 2 | **Resolution order, not an approximation of it** | `lib/elf_loader.cpp:2034-2078` | `DT_SYMBOLIC` first, then global scope in load order, then the image's own closure, with `RTLD_DEEPBIND` swapping the last two — then the unversioned-provider fallback. A loader that gets this wrong binds the right name to the wrong definition and fails far from the cause. |
| 3 | **Static providers short-circuit the mapping entirely** | `lib/dlfcn.cpp:294-302`, table built at `lib/dlfcn.cpp:179-201` | Before touching the disk, `dlopen` checks a registry of names the executable already satisfies. ⭐ A `DT_NEEDED libwayland-client.so.0` can be answered by the copy linked in, so the host object never drags a second one in. This is the mechanism that keeps a foreign libc out. |
| 4 | **Fail loudly, per symbol, instead of refusing the object** | `lib/glibc_stubs.cpp`, generated by `dev/generate_glibc_stubs.py` | An unimplemented entry point is a **unique** stub that aborts naming itself and its version. ⭐ The alternative — refusing to load anything with an unimplemented import — rejects objects that would never have called it. |
| 5 | **`LD_TRACE_LOADED_OBJECTS` in `ldd` format, including names served *without* a mapping** | `lib/elf_loader.h:21-26`, `elf_loader.cpp:553-564` | ⭐ Directly useful to `pgb verify`: it makes "which of these did the binary satisfy internally" observable from inside, beside the syscall trace this project takes from outside. Two independent instruments on the same question. |
| 6 | **`AT_SECURE` discipline** | `lib/elf_loader.h:11-14`, `dlfcn.cpp:24` | A set-uid process must ignore the environment's search paths and debug switches. A loader added to `pgb` inherits this obligation the moment it honours any variable. |
| 7 | **The demand-ranked ABI worklist** | `dev/abi_demand.py`, `dev/abi-demand.txt` | ⭐ The shape of `experiments/73-`: rank the remaining work by how many real installations demand each symbol, rather than by the ABI's alphabet. |

## ⛔ What must NOT be ported

| | |
|---|---|
| **`lib/glibc_shim.cpp` (5,948 lines) and `lib/glibc_stubs.cpp`** | ⭐ The whole point of taking this route *here*. They translate glibc's ABI onto a musl runtime. A static **glibc** host has no translation to do — `73-` measures 90.8%–97.8% of the demand already definable, with every remainder named. Porting the bridge would be porting the solution to a problem this project does not have. |
| **`lib/musl_tls.c` (the static TLS pad)** | ⛔ **The most musl-coupled file in the tree, and the hardest part to replace.** It includes `ext/musl/src/internal/pthread_impl.h`, writes `libc.tls_head` and `libc.tls_size` directly, and relies on musl's `__copy_tls` seeding each new thread. glibc's equivalent is `_dl_tls_static_surplus` and `__libc_setup_tls`, which is a different mechanism with different invariants. ⚠ **Cost this honestly before planning a loader**: it is the one piece where "we are glibc, so it is simpler" is *not* obviously true. |
| **`ext/` (musl, LLVM runtimes, Vulkan, zlib, libpng) and `build.py`** | A whole vendored toolchain and a bespoke build system. `pgb` has a pinned environment already. |
| **The Vulkan demo and its ICD discovery** | It is solo's proof, not a mechanism. GPU drivers are not this project's class. |

## Building it — three attempts, and what they show

⛔ **The documented default target does not build here.** README: *"The default
target builds the standalone archive: `./build`."*

| attempt | result |
|---|---|
| `./build test` | ⛔ fails fetching a pinned fixture: `Fedora-Container-Base-Generic-44-1.7.x86_64.oci.tar.xz`, **SHA-256 mismatch**, `tst/download.py:74`. The pin no longer matches what that URL serves. |
| `./build`, clean tree, gcc 13.3 | ⛔ `lib/iface_handle.cpp` → `/usr/include/c++/13/cwchar` → `ext/musl/include/wchar.h:31: fatal error: bits/alltypes.h: No such file or directory` |
| `./build`, clean tree, clang 18 | ⛔ identical failure, same file, same line |

⭐ **Not a compiler problem: a build-graph one.** `build.py:192-205` declares a
`musl_alltypes` step that generates `$(B)/ext/musl/include/bits/alltypes.h`,
and `build.py:261` lists it as a dependency of one target group. The default
archive target compiles `lib/*.cpp` with musl's include directory on the path
but without that edge, so the host's `<cwchar>` pulls musl's `wchar.h` before
the generated header exists. `./build test` gets past it because it builds
musl first for other reasons.

⚠ **Their CI would not see it**, and ⛔ **the count this document first gave
was wrong.** `references/pg83__solo/tree/.github/workflows/ci.yml` at the pin
has **nine** jobs, not six: `fetch-x86_64`, `fetch-aarch64`, `alpine-musl`,
`fedora-gcc`, `ubuntu-arm`, `ubuntu-clang`, `ubuntu-qemu`, `nixos-lavapipe`,
`termux-bionic-x86_64`. Three of them — the two fetchers and the Termux row —
were left out of the original sentence with nothing saying so.

⭐ **The conclusion survives the correction, and re-deriving it is what
establishes that.** Exactly one job runs the bare default target,
`alpine-musl` at line 87 (`./build -j12 -Duse_corpus=…` with no target), and
Alpine is musl. Every glibc row runs a *named* target — `pthread_test`,
`test`, `corpus`, `vulkan_test`, `secure_test` — which is the path that gets
past the missing build-graph edge. And no row is a clean tree: all nine
restore a corpus cache first. So no job builds the standalone archive target
on a glibc host from a clean tree, which is the failure reproduced above.

⛔ **This does not impugn the design, and it is not evidence about the
loader.** It is evidence about one build target, and it is the reason
every behavioural statement in this document is marked as a code read.

## What the tracker had

Five items total. ⭐ Two paid, and one is a finding this project can use.

| | |
|---|---|
| ⭐ **PR #3, `--eh-frame-hdr`** | *"gcc suppresses `--eh-frame-hdr` for `-static` links (`gcc/config/gnu-user.h`: `%{!static\|static-pie:--eh-frame-hdr}`), and GNU ld then leaves the executable without `.eh_frame_hdr` / `PT_GNU_EH_FRAME`."* Static LLVM libunwind finds unwind tables only through that segment, so **every C++ exception terminated the process** — `catch (...)` in `main` never ran. ⚠ **Their CI did not catch it**: the Ubuntu leg uses clang, which always passes the flag; the Fedora leg does not run the smoke test. ⭐ **`poc/60-leveldb` in this tree throws and catches and asserts on the caught payload** (`run.sh:86-90`, `:149`), so this project has the check their CI lacked — but it has never been checked against `PT_GNU_EH_FRAME` directly. `TODO` T-018. |
| ⭐ **PR #5, `/proc/self/exe`, DECLINED by the maintainer** | The proposal intercepted `readlink`/`readlinkat`/`realpath` so a guest sees its own path. The ruling: *"The surface is unbounded. `readlink`, `readlinkat`, and `realpath` are three of many readers of `/proc/self/exe`. The same answer is reachable through `open()` … `stat`/`fstatat`, `fopen`, `/proc/<pid>/exe` with the process's own pid, a dirfd-relative `openat`, and `std::filesystem`."* ⭐ **This is a costing this project would otherwise have to derive.** `pgb`'s `--wrap` mechanism is the same shape, and the ruling says where that shape stops paying: intercepting a *value* is bounded, intercepting every *route to a value* is not. |
| PR #4, `DT_RUNPATH` for `dlopen` from loaded code | glibc resolves a bare `dlopen` name through the **calling object's** `DT_RUNPATH`/`DT_RPATH`. A loader that consults only the requester's own `DT_NEEDED` paths misses siblings reached via `$ORIGIN`. A correctness detail any loader here would have to reproduce. |
| issue #2, NixOS `VkResult -9` | Fixed by scanning `/run/opengl-driver/share`. ⚠ Also an observation worth keeping: *"loading the system's `libvulkan.so.1` dynamically does not save you either"* on NixOS. |
| issue #1, `ix` build permissions | About the author's other project. Not this. |

## What this changes about the plan

⭐ **`docs/AGENTS.md` §13 item 4 lists three routes to the host-plugin class.
This is a fourth, it is cheaper than route C, and it is now the best-evidenced
of the four.**

| route | status after this sweep |
|---|---|
| A — `--wrap` on `dlopen` against a compiled-in table | ✅ **built**, `--wrap-dlopen`, 11 of 11. Serves a program's **own** plugins. Unchanged. |
| B — port `cross-libc-dlopen`'s full rewrite | ⚠ **weakened.** `73-`'s second control shows what stripping versions off a named provider does: the loader asserts. T-031 keeps its other two steps, but the one `50-` ported is now measured to be actively harmful where it bites. |
| C — carry a loader (tier 2) | unchanged, and still gives up the single ordinary ELF. |
| ⭐ **D — compile a loader IN, resolve against our own static glibc** | **new.** No second libc, no `ld.so`, still one ordinary ELF, and `73-` measures the symbol demand as 90.8%–97.8% already met with zero unexplained residue. |

⛔ **Route D is not free and the write-up must not read as if it were.** What
is measured is that the *names* resolve. What is not measured is any of:
`lib/musl_tls.c`'s glibc equivalent; whether glibc's own static-TLS surplus
can be donated the way musl's pad is; IFUNC resolvers run against the right
hwcaps; whether `pgb`'s existing `--wrap` mechanisms and a loaded object's
libc calls stay coherent. ⚠ solo needed 2,707 lines of `elf_loader.cpp` for
the mapping alone, and that part does **not** get cheaper by being glibc.

`TODO` T-033 carries route D with that split stated.

## Verdict lines, for `prior-art.md`

| reference | commit | depth | verdict |
|---|---|---|---|
| `pg83/solo` | `79451211` | four passes; `dlfcn.cpp` and `elf_loader.h` in full, `elf_loader.cpp` at the resolver/provider/TLS sites, `musl_tls.c`, `dev/`, CI; three build attempts, all failed | ⭐ **adopt the mechanism, refuse the implementation.** The compiled-in ELF loader is route D. The 5,948-line glibc→musl bridge is precisely what a static-glibc host does not need, and `experiments/73-` measures why. |

⛔ **Assume more claims here are wrong.** The three defects `73-` found in its
own instrument — a linker script read as an archive, a column order read
backwards, and a control that measured the one case glibc guards — were all
found *after* they had produced a plausible-looking result.
