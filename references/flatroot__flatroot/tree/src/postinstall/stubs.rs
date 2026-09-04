//! `StubSet`: puts stub commands and pre-seeded files ahead of the real
//! tools, so a distro's privileged install scripts run to completion in
//! an unprivileged build even though none of the privileged effects are
//! real.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::Result;

use super::rootfs::Rootfs;

/// Installs stub programs and pre-seeded files into one rootfs.
pub struct StubSet<'a> {
  rootfs: Rootfs<'a>,
}

impl<'a> StubSet<'a> {
  /// Binds stub installation to one rootfs.
  pub fn new(rootfs: Rootfs<'a>) -> Self {
    Self { rootfs }
  }

  /// The `.flatroot/bin` directory the scripts' PATH lists before the
  /// real system locations, created on first use.
  fn bin_dir(&self) -> Result<PathBuf> {
    let bin_dir = self.rootfs.path().join(".flatroot/bin");
    fs::create_dir_all(&bin_dir)?;
    Ok(bin_dir)
  }

  /// Installs stubs that do nothing and exit 0, one per name.
  pub fn noop(&self, names: &[&str]) -> Result<()> {
    let bin_dir = self.bin_dir()?;
    for name in names {
      let path = bin_dir.join(name);
      fs::write(&path, "#!/bin/sh\nexit 0\n")?;
      fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
  }

  /// Installs a no-op stub only where none exists, so an earlier
  /// tailored stub is never overwritten.
  pub fn noop_keep(&self, names: &[&str]) -> Result<()> {
    let bin_dir = self.bin_dir()?;
    for name in names {
      let path = bin_dir.join(name);
      if !path.exists() {
        fs::write(&path, "#!/bin/sh\nexit 0\n")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
      }
    }
    Ok(())
  }

  /// Installs one stub with the given script body, for a command whose
  /// answer the scripts branch on.
  pub fn script(&self, name: &str, body: &str) -> Result<()> {
    let path = self.bin_dir()?.join(name);
    fs::write(&path, body)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    Ok(())
  }

  /// Installs a stub with the given body only where none exists.
  pub fn script_keep(&self, name: &str, body: &str) -> Result<()> {
    let path = self.bin_dir()?.join(name);
    if !path.exists() {
      fs::write(&path, body)?;
      fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
  }

  /// Writes a file at `rel` inside the rootfs, for a script expecting a
  /// live system to have placed it; `exec` marks it executable.
  pub fn seed(&self, rel: &str, body: &str, exec: bool) -> Result<()> {
    let path = self.rootfs.path().join(rel);
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(&path, body)?;
    if exec {
      fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
  }

  /// Writes the file only where nothing exists, so a package's own copy
  /// is never overwritten.
  pub fn seed_keep(&self, rel: &str, body: &str) -> Result<()> {
    let path = self.rootfs.path().join(rel);
    if !path.exists() {
      if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
      }
      fs::write(&path, body)?;
    }
    Ok(())
  }
}
