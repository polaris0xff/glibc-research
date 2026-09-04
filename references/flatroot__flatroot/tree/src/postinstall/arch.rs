//! Arch-family post-install (shared by CachyOS): runs each package's
//! `install` script, with stubs ahead of the privileged commands those
//! scripts call.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use anyhow::Result;

use super::rootfs::Rootfs;
use super::script::{ScriptEntry, ScriptFlavour, ScriptReplay};
use super::stubs::StubSet;
use crate::internal::fs::Fs;
use crate::sandbox::Sandbox;

/// The Arch-family post-install behaviour, shared unchanged by CachyOS.
pub struct ArchPostInstall;

impl ArchPostInstall {
  /// Installs no-op stubs for Arch's user/group/system-file helpers,
  /// plus a `vercmp` stub answering "equal" so version-gated upgrade
  /// blocks are skipped during a fresh build.
  pub fn stubs_install(&self, rootfs: &Rootfs) -> Result<()> {
    let stubs = StubSet::new(*rootfs);

    stubs.noop_keep(NOOP_STUBS)?;

    // vercmp stub — returns "0" (equal) so version-gated upgrade blocks are skipped
    stubs.script_keep("vercmp", "#!/bin/sh\necho 0\n")?;

    Ok(())
  }

  pub fn scripts_run(&self, rootfs: &Rootfs, sandbox: &Sandbox, arch: &str) -> Result<()> {
    ScriptReplay::new(*rootfs, sandbox, ArchFlavour, arch).run()
  }
}

/// The privileged commands stubbed to no-ops — kept deliberately narrow
/// so the cache-regeneration commands the hooks phase runs for real are
/// not silently swallowed.
const NOOP_STUBS: &[&str] = &[
  "groupadd",
  "useradd",
  "usermod",
  "groupmod",
  "systemd-sysusers",
  "systemd-tmpfiles",
  "install-info",
  "xdg-icon-resource",
];

/// Arch's `ScriptFlavour`: runs the `post_install` function of each
/// package's `install` script with `bash`, best-effort.
pub struct ArchFlavour;

impl ScriptFlavour for ArchFlavour {
  fn interpreter(&self) -> &'static str {
    "bash"
  }

  fn eligible(&self, entry: &ScriptEntry) -> Result<bool> {
    let install_file = entry.script("install");
    if !install_file.exists() {
      return Ok(false);
    }
    match fs::read_to_string(&install_file) {
      Ok(content) => Ok(content.contains("post_install")),
      Err(_) => Ok(false),
    }
  }

  fn stage(&self, rootfs: &Rootfs, entry: &ScriptEntry, interpreter: &str) -> Result<Option<Vec<String>>> {
    let install_file = entry.script("install");
    let content = fs::read_to_string(&install_file)?;

    // Wrapper without set -e — best effort, don't abort on first failure
    let wrapper = "#!/bin/bash\n\
             source /.flatroot/current-install\n\
             post_install \"$1\" || true\n";

    let wrapper_dest = rootfs.path().join(".flatroot/current-install-wrapper");
    let install_dest = rootfs.path().join(".flatroot/current-install");
    fs::write(&install_dest, &content)?;
    fs::write(&wrapper_dest, wrapper)?;
    fs::set_permissions(&wrapper_dest, fs::Permissions::from_mode(0o755))?;
    fs::set_permissions(&install_dest, fs::Permissions::from_mode(0o755))?;

    Ok(Some(vec![
      interpreter.to_string(),
      "/.flatroot/current-install-wrapper".to_string(),
      "".to_string(),
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
    "install"
  }

  fn cleanup(&self, rootfs: &Rootfs) {
    Fs::remove_lenient(&rootfs.path().join(".flatroot/current-install-wrapper"));
    Fs::remove_lenient(&rootfs.path().join(".flatroot/current-install"));
  }

  fn finish_message(&self) -> &'static str {
    "Install scripts completed"
  }
}
