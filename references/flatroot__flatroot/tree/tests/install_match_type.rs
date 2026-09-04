//! `flatroot install --type <library|path>` integration tests. The match
//! space resolves the positional patterns to owning packages before seeding,
//! so a user can install by the library a program failed to load or by the
//! concrete installed path they need on the rootfs. `--no-deps` keeps each
//! case light: the point under test is the resolution, not the closure.

use assert_cmd::Command;
use tempfile::TempDir;

/// Run one install into a fresh root under a fresh cache, returning the
/// command's output and the root directory for inspection.
fn install_run(from: &str, extra: &[&str]) -> (std::process::Output, TempDir) {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let mut args: Vec<&str> = vec!["--from", from, "install", "-o"];
  let root_str = root.path().to_str().unwrap().to_string();
  args.push(&root_str);
  args.extend_from_slice(extra);
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&args)
    .output()
    .unwrap();
  (output, root)
}

#[test]
fn install_type_path_debian_resolves_owner() {
  // Debian Contents records bin/bash under the bash package; the pattern
  // resolves to the owner, the owner is installed, and the record names
  // the package — never the pattern.
  let (output, root) = install_run("debian:bookworm", &["--type", "path", "--no-deps", "bin/bash"]);
  assert!(output.status.success(), "install --type path failed:\n{}", String::from_utf8_lossy(&output.stderr));
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("Resolved 'bin/bash' -> bash"), "the resolution must be echoed, stderr:\n{}", stderr);
  assert!(root.path().join("bin/bash").exists(), "the owning package's payload must land");
  let packages = std::fs::read_to_string(root.path().join(".flatroot/packages")).unwrap();
  assert!(packages.contains("bash"), "the record names the resolved package:\n{}", packages);
}

#[test]
fn install_type_library_alpine_resolves_owner() {
  // Alpine has no file lists, but its `so:` provides cover libraries —
  // library installs work there through the provides leg alone.
  let (output, root) = install_run("alpine:v3.21", &["--type", "library", "--no-deps", "libc.musl-x86_64.so.1"]);
  assert!(output.status.success(), "install --type library failed:\n{}", String::from_utf8_lossy(&output.stderr));
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("Resolved 'libc.musl-x86_64.so.1' -> musl"),
    "the resolution must be echoed, stderr:\n{}",
    stderr
  );
  assert!(root.path().join("lib/libc.musl-x86_64.so.1").exists(), "musl's payload must land");
}

#[test]
fn install_type_path_alpine_is_rejected() {
  // apk publishes no file lists, so the path match space cannot answer
  // there — the request is refused with the reason, not answered empty.
  let (output, _root) = install_run("alpine:v3.21", &["--type", "path", "--no-deps", "usr/bin/bash"]);
  assert!(!output.status.success(), "alpine --type path must fail");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("unsupported for apk sources"), "error must explain the apk gap, stderr:\n{}", stderr);
}

#[test]
fn install_type_path_empty_pattern_fails_naming_it() {
  let (output, _root) = install_run("debian:bookworm", &["--type", "path", "--no-deps", "zzz/nonexistent"]);
  assert!(!output.status.success(), "an empty match must fail, never install less than asked");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("zzz/nonexistent"), "the failing pattern must be named, stderr:\n{}", stderr);
}
