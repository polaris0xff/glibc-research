---
tags:
  - explanation
  - multiarch
  - pipeline
---

# Multiarch

A Linux system can run binaries from more than one architecture. An `x86_64` host can execute `i686` (32-bit x86) binaries through the kernel's compatibility mode. An `aarch64` host can execute `armv7l` binaries through the same mechanism. Distributions support this by publishing separate package indices per architecture and allowing packages from multiple architectures to be installed into the same rootfs.

FlatRoot supports multiarch through the `--arch` flag. `flatroot --arch x86_64,i686 --from debian:bookworm install bash` builds a rootfs with both 64-bit and 32-bit packages. The two architectures share the same directory tree — there is one `/usr/lib`, not separate trees per architecture. The 32-bit packages install their libraries alongside the 64-bit ones, with the architecture encoded in the library path (`/usr/lib/i386-linux-gnu/`) or the soname (`libc.so.6` for 64-bit, `libc.so.6` for 32-bit in a different directory).

This page develops how FlatRoot runs the pipeline once per architecture, what is shared and what is separate, and why the post-install passes run once at the end rather than once per architecture.

## One pipeline, two architectures

When `--arch` contains multiple architectures, FlatRoot runs stages 1 through 6 of the pipeline once per architecture. Each architecture has its own index — the `i686` index contains different packages than the `x86_64` index, even though both come from the same distro release. Each architecture has its own resolver run, producing its own install set. Each architecture's packages are downloaded and extracted independently.

## What is shared

The rootfs directory is shared. Both architectures extract into the same directory tree. Each architecture's resolver builds its own full dependency closure independently — Essential packages like `base-files` are installed for both architectures. They create the same directory layout (`/etc`, `/usr`), so the second extraction is harmless. Where packages differ across architectures, their paths naturally don't collide: 32-bit libraries land under `/usr/lib/i386-linux-gnu/` while 64-bit libraries land under `/usr/lib/x86_64-linux-gnu/`.


The `.flatroot/` metadata directory is shared. The `manifest` records both architectures. The `packages` file contains records from both architectures in a single list, sorted by `(name, architecture)`. The `files/` directory contains per-package file lists from both passes, with the architecture encoded in the filename (`bash:x86_64`, `libc6:i686`).

## What is separate

The index database is separate. The `x86_64` index contains only `Architecture: amd64` packages; the `i686` index contains only `Architecture: i386` packages. The two databases are stored in separate files under `~/.cache/flatroot/index/`, keyed on the source's `cache_id` (e.g. `debian-bookworm-amd64.db` and `debian-bookworm-i386.db` for a multiarch Debian install), and queried independently.

The resolver run is separate. Each architecture's resolver walks its own index, resolves its own seed list, and produces its own install set. The `i686` resolver does not know about the `x86_64` packages already installed; it only knows about the `i686` index. This is intentional: the `i686` package `libc6` should be installed even though the `x86_64` `libc6` is already present, because the two are different packages with different sonames, different library paths, and different purposes.

The download and extraction are separate. Each architecture's packages are downloaded from the mirror's per-architecture pool and extracted into the shared rootfs. The two passes can write to the same directories — both put libraries under `/usr/lib` — but the paths do not collide because the architecture is encoded in the directory name (`/usr/lib/x86_64-linux-gnu/` vs. `/usr/lib/i386-linux-gnu/`).

## Why post-install runs once

After both architectures' packages are extracted, FlatRoot runs the post-install passes — ldconfig, distro scripts, cache hooks — exactly once, not once per architecture.

The reason is that the caches are architecture-neutral. `fc-cache` builds a font cache that both 64-bit and 32-bit applications read. Running it twice would regenerate the same cache twice, wasting time. The ldconfig cache (`/etc/ld.so.cache`) lists libraries from both architectures — the ldconfig binary scans all library directories regardless of architecture. Running ldconfig after the `x86_64` pass would generate a cache missing the `i686` libraries; running it again after the `i686` pass would correct it. Running it once at the end is both correct and faster.

The distro scripts are also run once, after all packages are extracted. A Debian postinst that runs `update-alternatives --install` might register an alternative that a later package's postinst depends on. Running scripts per-architecture would mean the `x86_64` scripts run before the `i686` packages are extracted, and an `i686` script that depends on an alternative registered by an `x86_64` script would fail. Running all scripts after all extractions avoids ordering dependencies between architectures.

## The ldconfig pass

ldconfig is the one post-install pass that must handle multiarch explicitly. The `ldconfig` binary inside the rootfs is typically from the first architecture's libc package — if `x86_64` was first, the ldconfig at `/sbin/ldconfig` is the 64-bit one. A 64-bit ldconfig scans the library directories configured in `/etc/ld.so.conf` and generates a cache that the 64-bit dynamic linker reads.

The 32-bit dynamic linker also reads `/etc/ld.so.cache`, but it interprets the cache entries through its own architecture filter. A single ldconfig run generates a cache that both dynamic linkers can use, as long as the library directories for both architectures are listed in `/etc/ld.so.conf`. FlatRoot ensures this by running the rootfs's ldconfig once after all packages are extracted, with no architecture-specific flags.

--8<-- "_glossary.md"
