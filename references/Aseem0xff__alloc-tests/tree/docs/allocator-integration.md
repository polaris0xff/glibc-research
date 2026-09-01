# Allocator integration

How each allocator is built, how it is attached to a program, and what does not
work. Everything here is derived from a recipe in `allocators/<id>/build.sh` —
if this page and that script disagree, **the script is right** and this page is
stale.

---

## The four mechanisms

⭐ **These are four different experiments.** Conflating them is the most common
way an allocator benchmark ends up answering a question nobody asked.

### 1. `rust-global` — link the allocator into the application

The application declares a `#[global_allocator]` that calls the allocator's
**prefixed** C API. libc's `malloc` is untouched and still serves any C code in
the process.

```rust
#[global_allocator]
static ALLOC: rgalloc_shim::Alloc = rgalloc_shim::Alloc;
```

The shim (`crates/rgalloc-shim`) binds to `mi_malloc`, `je_mallocx`, `sn_malloc`,
`rpaligned_alloc` or `h_malloc` depending on its cargo feature.

⭐ **Prefixed symbols are chosen deliberately.** Binding to `mi_malloc` rather
than `malloc` means a missing archive **fails the link** instead of silently
resolving to libc and producing a full set of numbers for the wrong allocator.

**Applies to:** anything you can rebuild from source.
**Does not tell you:** what happens to C libraries in the same process, which
keep using libc's allocator.

### 2. `libc-surgery` — replace the distribution's allocator

⭐ **This is the mechanism that answers "can I ship a container image whose
allocator is different".** `ar -M` rewrites `libc.a`: delete the archive members
that define `malloc`/`free`/…, add the allocator's archive in their place.

```
CREATE  /tmp/libc.a
ADDLIB  /usr/lib/libc.a.orig
DELETE  malloc.lo free.lo calloc.lo ...      <- derived, see below
ADDLIB  /path/to/liballocbench.a
SAVE
END
```

Afterwards **every statically linked binary built in that image uses the new
allocator, with no build flags at all.**

The technique comes from `references/haskell-wasm__rust-alpine-mimalloc`
(`tree/build.sh`). Two things are done differently here:

- ⛔ **The member list is derived, not hard-coded.** Measured on Alpine musl
  **1.2.6** (`experiments/50-libc-surgery-verify.sh`, 2026-09-01), deriving it
  reproduces the prior art's 13 names **exactly** — so on this musl the
  hard-coded list is correct. ⚠ The objection is not that it is wrong today; it
  is that **nothing checks**. Those names are a property of one musl release,
  and a hard-coded delete that stops matching is silent: `libc.a` ends up with
  two definitions of `malloc` and link order decides which one runs.
  `alloc-runner ar-members` reads the archive instead, so the list adapts.
- ⚠ **Deriving from the public entry points alone is not enough.** On musl
  1.2.6 `lite_malloc.lo` defines `malloc` while `malloc.lo` defines
  `__libc_malloc_impl` — the actual mallocng implementation. Keying only on
  public names displaces 11 members and leaves the implementation in the
  archive. The internal names are listed too, which brings it to 13.
- ⛔ **And a symbol that merely mentions malloc is not an allocator symbol.**
  Adding `__malloc_atfork` looked obviously right and deleted `fork.lo` — which
  also defines **`fork`**. The splice still passed its own malloc/free check,
  because that check says nothing about the rest of libc. There is now a guard:
  a member that also defines `fork`, `pthread_create`, `printf`, `open` or
  `memcpy` is **refused** before anything is cut.
- ⛔ **The result is asserted.** After the splice, exactly one archive member may
  define `malloc` and exactly one may define `free`. `libc-surgery.sh` exits 1
  otherwise.

⚠ **Rust ships its own musl `libc.a`**, under
`$RUSTUP_HOME/toolchains/*/lib/rustlib/<target>/lib/self-contained/`. Patching
only `/usr/lib/libc.a` leaves every Rust musl build using the unpatched copy.
Every copy found is patched and the count is printed.

### 3. `link-override` — the naive approach

Put the allocator's archive ahead of libc on the link line and hope the linker
resolves `malloc` from it.

⚠ **Recorded here because its failure is informative**, not because it is
recommended. See the results section below.

### 4. `preload` — swap at run time

`LD_PRELOAD` a shared object into a **dynamically linked** binary. The only
mechanism Mesh and Google tcmalloc support.

⛔ **Preload results are never compared with static ones.** A dynamic binary
going through the PLT is a different binary; the report keeps them in separate
tables.

⚠ **Status: the recipes do not yet implement `MODE=preload`.** The mechanism is
planned and wired through the orchestrator, but the per-allocator shared-object
build is not written. See `docs/AGENTS.md` §13.

---

## Per-allocator recipes

Every recipe receives `SRC`, `OUT`, `MODE` (`prefixed`|`override`), `PIC`,
`LIBC`, `TARGET_ARCH`, `CC`, `CXX`, `AR`, and must produce
`$OUT/lib/liballocbench.a` — one archive name for every allocator, so the
consumer never has to know which one it linked.

A recipe exits **3** with `UNSUPPORTED: <reason>` when a configuration cannot be
built. That is a result, and the report prints it.

### mimalloc — `cmake`

```sh
cmake -S $SRC -B $B -DCMAKE_BUILD_TYPE=Release \
  -DMI_BUILD_SHARED=OFF -DMI_BUILD_OBJECT=OFF -DMI_BUILD_TESTS=OFF \
  -DMI_OVERRIDE=<ON|OFF>          # ON defines malloc; OFF is prefix-only
  -DMI_LIBC_MUSL=<ON|OFF> \
  -DCMAKE_POSITION_INDEPENDENT_CODE=<ON|OFF>
```

`MI_OVERRIDE` is the whole difference between the two mechanisms, which is why
mimalloc is the allocator both are demonstrated with.

⚠ `MI_SKIP_COLLECT_ON_EXIT` is deliberately **not** set, although the prior art
sets it. It makes process exit cheaper by skipping a final collect, which would
flatter mimalloc specifically on the `startup` workload.

⚠ **PIC matters.** The prior art removes `POSITION_INDEPENDENT_CODE` from the
static target. That is correct for a non-PIE binary and **breaks static-PIE**.
Here PIC is a build parameter and the cache key includes it.

### jemalloc — `autotools`

```sh
$SRC/configure --with-jemalloc-prefix=<je_|""> \
  --disable-cxx --disable-initial-exec-tls \
  --enable-static --disable-shared --disable-doc
```

Two flags are **required**, not tuning:

- `--disable-cxx` — otherwise `jemalloc_cpp.cpp` defines `operator new`/`delete`
  and pulls a C++ runtime into the archive. Linking that into a Rust binary with
  no C++ runtime fails on undefined `__cxa_*`.
- `--disable-initial-exec-tls` — the initial-exec TLS model needs a static TLS
  block reserved by the dynamic loader. **A static binary has no loader.** Left
  enabled, this is the difference between a working static jemalloc and one that
  aborts on the first allocation in a thread.

### snmalloc — `cmake`, C++20

```sh
cmake -S $SRC -B $B -DCMAKE_BUILD_TYPE=Release \
  -DSNMALLOC_BUILD_TESTING=OFF -DSNMALLOC_STATIC_LIBRARY=ON \
  -DSNMALLOC_STATIC_LIBRARY_PREFIX=<sn_|""> \
  -DSNMALLOC_CLEANUP=CXX11_DESTRUCTORS   # musl only
cmake --build $B --target snmallocshim-static
ar d liballocbench.a new.cc.o             # see below
```

- `SNMALLOC_CLEANUP=CXX11_DESTRUCTORS` on musl: without it the shim builds and
  then misbehaves at thread teardown. Recorded in
  `references/daanx__mimalloc-bench` `tree/build-bench-env.sh` at commit
  `3ad2732048312b0c`.
- ⛔ **`new.cc.o` is removed.** It defines the C++ operators, and so does
  `libstdc++.a`; a static link ends in
  `multiple definition of 'operator delete(void*, unsigned long)'`. ripgrep is
  Rust and never calls them. For `libc-surgery`, an `operator new` inside
  `libc.a` would change every C++ program built in the image — a much larger
  claim than this project makes.
- The recipe asks the compiler where `libstdc++.a` is
  (`$CXX -print-file-name=libstdc++.a`), because rustc does not search gcc's own
  version directory and `-l static=stdc++` otherwise fails.

### rpmalloc — direct compile

```sh
$CC -O3 -fPIC -std=c11 -D_GNU_SOURCE -DENABLE_PRELOAD=0 -c rpmalloc/rpmalloc.c
# override mode only:
$CC ... -DENABLE_OVERRIDE=1 -c rpmalloc/malloc.c
ar rcs liballocbench.a *.o
```

⚠ Not built through `configure.py`/ninja: that also builds C++ test binaries,
which is the failure in upstream mimalloc-bench issue 256.

⚠ **rpmalloc is the only allocator here that needs explicit initialisation** —
`rpmalloc_initialize()` once per process and `rpmalloc_thread_initialize()` on
**every thread** before first use. Skipping the thread step does not fail
loudly; it corrupts. The shim guards every allocation path with rpmalloc's own
`rpmalloc_is_thread_initialized()` rather than a Rust `thread_local`, because a
`thread_local` with a destructor would itself allocate and recurse into the
allocator during teardown.

⚠ At the pinned revision `rpmalloc.c` defines the plain `malloc` names itself,
so `-DENABLE_OVERRIDE=0` is not enough for a prefixed archive; the aliases are
localised with `objcopy` and the result re-checked.

### hardened_malloc — upstream `make`, then `ar`

Upstream builds a shared object; a static archive is not a target it has, so the
Makefile builds the objects and `ar` archives them. Using upstream's own rule
keeps the compile flags upstream's.

⭐ **`-DH_MALLOC_PREFIX` is the mode switch**, and it is not obvious.
`include/h_malloc.h` reads:

```c
#ifndef H_MALLOC_PREFIX
#define h_malloc malloc
#define h_free   free
...
```

so the `h_`-prefixed API in the header **is** the plain malloc API unless that
macro is defined. Building without it and then looking for `h_malloc` finds
nothing.

⚠ The define goes in `SHARED_FLAGS`, **not** `CPPFLAGS`. The Makefile builds
`CPPFLAGS` in two stages and a command-line `make CPPFLAGS=...` overrides the
whole variable *and* makes every later `+=` a no-op — dropping `-I include` and
silently dropping ~15 `CONFIG_*` defines, which would build a
differently-configured allocator than the result claims.

⚠ `TARGET_ARCH=` is passed on the make command line. `TARGET_ARCH` is a **GNU
make built-in** used by the implicit C rule; this project's contract also uses
that name, and exporting it puts the bare word `x86_64` on the compiler line.

Three deliberate deviations from upstream defaults, each recorded in the build
metadata:

| flag | why |
| --- | --- |
| `CONFIG_NATIVE=false` | upstream defaults to `true`, adding `-march=native`. Not reproducible, not comparable between runners, and no other allocator here is built that way. |
| `CONFIG_CXX_ALLOCATOR=false` | drops `new.cc`, whose operators would pull a C++ runtime into a Rust binary |
| `-flto` removed | ⭐ **every** allocator here is built without internal LTO, so that dimension is constant. hardened_malloc is the only one that forces it on. LTO is measured as an *application* build profile instead, where it applies to every cell equally. |

### Mesh — `preload` only

⛔ **No prefixed C API.** Its published interface is the malloc override itself,
so there is no symbol a `#[global_allocator]` shim can bind to that is
distinguishable from libc's.
⛔ **No static archive target**, and its runtime depends on interposing `mmap`
and on `pthread_atfork` handlers registered at load time, which are not reachable
from an archive spliced into `libc.a`.

Upstream publishes **no git tag at all**, so it is tracked by branch head and
pinned by commit. Its build is run serially — upstream issue 96 is a parallel
build race.

### Google tcmalloc — `preload` only

⛔ **No prefixed C API**; the supported interface is the malloc/operator-new
replacement plus the C++ `MallocExtension`.
⛔ **No self-contained static archive**: the Bazel `tcmalloc` target is a C++
library whose link closure pulls in Abseil and libstdc++. Splicing that into
`libc.a` would put a C++ runtime inside every C program's libc.
⛔ Upstream states **musl is not supported**.

⚠ Bazel is pinned to **8.6.0** through bazelisk. Bazel 9 breaks this build —
upstream mimalloc-bench issue 258.

---

## What the mechanisms produced

Measured on Alpine musl 1.2.6, x86_64, static-pie, mimalloc, 2026-09-01.

| mechanism | outcome |
| --- | --- |
| `rust-global` | **works** |
| `libc-surgery` | **works** — 13 musl members displaced from each of 2 `libc.a` copies, with `malloc`, `free`, `calloc` and `realloc` each defined exactly once afterwards and no musl object providing them. Measured 0.460× against the control (0.444× in an earlier run; see the magnitude caveat in `docs/results.md`) |
| `link-override` | ⛔ **fails on musl-static**: `multiple definition of __libc_malloc / __libc_free / __libc_realloc / aligned_alloc / calloc`. Pulling the whole archive in ahead of libc brings mimalloc's compatibility aliases, which collide with musl's own. This is why the surgery exists: deleting the displaced members is not optional. |

⚠ **Statically replacing glibc's allocator is not supported at all.** glibc's
malloc object also defines symbols the rest of glibc references internally, so
removing it breaks the archive and leaving it gives two `malloc`s. The planner
marks those cells unsupported with that reason rather than discovering it as a
link error every run. The same allocators *are* measured on glibc through
`rust-global`.
