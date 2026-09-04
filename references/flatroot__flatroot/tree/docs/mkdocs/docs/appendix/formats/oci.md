---
tags:
  - appendix
  - formats
  - export
---

# OCI image format

The **OCI image format** is the standard packaging format for container images, maintained by the [Open Container Initiative](https://opencontainers.org/). It's the format `docker load` and `podman load` consume, the format registries like Docker Hub and `ghcr.io` store internally, and what `flatroot export --format oci` produces from a built rootfs.

## What's inside an OCI image

An OCI image is a tar archive containing:

| Path | Purpose |
|------|---------|
| `oci-layout` | Format version marker (required) |
| `index.json` | Top-level descriptor — points at manifests (required) |
| `blobs/sha256/<hash>` | Content-addressed blobs — config JSON, layer tarballs (required) |

A `manifest.json` file is also written, but solely for docker's classic (pre-containerd) image store — it is not part of the OCI layout specification. Podman reads the `org.opencontainers.image.ref.name` annotation on the index and docker's containerd store reads the `io.containerd.image.name` annotation; only the classic docker store consults `manifest.json`'s `RepoTags`.

Everything inside `blobs/sha256/` is named by the SHA-256 of its content. That's what makes the format *content-addressed*: identical content deduplicates across images, and any tampering produces a different hash — integrity checking for free.

A typical single-layer flatroot export has three content-addressed blobs plus the fixed-name index:

- **Config** (blob) — JSON describing exactly the architecture, OS, and the layer `diff_id`s. flatroot writes no env, cmd, or labels, so a loaded image has no default command.
- **Layer** (blob) — a gzipped tar of the rootfs contents.
- **Manifest** (blob) — JSON describing the image (schema version, config blob reference, list of layer blob references).
- **Index** — the fixed-name `index.json` at the archive root (not a content-addressed blob), pointing at the manifest.

## How flatroot builds it

```
flatroot export --format oci -t myapp:v1.0 <rootfs> image.tar
```

flatroot walks the rootfs, streams it into a single gzipped tar as one layer, computes the SHA-256 of the compressed layer (for the blob filename) and the uncompressed tar (for the `diff_id` field in the config), writes the config and manifest, and wraps everything into the outer tar.

The top-level `.flatroot/` metadata directory is excluded from the layer tar — see [Export exclusion](../../reference/metadata.md#export-exclusion) for the exact invariant and how it's tested.

The resulting archive loads with:

```
docker load < image.tar
podman load < image.tar
```

or can be pushed to a registry:

```
skopeo copy oci-archive:image.tar docker://registry/ref
```

## Why OCI instead of raw tar

An OCI image carries the metadata a container runtime needs to identify the image — its architecture, OS, and layer identity (flatroot sets no default command or env vars) — content addressing gives integrity checking for free, and the format is the lingua franca of container tooling. A raw tar requires the consumer to know how to turn it into a runnable container. OCI removes that step.

flatroot also supports a plain tar export (`--format tar`, always gzip-compressed) for cases where you only want the filesystem tree.

## Further reading

- [OCI Image Format specification](https://github.com/opencontainers/image-spec)
- [Rootfs Metadata — export exclusion](../../reference/metadata.md#export-exclusion)
- [CLI reference — export](../../reference/cli.md#export)

--8<-- "_glossary.md"
