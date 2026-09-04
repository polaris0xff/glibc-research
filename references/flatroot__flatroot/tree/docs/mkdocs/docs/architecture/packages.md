# Packages

FlatRoot builds a complete Linux root filesystem from a distribution's own published packages, without root privileges and without the host's package manager. Its code is divided along the grain of that job: a thin command surface at the top, a layer that acquires and understands distribution packages, a dependency-resolution core, a set of binary- and system-inspection helpers, a persistence layer that remembers what was fetched and built, and a finishing stage that makes an unprivileged extraction behave like an installed system. Most concerns are organized by *what they do* across every distribution, rather than by distribution, so each concern has a single home and the per-distribution knowledge lives as leaves under it.

## Command-Line Entry and Dispatch

The single front door: turn a user's invocation into a settled request and route it to the one place that fulfils it.

*Source: `src/main.rs`, `src/parser.rs`, `src/executor.rs`*

**Entrypoint**

The bare program entry: stand up the runtime, let the parser settle the invocation, and hand the result to the executor. It accumulates no logic of its own.

**Argument surface**

Owns the declarative shape of every flag and subcommand. It produces fully-settled request values and knows nothing about how any request is carried out; the refusal of bad combinations (an empty `--strategy` set, an OCI export named without a tag) lives one layer down in the executor and the export plan, not in the argument surface.

**Executor**

The thin hinge that reads the settled subcommand and hands control to the matching orchestrator. It carries only the routing decision and a small amount of cross-cutting framing; the substance lives one layer down.

## Subcommand Orchestration

One orchestrator per user intent — assemble a rootfs, ask what a package pulls in, list what is available, package a finished tree. Each owns a complete journey from settled request to finished outcome and coordinates the lower layers without reimplementing them.

*Source: `src/commands/`*

**Rootfs assembly orchestrator**

Owns the end-to-end build journey: select the distribution, then drive resolution, download, and extraction once per requested architecture, write the manifest once after every architecture succeeds, and run the post-install hand-off once at the end — against the primary architecture, and only when something was actually extracted. It is the tool's headline promise expressed as a sequence; the steps themselves belong to other packages.

**Catalogue interrogation set**

The read-only commands that let a user discover what a distribution offers before committing: package, library, and installed-path search, release enumeration, supported-distribution listing, and a raw-query escape hatch. Each answers one question and mutates nothing.

**Dependency-trace orchestrator**

Coordinates the "what does this pull in" question by running a declared-metadata pass and an actual-runtime-linkage pass independently, then reconciling their findings into one classified outcome. It settles intent and merges; the two analysis strategies and the result-shaping live in their own sibling units, the strategies under a passes group and the merged result in its own. That result renders its plain and JSON forms through the shared read-output formatter rather than a renderer of its own; the navigable tree shape lives separately in `src/dep_tree.rs`.

**Export orchestrator**

Settles the desired output format — explicit or inferred — and tag, refuses bad combinations before any work begins, then drives the matching format strategy. The format strategies for archive, image, and filesystem-image variants each stand alone behind a shared export contract, with the content-addressed image format further split into its wire-shape descriptions and its blob-staging workbench.

**Run fetch session**

The run's settled fetch posture — the durable cache home and the shared network client carrying the user's retry tolerance — fixed once when a command starts. Every catalogue open and release listing in the invocation draws on this one pair, so no command re-derives storage mid-run. The batch archive downloader is the one exception: it builds its own HTTP client per instance, carrying only the session's retry count, because it streams package archives rather than caching index responses through the shared client.

**Per-architecture session opener**

The shared preamble every catalogue-touching command runs against the fetch session: bind a distribution-and-architecture choice to a ready index and remote backend, so each command starts from an identical, opened picture rather than re-deriving it.

**Read-output formatter**

The one place the dual plain and JSON encoding for every read command is decided, so the two views cannot drift apart. Each result shape declares how it renders; this unit only negotiates the output mode.

## Package Vocabulary

The common language every other package speaks: the settled description of a package and its relationships, carrying no behaviour of its own.

*Source: `src/package.rs`*

**Package description**

A self-contained value: identity, version, every flavour of dependency, what it provides, checksums, and file facts. It is the currency passed between acquisition, resolution, persistence, and the manifest.

**Installed-package identity**

The name-plus-architecture pair that makes one installed package distinct in a multilib tree, owning the one on-disk `name:arch` spelling used for both a package's staged scripts and its manifest file ledger — so every encoder and decoder of that spelling answers to a single rule.

**Dependency and constraint shapes**

A requirement slot with its alternatives and optional version demand, plus the small text round-trip that lets these survive a trip through storage. The encoding is translation, not resolution.

**Relationship kinds and conditional-formula shape**

The closed vocabulary distinguishing hard, soft, negative, and conditional relationships, and the tree shape of a conditional dependency formula. These are type queries and classifications, never evaluation.

## Distribution Catalog

Hold, per distribution, the settled facts needed to find its packages: which mirrors serve it, how its repositories are laid out, which packages form its base, and how its releases are discovered. One distribution's knowledge of *where things live* has its home here.

*Source: `src/distro/`*

**Distribution contract and roster**

The single trait every distribution answers and the one authoritative list of which distributions exist. Every distribution-aware decision elsewhere funnels back to this roster.

**Settled source description**

The fact-sheet a distribution produces for a chosen release and architecture: its repositories, mirror lists, layout per packaging format, and base packages. A pure data carrier consumed by the access layer.

**Identity and format routing**

The small enums that map a distribution to its packaging convention, and that convention to its checksum and version-comparison choices. Routing knowledge, kept in one place so the rest of the system dispatches consistently.

**Release discovery**

The strategies for learning which releases exist, whether from fixed knowledge or by walking a mirror's directory listing and filtering it by family-specific rules, presented uniformly as a grid.

**Per-distribution leaves**

One stateless unit per supported distribution, each answering the same handful of questions with that distribution's real mirror URLs, repository paths, and base set.

## Format-Neutral Repository Access

Present the rest of the system with one distribution-blind way to fetch an index, resolve a download URL, extract an archive, and answer format-specific questions — so resolution and assembly never branch on which distribution they are serving.

*Source: `src/remote/`*

**Access contract and dispatcher**

A single contract and a single implementation that wraps a settled source description and dispatches each call to the matching packaging-format machinery. Adding a distribution never touches this interface; it only adds a source description.

**Library-provider lookup rules**

The per-packaging-ecosystem logic for turning a shared-library name into the package that ships it, with each ecosystem's naming conventions captured as one variant of a single resolver.

## Network Source and Mirror Fallback

Abstract *where bytes come from*: a single endpoint or an ordered set of equivalent endpoints tried in turn, so callers fetch without caring whether a mirror is live, date-pinned, or a fallback in a list.

*Source: `src/mirror/`*

**Source contract**

The small interface for fetching, probing, and listing, behind which any concrete source hides.

**Single-endpoint source**

One HTTP(S) endpoint, handling live and date-pinned forms by settling the date into the base URL up front.

**Ordered fallback set**

A preference-ordered group of equivalent endpoints that hides transient failures by trying each in turn, with explicit "consult them all" variants for the cases that need every source.

## Package-Format Parsing and Extraction

Understand each packaging format's two artefacts: the repository index that lists what exists, and the binary archive that holds a package's files and its install scripts.

*Source: `src/pkg/`*

**Format dispatch and contract**

The shared two-method contract — populate the catalogue from the index, and unpack an archive into files plus isolated post-install scripts — and the dispatch that selects a format implementation.

**Per-format implementations**

One unit per packaging convention, each turning the published repository index into catalogue entries and unpacking the binary container into files and isolated post-install scripts. Where a format's index and container are themselves elaborate, the work is broken into separate units for index assembly, container unpacking, conditional-formula parsing, and architecture-qualification.

## Verified Parallel Download

Turn a resolved set of packages into verified local archives: consult the cache, fetch what is missing in parallel, and prove each archive against its checksum before it is trusted.

*Source: `src/downloader.rs`*

**Batch downloader**

Owns the fetch lifecycle: cache hit-testing, bounded-parallel download, and checksum verification with re-fetch on mismatch. It understands packages and integrity, but nothing about packaging formats or distributions.

## Dependency Closure Resolution

Compute the complete, ordered set of packages that satisfies a request, honouring alternatives, version constraints, virtual capabilities, and the two conditional-pull mechanisms that two distribution families add.

*Source: `src/resolver/`*

**Walk engine and accumulating closure**

The one breadth-first engine and the growing answer it fills: the visited set, the install order, and the provenance edges. Both the install path and the analysis path enter here, so they compute identical closures — one path takes the flat order, the other also takes the edges.

**Atomic transitive sub-walk**

A narrower, all-or-nothing walk used when a triggered package must be pulled in with its complete hard-dependency closure as a unit, snapshotting and restoring state so a failed pull leaves no partial trace.

**Conditional-pull fixpoints**

Two orchestration loops, one per distribution family's conditional mechanism, each repeatedly evaluating its triggers against the current closure and folding fired packages in through the atomic sub-walk until nothing new fires. They evaluate and orchestrate; they do not re-walk.

**Name resolution**

The bridge from an abstract name to a concrete package: exact match, then preferred virtual-capability provider, then file-path owner. Used by every walk variant.

**Resolution data**

The read-only configuration backdrop and the provenance-edge type, both pure values.

## Dependency Graph Assembly

Shape the resolver's flat result into a navigable graph for explanation — which packages were requested, and the provenance edges among all of them.

*Source: `src/dep_tree.rs`*

**Graph builder**

Calls the shared walk and post-processes its output: remap virtual names to concrete providers, group edges by parent, and assemble the node-and-edge graph the analysis commands render. It assembles; it does not resolve.

## Version Ordering and Constraints

Decide which of two versions is newer and whether a candidate satisfies a constraint, under each packaging family's distinct version grammar.

*Source: `src/version/`*

**Constraint parsing and dispatch**

Parse an operator-and-version constraint and route the comparison to the right family grammar behind one shared contract.

**Per-family comparators**

The Debian, RPM, pacman, and apk ordering algorithms, one per packaging family, each implementing that family's version grammar and pre-release handling as pure ordering logic.

## Binary and System Inspection

A small cluster of read-only helpers that answer factual questions about compiled artefacts and the host. Each is a distinct fact-source sharing one character: pure inspection, no domain policy.

*Source: `src/elf.rs`, `src/arch.rs`, `src/library.rs`, `src/path.rs`, `src/path_index.rs`*

**ELF linkage inspection**

Extracts a binary's declared runtime library needs, its own library name, and its search paths. It reads ELF and nothing else: it does not resolve names, extract archives, or compare versions.

**Architecture identity**

The canonical processor enum and the translations between every naming dialect — kernel, Debian, Alpine, image, GNU triplet — plus detection from an ELF machine field. Pure naming and detection.

**Library-provider lookup**

Answers "which packages provide this library" by canonicalizing decorated library names and merging advertised capabilities with on-disk path facts. It reads two sources and unifies them; it mutates neither.

**Installed-path lookup**

Answers "which package owns this installed file path" by matching a path or glob against the file-ownership index. The path counterpart to library-provider lookup, it backs the `--type path` search space; like its siblings it only reads.

**File-to-package index**

A compact build-once, read-many mapping from a file path to its owning package, with the write side and the read side cleanly separated around a shared on-disk format.

## Persistent Package Catalogue

Hold the fetched distribution metadata in a fast local store and answer questions against it: which packages exist, what they provide, and what they depend on.

*Source: `src/db.rs`*

**Catalogue handle and lifecycle**

Opening, populating, and freshness-testing the store, binding the family-specific version comparator into the store as a sort order so version-aware queries are answered in one place, and vending the companion file-ownership record that lives beside the store as its matched pair.

**Write side**

Inserting packages and their relationships during population, and accumulating the companion file-ownership facts alongside them — the one writer owns both halves of the catalogue pair, and the populate lifecycle commits the companion once the whole catalogue is in.

**Query views**

Three focused read interfaces over the store — one each for packages, capability-providers, and dependencies — each exposing only the questions its subject answers.

## Rootfs Build Record

Remember what a build produced: the sources, architectures, and per-package facts that let a finished tree be described, verified, and guarded against an incompatible second build.

*Source: `src/manifest/`*

**In-memory authority and on-disk paths**

The settled record kept in memory, its persistence, and the resolution of where on disk the tool's whole reserved corner lives — the manifest, the per-package file ledgers, and the staged-scripts directory the extractors fill and post-install reads back.

**Text codec and per-package record**

The translation between the record and its on-disk text, and the shape of one package's stored facts.

**Integrity attestation**

The closed set of checksum schemes and the pairing of a scheme with a digest, including inferring the scheme from a digest's appearance.

## Unprivileged Execution Sandbox

Provide an isolated environment in which a partially-built rootfs appears as the root filesystem, so install scripts run as if on a real system without real privileges.

*Source: `src/sandbox.rs`*

**Namespace sandbox**

User- and mount-namespace creation, identity mapping, the minimal mount set, and the root pivot, plus a capability pre-check. Pure low-level isolation with no distribution or domain knowledge.

## Post-Install Finishing

Make an unprivileged extraction behave like an installed system: refresh the loader cache, replay each distribution's package scripts safely, and regenerate content-derived caches — in a fixed order.

*Source: `src/postinstall/`*

**Phase sequencing**

Verify the sandbox is available, then run the loader-cache, script-replay, and cache-regeneration phases in order.

**Layout introspection**

Answer "where does this distribution keep its binaries and libraries" across differing conventions, so later steps need no distribution knowledge.

**Generic script-replay engine**

A distribution-neutral engine that replays package scripts, parameterized by a flavour contract describing one distribution's interpreter, eligibility, staging, environment, and cleanup.

**Per-distribution flavours and stand-ins**

One flavour per distribution family, plus the management of harmless stand-in programs that absorb calls a real install would make to absent system tools.

**Cache-regeneration orchestration**

The best-effort table of content-derived caches and their rebuilders, each located, precondition-checked, and run, tolerating absence without failing the build.

## Final Permission Repair

Repair the one artefact an unprivileged extraction cannot get right: files left unreadable because the build never ran as their owner.

*Source: `src/postfixes/`*

**Permission walk**

Walks the finished tree and restores sane permissions to files the unprivileged extraction left inaccessible, behind a small apply entry point.

## Generic Utility Tier

A deliberately domain-agnostic foundation: behaviour that any program of this kind would need, expressed without any knowledge of packages, distributions, or rootfs assembly.

*Source: `src/internal/`*

**HTTP transport**

A caching, retrying client that moves bytes and knows nothing of what they mean.

**Archive and compression handling**

Generic tar handling — both unpacking archives and building the name-sorted archives the tar and OCI exporters emit — plus compressor inference, each a mechanism with no domain knowledge.

**Flat-text rendering**

The bijective nested-to-flat `KEY=VALUE` encoder underlying the plain output mode.

**Small primitives**

Cache-directory resolution, atomic file writes and parent creation, a file-descriptor wrapper, human-readable sizes, and external-tool lookup, each a single generic concern.

## User-Facing Progress Output

Speak to the user with one voice. Warnings are gated by a single global verbosity setting — hidden unless `--verbose` — while spinners and progress bars always display.

*Source: `src/ui.rs`*

**Run voice**

The centralized output surface and the verbosity flag every other package reads through, so diagnostics stay consistent.
