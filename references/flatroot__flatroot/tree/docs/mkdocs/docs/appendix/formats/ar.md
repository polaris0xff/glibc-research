---
tags:
  - appendix
  - formats
---

# ar (Unix archive)

`ar` is one of the oldest Unix archive formats — originally used to pack object files into static libraries (`libfoo.a`). It is not a compression format: it just concatenates members with a small fixed-size header per file. Debian and Ubuntu reuse it as the outer wrapper for `.deb` packages.

## Layout

An `ar` archive starts with the 8-byte magic `!<arch>\n` followed by zero or more members. Each member has a 60-byte ASCII header:

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0 | 16 | Filename | Space-padded |
| 16 | 12 | Modification time | Decimal Unix timestamp |
| 28 | 6 | Owner UID | Decimal |
| 34 | 6 | Group GID | Decimal |
| 40 | 8 | Mode | Octal |
| 48 | 10 | Size | Decimal bytes |
| 58 | 2 | End marker | `` `\n `` |

The file content follows, padded to an even byte boundary. No compression; no deduplication; no central index.

## How .deb uses it

A `.deb` archive contains exactly three members, in fixed order:

```
debian-binary       (format version, always "2.0\n")
control.tar.*       (package metadata + maintainer scripts)
data.tar.*          (the payload tree to extract)
```

The `.tar.*` members are compressed: `.tar.xz` (modern, most common), `.tar.gz` (older releases), `.tar.bz2` (wheezy era), or `.tar.zst` (experimental). The outer `ar` wrapper is uncompressed. flatroot detects the compression from the member name and decompresses accordingly.

## Why ar and not tar

Historical choice. `.deb` predates tar's ubiquity in the Linux packaging world, and the three-member layout is fixed enough that the minimalism of `ar` is a feature rather than a limitation. Moving `.deb` to a single tar today would break every existing tool for no functional gain.

## Further reading

- [ar(5) man page](https://man7.org/linux/man-pages/man5/ar.5.html)
- [Debian Policy — binary package format](https://www.debian.org/doc/debian-policy/ch-binary.html)
- [Debian distribution notes](../../explanation/the-distro-abstraction.md)

--8<-- "_glossary.md"
