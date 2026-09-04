<!--
  Maintainer note: each term is repeated for every grammatical inflection
  it appears in (singular/plural, sometimes adjective forms) because the
  mkdocs `*[term]:` abbreviation extension matches exact strings, not
  lemmas. Adding a new term means adding every form that appears in the
  prose. Removing a duplicate "for cleanup" silently breaks the tooltip
  for that form.

  This is the single unified glossary for the entire flatroot
  documentation. Every page includes it via `--8<-- "_glossary.md"`.

  Terms whose meaning shifts with context (state, phase, configuration,
  method, entry point, wrapper, runtime, …) are intentionally absent —
  defining them once would mislead readers who meet them in another
  surrounding.
-->

<!-- ────────────────────────────────────────────────────────────── -->
<!-- A                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[ABI]: Application Binary Interface — the contract between a library and its callers covering symbol names, calling conventions, struct layouts, and version tags. A library with a newer ABI exports everything an older one does plus more.
*[ABIs]: Application Binary Interfaces.
*[absolute path]: A filesystem path starting with `/`, identifying a file relative to the root of the filesystem hierarchy irrespective of the process's current working directory.
*[absolute paths]: Filesystem paths starting with `/`.
*[abstract syntax tree]: A tree-shaped data structure representing the parsed form of an expression. Leaves hold operands; internal nodes hold operators. flatroot uses one to store RPM rich dependencies whose truth value cannot be known at parse time.
*[abstract syntax trees]: Tree-shaped data structures representing parsed expressions.
*[alternative]: A choice in a dependency declaration. "pkg-a | pkg-b" means either of these will satisfy the slot; the resolver picks one.
*[alternatives]: Choices in a dependency declaration. "pkg-a | pkg-b" means either will satisfy the slot; the resolver picks one.
*[apk]: Alpine Linux's package format and the name of its on-host package manager. An `.apk` file is three concatenated gzip streams — signature, control, and data — that flatroot parses and extracts without invoking the apk tool.
*[APKINDEX]: Alpine's package index file format. Each record uses single-letter field prefixes (`P:` name, `V:` version, `D:` depends, `p:` provides, `i:` install-if).
*[ar]: A Unix archive format that concatenates members with a small fixed-size header per file. Debian and Ubuntu use it as the outer wrapper for `.deb` packages.
*[architecture]: A CPU family and ABI convention (x86_64, aarch64, i686, armv7l, riscv64). flatroot's `--arch` flag selects the target architecture for the rootfs; per-distro internal names (Debian's `amd64`, Alpine's `x86`) are mapped from this name.
*[architectures]: CPU families and ABI conventions.
*[AST]: Abstract Syntax Tree — see *abstract syntax tree*. flatroot stores RPM rich-dep ASTs in the `rich_deps.ast` column of the package index.
*[atomic]: All-or-nothing. The operation either completes fully or has no observable effect.
*[atomically]: All-or-nothing. Either every change happens or none do.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- B                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[base packages]: A fixed per-distro set of packages flatroot always includes in the seed (libc, shell, coreutils, directory-layout) to make the rootfs functional. Hardcoded for non-Debian distros; populated dynamically from `Essential: yes` on Debian and Ubuntu.
*[basename]: The last component of a filesystem path — the filename after the final `/`. `bash_5.2.15-2+b10_amd64.deb` is the basename of `pool/main/b/bash/bash_5.2.15-2+b10_amd64.deb`.
*[BFS]: Breadth-first search — an algorithm that walks a graph level by level: root first, then everything connected to the root, then their neighbours, and so on.
*[bind mount]: A kernel mechanism that exposes a file or directory at a second path, identical to the original. Used inside mount namespaces to construct sandbox filesystem views.
*[bind mounts]: Kernel mechanisms exposing files or directories at second paths.
*[blob]: A content-addressed file inside an OCI image, named by the SHA-256 of its content. Config JSON, layer tarballs, and manifests are stored as blobs under `blobs/sha256/`.
*[blobs]: Content-addressed files inside an OCI image, named by the SHA-256 of their content.
*[breaks]: A Debian dependency-shaped declaration with the opposite meaning of *depends*: "this package cannot coexist with package P (in some version range)." flatroot records breaks and warns when both halves end up together; it does not block the install.
*[bubblewrap]: A small unprivileged sandbox tool using Linux user and mount namespaces. The canonical rootless sandbox; the conceptual model for flatroot's own user-namespace sandbox in `src/sandbox.rs`.
*[bwrap]: bubblewrap's CLI command. Constructs a sandbox from bind-mount and namespace flags.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- C                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[candidate]: A real package that could satisfy a dependency name during resolution — either the package itself if the name is real, every package that provides the name if it is virtual, or the path-owner if the name is a file path.
*[candidates]: Real packages that could satisfy a dependency name during resolution.
*[CAP_SYS_ADMIN]: A Linux capability granting the right to perform mount, namespace, and similar administrative operations. User namespaces grant a "fake" CAP_SYS_ADMIN scoped to namespace-internal resources only.
*[CAP_SYS_CHROOT]: The Linux capability required to call `chroot(2)`. Reserved for root in the host namespace, which is why ordinary `chroot` invocations require `sudo`.
*[capability]: A Linux fine-grained privilege bit (CAP_NET_ADMIN, CAP_SYS_ADMIN, CAP_SYS_CHROOT, …) the kernel can grant independently of full root. Also: RPM's name for a virtual provides — `libGL.so.1` and `bash(x86-64)` are capabilities a package may declare it provides.
*[capabilities]: Linux's fine-grained privilege bits — the kernel splits "root" into separate capabilities that can be granted independently. Also: RPM's name for the entries inside `<rpm:provides>`.
*[checksum]: A cryptographic hash of a file's bytes, used to verify integrity. flatroot records SHA-256 for most distros, SHA-512 for openSUSE, and a Q1-prefixed base64 SHA-1 for Alpine.
*[chroot]: A Linux system call (and the `chroot(1)` command) that changes the apparent root directory of a process and its descendants. Requires `CAP_SYS_CHROOT`, so ordinary use needs `sudo`. Provides filesystem isolation only, not a sandbox.
*[CLI]: Command-Line Interface — the surface a tool exposes through commands, flags, positionals, and environment variables.
*[CLONE_NEWNS]: The flag to `clone()`/`unshare()` requesting a fresh mount namespace.
*[CLONE_NEWUSER]: The flag to `clone()`/`unshare()` requesting a fresh user namespace.
*[closure]: The complete set of packages reachable from a starting set by following dependencies. Computing this is the resolver's job.
*[conflict]: A dependency-shaped declaration with opposite meaning: "this package cannot coexist with package P (in some version range)." flatroot records conflicts and warns when both halves end up together; it does not block the install.
*[conflicts]: Dependency-shaped declarations expressing incompatibility.
*[content-addressed]: A storage layout where every object is named by the cryptographic hash of its content. Identical content deduplicates automatically; any tampering produces a different name. OCI's `blobs/sha256/` layout is content-addressed.
*[cpio]: A Unix archive format like tar — a stream of (header, filename, content) triples with no central index. RPM uses cpio (SVR4/newc variant) as the payload format inside every `.rpm` file.
*[CycloneDX]: An SBOM standard maintained by the OWASP Foundation, focused on vulnerability tracking and supply-chain attestation. Serializations: JSON (canonical), XML, Protobuf.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- D                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[deb]: Debian's binary package format and the file extension for it. A `.deb` file is an `ar` archive holding `debian-binary`, `control.tar.*`, and `data.tar.*`. Ubuntu uses the same format.
*[dpkg]: Debian's low-level package management tool — the binary that installs `.deb` files on a live system. flatroot does not invoke `dpkg`; it parses the index and extracts archives directly.
*[dpkg-vercmp]: Debian's version comparison algorithm, defined by `dpkg --compare-versions`. Alternates non-digit and digit passes, with tilde sorting before everything. Used by flatroot for Debian and Ubuntu version comparisons (Arch and CachyOS use libalpm `vercmp`, Alpine uses apk-tools' own ordering).
*[DT_NEEDED]: An ELF dynamic-section entry naming a soname the binary references. The dynamic linker resolves each DT_NEEDED at load time; the analyze command's linker pass walks them to discover runtime dependencies.
*[DT_SONAME]: An ELF dynamic-section entry declaring the soname a shared library publishes for itself. The dynamic linker records it at load time; DT_NEEDED references match against it.
*[DwarFS]: Deduplicating Warp-speed Advanced Read-only File System — a compressed read-only filesystem optimised for extreme compression while preserving fast random access. flatroot exports rootfs trees to DwarFS via `mkdwarfs`.
*[dynamic linker]: The userspace program (`ld-linux` on glibc systems) the kernel runs to load a dynamically-linked binary's libraries before its own code starts.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- E                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[edge]: One recorded parent-to-child relationship in the closure, with metadata about the dependency kind and the alternative picked.
*[edges]: Recorded parent-to-child relationships in the closure, each with metadata about the dependency kind and the alternative picked.
*[edge list]: Every parent-to-child relationship the walker recorded during the walk, with metadata.
*[ELF]: Executable and Linkable Format — Linux's standard binary format for executables, shared libraries, and object files.
*[epoch]: An optional integer prefix on a version string (`2:1.0`), separated by `:`, that dominates every other component during version comparison. `2:1.0` is always newer than `1:999.0`. Used when an upstream renumbering would otherwise break ordering.
*[Essential]: A `Key: Value` field Debian and Ubuntu use to mark packages required for a functional system (`dash`, `coreutils`, `dpkg`, …). flatroot scans the index for `Essential: yes` and adds every such package to the seed list.
*[Essential package]: A Debian/Ubuntu package marked `Essential: yes` in the index. Always added to the seed automatically.
*[Essential packages]: Debian/Ubuntu packages marked `Essential: yes` in the index.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- F                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[fallback]: A path taken when the normal mechanism produces no result. flatroot's mirror lists are ordered fallback chains — a current Debian release finds `Packages.gz` at `deb.debian.org` immediately; an EOL release 404s and the fetch falls through to `archive.debian.org`.
*[FHS]: Filesystem Hierarchy Standard — the convention placing libraries under `/lib`, `/usr/lib`, configuration under `/etc`, and so on. Shared across every mainstream Linux distribution and the shape every rootfs flatroot builds.
*[fixpoint]: A loop that repeats until iterating again produces no change. Used for conditional dependencies because pulling in a package can flip another rule's outcome.
*[fixpoints]: Loops that repeat until iterating again produces no change. The resolver has two — rich-dep and install-if.
*[fixpoint convergence]: The point at which a fixpoint loop has stopped producing change. Termination is guaranteed because the install set only grows and the index is finite, so eventually a full pass adds nothing.
*[flat list]: A sequence with no nesting — every entry is a single package name, not a sub-list. Contrast with a tree.
*[flat ordered list]: An ordered sequence with no nesting — every entry is a single package name, not a sub-list. Contrast with a tree.
*[FUSE]: Filesystem in Userspace — a kernel interface letting userspace programs implement filesystems without kernel-mode code. DwarFS uses it at mount time.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- G                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[glob]: A pattern-matching syntax for file names: `*` (any sequence), `?` (single character), `[abc]` (character class). flatroot's `search` and `analyze trace` accept glob patterns to match package names.
*[glob pattern]: A string using `*`, `?`, and `[]` wildcards to match a set of names rather than a literal one.
*[glibc]: The GNU C Library — the standard C library used by every mainstream Linux distribution other than Alpine. Ships `libc.so.6`, `ld-linux*.so.*`, `ldconfig`, and the NSS plugin loader.
*[glibc family]: The set of libraries shipped from the same glibc source release: `libc.so.6`, `libpthread.so.0`, `libdl.so.2`, `libm.so.6`, `libnsl.so.1`, `libutil.so.1`, `librt.so.1`, `libnss_*`. They have to come from the same release; mixing them corrupts private symbol channels.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- H                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[hard dependency]: A required dependency. Missing one is an error; the resolver never silently skips a required dependency.
*[hard dependencies]: Required dependencies. Missing any one is an error; the resolver never silently skips a required dependency.
*[hook]: A distro-agnostic command flatroot runs after extraction to regenerate a runtime cache (fonts, icons, MIME types, GSettings schemas, …). Hooks fire when the corresponding binary exists in the rootfs.
*[hooks]: Distro-agnostic commands run after extraction to regenerate runtime caches.
*[host]: The user's machine — the system where the flatroot binary runs. Distinct from the target rootfs, which lives in a directory on the host but is structured as an independent Linux installation.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- I                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[init system]: The first userspace process the kernel starts (PID 1). systemd, OpenRC, and SysVinit are init systems; flatroot's per-flavour stub commands no-op the init-system calls a maintainer script might make (the Debian/RPM flavour stubs `systemctl`, the Alpine flavour stubs `rc-update`/`rc-service`/`openrc`) because no init runs inside a rootfs context. The Arch flavour stubs no service-manager command at all.
*[install order]: The flat ordered list of every package in the closure, in the resolver's BFS visit order — a package precedes its own dependencies, and layout-creating packages land first only because the base and Essential seeds are queued ahead of the user's requests. What the install command iterates over.
*[install set]: The accumulating collection of packages the walker has committed during one resolution. Starts empty, grows as the resolver runs, and is consumed when the resolver returns. The conditional phases read from this set to decide which packages to fold in.
*[install-if]: An Alpine-only conditional dependency form. A package's `i: A B` means "install me when both A and B are in the closure."
*[IPC]: Inter-Process Communication. `bwrap --unshare-ipc` creates a fresh IPC namespace so System V IPC objects and POSIX message queues do not cross the sandbox boundary.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- J                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[JSON]: JavaScript Object Notation — a lightweight data-interchange format. flatroot's `--format=json` output and the OCI image manifest are JSON.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- K                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[kernel]: The Linux kernel — the privileged core of the operating system that manages hardware, enforces security boundaries, and exposes the system-call interface. The piece flatroot never produces; rootfs trees plug into the host's kernel via chroot, bubblewrap, or a container runtime.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- L                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[ld.so.cache]: The precomputed cache at `/etc/ld.so.cache` that the dynamic linker consults to find shared libraries at program startup. flatroot's post-install pass runs `ldconfig` to rebuild it.
*[ldconfig]: The glibc utility that builds `/etc/ld.so.cache`. flatroot runs it inside the user-namespace sandbox after extraction so that post-install scripts can link against newly-installed libraries.
*[LZMA]: A high-ratio compression algorithm (Lempel-Ziv-Markov chain). Used as one of the per-section compression choices inside DwarFS and as a payload compression in modern RPMs (`xz`).

<!-- ────────────────────────────────────────────────────────────── -->
<!-- M                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[magic]: A fixed byte signature at a known offset in a file that identifies its format. `!<arch>\n` at offset 0 identifies an ar archive; `070701` identifies an SVR4/newc cpio header; `FRPI` identifies a flatroot path index.
*[magic number]: A fixed integer or byte sequence at a known offset that identifies a file format.
*[magic bytes]: A fixed byte sequence at a known offset that identifies a file format.
*[manifest]: The dpkg-style record file at `.flatroot/manifest` summarising a built rootfs (flatroot version, sources, architectures, package count). The OCI image format also calls its image-level descriptor a manifest — surrounding context disambiguates.
*[mirror]: A server that hosts a distribution's package archives. flatroot fetches indices and packages from one or more mirrors selected from a per-distro fallback chain.
*[mount namespace]: A kernel feature giving a process its own private mount table. Mounts inside the namespace are visible only to processes in that namespace.
*[mount namespaces]: Kernel features giving processes private mount tables.
*[multiarch]: Debian's filesystem convention putting per-architecture libraries under `/usr/lib/<tuple>/` — for example `/usr/lib/x86_64-linux-gnu/`. Distinct from the RHEL convention of putting them all in `/usr/lib64/`.
*[multilib]: A host or rootfs with libraries for more than one CPU architecture installed (typically x86_64 + i386). `--from arch:multilib` enables Arch's multilib repository for 32-bit compatibility packages.
*[musl]: A small, MIT-licensed C library used by Alpine Linux instead of glibc. Does not ship `ldconfig`; resolves shared libraries through a different search strategy. Binaries built against musl are not ABI-compatible with glibc.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- N                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[name resolution]: The translation step that turns a name in a dependency declaration into a real installable package. A name can already be a real package, a virtual name several packages provide, or a file path some package owns.
*[namespace]: A kernel feature giving a process a private view of some system resource — mount table, UID/GID mapping, network interfaces, etc.
*[namespaces]: Kernel features giving processes private views of system resources.
*[node]: One entry in a tree or graph. Each package in the resolver's internal dependency graph is a node (the analyze command's output is a flat, name-sorted list, not a rendered tree).
*[nodes]: Entries in a tree or graph. Each package in the resolver's internal dependency graph is a node (the analyze command's output is a flat, name-sorted list, not a rendered tree).

<!-- ────────────────────────────────────────────────────────────── -->
<!-- O                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[OCI]: Open Container Initiative — a standard for container images and runtimes. `flatroot export --format oci` produces an OCI image archive consumable by `docker load`, `podman load`, or `skopeo`.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- P                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[pacman]: Arch Linux's package format and on-host package manager. `.pkg.tar.zst` files are zstd-compressed tarballs containing metadata files (`.PKGINFO`, `.BUILDINFO`, `.MTREE`, `.INSTALL`) at the root.
*[package index]: The database of every package the active distribution publishes. Built once per `--from` source; cached locally afterwards.
*[payload]: The file content portion of a package, distinct from its metadata headers. RPM's payload is a cpio archive; deb's payload is `data.tar.*`. The conditional rich-dep `(A if B)` also calls A the payload — the thing that gets installed when the condition holds.
*[pinning]: Fixing a distribution source to a specific date so repeated runs produce the same rootfs. Spelled `<distro>:<release>@YYYY-MM-DD` on the command line.
*[pivot_root]: A Linux system call that switches the calling process's root filesystem to a new one. Used by bubblewrap and flatroot's sandbox to replace the namespace's root with the target rootfs.
*[post-install]: The pipeline stage that runs after extraction: `ldconfig`, distro-specific scripts (`postinst`, `.INSTALL`, scriptlets) under the sandbox, and content-based cache hooks.
*[postinst]: Debian's per-package post-install script, embedded in `control.tar.*`. flatroot extracts it to `.flatroot/scripts/<pkg>:<arch>/postinst` and runs it inside the sandbox with stub commands on PATH.
*[postinstall]: See *post-install*. The two spellings appear interchangeably across the docs.
*[provenance]: The recorded trail of why each installed package is in the set — which package depended on it, what kind of dependency, which alternative was picked when several were offered, and any version constraint declared. This trail is recorded internally on the resolver's edges; the analyze command's output keeps only each package's bare dependency names, not the kind, picked alternative, or constraint.
*[provider]: A real package that claims to provide a virtual package. gawk and mawk are providers of the virtual awk.
*[providers]: Real packages that claim to provide a virtual package. gawk and mawk are providers of the virtual awk.
*[provides version]: The version a package declares for a virtual it provides, separate from the package's own version. On Arch, `gcc-libs` (package version `15.2.1`) may declare it provides `libgomp.so` at provides version `1-64`. The picker checks constraints against the provides version, not the package version.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- Q                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[queue]: A first-in-first-out buffer. The BFS pulls names off the front and pushes new ones on the back.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- R                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[recommended packages]: Debian's `Recommends:` field — softer than `Depends:`. Installed only when the user opts in with `--with=recommends`.
*[repository]: A directory of related packages published together by a distribution (Debian's `main` / `contrib` / `non-free`, Ubuntu's `main` / `universe`, Arch's `core` / `extra` / `multilib`, CentOS's `BaseOS` / `AppStream`).
*[resolution]: The process of figuring out a package's closure.
*[resolved set]: The set of packages already committed to the closure at some point during a walk. Synonymous with "the install set so far".
*[resolver]: The component that turns a list of names the user supplies into the complete ordered list of every package needed plus the dependency edges that justify each inclusion. Reads the on-disk SQLite index (and lazily loads the binary path index) and produces the install order; touches no network and writes nothing.
*[revision]: The packaging-revision component of a version string, after the last `-` (e.g. the `2+b10` in Debian's `5.2.15-2+b10`). Tracks distribution-side packaging changes that do not correspond to a new upstream release.
*[RFC 822]: The 1982 internet standard for electronic mail message headers — the `Key: Value` format every email header line uses. Debian adopted it for every piece of package metadata: `Packages.gz`, `Release`, `control` inside `.deb`.
*[rich dependency]: An RPM-family conditional dependency form. "install A only if B is also being installed" is a rich dependency that the rich-dep fixpoint handles.
*[rich dependencies]: RPM-family conditional dependency forms. "install A only if B" is a rich dependency that the rich-dep fixpoint handles.
*[rich-dep]: An RPM-family conditional dependency form. "install A only if B" is handled by the rich-dep fixpoint.
*[roll back]: Restore a previous state. The transitive sub-walk snapshots the walker before mutating; on failure, the snapshot is moved back.
*[rolls back]: Restores a previous state. The transitive sub-walk snapshots the walker before mutating; on failure, the snapshot is moved back.
*[rollback]: The act of restoring the install set to its state at a chosen earlier point — undoing every mutation made since that point. The transactional sub-walk uses snapshot-and-restore to provide one.
*[rolling repository]: A package repository that updates continuously rather than as point-in-time releases. Arch, CachyOS, and Alpine edge are rolling.
*[rolling-repo]: A package repository that updates continuously rather than as point-in-time releases. Arch, CachyOS, and Alpine edge are rolling.
*[rootfs]: The directory tree that appears as `/` to a running Linux process. Holds the minimum set of files needed for programs to run: executables in `/bin`, libraries in `/lib`, configuration in `/etc`, and the rest of the FHS-standard companions. flatroot's output.
*[rootfs's]: Belonging to a built rootfs.
*[RPM]: Red Hat Package Manager — the package format used by Fedora, CentOS, AlmaLinux, Rocky Linux, and openSUSE. `.rpm` files are a custom binary format with headers and a cpio payload.
*[RPM family]: The distributions whose package format is RPM: Fedora, CentOS, AlmaLinux, Rocky Linux, openSUSE.
*[RPM-family]: The distributions whose package format is RPM: Fedora, CentOS, AlmaLinux, Rocky Linux, openSUSE.
*[rpm-vercmp]: RPM's version comparison algorithm, defined by `rpmvercmp()` in rpmlib. Similar to dpkg's algorithm but discards separators between alphanumeric segments rather than weighting them. Used by flatroot for Fedora, CentOS, AlmaLinux, Rocky, and openSUSE.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- S                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[sandbox]: An isolated execution environment, typically created via bubblewrap or an equivalent user+mount-namespace combination. flatroot runs every post-install script inside one so that scripts written for live systems cannot affect the host.
*[SBOM]: Software Bill of Materials — a machine-readable inventory of every software component in a build artifact (name, version, origin, license, hashes, dependencies). flatroot does not emit SBOMs directly; `.flatroot/packages` carries most of the needed fields (name, version, source, URL, checksum, dependencies) but not license — neither the records nor the index hold license data.
*[scriptlet]: An RPM post-install script embedded inside the main RPM header. flatroot extracts shell scriptlets to `.flatroot/scripts/<pkg>:<arch>/postinst` and runs them under the same machinery as Debian `postinst`. Lua scriptlets are detected and skipped.
*[scriptlets]: RPM post-install scripts embedded inside the main RPM header.
*[seed list]: The merged starting list the resolver walks from: every package the user named, plus the per-distro base set, plus every Essential package on Debian/Ubuntu. The walk treats every name identically — it does not know which entries came from which source.
*[snapshot]: A frozen-in-time view of a distribution's archive. flatroot reaches snapshots through services like `snapshot.debian.org`, `snapshot.ubuntu.com`, and `archive.archlinux.org`.
*[snapshot mirror]: A mirror that serves a frozen-in-time view of a distribution's packages, indexed by date. Replaces the live mirror when a `@YYYY-MM-DD` suffix is set on the source.
*[snapshot pinning]: Fixing a source to `<distro>:<release>@YYYY-MM-DD` so the package index and every downloaded archive come from the snapshot mirror for that exact date.
*[snapshot service]: The HTTP service that fronts a distribution's snapshot archive (`snapshot.debian.org`, `snapshot.ubuntu.com`, `archive.archlinux.org`).
*[soft dependency]: An optional dependency. Recommends, Suggests, OptDepends. Followed only when the user opts in; missing ones are silently skipped.
*[soft dependencies]: Optional dependencies. Recommends, Suggests, OptDepends. Followed only when the user opts in; missing ones are silently skipped.
*[soname]: The version-tagged name a binary records for each shared library it links against, like libssl.so.3. The dynamic linker matches by soname at run time.
*[sonames]: Version-tagged names libraries declare for themselves, like `libssl.so.3` or `libGL.so.1`.
*[SPDX]: An SBOM standard maintained by the Linux Foundation, focused on license compliance and provenance. Serializations: tag-value (canonical), JSON, YAML.
*[SQLite]: A small, file-based, single-process SQL database engine. flatroot caches every parsed package index as a SQLite database under `~/.cache/flatroot/index/`.
*[SquashFS]: A compressed read-only filesystem format built into the Linux kernel. flatroot exports rootfs trees to SquashFS via `mksquashfs`; mount with `mount -t squashfs`.
*[stub command]: A short shell script flatroot installs at `.flatroot/bin/` to shadow a privileged or missing command that post-install scripts assume is present. No-op for `chown`, `systemctl`, `dpkg-trigger`; functional for `dpkg --compare-versions`, `update-alternatives --install`, `add-shell`.
*[stub commands]: Short shell scripts that shadow privileged or missing commands during post-install.
*[suite]: Debian's name for a release: `bookworm`, `bullseye`, `trixie`, plus the `-updates` / `-security` companions. The path component between `dists/` and `main/` on a Debian mirror.
*[symlink]: A filesystem symbolic link — an entry whose content is a path to another file.
*[symlinks]: Filesystem symbolic links.
*[syscall]: Short for system call — the mechanism by which userspace programs request services from the kernel.
*[system call]: The mechanism by which userspace programs request services from the kernel — opening files, allocating memory, creating processes, communicating with hardware.
*[system calls]: Kernel service requests from userspace.
*[systemd]: The init system and service manager on most modern Linux distributions. flatroot's stub commands no-op `systemctl` and friends because no systemd is running in a rootfs context.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- T                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[transitive]: Reachable through a chain. If A depends on B and B depends on C, then C is a transitive dependency of A.
*[transitively]: Through a chain. The closure is computed transitively — every package reachable through any chain of dependencies.
*[transitive sub-walk]: A primitive both fixpoints use to fold a triggered package's full hard-dep closure into the walker, atomically. Rolls back on failure.
*[tree]: A data structure where each entry has zero or more children. The resolver builds one internally (`src/dep_tree.rs`); the analyze command itself renders a flat, name-sorted entry list rather than a tree.
*[trigger]: One name in an Alpine install-if rule. The rule fires only when all triggers are present in the closure.
*[triggers]: Names in an Alpine install-if rule. The rule fires only when all triggers are present in the closure.
*[TTL]: Time-To-Live — how long a cached value is considered fresh before it is re-fetched. flatroot's package index has a one-hour TTL; package archives are cached permanently and reused while their checksum matches.
*[tuple]: Short for multiarch tuple — the architecture identifier like `x86_64-linux-gnu` or `i386-linux-gnu`.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- U                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[unshare]: The Linux system call (and command) that creates fresh namespaces for the calling process.
*[upstream]: The original maintainer or project from which a distribution packages a piece of software. The `upstream` component of a Debian version is the one set by that project, before any distribution-revision suffix.
*[upstream version]: The component of a version string set by the upstream project, before any distribution-revision suffix. The interesting part of any version comparison.
*[user namespace]: A kernel feature giving an unprivileged user namespace-scoped UID 0 and capabilities — privilege only with respect to namespace-internal resources, never the host.
*[user namespaces]: Kernel features granting unprivileged users namespace-scoped fake-root capabilities.
*[userspace]: The part of the operating system that runs outside the kernel — applications, libraries, daemons. ldconfig, the dynamic linker, and every package flatroot installs are userspace.
*[UTS]: Unix Timesharing System namespace — isolates the hostname and domain name. `bwrap --unshare-uts` creates one.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- V                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[version constraint]: A rule restricting which versions of a package satisfy a dependency. ">= 2.0", "< 3". Checked using the distro's version-comparison rules.
*[version constraints]: Rules restricting which versions of a package satisfy a dependency. ">= 2.0", "< 3". Checked using the distro's version-comparison rules.
*[virtual package]: A name that no real package owns directly, but that one or more real packages provide. awk is virtual; gawk and mawk provide it.
*[virtual packages]: Names that no real package owns directly, but that one or more real packages provide.
*[virtual name]: A name that no real package owns directly, but that one or more real packages provide. awk is virtual; gawk and mawk provide it.
*[virtual names]: Names that no real package owns directly, but that one or more real packages provide.
*[visited set]: The set of package names the walker has already committed to the closure. Used internally to deduplicate; not exposed to callers.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- W                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[walker]: The single object the resolver builds for one resolution. Bundles the read-only configuration with the install set being built; every operation the resolver performs is a method on it.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- X                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[XDG]: The XDG Base Directory Specification — environment variables that locate user-specific files (`XDG_CACHE_HOME`, `XDG_CONFIG_HOME`, …). flatroot honours `XDG_CACHE_HOME` for its cache root unless `FLATROOT_CACHE_HOME` is set.

<!-- ────────────────────────────────────────────────────────────── -->
<!-- Y                                                                 -->
<!-- ────────────────────────────────────────────────────────────── -->

*[YAML]: YAML Ain't Markup Language — a human-readable data-serialisation format. SPDX SBOMs and many configuration files use YAML; flatroot's manifest uses dpkg-style records instead.
