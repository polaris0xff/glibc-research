---
tags:
  - reference
  - formats
  - debian
  - ubuntu
---

# deb

Distros: Debian, Ubuntu. Parser: `src/pkg/deb.rs`.

## Archive

| Property | Value |
|----------|-------|
| Extension | `.deb` |
| Container | [`ar`](../../appendix/formats/ar.md) |
| Members | `debian-binary`, `control.tar.*`, `data.tar.*` |
| Data compression | xz (modern), gzip (older), bzip2 (wheezy), zstd (experimental) |

`debian-binary` carries the format version string (`2.0`) and is ignored. `control.tar.*` holds package metadata. `data.tar.*` holds the filesystem tree. Compression is detected from the member name.

### control.tar.* contents

| File | Purpose |
|------|---------|
| `control` | Package metadata (name, version, dependencies, …) — listed for reference only; **not parsed** by FlatRoot |
| `postinst` | Post-install script |
| `prerm` | Pre-removal script (ignored by FlatRoot) |
| `md5sums` | Per-file MD5 hashes (ignored) |
| `conffiles` | Configuration file paths (ignored) |

FlatRoot reads no package metadata from `control.tar.*` — every field comes from the `Packages.gz` index. The whole `control.tar.*` member is unpacked into the package's staging directory, and only `postinst` is consulted from it; the rest is ignored.

## Index

| Property | Value |
|----------|-------|
| Format | Gzip-compressed [RFC 822](../../appendix/formats/rfc822.md) text records |
| File | `Packages.gz` |
| Fetch path | `{mirror}/dists/{suite}/{component}/binary-{arch}/Packages.gz` |
| Record separator | Blank line |
| Field separator | `Key: Value` |

Each blank-line-separated record describes one package:

```
Package: bash
Version: 5.2.15-2+b10
Depends: libc6 (>= 2.36), libtinfo6 (>= 6)
Provides: bash-static
Filename: pool/main/b/bash/bash_5.2.15-2+b10_amd64.deb
SHA256: abc123...
Essential: yes
```

### Supplementary index

| File | Purpose |
|------|---------|
| `Contents-<arch>.gz` **and** `Contents-all.gz` | Path-to-package mapping — the per-arch packages and the `Architecture: all` packages respectively — populating the [binary path index](../path_index.md) |

On Debian both listings are component-scoped under `main` (`dists/<suite>/main/Contents-*.gz`) and fetched per repo; the long-term archive merges them into the per-arch file and drops `Contents-all`. Ubuntu never splits out `Contents-all` and declares a single suite-level `Contents-<arch>.gz` on the base suite's first component only — the `-updates`/`-security` suites and other components declare none.

## Versioning

| Property | Value |
|----------|-------|
| Shape | `[epoch:]upstream[-revision]` |
| Algorithm | [dpkg](../../appendix/versioning/version-comparison.md) |
| Epoch separator | `:` |
| Revision separator | `-` |
| Special characters | `~` sorts before everything; `+` sorts after |

Constraint operators use doubled angle brackets to avoid shell ambiguity:

| Operator | Meaning |
|----------|---------|
| `<<` | Strictly less |
| `<=` | Less or equal |
| `=` | Equal |
| `>=` | Greater or equal |
| `>>` | Strictly greater |

## Dependencies

| Property | Value |
|----------|-------|
| Group separator | `,` |
| Alternative separator | `|` |
| Constraint syntax | `(<<= version)` appended to name |

```
Depends: debconf (>= 0.5) | cdebconf, libc6 (>= 2.36)
```

### Dependency kinds

| Field | Kind | Followed by default |
|-------|------|---------------------|
| `Depends` | Hard | Yes |
| `Pre-Depends` | Hard (merged with Depends) | Yes |
| `Recommends` | Soft | No (`--with=recommends`) |
| `Suggests` | Soft | No (`--with=suggests`) |
| `Conflicts` | Conflict (warning only) | — |
| `Breaks` | Conflict (warning only) | — |

`Pre-Depends` is merged with `Depends` during parsing — FlatRoot extracts all packages before running scripts, so the distinction is meaningless.

`Recommends` and `Suggests` are only resolved when explicitly enabled.

Multi-Arch qualifiers (`:any`) appearing in dependency names are stripped during parsing.

### Essential

Packages marked `Essential: yes` in the index are queried and added to the seed list automatically. This is the Debian-family equivalent of base packages. Typical members: `dash`, `coreutils`, `grep`, `sed`, `base-passwd`, `base-files`, `dpkg`, `libc-bin`, `tar`, `gzip`.

## Checksum

| Property | Value |
|----------|-------|
| Algorithm | SHA-256 |
| Prefix | `sha256:` |
| Scope | Full archive |

## Post-install

| Property | Value |
|----------|-------|
| Script file | `postinst` from `control.tar.*` |
| Extracted to | `.flatroot/scripts/<pkg>:<arch>/postinst` |
| Invocation | `/bin/sh -e <script> configure ""` |
| Environment | `DEBIAN_FRONTEND=noninteractive`, stdin closed |
| Runner | `src/postinstall/debian.rs` (`DebPostInstall`) |

--8<-- "_glossary.md"
