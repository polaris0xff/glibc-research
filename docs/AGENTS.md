# AGENTS.md — the standalone handoff for this repository

⭐ **You can read only this file and be able to both check the existing work
and continue it.** It assumes no conversation history and no memory of a
previous session. Everything it claims is backed by a script in this tree that
you can re-run.

⚠ **Read [§20 Known-weak claims](#20-known-weak-claims-read-these-before-the-conclusions)
before you read any conclusion.** A previous revision of this document would
have had four claims wrong; that section is where the current ones are kept
honest.

---

## 1. What this project is

A **research project with a working tool** that answers one question:

> Can a normal Linux ELF binary be built with GLIBC such that it runs
> reliably on both GLIBC and MUSL systems, without a separate packaging
> format and without significant runtime overhead?

The answer this repository establishes, with evidence, is in [§6](#6-the-answer-so-far).

The tool is [`pgb`](../pgb) (**p**ortable **g**libc **b**uild). It produces an
ordinary statically linked ELF executable — no launcher, no AppDir, no
loader, no directory beside it.

## 2. The exact problem

`gcc -static` against glibc does **not** produce a self-contained binary.
`file` says "statically linked" and `ldd` says "not a dynamic executable", and
both are misleading. Measured across 11 pinned distributions
([§13](#13-test-environments)):

| what the binary still reaches for | what happens | measured in |
|---|---|---|
| host `/etc/nsswitch.conf` and the `libnss_*.so` it names | loaded on 5 of 11; **SIGFPE on Arch Linux and openSUSE Leap 15.6** | `experiments/20-` |
| host gconv modules for character conversion | **SIGFPE/SIGABRT on Debian 11, Debian 12, Ubuntu 20.04**; 11 of 12 encodings silently unavailable everywhere else | `experiments/30-` |
| host glibc locale data | `setlocale(C.UTF-8)` → NULL and codeset → `ANSI_X3.4-1968` on all 4 musl hosts | `experiments/30-` |

Each of those is a **`dlopen` of a host shared object from inside a "static"
process**, and each host object carries `DT_NEEDED libc.so.6`, so a second
libc and the dynamic loader enter the process.

## 3. Why the problem exists

glibc's NSS and gconv are **plugin architectures**. The service to use is named
in host configuration at run time and loaded with `dlopen`. Static linking
links the *dispatcher*, never the *plugins*, and cannot: which plugin is wanted
is not known until the program runs on a particular host.

glibc 2.34 built the `files` and `dns` NSS services into libc. That removed the
*default* dlopen and nothing else: `resolve`, `myhostname`, `mymachines`,
`mdns4_minimal`, `compat`, `sss` and `systemd` are all still external, and
modern distributions name them by default.

## 4. What "portable static GLIBC" means here

A binary qualifies when **all** of these hold, on every environment in
[§13](#13-test-environments):

1. it runs, exit status is the program's own, no signal;
2. it loads **no host shared object** — checked by syscall trace attributed to
   its own pid, not by `ldd`;
3. its **functionality** works, exercised by real assertions, not `--version`.

⚠ **Criterion 2 is about SHARED OBJECTS, not about host data, and an earlier
version of this file had that wrong.** It said "no host shared object *and no
host gconv/NSS data*", which is both unachievable and undesirable:

- glibc still **opens** `/etc/nsswitch.conf` under the NSS override. It does
  not *use* what the file names, which is the property that matters.
- a program that finds and honours the host's locale where the host has one is
  behaving **correctly**. CPython reads Debian's `C.utf8` tree and is right to.

⭐ **The property is INDEPENDENCE, not abstinence: the binary must work whether
or not the host data exists.** The matrix demonstrates that directly, because
the four musl environments have no glibc locale data, no gconv tree and (on
Alpine) no terminfo, and the same binaries pass there. Host data reads are
therefore *reported* in their own column and never asserted; a host `.so` load
is a failure.

⛔ **`file`, `ldd` and `readelf` are not the criterion.** A binary that
satisfies all three and dies on Arch has failed; a binary those tools call
"dynamic" that works everywhere has passed.

## 5. What does NOT count as success

- passing on one glibc and one musl distribution and calling it universal;
- a `--version` check standing in for a functional test;
- an empty strace with no positive control proving the probe can see anything;
- a build whose toolchain came from the developer's own machine.

## 6. The answer so far

**Yes, for a well-defined and large class of programs**, and the class is
defined by one property: *the program does not need to load host plugins.*

Five real projects ([§12](#12-proof-of-concept-projects)) build unmodified and
pass full functional tests on all 11 environments, loading zero host objects.

**The limits are real and are not hidden** — [§11](#11-known-limitations).
The sharpest one: `dlopen` of a host object from a static glibc binary
**sometimes works**, which is worse than never working. Measured: gawk's own
extension loads on Debian 12 and Arch, is refused on the other nine.

## 7. Architecture

Three mechanisms, all at tier 2–3 of the brief's preference order (automatic
toolchain change / generic runtime technique). **No application source is
patched by any of them.**

| mechanism | how | file |
|---|---|---|
| **NSS** | a constructor calls `__nss_configure_lookup()` — a public `GLIBC_2.2.5` symbol present in `libc.a` — pinning every database to services glibc ≥ 2.34 implements *inside* libc. Nothing named in the host's nsswitch.conf can then be dlopen'd. | `tool/runtime/pgb-nssfix.c` |
| **iconv** | `-Wl,--wrap=iconv_open,--wrap=iconv,--wrap=iconv_close` redirects the three public entry points to statically linked GNU libiconv. `--wrap` acts at the final link, so it catches calls from any object including archives built before this tool existed. | `tool/runtime/pgb-iconv.c` |
| **locale** (opt-in) | `-Wl,--wrap=setlocale`; the C.UTF-8 tree is embedded and materialised **only** if the host cannot answer a UTF-8 request. | `tool/runtime/pgb-locale.c` |

⭐ **Why the iconv shim lives in an archive and nssfix does not.** An archive
member is pulled in only when a symbol it defines is referenced. Nothing
references `__wrap_iconv_open` unless the program calls `iconv_open`, so a
program that does no conversion links none of libiconv. Measured: 940 KiB
versus 2.1 MiB for the same source. The nssfix constructor has no referenced
symbol at all, so it is passed as a plain object with `-Wl,-u,pgb_runtime_anchor`.

**Delivery is compiler wrappers on `PATH`** plus `CC`/`CXX`. autotools, CMake,
meson and plain make all pick them up with no knowledge of `pgb`. Each wrapper
inspects its own argv: `-c`/`-E`/`-S`/`-M` is a compile, `-shared` is passed
through untouched (a `./configure` shared-library probe must keep working), and
anything else is an executable link that gets the portable flags.

## 8. Repository structure

```
pgb                       the tool. POSIX sh. Start at its header comment
tool/runtime/*.c          the three runtime mechanisms
scripts/common/
  oci-pull.sh             pull an OCI image to a rootfs with no daemon
  rootfs-run.sh           chroot into one, private mount namespace
  fetch-rootfs.sh         materialise the pinned test bed
  rootfs-images.txt       ⭐ the 11 environments, pinned by manifest digest
  mine-repo.sh            the reference-sweep fetcher (vendored, see §17)
scripts/build-libiconv.sh GNU libiconv, pinned
experiments/              numbered, re-runnable, each answers one question
  lib.sh                  conditions block, assertions, pid-attributed tracing
poc/                      the five proof-of-concept projects
  common.sh               the POC contract: build, inspect, matrix, observe
evidence/                 RESULT.txt per experiment and POC, committed
references/               ⭐ the corpus: 12 upstream trees + trackers, tracked
docs/                     this file and the write-ups
tmp/START.md              the original brief this project answers
```

## 9. How to run everything

```sh
# 0. prerequisites: root + CAP_SYS_ADMIN (for chroot), curl, python3, a C
#    toolchain, strace. `sh pgb doctor` tells you what is missing.
sh pgb doctor

# 1. the test bed, ~1.5 GiB, pinned by digest
sh scripts/common/fetch-rootfs.sh
sh scripts/common/fetch-rootfs.sh --list      # what is pinned vs on disk

# 2. the experiments, in order. Exit 0 matched expectation, 1 did not,
#    2 could not run.
sh experiments/10-probe-the-host.sh
sh experiments/20-static-glibc-nss-dlopen.sh
sh experiments/30-gconv-and-locale.sh

# 3. the build environment (pinned Debian 12, glibc 2.36) and libiconv
sh pgb env create
sh pgb env info

# 4. build something and check it
sh pgb build -- make
sh pgb verify ./your-binary

# 5. the POCs. Each fetches, verifies a sha256, builds, and runs the matrix.
for p in poc/*/run.sh; do sh "$p"; done
```

`sh pgb explain` prints every injected flag and the experiment behind it.

## 10. Status of every piece

| item | status | note |
|---|---|---|
| chroot/OCI test bed, 11 environments | **COMPLETE** | x86_64 only |
| `oci-pull.sh`, `rootfs-run.sh` selftests | **COMPLETE** | both carry positive controls |
| experiment 10 (host probe) | **COMPLETE** | |
| experiment 20 (NSS) | **COMPLETE** | 37 assertions |
| experiment 30 (gconv + locale) | **COMPLETE** | 24 assertions |
| `pgb`, chroot engine | **COMPLETE** | the engine everything was measured with |
| `pgb`, host engine | **COMPLETE** | works; warns that it is uncontrolled |
| `pgb`, docker/podman engine | **UNTESTED** | no daemon on the development machine. Code exists, has never run. |
| NSS mechanism | **COMPLETE** | zero host NSS modules on 11 of 11 |
| iconv mechanism | **COMPLETE** | 12 of 12 encodings on 11 of 11 |
| locale mechanism (`--embed-locale`) | **COMPLETE** | UTF-8 on 11 of 11 |
| POC 10 gawk | **COMPLETE** | |
| POC 20 nano (+ncurses) | **COMPLETE** | |
| POC 30 curl (+OpenSSL, zlib) | **COMPLETE** | |
| POC 40 jq (+oniguruma) | **COMPLETE** | |
| POC 50 CPython | **IN PROGRESS** | see §21 |
| host `dlopen` of plugins | **FAILED / KNOWN LIMITATION** | §11, and it is host-dependent |
| terminfo, CA bundle | **KNOWN LIMITATION** | data, not libc. §11 |
| aarch64 | **UNTESTED** | §14 |
| glibc version floor (< 2.34) | **UNTESTED** | §21, experiment 21 is planned not written |
| performance / overhead | **UNTESTED** | §21 |
| CI | **PLANNED** | §21 |

## 11. Known limitations

⛔ **These are measured, not guessed, and they are the honest cost of the
approach.**

1. **`dlopen` of a host shared object is host-dependent, and success is the
   worse outcome.** gawk's own `filefuncs.so`, built by the same build:
   **loads** on Debian 12 and Arch Linux, **refused** on Ubuntu 20.04,
   Rocky 8, openSUSE Leap, Fedora 42, Debian 11 and all four musl hosts. Where
   it loads, the trace shows the host's `ld-linux` and `libc.so.6` entering the
   process. A program whose *core function* is loading host plugins is outside
   the class this tool serves. Evidence: `evidence/poc/10-gawk/observation.txt`.
2. **NSS data from LDAP, SSSD, NIS, mDNS and systemd-resolved is gone.**
   Keeping those modules out *is* the fix; losing them is its price.
3. **Data dependencies are not libc dependencies and are not solved by static
   linking.** Five found so far, each with its own path convention:
   gconv (solved by static libiconv), locale (solved, opt-in), **terminfo**
   (unsolved: no database on Alpine, wrong entry on Void), **CA bundles**
   (unsolved: one compiled-in path works on 5 of 11), and a language runtime's
   own library tree (CPython's stdlib, handled by shipping it).
4. **`-static` pulls whole archives**, so an optional dependency that is not
   fully static fails the link. Not a defect in any project: dynamic linking
   defers those symbols to a library never loaded, static linking resolves them
   and they are absent. Seen in CPython's `nis` module via `libtirpc`.
5. **A private-prefix dependency build can bake the build prefix into runtime
   search paths.** ncurses compiles its terminfo search path from `--prefix`;
   without `--with-terminfo-dirs` the binary looked for terminal descriptions
   under the build prefix and `setupterm()` failed on **all 11** — including
   the seven with a perfectly good `/usr/share/terminfo`. It still passed a
   `--version` test.
6. **The kernel is not abstracted.** A chroot bed shares the host kernel, so
   nothing here tests behaviour that depends on the target's kernel version.
7. **Only x86_64 has been run.** §14.

## 12. Proof-of-concept projects

Chosen to stress *different* failure areas, none of them Rust, Go, or C that
trivially links statically.

| # | project | version | stresses | status |
|---|---|---|---|---|
| 10 | GNU awk | 5.3.1 | locale (LC_CTYPE/LC_NUMERIC), iconv, **dlopen extension API** | COMPLETE |
| 20 | GNU nano + ncurses 6.5 | 8.2 | **terminfo data**, static dependency chain, iconv, multibyte | COMPLETE |
| 30 | curl + OpenSSL 3.0.15 + zlib | 8.11.0 | **getaddrinfo/NSS**, real DNS, real TLS, **CA bundle data**, 3-package chain | COMPLETE |
| 40 | jq + oniguruma 6.9.9 | 1.7.1 | Unicode round-trip, surrogate pairs, optional-dependency detection | COMPLETE |
| 50 | CPython | 3.12.7 | **dlopen extension modules**, large data tree, NSS via socket/pwd, locale | IN PROGRESS |

Every one builds from a stock tarball with a stock `./configure`. **No source
patch exists in this repository** — see [§16](#16-patches).

## 13. Test environments

Pinned by manifest digest in `scripts/common/rootfs-images.txt`. ⛔ Re-pulling
a tag without updating that file silently changes what every result describes —
`archlinux:latest` in particular is a rolling tag.

**musl (4):** Alpine 3.22, Alpine 3.20, Alpine 3.10, Void Linux musl
**glibc (7):** Debian 11, Debian 12, Ubuntu 20.04, Rocky 8, openSUSE Leap 15.6,
Fedora 42, Arch Linux

Compatibility is stated only for these. ⚠ **This is not "works on Linux".** It
is eleven filesystems on one kernel on one machine.

## 14. Architecture coverage

| | |
|---|---|
| **x86_64** | every result in this repository |
| **aarch64** | **UNTESTED.** `oci-pull.sh --arch arm64` and `fetch-rootfs.sh --arch arm64` exist and re-resolve by tag, which trades the digest pin away and says so. Nothing has been run. |

The CPU baseline is `-march=x86-64` (or `armv8-a`), never `-march=native`.

## 15. Vendored components

| what | upstream | revision | why |
|---|---|---|---|
| `scripts/common/mine-repo.sh` | `Azathothas/TEMPLATE` | `main`, fetched 2026-09-01 | the methodology mandates it and forbids writing your own fetcher |
| GNU libiconv 1.18 | ftp.gnu.org | pinned in `scripts/build-libiconv.sh` | fetched and built, not committed |

`references/` holds 12 upstream trees with a `PROVENANCE.md` each, naming the
commit, the route, and what could not be fetched. ⛔ One deliberate deletion:
`references/pkgforge-dev__cross-libc-dlopen/tree/docs/AGENTS.md`, removed
because the vendoring methodology forbids carrying a third party's agent
instruction file into this tree. Recorded in that repository's `PROVENANCE.md`.

## 16. Patches

**There are none.** No file under `references/` or in any POC's upstream source
is modified. Two POCs pass *configuration*, which is not a patch and is
recorded here:

| project | configuration | why it is not a patch |
|---|---|---|
| CPython | `Modules/Setup.local` generated from configure's own `Modules/Setup.stdlib` with `*shared*` → `*static*` | CPython's own file documents exactly this (`ln -sfr Modules/Setup.stdlib Modules/Setup.local`). No source file changes. |
| CPython | `py_cv_module_nis=n/a`, `--disable-test-modules` | both are supported configure inputs |
| ncurses | `--with-terminfo-dirs=...` | a configure flag, and required for correctness — see §11.5 |

## 17. Methodology this project follows

Three documents, fetched from `Azathothas/TEMPLATE`:
`docs/methodology/{experiments,references,vendoring}.md`. The obligations that
actually shaped this tree:

- an experiment is **a script in the tree**, numbered, with the question in its
  header, pinned inputs, conditions printed, and exit codes that mean 0/1/2 —
  never a transcript;
- **a negative result is committed**;
- **measure from outside**, and check whether the instrument perturbed the
  measurement (it did, twice — §20);
- **an absence is not a zero** without a positive control that the probe finds;
- **keep the corpus**, tracked, with provenance;
- **never fabricate a number**.

## 18. Evidence

`evidence/<name>/RESULT.txt` is the committed output of each experiment and
POC, with its conditions block. Raw traces and build trees are regenerated by
re-running and are `.gitignore`d — the script is the reproduction path.

## 19. Key findings, each with where to check it

1. **`__nss_configure_lookup` works from a static link and closes the NSS
   hole completely.** Zero host NSS modules on 11 of 11, DNS still resolving.
   `experiments/20-`.
2. **glibc still *opens* `/etc/nsswitch.conf` under the override.** It does not
   *use* what the file names. Stating it the other way round would be wrong.
3. **The openSUSE crash arrives through `passwd: compat`, not through DNS.** A
   hosts-only fix would have missed it, which is why the override covers all 14
   databases.
4. **Static glibc iconv has no working configuration.** Path matches → crash;
   path differs → silent loss. There is no third column.
5. **Static GNU libiconv removes the whole class**, with no data directory and
   no source change, at ~900 KiB and only for programs that call `iconv`.
6. **glibc's C.UTF-8 is files on disk, not code in libc.**
7. **A glibc locale is a tree, not a directory** — `LC_MESSAGES` is itself a
   directory. Missing one category fails the whole `LC_ALL` composite silently.
8. **`dlopen` from a static glibc binary is host-dependent** — §11.1.
9. **Five distinct host *data* dependencies exist**, of which two remain
   unsolved — §11.3.

## 20. Known-weak claims, read these before the conclusions

⛔ **This revision corrected four claims that earlier versions of this work got
wrong.** That is the only honest estimate of how many are still wrong.

1. *"A static binary cannot dlopen an extension."* **Reversed by measurement.**
   It can, on 2 of 11. The POC that asserted it had also been built with
   `--disable-extensions`, which made the assertion pass for the wrong reason.
2. *"The nano binary handles terminals."* **False for all 11** until
   `--with-terminfo-dirs` was added; `--version` never initialises curses so
   the functional test could not see it.
3. *"curl verifies TLS on Debian/Ubuntu."* **An artefact of the harness.** This
   environment exports `CURL_CA_BUNDLE` for its proxy and `rootfs-run.sh`
   replicates it; the probe was measuring itself. It now unsets those.
4. *"`--embed-locale` works."* **It silently did nothing** for two rounds:
   first the option was dropped crossing into the chroot, then the data symbols
   were weak `const` definitions in the file that read them, so GCC
   constant-folded the count to 0.

⚠ **Assume more remain.** In particular, nothing here has been run on a second
machine, on a second kernel, or on a second architecture.

## 21. TODO, in the order worth doing

1. **Finish POC 50 (CPython).** In progress; the build is long. State: the
   static-module mechanism and the two configure inputs are settled (§16); the
   last observed failure was `_testinternalcapi` linking into
   `Programs/_freeze_module`, addressed with `--disable-test-modules`.
2. **Write `experiments/21-glibc-version-floor.sh`.** The claim that glibc
   ≥ 2.34 is required (because `files`/`dns` became builtin there) is currently
   *reasoned*, not measured. Build the experiment-20 probe inside the
   `debian-11` rootfs (glibc 2.31) and check whether the override still leaves
   `libnss_files.so.2` being dlopen'd. **This is the highest-value missing
   measurement** — it is the justification for the pinned build image.
3. **Overhead.** Startup time, RSS, binary size and build time, for: native
   dynamic, plain `gcc -static`, and `pgb`. Nothing is measured yet and no
   number should be quoted until it is.
4. **CI.** A workflow that runs experiments 10/20/30 and the POC matrix. It
   must exercise the portability objective, not merely compile the tool.
   ⚠ GitHub runners are unprivileged, so `rootfs-run.sh`'s chroot will not work
   there — CI needs the container engines instead, which is also how the
   **UNTESTED** docker/podman engine finally gets exercised.
5. **aarch64**, per §14.
6. **`--embed-terminfo`**, if the terminfo limitation matters. The mechanism is
   already proven by `--embed-locale`; ncurses reads `TERMINFO` from the
   environment, so it is the same shape.

## 22. Things a future session should NOT redo

- **Do not try to make host NSS modules load correctly.** The goal is to keep
  them out; that is the fix, and §19.1 shows it works.
- **Do not bundle glibc's gconv modules.** They carry `DT_NEEDED libc.so.6`, so
  bundling them reintroduces the second libc on every musl host. Static
  libiconv is the answer and it is measured.
- **Do not use `ldd`/`file` output as a test.** §4.
- **Do not write a new OCI puller or reference fetcher.** Both exist and carry
  selftests.
- **Do not assert a limitation without measuring it.** §20.1 is what that costs.
