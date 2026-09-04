//! The OCI export: turns a prepared rootfs into a single-layer
//! container image packed as one tar archive that both Docker and
//! Podman load without a registry.

use std::path::Path;

use anyhow::Result;

use crate::commands::export::Exporter;

use super::super::partial::DestPartial;
use super::super::source::RootfsSource;
use super::model::{DockerManifestEntry, OciConfig, OciIndex, OciLayout, OciManifest};
use super::store::OciImage;

/// The OCI export, carrying the tag the resulting image is named with.
pub struct OciExporter<'a> {
  pub tag: &'a str,
}

impl Exporter for OciExporter<'_> {
  /// Produces a loadable OCI image at `output`: a single rootfs layer
  /// plus the config, manifest, index, and Docker-compatible
  /// `manifest.json`, packed into one tar archive.
  fn export(&self, source: &RootfsSource, output: &Path) -> Result<()> {
    eprintln!("Exporting OCI image to {}...", output.display());

    let arch = source.arch();
    eprintln!("  detected architecture: {} ({})", arch.as_uname(), arch.as_goarch());

    let image = OciImage::stage()?;

    eprintln!("  creating rootfs layer...");
    let layer = image.layer_append(source)?;

    let config = image.blob_write_json(&OciConfig::for_layer(arch, &layer))?;
    let manifest = image.blob_write_json(&OciManifest::single_layer(&config, &layer))?;
    image.file_write_json("index.json", &OciIndex::for_manifest(self.tag, &manifest, arch))?;
    image.file_write_json("oci-layout", &OciLayout::CURRENT)?;
    image.file_write_json("manifest.json", &[DockerManifestEntry::for_image(self.tag, &config, &layer)])?;

    eprintln!("  writing output tar...");
    let dest = DestPartial::begin(output)?;
    image.seal(dest.path())?;
    dest.commit()?;

    eprintln!("Done.");
    Ok(())
  }
}
