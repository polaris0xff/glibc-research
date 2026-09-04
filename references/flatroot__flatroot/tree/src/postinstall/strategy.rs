//! `PostInstall`: the closed set of post-install behaviors —
//! Debian-family (also used by the RPM families), Arch-family, and
//! Alpine.

use anyhow::Result;

use crate::postinstall::rootfs::Rootfs;
use crate::postinstall::{alpine::AlpinePostInstall, arch::ArchPostInstall, debian::DebPostInstall};
use crate::sandbox::Sandbox;

/// The post-install behavior a rootfs is finished with; each
/// distribution maps to exactly one (see `KindDistro::post_install`),
/// so the distributions collapse to the three behaviors that differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostInstall {
  Debian,
  Arch,
  Alpine,
}

impl PostInstall {
  /// Installs stub executables ahead of the real ones on the scripts'
  /// PATH, so the privileged commands the scripts call succeed as
  /// no-ops while the rest of each script still runs.
  pub fn stubs_install(&self, rootfs: &Rootfs) -> Result<()> {
    match self {
      PostInstall::Debian => DebPostInstall.stubs_install(rootfs),
      PostInstall::Arch => ArchPostInstall.stubs_install(rootfs),
      PostInstall::Alpine => AlpinePostInstall.stubs_install(rootfs),
    }
  }

  /// Runs each package's saved install scripts inside the sandbox,
  /// under this behavior's interpreter, eligibility, and environment
  /// rules. Copying files alone leaves a package half-installed; this
  /// finishes the job.
  pub fn scripts_run(&self, rootfs: &Rootfs, sandbox: &Sandbox, arch: &str) -> Result<()> {
    match self {
      PostInstall::Debian => DebPostInstall.scripts_run(rootfs, sandbox, arch),
      PostInstall::Arch => ArchPostInstall.scripts_run(rootfs, sandbox, arch),
      PostInstall::Alpine => AlpinePostInstall.scripts_run(rootfs, sandbox, arch),
    }
  }
}
