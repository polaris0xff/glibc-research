---
tags:
  - how-to
  - query
  - cli
  - dependencies
---

# Query Packages

This guide shows how to discover which packages are available in a distribution before installing any of them. FlatRoot's read-only operations cover listing the supported distributions, listing the releases of one, searching a distribution's index by package name, library, or installed path, and running structured SQL against the index database. None of them touch the host or an existing rootfs — they only read (and cache) the distribution's metadata. For the database layout if you want to write your own queries, see [Database Reference](../reference/database.md).

## List supported distributions

Show all supported distribution backends and their source format. No network access required.

```bash
flatroot remote list
```

## List available releases

Fetch and display available releases for a distribution. Mirror-walked distros (Debian, Ubuntu, Alpine, Fedora, AlmaLinux, Rocky) query their mirrors dynamically (directory listing + per-codename `Release` file for the deb shape; no package indices are downloaded); Arch, CachyOS, CentOS, and openSUSE return a fixed built-in table with no network access.

```bash
# Debian releases (current + archived)
flatroot --from debian release list

# Ubuntu releases
flatroot --from ubuntu release list

# Fedora releases (current + archived)
flatroot --from fedora release list

# CentOS releases
flatroot --from centos release list

# AlmaLinux releases
flatroot --from alma release list

# Rocky Linux releases
flatroot --from rocky release list

# openSUSE releases
flatroot --from opensuse release list

# CachyOS releases
flatroot --from cachyos release list

# Arch Linux (static table — reports `rolling` and `multilib` rows)
flatroot --from arch release list

# Alpine Linux (scrapes the mirror for channels)
flatroot --from alpine release list
```

## Search packages

Search for packages by name using glob patterns. Case-insensitive. The package index is cached as a SQLite database — repeated searches are instant.

```bash
# Exact match
flatroot --from debian:bookworm search firefox-esr

# Glob: packages starting with a prefix
flatroot --from debian:bookworm search 'firefox*'

# Glob: single-character wildcard
flatroot --from debian:bookworm search 'python3.??'

# Glob: packages containing a substring
flatroot --from arch:rolling search '*gtk*'

# Search in a specific architecture
flatroot --from debian:bookworm --arch i686 search wine
```

Glob characters: `*` (any sequence), `?` (single character), `[abc]` (character class), `[!abc]` (negated class), `[a-z]` (range), and `{a,b}` (alternation) — the full `globset` dialect.

## SQL queries

Run arbitrary SQL against the package index database for advanced analysis.

```bash
# Inline query (here-string for one-liners)
flatroot --from debian:bookworm query <<<"SELECT count(*) FROM packages"

# List essential packages (heredoc)
flatroot --from debian:bookworm query <<'SQL'
SELECT name, version FROM packages WHERE essential = 1;
SQL

# From a .sql file
flatroot --from debian:bookworm query analysis.sql

# From stdin (pipe)
echo "SELECT name, size FROM packages ORDER BY size DESC LIMIT 10" | flatroot --from debian:bookworm query

# Find what depends on a specific package (join to resolve package name)
flatroot --from debian:bookworm query <<<"SELECT DISTINCT p.name FROM dependencies d JOIN packages p ON p.id = d.package_id WHERE d.name = 'libssl3' AND d.kind = 'depends'"

# Find providers of a virtual package
flatroot --from debian:bookworm query <<<"SELECT p.name FROM provides pr JOIN packages p ON p.id = pr.package_id WHERE pr.name = 'awk'"
```

Output is the project's plain dotted `KEY=VALUE` encoding by default, scoped under `query.<index>.<sql_column>=<value>`. SQL `NULL` columns are omitted from the plain form (per the encoding's "absent = no line" idiom). Pass `--format=json` for the pretty-printed JSON form, where `NULL` columns are emitted as `null` literals — so the two encodings are *not* bijective for query (plain drops the NULLs that JSON keeps). The database is opened read-write with no statement-kind guard: query is read-only only by convention, and a mutating statement (`DELETE`, `UPDATE`) really does change the cached index until its TTL lapses. See the [Output section](../architecture/cli.md#output) of the CLI design page for the full encoding spec.

## Force index refresh

`search` and `query` consult the cached SQLite database, which honours a 1-hour TTL. Within that window, repeated commands against the same source reuse the cache with no network access. For the mirror-walked distros, `release list` re-fetches the upstream directory listing on every run; for Arch, CachyOS, CentOS, and openSUSE it returns a built-in table with no network at all. `remote list` never makes a network call. To force a fresh fetch, delete the relevant index file — but note this only rebuilds the SQLite database from the raw-index HTTP cache, which lives under the same `index/` directory and is itself served within its own 1-hour TTL; a genuine re-fetch from the mirror requires that raw cache to have expired too:

```bash
rm ~/.cache/flatroot/index/debian-bookworm-amd64.db
```

Only the `.pathidx` file is rebuilt alongside the `.db` on the next command; the raw-index cache files are reused until their own TTL lapses. See [Cache reference](../reference/cache.md) for the full cache layout.

For the full schema — tables, column types, indexes, example joins, and the binary path index — see [Database Reference](../reference/database.md).

--8<-- "_glossary.md"
