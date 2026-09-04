# HISTORY/entries/runtime.md — the CLOSED runtime entries

⛔ **Nothing here is work.** Every entry below is `done`. They were moved
out of `TODO/runtime.md` on 2026-09-03c so that `TODO/` carries only what is
left, at the operator's instruction:

> *"strip away the fat, things that are already resolved and fixed and just
> send them straight into /HISTORY/\*, the TODO/\* must be lean and contain
> only what's left"*

⭐ **They keep their `T-` ids and their rows in [`../../TODO/INDEX.md`](../../TODO/INDEX.md)**,
which is what stops any of this being rediscovered. `sh TODO/check.sh`
checks this file against those rows exactly as it checked `TODO/`.

⛔ Do not reopen an entry here. A defect that still matters is a NEW entry.

---

## T-030 — `--wrap-dlopen` against a compiled-in table

**Source** `docs/research/prior-art.md`, `allyourcodebase/pipewire`.
**Category** runtime · **Priority** P1 · **Effort** M · **Status** ✅ done

**Problem.** A program loading its own plugins is servable only by hand today
(POC 50). The generic mechanism is not built.

**Premise.** ⭐ Proven prior art, read at file level:
`references/allyourcodebase__pipewire/tree/src/wrap/dlfcn.zig` exports
`__wrap_dlopen`/`__wrap_dlsym`/`__wrap_dlclose` against a compiled-in table.
That is the same delivery mechanism `pgb` already uses for `iconv_open`.

**Approach.** Cheapest of the four routes. Generate the table from the plugins
the build produced; wrap at the final link as `pgb-iconv.c` does.

**Prove.** POC 50's CPython rebuilt with `--wrap-dlopen` instead of hand-written
`Modules/Setup.local`, passing the same matrix.

### Progress — the mechanism, built and measured

⚠ **This section was written while the entry was still open**, before the
operator ruled on the corrected acceptance. It is kept because it is what the
mechanism was proved on first: a purpose-built subject, not a real project.

**Landed:**

| | |
|---|---|
| `tool/runtime/pgb-dlopen.h` | the table layout, shared by the reader and the generated writer |
| `tool/runtime/pgb-dlopen.c` | `__wrap_dlopen`/`dlsym`/`dlclose`/`dlerror` against that table |
| `pgb --wrap-dlopen NAME=OBJ[,OBJ...]` | repeatable; generates the table with `nm --defined-only --extern-only` |
| `pgb explain` | prints the four wraps and the generated table when the flag is given, and ⭐ is byte-identical to before when it is not |
| `experiments/71-wrap-dlopen.sh` | the measurement |

**Measured**, `evidence/71-wrap-dlopen/RESULT.txt`, 11 environments:

```
  wrapped: all six assertions pass, every environment      = 11 of 11
  wrapped: loaded no host shared object, every environment = 11 of 11
  plain arm ran (observed, not asserted)                   =  0 of 11
```

⭐ **Three of the six assertions are NEGATIVE**, because a wrapper that
returned a handle for everything would pass a naive "did dlopen work" test:
a file-local `static` must **not** resolve through `dlsym`, a plugin absent
from the table must **not** open, and `dlerror()` must be set in both cases.
The subject also calls **through** the returned pointer and checks the value,
since a non-NULL pointer proves a table lookup and not a working plugin.

⭐ **Cost: 544 bytes.** The wrapped binary is 1,012,960 against the plain
arm's 1,012,416 for this table. The mechanism is close to free; what a real
project pays is the size of the plugins it links in, which it would have paid
anyway.

### ⛔ The acceptance as written cannot be satisfied, and the reason is a result

**The premise keeps its title**, per `../docs/methodology/authoring.md`. What
follows is the correction.

The `Prove` above needs a subject: CPython with modules as
`lib-dynload/*.so`, so that the interpreter actually calls `dlopen` and
`--wrap-dlopen` has something to answer. **That subject cannot be built**, and
it fails nowhere near the plugin-loading code. CPython 3.12.7, `--disable-shared`,
modules left shared, built in the pinned environment:

```
ImportError: build/lib.linux-x86_64-3.12/math.cpython-312-x86_64-linux-gnu.so:
             undefined symbol: PyLong_AsLongLongAndOverflow
make: *** [Makefile:1125: checksharedmods] Error 1
```

Measured on that interpreter: `statically linked`, PT_INTERP 0, DT_NEEDED 0,
and ⛔ **`.dynsym` entries: 0**. `math.so` has 27 undefined `Py*` symbols. The
interpreter **does** define the missing one — `nm ./python` finds it — but
only in its static symbol table, which a dynamic loader never consults.

⭐ **`experiments/72-static-host-plugin-abi.sh` reduces that to a subject that
builds in a second**, so it is a check this project keeps rather than a story
about one afternoon:

| arm | `.dynsym` | outcome |
|---|---|---|
| dynamic host, `-rdynamic` (**positive control**) | 11 | `PASSED`, the plugin called back |
| static host | **0** | `FAIL dlopen: undefined symbol: host_api_add` |
| static host + `--wrap-dlopen` | 0 | `PASSED`, the plugin called back |

⛔ **The finding is structural, not a defect.** A statically linked executable
has an **empty dynamic symbol table**, so a shared object loaded into it can
never resolve a reference back to the host program. `-rdynamic` exports
through `.dynsym`, and there is no `.dynsym` to export through. ⚠ This is
*prior* to `experiments/50-`'s loader failures: even a loader that worked
perfectly would have nowhere to look.

⭐ **Which turns `--wrap-dlopen` from a convenience into the only thing that
can work** for a plugin that calls into its host — and explains why CPython's
own `Modules/Setup.local` mechanism has the shape it does. The two converge by
necessity rather than by coincidence.

### ⭐ The corrected acceptance — ACCEPTED by the operator, 2026-09-01b

⛔ **This entry now closes on the text below and NOT on the `Prove` above.**
The ruling was taken at the start of the session of 2026-09-01b, on the
question the previous session left open, and it is recorded here rather than
only in a transcript:

> `--wrap-dlopen` builds a project whose plugin loading is **not**
> configurable at build time — i.e. one with no `Setup.local` equivalent —
> with its plugin directory emptied and the functionality intact, on 11 of 11.

⚠ **The original `Prove` keeps its place above** per
`../docs/methodology/authoring.md`: it is what the entry was opened on, and
`experiments/72-` is why it moved.

⚠ CPython is a poor subject for this entry precisely *because* it already has
a static-modules mechanism: rebuilding it on `--wrap-dlopen` would demonstrate
the two routes agreeing, not the mechanism reaching something that was out of
reach. **T-002 names the right kind of subject** and this entry should take
its subject from there.

### ✅ Closed with `poc/70-sqlite-extensions/` — the corrected acceptance, met

⭐ **One build served this entry and T-002**, which is what the previous
session's work order predicted. `evidence/poc/70-sqlite-extensions/RESULT.txt`,
`pass=20 fail=0 skip=0`.

SQLite satisfies "not configurable at build time" strictly: `.load` calls
`dlopen()` on a path the user names and derives the entry point from the
filename, and there is no configure switch that links an extension in while
keeping `.load` working. **Fifteen** extensions, the plugin directory created
**empty** on every target:

```
  11 of 11   functional test ok, values asserted, 3 negative assertions
  11 of 11   host shared objects loaded: none
```

⛔ **And the mechanism did not survive the scale unchanged.** Every SQLite
extension defines a non-static `sqlite3_api`, so any two collided at link
time; two of them define the same entry point by upstream design. Fixed with
per-plugin symbol namespacing (`objcopy --redefine-syms`) — the full account
is in T-002, and the collision is kept as a live check rather than written up
and deleted.


## T-033 — route D: compile an ELF loader in, resolve against our own static glibc

**Source** `docs/research/solo.md`, the `pg83/solo` sweep, session of
2026-09-01b. **Category** runtime · **Priority** P1 · **Effort** L ·
**Status** ✅ done — **SUPERSEDED BY T-064**

## ⛔ CLOSED 2026-09-03 AS A DUPLICATE, and it had been open describing finished work

⭐ **T-064 IS THIS ENTRY.** *"Route D: compile an ELF loader in, resolve against
our own static glibc"* is what `pgb build --host-dlopen` does and what T-064
closed — `tool/runtime/pgb-elfload.c`, `experiments/76-`, 11 of 11 carried with
zero host shared objects, a real host `.so` on 7 of 7 glibc rows. `docs/AGENTS.md`
§13 has recorded T-064 as closed since it landed.

⛔ **AND IT COST SOMETHING TO LEAVE OPEN.** `docs/REQUIREMENTS.md` — the
operator's binding acceptance bar — pointed its ONE remaining unmet issue at
**this entry**, in a sentence naming no shipped mechanism, so the bar read as
though nothing had been built for the host-plugin class. Both cells are
corrected and now name T-064 and T-068. ⚠ The row itself stays **open** on that
page: the issue is *host-dependence* and it persists.

⚠ **Nothing here is re-closed on new evidence.** This is the record catching up
with a measurement that was taken on 2026-09-02: `RULES.md` §"The record is part
of the change" is the rule it broke.

**Problem.** `docs/limitations.md` §1 is the project's one measured, unfixed
failure and the reason `REQUIREMENTS.md` part 1 is not met: `dlopen` of a
**host** shared object from a static glibc binary is host-dependent, and
success is the worse outcome because the host's `ld.so` and `libc.so.6` enter
the process.

**Premise.** ⭐ **Measured, `experiments/73-`, and it is what makes this entry
worth opening rather than arguing.** `pg83/solo` (MIT, `79451211`) shows the
shape: do not use the host loader at all — map the object yourself and resolve
its imports against a table of the executable's own symbols. solo does it on
musl and pays 5,948 lines translating the guest's glibc imports onto a musl
runtime. ⛔ **A static glibc host has no translation to do**, and `73-`
measures how much of the demand its own libc already meets:

```
  6,007 host shared objects, the seven glibc environments (6,392 in all 11)
  90.8% - 99.3% of every GLIBC_/GCC_-versioned import already definable
  class E, the unexplained residue                          = 0
```

Every remaining symbol falls into a class with a measured reason: the host's
`ld.so` exports it (a compiled-in loader owns those), a version ceiling, or a
symbol `libc.so.6` keeps that `libc.a` never had. `docs/research/solo.md` has
the table and the four mechanisms worth taking, at file and line.

**Approach.** ⚠ **Split before starting; this is not one entry.** In order:

1. **The provider table.** `pgb --wrap-dlopen` already generates one with `nm`
   (`tool/runtime/pgb-dlopen.h`). This is the same mechanism applied to the
   **libc** instead of the application's plugins, and taking each address is
   also what forces the archive members into the link.
2. **The mapper.** `PT_LOAD` mapping, `DT_NEEDED` walk, relocations, RELRO,
   initialisers. ⛔ **This does not get cheaper for being glibc**: solo spends
   2,707 lines on it (`lib/elf_loader.cpp`).
3. **TLS.** ⛔ **The hard part, and the one place "we are glibc, so it is
   simpler" is NOT obviously true.** solo donates a `thread_local` pad that
   musl sizes into every thread and registers guest blocks into
   `libc.tls_head` (`lib/musl_tls.c`). glibc's equivalent is
   `_dl_tls_static_surplus`/`__libc_setup_tls`, a different mechanism. Measure
   this before committing to the rest.
4. **The compat layer for what `73-` named**: the 20 class-B symbols (mostly
   the `__isoc23_*` family, which are aliases with C23 `strtol` semantics) and
   the 49 class-S ones (sunrpc — `libtirpc.a` is in the pinned environment and
   defines them, measured).

**Prove.** A `pgb` binary loads a **host** shared object, calls into it, the
plugin calls back into the host program, and `pgb verify` reports **zero host
shared objects** on all eleven — the last clause being the whole point, since
`experiments/50-` already has "it loaded" on two environments and that was the
failure.

**Blockers, named.**
- ⛔ Symbol availability is not a working `dlopen`. `73-` counts names. IFUNC
  resolution, the stdio ABI, pthread object sizes and TLS layout are untested.
- ⚠ `docs/design/tiers.md`'s bar applies: this stays tier 1 — one ordinary ELF,
  nothing mounted, nothing written — or it is route C wearing a different name.

⭐ **Relation to the other three routes**, `docs/AGENTS.md` §7:
route A is built and serves a program's own plugins; route B is **weakened** by
`73-`'s second control (stripping versions off the object named in
`DT_VERNEED` makes glibc's loader assert, `dl-lookup.c:106`); route C gives up
the single ELF. This is the cheapest route that reaches the host-plugin class
without giving that up.

**Depends on** T-018 (done): a loader cannot unwind across the boundary unless
the executable's own unwind tables are discoverable through program headers.


## T-032 — `--embed-terminfo` and a CA-bundle answer

**Source** `docs/limitations.md` §3 · **Category** runtime · **Priority** P1 · **Effort** S · **Status** ✅ done

⚠ **PROMOTED to P1**: the operator's framing of the goal names both —
*"no networking/iconv/gconv/nss/locale/**cert**/etc issues"* — and they are two
of the three open rows of `REQUIREMENTS.md` part 2.

**Problem.** Two of five host data dependencies are open. Both are reached
through an environment variable, which is the shape `--embed-locale` already
proved.

**Prove.** POC 20's `setupterm()` probe passing on all 11, and POC 30's curl
verifying TLS on all 11 with the harness's own CA variables unset.

### ✅ CLOSED 2026-09-01d — both POCs run to completion, 11 of 11 each

⭐ **The acceptance is the two POC runs and they are done.** Only the runs were
owed; the mechanisms had landed in the previous session.

**POC 20 — `--embed-terminfo`.** `evidence/poc/20-nano/RESULT.txt`, nano 8.2 on
ncurses 6.5. **12 assertions, 0 failures.** The `setupterm()` check is *inside*
the functional test, not beside it, and it runs with `TERMINFO` and
`TERMINFO_DIRS` unset so the harness cannot answer for the host:

```
alpine 3.22 / 3.20 / 3.10   host-terminfo-tree=no    setupterm(xterm-256color)=OK
the other eight             host-terminfo-tree=yes   setupterm(xterm-256color)=OK
```

**POC 30 — `--embed-cacert`.** `evidence/poc/30-curl/RESULT.txt`, curl 8.11.0 on
OpenSSL 3.0.15 and zlib 1.3.1. **12 assertions, 0 failures.** Step 8,
`https-verify-host`, verifies a real TLS handshake **with the harness's own
`CURL_CA_BUNDLE`, `SSL_CERT_FILE`, `SSL_CERT_DIR` and `CURL_CA_PATH` unset**,
on all eleven. The old *"trust store missing on host"* branch is a failure now,
not an `ok`.

Host shared objects loaded: **none, on every row of both**.

⚠ **One instrument note, and it is the rule this session added to
`RULES.md`.** POC 30's first run reported `voidlinux-musl: SIG9`. Nothing was
wrong with the binary: `experiments/85-` was reaping the same rootfs at the
same moment, and its reaper kills by `/proc/PID/root` and cannot tell one
run's process from another's. Re-run serialised, it passes. ⛔ **A row that
says SIG9 in an otherwise clean table is what concurrency on the shared bed
looks like** — it does not announce itself.

⛔ **And one real defect found by the re-run**: `tool/runtime/pgb-cacert.c` was
missing `<stdio.h>`, so every `--embed-cacert` build printed
`implicit declaration of function 'snprintf'` and compiled a call whose return
type C only assumes. `pgb-locale.c` carries the same include with a note
saying the same mistake was made and fixed *there* first. A warning under
gcc 12 and an **error** under C23. The other five runtime pieces were checked
and are clean.

**Landed:**

| | |
|---|---|
| `tool/runtime/pgb-cacert.c` | a constructor that probes nine known trust-store locations, then materialises an embedded copy only where there is none |
| `tool/runtime/pgb-terminfo.c` | the same shape for terminal descriptions, with the host's database preferred |
| `pgb --embed-cacert`, `--embed-terminfo` | both opt-in, both carried across the engine boundary |
| `experiments/74-cacert.sh` | 6 assertions |
| `experiments/75-terminfo.sh` | 4 assertions |
| `poc/common.sh` `POC_PGB_FLAGS` | lets a POC ask for an opt-in mechanism for its own builds |

**Measured**, `evidence/74-cacert/RESULT.txt` and `evidence/75-terminfo/RESULT.txt`:

| | with the mechanism | without |
|---|---|---|
| a usable **TLS trust store** found | **11 of 11** | 5 of 11 |
| a **terminal description** for `$TERM` reachable | **11 of 11** | 7 of 11 |
| never overrode a value the caller had set | 11 of 11 | — |
| wrote anything to the filesystem | **3 of 11** (CA), **4 of 11** (terminfo) — exactly the hosts with nothing of their own | — |

⭐ **The finding that shaped both mechanisms**: most of the failures were never
*"this machine has no certificates"*. Rocky keeps its bundle at
`/etc/pki/tls/certs/ca-bundle.crt`, openSUSE at `/etc/ssl/ca-bundle.pem`,
Alpine 3.10 at `/etc/ssl/cert.pem`. **The data was there all along, on a path
the binary had never been told about.** Only three of eleven genuinely ship
none. So the first layer is to *look*, and the embedded copy is a fallback.

⛔ **And the order of those two layers is a SECURITY property, asserted as
one.** The embedded bundle is a build-time snapshot; roots are revoked and
expire. A binary preferring its own stale copy over a store an administrator
maintains would be a security regression wearing a portability fix's clothes.
`74-` checks, against an independent oracle rather than the shim's own answer,
that nothing was written on any host that has a store.

**What was left, and is now done:** the two POC runs above. Both POCs already
carried their flags and their new assertions; only the runs were owed.

⚠ **One confound to watch when running POC 30**: this development environment
routes HTTPS through a proxy that exports its own `CURL_CA_BUNDLE` and
`SSL_CERT_FILE`. The POC already unsets them for exactly that reason and the
existing observation records why — `docs/history/corrections.md`. If the new
step 8 fails on hosts that DO have a store, suspect the proxy's certificate
before suspecting the mechanism, and say which it was.

---


## T-064 — ⛔ P0: make static glibc's `dlopen` REALLY solved, with our own loader

**Source** ⭐ **operator, 2026-09-02b**: *"static glibc really 'solved', restudy
solo reference, implement a better faster version of cross libc dlopen"*.
**Category** runtime · **Priority** P0 · **Effort** XL · **Status** done

⛔ **WORK UNTIL IT IS MET OR THE PREMISE IS SIGNIFICANTLY ADVANCED.** This is
not a spike. `docs/limitations.md` §1 is the project's one measured, unfixed
failure and the reason `REQUIREMENTS.md` part 1 is not met.

**Problem.** A static glibc binary cannot `dlopen` a host shared object. Where
it succeeds — ⭐ **exactly the row whose host glibc equals the build's**, which
is Fedora 42 at the 2.41 pin and was Debian 12 at 2.36, measured in
`poc/10-gawk` — the success is the *worse* outcome: the host loader and a
second libc enter the process. `docs/limitations.md` §1.

**Premise, and it is already measured rather than hoped for.**

- `experiments/73-` parsed **6,392 real host shared objects**, 6,007 of them
  across the seven glibc environments: **90.8%–99.3%** of every `GLIBC_`/`GCC_`-versioned import
  is already definable by the pinned static glibc, and the unexplained residue
  is **zero**. The symbols are there.
- `experiments/72-` measured why the host loader can never be the answer: a
  static executable's **dynamic symbol table is empty**, so a host-loaded
  plugin has nothing to bind back to. ⛔ A perfect loader would still have
  nowhere to look — which is exactly why the loader has to be **ours**.
- `references/pg83__solo` is a working ELF loader: `lib/elf_loader.cpp` is
  **2,707 lines** and `lib/dlfcn.cpp` **370**. ⭐ It pays most of that
  translating a guest's glibc imports onto **musl**. A static **glibc** host
  has no translation to do, so the port is a subtraction, not a rewrite.

**Approach.** Map the object ourselves, walk `DT_NEEDED`, relocate, and bind
its imports to a table of the executable's own symbols — the same generated
table `--wrap-dlopen` already builds, pointed at libc instead of at the
application's plugins. Nothing of the host's is mapped, so no second libc
enters and the output stays one ordinary ELF.

⚠ **The honest unknowns, named so they are not discovered late:** TLS is the
one place where "we are glibc, so it is simpler" is not obviously true;
`IFUNC`/`STT_GNU_IFUNC` resolvers run at relocation time; and symbol
*versioning* must be honoured, not ignored, or the wrong definition binds
silently.

**Relationship to the older entries.** T-033 is route D and is this entry's
predecessor — it stays open only as the research note. T-031 (port
cross-libc-dlopen's full rewrite) is the *other* direction, letting host
objects in, and ⛔ **is not this**: `experiments/50-` already measured that
route producing no effect.

**Prove.** ⛔ Not "it links". A POC that `dlopen`s a **real host shared
object** — start with the gawk extension `poc/10-gawk` already fails on — on
**11 of 11**, with `pgb verify` reporting **zero host shared objects loaded**
on every row, against a control that uses the host loader and fails. Plus a
measurement against solo's own loader: ours must be smaller in lines and
faster to first symbol, or the "better faster" in the instruction is unmet.

## ✅ Done — `sh experiments/76-host-dlopen.sh`, exit 0, four of four

`evidence/76-host-dlopen/RESULT.txt`. The mechanism is
`tool/runtime/pgb-elfload.c` and `pgb build --host-dlopen`.

```
TARGET                 LIBC   CARRIED   NATIVE    CONTROL   HOST .so LOADED
alpine-3.22            musl   ok        exit1     exit1     none
alpine-3.20            musl   ok        exit1     exit1     none
alpine-3.10            musl   ok        exit1     exit1     none
voidlinux-musl         musl   ok        exit1     exit1     none
debian-11              glibc  ok        ok        SIG6      none
debian-12              glibc  ok        ok        SIG11     none
ubuntu-20.04           glibc  ok        ok        SIG6      none
rockylinux-8           glibc  ok        ok        SIG6      none
opensuse-leap-15.6     glibc  ok        ok        SIG8      none
fedora-42              glibc  ok        ok        SIG8      none
archlinux-latest       glibc  ok        ok        SIG11     none

  carried: nine assertions pass, every environment     = 11 of 11
  carried: loaded no host shared object, every one     = 11 of 11
  native:  loads a real host object on every glibc row  = 7 of 7
  native:  refuses CLEANLY on every musl row, no signal = 4 of 4
  control: ran                                          = 0 of 11
```

⭐ **On the four musl rows the carried arm is a GLIBC shared object being
`dlopen`'d on a machine that ships no glibc**, from one ordinary static ELF —
`PT_INTERP=0 DT_NEEDED=0` — with nothing beside it.

**Against solo, at `79451211`.** ⭐ Smaller, and by more than the instruction
hoped for: the expensive half of solo is the part a glibc host does not need.

| | |
|---|---|
| ours, `pgb-elfload.c` | **1,093 code lines** (1,555 with comments) |
| solo, `elf_loader.cpp` + `dlfcn.cpp` | 2,332 code lines |
| solo, `glibc_shim.cpp` + `musl_tls.c` | 6,296 raw lines — glibc onto **musl**, and ours needs none of it |

**Time to first symbol**, debian-12, one sample each, two objects in one
process:

| | first | second |
|---|---|---|
| ours | **147,543 ns** | 166,220 ns |
| the same static binary reaching the host `ld.so` | 711,066 ns | 41,430 ns |

⚠ **The per-load figures are at the noise floor** — ours' second exceeds its
first — and are recorded, never asserted. The first column is not: the control
must bring in `ld.so` and `libc.so.6` before it can load anything.
⚠ **solo is not an arm and cannot be** — `docs/research/solo.md` records three
failed build attempts on this machine, so nothing here has executed it.

**The three honest unknowns the entry named, answered.**

| | |
|---|---|
| **TLS** | half right. General dynamic was easy — the loader owns `__tls_get_addr`, which is `experiments/73-`'s class A. Initial exec needed glibc's own surplus: `_dl_tls_static_used`/`_size`/`_align` are plain 8-byte `GLOBAL OBJECT`s in `libc.a`, and `__libc_setup_tls` already allocates `memsz + surplus` in every thread, so the space exists and is only unclaimed. ⚠ A module is seeded with its init image in the loading thread only. |
| **IFUNC** | `R_X86_64_IRELATIVE` runs the resolver at relocation time, as `ld.so` does. No separate problem appeared. |
| **symbol versioning** | honoured, not ignored. A wrong version is a loud miss; an unversioned definition satisfies a versioned reference, which is `ld.so`'s documented compatibility rule and where the provider table sits. |

**What is left, measured on 904 host objects on the build host** (818 load):
20 undefined symbols, 4 `TLSDESC`, 2 objects wanting 56,248 bytes of static TLS
against ~3,168 bytes of headroom, and 30 crashes that are almost all objects no
static image should load — NSS modules, sanitizer and allocator interposers.
The exception is `libLLVM`, which maps and relocates cleanly and dies in the
605th of its C++ static constructors. ⚠ These are **T-068**, not this entry.

**Four defects found while building it, every one by something disagreeing.**

| | |
|---|---|
| `libm.a` is a **GNU ld script**, not an archive | read as `ar` it yields zero symbols in silence; the table had 4,891 names instead of 7,216 |
| `__tls_get_addr` is in **no archive** | `ld.so` exports it; 398 of 492 undefined-symbol failures were that one name |
| **`DT_RELR`** was ignored | Fedora and Arch pack relative relocations into a bitmap; ignoring it is a **silent wrong answer**, not a failure — `init_array[0] 0x670`, an unrelocated vaddr |
| `make` did not depend on the **go:embed'd** C | editing the loader printed "Nothing to be done" and the next build used the previous loader |


## T-068 — the residue `--host-dlopen` does not load, and it is 86 of 904

**Source** the sweep in T-064: every shared object on the build host, 904 of
them, one fork each, through `tool/runtime/pgb-elfload.c`.
**Category** runtime · **Priority** P1 · **Effort** M · **Status** ✅ done

## ✅ CLOSED 2026-09-03 — the Prove, run, with the arc that earned the zero

    sh experiments/93-host-object-residue.sh
    ok=882 refused=122 failed=478 crash=45 hang=0
    45 of 45 also crash glibc's own ld.so
    ok  nothing crashes this loader that glibc's loader loads = 0
    pass=6 fail=0 skip=0     VERDICT: matched expectation

⭐ **`libLLVM-17.so.1` is `ok` in `per-object.txt`** — the Prove asked for it
"either loading or its 605th constructor explained", and it loads.

⛔ **THE ZERO IS EARNED, AND THE ARC IS WHY IT CAN BE BELIEVED.** One machine,
one population of 1,527 objects, four builds:

| build | ok | crash | ⛔ crashes that glibc LOADS |
|---|---|---|---|
| at session start | 406 | 45 | — |
| **+ the iconv fix** | 620 | 55 | ⛔ **10** |
| **+ the general-dynamic TLS fix** | 629 | 46 | ⛔ **1** |
| **+ the structural interposer refusal** | 628 | **45** | ⭐ **0** |
| **+ `_dl_mcount_wrapper_check`, and the versioned-lookup fix** | ⭐ **882** | 45 | ⭐ **0** |

⚠ **The first `DIFFER = 0`, recorded 2026-09-02e, was not the loader being
right** — the ten objects the control should have caught were failing earlier,
on iconv, and never reached the code that crashes them. The assertion's
falsifiability was demonstrated live this session: it read 10, then 1, then 0,
as each defect was fixed.

⚠ **`ok` is 628 and not 629 on purpose**: `gprofng/libgp-heap.so` is a lost
`ok`, and it is the right answer — it "loaded" only because its constructor had
not allocated yet.

### ⭐ AND THE 631 UNDEFINED SYMBOLS WERE JOINED TO 73-, WHICH IS WHAT THIS ENTRY ASKED FOR

⛔ **"Our loader failed on 631 objects" is not a defect count either**, and the
control that says so is this entry's own, pointed at failures instead of
crashes. Of the 631, **glibc's own `ld.so` also fails 374** — plugins of a host
PROGRAM: CPython (`PyExc_ValueError`, `_Py_NoneStruct`), Perl
(`Perl_stack_grow`), PostgreSQL (`CurrentMemoryContext`), PHP (`zend_*`).
Their symbols live in the executable that loads them.

⭐ **The residue that is ours was the other 257, and it was SIX symbols:**

| symbol | objects | class | outcome |
|---|---|---|---|
| `_dl_mcount_wrapper_check@GLIBC_2.2.5` | **247** | S | ✅ **owned**, as a no-op |
| `dm_task_get_info@DM_1_02_97` | 4 | version | ✅ **lookup fixed** |
| `lzma_cputhreads@XZ_5.2` | 3 | version | ✅ |
| `lzma_stream_encoder_mt@XZ_5.2` | 1 | version | ✅ |
| `xdr_void@GLIBC_2.2.5` | 1 | S, sunrpc → libtirpc | ⛔ open |
| `_rtld_global_ro@GLIBC_PRIVATE` | 1 | A, `ld.so` owns it | ⛔ open, and a fake would be a silent wrong answer |

**254 of 257 load.** The three left are `libnsl.so.1`, `libmvec.so.1`, and
`libcuilo.so`, which hits `more than 64 objects loaded` — a limit of this
loader's object table, not a symbol problem, and the one place LibreOffice's
dependency graph is bigger than the loader.

⛔ **AND THE VERSION LOGIC IS UNDER-MEASURED, WHICH THE GUARD-MUTATION REVIEW
FOUND AND THIS ENTRY DOES NOT ROUND OFF.** Two mutations:

    el_accept ignores the version entirely   the motivating objects STILL LOAD
    drop the unversioned fallback            254 of 257 -- IDENTICAL

The three branches of the version rule are exercised by exactly one of this
host's 1,527 objects' worth of demand. *A wrong version is never returned* holds
by construction — exact match wins, unversioned is a fallback, wrong keeps
looking — but nothing asserts it. ⭐ **A synthetic versioned fixture is what
would close that, and it is not written.**

⛔ **What is NOT closed, each with its own home:** the 376 `undefined` that
remain (374 of them not ours), the 3 `tlsdesc`, the 29 `missing-dep`, the two
named symbols above, and the one object the classifier still cannot name.

⛔ **This exists so T-064's residue is carried as work rather than rounded off
in a summary.** ⚠ **The title's `86 of 904` is the number this entry was
OPENED on, and it is not reproducible** — that sweep was ad-hoc and never
committed, which is what `experiments/93-host-object-residue.sh` was written to
replace. The title is left as the entry's own history; everything below is the
harness's.

## ⭐ THE HARNESS, AND WHAT IT MEASURES — `experiments/93-host-object-residue.sh`

One `timeout`ed fork per object, so a crash leaves the rest measurable, and it
classifies **from the loader's own `el_err()` strings** rather than from an
exit status, with anything it cannot classify **printed rather than absorbed**.
⚠ **The population is this HOST's shared objects, so the count moves with the
machine** — `docs/AGENTS.md` §0b's conditions rule, in a count.

    93 - host shared objects through pgb-elfload
    host        : Linux 6.18.44-fc-v22        objects : 1527

    ok=446  refused=119  failed=917  crash=45  hang=0

      undefined              889      served-by-image        10
      not-an-object           96      refused-interposer      9
      crash                   45      refused-nss             4
      missing-dep             27      other                   1

## ⛔ THE PROVE WAS THE WRONG QUESTION, AND THE CONTROL IS WHAT SETTLED IT

This entry's Prove asked for **the crash count at zero**. ⛔ Driving it there
directly would have been wrong twice over, because *"our loader crashed on N
objects"* is not a defect count: **some host objects cannot be loaded
standalone by ANY loader.**

⭐ **So 93- builds a second, ORDINARY DYNAMIC probe** — same source, same
`dlopen`, the host's real `ld.so` — and runs it on every object that crashed
ours:

    of 46 objects that crashed this loader, glibc's own crashed on 45

The 45 are **xtables extensions**. Their `DT_INIT_ARRAY` calls
`xtables_register_target()`, and `libxtables` dereferences `xt_params` —
verified an 8-byte GLOBAL OBJECT in its `.bss`, which only the iptables
**program** assigns. Outside iptables it is NULL and the fault is at
`si_addr=0x18`, an offset into the struct it points at. **Not our defect.**

⭐ **THE ONE THAT DIFFERED IS A REAL DEFECT, AND IT WOULD HAVE BEEN BURIED
AMONG 45 NON-DEFECTS**: gprofng's `libgp-collector.so`, which glibc loads
cleanly and we segfaulted on, deterministically, 4 of 4. It **defines `mmap`
and imports `dlsym`/`dlvsym`** — the `LD_PRELOAD` interposer signature — so it
chains through `RTLD_NEXT`, which a static image does not have and
`pgb-dlopen.c` already reports as its own error. Declined by name with the
other interposers now. `libgp-collectorAPI.so`, the API half, is not an
interposer and still loads.

⛔ **AND THE TEMPTING WRONG FIX IS RECORDED SO NOBODY RE-DERIVES IT.** A rule
declining `libxt_ libipt_ libip6t_ libebt_ libarpt_` was written, measured, and
**taken out**: it drives the crash count to zero and drops `ok (loaded)` from
**446 to 377**, because 69 xtables modules load fine. That trades 69 measured
successes for a green number. ⚠ 93- measures **loading**, not behaviour, and
unmeasured is not a licence.

**So the Prove is restated:**

> ⭐ **Nothing may crash this loader that glibc's loader loads.** It is a
> *falsifiable* zero: remove the `libgp-collector` refusal and the control
> fails at 1, naming the object and printing that glibc loads it at exit 0.
> `hang = 0` is asserted separately — a refusal with a message is the loader
> working, a signal is it failing, and the two are never summed.

### ⛔ AND THE FIRST TIME IT PASSED, IT PASSED BECAUSE OF A DEFECT

⚠ **That `DIFFER = 0` was not the loader being right.** Re-run after the
`--host-dlopen` iconv fix (see below), the same sweep on the same machine gives
**`DIFFER = 10`** — ten objects crash this loader that glibc's loader loads. All
ten had been failing **earlier**, on iconv, so they never reached the code that
crashes them.

    ok=620 refused=119 failed=733 crash=55 hang=0    (was ok=446 ... crash=45)
    FAIL  nothing crashes this loader that glibc's loader loads = 10, expected 0

⭐ **Controlled on one machine, one population, the iconv fix alone:
`ok = 406 → 620`.**

### ⭐ TWO REAL LOADER DEFECTS, 2026-09-02f, and neither was found by reading

**1. `--host-dlopen` could not load anything that used iconv.** The generated
provider table declares every glibc symbol as `extern char NAME[] __attribute__
((weak))` — an *undefined* reference — so `-Wl,--wrap=iconv_open` rewrote it to
`__wrap_iconv_open`, and **a weak undefined reference does not pull a member out
of an archive**. The table entry held NULL and every host object importing
iconv failed, naming the *unwrapped* symbol. `internal/wrapper/flags.go`, and
`internal/wrapper` gains its first selftest — 24 cases, both directions.

**2. ⛔ The two halves of a general-dynamic TLS pair disagreed, silently.**
`R_X86_64_DTPMOD64` searched every loaded object for the symbol;
`R_X86_64_DTPOFF64` searched only the object being relocated. A reference to a
thread-local defined in **another** module therefore came out as *(the right
module, offset 0)*. ⚠ **That is not primarily a crash — it is one module
reading and WRITING at offset 0 of another module's thread storage**, which is
the exact hazard the `R_X86_64_TPOFF64` case eight lines below already refuses
by name to avoid.

⭐ **How it was found, because none of it is visible at the point of failure.**
A `SIGSEGV` handler gave `rip=0, si_addr=0` — naming nothing — and a stack read
put the return address one instruction after `call *%rbp` in glibc's
`__pthread_once_slow`. ⚠ **The obvious reading, "a NULL init routine", was
wrong**: a temporary relocation trace showed `__once_proxy` resolving correctly
and `%rbp` equal to it, so the fault was *inside* `__once_proxy`, whose thirty
bytes are `__tls_get_addr(&__once_call)`, load, `jmp *%rax`.

    libLLVM-17   __tls_get_addr(__once_call) -> block + 0      writes here
    libstdc++    __tls_get_addr(__once_call) -> block + 0x10   reads here

⛔ **Two other suspects were ruled out by measurement, not by reading.** The 16
weak symbols that load binds to zero: **not one** is defined anywhere in
libLLVM's own `DT_NEEDED` closure, so glibc binds them to zero too. And a lookup
succeeding with a null address: a new diagnostic says so explicitly, and it
fired zero times.

**Fixed**: one lookup serves both halves, and a pair nobody can satisfy is
refused by name. Measured on the seven LLVM-family objects the control named:

    before  SIGSEGV, 7 of 7        after  loaded, 7 of 7

## ⛔ WHAT IS LEFT, RANKED BY WHAT IT WOULD BUY

| n | what | route |
|---|---|---|
| **889** | **undefined symbol** — by far the largest class | ⭐ **`experiments/73-` is the instrument that already classifies these**, and it has just been re-run at the 2.41 pin: class A (host `ld.so` owns it), class B (host glibc newer than the pin — now **5 distinct symbols**, all on `archlinux-latest`), class S (in `libc.so.6`, never in `libc.a` — 50 symbols), class D (not the host libc's either), class E (**empty**). ⛔ The two harnesses have never been **joined**: 73- ranks symbols across eleven environments and 93- names objects on one host. Joining them is this row's work, and it turns 889 into a per-symbol worklist with a measured population behind each |
| **96** | **not-an-object** | ⚠ **not a failure and possibly not even a miss.** `libc.so`, `libm.a` and friends in `/usr/lib` may be GNU **ld scripts**, not ELF — three independent sightings, `docs/history/corrections.md` C18. Confirm the whole 96 is that class rather than assuming it |
| **45** | **crash** | ✅ **explained, and not ours** — the control above. They stay visible rather than being refused into invisibility |
| **27** | **missing-dep** | a `DT_NEEDED` the image neither provides nor can find. Decide per object whether the dependency is one a bundle would legitimately carry (`design/host-fallback.md`'s four classes) or one no static image should pull in |
| **10** | **served-by-image** | ⭐ **not a failure at all** — the object names a `DT_NEEDED` the image already satisfies, so it is ANSWERED rather than opened. This is the mechanism that keeps a second libc out, counted |
| **13** | **refused by name** — 9 interposers, 4 `libnss_*` | ✅ **policy, and the policy is measured.** `docs/AGENTS.md` §14: keeping NSS out **is** the fix. An interposer must be present before libc initialises, which in this image it already has |
| **1** | ⛔ **unrecognised by the classifier** — `libsyslookup.so`, *"relocation names symbol 1 of 1"* | the one row that says the classifier is incomplete. It is **printed rather than absorbed**, which is why it is visible at all |

⚠ **And two things that are not counts.**

1. **A module placed in glibc's static TLS surplus is seeded with its
   initialisation image in the thread that loaded it**; threads created
   afterwards see zeros. Correct for the modules whose `PT_TLS` `p_filesz` is 0,
   wrong for the rest. ⭐ The route is `-Wl,--wrap=pthread_create`, which `pgb`
   already uses for four other mechanisms — seed the new thread's slices on the
   way in. The surplus itself is **T-072**.
2. ⛔ **`R_X86_64_TLSDESC` needs a resolver trampoline.** solo implements it;
   `lib/elf_loader.cpp` at `79451211` is the read. ⚠ It did not appear as a
   named reason in this run's classification, which is a statement about this
   host's objects and not about the relocation.

---


## T-073 — ⭐ the own-symbol table answered when one of its two names had to defer

**Source** operator, 2026-09-03, on `pkgforge-dev/cross-libc-dlopen#28` / PR 30.
**Category** runtime · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** `el_provider()` in `tool/runtime/pgb-elfload.c` checked
`el_own_syms[]` **first and unconditionally**. The table's two entries have
**opposite** requirements and nothing in the code, the comments or the harness
distinguished them:

| name | requirement | why |
|---|---|---|
| `__tls_get_addr` | ⛔ **must WIN over everything** | the module ids it is handed are minted by this loader's own `R_X86_64_DTPMOD64` case; nothing else in the process can interpret them |
| `_dl_mcount_wrapper_check` | ⛔ **must YIELD to any real definition** | it is a stand-in whose whole content is *do nothing*, so answering shadows a real one with a no-op |

⭐ **One table cannot express both**, and the table was safe **by accident**:
`el_resolve()` happens to call `el_provider()` last, which is right for
`_dl_mcount_wrapper_check` and wrong for `__tls_get_addr`. ⛔ **Nothing
asserted that ordering** and neither entry recorded that it depended on it.

**Premise.** ⚠ Upstream's #28 is the same shape and is **not our bug** — it is
an `LD_PRELOAD` interposition defect in a forwarding shim, `pgb` ships no
preload shim and its output has no `PT_INTERP`, so interposition cannot reach
it (`docs/AGENTS.md` §14 already refuses that route). What transfers is the
**defect class**: a lookup that ANSWERS when it should DEFER. This tree has
now paid for it four times — the weak `iconv_open` provider entry that held
NULL, `R_X86_64_DTPOFF64` answering offset 0 (both T-068), upstream's 358
zero-returning stubs, and this.

### ✅ CLOSED 2026-09-03 — the Prove, run

    sh experiments/94-own-symbol-order.sh
    pass=16 fail=0 skip=0     VERDICT: matched expectation

| arm | fixed loader | reversal (pre-T-073) |
|---|---|---|
| A `_dl_mcount_wrapper_check`, a loaded object defines it | `provider_calls=1` | `provider_calls=1` |
| B the same, nothing else defines it | `standin_fire=1` | `standin_fire=1` |
| C ⭐ `__tls_get_addr`, a loaded object defines it | ⭐ **`decoy_calls=0`** | ⛔ **`decoy_calls=2`** |
| D the same, nothing else defines it | `tls=0x5eeded` | `tls=0x5eeded` |

⭐ **11 of 11 on the bed**, both directions, including all four musl rows —
a glibc `.so` `dlopen`'d on Alpine with its general-dynamic TLS binding to the
compiled-in loader rather than to the decoy sitting in front of it.

### ⛔ THE PART WORTH CARRYING: the round trip passes under the defect

⚠ **`tls=0x5eeded` is correct in BOTH columns.** The decoy returns its own slab
to every caller, so a thread-local written and read back through it is
self-consistent and every value assertion is green. **Only the call count
separates them.** An experiment written around the value — which is the obvious
way to write it — would have measured nothing and reported a pass.

⭐ That is why the entry is `done` and not "reviewed": the defect was **live**,
it was **silent**, and the shape of the naive test hides it.

### Is it reachable on a real host?

⚠ **Measured, and the answer is "only through objects that are refused today"**
— which is a reason it had not bitten, not a reason it was safe. Of 2,514
shared objects on the build host, **four** define either name:

    __tls_get_addr             ld-linux-x86-64.so.2 (GLOBAL), libasan.so.8 (WEAK),
                               libtsan.so.2 (WEAK)
    _dl_mcount_wrapper_check   libc.so.6 (GLOBAL)

`libasan`/`libtsan` are refused by `el_refused_class()` as interposers, and
`libc.so.6` is answered by `el_soname_served()` rather than opened. ⛔ **Every
one of those refusals is for an unrelated reason and any of them could change**
— the interposer list is edited whenever a new one is met, and the served-soname
list follows the link. The ordering must not depend on them.

**The fix.** Two tables, consulted at two points of `el_resolve()`:
`el_own_syms_first` before the symbolic self-lookup, the loaded objects and the
dependency closure; `el_own_syms_last` after the generated provider table.
⭐ `_dl_mcount_wrapper_check` now also yields to **our own static glibc**, which
the single table shadowed — it is not in `libc.a` today, so nothing moved, and
if a pin move ever puts it there the real one wins instead of the no-op.

**The control.** `PGB_T073_OWNSYMS_UNORDERED=1` merges the two tables back into
`el_provider()`, reproducing the pre-T-073 shape exactly. ⭐ It is a **builder**
knob, not a runtime one — `internal/wrapper` passes `-D` and names the object
`pgb-elfload-unordered.o`, so a shipped binary carries no way to reach it and
the mtime cache cannot serve the wrong object. Same shape as
`SharedWrappers` for `experiments/87-`.

**No regression.** `experiments/93-` re-run after the change:
`ok=882 refused=122 failed=478 crash=45 hang=0` — identical to the pre-change
baseline in every column. `experiments/76-` `pass=4 fail=0`.

## T-076 — ⭐ the TENTH quirk, FOUND AND CLOSED THE SAME DAY: the timezone database

**Source** ⭐ **found 2026-09-03c** by taking the operator's *"fix all remaining
GLIBC quirks if there still are some"* as a question about **completeness**
rather than about the eight that are closed.
**Category** runtime · **Priority** P1 · **Effort** M · **Status** ✅ done

⛔ **`docs/REQUIREMENTS.md` said of its nine issues: *"there is no unenumerated
remainder."* That was false, and it was the sentence that made part 2 of the
operator's bar countable.** `grep -rn zoneinfo` over `docs/`, `TODO/`,
`experiments/`, `poc/`, `internal/` and `tool/` returned **nothing**. Nobody
had looked.

**Measured** — `experiments/97-timezone.sh`, **pass=10 fail=0 skip=0**,
`evidence/97-timezone/RESULT.txt`:

| | |
|---|---|
| static `libc.a` | names `/usr/share/zoneinfo`, `/etc/localtime` and honours `TZDIR`, and carries **no data** |
| resolve `Europe/Berlin` correctly | **7 of 11** — `CEST +0200`, hour 02 |
| ⛔ cannot, and do not say so | **4 of 11** — alpine 3.10, 3.20, 3.22 and ⚠ **ubuntu-20.04, which is glibc** |

⛔ **The failure mode is worse than "it returns UTC".** With no database glibc
re-reads `TZ=Europe/Berlin` as a POSIX zone specification — a bare abbreviation
with no offset — and prints:

    Europe +0000 00

⭐ **the zone name the caller ASKED FOR, with a UTC offset.** So `%Z`, the field
that looks like a confirmation, is an echo of the input, and the only field
carrying the defect is the offset. A log line reading `Europe` beside a
timestamp two hours out is the production shape of this bug.

⚠ **And it is not a musl story.** Three of the four are Alpine; the fourth is a
Debian-family image that simply does not install `tzdata`.

**What is left.**

1. ⭐ **The fix has a precedent and it is the same one twice over.** `terminfo`
   and the CA bundle are both host databases that some environments lack, and
   both were closed by an **opt-in `--embed-*`**. `--embed-tzdata` is the
   third of that family. ⚠ Unlike those two, glibc offers a documented hook —
   `TZDIR` — which may make it cheaper; ⛔ that is a guess and the mechanism
   has not been chosen.
2. ⚠ **Decide what "correct" means when the zone is unknown.** Silently
   answering with a UTC offset is the defect; refusing is a behaviour change
   for programs that do not care about time zones. The `--embed-*` family's
   answer — opt in, and be exact when you do — is the likely one.
3. ⛔ **Size.** A full `tzdata` is ~1,800 files and a few hundred KB
   compiled; embedding all of it is not obviously right for a program that
   uses one zone.

⭐ **AND THE METHOD MATTERS MORE THAN THE ROW.** One grep found a tenth issue
in a list called complete. The next candidates, each worth one measurement:
`/etc/services` and `/etc/protocols` for `getservbyname`; `libgcc_s.so.1` for
`pthread_cancel` and `backtrace` — ⚠ **probed the same day: 0 mentions in the
build host's `libc.a` at glibc 2.39**, so likely already dead upstream, but
**not measured on the pinned 2.41**.

**Prove.** All eleven resolve `Europe/Berlin` to `CEST +0200`, with the same
binary, and `experiments/97-` asserts `MISSING = 0`.

## ⭐ CLOSED 2026-09-03c — `--embed-tzdata`, 11 of 11

⛔ **Found and fixed in one session, which is worth stating because the entry
above was written before the fix existed and reads as though it were open.**

`experiments/97-timezone.sh` now runs **two arms**, **pass=13 fail=0 skip=0**:

| | arm A — plain `cc -static` | arm B — `pgb build --embed-tzdata` |
|---|---|---|
| resolve `Europe/Berlin` | **7 of 11** | ⭐ **11 of 11**, `CEST +0200` |
| cannot, and do not say so | ⛔ **4 of 11** | ⭐ **0** |
| artefact | 960,584 B | 1,153,792 B |
| ⭐ cost of the carried zones | | **193,208 B** for 20 zones |

⭐ **The control that stops arm B being vacuous is asserted, not assumed**:
*"arm A had rows that FAILED, so arm B is not vacuous"*. Without it a run where
every host happened to carry tzdata would look identical to a working fix.

**The mechanism, and it is the terminfo one because glibc offers the same hook.**
`tool/runtime/pgb-tzdata.c` is a constructor that (1) leaves a caller's `TZDIR`
alone, (2) ⭐ **returns early when the host has a zone database at all — the
host always wins**, and (3) otherwise unpacks the carried zones under `$TMPDIR`
and points `TZDIR` at them with `setenv`'s overwrite flag 0. That is tier 2 of
the preference order — an automatic toolchain change — and it touches no
application source.

⚠ **A HANDFUL, NOT A DATABASE, and the entry says so rather than implying
otherwise.** `cfg.DefaultTzdataZones` carries **20** zones; tzdata is ~1,800
files and carrying all of them would multiply a 2 MB binary.
`PGB_TZDATA_ZONES` overrides the set at build time. ⛔ **A zone that is not
carried behaves exactly as it did before**, which is the honest floor: this
closes the case it carries and no other.

⚠ **AND IT WRITES TO `$TMPDIR`**, which is a real cost worth naming: the
project's shape claim is *"one ordinary ELF that mounts nothing and writes
nothing"*, and this option, like `--embed-terminfo` before it, writes files at
startup on hosts that need it. It is opt-in for that reason.

⭐ **A carried selftest caught the change while it was being made**, which is
what they are for: `cfg`'s *"every variable `Export()` writes is rendered as a
container `-e` argument"* went red on `PGB_OPT_EMBED_TZDATA` the moment the
option was exported without being added to `cfg.OptVars` — T-019's defect
class, caught in seconds instead of three jobs later. Two more cases pin the
link line in both directions and were proved able to fail by disabling the
branch.

## T-078 — ⭐ the three-way parity matrix, and TWO of its seven predictions were wrong

**Source** ⭐ **operator, 2026-09-03d**: *"a markdown table covering 'vanilla'
GLIBC static binaries vs 'Ours' static binaries vs native MUSL static binaries
must be compared on all possible comparisons that they can be compared with"*,
against the claim *"our static glibc binary and a native musl static binary are
at feature/standalone parity. No buts and no ifs."*
**Category** runtime · **Priority** P1 · **Effort** L · **Status** ✅ done

**Delivered.** The matrix is in [`../../docs/comparison.md`](../../docs/comparison.md),
every cell a measurement or a dash, each naming its experiment.
`experiments/63-` builds ONE probe four ways and runs it on the eleven —
`pass=16 fail=0 skip=0`, **two runs with every cell identical**.

⛔ **`skip=0` is the number the Prove line said to read first.** `60-` and
`61-` skip arms they cannot build, so a missing musl toolchain yields a green
run with an empty column. `musl-gcc` was installed at session start
(`musl-tools` 1.2.4-2) and every arm built.

| axis | vanilla | pgb | musl |
|---|---|---|---|
| rows with a crashed axis | ⛔ 5 / 11 | ✅ 0 | ✅ 0 |
| NSS `getpwuid` / `gethostid` | 10 / 11, SIGFPE on Arch | ✅ 11 / 11 | ✅ 11 / 11 |
| iconv encodings | 1 / 12, crashes on 3 | ✅ 12 / 12 | 10 / 12 |
| locale, environment default | ⛔ 0 / 11 | ⛔ **0 / 11** | ⭐ **11 / 11** |
| locale, when requested | 7 / 11 | ✅ 11 / 11 | 11 / 11 |
| timezone | 7 / 11 | ✅ 11 / 11 | 7 / 11 |
| `/etc/services` | 8 / 11 | ⛔ 8 / 11 | ⛔ 8 / 11 |
| host `.so` loaded | 0 envs | 0 envs | 0 envs |
| artefact size | 1,148,360 B | 2,722,968 B | **237,440 B** |
| malloc, 4 threads | 8.40 ns | 6.25–12.32 ns on all 11 | 606–705 ns |

⭐ **The vanilla arm is built INSIDE the pinned environment**, so it differs
from `pgb` only by the injected mechanisms. A host-built arm is kept as a
control and agrees on every capability axis.

⛔ **TWO PREDICTIONS WERE WRONG AND ARE RECORDED RATHER THAN REWRITTEN.**

- **Q3 falsified on both halves.** With no `LANG` set, native musl answers
  **UTF-8 on 11 of 11** and every glibc arm — `pgb` included — answers
  `ANSI_X3.4-1968` on 11 of 11. musl's minimal locale support does not mean a
  poor codeset: its default charset *is* UTF-8. ⭐ **That is the one axis where
  musl beats us**, which makes Q7 wrong as stated too. The axis was NOT
  softened; the losing row is in the shipped table.
- **Q1 was scored against the wrong number**, by this experiment's own defect:
  the per-axis fork is what makes a crash survivable, so the parent always
  exits 0 and the counter was reading the parent. It reported "crashed = 0" on
  a run whose rows read `nss=SIG8`, `hostid=SIG8`, `iconv=SIG6`.

⚠ **A third defect, caught only because the rule is two runs**: the UTF-8
counter globbed the whole output line, so adding a second axis containing
`UTF-8` moved the glibc arms 0 → 7 between two runs with no change to the
subject. Tokens are extracted by name now; the zeros were true.

⭐ **Verdict on the operator's claim**: level with or ahead of native musl on
every axis except the environment-default codeset, and ahead on iconv, timezone
and throughput. ⛔ *"No buts and no ifs"* is therefore **not yet true** — one
measured "but" (unset-`LANG` codeset) and one measured "if" (`/etc/services`).

## T-079 — ⭐ the list was TEN and it is ELEVEN: `/etc/services`, found by a search

**Source** ⭐ **operator, 2026-09-03d**: *"GLIBC static is truly complete, no
edgecases exist ... No buts and no ifs."*
**Category** runtime · **Priority** P1 · **Effort** M · **Status** ✅ done

⛔ **It is not complete.** `experiments/82-` answers with a **search**, which is
what the entry demanded after the same question was answered wrongly once.

**The method, and it is re-runnable.** Enumerate every absolute path the
**pinned** `libc.a` names — **78** at glibc 2.41 — classify each against the
ten known rows, and print the **19** no row owns. That is the generalisation of
the one `strings` call that found the tenth.

⭐ **THE ELEVENTH ROW: `/etc/services` and `/etc/protocols`.**
`getservbyname("http","tcp")` and `getprotobyname("tcp")` return **NULL on 3 of
11 — debian-11, debian-12 and ubuntu-20.04, and ALL THREE ARE GLIBC**, while
all four musl environments ship the file. ⚠ That is the inverse of the
intuitive direction, which is why it was run rather than reasoned about.

⛔ **Not a restatement of NSS.** NSS is closed for **dispatch** —
`__nss_configure_lookup` pins the `services` database to `files` — and that
cannot conjure a `files` backing store the host does not have. ⭐ The failure is
a **NULL return, not a wrong value**, so it is louder than gconv's and
timezone's: a caller that checks sees it.

⭐ **A second finding, from probing the tail instead of assuming it harmless**:
`gethostid()` dies with **SIGFPE on Arch**. With no `/etc/hostid` — **0 of 11**
have one — glibc falls back to resolving the machine's own hostname, an NSS
`hosts` lookup. ⚠ **That is row 1, not a new row**; what was under-described is
its reach, since nothing here had ever called `gethostid()`.

⛔ **WHERE THE SEARCH CANNOT SEE, because an absence is not a zero**: paths
assembled at run time from `%s/%s` and a variable; host data belonging to
**other** static libraries — ⚠ terminfo (ncurses) and the CA bundle (OpenSSL)
are invisible to it **by construction**; and anything reached through a host
daemon rather than a file.

⚠ **Two defects in the experiment's own instrument**, both found by
disagreement and both now asserted against: presence was tested with
`[ -e "$rootfs$path" ]`, which resolves an **absolute symlink against the
HOST**, so Alpine's `/bin/sh -> /bin/busybox` read as absent on three rows
(`/bin/sh` on 11 of 11 is now a positive control); and the probe buffered
stdout to a pipe, so a crash discarded every answer already computed.

## T-085 — ⭐ the ELEVENTH quirk CLOSED: `--embed-netdb`, and the boundary `--wrap` cannot cross

**Source** ⭐ **operator, 2026-09-04**: *"the ELEVENTH glibc-static quirk
(/etc/services, 3 of 11 fail, all glibc) has a measurement and NO mechanism —
the precedent is --embed-tzdata, look first and carry a fallback."*
**Category** runtime · **Priority** P1 · **Effort** S · **Status** ✅ done

⭐ **Closed on all eleven: `experiments/66-`, `pass=12 fail=0 skip=0`, two runs
byte-identical.** T-079 found the row; this entry is the mechanism.

**The precedent did not transfer, and that is the interesting part.**
`--embed-terminfo`, `--embed-cacert` and `--embed-tzdata` all point a **search
variable** — `TERMINFO`, `SSL_CERT_FILE`, `TZDIR` — at carried data, so the
library does its own lookup and finds ours. ⛔ `/etc/services` has **no such
variable**: the path is compiled into glibc and there is nothing to redirect.
So this mechanism wraps the **call** instead — `-Wl,--wrap` on **eight**
symbols: `getservbyname`, `getservbyport`, `getprotobyname`,
`getprotobynumber`, and the `_r` form of each. ⚠ **Checked against
`internal/wrapper/flags.go` and `tool/runtime/pgb-netdb.c` rather than written
from memory, and the first draft of this paragraph was wrong**: it listed
`getservent` among them, which is not wrapped. The `*ent` iteration family
(`setservent`/`getservent`/`endservent`) is untouched, so a program that walks
the database rather than looking a name up is **not** served.

⭐ **The ORDER is the same order, and it is the security property**, stated in
`AGENTS.md` §7 item 3: each wrapper calls `__real_*` **first**, so a host that
maintains the file wins, and the carried table is a fallback. Two assertions
hold it up rather than a comment: N3 measures that a name **nobody** carries
still answers NULL on 11 of 11 (so the table is not answering blindly), and the
arm A / arm B split measures that the three rows without the file are exactly
the rows the table rescues.

| | arm A `gcc -static` | arm B `--embed-netdb` |
|---|---|---|
| `getservbyname("http","tcp")` | 8 of 11 | **11 of 11** |
| `getprotobyname("tcp")` | 8 of 11 | **11 of 11** |
| a name nobody carries → NULL | 11 of 11 | **11 of 11** |
| artefact | 1,053,656 B | **1,046,376 B** |

⚠ **The size row is not a typo: the tables cost −7,280 B.** The arm with the
embedded databases is *smaller* than the arm without them, so the size argument
against carrying them does not survive contact with the number.

⛔ **THE BOUNDARY, AND IT WAS PRE-REGISTERED AS A FAILURE BEFORE THE RUN.**
`getaddrinfo(NULL, "http", …)` still resolves on only **8 of 11**. `--wrap`
redirects the **public** symbol; glibc's `getaddrinfo` reaches its own internal
`__getservbyname_r`, which the linker cannot interpose. ⚠ It is written into
`experiments/66-`'s header as expectation N4 **precisely so that a green result
would have to be explained rather than pocketed** — an experiment that only
registers the outcomes it wants is an advertisement. A program that needs it
must pass the port number, or the mechanism has to change shape (an
`ld --defsym` alias, or building the tables into a private `libc.a` member).

**Where this was looked for and where it was not.** `libc.a` names
`/etc/services` and `/etc/protocols` — asserted, not assumed, as the first two
checks of the run. What was **not** examined: `/etc/networks`, `/etc/ethers`,
`/etc/rpc`, and the `getnetbyname`/`getrpcbyname` families that read them.
They are the same shape and are untested here.

## T-086 — ⭐ the ONE axis where native musl beat both glibc columns, closed: `--utf8-default`

**Source** ⭐ **operator, 2026-09-04**: *"the environment-default codeset is the
one axis where native musl beats both glibc columns 11-0. COMPLETE THIS TOO."*
**Category** runtime · **Priority** P1 · **Effort** S · **Status** ✅ done

⭐ **Closed on all eleven: `experiments/67-`, `pass=7 fail=0 skip=0`, two runs
byte-identical.**

⛔ **THIS IS A CHANGE TO A DOCUMENTED DEFAULT, NOT A REPAIR, and that is why it
is a separate opt-in flag.** POSIX leaves the choice to the implementation when
`setlocale(cat, "")` is called and the environment says nothing; glibc chooses
`C` (`ANSI_X3.4-1968`) and musl chooses UTF-8, unconditionally. Neither is
wrong. `--utf8-default` makes a `pgb` binary choose what musl chooses.

| arm | what it is | codeset |
|---|---|---|
| A | `gcc -static` | `ANSI_X3.4-1968` on 11 |
| M | native musl static | **UTF-8 on 11** |
| B | `pgb build --embed-locale` | `ANSI_X3.4-1968` on 11 |
| C | `pgb build --utf8-default` | **UTF-8 on 11** |
| E | arm C under `LANG=C` | `ANSI_X3.4-1968` on 11 |

⭐ **ARM E IS THE ROW THAT MAKES ARM C MEAN ANYTHING.** A mechanism that forced
UTF-8 unconditionally would score arm C green and be a **bug**: a program run
under `LANG=C` asked for a single-byte locale and must get one. Arm E is the row
that can fail for that reason, and it is green on 11 of 11. ⛔ The substitution
fires only when `LC_ALL`, `LC_CTYPE` **and** `LANG` are all unset.

⚠ **`--embed-locale` cannot do this and arm B measures that it does not try.**
That mechanism answers a **request** the host could not satisfy; here the host
satisfied the request perfectly — the answer was `C`, which is what was asked
for. Two different failures, two flags. `--utf8-default` implies
`--embed-locale`, enforced in the flag parser and again in `cfg.Load`, because
the substituted `C.UTF-8` has to come from somewhere.

⚠ **ONE AXIS.** `C.UTF-8` is the C locale with a UTF-8 charset, not a full
locale. This closes the environment-default **codeset** and says nothing about
collation, message catalogues, or any other `LC_*` category.
