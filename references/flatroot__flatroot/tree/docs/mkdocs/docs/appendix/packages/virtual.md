---
tags:
  - appendix
  - dependencies
---

# Virtual Packages

A virtual package is a name that doesn't correspond to a real package in the repository but is "provided" by one or more real packages. When a package depends on a virtual name, the resolver picks a real provider that satisfies the dependency.

## How they work

A package declares that it provides a virtual name via the `Provides` field in the index. For example, both `mawk` and `gawk` declare that they provide `awk`. When another package has `Depends: awk`, the resolver looks up the providers of `awk` and picks one.

FlatRoot stores every provides relationship in the SQLite index's `provides` table during the index-population stage that runs before any resolution. Each virtual name maps to all real packages that provide it. The resolver queries that table on demand during the walk, ranked by the distribution's `Priority` ordinal first and by provider name only as a tiebreak — deterministic, and faithful to the distribution's own preference.

## Examples by distro

### Debian / Ubuntu

```
Package: mawk
Provides: awk

Package: gawk
Provides: awk
```

A dependency on `awk` resolves to `mawk`, not the alphabetically-first `gawk`: Debian gives `mawk` the more-preferred `Priority`, and the resolver honours that ordinal ahead of the name.

### Arch Linux

```
%PROVIDES%
sh
libgomp.so=1-64
```

Arch uses versioned provides for shared library [sonames](../system/soname.md). `libgomp.so=1-64` means this package provides the 64-bit version of `libgomp.so`. A 32-bit package would provide `libgomp.so=1-32`. The resolver uses the provides version (not the package version) when checking version constraints, which is how it distinguishes between architectures.

### Alpine Linux

```
p:so:libfoo.so.1=1.0 cmd:busybox
```

Alpine namespaces virtual packages with prefixes: `so:` for shared libraries, `cmd:` for commands, `pc:` for pkg-config. These are naming conventions — the resolver treats them as regular names.

### RPM (CentOS, Fedora, AlmaLinux, Rocky, openSUSE)

```xml
<rpm:provides>
    <rpm:entry name="/bin/bash"/>
    <rpm:entry name="libc.so.6(GLIBC_2.17)(64bit)"/>
    <rpm:entry name="bash" flags="EQ" ver="4.2.46" rel="34.el7"/>
</rpm:provides>
```

RPM packages can provide file paths (`/bin/bash`), shared library sonames with symbol versions (`libc.so.6(GLIBC_2.17)(64bit)`), and capability names. File path ownership is stored in the [binary path index](../../reference/path_index.md), built from `filelists.xml`. The resolver consults the path index only as its last resort for a name: it tries a literal package first, then the provides map (where `primary.xml`'s own file-path provides like `/bin/bash` usually answer), and reaches the path index last.

## Provides versions

Each provides entry can include a version, which is separate from the package version. When a dependency has a version constraint on a virtual name, the resolver checks the provides version, not the package version.

For example, on current Arch, `libgomp` provides `libgomp.so` at version `1-64` and `lib32-gcc-libs` provides it at `1-32`. A dependency on `libgomp.so=1-64` checks `satisfies("1-64", "=1-64")` using the provides version — which selects the 64-bit provider while `=1-32` selects the 32-bit one.

--8<-- "_glossary.md"
