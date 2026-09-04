---
tags:
  - cli
  - configuration
---

# Environment Variables

This page covers the environment variables used by FlatRoot, which are split into internal behavior and CLI fallback flags.

## Variables Read by FlatRoot

### Flag fallbacks

Every CLI flag has a `FLATROOT_ARG_*` fallback.

- Globals follow `FLATROOT_ARG_<FLAG>`
- Subcommand flags follow `FLATROOT_ARG_<COMMAND>_<FLAG>`
- Flags on a sub-subcommand follow `FLATROOT_ARG_<GROUP>_<VERB>_<FLAG>`.
- Boolean flags accept lowercase `true` or `false` only.
- The CLI flag wins when both are set.

#### Global flags

| Variable | Fallback for | Values |
|----------|--------------|--------|
| `FLATROOT_ARG_FROM` | `--from <SOURCE>` | `<distro>:<release>[@YYYY-MM-DD]` — distros listed by `flatroot remote list`; the `@date` snapshot pin is accepted only by debian, ubuntu, and arch (the other seven refuse it) |
| `FLATROOT_ARG_ARCH` | `--arch <ARCH>` | `x86_64`, `aarch64`, `armv7l`, `i686` (alias `x86`), `riscv64`; defaults to host arch. A comma-separated multiarch list works for `install` only — `search`, `query`, and `analyze trace` reject a comma list |
| `FLATROOT_ARG_HTTP_RETRIES` | `--http-retries <N>` | non-negative integer (default `3`) |
| `FLATROOT_ARG_VERBOSE` | `-v`, `--verbose` | `true`, `false` (default `false`) |

#### `install`

| Variable | Fallback for | Values |
|----------|--------------|--------|
| `FLATROOT_ARG_INSTALL_OUTPUT` | `-o`, `--output <PATH>` | filesystem path |
| `FLATROOT_ARG_INSTALL_WITH` | `--with <KIND>` (comma-separated) | comma-separated subset of `recommends`, `suggests` (default empty) |
| `FLATROOT_ARG_INSTALL_POSTINSTALL` | `--postinstall <PHASE>` (comma-separated) | comma-separated subset of `ldconfig`, `scripts`, `hooks`; or `none` (mutually exclusive with the others); default `ldconfig,scripts,hooks` |
| `FLATROOT_ARG_INSTALL_NO_DEPS` | `--no-deps` | `true`, `false` (default `false`) |
| `FLATROOT_ARG_INSTALL_PARALLEL` | `-p`, `--parallel <N>` | positive integer (default `4`) |
| `FLATROOT_ARG_INSTALL_EXCLUDE` | `--exclude <PKGS>` | comma-separated package names |
| `FLATROOT_ARG_INSTALL_TYPE` | `--type` | `package` (default), `library`, `path` |
| `FLATROOT_ARG_INSTALL_MATCH` | `--match` | `any` (default), `all` |

#### `search`

| Variable | Fallback for | Values |
|----------|--------------|--------|
| `FLATROOT_ARG_SEARCH_TYPE` | `--type` | `package` (default), `library`, `path` |
| `FLATROOT_ARG_SEARCH_MATCH` | `--match` | `any` (default), `all` |
| `FLATROOT_ARG_SEARCH_FORMAT` | `--format` | `plain` (default), `json` |

#### `query`

| Variable | Fallback for | Values |
|----------|--------------|--------|
| `FLATROOT_ARG_QUERY_FORMAT` | `--format` | `plain` (default), `json` |

#### `export`

| Variable | Fallback for | Values |
|----------|--------------|--------|
| `FLATROOT_ARG_EXPORT_FORMAT` | `--format` | `oci`, `tar`, `dwarfs`, `sqfs` |
| `FLATROOT_ARG_EXPORT_TAG` | `-t`, `--tag` | OCI-style `name:tag` string |

#### `remote list` and `release list`

| Variable | Fallback for | Values |
|----------|--------------|--------|
| `FLATROOT_ARG_REMOTE_LIST_FORMAT` | `remote list --format` | `plain` (default), `json` |
| `FLATROOT_ARG_RELEASE_LIST_FORMAT` | `release list --format` | `plain` (default), `json` |

#### `analyze trace`

| Variable | Fallback for | Values |
|----------|--------------|--------|
| `FLATROOT_ARG_ANALYZE_TRACE_TYPE` | `--type` | `package` (default), `library`, `path` |
| `FLATROOT_ARG_ANALYZE_TRACE_MATCH` | `--match` | `any` (default), `all` |
| `FLATROOT_ARG_ANALYZE_TRACE_FORMAT` | `--format` | `plain` (default), `json` |
| `FLATROOT_ARG_ANALYZE_TRACE_STRATEGY` | `--strategy` (comma-separated) | comma-separated subset of `declared`, `linker`; default `declared,linker` |
| `FLATROOT_ARG_ANALYZE_TRACE_WITH` | `--with` (comma-separated) | comma-separated subset of `recommends`, `suggests`; default empty |

### Internal configuration

Variables in the bare `FLATROOT_*` namespace configure subsystems that have no CLI counterpart.

| Variable | Role | Values | Precedence |
|----------|------|--------|------------|
| `FLATROOT_CACHE_HOME` | Cache root directory | filesystem path | Beats `XDG_CACHE_HOME` and the `~/.cache` default |
| `XDG_CACHE_HOME` | XDG-standard cache root; cache lives at `$XDG_CACHE_HOME/flatroot` | filesystem path | Beats the `~/.cache` default |
| `HOME` | Used to construct the default cache path `$HOME/.cache/flatroot` when neither override is set | filesystem path | Falls back to `/tmp` if `HOME` itself is unset |

See [Cache](cache.md) for the full cache-resolution semantics, on-disk layout, and TTLs.

## Examples

```bash
# Redirect the cache to a CI-friendly path.
export FLATROOT_CACHE_HOME=/build/cache
flatroot --from debian:bookworm install -o ./root bash

# Set the install output via env so multiple commands inherit it.
export FLATROOT_ARG_INSTALL_OUTPUT=/build/rootfs
flatroot --from debian:bookworm install bash python3
flatroot --from debian:bookworm install --with=recommends gcc

# Pin a CI pipeline to one source via env, no flags needed.
export FLATROOT_ARG_FROM=debian:bookworm
export FLATROOT_ARG_INSTALL_POSTINSTALL=none
flatroot install -o ./root bash

# Honour XDG without an explicit override.
export XDG_CACHE_HOME=$HOME/.local/cache
flatroot --from arch:rolling install -o ./arch firefox
```

--8<-- "_glossary.md"
