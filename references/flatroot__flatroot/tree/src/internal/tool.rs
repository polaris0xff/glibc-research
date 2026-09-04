//! `Tool`: an external program some export formats delegate to
//! (`mkdwarfs`, `mksquashfs`), confirmed installed before work begins;
//! a failed run is an export failure.

use std::os::fd::AsFd;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// An external program an export depends on and the host is expected to
/// provide, located once so later failures can name it.
pub struct Tool {
  name: &'static str,
  path_bin: PathBuf,
}

impl Tool {
  /// Confirms the tool is on `PATH` up front, so its absence is
  /// reported as a missing prerequisite before any real work starts.
  pub fn locate(name: &'static str) -> Result<Self> {
    let path_bin = which::which(name)
      .map_err(|_| anyhow::anyhow!("{} not found in PATH. Install it with your system package manager.", name))?;
    Ok(Self { name, path_bin })
  }

  /// Runs the tool and treats anything short of success as a failure
  /// named after the tool, so a half-finished or absent result is never
  /// mistaken for a completed export.
  ///
  /// The tool streams progress to its stdout, which under the output
  /// contract is reserved for flatroot's command data; it is forwarded
  /// to stderr (a dup of fd 2) so a consumer piping flatroot's stdout
  /// never sees it while a watching user still does.
  pub fn run(&self, args: &[&str]) -> Result<()> {
    let fd_stderr = std::io::stderr()
      .as_fd()
      .try_clone_to_owned()
      .context("failed to duplicate stderr for the packaging tool")?;
    let status = Command::new(&self.path_bin)
      .args(args)
      .stdout(Stdio::from(fd_stderr))
      .status()?;
    if !status.success() {
      bail!("{} failed (exit {})", self.name, status.code().unwrap_or(-1));
    }
    Ok(())
  }
}
