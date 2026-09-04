//! Packages a rootfs as a single gzip-compressed tar archive any ordinary
//! unpacking tool can restore.

use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use flate2::Compression;
use flate2::write::GzEncoder;

use flatroot::internal::tar::Tar;

use super::Exporter;
use super::partial::DestPartial;
use super::source::RootfsSource;

/// The tar export: a gzip-compressed tar archive of the rootfs, with no
/// external tool needed.
pub struct TarExporter;

impl Exporter for TarExporter {
  /// Produces the gzip-tar archive, excluding the builder's
  /// `.flatroot/` metadata. Files are written as plain USTAR entries
  /// (no GNU-sparse) so stricter container archive readers accept them.
  fn export(&self, source: &RootfsSource, output: &Path) -> Result<()> {
    eprintln!("Exporting tar.gz to {}...", output.display());

    let dest = DestPartial::begin(output)?;
    let file = File::create(dest.path()).with_context(|| format!("Failed to create {}", dest.path().display()))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive.follow_symlinks(false);
    // tar-rs detects on-disk sparse regions by default and emits GNU-sparse
    // (type-S) entries. Plain `tar -x` accepts them; docker's containerd
    // archive parser rejects "unhandled tar header type 83". Disable sparse
    // detection so every file is written as a regular USTAR entry.
    archive.sparse(false);

    Tar::append_dir_sorted(&mut archive, source.path(), OsStr::new(".flatroot"))?;

    let encoder = archive.into_inner().context("Failed to finalize tar archive")?;
    encoder.finish().context("Failed to finalize gzip stream")?;
    dest.commit()?;

    eprintln!("Done.");
    Ok(())
  }
}
