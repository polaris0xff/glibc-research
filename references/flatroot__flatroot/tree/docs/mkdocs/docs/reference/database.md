---
tags:
  - database
  - reference
---

# Database Reference

FlatRoot's package index is a SQLite database at `~/.cache/flatroot/index/` (or under `$FLATROOT_CACHE_HOME` / `$XDG_CACHE_HOME/flatroot` when those are set). This page documents the schema, indexes, binary path-index format, and common queries. For the design rationale (why SQLite, cache lifecycle, why the path index is a separate binary file), see the [Cache Reference](cache.md) and the [Path Index Reference](path_index.md).

## Location

```
~/.cache/flatroot/index/debian-bookworm-amd64.db
~/.cache/flatroot/index/arch-rolling-x86_64.db
~/.cache/flatroot/index/fedora-42-x86_64.db
```

The database can be queried with `flatroot query` or directly with any SQLite tool:

```bash
sqlite3 ~/.cache/flatroot/index/debian-bookworm-amd64.db
```

## Schema

The SQLite database contains the tables `packages`, `dependencies`, `provides`, `install_if`, and `rich_deps`. All non-`packages` tables reference `packages.id` as a foreign key — look up or join through it when you need a package name.

### Packages { #packages }

Core package metadata. One row per `(name, version)` pair — multiple versions of the same package can coexist, and a name lookup returns the **highest version** under the family's `version_compare` collation (not the latest-inserted row — the queries rank with `ROW_NUMBER() OVER (PARTITION BY name ORDER BY version COLLATE version_compare DESC)` precisely because `MAX(id)` would pick the latest-walked repo's row instead).

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER PRIMARY KEY | Auto-generated row identifier referenced by other tables |
| `name` | TEXT NOT NULL | Package name (indexed via `idx_packages_name`) |
| `version` | TEXT NOT NULL | Package version |
| `description` | TEXT NOT NULL DEFAULT '' | Human-readable summary |
| `filename` | TEXT NOT NULL | Archive location as `<mirror base URL>\|<repository-relative path>`; the download path splits on `\|` to rebuild the absolute URL and rejects any unprefixed value as an index-population invariant violation |
| `size` | INTEGER NOT NULL DEFAULT 0 | Download size in bytes |
| `checksum` | TEXT NOT NULL DEFAULT '' | SHA-256 hex (most distros), SHA-512 hex (openSUSE), Q1+base64 SHA-1 (Alpine) |
| `essential` | INTEGER NOT NULL DEFAULT 0 | 1 if the package is marked Essential (Debian/Ubuntu only) |
| `priority` | INTEGER | Debian `Priority:` field as an ordinal (lower = preferred); drives provider selection. NULL when the index publishes no priority |

### Dependencies { #dependencies }

All dependency types in a single table, distinguished by `kind`. Alternatives within a dependency group share the same `group_id` and are ordered by `position`.

| Column | Type | Description |
|--------|------|-------------|
| `package_id` | INTEGER NOT NULL → packages(id) | Package that declares this dependency |
| `kind` | TEXT NOT NULL | One of: `depends`, `recommends`, `suggests`, `conflicts`, `breaks` |
| `group_id` | INTEGER NOT NULL | Groups alternatives together (e.g., `debconf | cdebconf` share a group) |
| `position` | INTEGER NOT NULL | Order within a group (0 = preferred alternative) |
| `name` | TEXT NOT NULL | The dependency target — a package name, a virtual name, or an absolute file path (RPM `Requires: /usr/bin/sh`), which the resolver routes to the binary path index |
| `version_constraint` | TEXT | Optional constraint (e.g., `>= 2.36`, `<< 3.0`) |

**Example:** The Debian dependency `Depends: debconf (>= 0.5) | cdebconf, bash` declared by the package with `id = 42` produces:

| package_id | kind | group_id | position | name | version_constraint |
|-----------|------|----------|----------|------|--------------------|
| 42 | depends | 0 | 0 | debconf | >= 0.5 |
| 42 | depends | 0 | 1 | cdebconf | NULL |
| 42 | depends | 1 | 0 | bash | NULL |

### Virtual Packages { #provides }

Virtual package mappings. A package can provide one or more virtual names, optionally with a version.

| Column | Type | Description |
|--------|------|-------------|
| `package_id` | INTEGER NOT NULL → packages(id) | Package that provides the virtual name |
| `name` | TEXT NOT NULL | Virtual package name (e.g., `awk`, `libfoo.so`, `cmd:busybox`) |
| `version` | TEXT | Optional provides version (separate from the package version) |

### Conditional Installation { #install_if }

Alpine-only conditional triggers. A package with install-if conditions is automatically installed when ALL its trigger packages are present (AND semantics).

| Column | Type | Description |
|--------|------|-------------|
| `package_id` | INTEGER NOT NULL → packages(id) | Package to auto-install |
| `trigger` | TEXT NOT NULL | Package name that must be present |

### Rich Dependencies { #rich_deps }

RPM conditional dependency expressions stored as a JSON-serialized [AST](../appendix/dependencies/ast.md). Only entries containing `if` or `unless` operators are stored — non-conditional rich deps are flattened into the `dependencies` table at parse time.

| Column | Type | Description |
|--------|------|-------------|
| `package_id` | INTEGER NOT NULL → packages(id) | Package that declares the rich dependency |
| `ast` | TEXT NOT NULL | JSON-serialized AST of the conditional expression |

The AST uses these node types: `Pkg`, `And`, `Or`, `If`, `Unless`, `With`, `Without`.

**Example AST** for `(appstream-data if PackageKit)`:

```json
{"If":{"payload":{"Pkg":{"name":"appstream-data","version_constraint":null}},"condition":{"Pkg":{"name":"PackageKit","version_constraint":null}}}}
```

## Indexes

A database index is a side table holding a sorted copy of one or more columns from another table, with each value paired with a pointer back to the row it came from — the database's equivalent of the alphabetical index at the back of a book. Without one, a query like `WHERE name = 'bash'` makes SQLite scan every row in the table looking for matches. With `idx_packages_name` in place, SQLite binary-searches the sorted list of names, finds `bash` in a handful of comparisons, follows the pointer to the matching row, and stops. Multi-column indexes (like `idx_deps_pkg` on `(package_id, kind)`) work the same way but sort by the listed columns in order, so they accelerate queries that filter on both.

| Index | Table | Columns | Purpose |
|-------|-------|---------|---------|
| `idx_packages_name` | packages | (name) | Fast lookup of a package by name |
| `idx_deps_pkg` | dependencies | (package_id, kind) | Look up a package's dependencies by kind |
| `idx_deps_name` | dependencies | (name) | Reverse dependency lookup ("who depends on X") |
| `idx_provides_name` | provides | (name) | Find providers of a virtual name |
| `idx_install_if_pkg` | install_if | (package_id) | Find install-if triggers for a package |
| `idx_rich_deps_pkg` | rich_deps | (package_id) | Find rich deps for a package |

## Binary path index

The SQLite schema above covers package metadata and dependency relationships. Path-to-package mappings — what `/`-prefixed dependencies (RPM `Requires: /usr/bin/python3` and equivalents), the analyze command's soname-by-path fallback, the `--type library` shipped-files leg, and the whole `--type path` search space need — live in a companion binary file alongside the `.db`. When a lookup needs them, flatroot queries the binary index directly instead of running a SQL query. The file is written for every supported distro and refreshed together with the `.db`. Alpine's `.pathidx` is syntactically valid but contains zero entries, because Alpine expresses sonames through APKINDEX `p:` lines under the `so:` namespace and has no file-path dependency mechanism. See [Path Index](path_index.md) for the on-disk format.

## Example queries

Queries against dependency tables must either use `package_id` directly or join through `packages` to resolve the name.

```sql
-- Package count
SELECT count(DISTINCT name) FROM packages;

-- Essential packages
SELECT DISTINCT name, version FROM packages WHERE essential = 1 ORDER BY name;

-- Dependencies of a package (join to resolve package name)
SELECT d.name, d.version_constraint FROM dependencies d
JOIN packages p ON p.id = d.package_id
WHERE p.name = 'bash' AND d.kind = 'depends'
ORDER BY d.group_id, d.position;

-- Reverse dependencies: distinct packages that depend on libssl3
SELECT DISTINCT p.name FROM dependencies d
JOIN packages p ON p.id = d.package_id
WHERE d.name = 'libssl3' AND d.kind = 'depends';

-- Find all providers of a virtual package
SELECT p.name, prov.version FROM provides prov
JOIN packages p ON p.id = prov.package_id
WHERE prov.name = 'awk'
ORDER BY p.name;

-- Packages with the most dependencies
SELECT p.name, count(*) AS dep_count FROM dependencies d
JOIN packages p ON p.id = d.package_id
WHERE d.kind = 'depends'
GROUP BY p.name
ORDER BY dep_count DESC LIMIT 20;

-- Alpine install-if triggers, grouped by package
SELECT p.name, group_concat(i.trigger) FROM install_if i
JOIN packages p ON p.id = i.package_id
GROUP BY p.name;

-- Largest packages
SELECT name, size FROM packages ORDER BY size DESC LIMIT 10;

-- Packages with rich deps
SELECT DISTINCT p.name, r.ast FROM rich_deps r
JOIN packages p ON p.id = r.package_id;
```

--8<-- "_glossary.md"
