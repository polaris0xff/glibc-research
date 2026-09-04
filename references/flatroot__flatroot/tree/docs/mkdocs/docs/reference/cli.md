---
tags:
  - reference
  - cli
---

# CLI Reference

The tree below summarises every accepted flag at a glance; the sections that follow document each command, its options, the `<distro>:<release>[@<date>]` source format, the target-architecture naming convention, and the environment variables that back each flag. For the reasoning behind each command's shape — why some take positionals and others take flags, why the noun-verb groups exist, why env vars are namespaced the way they are — see [CLI Architecture](../architecture/cli.md).

```
flatroot [GLOBAL OPTIONS]
├── --from <SOURCE>              Package source: <distro>:<release>[@<date>]
├── --arch <ARCH>                Target architecture (comma-separated multiarch on install only)
├── --http-retries <N>           Max HTTP retry attempts (default: 3)
├── -v, --verbose                Emit diagnostic warnings (default: off)
└── <COMMAND>
    ├── install -o <PATH> [OPTIONS] <PATTERNS>...
    │   ├── -o, --output <PATH>     Target rootfs directory (or $FLATROOT_ARG_INSTALL_OUTPUT)
    │   ├── --type <T>              package | library | path (default package) — pattern match space
    │   ├── --match <M>             any | all (default any) — combine path/library owner sets
    │   ├── --with <KIND>           recommends,suggests (default none) — soft-dep widening
    │   ├── --postinstall <PHASE>   ldconfig,scripts,hooks,none (default ldconfig,scripts,hooks)
    │   ├── --no-deps               Install only resolved packages (skip resolution)
    │   ├── -p, --parallel <N>      Parallel downloads (default: 4)
    │   └── --exclude <PKGS>        Comma-separated packages to exclude
    ├── export [--format <FMT>] [--tag <TAG>] <SRC_DIR> <OUTPUT>
    │   ├── --format <FMT>       oci | tar | dwarfs | sqfs — inferred from extension when unambiguous
    │   ├── -t, --tag <TAG>      Image tag (OCI only)
    │   ├── <SRC_DIR>            Source rootfs directory (positional)
    │   └── <OUTPUT>             Output file path (positional)
    ├── search [--type <T>] [--match <M>] [--format <FMT>] <PATTERNS>...
    │   ├── --type <T>           package | library | path (default package)
    │   ├── --match <M>          any | all (default any)
    │   └── --format <FMT>       plain | json (default plain)
    ├── query [--format <FMT>] [FILE]
    │   ├── --format <FMT>       plain | json (default plain)
    │   └── FILE                 .sql file; omitted or `-` reads from stdin
    ├── remote
    │   └── list [--format <FMT>]
    ├── release
    │   └── list [--format <FMT>]            (requires --from)
    └── analyze
        └── trace [OPTIONS] <PATTERNS>...
            ├── --type <T>              package | library | path (default package)
            ├── --match <M>             any | all (default any)
            ├── --format <FMT>          plain | json (default plain)
            ├── --strategy <S,S,...>    declared,linker (default both)
            └── --with <D,D,...>        recommends,suggests (default none)
```

## Commands

| Command | Description | Network |
|---------|-------------|---------|
| `install -o <PATH> [--type <T>] <PATTERNS>...` | Install packages and dependencies into the target directory, naming them by package, library, or installed file path | Yes |
| `export [--format <FMT>] [-t <TAG>] <SRC_DIR> <OUTPUT>` | Export a rootfs directory to a portable format | No |
| `search [--type <T>] <PATTERNS>...` | Match packages by glob pattern against names, library files, or full installed paths | Yes (first run) |
| `query [FILE]` | Run SQL against the package index database (stdin when FILE is omitted or `-`) | Yes (first run) |
| `remote list` | Show supported distribution backends | No |
| `release list` | Show available releases for a distro | Yes for mirror-walked distros; no for the static-table ones (Arch, CachyOS, CentOS, openSUSE) |
| `analyze trace [--type <T>] <PATTERNS>...` | Walk the declared and linker dependency closure of one or more seed packages | Yes (first run, plus archive downloads for the linker pass) |

## Global Options

| Option | Default | Description |
|--------|---------|-------------|
| `--from <SOURCE>` | *(required)* | Package source. Format: `<distro>:<release>[@<date>]`. |
| `--arch <ARCH>` | *(host)* | Target architecture (kernel naming). Single (`x86_64`); a comma-separated list (`x86_64,i686`) is accepted by `install` only — `search`, `query`, and `analyze trace` take a single architecture. |
| `--http-retries <N>` | `3` | Maximum retry attempts for failed HTTP requests. |
| `-v`, `--verbose` | `false` | Emit diagnostic warnings to stderr. Hidden by default when the failure does not affect the command's user-visible result. |

`--from` is required for all commands except `remote list` and `export`. There is no default.

---

## `install`

```
flatroot --from <SOURCE> install -o <PATH> [OPTIONS] <PATTERNS>...
```

Install packages and their dependencies into the target directory. At least one pattern is required; `--type` selects what the patterns name — packages (default), libraries, or installed file paths — and the resolved packages are installed with their closure. Patterns follow the same dialect as `search`: a pattern without wildcards matches exactly, and an explicit wildcard expanding to several packages installs them all. A pattern matching nothing fails the run.

### Install Options

| Option | Description |
|--------|-------------|
| `-o`, `--output <PATH>` | Target directory where packages are extracted. Created if it doesn't exist. Falls back to `FLATROOT_ARG_INSTALL_OUTPUT` env var. |
| `--type <T>` | Match space for the patterns. `package` (default) resolves against package names; `library` resolves each pattern to the packages owning matching libraries; `path` resolves each pattern to the packages shipping matching installed paths (spelled without the leading slash, e.g. `usr/bin/bash`) — unsupported on apk sources, which publish no file lists. Each non-trivial resolution is echoed on stderr (`Resolved 'bin/bash' -> bash`). |
| `--match <M>` | How patterns combine when each resolves to a set of owning packages (`--type library`/`--type path`). `any` (default) unions the owners; `all` intersects them — only a package owning a match for every pattern is seeded. Rejected with `--type package`. An empty `all` intersection fails naming the patterns. Falls back to `FLATROOT_ARG_INSTALL_MATCH`. |
| `--with <KIND>` | Comma-separated soft-dependency kinds to widen the install set with. Values: `recommends` (Debian `Recommends:`), `suggests` (Debian `Suggests:` and Arch `%OPTDEPENDS%`). Default: none. |
| `--postinstall <PHASE>` | Comma-separated post-install phases to run. Values: `ldconfig`, `scripts`, `hooks`, `none`. Default: `ldconfig,scripts,hooks`. The `none` sentinel skips every phase and cannot be combined with phase values. |
| `--no-deps` | Install only the resolved packages — skip dependency resolution and base packages. |
| `-p`, `--parallel <N>` | Number of parallel downloads (default: `4`). Set to `1` for sequential. |
| `--exclude <PKGS>` | Comma-separated list of packages to exclude from dependency resolution. |

Examples:

```bash
# By package name (default) — exact unless a wildcard is given
flatroot --from debian:bookworm install -o ./root bash
flatroot --from debian:bookworm install -o ./root 'php8.2-*'      # every match installs

# By library — the soname a program failed to load
flatroot --from debian:bookworm install -o ./root --type library 'libssl.so.3'

# By installed path — a command, an application binary, a config
flatroot --from debian:bookworm install -o ./root --type path 'bin/bash'
flatroot --from debian:bookworm install -o ./root --type path 'usr/bin/gimp'

# Disambiguate a path several packages ship — intersect on a second file
flatroot --from debian:bookworm install -o ./root --type path --match all usr/sbin/sendmail usr/sbin/postfix
```

By default, FlatRoot installs only hard dependencies plus:

- **Base packages** per distro (Debian/Ubuntu: `Essential: yes`, Arch/CachyOS: `filesystem glibc bash coreutils`, Alpine: `musl busybox alpine-baselayout busybox-binsh`, CentOS/Fedora/Alma/Rocky: `filesystem basesystem bash coreutils glibc glibc-common`, openSUSE: `filesystem bash coreutils glibc`)
- **Install-if triggers** (Alpine `i:` field — conditional packages like image loaders)

Soft dependencies (`Recommends:`, `Suggests:`, `%OPTDEPENDS%`) are **not** installed unless `--with` lists them.

Pipeline: fetch index → resolve dependencies → download (cached) → extract → ldconfig → postinst scripts → cache generation hooks.

For multiarch (`--arch x86_64,i686`), resolution, download, and extraction run once per architecture into the same directory; the manifest is written once after every architecture succeeds, and post-install (ldconfig, scripts, hooks) runs once at the end against the primary architecture — and only when something was extracted.

---

## `export`

```
flatroot export [--format <FORMAT>] [--tag <TAG>] <SRC_DIR> <OUTPUT>
```

Export a rootfs directory to a portable format. Both paths are positional and required. `--format` is inferred from the output extension when unambiguous (`.sqfs`, `.squashfs`, `.dwarfs`, `.tar.gz`) and required for `.tar` (plain tar and OCI both produce tar files).

### Export Options

| Option | Description |
|--------|-------------|
| `<SRC_DIR>` | Source rootfs directory (positional, required). |
| `<OUTPUT>` | Output file path (positional, required). |
| `--format <FORMAT>` | Output format: `oci`, `tar`, `dwarfs`, `sqfs`. Inferred from extension when unambiguous. |
| `-t`, `--tag <TAG>` | Image tag (e.g., `myapp:v1.0`). Required for OCI format, ignored for others. |

### Export Formats

| Format | Description | External binary |
|--------|-------------|-----------------|
| `oci` | [OCI container image](../appendix/formats/oci.md). Load with `docker load` or `podman load`. | None |
| `tar` | Compressed tar.gz archive. | None |
| `dwarfs` | [DwarFS](../appendix/formats/dwarfs.md) compressed read-only filesystem. | `mkdwarfs` |
| `sqfs` | [SquashFS](../appendix/formats/squashfs.md) compressed read-only filesystem. | `mksquashfs` |

---

## `search`

```
flatroot --from <SOURCE> search [--type <T>] [--format <FMT>] <PATTERNS>...
```

Match packages against one or more glob patterns and emit the matches as a structured stream. Case-insensitive. The package index is cached as a SQLite database (1-hour TTL).

### Search Options

| Option | Description |
|--------|-------------|
| `<PATTERNS>...` | One or more glob patterns (positional, at least one required). Patterns are unioned and deduped. |
| `--type <T>` | `package` (default) matches each pattern against package names; `library` matches against shared objects (`.so`, `.so.*`) and static archives (`.a`), resolving each match back to its owning package; `path` matches against full installed paths from the distribution's file lists, resolving each match back to the package that ships it. |
| `--match <M>` | `any` (default) renders every matched pair; `all` keeps only pairs whose package owns a match for every pattern — a preview of the `install --match all` intersection. Rejected with `--type package`. Falls back to `FLATROOT_ARG_SEARCH_MATCH`. |
| `--format <FMT>` | `plain` (default — dotted `KEY=VALUE`) or `json` (pretty-printed). |

Output is the project's plain dotted `KEY=VALUE` encoding, scoped under `search.<index>.*`. The `package` match space emits `description`, `name`, and `version` per entry; the `library` match space additionally emits the matched `library` filename; the `path` match space additionally emits the matched `path`. See the [Output section](../architecture/cli.md#output) of the CLI design page for the full spec.

Glob characters: `*` (any sequence), `?` (single character), `[abc]` (character class). Without glob characters, matches the exact package or library name.

The `path` match space matches patterns against the full installed path spelled without the leading slash (`usr/bin/bash`, `etc/ssl/*`), the same spelling the tool prints paths in. It is unsupported on apk sources: Alpine publishes no file lists, and the request is refused with an error rather than answered empty.

Examples:

```bash
# Package-name match space (default)
flatroot --from debian:bookworm search firefox-esr    # exact match
flatroot --from debian:bookworm search 'firefox*'     # packages starting with firefox
flatroot --from debian:bookworm search '*gtk*'        # packages containing gtk
flatroot --from debian:bookworm search 'python3.??'   # python3.11, python3.12, etc.
flatroot --from debian:bookworm search 'libssl*' 'libcrypto*'   # multiple patterns unioned

# Library match space — resolve a soname back to its owning package
flatroot --from debian:bookworm search --type library 'libssl.so.3'
flatroot --from debian:bookworm search --type library 'lib*.so.*'

# Path match space — resolve an installed path back to its owning package
flatroot --from debian:bookworm search --type path 'bin/bash'
flatroot --from fedora:42 search --type path 'etc/ssl/*'

# Preview an intersection without installing — only the package owning both paths
flatroot --from debian:bookworm search --type path --match all usr/sbin/sendmail usr/sbin/postfix
```

---

## `query`

```
flatroot --from <SOURCE> query [FILE]
```

Run a SQL query against the package index database. The connection is opened read-write with no statement-kind guard, so query is read-only only by convention — a mutating statement (`DELETE`, `UPDATE`) does change the cached index until its TTL lapses. Output is the project's plain dotted `KEY=VALUE` encoding, scoped under `query.<index>.*`: each result row is one indexed entry whose object keys are the SQL column names. SQL `NULL` columns are omitted from the plain form (per the encoding's "absent = no line" idiom) and emitted as JSON `null` under `--format=json`. See the [Output section](../architecture/cli.md#output) of the CLI design page for the full spec. SQL is read from the positional `FILE`; when `FILE` is omitted or given as `-`, SQL is read from stdin.

The database schema covers `packages`, `dependencies`, `provides`, `install_if`, and `rich_deps`.

Examples:

```bash
# SQL from a file
flatroot --from debian:bookworm query analysis.sql

# SQL piped from a command (bare invocation reads stdin)
echo "SELECT count(*) FROM packages" | flatroot --from debian:bookworm query

# Heredoc via explicit `-`
flatroot --from debian:bookworm query - <<'SQL'
SELECT name FROM packages WHERE essential = 1;
SQL

# Find what depends on libssl3 (join through packages to resolve the parent name)
flatroot --from debian:bookworm query <<<"SELECT DISTINCT p.name FROM dependencies d JOIN packages p ON p.id = d.package_id WHERE d.name = 'libssl3' AND d.kind = 'depends'"
```

---

## `remote list`

```
flatroot remote list
```

Show supported distribution backends. No `--from` flag needed, no network. Output is the project's plain dotted `KEY=VALUE` encoding, scoped under `remote.list.<index>.*` with `format`, `name`, and `status` per backend; JSON via `--format=json`.

---

## `release list`

```
flatroot --from <DISTRO> release list
```

Fetch available releases for the distro. Mirror-walked distros (Debian, Ubuntu, Alpine, Fedora, AlmaLinux, Rocky) query their mirrors to discover releases dynamically; Arch, CachyOS, CentOS, and openSUSE return a fixed built-in table with no network request. Only the distro prefix is needed (e.g., `--from debian`, not `--from debian:bookworm`). Output is the project's plain dotted `KEY=VALUE` encoding, scoped under `release.list.<index>.*`; the field set per release is distro-specific (Debian carries `codename`/`mirror`, Alpine carries `mirror`/`name`/`type`, CentOS's static table carries `codename`/`mirror`/`suite`/`version`, etc.). JSON via `--format=json`.

`release list` is intentionally lightweight — it fetches only the directory listing from each mirror plus, for the deb shape, one small `Release` file per codename. No package indices are downloaded. ABI information (glibc / musl version per release) is not part of this output; resolve it on a specific source with `flatroot --from <distro>:<release> search libc6` (or `musl`).

---

## `analyze trace`

```
flatroot --from <SOURCE> analyze trace [OPTIONS] <PATTERNS>...
```

Walk the full dependency closure of one or more seed packages and render every package the seeds pull in. Each positional pattern is glob-matched against the index according to `--type`; the resulting package set is unioned and deduped to form the seed list (or, under `--match all`, intersected — only a package owning a match for every pattern seeds the trace), and the trace runs against that set. Two passes run by default. **Pass 1** (declared) walks every `Depends` edge from each seed, and resolves RPM rich-deps and Alpine install-if triggers unconditionally — those fixpoints always run, not via `--with`; `--with` widens the walk along the soft `Recommends`/`Suggests` edges only. **Pass 2** (linker) downloads and extracts each visited package, walks its ELF binaries' `DT_NEEDED` sonames, resolves each soname to a providing package via the active backend's rules, and follows that closure transitively. Useful for surfacing under-declarations: packages whose binaries link against something the index does not declare a dependency on.

### Trace Options

| Option | Description |
|--------|-------------|
| `<PATTERNS>...` | One or more glob patterns (positional, at least one required). Patterns are unioned and deduped before the trace runs. A pattern matching zero packages contributes nothing — an aggregate empty match exits 0 with no stdout. |
| `--type <T>` | Match space for the patterns. `package` (default) matches each pattern against package names; `library` matches against shared objects (`.so`, `.so.*`) and static archives (`.a`), resolving each match back to its owning package; `path` matches against full installed paths spelled without the leading slash (`usr/bin/bash`), resolving each match back to the package that ships it — unsupported on apk sources, which publish no file lists. |
| `--match <M>` | `any` (default) unions the seed owners; `all` intersects them — only a package owning a match for every pattern seeds the trace. Rejected with `--type package`. Falls back to `FLATROOT_ARG_ANALYZE_TRACE_MATCH`. |
| `--format <FORMAT>` | `plain` (default — dotted `KEY=VALUE`) or `json` (pretty-printed). The two are bijective — the same data in two syntaxes. |
| `--strategy <S,S,...>` | Comma-separated trace strategies. `declared` walks the index's stated dependency edges; `linker` extracts each package's ELF binaries and follows their `DT_NEEDED` sonames. Defaults to both. At least one strategy is required — an empty `--strategy=` is rejected at parse time. |
| `--with <D,D,...>` | Comma-separated soft-dep kinds to widen the declared closure with. `recommends` follows Debian's `Recommends:` edges; `suggests` follows `Suggests:` and Arch's `%OPTDEPENDS%`. Defaults to none. Has no effect on the linker strategy. |

Each output entry carries a `reason` field with one of four classifications:

| Reason | Meaning |
|--------|---------|
| `target` | A seed the user's pattern matched. |
| `declared` | Declared strategy only. Pulled in by the resolver, but no inspected binary linked against it. |
| `linker` | Linker strategy only. **Under-declaration**: a seed's closure links against this package transitively, but the index does not declare a dependency on it. |
| `declared_linker` | Both. The typical case for runtime-linked dependencies. |

A trailing stderr line reports any sonames the linker strategy named that the index could not resolve to a package — actionable diagnostic about index completeness. Per-binary unresolved entries land on each consumer's `unresolved_sonames` field.

---

## Source Format

```
<distro>:<release>[@<YYYY-MM-DD>]
```

| Example | Description |
|---------|-------------|
| `debian:bookworm` | Current Bookworm from `deb.debian.org` |
| `debian:buster` | EOL Buster (falls back to `archive.debian.org`) |
| `debian:bookworm@2024-06-15` | Bookworm snapshot from `snapshot.debian.org` |
| `arch:rolling` | Current Arch from `geo.mirror.pkgbuild.com` |
| `arch:rolling@2024-06-15` | Arch snapshot from `archive.archlinux.org` |
| `arch:multilib` | Arch with the `multilib` repo enabled (32-bit support) |
| `alpine:v3.21` | Alpine 3.21 from `dl-cdn.alpinelinux.org` |
| `alpine:edge` | Alpine edge (rolling) |
| `ubuntu:noble` | Current Ubuntu LTS from `archive.ubuntu.com` |
| `ubuntu:noble@2024-06-15` | Ubuntu snapshot from `snapshot.ubuntu.com` |
| `centos:7` | CentOS 7.9 from `vault.centos.org` |
| `centos:8` | CentOS 8.5 from `vault.centos.org` |
| `centos:stream9` | CentOS Stream 9 from `mirror.stream.centos.org` |
| `fedora:42` | Fedora 42 from `dl.fedoraproject.org` |
| `fedora:rawhide` | Fedora Rawhide (rolling development) |
| `alma:9` | AlmaLinux 9 from `repo.almalinux.org` |
| `alma:8` | AlmaLinux 8 from `repo.almalinux.org` |
| `rocky:9` | Rocky Linux 9 from `dl.rockylinux.org` |
| `rocky:8` | Rocky Linux 8 from `dl.rockylinux.org` |
| `opensuse:tumbleweed` | openSUSE Tumbleweed (rolling) from `download.opensuse.org` |
| `opensuse:15.6` | openSUSE Leap 15.6 from `download.opensuse.org` |
| `cachyos:rolling` | CachyOS (rolling, Arch-based, x86-64-v3) from `mirror.cachyos.org` |

## Architecture Format

FlatRoot uses Linux kernel architecture names (`uname -m`). Defaults to the host architecture.

| Architecture | `--arch` value |
|-------------|---------------|
| 64-bit x86 | `x86_64` |
| 32-bit x86 | `i686` |
| 64-bit ARM | `aarch64` |
| 32-bit ARM | `armv7l` |
| 64-bit RISC-V | `riscv64` |

??? note

    Multi: `x86_64,i686` — runs resolution, download, and extraction per architecture into the same directory (post-install runs once at the end). Libraries go into architecture-specific paths (`/usr/lib/x86_64-linux-gnu/`, `/usr/lib/i386-linux-gnu/`).  Each backend maps to its native naming internally (e.g., `x86_64` → `amd64` for Debian, `i686` → `x86` for Alpine).

## Environment Variables

Every CLI flag has a `FLATROOT_ARG_*` fallback — a flag passed on the command line wins over the matching env var, which wins over the built-in default. See [Environment](environment.md) for the full inventory, the namespace conventions, and the variables FlatRoot sets in the post-install sandbox.

--8<-- "_glossary.md"
