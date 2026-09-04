//! `Postfix` runs the final corrections in a fixed order; a correction
//! that fails is an error, so a rootfs that cannot be made consistent
//! is never reported as finished.

use std::path::Path;

use anyhow::Result;

use crate::postfixes::permissions::PermissionFix;

/// The final build stage, gathering the corrections an unprivileged
/// build leaves undone.
pub struct Postfix;

impl Postfix {
  /// Applies each correction in a fixed order; the first failure stops
  /// the run, so an inconsistent rootfs is never reported as finished.
  pub fn apply(root: &Path) -> Result<()> {
    PermissionFix::apply(root)?;
    Ok(())
  }
}
