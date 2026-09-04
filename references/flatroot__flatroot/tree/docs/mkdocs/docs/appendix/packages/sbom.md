---
tags:
  - appendix
  - reproducibility
  - metadata
---

# SBOM, CycloneDX, and SPDX

A **Software Bill of Materials** (SBOM) is a machine-readable inventory of every software component that went into a build artifact: name, version, origin, license, cryptographic hashes, and dependency relationships. SBOMs exist because you cannot audit a binary for known vulnerabilities, license incompatibilities, or supply-chain tampering without knowing exactly what's inside.

flatroot does not emit SBOMs directly, but every rootfs it builds records most of the data an SBOM needs — with one gap: it captures no license information, since neither the package records nor the catalogue carry a license field.

## Where flatroot writes the inventory

The `.flatroot/packages` file (see [Rootfs Metadata — packages](../../reference/metadata.md#packages)) holds one dpkg-style record per installed `(name, architecture)` pair with a fixed field set:

| flatroot field | SBOM relevance |
|----------------|----------------|
| `Package` | Component name |
| `Version` | Component version |
| `Architecture` | Platform target |
| `Source` | The verbatim `--from` selector — `<distro>:<release>`, with a `@date` only when the user pinned one (provenance) |
| `Url` | Fully qualified download URL (self-contained provenance) |
| `Checksum` | Typed hash (`sha256:` or `q1-sha1:`) for integrity |
| `Size` | Archive size in bytes |
| `Depends` | Flattened hard dependency list |

This covers most of the field set common SBOM schemas expect; the notable omission is license, which a downstream emitter must source elsewhere. External tooling can otherwise read `.flatroot/packages` and emit CycloneDX or SPDX directly.

## CycloneDX

[CycloneDX](https://cyclonedx.org/) is an SBOM standard maintained by the [OWASP Foundation](https://owasp.org/). Its focus is security: vulnerability tracking, cryptographic component attestation, and dependency graphs. Serializations: JSON (canonical), XML, Protobuf.

Typical mapping of `.flatroot/packages` to CycloneDX:

| flatroot field | CycloneDX field |
|----------------|-----------------|
| `Package` | `name` |
| `Version` | `version` |
| `Checksum` | `hashes[0]` (algorithm derived from the prefix) |
| `Url` | `externalReferences[].url` with `type: "distribution"` |
| `Source` | `supplier.name` or a custom `properties[]` entry |
| `Depends` | `dependencies[].dependsOn` |

## SPDX

[SPDX](https://spdx.dev/) is the Linux Foundation's SBOM standard. Its focus is license compliance and provenance. Serializations: tag-value (canonical), JSON, YAML.

Typical mapping of `.flatroot/packages` to SPDX:

| flatroot field | SPDX field |
|----------------|------------|
| `Package` | `PackageName` |
| `Version` | `PackageVersion` |
| `Checksum` | `PackageChecksum` |
| `Url` | `PackageDownloadLocation` |
| `Source` | `PackageSupplier: Organization:` (distro-derived) |
| `Depends` | `Relationship: ... DEPENDS_ON ...` |

## Which format to use

- If your primary concern is vulnerability scanning or supply-chain attestation, CycloneDX is more common in that ecosystem.
- If your primary concern is license audit or enterprise/government procurement compliance, SPDX is more common.
- For container-focused pipelines, most scanners accept either. Some projects publish both.

flatroot's metadata is format-agnostic; the choice lives in your SBOM-emitting tool, not in flatroot.

## Further reading

- [Rootfs Metadata — packages file](../../reference/metadata.md#packages) — the structure flatroot writes.
- [CycloneDX specification](https://cyclonedx.org/specification/overview/)
- [SPDX specification](https://spdx.github.io/spdx-spec/)

--8<-- "_glossary.md"
