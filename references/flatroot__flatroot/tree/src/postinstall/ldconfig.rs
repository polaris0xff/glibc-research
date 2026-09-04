//! Runs `ldconfig` inside the rootfs to rebuild `/etc/ld.so.cache`, so
//! programs in the finished rootfs resolve their shared libraries.

use anyhow::Result;

use super::rootfs::Rootfs;
use crate::sandbox::Sandbox;
use crate::ui::RunVoice;

/// Runs the rootfs's own `ldconfig` to rebuild the loader cache; a
/// rootfs without one (musl) is skipped, not an error.
pub struct Ldconfig;

impl Ldconfig {
  /// Rebuilds `/etc/ld.so.cache` from the rootfs's libraries. A rootfs
  /// without `ldconfig` (musl) skips this, and a failed run is
  /// reported, not fatal.
  pub fn run(rootfs: &Rootfs, sandbox: &Sandbox) -> Result<()> {
    let bin = match rootfs.binary_find(&["ldconfig"], None) {
      Some(b) => b,
      None => {
        eprintln!("ldconfig not found, skipping");
        return Ok(());
      }
    };

    let pb = RunVoice::spinner("Running ldconfig");
    let output = sandbox.run(&[&bin], &[])?;

    if output.status.success() {
      RunVoice::finish_ok(&pb, "ldconfig completed");
    } else {
      pb.finish_and_clear();
      eprintln!("ldconfig failed (exit {})", output.status.code().unwrap_or(-1));
    }
    Ok(())
  }
}
