//! `PostInstallPlan` pairs the chosen `Phase`s with the rootfs they act
//! on and runs them in one fixed order, since each later phase relies
//! on what the earlier ones did.

use std::path::Path;

use anyhow::Result;

use crate::postinstall::hooks::CacheHooks;
use crate::postinstall::ldconfig::Ldconfig;
use crate::postinstall::rootfs::Rootfs;
use crate::postinstall::strategy::PostInstall;
use crate::sandbox::Sandbox;

/// One post-install phase a caller can enable independently — a fast
/// build might skip the expensive cache rebuilds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
  /// Run `ldconfig` from inside the rootfs to populate
  /// `/etc/ld.so.cache`.
  Ldconfig,
  /// Install the distro's stub PATH and replay each package's
  /// post-install script inside the sandbox.
  Scripts,
  /// Probe the rootfs for cross-package cache-rebuild binaries
  /// (pixbuf, fontconfig, MIME, GSettings, icon, locale, CA,
  /// dconf) and run whichever ones it finds.
  Hooks,
}

/// The chosen phases paired with the rootfs they act on and its
/// architecture.
pub struct PostInstallPlan<'a> {
  rootfs: Rootfs<'a>,
  phases: &'a [Phase],
  arch: &'a str,
}

impl<'a> PostInstallPlan<'a> {
  /// Pairs the chosen phases with the rootfs at `root`; `arch` selects
  /// which architecture's staged scripts run.
  pub fn new(root: &'a Path, phases: &'a [Phase], arch: &'a str) -> Self {
    Self {
      rootfs: Rootfs::new(root),
      phases,
      arch,
    }
  }

  /// Runs the chosen phases in their fixed order; an empty phase set
  /// does nothing. Errors before any work when the host lacks
  /// unprivileged user namespaces, since every phase runs sandboxed
  /// inside the rootfs.
  pub fn run(&self, post_install: &PostInstall) -> Result<()> {
    if self.phases.is_empty() {
      return Ok(());
    }

    Sandbox::available().map_err(|reason| {
      anyhow::anyhow!(
        "Post-install requires unprivileged user namespaces.\n  \
         {}\n  \
         To enable: sysctl kernel.unprivileged_userns_clone=1\n  \
         To skip post-install: use --postinstall=none",
        reason
      )
    })?;

    // One sandbox session presenting this rootfs at `/`, replayed across
    // every pass below — each `run` call supplies its own command and env.
    let sandbox = Sandbox::new(self.rootfs.path());

    if self.phases.contains(&Phase::Ldconfig) {
      Ldconfig::run(&self.rootfs, &sandbox)?;
    }

    // `stubs_install` is the precondition for `scripts_run` — postinst
    // scripts shadow privileged commands through the stub PATH, so the
    // two are bundled with the `Scripts` phase rather than exposed as a
    // separately selectable step.
    if self.phases.contains(&Phase::Scripts) {
      post_install.stubs_install(&self.rootfs)?;
      post_install.scripts_run(&self.rootfs, &sandbox, self.arch)?;
    }

    if self.phases.contains(&Phase::Hooks) {
      CacheHooks::run(&self.rootfs, &sandbox)?;
    }

    Ok(())
  }
}
