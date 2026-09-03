# runtime — the four mechanisms, and reaching the plugin class

`tool/runtime/*.c`. Routes: [../docs/AGENTS.md`](../docs/AGENTS.md) §7.

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

## T-031 — Port cross-libc-dlopen's full rewrite, not one function

**Source** `docs/limitations.md` §1 · **Category** runtime · **Priority** P2 · **Effort** L · **Status** open

**Problem.** `experiments/50-` ported `cld_strip_versions()` — one function of
roughly forty from a 2015-line file — and found no effect. The two steps it did
not port are the ones aimed at the failure it observed.

**Premise.** ⚠ The untested steps drop the `DT_NEEDED` edges that pull a
foreign libc in (`cross-libc-dlopen.c:1857`) and rebind the remaining imports.
Upstream's `docs/limits.md` says the static-glibc case is one where `dlopen`
*works* and labels all three static cases unverified.

**Approach.** `CROSS_LIBC_DLOPEN_DRYRUN` makes the rewrite path testable with
no GPU and no Alpine — cheaper than the instrument `50-` built.

**Prove.** `experiments/51-*.sh` re-runs `50-`'s two arms plus a third carrying
the full rewrite, and the table shows what changed on each of 11.

## T-033 — route D: compile an ELF loader in, resolve against our own static glibc

**Source** `docs/research/solo.md`, the `pg83/solo` sweep, session of
2026-09-01b. **Category** runtime · **Priority** P1 · **Effort** L · **Status** open

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
**Category** runtime · **Priority** P1 · **Effort** M · **Status** open

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
