---
tags:
  - reference
  - cache
---

# Cache Reference

FlatRoot caches both package indices and downloaded package archives locally to avoid redundant network requests across runs.

## Cache Location

| Priority | Source | Resulting Path |
|----------|--------|----------------|
| 1 | `$FLATROOT_CACHE_HOME` | Exact value of the variable |
| 2 | `$XDG_CACHE_HOME` | `$XDG_CACHE_HOME/flatroot` |
| 3 | Default | `~/.cache/flatroot` |

The directory is created automatically if it doesn't exist.

## Directory Structure

The cache has three sections: per-remote package archives, parsed index databases, and raw index file cache.

```
~/.cache/flatroot/
├── debian/bookworm/amd64/              # package archives per remote
│   ├── bash_5.2.15-2+b10_amd64.deb
│   └── libc6_2.36-9+deb12u13_amd64.deb
├── arch/rolling/x86_64/
│   └── firefox-128.0-1-x86_64.pkg.tar.zst
├── fedora/42/x86_64/
│   └── bash-5.2.37-1.fc42.x86_64.rpm
└── index/
    ├── debian-bookworm-amd64.db        # SQLite database (parsed index)
    ├── arch-rolling-x86_64.db
    ├── 2bb772d4...                     # raw index file cache (SHA-256 hash of URL)
    └── a3f18bc2...
```

Package archives are stored under `{cache_key}/{basename}`, where `cache_key` is `{distro}/{release}/{distro_arch}` and `basename` is extracted from the package's filename in the index.

Index databases are SQLite files stored under `index/{distro}-{release}-{arch}.db`.

Raw index files (the compressed metadata downloaded from mirrors) are also cached under `index/` with SHA-256 hashes as filenames. These serve as the intermediate data from which the SQLite databases are built.

## Index Caching

The SQLite index database is cached with a 1-hour TTL. Within that window, repeated commands (e.g., multiple `search` queries or an `install` after a `search`) reuse the cached database without hitting the network. After 1 hour, the raw index is re-fetched and the database is rebuilt.

## Package Archive Caching

Package archives are cached permanently (no TTL). Each archive is verified by checksum before reuse:

1. If the file exists and its checksum matches the index value, the package is served from cache.
2. If the file exists but the checksum doesn't match, it is re-downloaded.
3. If the file doesn't exist, it is downloaded.
4. If the index published no checksum for the package, it is always re-downloaded — a cached copy can never be proven genuine.

--8<-- "_glossary.md"
