---
tags:
  - appendix
  - dependencies
  - packages
---

# Shared-library sonames

A **soname** (short for "shared object name") is the versioned filename a shared library publishes as its ABI contract. When an executable links against `libfoo.so.1`, the linker records `libfoo.so.1` — not the physical file (`libfoo.so.1.4.2`) — so the binary keeps working across minor library updates as long as the soname is unchanged.

## Where sonames live

The soname is stored inside the library itself as the ELF `DT_SONAME` dynamic tag, set at link time by `ld -soname=libfoo.so.1`. The library file on disk is typically the concrete upstream version, reached through a symlink chain:

```
libfoo.so        -> libfoo.so.1
libfoo.so.1      -> libfoo.so.1.4.2
libfoo.so.1.4.2  (real file, contains DT_SONAME=libfoo.so.1)
```

A major-version bump in the soname (`libfoo.so.1` → `libfoo.so.2`) signals an ABI break: binaries linked against `.so.1` will not run against `.so.2` and must be rebuilt. Minor upstream version bumps (`1.4.2` → `1.5.0`) that preserve ABI keep the same soname.

## How sonames surface in package metadata

Every supported distribution exposes sonames in its package index so the resolver can match "this binary needs `libfoo.so.1`" to "this package provides `libfoo.so.1`":

| Distro | Mechanism | Example |
|--------|-----------|---------|
| Debian / Ubuntu | Package name encodes the soname version | `libssl3`, `libc6` |
| Arch | `%PROVIDES%` with versioned soname | `libgomp.so=1-64` |
| Alpine | `so:` prefix in `p:` field | `so:libc.musl-x86_64.so.1` |
| RPM | Capability name in `<rpm:provides>` | `libc.so.6(GLIBC_2.17)(64bit)` |

flatroot treats the Arch, Alpine, and RPM forms as entries in the providers map (see [Virtual Packages](../packages/virtual.md)): a dependency naming such a soname resolves to whatever package provides it, through the same machinery as other virtual names. Debian and Ubuntu are different — the soname is baked into the *package name* (`libssl3`), so a soname dependency is an ordinary tier-1 package lookup, and DT_NEEDED resolution during the analyze linker pass falls back to the path index and library directories rather than the providers table.

## 64-bit vs 32-bit disambiguation

When a rootfs holds multiple architectures (multiarch: `--arch x86_64,i686`), both 32-bit and 64-bit copies of the same library coexist. Sonames alone don't disambiguate — `libgomp.so.1` is `libgomp.so.1` regardless of bitness. Arch attaches a *provides version* to carry architecture: `libgomp.so=1-64` for 64-bit, `libgomp.so=1-32` for 32-bit. flatroot's resolver checks the provides version (not the package version) against any constraint, so a dependency on `libgomp.so=1-64` binds to the 64-bit package and `=1-32` binds to the 32-bit one. Debian achieves the same through multiarch suffixed package names (`libfoo1:amd64`, `libfoo1:i386`).

## Further reading

- [`ld(1)` — `-soname` option](https://man7.org/linux/man-pages/man1/ld.1.html)
- [ELF specification — dynamic section](https://refspecs.linuxbase.org/elf/elf.pdf)
- [Virtual Packages](../packages/virtual.md) — how sonames plug into the providers mechanism.
- [ldconfig](ldconfig.md) — the runtime cache that lets binaries find a library by its soname.

--8<-- "_glossary.md"
