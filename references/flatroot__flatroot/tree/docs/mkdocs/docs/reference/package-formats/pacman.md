---
tags:
  - reference
  - formats
  - arch
  - cachyos
---

# pacman

Distros: Arch Linux, CachyOS. Parser: `src/pkg/pacman.rs`.

## Archive

| Property | Value |
|----------|-------|
| Extension | `.pkg.tar.zst` |
| Container | Zstd-compressed tar |
| Metadata files | `.PKGINFO`, `.BUILDINFO`, `.MTREE`, `.INSTALL` (optional) |
| Payload | All other entries in the tar |

FlatRoot extracts `.INSTALL` to `.flatroot/scripts/<pkg>:<arch>/install` and skips the other dot-prefixed metadata files.

### .PKGINFO contents (for reference — not parsed)

FlatRoot does **not** read `.PKGINFO`; it is skipped along with the other dot-prefixed metadata files. Every package fact comes from the repository index's `desc` entries (the `%FIELD%` form shown under [Index](#index)). The fields below describe the in-archive format only:

| Key | Content |
|-----|---------|
| `pkgname` | Package name |
| `pkgver` | Package version |
| `depend` | Dependency entry |
| `optdepend` | Optional dependency (name + description, `:` separated) |
| `provides` | Virtual package name (optional `=version` suffix) |

## Index

| Property | Value |
|----------|-------|
| Format | Tar of per-package directories, gzip- or zstd-compressed (detected from magic bytes) |
| File | `<repo>.db.tar.gz` (Arch); `<repo>.db` with no suffix on CachyOS's v3 repos |
| Fetch path | `{mirror}/{path_dir_packages}/<repo>.db[.tar.gz]` |
| Repos | Arch: `core`, `extra` (plus `multilib` via `--from arch:multilib`). CachyOS layers `cachyos-core-v3`, `cachyos-extra-v3`, `cachyos-v3` on top of Arch's `core`/`extra`. |

Each package is a directory inside the tar containing a `desc` file. The `desc` format uses `%FIELD%` headers with values on subsequent lines, separated by blank lines:

```
%NAME%
bash

%VERSION%
5.3.009-1

%DEPENDS%
readline
glibc
ncurses

%PROVIDES%
sh
```

### Supplementary index

| File | Purpose |
|------|---------|
| `<repo>.files` (`.tar.gz` on Arch, no suffix on CachyOS v3) | Path-to-package mapping, populates the [binary path index](../path_index.md) |

Fetched sequentially per repo: each repo's `.db` is fetched, parsed, and inserted before its `.files` is fetched.

## Versioning

| Property | Value |
|----------|-------|
| Shape | `[epoch:]pkgver-pkgrel` |
| Algorithm | [libalpm vercmp](../../appendix/versioning/version-comparison.md) (an rpmvercmp-derived segment walk) |
| Epoch separator | `:` |
| Release separator | `-` |

Constraint operators use single angle brackets (no doubled brackets like Debian):

| Operator | Meaning |
|----------|---------|
| `<` | Strictly less |
| `<=` | Less or equal |
| `=` | Equal |
| `>=` | Greater or equal |
| `>` | Strictly greater |

Tildes are not used in Arch version strings.

## Dependencies

| Property | Value |
|----------|-------|
| Declaration | One name per line under `%DEPENDS%` |
| Constraint syntax | Concatenated directly: `glibc>=2.27` |
| Alternative syntax | None — each line is a single required package |

```
%DEPENDS%
glibc>=2.27
readline
ncurses
```

### Dependency kinds

| Field | Kind | Followed by default |
|-------|------|---------------------|
| `%DEPENDS%` | Hard | Yes |
| `%OPTDEPENDS%` | Soft (suggests) | No (`--with=suggests`) |

`%OPTDEPENDS%` entries have the shape `name: description`. FlatRoot strips the description and treats the name as a suggests-level dependency.

### Provides

`%PROVIDES%` entries can include a version: `libgomp.so=1-64`. The version after `=` distinguishes 64-bit providers (`=1-64`) from 32-bit providers (`=1-32`) of the same soname. The resolver checks dependency version constraints against this provides version, not the package version.

Arch has no `Essential` mechanism. Base packages (`filesystem`, `glibc`, `bash`, `coreutils`) are hardcoded.

## Checksum

| Property | Value |
|----------|-------|
| Algorithm | SHA-256 |
| Prefix | `sha256:` |
| Scope | Full archive |

## Post-install

| Property | Value |
|----------|-------|
| Script file | `.INSTALL` from archive root |
| Extracted to | `.flatroot/scripts/<pkg>:<arch>/install` |
| Wrapper | `#!/bin/bash` + `source /.flatroot/current-install` + `post_install "$1" \|\| true` |
| Invocation | `bash /.flatroot/current-install-wrapper ""` (the script's `$1` is an empty string positional arg) |
| Runner | `src/postinstall/arch.rs` (`ArchPostInstall`) |

`.INSTALL` files contain shell functions rather than standalone scripts. Uses `/bin/bash` (not `/bin/sh`) because files may use bashisms. No `set -e` and `|| true` after the lifecycle call — failures are non-fatal and don't abort the rest of the replay.

### Arch-specific stubs

| Command | Behaviour |
|---------|-----------|
| `vercmp` | Returns `0` (equal) — so version-guarded upgrade/migration blocks are *skipped* (the script reads the package as already current) |
| `groupadd`, `groupmod`, `useradd`, `usermod` | No-op |
| `systemd-sysusers`, `systemd-tmpfiles` | No-op |
| `install-info`, `xdg-icon-resource` | No-op |

--8<-- "_glossary.md"
