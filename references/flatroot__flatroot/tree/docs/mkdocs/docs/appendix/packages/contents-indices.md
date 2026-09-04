---
tags:
  - appendix
  - packages
---

# Contents Indices

A contents index is a repository-level listing that maps every installed file path to the package that ships it — the answer to "which package would put `usr/bin/convert` on my system?" before anything is downloaded. Distributions publish it so that file-level questions can be answered from repository metadata alone: utilities like `apt-file` and `dnf provides` are built on it, and it is the only way to resolve a dependency or a request phrased as a file path back to an installable package.

## The line format

Every family's listing reduces to the same fact — one path, one owner — but Debian's format is the canonical example: a whitespace-separated path and a qualified package list, one file per line, no leading slash.

```
usr/bin/bash                                            shells/bash
usr/share/fonts/truetype/dejavu/DejaVuSans.ttf          fonts/fonts-dejavu-core
etc/passwd                                              base/base-passwd,admin/passwd
```

## What each family publishes

| Family | Listing | Scope |
|---|---|---|
| Debian | `dists/<suite>/main/Contents-<arch>.gz` **and** `dists/<suite>/main/Contents-all.gz` | split: per-arch carries architecture-specific packages only; `Contents-all` carries every `Architecture: all` package |
| Ubuntu | `dists/<suite>/Contents-<arch>.gz` (suite level) | merged: each per-arch file includes `Architecture: all` packages |
| RPM (CentOS, Fedora, Alma, Rocky, openSUSE) | `repodata/…-filelists.xml` | complete: every package's files, all architectures the repo serves |
| Arch / CachyOS | `<repo>.files` database | complete per repository |
| Alpine | — | none: APKINDEX carries no file lists; only `so:` (libraries) and `cmd:` (commands) are discoverable, through the provides namespaces |

## Debian's split: per-arch plus Contents-all

Since the `Contents-all` split (Debian 9 "stretch"), Debian's per-arch listing on the live mirrors deliberately excludes `Architecture: all` packages. The consequence is easy to underestimate: fonts, icon themes, time-zone data, documentation, and most pure-Python libraries are all `Architecture: all`, so a consumer reading only `Contents-<arch>` sees none of their files. Measured against `bookworm` on `deb.debian.org`: `Contents-amd64` holds zero entries under `usr/share/zoneinfo/` (tzdata), zero under `usr/share/fonts/truetype/dejavu/` (fonts-dejavu-core), and zero for python3-requests' module tree — while `Contents-all` holds 1,265, 22, and 18 entries for the same prefixes respectively. Both files must be read for full coverage; the two are disjoint by construction, since a package is either architecture-specific or `Architecture: all`, never both.

## The mirror lifecycle changes the layout

The same suite is laid out differently depending on which mirror generation serves it:

| Mirror | Per-arch Contents | Separate `Contents-all` |
|---|---|---|
| `deb.debian.org` (current suites) | split — arch-specific packages only | published |
| `snapshot.debian.org` (dated pins) | split, preserved as on the live mirror at that date | published |
| `archive.debian.org` (retired suites) | **merged — `Architecture: all` entries are folded in on import** | removed, and absent from the suite's `Release` manifest |

So a retired suite such as `buster` answers every file-ownership question from its per-arch file alone, and the `Contents-all` URL that worked while the suite was current returns 404 from the long-term archive — the file is genuinely gone there, not relocated. Ubuntu never splits in the first place: its suite-level per-arch Contents is always the merged form, on every mirror. A consumer that wants full coverage across all three mirror generations must therefore read both files where they exist and tolerate the absence of `Contents-all` where the merged per-arch file already carries everything.
