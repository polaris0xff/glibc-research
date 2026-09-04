---
tags:
  - architecture
  - spec
---

# Specification

This section is the behavioural specification of FlatRoot: one page per use case, each recording what the tool promises for that request and how the promise travels through the system's packages. The pages are contracts, not tutorials — for hands-on instructions see the How-To guides, and for the meaning of each package see [Packages](../packages.md).

Every page follows the same spec shape: a status header (these are as-built specs — the code is the source of truth, so there are no open clarifications), **User Scenarios & Testing** with user stories, each independently testable and carrying **Given/When/Then** acceptance scenarios, then the atomized **functional requirements** — grouped by concern under bold group headers, each requirement a normative MUST statement identified as `FR-<GROUP>-NNN` and followed by a concrete annotated example of the behaviour it pins (every guarantee and every deliberate refusal) — the **key entities** the use case reads or produces, measurable **success criteria** (`SC-###`), the **assumptions** the request relies on, and the **edge cases** with their settled answers. Identifiers are page-scoped: `FR-SOURCE-001` on one page is unrelated to `FR-SOURCE-001` on another; cite them as page + ID. A requirement group describing behaviour that is specified but not yet implemented is explicitly marked *(proposed — not yet built)*, and the page's status header names it.

One deliberate departure from a to-be-built spec: each page closes with an **Execution flow (as-built)** section — the ordered operations table (each step naming the package that performs it and the source that implements it) and the package-level sequence diagram. A prescriptive spec would exile that material to a separate plan document; here it is the trace that binds every requirement back to the architecture, which is this spec's purpose.

## Use cases

| Use case | Trigger | Outcome |
|---|---|---|
| [Installing Packages](install.md) | `install` | A usable rootfs directory holding the requested packages and their closure, with a truthful install record |
| [Searching Packages](search.md) | `search` | The packages or libraries matching the given patterns, with enough detail to act on each hit |
| [Querying the Catalogue](query.md) | `query` | The result rows of a free-form SQL question posed against the local package catalogue |
| [Listing Distributions](remotes.md) | `remote list` | The roster of buildable distributions and the selector syntax each accepts |
| [Listing Releases](releases.md) | `release list` | The releases a chosen distribution currently offers, validated as genuinely installable |
| [Exporting a Rootfs](export.md) | `export` | A finished rootfs sealed into one portable artefact — archive, container image, or mountable filesystem |
| [Tracing Dependencies](trace.md) | `analyze trace` | The classified dependency closure of a set of seeds — declared edges, real linker edges, and the gaps between them |

## Diagram participants

The sequence diagrams speak one fixed vocabulary: every participant is a package from the [Packages](../packages.md) page, under the short alias below. Participants appear in a diagram only when the use case actually flows through them.

| Participant | Package | Source |
|---|---|---|
| `Entry` | [Command-Line Entry and Dispatch](../packages.md#command-line-entry-and-dispatch) | `src/main.rs`, `src/parser.rs`, `src/executor.rs` |
| `Command` | [Subcommand Orchestration](../packages.md#subcommand-orchestration) | `src/commands/` |
| `Distro` | [Distribution Catalog](../packages.md#distribution-catalog) | `src/distro/` |
| `Remote` | [Format-Neutral Repository Access](../packages.md#format-neutral-repository-access) | `src/remote/` |
| `Mirror` | [Network Source and Mirror Fallback](../packages.md#network-source-and-mirror-fallback) | `src/mirror/` |
| `Format` | [Package-Format Parsing and Extraction](../packages.md#package-format-parsing-and-extraction) | `src/pkg/` |
| `Catalogue` | [Persistent Package Catalogue](../packages.md#persistent-package-catalogue) | `src/db.rs` |
| `Resolver` | [Dependency Closure Resolution](../packages.md#dependency-closure-resolution) | `src/resolver/` |
| `Graph` | [Dependency Graph Assembly](../packages.md#dependency-graph-assembly) | `src/dep_tree.rs` |
| `Download` | [Verified Parallel Download](../packages.md#verified-parallel-download) | `src/downloader.rs` |
| `Inspect` | [Binary and System Inspection](../packages.md#binary-and-system-inspection) | `src/elf.rs`, `src/arch.rs`, `src/library.rs`, `src/path.rs`, `src/path_index.rs` |
| `Record` | [Rootfs Build Record](../packages.md#rootfs-build-record) | `src/manifest/` |
| `Sandbox` | [Unprivileged Execution Sandbox](../packages.md#unprivileged-execution-sandbox) | `src/sandbox.rs` |
| `Postinstall` | [Post-Install Finishing](../packages.md#post-install-finishing) | `src/postinstall/` |
| `Repair` | [Final Permission Repair](../packages.md#final-permission-repair) | `src/postfixes/` |
| `Utility` | [Generic Utility Tier](../packages.md#generic-utility-tier) | `src/internal/` |
| `Upstream` | The distribution's mirror servers — external to FlatRoot, shown where bytes actually leave the machine | — |

Two conventions keep the diagrams readable. First, they show control and data flow between packages, not type reuse — a package that merely imports another's value types (the [Package Vocabulary](../packages.md#package-vocabulary), the version comparators, the integrity claim) is credited in the operations table instead. Second, the cross-cutting packages every flow leans on — [User-Facing Progress Output](../packages.md#user-facing-progress-output) and most of the [Generic Utility Tier](../packages.md#generic-utility-tier) — are omitted unless a flow's substance genuinely runs through them, as it does for export.

--8<-- "_glossary.md"
