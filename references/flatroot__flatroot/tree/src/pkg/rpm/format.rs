//! `RpmFormat`, the RPM `PackageFormat` implementation, delegating to
//! the repodata reader and the `.rpm` container parser.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::db::IndexWriter;
use crate::distro::SourceDistro;
use crate::internal::http::HttpClient;
use crate::package::PackageIdentity;
use crate::pkg::PackageFormat;
use crate::pkg::rpm::container::RpmContainer;
use crate::pkg::rpm::repodata::RepodataReader;

/// The RPM-family `PackageFormat` implementation.
pub struct RpmFormat;

impl PackageFormat for RpmFormat {
  /// Reads each repository's repodata into the package index and the
  /// file-to-package data, dropping packages built for other
  /// architectures.
  fn index_fetch(&self, source: &SourceDistro, writer: &mut IndexWriter, http: &Arc<HttpClient>) -> Result<()> {
    let mut reader = RepodataReader::new(writer, http, &source.native_arch);

    for repo in &source.repos {
      reader.repo_ingest(repo)?;
    }

    Ok(())
  }

  /// Extracts one RPM into the rootfs: decompresses the cpio payload,
  /// writes its files, and saves the POSTIN scriptlet for the
  /// post-install pass. Returns the files placed.
  fn extract(&self, path_file_archive: &Path, root: &Path, pkg: &PackageIdentity) -> Result<Vec<PathBuf>> {
    let data = std::fs::read(path_file_archive)?;
    let archive = RpmContainer::open(&data);

    // Decompress the compressed cpio payload (auto-detects xz, gzip,
    // or zstd from the payload's leading magic bytes).
    let decompressed = archive
      .payload_decompress()
      .with_context(|| format!("No compressed payload found in {}", path_file_archive.display()))?;

    // Extract cpio archive to rootfs and collect payload paths.
    let files = RpmContainer::cpio_extract(&decompressed, root)?;

    // Lift the POSTIN scriptlet from the RPM header.
    archive.scripts_stage(root, pkg)?;

    Ok(files)
  }
}
