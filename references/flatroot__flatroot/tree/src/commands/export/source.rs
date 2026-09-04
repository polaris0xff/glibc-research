//! `RootfsSource`: the validated source directory an export reads,
//! checked once and shared by every format.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use flatroot::arch::Arch;

/// The validated rootfs an export packages: its directory and its
/// target architecture.
pub struct RootfsSource {
  path_dir_root: PathBuf,
  arch: Arch,
}

impl RootfsSource {
  /// Checks that `dir` is an existing directory and detects the
  /// rootfs's target architecture.
  pub fn open(dir: &Path) -> Result<Self> {
    if !dir.exists() || !dir.is_dir() {
      bail!("Source directory '{}' does not exist or is not a directory", dir.display());
    }
    let arch = Arch::detect(dir).context("Failed to detect rootfs architecture")?;
    Ok(Self {
      path_dir_root: dir.to_path_buf(),
      arch,
    })
  }

  /// The rootfs directory.
  pub fn path(&self) -> &Path {
    &self.path_dir_root
  }

  /// The architecture the rootfs targets.
  pub fn arch(&self) -> Arch {
    self.arch
  }

  /// The rootfs directory as UTF-8 text, for an external tool's command
  /// line; a non-UTF-8 path is an error.
  pub fn path_str(&self) -> Result<&str> {
    path_str(&self.path_dir_root, "Source directory")
  }
}

/// `path` as UTF-8 text for an external tool's command line; a
/// non-UTF-8 path is an error naming `what`.
pub fn path_str<'p>(path: &'p Path, what: &str) -> Result<&'p str> {
  path
    .to_str()
    .ok_or_else(|| anyhow::anyhow!("{} path is not valid UTF-8", what))
}
