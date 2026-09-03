# portable static glibc

A **toolchain** for building a normal Linux ELF against glibc that runs
unchanged on glibc *and* musl distributions. No launcher, no AppDir, no
packaging format, nothing beside it — one file you copy and run.

⭐ **`pgb` is not a packaging format.** AppImage, Flatpak and snap answer *how
does this reach a machine*. `pgb` answers *how does a developer get from source
to a binary that runs* — and the answer it hands back is an ordinary
executable. [`docs/design/toolchain.md`](docs/design/toolchain.md) is where it
is going: `pgb build <url-or-package>`, with the tool resolving the source,
planning the dependencies and linking statically as far as each one allows.

```sh
./pgb env create             # a pinned Debian 13 build environment (glibc 2.41)
./pgb build -- make          # your project's own build, unmodified
./pgb verify ./yourprogram   # run it on 11 real distributions
```

---

## Why `gcc -static` is not enough

`file` says "statically linked". `ldd` says "not a dynamic executable". Both
are misleading, and the gap is not small. Measured across 11 distributions
pinned by digest:

| a plain `gcc -static` glibc binary | result |
|---|---|
| reads the **host** `/etc/nsswitch.conf` and `dlopen`s the modules it names | host NSS modules loaded on 5 of 11; **SIGFPE on Arch Linux and openSUSE Leap** |
| reaches character encodings through `dlopen`ed gconv modules | **SIGFPE/SIGABRT on Debian 11, Debian 12, Ubuntu 20.04**; 11 of 12 encodings silently unavailable everywhere else |
| needs glibc locale files for a UTF-8 codeset | `ANSI_X3.4-1968` on all 4 musl hosts |

Each of those is a host shared object entering a "static" process, carrying
`DT_NEEDED libc.so.6` — a **second libc** in a process that was supposed to
have none.

⚠ **There is no set of distributions where a plain static glibc binary is
safe.** Which ones "work" depends on which subsystem the program touches, not
on the distribution.

## What `pgb` does

Four mechanisms, none of which changes a line of application source:

| | |
|---|---|
| **NSS** | a constructor calls `__nss_configure_lookup()` — a public `GLIBC_2.2.5` symbol that is in `libc.a` — pinning every database to services glibc ≥ 2.34 implements *inside* libc. The host's nsswitch.conf then names nothing that can be loaded. |
| **iconv** | `-Wl,--wrap` redirects the three public iconv entry points to statically linked GNU libiconv. It acts at the final link, so it catches calls from any object, including archives built long before this tool existed. |
| **locale** | opt-in `--embed-locale`: C.UTF-8 embedded, written out only if the host cannot answer a UTF-8 `setlocale`. |
| **own plugins** | opt-in `--wrap-dlopen`: `dlopen`, `dlsym`, `dlclose` and `dlerror` answered from a table `pgb` generates with `nm` from the objects your build produced. A program loading its *own* plugins never needed a loader — the code is already in the link and `dlopen` is only doing a name lookup. Nothing is mapped, so no second libc can enter. ⚠ Not for **host** plugins; see the limitations. |

Delivery is compiler wrappers on `PATH`. autotools, CMake, meson and plain
make pick them up without knowing `pgb` exists. `pgb explain` prints every
injected flag and the experiment behind it.

## Evidence

**Ten real projects**, stock tarballs, stock `./configure`, **no source
patches**, each passing a real functional test on all 11 environments while
loading zero host shared objects:

| project | what it stresses |
|---|---|
| **GNU awk** 5.3.1 | locale, iconv, and a `dlopen` extension API |
| **GNU nano** 8.2 + ncurses 6.5 | terminfo data, a static dependency chain, multibyte |
| **curl** 8.11.0 + OpenSSL | `getaddrinfo`/NSS, real DNS, real TLS, CA bundles |
| **jq** 1.7.1 + oniguruma | Unicode round-trip, surrogate pairs |
| **CPython** 3.12.7 | 49 extension modules linked *in*, empty `lib-dynload` |
| **LevelDB** 1.23 | C++ static init, exceptions, RTTI, iostreams/locale |
| **SQLite** 3.47.0 + 15 extensions | `dlopen` by user path, `dlsym` by derived name |
| **MLT** 7.30.0 + ffmpeg 7.1 | a 142 MB static libavcodec, 8 `dlopen`'d modules |
| **Qt 6.11.1** | a very large C++ build, static plugin import, QPA |
| ⭐ **Qt 6.11.1 + xcb** | a **real X window**, OpenSSL linked into QtNetwork, QtSql |

⭐ The largest is a static Qt 6 application that opens a mapped, exposed window
through the real xcb plugin and does a SQLite round trip returning `日本` — on
all 11 environments, with zero host shared objects.

```sh
for p in poc/*/run.sh; do sh "$p"; done
```

## Why glibc, and not just build against musl

A static musl binary is also portable — it is smaller and starts faster. What
it is not is glibc, and that shows up the moment the program does work. Same
machine, same compiler, libc the only variable
([`experiments/61-`](experiments/61-libc-throughput.sh)), ns per operation:

| | glibc static | musl static |
|---|---|---|
| malloc, 4 threads | **4.53** | 584.71 |
| qsort | **93.20** | 921.49 |
| strlen/strchr/strstr | **149.14** | 1051.09 |

⭐ On **Alpine**, where the ordinary choice is a musl build, a `pgb` binary does
that 4-thread allocator workload in **4.68 ns** against musl's **592**. glibc's
throughput on a machine that ships no glibc, and `pgb` costs nothing over a
plain static build to carry it.

## Open problems

⚠ **`pgb`'s static output does not beat an anylinux AppImage on portability
and does not run in more places.** On the same 11 environments both run
everywhere and both deliver glibc's throughput. `pgb` is smaller and simpler in
shape — one file, no mount, no extraction, nothing written — and
[Anylinux-AppImages](https://github.com/pkgforge-dev/Anylinux-AppImages)
reaches software `pgb` does not yet: as its own guide puts it, *"Compile
statically! Sure, that works, go and compile all of kdenlive statically and get
back to me once you get it done."*

⛔ **And where `pgb` bundles instead, it is bigger and slower.** Measured
against `kdenlive-AppImage-Enhanced`, same upstream release
([`experiments/90-`](experiments/90-kdenlive-vs-enhanced.sh)): ours
477,191,058 B against 191,900,604, renders 3,559 ms against 1,323 and starts
181 ms against 52. ⭐ **The one column it wins is the one this project is
about**: ours loads **zero host shared objects on 11 of 11**, the competitor on
4 of 11. Closing the other three is the work, not the boundary —
[`docs/comparison.md`](docs/comparison.md) has both columns and
[`docs/AGENTS.md`](docs/AGENTS.md) §13 has the routes.

⚠ **`dlopen` of a *host* shared object is host-dependent, and success is the
worse outcome.** ⭐ Measured at the current pin, gawk's own extension loads on
**exactly one of eleven — Fedora 42, the row whose host glibc equals the
build's** — dragging the host loader and libc into the process. Below glibc
2.33 it is refused with an honest link error; in between it takes a SIGFPE.
`docs/limitations.md` §1 has the messages. Four untried routes to fixing it are
listed in `docs/AGENTS.md` §7; none has been shown to be closed.

⭐ **Static linking says nothing about data, and all six are now closed.**
Six distinct host data dependencies were found: gconv, locale, terminfo, CA
bundles, NSS and — ⚠ **found only on 2026-09-03c, by asking whether the list
was complete** — the **timezone database**, which four of the eleven test
environments do not ship. Each has a mechanism — `--embed-locale`,
`--embed-terminfo`, `--embed-cacert`, `--embed-tzdata`, the iconv `--wrap` and
the NSS override — and `pgb explain` prints every flag it injects and the
experiment behind it.

Everything measured, everything not measured, and every claim that was made
and then disproved: [`docs/limitations.md`](docs/limitations.md) and
[`docs/history/corrections.md`](docs/history/corrections.md).

## Where things are

| | |
|---|---|
| [`docs/AGENTS.md`](docs/AGENTS.md) | ⭐ **the standalone handoff.** Read this first if you are picking the project up |
| [`docs/limitations.md`](docs/limitations.md) | the open problems, with the measurement behind each and the route out |
| [`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md) | the operator's acceptance bar, and how far short of it this is |
| [`docs/design/toolchain.md`](docs/design/toolchain.md) | ⭐ **what `pgb` is and where it is going**, and the language decision |
| [`docs/comparison.md`](docs/comparison.md) | the head-to-head: every way to ship the same program, same 11 environments |
| [`docs/research/prior-art.md`](docs/research/prior-art.md) | the reference sweep, verdicts and provenance |
| `experiments/` | numbered, re-runnable. Exit 0 matched, 1 did not, 2 could not run |
| `poc/` | the ten projects, and `92-miniflux` in progress |
| `references/` | the corpus: 32 upstream trees and their trackers, tracked |
| `evidence/` | the committed output of every experiment and POC |

## Requirements

`make` builds `./pgb` — it is one static Go binary and is **not committed**.

Running the test bed needs root and `CAP_SYS_ADMIN` (`unshare --mount` +
`chroot`), plus `curl`, `tar` and `xz`; `pgb verify` also needs `strace`.
`pgb doctor` reports what is missing and `pgb bootstrap` prepares a fresh
machine. The **docker** engine is exercised on every CI run
(`pgb verify --engine docker`, with a deliberately failing control); the
**podman** engine shares that code path and is untested.

## Licence

MIT. Vendored components keep their own; see [`docs/AGENTS.md`](docs/AGENTS.md) §12.
