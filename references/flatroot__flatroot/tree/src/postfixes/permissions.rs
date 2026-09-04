//! Restores owner-readable access to files an unprivileged build left with
//! no permissions — the ones that expected a privileged step to grant
//! access afterward — so the tree is usable by whoever built it.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, Result};

/// Restores owner-readable access to every file the unprivileged build
/// could not permission.
pub struct PermissionFix;

impl PermissionFix {
  /// Repairs file access across the finished rootfs.
  pub fn apply(root: &Path) -> Result<()> {
    Self::walk(root)
  }

  /// Walks the whole tree, giving plain owner-readable access only to
  /// ordinary files left with no permissions; deliberately-set files are
  /// untouched and symlinks are not followed, so the fix cannot escape the
  /// tree.
  fn walk(dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
      let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
      let path = entry.path();
      let metadata = fs::symlink_metadata(&path).with_context(|| format!("failed to stat {}", path.display()))?;
      let file_type = metadata.file_type();

      if file_type.is_symlink() {
        continue;
      }

      if file_type.is_dir() {
        Self::walk(&path)?;
        continue;
      }

      if !file_type.is_file() {
        continue;
      }

      let mode = metadata.permissions().mode() & 0o7777;
      if mode == 0 {
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
          .with_context(|| format!("failed to set permissions on {}", path.display()))?;
      }
    }
    Ok(())
  }
}
