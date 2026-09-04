---
tags:
  - explanation
  - distro
  - pipeline
---

# The distro abstraction

FlatRoot installs packages from a range of distributions — Debian, Ubuntu, Arch Linux, CachyOS, Alpine Linux, CentOS, Fedora, AlmaLinux, Rocky Linux, and openSUSE. They span several distinct package formats, several distinct index formats, more than one version-comparison algorithm, more than one post-install script convention, and a different mirror layout per distro. A naive approach would write each pipeline stage once per distro: a Debian resolver, an Arch resolver, an Alpine resolver, an RPM resolver. One copy of the BFS loop per distro family, one copy of the downloader, one copy of the extractor — every bug fixed in each, every feature added in each.

FlatRoot does not do that. Every pipeline stage is written once, against a universal representation. The distro-specific knowledge — where to fetch, how to parse, how to extract, how to version-compare, how to run scripts — lives behind a single trait that each backend implements. The pipeline calls the trait at the translation points and operates on packages everywhere else.

This page develops why the abstraction is shaped the way it is, where the translation points are, and what each backend must provide.

## The universal package

Before any pipeline stage can run, every package in the distro's index must be represented in a form the pipeline can operate on without knowing which distro it came from. The universal representation is a struct with these fields:

- A **name** — the package's canonical name in the index (`bash`, `libc6`, `glibc`).
- A **version** — the upstream version string, verbatim. The string itself carries no discriminator about which comparison algorithm applies; the active backend reports that separately through `Remote::version_compare`, and the resolver registers it as a SQLite collation so every per-source query sorts and compares versions under the matching rules.
- A list of **dependencies** — every hard requirement the package declares, parsed into a uniform group-of-alternatives shape regardless of the native syntax.
- A list of **provides** — every virtual name the package claims to satisfy, each with an optional version.
- A **checksum** — the archive hash, stored as the raw digest the index publishes. The label recorded with it comes from the package format via `Remote::checksum_type` (`sha256:` for the deb/RPM/pacman families, `q1-sha1:` for Alpine), not from the digest string. Download verification, by contrast, *does* infer the real algorithm from the digest's length (`Checksum::infer`), so an openSUSE 128-hex digest is verified as SHA-512 even though it is recorded under the `sha256:` label.
- A **filename** — the archive's per-source location, encoded as `<url_base>|<rel_path>` so `Remote::download_url` can reconstruct the absolute URL on demand regardless of which mirror in the source's fallback chain delivered the index.
- A **size** — the archive size in bytes, or zero when the index does not publish one.
- Beyond these, the struct also carries the soft and negative relationships (`recommends`, `suggests`, `conflicts`, `breaks`), the conditional forms (Alpine `install_if`, RPM `rich_deps`), and the descriptive fields the index publishes (`essential`, `priority`, `description`).

The target architecture deliberately is *not* a field on this struct. It lives one level up on the `SourceDistro` the resolver was built against — `Package` rows in the SQLite index are already filtered to the active architecture at parse time, and carrying the arch on every row would be redundant.

Every distro's index, regardless of its native format, can be parsed into a list of these structs. Debian's RFC 822 `Packages.gz` becomes the same shape as Arch's `desc` tarball becomes the same shape as Alpine's `APKINDEX` becomes the same shape as RPM's `primary.xml`. The parsing is distro-specific; the output is universal.

## Where the translation points are

The pipeline has four points where it crosses from the universal representation into distro-specific territory. At each point, the pipeline calls a method on the backend trait. Between these points, the pipeline operates on packages and knows nothing about which distro they came from.

**Index fetch.** The backend constructs the mirror URL, downloads the index, and parses it into packages. The pipeline says "give me the index for `debian:bookworm` on `x86_64`." The Debian backend knows that means `dists/bookworm/main/binary-amd64/Packages.gz` on the snapshot mirror. The Alpine backend knows that means `APKINDEX.tar.gz` from both `main` and `community` repos. The pipeline receives a list of packages and never touches a URL or a gzip stream.

**Version comparison.** When the resolver checks a version constraint, it asks the backend "does version A satisfy constraint B?" The backend delegates to the correct algorithm — dpkg for Debian, Ubuntu, Arch, CachyOS, and Alpine; RPM for CentOS, Fedora, AlmaLinux, Rocky, and openSUSE. The resolver does not know which algorithm is running; it only knows whether the constraint was satisfied.

**Extraction.** When the download completes, the backend unpacks the archive into the rootfs. The Debian backend handles the `ar`-wrapped `control.tar` + `data.tar` structure of `.deb` files. The RPM backend handles the lead + signature + header + cpio payload of `.rpm` files. The pipeline hands each backend an archive path and a destination directory; the backend writes files to disk.

**Post-install.** After extraction, the backend runs the package's post-install scripts inside the sandbox. The Debian backend runs `postinst configure ""` with a set of Debian-specific stubs. The Arch backend sources `.INSTALL` and calls `post_install`. The Alpine backend runs `.post-install` with the package name as `$1`. The pipeline says "run scripts for these packages"; the backend knows how.

Everything else — the BFS loop that walks dependencies, the download queue that fetches archives in parallel, the ldconfig pass that regenerates the shared library cache, the content-based hooks that rebuild font and icon caches — is written once, against packages.

## Why the abstraction stops at scripts

One natural extension of the abstraction would be to parse post-install scripts into a universal representation too — a list of declarative operations that the pipeline could execute without a shell. A Debian `postinst` that calls `update-alternatives --install` and an RPM `%post` that does the same thing would both become "register alternative `x` at priority `y` pointing at `z`." The sandbox would not need a shell at all.

FlatRoot does not do this. Post-install scripts are Turing-complete. A Debian `postinst` can contain a `for` loop over kernel versions, a `case` statement over architectures, a `sed` invocation that edits a config file in place. Parsing these into a declarative form would require either a shell interpreter or a whitelist of known patterns, and the whitelist would drift out of date with every new package release.

Instead, FlatRoot runs the scripts as the distro intended — inside a sandbox where privileged operations are neutralised by stub commands. The stubs are the translation point: `chown` becomes a no-op, `systemctl` becomes a no-op, `update-alternatives --install` becomes a real symlink creation. The scripts think they are running on a live system; the sandbox makes that belief harmless. See [The sandbox](the-sandbox.md) for how.

## Format sharing across distros

Several distros share the same package format, and the abstraction reflects this. The Debian backend and the Ubuntu backend both use the same `.deb` parser and extractor; they differ only in mirror URLs and the suites fetched. The Arch backend and the CachyOS backend both use the same pacman-format parser and extractor. The RPM-family distros — CentOS, Fedora, AlmaLinux, Rocky, and openSUSE — share the same `.rpm` parser and extractor.

The sharing is not a configuration file that lists "these distros use this parser." It is Rust code: the parser and extractor are generic over the package record type, and each backend instantiates them with its own mirror configuration. A new RPM-based distro needs a new backend that provides mirror URLs; it gets parsing, extraction, version comparison, and post-install script running for free.

## What the abstraction does not hide

The universal package representation is not lossless. Some distro-specific information is discarded during parsing because no downstream stage needs it:

- Debian's `Section:` field is not stored. (Its `Priority:` field, by contrast, *is* kept — ingested as the ordinal that ranks virtual-package providers.)
- Arch's `.BUILDINFO` and `.MTREE` files are not parsed.
- Alpine's signature stream is not verified after extraction.
- RPM's `rpm:entry` elements that match `rpmlib(...)` or `config(...)` patterns are filtered out.

What is kept is what the resolver needs to make decisions: names, versions, dependencies, provides, conflicts, and the archive location. Everything else is specific to the distro's own tooling and has no meaning in a cross-distro pipeline.

--8<-- "_glossary.md"
