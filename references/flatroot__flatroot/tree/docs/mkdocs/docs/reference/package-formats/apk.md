---
tags:
  - reference
  - formats
  - alpine
---

# apk

Distros: Alpine Linux. Parser: `src/pkg/apk.rs`.

## Archive

| Property | Value |
|----------|-------|
| Extension | `.apk` |
| Container | Three concatenated gzip streams |
| Stream 1 | Signature (RSA, skipped) |
| Stream 2 | Control — tar with `.PKGINFO` and optional `.post-install` |
| Stream 3 | Data — tar with the filesystem tree |

Each stream is a separate gzip member. FlatRoot uses `bufread::GzDecoder` so decompression stops at gzip member boundaries, allowing the cursor to advance to the next stream.

### Control-stream contents

The control tar typically contains `.PKGINFO` (the package's own metadata, using `pkgname = busybox` / `pkgver = …` syntax) plus any `.post-install` / `.pre-install` lifecycle scripts. flatroot lifts the install scripts out and otherwise reads package metadata from the repository-level `APKINDEX` (see **Index** below), not from `.PKGINFO`.

### APKINDEX record fields (relevant)

| Prefix | Field |
|--------|-------|
| `P:` | Package name |
| `V:` | Package version |
| `D:` | Dependency |
| `p:` | Provides |
| `i:` | Install-if triggers |
| `C:` | Checksum (`Q1` + base64 SHA-1) |
| `S:` | Size |
| `T:` | Description |

## Index

| Property | Value |
|----------|-------|
| Format | Multi-stream gzip tar |
| File | `APKINDEX.tar.gz` |
| Fetch path | `{mirror}/{path_dir_packages}/APKINDEX.tar.gz` |
| Repos | `main`, `community` |

The tar contains a single `APKINDEX` file. Each record spans several lines — one field per line — and records are separated by a blank line (the parser flushes a record at every empty line). Fields use single-letter prefixes:

```
P:busybox
V:1.37.0-r31
T:Size optimized toolbox of many common UNIX utilities
D:so:libc.musl-x86_64.so.1
p:cmd:busybox
i:musl busybox
C:Q1abc123==
S:534120
```

No supplementary path index. Alpine expresses sonames through `p:` lines under the `so:` namespace, not through file-path dependencies. FlatRoot writes an empty `.pathidx` file for Alpine.

## Versioning

| Property | Value |
|----------|-------|
| Shape | `upstream[_suffix]-r<pkgrel>` |
| Algorithm | [apk-tools](../../appendix/versioning/version-comparison.md) (numeric components, optional trailing letter, ranked `_suffix` chain, `-r` revision) |
| Release separator | `-r` |

Suffix sort order (lowest to highest):

| Suffix | Meaning |
|--------|---------|
| `_alpha` | Alpha |
| `_beta` | Beta |
| `_pre` | Pre-release |
| `_rc` | Release candidate |
| (none) | Release |
| `_p` | Patch (post-release) |

`1.0_rc1 < 1.0 < 1.0_p1`.

The `~=` and `~` fuzzy operators are implemented as component-prefix matches (`ApkVersion::fuzzy_matches`): `pkg~=1.2` (or `pkg~1.2`) accepts every release whose numeric components begin `1.2` — e.g. `1.2`, `1.2.7` — not the bare `>=1.2 and <2.0` reading.

## Dependencies

| Property | Value |
|----------|-------|
| Declaration | `D:` line, space-separated tokens |
| Constraint syntax | Concatenated directly: `musl>=1.2.5` |
| Alternative syntax | None — each token is a single required package |
| Negative dependencies | `!pkg` (conflict, skipped during parsing) |

```
D:musl>=1.2.5 so:libc.musl-x86_64.so.1 zlib
```

### Virtual package namespaces

| Prefix | Namespace |
|--------|-----------|
| `so:` | Shared library soname |
| `cmd:` | Command name |
| `pc:` | pkg-config name |

These are naming conventions. The resolver treats `cmd:` and `pc:` as ordinary name strings, but `so:` is meaningful to library resolution: a soname lookup synthesizes a `so:<soname>` provider query on Alpine, and canonicalization strips the `so:` prefix.

### Install-if triggers

The `i:` field declares conjunctive triggers:

```
i:musl busybox
```

The package is installed when *all* trigger names are present in the resolved set. Evaluated in a fixpoint loop after the main BFS and RPM rich-dep phases. See [Install-If Triggers](../../appendix/dependencies/install-if.md).

### Soft dependencies

Alpine has no recommends/suggests mechanism. `--with=recommends` and `--with=suggests` have no effect on Alpine sources.

Alpine has no `Essential` mechanism. Base packages (`musl`, `busybox`, `alpine-baselayout`, `busybox-binsh`) are hardcoded.

## Checksum

| Property | Value |
|----------|-------|
| Algorithm | Q1 + base64-encoded SHA-1 |
| Prefix | `q1-sha1:` |
| Scope | Second gzip stream (control) compressed bytes only |

Not interchangeable with a full-file `sha1:` hash. FlatRoot verifies by parsing gzip member boundaries and hashing the second member's compressed bytes.

## Post-install

| Property | Value |
|----------|-------|
| Script file | `.post-install` from the control stream |
| Extracted to | `.flatroot/scripts/<pkg>:<arch>/post-install` |
| Invocation | `/bin/sh <script> <name>` |
| Runner | `src/postinstall/alpine.rs` (`AlpinePostInstall`) |

Scripts are standalone (no shell functions) and execute with the package name as `$1` (the staged identity carries name and arch, not version). Most are a few lines creating users or updating caches.

### Alpine-specific stubs

| Command | Behaviour |
|---------|-----------|
| `rc-update`, `rc-service`, `openrc` | No-op (OpenRC service management) |
| `apk` | No-op (package manager queries) |

--8<-- "_glossary.md"
