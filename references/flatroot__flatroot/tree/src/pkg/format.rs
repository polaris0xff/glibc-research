//! `PackageFormat`: the trait every package format implements — how it
//! publishes its index and how it extracts an archive.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use crate::db::IndexWriter;
use crate::distro::SourceDistro;
use crate::internal::http::HttpClient;
use crate::package::PackageIdentity;

/// One package format; callers pick the right one, then treat every
/// format identically.
pub trait PackageFormat: Send + Sync {
  /// Reads the format's published indexes from the source's mirrors
  /// into the local package index — package rows and file-to-package
  /// data — that the resolver and later stages read.
  fn index_fetch(&self, source: &SourceDistro, writer: &mut IndexWriter, http: &Arc<HttpClient>) -> Result<()>;

  /// Extracts one archive's files into the rootfs, sets the package's
  /// install scripts aside for the post-install pass, and returns the
  /// paths it wrote. Extraction order matters: some packages create
  /// directories later ones write into.
  fn extract(&self, path_file_archive: &Path, root: &Path, pkg: &PackageIdentity) -> Result<Vec<PathBuf>>;
}
