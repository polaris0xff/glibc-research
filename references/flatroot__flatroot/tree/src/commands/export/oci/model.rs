//! The JSON documents an OCI / Docker image is made of, as serializable
//! types, plus the constructors that build each from a stored blob or
//! layer.

use std::collections::HashMap;

use serde::Serialize;

use flatroot::arch::Arch;

use super::store::{BlobRef, LayerRef};

/// The layout-version marker announcing which revision of the
/// content-addressed layout a directory follows.
#[derive(Serialize)]
pub(super) struct OciLayout {
  #[serde(rename = "imageLayoutVersion")]
  image_layout_version: &'static str,
}

/// The top-level document an engine reads first to find which manifest to
/// follow.
#[derive(Serialize)]
pub(super) struct OciIndex {
  #[serde(rename = "schemaVersion")]
  schema_version: u32,
  manifests: Vec<OciIndexEntry>,
}

/// One pointer from the index to an image manifest, with the platform
/// detail a consumer verifies before fetching.
#[derive(Serialize)]
struct OciIndexEntry {
  #[serde(rename = "mediaType")]
  media_type: &'static str,
  digest: String,
  size: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  platform: Option<OciPlatform>,
  #[serde(skip_serializing_if = "Option::is_none")]
  annotations: Option<HashMap<&'static str, String>>,
}

/// The architecture and OS an image targets, so a consumer can refuse
/// an image its machine cannot run.
#[derive(Serialize)]
struct OciPlatform {
  architecture: &'static str,
  os: &'static str,
  #[serde(skip_serializing_if = "Option::is_none")]
  variant: Option<&'static str>,
}

/// One image's manifest: the descriptors pointing at its config and its
/// filesystem layers.
#[derive(Serialize)]
pub(super) struct OciManifest {
  #[serde(rename = "schemaVersion")]
  schema_version: u32,
  #[serde(rename = "mediaType")]
  media_type: &'static str,
  config: OciDescriptor,
  layers: Vec<OciDescriptor>,
}

/// One content-addressed reference to a blob: digest, size, and media
/// type.
#[derive(Serialize)]
struct OciDescriptor {
  #[serde(rename = "mediaType")]
  media_type: &'static str,
  digest: String,
  size: u64,
}

/// The image config: the architecture and OS the image targets and the
/// `diff_ids` of its layers' unpacked content.
#[derive(Serialize)]
pub(super) struct OciConfig {
  architecture: &'static str,
  os: &'static str,
  rootfs: OciRootfs,
}

/// The config's rootfs section: the `diff_ids` — the sha256 of each
/// layer's unpacked tar — an engine recognizes layers by.
#[derive(Serialize)]
struct OciRootfs {
  #[serde(rename = "type")]
  rootfs_type: &'static str,
  diff_ids: Vec<String>,
}

/// One entry of the Docker-specific `manifest.json` written beside the
/// OCI layout, so Docker's classic store — which ignores the layout —
/// still loads the archive.
#[derive(Serialize)]
pub(super) struct DockerManifestEntry {
  #[serde(rename = "Config")]
  config: String,
  #[serde(rename = "RepoTags")]
  repo_tags: Vec<String>,
  #[serde(rename = "Layers")]
  layers: Vec<String>,
  #[serde(rename = "LayerSources")]
  layer_sources: HashMap<String, DockerLayerSource>,
}

/// One layer's digest, size, and media type in `manifest.json`'s
/// `LayerSources`, so the classic loader can index the layer as it
/// would one pulled from a registry.
#[derive(Serialize)]
struct DockerLayerSource {
  #[serde(rename = "mediaType")]
  media_type: &'static str,
  size: u64,
  digest: String,
}

impl OciLayout {
  /// The one layout revision every produced image announces.
  pub(super) const CURRENT: OciLayout = OciLayout {
    image_layout_version: "1.0.0",
  };
}

impl OciConfig {
  /// The config for a single-layer image: the target architecture plus
  /// the layer's `diff_id`.
  pub(super) fn for_layer(arch: Arch, layer: &LayerRef) -> Self {
    OciConfig {
      architecture: arch.as_goarch(),
      os: "linux",
      rootfs: OciRootfs {
        rootfs_type: "layers",
        diff_ids: vec![format!("sha256:{}", layer.diff_id)],
      },
    }
  }
}

impl OciManifest {
  /// The manifest for a single-layer image: one config blob and one
  /// layer, each referenced by digest.
  pub(super) fn single_layer(config: &BlobRef, layer: &LayerRef) -> Self {
    OciManifest {
      schema_version: 2,
      media_type: "application/vnd.oci.image.manifest.v1+json",
      config: OciDescriptor {
        media_type: "application/vnd.oci.image.config.v1+json",
        digest: format!("sha256:{}", config.digest),
        size: config.size,
      },
      layers: vec![OciDescriptor {
        media_type: "application/vnd.oci.image.layer.v1.tar+gzip",
        digest: format!("sha256:{}", layer.blob.digest),
        size: layer.blob.size,
      }],
    }
  }
}

impl OciIndex {
  /// The index for a single image, pointing at its manifest and target
  /// platform.
  pub(super) fn for_manifest(tag: &str, manifest: &BlobRef, arch: Arch) -> Self {
    // The image's name is written in three forms because the three
    // loaders that consume this archive each read it from a different
    // place, in a different form:
    //
    //   - podman reads `org.opencontainers.image.ref.name` from this index
    //     entry and treats it as the full `name:tag` (prefixing `localhost/`
    //     on import).
    //   - docker with the containerd image store (the default on fresh
    //     modern installs) ignores `manifest.json` entirely and names the
    //     image from `io.containerd.image.name`, which must hold the
    //     fully-qualified reference. Docker never queries its store with
    //     the string the user typed: every lookup (`inspect`, `run`, `rmi`)
    //     first expands short names (`app:1` -> `docker.io/library/app:1`)
    //     and then compares against the stored name. When this annotation
    //     is absent, the loader falls back to storing `ref.name` verbatim —
    //     a bare `name:tag` no expanded lookup can ever match — so the
    //     image loads but is reachable only by ID, and `docker inspect
    //     <tag>` fails with "no such object".
    //   - docker with the classic store reads neither annotation; it tags
    //     from `RepoTags` in the engine-specific `manifest.json` written
    //     beside this layout.
    //
    // Writing all three is what lets a single archive land tagged under
    // every loader rather than only some.
    let mut annotations = HashMap::new();
    annotations.insert("org.opencontainers.image.ref.name", tag.to_string());
    annotations.insert("io.containerd.image.name", reference_normalize(tag));
    OciIndex {
      schema_version: 2,
      manifests: vec![OciIndexEntry {
        media_type: "application/vnd.oci.image.manifest.v1+json",
        digest: format!("sha256:{}", manifest.digest),
        size: manifest.size,
        platform: Some(OciPlatform {
          architecture: arch.as_goarch(),
          os: "linux",
          variant: arch.oci_variant(),
        }),
        annotations: Some(annotations),
      }],
    }
  }
}

/// Docker resolves image names only in fully-qualified form: every CLI
/// lookup first expands what the user typed through Docker Hub's short-name
/// rules (docker's `distribution/reference` library, `ParseNormalizedNamed`)
/// and only then consults the store — so a name stored in any other form is
/// permanently unreachable by name. This applies those same rules to the
/// export's `--tag`, so the name stored at load time and every future lookup
/// agree:
///
///   `app:1`                -> `docker.io/library/app:1`  (bare name: Docker
///                             Hub's default registry and `library/`
///                             namespace)
///   `org/app:1`            -> `docker.io/org/app:1`      (namespaced name:
///                             default registry only)
///   `ghcr.io/org/app:1`    -> unchanged                  (first component
///                             contains a dot — already a registry)
///   `localhost:5000/app:1` -> unchanged                  (a port, or the
///                             literal `localhost`, marks a registry too)
fn reference_normalize(tag: &str) -> String {
  match tag.split_once('/') {
    None => format!("docker.io/library/{tag}"),
    Some((registry, _)) if registry.contains('.') || registry.contains(':') || registry == "localhost" => {
      tag.to_string()
    }
    Some(_) => format!("docker.io/{tag}"),
  }
}

impl DockerManifestEntry {
  /// The `manifest.json` entry for one tagged image, so the archive
  /// loads under Docker's classic store.
  pub(super) fn for_image(tag: &str, config: &BlobRef, layer: &LayerRef) -> Self {
    let layer_key = format!("sha256:{}", layer.blob.digest);
    let mut layer_sources = HashMap::new();
    layer_sources.insert(
      layer_key.clone(),
      DockerLayerSource {
        media_type: "application/vnd.oci.image.layer.v1.tar+gzip",
        size: layer.blob.size,
        digest: layer_key,
      },
    );
    DockerManifestEntry {
      config: format!("blobs/sha256/{}", config.digest),
      repo_tags: vec![tag.to_string()],
      layers: vec![format!("blobs/sha256/{}", layer.blob.digest)],
      layer_sources,
    }
  }
}
