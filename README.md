# portable static glibc

Build a **normal Linux ELF executable** against glibc that runs unchanged on
glibc *and* musl distributions. No launcher, no AppDir, no packaging format,
nothing beside it — one file you copy and run.

```sh
sh pgb env create             # a pinned Debian 12 build environment
sh pgb build -- make          # your project's own build, unmodified
sh pgb verify ./yourprogram   # run it on 11 real distributions
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

Three mechanisms, none of which changes a line of application source:

| | |
|---|---|
| **NSS** | a constructor calls `__nss_configure_lookup()` — a public `GLIBC_2.2.5` symbol that is in `libc.a` — pinning every database to services glibc ≥ 2.34 implements *inside* libc. The host's nsswitch.conf then names nothing that can be loaded. |
| **iconv** | `-Wl,--wrap` redirects the three public iconv entry points to statically linked GNU libiconv. It acts at the final link, so it catches calls from any object, including archives built long before this tool existed. |
| **locale** | opt-in `--embed-locale`: C.UTF-8 embedded, written out only if the host cannot answer a UTF-8 `setlocale`. |

Delivery is compiler wrappers on `PATH`. autotools, CMake, meson and plain
make pick them up without knowing `pgb` exists. `sh pgb explain` prints every
injected flag and the experiment behind it.

## Evidence

Five real projects, stock tarballs, stock `./configure`, **no source patches**,
each passing a real functional test on all 11 environments while loading zero
host shared objects:

| project | what it stresses |
|---|---|
| **GNU awk** 5.3.1 | locale, iconv, and a `dlopen` extension API |
| **GNU nano** 8.2 + ncurses 6.5 | terminfo data, a static dependency chain, multibyte |
| **curl** 8.11.0 + OpenSSL 3.0.15 | `getaddrinfo`/NSS, real DNS, real TLS, CA bundles |
| **jq** 1.7.1 + oniguruma | Unicode round-trip, surrogate pairs |
| **CPython** 3.12.7 | 49 extension modules linked *in*, empty `lib-dynload`, NSS via `socket`/`pwd` |

```sh
for p in poc/*/run.sh; do sh "$p"; done
```

## Honest limits

⛔ **If you can build against musl instead, do that.** The same program was
built eight ways and run on the same 11 environments
([`docs/comparison.md`](docs/comparison.md)). A plain **static musl** binary
matches `pgb` exactly — 11/11 running, 11/11 loading no host shared object —
while starting about 6× faster (160 µs vs 980 µs per exec) and shipping
447 KB against 2.1 MB. `pgb` beat every *packaging format* on that matrix
(AppImage 2/11, onelf 3/11, Flatpak and snap 0/11) and did not beat that.

⭐ **So this tool is for when the build has to be glibc**: a dependency that
will not cross to musl, a prebuilt glibc-linked archive, glibc-specific
behaviour, or `--wrap` onto objects compiled long before this existed. That is
a narrower claim than "portable Linux binaries", and it is the one the
measurements support.

⛔ **`dlopen` of a *host* shared object is host-dependent, and success is the
worse outcome.** gawk's own extension loads on Debian 12 and Arch — dragging
the host loader and libc into the process — and is refused on the other nine.
A program whose core function is loading host plugins is outside the class this
tool serves.

⛔ **Static linking says nothing about data.** Five distinct host data
dependencies were found: gconv (solved), locale (solved), **terminfo** and
**CA bundles** (unsolved, distro-specific paths), and a runtime's own library
tree.

Everything measured, everything not measured, and what a previous revision got
wrong: [`docs/limitations.md`](docs/limitations.md) and
[`docs/history/corrections.md`](docs/history/corrections.md).

## Where things are

| | |
|---|---|
| [`docs/AGENTS.md`](docs/AGENTS.md) | ⭐ **the standalone handoff.** Read this first if you are picking the project up |
| [`docs/limitations.md`](docs/limitations.md) | what it cannot do, with the measurement behind each |
| [`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md) | the operator's acceptance bar, and how far short of it this is |
| [`docs/comparison.md`](docs/comparison.md) | ⭐ **the head-to-head**: eight ways to ship the same program, same 11 environments, and where `pgb` loses |
| [`docs/research/prior-art.md`](docs/research/prior-art.md) | the reference sweep, verdicts and provenance |
| `experiments/` | numbered, re-runnable. Exit 0 matched, 1 did not, 2 could not run |
| `poc/` | the five projects |
| `references/` | the corpus: 12 upstream trees and their trackers, tracked |
| `evidence/` | the committed output of every experiment and POC |

## Requirements

root and `CAP_SYS_ADMIN` (the test bed is `unshare --mount` + `chroot`, because
this was developed on a machine with no container daemon), plus `curl`,
`python3`, `strace` and a C toolchain. `sh pgb doctor` reports what is missing.
Docker and Podman engines exist in the tool and are **untested**.

## Licence

MIT. Vendored components keep their own; see [`docs/AGENTS.md`](docs/AGENTS.md) §12.
