//! `Exporter`, the trait every export format implements.

use std::path::Path;

use anyhow::Result;

use super::source::RootfsSource;

/// One export format: turns a prepared rootfs into a finished artifact,
/// so callers request an export without knowing how the format is
/// produced.
pub trait Exporter {
  /// Writes the artifact for `source` at `output`.
  fn export(&self, source: &RootfsSource, output: &Path) -> Result<()>;
}
