//! Packages a rootfs as a compressed, read-only SquashFS image —
//! mountable directly on Linux and immutable, suiting distribution and use
//! as a base.

use std::path::Path;

use anyhow::Result;

use super::Exporter;
use super::partial::DestPartial;
use super::source::RootfsSource;
use flatroot::internal::tool::Tool;

/// The SquashFS export, delegating to the host's `mksquashfs`.
pub struct SqfsExporter;

impl Exporter for SqfsExporter {
  /// Produces the SquashFS image via `mksquashfs`, which must be
  /// present; its failure is an export failure. The builder's private
  /// `.flatroot/` metadata is excluded so the artifact carries only the
  /// filesystem.
  fn export(&self, source: &RootfsSource, output: &Path) -> Result<()> {
    let mksquashfs = Tool::locate("mksquashfs")?;
    let dir_str = source.path_str()?;
    let dest = DestPartial::begin(output)?;

    eprintln!("Exporting SquashFS to {}...", output.display());

    let out_str = super::source::path_str(dest.path(), "Output")?;
    mksquashfs.run(&[dir_str, out_str, "-noappend", "-e", ".flatroot"])?;
    dest.commit()?;

    eprintln!("Done.");
    Ok(())
  }
}
