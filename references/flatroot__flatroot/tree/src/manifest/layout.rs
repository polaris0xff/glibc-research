//! `ManifestLayout`: the fixed placement of the tool's `.flatroot/`
//! metadata inside a rootfs — each well-known path, and the test for
//! whether a path falls under it.

use std::path::{Path, PathBuf};

use crate::package::PackageIdentity;

/// One rootfs's `.flatroot/` layout, deriving every well-known path
/// from the root so writers and readers never disagree.
pub struct ManifestLayout {
  /// The rootfs root directory; every well-known path resolves relative to
  /// the reserved subdirectory beneath it.
  root: PathBuf,
}

impl ManifestLayout {
  /// Binds the layout to the rootfs at `root`.
  pub fn new(root: &Path) -> Self {
    ManifestLayout {
      root: root.to_path_buf(),
    }
  }

  /// The `.flatroot` directory holding all the tool's own state, kept
  /// in one place so it is easy to recognize and to exclude from an
  /// export.
  pub(crate) fn dir_meta(&self) -> PathBuf {
    self.root.join(".flatroot")
  }

  /// The manifest summary file, at the fixed path a later compatibility
  /// check reads.
  pub(crate) fn file_manifest(&self) -> PathBuf {
    self.dir_meta().join("manifest")
  }

  /// The package list an incremental install reads to skip
  /// already-current packages.
  pub(crate) fn file_packages(&self) -> PathBuf {
    self.dir_meta().join("packages")
  }

  /// The directory of per-package file lists, so every file can be
  /// attributed to the package that placed it.
  pub(crate) fn dir_files(&self) -> PathBuf {
    self.dir_meta().join("files")
  }

  /// One package's file list, named by identity so per-arch entries
  /// stay distinct.
  pub(crate) fn file_entry(&self, pkg: &PackageIdentity) -> PathBuf {
    self.dir_files().join(pkg.dirname())
  }

  /// The directory where extraction saves each package's install
  /// scripts for the post-install pass to read back.
  pub fn dir_scripts(&self) -> PathBuf {
    self.dir_meta().join("scripts")
  }

  /// One package's saved-scripts directory, named by identity so the
  /// script run attributes work to the exact package and architecture.
  pub fn dir_scripts_entry(&self, pkg: &PackageIdentity) -> PathBuf {
    self.dir_scripts().join(pkg.dirname())
  }

  /// Whether a rootfs-relative path falls under `.flatroot/` — the
  /// shared test separating the tool's own metadata from real package
  /// content.
  pub fn is_metadata(rel: &Path) -> bool {
    rel
      .components()
      .next()
      .map(|c| c.as_os_str() == ".flatroot")
      .unwrap_or(false)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn is_metadata_matrix() {
    assert!(ManifestLayout::is_metadata(Path::new(".flatroot")));
    assert!(ManifestLayout::is_metadata(Path::new(".flatroot/manifest")));
    assert!(ManifestLayout::is_metadata(Path::new(".flatroot/scripts/bash:x86_64/postinst")));
    assert!(ManifestLayout::is_metadata(Path::new(".flatroot/flatroot")));
    assert!(!ManifestLayout::is_metadata(Path::new("usr/bin/bash")));
    assert!(!ManifestLayout::is_metadata(Path::new("flatroot/something")));
  }
}
