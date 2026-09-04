---
tags:
  - reference
  - formats
  - rpm
  - centos
  - fedora
---

# rpm

Distros: CentOS, Fedora, AlmaLinux, Rocky, openSUSE. Parser: `src/pkg/rpm/` (format, repodata, container, richdep, isa).

## Archive

| Property | Value |
|----------|-------|
| Extension | `.rpm` |
| Container | Custom binary (lead + headers + payload) |
| Magic | `0xEDABEEDB` (lead) |
| Header magic | `0x8EADE801` (signature and main headers) |
| Payload | Compressed [cpio](../../appendix/formats/cpio.md) (SVR4/newc) |
| Payload compression | xz (modern), gzip (older), zstd (newer Fedora/CentOS Stream) |

Structure:

1. **Lead** (96 bytes) — magic, format version, architecture.
2. **Signature header** — integrity data.
3. **Main header** — package metadata and scriptlets as tagged entries.
4. **Payload** — compressed cpio archive of the filesystem tree.

### Scriptlet tags

| Tag | Value | Content |
|-----|-------|---------|
| 1024 | `POSTIN` | Post-install script (NUL-terminated) |
| 1086 | `POSTINPROG` | Interpreter: `/bin/sh` or `<lua>` |

Shell scriptlets are extracted and executed. Lua scriptlets are skipped — they call into RPM's `posix.*` and `rpm.*` extension libraries unavailable outside the `rpm` binary.

## Index

| Property | Value |
|----------|-------|
| Format | XML, discovered through `repomd.xml` indirection |
| Entry point | `{mirror}/{path_dir_repo}/repodata/repomd.xml` |
| Package metadata | `primary.xml` (`.gz`, `.xz`, `.zst`, `.bz2`, or uncompressed) |
| File listings | `filelists.xml` (populates the [binary path index](../path_index.md)) |

`repomd.xml` lists `data` elements with checksummed hrefs to the other metadata files. FlatRoot follows the hrefs — paths are not hardcoded.

### Dependency entry shape

```xml
<rpm:requires>
    <rpm:entry name="libc.so.6(GLIBC_2.15)(64bit)"/>
    <rpm:entry name="bash" flags="GE" epoch="0" ver="4.0"/>
</rpm:requires>
```

### Filtered name patterns

| Pattern | Reason |
|---------|--------|
| `rpmlib(...)` | Internal RPM capability markers |
| `config(...)` | Configuration file tracking |
| `rtld(...)` | Runtime linker internals |
| `this-is-only-for-build-envs` | Build-environment-only marker, never a real dependency |

XML entity escaping (`&lt;`, `&gt;`) in `name` attributes is handled during parsing.

### Weak dependencies

RPM publishes four weak-dep sections — `<rpm:recommends>`, `<rpm:suggests>`, `<rpm:supplements>`, `<rpm:enhances>` — but FlatRoot does not parse them. `--with=recommends` and `--with=suggests` have no effect on RPM sources.

## Versioning

| Property | Value |
|----------|-------|
| Shape | `[epoch:]ver-rel` |
| Algorithm | [RPM](../../appendix/versioning/version-comparison.md) |
| Epoch | Separate integer (omitted when 0) |
| Separator handling | Segment delimiters (`.`, `-`, `_`) — skipped to find segment boundaries, not stripped: `1.10` is the two segments `1` and `10`, distinct from `110` |
| Tilde | Sorts before everything (same as dpkg) |

Constraint operators are encoded as flag names in the index and converted during parsing:

| Flag | Symbol |
|------|--------|
| `EQ` | `=` |
| `GE` | `>=` |
| `GT` | `>>` |
| `LE` | `<=` |
| `LT` | `<<` |

## Dependencies

| Property | Value |
|----------|-------|
| Entry element | `<rpm:entry>` within `<rpm:requires>` |
| Alternative syntax | None — each entry is a single required package |
| Constraint encoding | `flags`, `ver`, `rel` attributes on `<rpm:entry>` (`epoch` is not read) |

Sonames and capability names create implicit alternatives through the provides map. File-path dependencies are different: they resolve through the binary path index as the resolver's last resort, not through the provides map.

### Rich dependencies

RPM-family distros (primarily Fedora) can carry boolean expressions in entry names:

```xml
<rpm:entry name="(appstream-data if PackageKit)"/>
<rpm:entry name="(pkgconfig(foo) or pkgconfig(bar))"/>
```

The operator set covers `and`, `or`, `if`, `unless`, `with`, and `without`. Expressions without `if`/`unless` are flattened into plain dependencies at parse time. Expressions with conditionals are stored in the `rich_deps` table and evaluated in a fixpoint loop after the main BFS completes. See the [Rich-Dep AST](../../appendix/dependencies/ast.md) for the full grammar.

### File-path dependencies

RPM packages can depend on absolute paths:

```xml
<rpm:entry name="/usr/bin/python3"/>
```

These resolve through the binary path index. The path index is built from `filelists.xml` during index ingestion.

## Checksum

| Property | Value |
|----------|-------|
| Algorithm | SHA-256 (SHA-512 digest for openSUSE) |
| Prefix | `sha256:` (the recorded label for the whole RPM family, openSUSE included) |
| Scope | Full archive |

The recorded prefix is `sha256:` for every RPM-family distro; openSUSE's digest is a SHA-512 hash carried under that label, and download verification infers the real algorithm from the digest length rather than the prefix.

## Post-install

| Property | Value |
|----------|-------|
| Script source | Tag 1024 (`POSTIN`) in main header |
| Extracted to | `.flatroot/scripts/<pkg>:<arch>/postinst` |
| Invocation | `/bin/sh -e <script> configure ""` (shell); skipped (Lua) |
| Runner | `src/postinstall/strategy.rs` routes all five RPM-family distros to `PostInstall::Debian`, forwarding `stubs_install`/`scripts_run` to `DebPostInstall` (`src/postinstall/debian.rs`). There are no per-distro `centos.rs`/`fedora.rs`/… files. |

RPM scriptlets reuse the Debian post-install runner (same stub PATH, same `DPKG_MAINTSCRIPT_*` environment, same `configure ""` positional args). The runner pre-filters `libc6` and any Lua-only scriptlet before invocation.

--8<-- "_glossary.md"
