//! Helpers shared by the CLI-surface test crates, following the same
//! shared-module pattern the resolver, analyze, export, and post-install
//! suites already use for theirs.

use assert_cmd::Command;
use tempfile::TempDir;

/// A `flatroot` invocation with its cache isolated to a fresh tempdir. The
/// tempdir is returned alongside so the caller keeps it alive across the run.
pub fn flatroot() -> (Command, TempDir) {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  cmd.env("FLATROOT_CACHE_HOME", cache.path());
  (cmd, cache)
}
