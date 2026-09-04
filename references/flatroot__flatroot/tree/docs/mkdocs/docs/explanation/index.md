---
tags:
  - explanation
  - pipeline
  - distro
---

# Overview

A Linux root filesystem is a directory tree that looks like a running system: `/bin`, `/usr`, `/etc`, `/var`, the dynamic linker at `/lib/ld-linux.so`, the shared libraries every binary links against, the configuration files daemons read at startup. Container images ship one. Embedded devices boot from one. CI pipelines test against one. Every use case that runs Linux software in a controlled environment needs a rootfs.

Building one, however, has always required either root privileges or a host package manager that matches the target distribution. The standard tools are precise about this requirement: `debootstrap` needs root to create device nodes and run package postinst scripts. `dnf --installroot` needs root for the same reasons, and also needs `dnf` itself — which means the host must be an RPM-based distribution. `pacstrap` needs root and needs `pacman`. None of these tools can build a Debian rootfs on a Fedora host, or an Arch rootfs in a CI container that runs as an unprivileged user, or a CentOS rootfs on a machine that does not have `yum` installed.

The requirement is not arbitrary. Package managers assume they are running on the system they are managing. Post-install scripts call `chown`, create device nodes with `mknod`, register services with `systemctl`, and run `ldconfig` to update the shared library cache — all operations that either need root or need the package manager's own infrastructure to be present. The assumption is so deeply embedded that removing it means rethinking every stage of package installation from first principles.

These pages develop the mental model behind FlatRoot. They do not describe the pipeline phase by phase — that is the [Architecture](../architecture/index.md) section, which grounds each stage in the code. The pages here explain *why* the pipeline is shaped the way it is: what problem each piece solves, what alternatives were considered, and how the pieces fit together.

## The problem

A package installation tool that works without root and without a host package manager has to solve every problem the traditional tools solve by leaning on the host system. The subsections below cover the ones that drive FlatRoot's design.

**Index ingestion.** Every distribution publishes its package catalogue in a different format. Debian uses gzip-compressed RFC 822 text records. Arch uses gzipped tarballs of `desc` files. Alpine uses single-letter field prefixes in a multi-stream gzip. RPM distros use XML with a chain of indirection through `repomd.xml`. A tool that supports multiple distros must parse all of these into a single shape, or every downstream stage must speak four different formats.

**Dependency resolution.** A user types `flatroot install bash`. The package called `bash` declares it depends on `libc6`, which depends on `gcc-12-base`, which depends on `libc6` again — a cycle. `bash` also depends on `base-files`, which is a real package, but it also depends on `awk`, which is not — it is a virtual name provided by `gawk`, `mawk`, and `original-awk`. One of the providers must be chosen, deterministically, the same way every run. RPM packages can depend on file paths (`/bin/sh`), not package names. RPM and Alpine packages can declare conditional dependencies whose truth depends on what else is being installed. Turning the user's short list into the complete, ordered set of every package the rootfs needs is a graph problem with a long list of distinct complications.

**Sandboxed post-install.** Package post-install scripts assume they run as root on a live system. They call `chown`, `mknod`, `systemctl restart`, `update-initramfs`. In a rootfs built by an unprivileged user, every one of those calls either fails or does the wrong thing. The scripts must still run — some create essential symlinks, some generate caches, some register alternatives — but they must run in an environment where privileged operations are neutralised, not failed.

**Cross-distro generality.** The tool must handle Debian, Ubuntu, Arch, CachyOS, Alpine, CentOS, Fedora, AlmaLinux, Rocky, and openSUSE from a single codebase. Each distro has its own archive format (`.deb`, `.rpm`, `.pkg.tar.zst`, `.apk`), its own version comparison algorithm (dpkg vs. RPM), its own dependency syntax, its own post-install script conventions, its own mirror layout. The differences are not cosmetic. The RPM version comparison algorithm discards separators; the dpkg algorithm assigns them sort weights. A version constraint that is satisfiable under one algorithm can be unsatisfiable under the other.

## The core insight

All of these problems share a structure. Each one is a translation problem: translate a distro-specific input into a universal representation, operate on the representation with generic logic, translate the result back into distro-specific output. The translation points are the only parts that need to know which distro they are working with.

The universal representation is a package — a struct with a name, a version, a list of dependencies, a list of things it provides, a checksum, a download URL. (The target architecture is deliberately *not* a field on the struct: it lives on the `SourceDistro` the rows were parsed against, since the index is already filtered to one architecture.) Every distro's index, regardless of its native format, can be parsed into a list of these structs. Every downstream operation — resolving dependencies, downloading archives, extracting files, running post-install scripts — can be written once against the struct, without knowing which distro the struct came from.

The distro-specific knowledge lives behind a single trait. Each backend implements the per-distro capabilities the pipeline needs: where to fetch the index from, how to parse the native format into packages, how to extract the native archive format, and how to run post-install scripts. The pipeline calls these capabilities at the translation points and operates on packages everywhere else.

This is the central architectural idea FlatRoot is built around. The rest of these pages develop why each piece of the pipeline is shaped the way it is, what alternatives were considered, and how the pieces fit together.

## How these pages are organized

The pipeline runs as a sequence of stages — backend selection, index fetch, seed assembly, dependency resolution, download, extraction, post-install scripts, cache regeneration. These pages develop the why* behind the stages that involve design decisions — the ones where an alternative was considered and rejected, or where the obvious approach produces a broken rootfs.

1. **[The distro abstraction](the-distro-abstraction.md)** — the central idea the whole system is built around. Why one pipeline can serve ten distros, where the translation points are, and what each backend must provide.
2. **[The seed list](the-seed-list.md)** — why the user's command-line names are not enough to produce a working rootfs. The three sources that together form the starting point for resolution, and what goes wrong when any one is missing.
3. **[Dependency resolution](dependency-resolution.md)** — how FlatRoot grows a short list of names into the complete set of every package the rootfs needs. Why breadth-first, how cycles are broken, why install order matters, and what the resolver does not evaluate during the main walk.
4. **[Virtual names and providers](virtual-names-and-providers.md)** — the `awk` problem. Names in dependency declarations that are not real packages. The three-tier lookup that resolves them, why provider selection must be deterministic, and why the provides version is not the package version.
5. **[Conditional dependencies](conditional-dependencies.md)** — the two declaration forms that cannot be evaluated until the install set is known. Why the RPM `(appstream-data if PackageKit)` and Alpine `i: gdk-pixbuf librsvg` patterns need fixpoint loops, and how transactional sub-walks keep the install set consistent when an addition fails.
6. **[The sandbox](the-sandbox.md)** — why post-install scripts must run, why they cannot run as the host user, and how a user-namespace sandbox with stub commands makes them run safely without root.
7. **[Cache regeneration](cache-regeneration.md)** — why runtime caches must be rebuilt after every package is extracted. Why distro trigger systems do not work in a sandbox, and how a content-based approach replaces all of them with a single table of hooks.
8. **[The metadata layer](the-metadata-layer.md)** — what FlatRoot leaves behind in the rootfs. The `.flatroot/` directory as a declarative record of what was installed, where it came from, and which files it wrote. Why the manifest enables reinstallation, cross-distro protection, and downstream SBOM generation.
9. **[Cross-distro boundaries](cross-distro-boundaries.md)** — why FlatRoot refuses to install packages from a different source into an existing rootfs. The version-comparison, checksum, and ABI incompatibilities that make any source change unsafe, and why the only correct response is to refuse.
10. **[Multiarch](multiarch.md)** — why FlatRoot runs the pipeline once per architecture when building a multiarch rootfs. What is shared, what is separate, and how the post-install passes handle two architectures in one directory.

--8<-- "_glossary.md"
