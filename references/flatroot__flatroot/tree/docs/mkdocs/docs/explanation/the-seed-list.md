---
tags:
  - explanation
  - resolver
  - dependencies
---

# The seed list

The resolver takes a list of names and produces an install set. The natural assumption is that the list is whatever the user typed: `flatroot install bash` produces the input list `["bash"]`. That assumption produces a broken rootfs.

A rootfs assembled from `bash` plus only what `bash`'s own metadata declares it depends on is missing things every working rootfs needs but no single package's metadata directly demands: a libc the binaries link against, a directory layout the extractor expects (`/etc`, `/var`, `/usr`), a shell at `/bin/sh`, the small kit of utilities that post-install scripts invoke without declaring. Some distros encode these requirements as `Essential: yes` markers in the index; others assume the installing tool knows what they are and adds them.

The install command therefore merges three sources before calling the resolver: every package the user named on the command line, plus a per-distro hardcoded base set, plus — on Debian and Ubuntu — every package the index marks `Essential: yes`. The merged list is the **seed list**: the starting point the resolver actually walks from. This page explains why each source exists, what would go wrong without it, and why the merge order matters.

## Base packages: what every rootfs needs before the user asks

Strip a rootfs down to the smallest set of packages that can run a shell. Across every supported distro, three things are always present: a libc, a shell at `/bin/sh`, and a directory-layout package that creates the directory tree everything else extracts into. On most distros, a fourth thing is present: `coreutils`, because post-install scripts call `mv`, `ln`, `mkdir`, and `rm` without declaring a dependency on the package that provides them.

The base set is hardcoded per distro because the names differ:

| Distro | Libc | Layout | Shell | Coreutils |
|--------|------|--------|-------|-----------|
| Arch, CachyOS | `glibc` | `filesystem` | `bash` | `coreutils` |
| Alpine | `musl` | `alpine-baselayout` | `busybox-binsh` | `busybox` |
| CentOS, Fedora, Alma, Rocky | `glibc`, `glibc-common` | `filesystem`, `basesystem` | `bash` | `coreutils` |
| openSUSE | `glibc` | `filesystem` | `bash` | `coreutils` |

Debian and Ubuntu have no hardcoded base set. They rely on the Essential mechanism instead.

The base set is not derived from the index. It is maintained in the source code as a constant per distro. The reasoning is that the base set encodes a property of the distro — "this distro needs `filesystem` to create `/etc` and `/var`" — that the index itself does not state. No package declares a dependency on `filesystem` because every package assumes it is already there. The base set closes that assumption.

## Essential packages: when the distro declares what is mandatory

Debian and Ubuntu mark certain packages as `Essential: yes` in their index. The marker is the distro's own way of saying "this package is required for the system to function and must always be installed." Typical members: `dash` (the default `/bin/sh`), `coreutils`, `grep`, `sed`, `base-passwd`, `base-files`, `dpkg`, `libc-bin`, `tar`, `gzip`. The set is about thirty packages on a current Debian release.

FlatRoot queries the index for every package with `Essential: yes` and adds them all to the seed. The set is read from the index at install time, not hardcoded in the source. The advantage is that the distro can change its mind: when the next Debian release adds or drops a package from the Essential set, FlatRoot picks up the change on the next index fetch with no code change. The disadvantage is asymmetry: no other distro family has an Essential mechanism in its index format, so every other backend must ship a hardcoded list.

Why not hardcode the Debian Essential list and avoid the query? Because hardcoding would drift. The Essential set on Debian 12 (bookworm) is not the same as on Debian 13 (trixie). A hardcoded list would be correct for the release it was written against and wrong for every release after. The query costs one SQL `SELECT` per install and guarantees correctness for every release, past and future.

## User packages: what the user asked for

The user's command-line names are the third source. Unlike base and Essential names, a user-requested name that does not resolve to any package in the index is a fatal error. The resolver cannot install something it cannot find, and silently dropping a name the user explicitly asked for hides bugs.

The distinction matters. Base and Essential names that do not resolve are silently dropped because they originate inside FlatRoot's own assumptions — a stale base-package constant, a package that was renamed upstream. The user's names originate outside FlatRoot; a mismatch between what the user asked for and what the index contains is either a typo or a broken index, and both deserve a hard error.

## Merge order and determinism

The three sources are merged in a fixed order: base packages first, Essential packages second, user packages third. Duplicates are skipped — a name that appears in both base and Essential is added once, and a user package that matches either earlier source is added once.

The order is for determinism. Two runs of the same command against the same index must produce the same seed in the same order, which then produces the same install set. The walker's BFS treats every name equally regardless of its source, but the queue order determines the visit order, and the visit order must be identical across runs for the system to be reproducible.

The order also shapes the install order. Because the walker commits packages in queue order, and the base and Essential packages are queued ahead of the user's requests, those packages — among them the layout-creating `usrmerge`/`filesystem` — are committed first and extracted before the user's packages and the deeper closure. (The `analyze trace` command is a different path: it seeds only the user's own patterns, with no base or Essential injection, and renders a flat, name-sorted list of entries rather than a tree.)

## The seed as a contract

The seed list is the boundary between the install command and the resolver. The install command assembles it; the resolver consumes it. The resolver does not know which names were base, which were Essential, and which were user-requested. Every name in the seed is one iteration of the BFS queue, and every iteration follows the same rules.

This is intentional. If the resolver treated base packages differently from user packages — resolving their dependencies with different rules, or skipping them if they conflicted — the install set would depend on which source a name came from. Two names that are the same string would produce different behaviour depending on how they arrived in the seed. Keeping the sources opaque across the boundary prevents that.

--8<-- "_glossary.md"
