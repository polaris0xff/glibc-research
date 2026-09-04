---
tags:
  - reference
  - database
---

# Path Index Reference

The path index is a compact binary file flatroot writes alongside the SQLite database. It maps absolute file paths to the packages that own them. The resolver consults it when a dependency names a path the package index doesn't declare directly — RPM file-path requirements (`Requires: /usr/bin/python3`), Debian/Ubuntu Contents-arch entries, Arch/CachyOS `.files` data — and the analyze command consults it when a soname has no provider in the provides table. The `--type path` search space queries it directly (`query_glob_path`), and the `--type library` shipped-files leg queries it for libraries advertised by no `provides` entry (`query_glob`). Not queryable via `flatroot query`.

For why this is a separate binary file instead of another SQLite table, see the [Binary path index](database.md#binary-path-index) section of the Database Reference.

## Coverage

| Distro | Source data | Entries |
|--------|-------------|---------|
| Debian | `dists/<suite>/main/Contents-<arch>.gz` **and** `dists/<suite>/main/Contents-all.gz` (per-component) | populated |
| Ubuntu | `dists/<suite>/Contents-<arch>.gz` (suite-level, ingested once across components) | populated |
| Arch, CachyOS | `<repo>.files` (parallel to `<repo>.db`); Arch's are `.tar.gz`-compressed, CachyOS's v3 repos carry no suffix and are magic-sniffed | populated |
| CentOS, Fedora, AlmaLinux, Rocky, openSUSE | `repodata/.../filelists.xml` (`.gz`, `.xz`, `.zst`, `.bz2`, or uncompressed) | populated |
| Alpine | — | empty file |

Debian splits file ownership across two listings on its live and snapshot mirrors: `Contents-<arch>` carries architecture-specific packages only, and `Contents-all` carries every `Architecture: all` package (fonts, icons, zoneinfo, pure-Python libraries) — both are fetched and ingested, roughly doubling the Contents bytes a Debian index population downloads. The long-term archive (`archive.debian.org`) merges the two into the per-arch file on import and drops `Contents-all`; a listing definitively absent from every mirror is skipped with a `-v` warning, and a repo whose every declared listing is absent fails population. Ubuntu never splits — its per-arch suite-level Contents already folds `Architecture: all` packages in. See [Contents Indices](../appendix/packages/contents-indices.md) for the upstream layout in full.

Every distro writes a `.pathidx` file; Alpine's contains zero entries because Alpine resolves sonames purely via APKINDEX `p:` lines under the `so:` namespace and has no file-path dependency mechanism. Writing an empty file keeps `path_index::query` callers free of "does this distro have one?" branching — the read returns `None` for every path on Alpine.

## Location

```
~/.cache/flatroot/index/debian-bookworm-amd64.pathidx
~/.cache/flatroot/index/fedora-42-x86_64.pathidx
~/.cache/flatroot/index/arch-rolling-x86_64.pathidx
```

Written and refreshed together with the companion `.db` file: when the [database](database.md) is regenerated, any stale `.pathidx` from the previous run is deleted first.

## Binary format

The file has a 24-byte header, three sorted+deduplicated string pools (directories, filenames, package names), and a sorted array of entries.

| Region | Bytes | Description |
|--------|-------|-------------|
| Magic | 4 | `FRPI` (ASCII) |
| Version | 4 | `1` (u32, little-endian) |
| Dir count | 4 | Number of unique directory strings (u32 LE) |
| Filename count | 4 | Number of unique filename strings (u32 LE) |
| Pkg count | 4 | Number of unique package-name strings (u32 LE) |
| Entry count | 4 | Number of `(dir, filename, pkg)` triples (u32 LE) |
| Directory pool | var | Sorted alphabetically. Each entry: `u16` length + UTF-8 bytes |
| Filename pool | var | Same format as directory pool |
| Package-name pool | var | Same format as directory pool |
| Entries | entry_count × 12 | Sorted `(dir_id, filename_id, pkg_id)` triples, each field u32 LE |

The index is built by `PathIndexBuilder` in `src/path_index.rs` during index fetch and queried by `path_index::query` when the resolver or the analyze command needs to map a path or soname back to its providing package.

--8<-- "_glossary.md"
