//! Runs each package's saved install scripts — the maintainer scripts a
//! package manager would have run — with the per-distro differences
//! behind the `ScriptFlavour` trait, so the shared
//! find-order-run-report loop is written once.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use super::rootfs::Rootfs;
use super::step_report::StepReport;
use crate::manifest::ManifestLayout;
use crate::package::PackageIdentity;
use crate::sandbox::Sandbox;
use crate::ui::RunVoice;

/// One package's saved install scripts: the package identity plus the
/// directory the scripts were saved into.
pub struct ScriptEntry {
  pub identity: PackageIdentity,
  pub dir: PathBuf,
}

impl ScriptEntry {
  /// Recovers the package identity from the script directory's name,
  /// assuming `arch_default` when the name omits an architecture.
  fn parse(dir: PathBuf, dir_name: &str, arch_default: &str) -> Self {
    Self {
      identity: PackageIdentity::dirname_parse_or(dir_name, arch_default),
      dir,
    }
  }

  /// The path of one named script file inside this package's directory.
  pub fn script(&self, name: &str) -> PathBuf {
    self.dir.join(name)
  }
}

/// The per-distro rules of the script run; implement this and the
/// shared find-order-run-report loop stays distro-agnostic.
pub trait ScriptFlavour {
  /// The program inside the rootfs that runs the scripts; when the
  /// rootfs lacks it there is nothing to run.
  fn interpreter(&self) -> &'static str;

  /// Whether one saved package actually has a script for this pass, so
  /// the progress total reflects real work.
  fn eligible(&self, entry: &ScriptEntry) -> Result<bool>;

  /// Places one package's script where the sandbox can reach it and
  /// returns the argv to run; `None` when there is nothing to run.
  fn stage(&self, rootfs: &Rootfs, entry: &ScriptEntry, interpreter: &str) -> Result<Option<Vec<String>>>;

  /// The environment variables the scripts expect, PATH included.
  fn env(&self, entry: &ScriptEntry) -> Vec<(String, String)>;

  /// The distro's own name for this pass, used in progress lines and
  /// reports.
  fn label(&self) -> &'static str;

  /// Removes whatever `stage` placed, so the finished rootfs does not
  /// keep it.
  fn cleanup(&self, rootfs: &Rootfs);

  /// The closing progress line for this pass.
  fn finish_message(&self) -> &'static str;
}

/// One script run over a rootfs: the sandbox, the `ScriptFlavour`, and
/// the default architecture for script directories that name none.
pub struct ScriptReplay<'a, F: ScriptFlavour> {
  rootfs: Rootfs<'a>,
  sandbox: &'a Sandbox,
  flavour: F,
  arch: &'a str,
}

impl<'a, F: ScriptFlavour> ScriptReplay<'a, F> {
  /// Binds the run to `rootfs` and `sandbox`; `arch` is assumed for
  /// script directories that name no architecture.
  pub fn new(rootfs: Rootfs<'a>, sandbox: &'a Sandbox, flavour: F, arch: &'a str) -> Self {
    Self {
      rootfs,
      sandbox,
      flavour,
      arch,
    }
  }

  /// Runs each eligible package's scripts. A rootfs without the
  /// interpreter runs nothing; the eligible set is counted before
  /// running, so the progress total is accurate; one package's failure
  /// is reported without stopping the rest.
  pub fn run(&self) -> Result<()> {
    let scripts_dir = ManifestLayout::new(self.rootfs.path()).dir_scripts();
    if !scripts_dir.exists() {
      return Ok(());
    }

    // The interpreter is required to run the scripts. Probing through
    // `Rootfs` covers every standard location and uses `symlink_metadata`
    // so an absolute symlink target outside the rootfs does not get
    // resolved against the host filesystem.
    let interpreter = match self.rootfs.binary_find(&[self.flavour.interpreter()], None) {
      Some(b) => b,
      None => return Ok(()),
    };

    let mut dir_entries: Vec<_> = fs::read_dir(&scripts_dir)?.filter_map(|e| e.ok()).collect();
    dir_entries.sort_by_key(|e| e.file_name());

    let entries: Vec<ScriptEntry> = dir_entries
      .into_iter()
      .map(|e| {
        let dir_name = e.file_name().to_string_lossy().to_string();
        ScriptEntry::parse(e.path(), &dir_name, self.arch)
      })
      .collect();

    // Pre-filter to the set that will actually run, so the progress bar's
    // total reflects real work.
    let mut eligible: Vec<ScriptEntry> = Vec::new();
    for entry in entries {
      if self.flavour.eligible(&entry)? {
        eligible.push(entry);
      }
    }

    if eligible.is_empty() {
      return Ok(());
    }

    let pb = RunVoice::bar(eligible.len() as u64, "preparing");

    for entry in eligible {
      let dir_name = entry
        .dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

      let argv = match self.flavour.stage(&self.rootfs, &entry, &interpreter)? {
        Some(a) => a,
        None => {
          pb.inc(1);
          continue;
        }
      };

      pb.set_message(format!("{} {}", self.flavour.label(), dir_name));

      let env = self.flavour.env(&entry);
      let env_ref: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
      let cmd: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();

      let output = self.sandbox.run(&cmd, &env_ref);
      StepReport::print(&pb, self.flavour.label(), &dir_name, &output);

      pb.inc(1);
      self.flavour.cleanup(&self.rootfs);
    }

    RunVoice::finish_ok(&pb, self.flavour.finish_message());
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::{ScriptEntry, ScriptFlavour, ScriptReplay};
  use crate::postinstall::Rootfs;
  use crate::sandbox::Sandbox;
  use anyhow::Result;
  use std::cell::Cell;
  use std::fs;
  use std::rc::Rc;
  use tempfile::TempDir;

  /// A flavour that declares every entry eligible but always declines to stage
  /// it (`stage` returns `None`). The shared counters let the test observe that
  /// the runner offered the entry to `stage` exactly once and then took the
  /// quiet skip path — without invoking the sandbox, `StepReport`, or
  /// `cleanup`, which belong only to the staged (`Some`) branch.
  struct DeclineFlavour {
    staged: Rc<Cell<u32>>,
    cleaned: Rc<Cell<u32>>,
  }

  impl ScriptFlavour for DeclineFlavour {
    fn interpreter(&self) -> &'static str {
      "sh"
    }
    fn eligible(&self, _entry: &ScriptEntry) -> Result<bool> {
      Ok(true)
    }
    fn stage(&self, _rootfs: &Rootfs, _entry: &ScriptEntry, _interpreter: &str) -> Result<Option<Vec<String>>> {
      self.staged.set(self.staged.get() + 1);
      Ok(None)
    }
    fn env(&self, _entry: &ScriptEntry) -> Vec<(String, String)> {
      Vec::new()
    }
    fn label(&self) -> &'static str {
      "test-scripts"
    }
    fn cleanup(&self, _rootfs: &Rootfs) {
      self.cleaned.set(self.cleaned.get() + 1);
    }
    fn finish_message(&self) -> &'static str {
      "test scripts done"
    }
  }

  // covers: POST-044
  #[test]
  fn stage_returning_none_advances_and_skips_without_cleanup() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    // The interpreter the flavour names must resolve inside the rootfs, or
    // `run` reports "nothing to do" before reaching the per-entry loop.
    fs::create_dir_all(root.join("usr/bin")).unwrap();
    fs::write(root.join("usr/bin/sh"), b"#!/bin/sh\n").unwrap();
    // Exactly one staged package directory, so there is one eligible entry.
    fs::create_dir_all(root.join(".flatroot/scripts/pkgfoo")).unwrap();
    fs::write(root.join(".flatroot/scripts/pkgfoo/postinst"), b"#!/bin/sh\nexit 0\n").unwrap();

    let staged = Rc::new(Cell::new(0));
    let cleaned = Rc::new(Cell::new(0));
    let flavour = DeclineFlavour {
      staged: Rc::clone(&staged),
      cleaned: Rc::clone(&cleaned),
    };

    let rootfs = Rootfs::new(root);
    let sandbox = Sandbox::new(root);
    let replay = ScriptReplay::new(rootfs, &sandbox, flavour, "amd64");

    // Declining a staged entry is not a failure — the pass completes Ok.
    replay.run().expect("declining a staged entry must not error the pass");

    assert_eq!(staged.get(), 1, "the one eligible entry must be offered to stage()");
    assert_eq!(cleaned.get(), 0, "a declined (None) entry skips cleanup, which is part of the staged branch only");
  }
}
