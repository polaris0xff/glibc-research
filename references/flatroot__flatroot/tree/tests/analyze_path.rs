//! `flatroot analyze trace --type=path` integration tests. The path
//! match space seeds the trace from whichever packages ship paths
//! matching the pattern, drawing on the distribution's file lists —
//! so one case verifies a full path resolves to its owning package
//! and seeds the trace (`reason=target`), and one verifies the apk
//! family, which publishes no file lists, is refused with the reason
//! rather than answered with an empty trace.

use assert_cmd::Command;
use tempfile::TempDir;

mod analyze;

#[test]
fn analyze_path_debian_bash() {
  // Debian Contents records bin/bash under the bash package; the
  // trace must seed from that ownership fact.
  let outcome = analyze::run_analyze_args("debian:bookworm", &["--type=path", "bin/bash"]);
  analyze::assert_seed_target(&outcome, "bash");
}

#[test]
fn analyze_path_alpine_is_rejected() {
  // apk publishes no file lists, so the path match space cannot answer
  // there — the request is refused with the reason, not answered empty.
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.21",
      "analyze",
      "trace",
      "--type=path",
      "usr/bin/bash",
    ])
    .output()
    .unwrap();
  assert!(!output.status.success(), "alpine --type=path must fail");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("unsupported for apk sources"), "error must explain the apk gap, stderr:\n{}", stderr);
}
