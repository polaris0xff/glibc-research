# Limitations

⛔ **Every item here is measured, and the command that measures it is named.**
Nothing on this page is a precaution or a guess. If a limitation could be
removed, it says how.

---

## 0. A static program cannot host a shared plugin that calls back into it

⛔ **Structural, measured, and prior to everything in §1.** A statically
linked executable has an **empty dynamic symbol table**. A shared object
loaded into it therefore cannot resolve any reference back to the host
program — there is nothing for the loader to look in. `-rdynamic` is the flag
that exports a host's symbols to its plugins, and it works by populating
`.dynsym`; a `-static` link has no `.dynsym` to populate.

⚠ **This is not §1.** §1 is about glibc's loader failing when a static binary
borrows the host's `ld.so`. This is about symbol resolution having nowhere to
look **even if that loader worked perfectly**.

**Reproduce**, one second:

```sh
sh experiments/72-static-host-plugin-abi.sh
```

| arm | `.dynsym` | outcome |
|---|---|---|
| dynamic host, `-rdynamic` (positive control) | 11 | plugin loads and calls back |
| static host | **0** | `dlopen: undefined symbol: host_api_add` |
| static host + `--wrap-dlopen` | 0 | plugin loads and calls back |

**Found by** trying to build CPython 3.12.7 with a static interpreter and
shared stdlib modules — the subject `TODO` T-030's acceptance asked for. It
fails at `checksharedmods` with
`undefined symbol: PyLong_AsLongLongAndOverflow`, and the interpreter defines
that symbol; it is simply not in a table a loader consults.

⭐ **The consequence is the useful part.** For a plugin that calls into its
host, linking it in is **not a workaround for `dlopen`** — it is the only
thing that can work, and `--wrap-dlopen` is what makes the program's existing
`dlopen` calls find it. It also explains why CPython's own
`Modules/Setup.local` has the shape it does: the two routes converge by
necessity.

⭐ **Corroborated independently.** `leleliu008/python-distribution`, a
self-contained relocatable CPython, sets CPython's own
`MODULE_BUILDTYPE=static` with `--enable-static --disable-shared`
(`references/leleliu008__python-distribution/tree/build.sh:1029,1035` @
`987e937a`) — every stdlib module linked into the interpreter. It does not
attempt a static interpreter with shared modules either. ⚠ Agreement from an
independent implementation is not proof; the proof is `experiments/72-`.

⚠ **A plugin that does NOT call back** — one reached only through a vtable
handed to it, or one that only exports data — is not affected by this, and
`experiments/50-`'s findings are what govern there instead.

## 1. `dlopen` of a host shared object is host-dependent, and success is worse than failure

**The measurement.** POC 10 builds gawk with its extension API enabled and
points it at `filefuncs.so` — gawk's *own* extension, produced by that same
build, correct for that gawk version — placed on disk beside the binary.

| outcome | environments |
|---|---|
| **LOADED** | Debian 12, Arch Linux |
| refused | Ubuntu 20.04, Rocky 8, openSUSE Leap 15.6, Fedora 42, Debian 11, Alpine ×3, Void musl |

⚠ **The two that "work" are the problem.** On those, the trace shows the host's
`ld-linux-x86-64.so.2` and `libc.so.6` mapped into the process alongside the
statically linked glibc. That is the two-libc state every other mechanism here
exists to prevent; it simply has not crashed yet.

**Reproduce:** `sh poc/10-gawk/run.sh`, then read
`evidence/poc/10-gawk/observation.txt`.

### Does cross-libc-dlopen's rewrite fix this? One function of it does not.

`experiments/50-host-plugin-feasibility.sh` ports `cld_strip_versions()`
(`cross-libc-dlopen.c:811-817` @ `1cecf50e`) into a static `pgb` binary and
runs two arms per environment, **each in its own child** so a crash in one
still leaves the other measurable.

| environment | arm A plain `dlopen` | arm B version-stripped copy |
|---|---|---|
| Alpine 3.10/3.20/3.22, Void musl | handle returned | handle returned |
| Debian 12, Arch | handle returned | handle returned |
| Debian 11, Ubuntu 20.04 | **SIGABRT** | **SIGABRT, identical** |
| Rocky 8 | **SIGABRT** | **SIGABRT, identical** |
| openSUSE Leap, Fedora 42 | **SIGFPE** | **SIGFPE, identical** |

⛔ **Arm B changes nothing anywhere.** No environment produced a symbol-version
error. The process dies *inside glibc's loader*, before `dlerror()` is ever
set:

```
dl-call-libc-early-init.c:37: _dl_call_libc_early_init:
    Assertion `sym != NULL' failed.              Debian 11, Ubuntu 20.04
dl-machine.h:487: elf_machine_rela_relative:
    Assertion `R_X86_64_RELATIVE' failed.        Rocky 8
signal 8 (SIGFPE) in the loader                  openSUSE Leap, Fedora 42
```

⭐ **The diagnosis.** A static glibc binary has no loader of its own, so
`dlopen` borrows the **host's** `ld.so` and `libc.so.6` — the traces in §1
above show both being opened. What breaks is that pairing: our statically
linked glibc 2.36 driving a host loader and host libc of a different version.
`_dl_call_libc_early_init` looking for `__libc_early_init` in a host libc that
does not present it the expected way is that mismatch exactly. Symbol
versioning is not involved, so neutralising version tags cannot help.

⚠ **Do not read the "handle returned" rows as success.** The probe checks that
`dlopen` returned non-NULL. It does **not** call anything in the object, and on
the musl rows the object it loaded was musl-linked. Whether those libraries are
*usable* is untested, and the two-libc hazard in §1 applies regardless.

⛔ **And version-stripping is now measured to be worse than a no-op where it
bites.** `experiments/73-`'s second control rebuilds the very object named in a
reference's `DT_VERNEED` without its version definitions, and glibc's loader
does not fall back — it **asserts**:

```
no version information available (required by ./main)
Inconsistency detected by ld.so: dl-lookup.c: 106: check_match:
    Assertion `version->filename == NULL || ! _dl_name_match_p (version->filename, map)' failed!
```

⭐ **glibc treats a named provider that lost its versions as a bug in that
library, not as a compatibility case.** The compatibility rule is real, and it
applies only when the definition comes from a *different* object than the one
the verneed entry names — which 73-'s first control confirms, and which is
exactly where a compiled-in provider table sits. That distinction is what makes
§1's route D viable and 50-'s arm B a dead end.

### ⭐ Route D: do not use the host loader at all

`docs/research/solo.md`, `TODO` T-033. The diagnosis above says the failure is
*the pairing* — our static glibc driving a host `ld.so` and a host `libc.so.6`.
⭐ **Then do not borrow either.** Map the object with a loader compiled into the
binary and bind its imports to the executable's own symbols. `pg83/solo` does
this on musl; `experiments/73-` measures what it would cost here:

```
  5,807 host shared objects, the seven glibc environments
  90.8% - 97.8% of every GLIBC_/GCC_-versioned import already definable
  by the pinned static glibc.  Unexplained residue: 0.
```

⚠ **That is symbol availability, not a working `dlopen`.** T-033 names the
unknowns — the mapper is 2,707 lines in solo and does not get cheaper for being
glibc, and TLS is the one place where "we are glibc, so it is simpler" is not
obviously true.

⚠ **What arm B does NOT establish.** It ported `cld_strip_versions()` — 7 lines, one function out of
roughly forty in a 2015-line file. The rewrite it comes from
(`cross-libc-dlopen.c:1857`) is three coordinated steps, and the two that were
**not** ported are the ones aimed at the failure above: dropping the
`DT_NEEDED` edges that pull a foreign libc in, and renaming the imports that
are left. ⛔ So what is measured is that *one* function of forty has no effect on its
own — not that the approach fails.

⭐ Upstream's own `docs/limits.md` scopes the question further: for **static glibc** it says `dlopen` *works*,
and that "the real blocker is more likely the preload path than `dlopen`",
because a fully static binary has no `LD_PRELOAD` for the interposer to arrive
through. It labels all three static cases **UNVERIFIED** upstream. It also
ships `CROSS_LIBC_DLOPEN_DRYRUN`, a cheaper instrument than the one
`experiments/50-` built. `docs/research/prior-art.md` has the detail.

**So what would work?** The diagnosis still names a requirement: the process
must carry **its own loader and its own libc**, which is precisely what
cross-libc-dlopen assumes — it is an `LD_PRELOAD` for a process that already
has both. That is a bundled-glibc **dynamic** binary, not a static one. ⚠ But
whether the full rewrite would also work *without* that, in the static case,
is now recorded as untested rather than as settled.

⚠ **Carrying a loader is a second output mode, not a patch to this one**, and
it costs the property that makes the current one worth having: a single normal
ELF with no interpreter. It is the most expensive of the three routes in
`docs/AGENTS.md` §13 item 4, which is why it is listed last.

**The class served today is: programs that do not need to load host plugins.**
A program whose core function is loading them — a browser using system codecs,
a desktop application needing the host's GPU driver — is served right now by
one of the bundling approaches in `docs/comparison.md`, and is the target of
that §13 item.

⭐ **A program that loads its OWN plugins is a different case and can be
served**: build them into the binary. POC 50 does exactly that, turning 49
CPython extension modules from `dlopen`ed `.so` files into linked-in code, with
`lib-dynload` left empty.

---

## 2. NSS data beyond `files` and `dns` is gone

`__nss_configure_lookup` pins every database to services glibc implements
internally. That means **no LDAP, no SSSD, no NIS, no mDNS, no
systemd-resolved, no `nss-mymachines`**. Users and hosts come from
`/etc/passwd`, `/etc/group`, `/etc/hosts` and DNS.

⚠ **This is the fix, not a side effect.** Those modules are exactly what
crashes the binary on Arch and openSUSE. But it is a real behaviour change: a
program that must resolve enterprise directory identities cannot use this
approach, and no flag changes that.

**Reproduce:** `sh experiments/20-static-glibc-nss-dlopen.sh`.

### 2a. The one behaviour a user is likely to notice: the machine's own hostname

`nss-myhostname` is what makes a systemd distribution resolve its own
`gethostname()` result without an `/etc/hosts` entry. Dropping it removes that.

Measured, resolving the host's own name (`vm` here):

| environment | plain `-static` | with `pgb` |
|---|---|---|
| Fedora 42 | **resolves** (via `libnss_myhostname`) | ⛔ `Name or service not known` |
| Arch Linux | crashes (SIGFPE) | `Name or service not known` |
| Debian 12 | `Name or service not known` | `Name or service not known` |
| Alpine 3.22 | `Name or service not known` | `Name or service not known` |

⭐ **This is a genuine cost of the fix, and the Fedora row is the honest
statement of it**: on a distribution that ships `myhostname`, the portable
binary loses a lookup the plain one had. On the others there is no difference,
because they never provided it either. A program that must resolve its own
hostname should add it to `/etc/hosts` or use `gethostname()` directly.

### 2b. Not a pgb limitation, but it looks like one: `localhost`

Debian 11, Debian 12 and Ubuntu 20.04 ship a root filesystem whose
`/etc/hosts` contains **no localhost line at all** — Docker injects one when it
starts a container, and a plain unpacked image has none. `getaddrinfo("localhost")`
therefore fails there with `EAI_NONAME`.

⛔ **Do not attribute that to the NSS override.** A plain `gcc -static` binary
was measured failing identically on the same three images. It is a property of
those images, and the POCs that need a resolvable name now create one rather
than assuming `localhost`.

⚠ **`curl` hides this**, which is worth knowing when reading POC 30: curl
special-cases `localhost` per RFC 6761 and answers it without consulting a
resolver at all, so a curl test using `localhost` measures curl, not NSS. POC
30's NSS check uses a name it puts in `/etc/hosts` itself.

---

## 3. Data dependencies are not libc dependencies, and static linking does not touch them

Five distinct ones found, each with its own per-distribution path convention.
⭐ **This is the finding most likely to be underestimated**: three of the five
have nothing to do with glibc at all, and static linking is silent about all of
them.

| dependency | status | evidence |
|---|---|---|
| **gconv** (character encodings) | ✅ **solved** — static GNU libiconv behind `-Wl,--wrap`, no data directory | `experiments/30-` |
| **glibc locale** (`/usr/lib/locale`) | ✅ **solved, opt-in** — `--embed-locale`, materialised only when the host cannot answer | `experiments/30-` |
| **terminfo** (`/usr/share/terminfo`) | ⛔ **unsolved** | POC 20 |
| **TLS CA bundle** | ⛔ **unsolved** | POC 30 |
| **a runtime's own library tree** (CPython's stdlib) | ⚠ **shipped, not solved** — 98 MiB beside a 46 MiB binary | POC 50 |

**terminfo, measured** with a `setupterm()` probe linked against the same
static ncursesw:

| result | environments |
|---|---|
| `OK` | all 7 glibc |
| `rc=-1 err=-1` — no terminfo database on the host at all | Alpine 3.10, 3.20, 3.22 |
| `rc=-1 err=0` — database present, no `xterm-256color` entry | Void musl |

**CA bundle, measured** with the harness's own CA variables unset, since curl
compiles exactly one path in:

| result | environments |
|---|---|
| verified | Alpine 3.20, Alpine 3.22, Void musl, Fedora 42, Arch — 5 of 11 |
| unverified | Debian 11, Debian 12, Ubuntu 20.04 (no bundle at all in the minimal image); Rocky 8 (`/etc/pki/tls/certs/ca-bundle.crt`); openSUSE Leap (`/etc/ssl/ca-bundle.pem`); Alpine 3.10 (`/etc/ssl/cert.pem`) |

⚠ **Bundling a libc does not solve gconv either, and that is measured rather
than argued.** `experiments/60-` ran onelf — a bundled glibc with its own
loader in one file — across the same 11. The bundling works: no host object on
any of them. It still fails the encoding assertions on **8 of 11**, and the 3
it passes it passes by reaching the *host's* gconv modules. The `--wrap` onto
static libiconv is what solves gconv, in any tier. `docs/design/tiers.md`.

**Can terminfo and the CA bundle be fixed?** Yes, by the same mechanism as
`--embed-locale`: both are found through an environment variable (`TERMINFO`,
`CURL_CA_BUNDLE`/`SSL_CERT_FILE`), so embedding and materialising works
identically. Neither is implemented. ⚠ Both are **application** data rather
than libc data, which is why they are not in the tool by default: the argument
for a glibc portability tool owning a terminal database is weak.

---

## 4. `-static` resolves what dynamic linking defers, so an incompletely-static dependency fails the link

CPython's `nis` module links `libtirpc`, whose `svc_auth_gss.o` references
GSSAPI symbols. Dynamically that never matters — the reference is deferred to a
library that is never loaded. Statically it is resolved at link time and the
symbols are simply absent:

```
libtirpc.a(libtirpc_la-svc_auth_gss.o): undefined reference to `gss_import_name'
make: *** [Programs/_freeze_module] Error 1
```

⚠ **The error lands on a BUILD-TIME tool**, so it reads as a broken toolchain
rather than as one unwanted optional module. This is a general property of
static linking, not a defect in CPython or in `pgb`. The remedy is to drop the
optional module (`py_cv_module_nis=n/a`) or to supply its dependency fully.

---

## 5. A private-prefix dependency build can bake the build prefix into runtime search paths

ncurses compiles its terminfo **search path** in at configure time, derived
from `--prefix`. Built into a private prefix without `--with-terminfo-dirs`,
the resulting binary looks for terminal descriptions under a path that exists
on the build machine and on no target.

⛔ **Measured: `setupterm()` failed on all 11 environments**, including the
seven with a perfectly good `/usr/share/terminfo` — and nano still passed a
`--version` functional test, so the binary would have shipped and failed on
first use. Fixed in POC 20 with explicit `--with-terminfo-dirs`.

⚠ **Anything with a compiled-in path can do this**, not only ncurses. When
building a dependency into a private prefix, check what paths it recorded.

---

## 6. The kernel is not abstracted, and the test bed shares one

`scripts/common/rootfs-run.sh` is `unshare --mount` plus `chroot`. The target
distribution's userland is the only one visible, which is what makes the
results meaningful — but **the kernel is the host's**, Linux 6.18.44 for every
number here.

So this bed **can** falsify "runs on musl" and **cannot** test:

- behaviour depending on the target's kernel version or configuration;
- the minimum kernel the build glibc requires;
- anything needing a real init, systemd, or a PID namespace.

⚠ It is also **not a security boundary**: PID, network, user and IPC
namespaces are shared unless `--private-net` is passed.

---

## 7. Coverage limits

| | |
|---|---|
| **the program has to be statically linkable today** | ⛔ **The open problem, and not one any matrix here measures — it is a property of the software being packaged.** `Anylinux-AppImages`' own guidance puts it: *"Compile statically! Sure, that works, go and compile all of kdenlive statically and get back to me once you get it done."* A large dynamic dependency graph — desktop toolkits, GPU stacks, anything loading host plugins — is served by bundling every library today. ⭐ The route is to push each dependency up the brief's preference order until only the irreducible remainder is left: `docs/design/toolchain.md`, and `docs/AGENTS.md` §13 item 4. |
| **startup and size, against musl** | ⚠ A static **musl** binary starts about 6× faster (160 µs vs 980 µs per exec) and ships 447 KB against 2.1 MB — real advantages for short-lived processes. ⛔ They are not a reason to prefer musl generally: at steady state glibc is 3–129× faster on the same workloads (`experiments/61-`), which is why the brief asks for glibc. |
| **architecture** | x86_64 only. aarch64 is **untested** — `docs/AGENTS.md` §9 and §13 item 3. |
| **machines** | one. Every result is one machine, one kernel, one day. |
| **glibc floor** | the build image is pinned at glibc 2.36 because `files`/`dns` became builtin in 2.34. ✅ **Measured**, not reasoned: `experiments/21-glibc-version-floor.sh` builds the same source at 2.31 and at 2.36 against the same target, and below the floor the override **moves** the `dlopen` rather than removing it — `libnss_files.so.2` and `libnss_dns.so.2` are opened with and without it. `docs/history/corrections.md` C6 has the table. ⚠ This row said "reasoned, not measured — planned and unwritten" for most of the project's life and was left stale after the experiment landed. |
| **container engines** | `pgb --engine docker` and `--engine podman` are **written and never run**: this machine has no daemon. |
| **overhead** | measured (`experiments/40-overhead.sh`): **no measurable startup or memory difference** from plain `gcc -static` — two runs disagreed on the sign of the RSS delta, so both sit at the instrument's noise floor. ⛔ Do not quote a figure for either. Binary size is real: static libiconv roughly doubles a small binary, and only for programs that call `iconv`. |
