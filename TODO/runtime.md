# runtime — the four mechanisms, and reaching the plugin class

`tool/runtime/*.c`. Routes: [`../docs/AGENTS.md`](../docs/AGENTS.md) §13 item 4.

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

**Approach.** Cheapest of the three routes. Generate the table from the plugins
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
  5,807 host shared objects, the seven glibc environments
  90.8% - 97.8% of every GLIBC_/GCC_-versioned import already definable
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

⭐ **Relation to the other three routes**, `docs/AGENTS.md` §13 item 4:
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
**Category** runtime · **Priority** P0 · **Effort** XL · **Status** open

⛔ **WORK UNTIL IT IS MET OR THE PREMISE IS SIGNIFICANTLY ADVANCED.** This is
not a spike. `docs/limitations.md` §1 is the project's one measured, unfixed
failure and the reason `REQUIREMENTS.md` part 1 is not met.

**Problem.** A static glibc binary cannot `dlopen` a host shared object. Where
it succeeds — Debian 12 and Arch, measured in `poc/10-gawk` — the success is
the *worse* outcome: the host loader and a second libc enter the process.

**Premise, and it is already measured rather than hoped for.**

- `experiments/73-` parsed **5,807 real host shared objects** across the seven
  glibc environments: **90.8%–97.8%** of every `GLIBC_`/`GCC_`-versioned import
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
