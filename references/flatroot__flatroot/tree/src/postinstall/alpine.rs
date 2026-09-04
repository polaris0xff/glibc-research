//! Alpine post-install: stubs for the service and package-management
//! commands an unprivileged rootfs lacks, then each package's
//! `post-install` script run inside the sandbox.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use anyhow::{Context, Result};

use super::rootfs::Rootfs;
use super::script::{ScriptEntry, ScriptFlavour, ScriptReplay};
use super::stubs::StubSet;
use crate::internal::fs::Fs;
use crate::sandbox::Sandbox;

/// Alpine's post-install behavior.
pub struct AlpinePostInstall;

impl AlpinePostInstall {
  /// Installs no-op stubs for Alpine's service and package-management
  /// commands, so an unsupported call succeeds quietly instead of
  /// aborting a script.
  pub fn stubs_install(&self, rootfs: &Rootfs) -> Result<()> {
    let stubs = StubSet::new(*rootfs);
    stubs.noop_keep(NOOP_STUBS)?;
    Ok(())
  }

  pub fn scripts_run(&self, rootfs: &Rootfs, sandbox: &Sandbox, arch: &str) -> Result<()> {
    ScriptReplay::new(*rootfs, sandbox, AlpineFlavour, arch).run()
  }
}

/// Alpine's service and package-management commands that cannot work in
/// an unprivileged rootfs, each stubbed to a no-op.
const NOOP_STUBS: &[&str] = &["rc-update", "rc-service", "openrc", "apk"];

/// Alpine's `ScriptFlavour`: runs each package's `post-install` script
/// with `sh`, passing the package name as the argument apk would.
pub struct AlpineFlavour;

impl ScriptFlavour for AlpineFlavour {
  fn interpreter(&self) -> &'static str {
    "sh"
  }

  fn eligible(&self, entry: &ScriptEntry) -> Result<bool> {
    Ok(entry.script("post-install").exists())
  }

  fn stage(&self, rootfs: &Rootfs, entry: &ScriptEntry, _interpreter: &str) -> Result<Option<Vec<String>>> {
    let post_install = entry.script("post-install");

    // Copy script into rootfs for sandbox execution. We already confirmed
    // `post_install.exists()` above, so a read failure here is a real I/O
    // error (permissions, vanished mid-loop) that the caller needs to see.
    let script_dest = rootfs.path().join(".flatroot/current-postinstall");
    let content =
      fs::read_to_string(&post_install).with_context(|| format!("failed to read {}", post_install.display()))?;
    fs::write(&script_dest, &content)?;
    fs::set_permissions(&script_dest, fs::Permissions::from_mode(0o755))?;

    Ok(Some(vec![
      "/bin/sh".to_string(),
      "/.flatroot/current-postinstall".to_string(),
      entry.identity.name.clone(),
    ]))
  }

  fn env(&self, _entry: &ScriptEntry) -> Vec<(String, String)> {
    vec![
      ("PATH".to_string(), "/.flatroot/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()),
      ("HOME".to_string(), "/root".to_string()),
      ("TERM".to_string(), "dumb".to_string()),
    ]
  }

  fn label(&self) -> &'static str {
    "post-install"
  }

  fn cleanup(&self, rootfs: &Rootfs) {
    Fs::remove_lenient(&rootfs.path().join(".flatroot/current-postinstall"));
  }

  fn finish_message(&self) -> &'static str {
    "Post-install scripts completed"
  }
}
