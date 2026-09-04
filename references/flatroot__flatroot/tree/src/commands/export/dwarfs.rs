//! Packages a rootfs as a single compressed, read-only DwarFS image
//! that mounts on demand, so the whole filesystem ships as one file.

use std::path::Path;

use anyhow::Result;

use super::Exporter;
use super::partial::DestPartial;
use super::source::RootfsSource;
use flatroot::internal::tool::Tool;

/// The DwarFS export, delegating compression to the host's `mkdwarfs`
/// rather than reimplementing the format.
pub struct DwarfsExporter;

impl Exporter for DwarfsExporter {
  /// Produces the DwarFS image via `mkdwarfs`, which must be present;
  /// its failure is an export failure. The builder's private
  /// `.flatroot/` metadata is filtered out so the artifact carries only
  /// the filesystem.
  fn export(&self, source: &RootfsSource, output: &Path) -> Result<()> {
    let mkdwarfs = Tool::locate("mkdwarfs")?;
    let dir_str = source.path_str()?;
    let dest = DestPartial::begin(output)?;

    eprintln!("Exporting DwarFS to {}...", output.display());

    let out_str = super::source::path_str(dest.path(), "Output")?;
    mkdwarfs.run(&["-i", dir_str, "-o", out_str, "--filter", "- /.flatroot/"])?;
    dest.commit()?;

    eprintln!("Done.");
    Ok(())
  }
}
