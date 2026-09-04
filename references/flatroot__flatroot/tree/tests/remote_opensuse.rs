//! Tests against download.opensuse.org (openSUSE releases).

use assert_cmd::Command;
use flatroot::sandbox::Sandbox;
use predicates::str::contains;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Heavy dependency resolution (350+ packages)
// ---------------------------------------------------------------------------

#[test]
fn install_gimp_tumbleweed() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:tumbleweed",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "gimp",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/gimp").symlink_metadata().is_ok());
}

#[test]
fn install_podman_tumbleweed() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:tumbleweed",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "podman",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/podman").symlink_metadata().is_ok());
}

// ---------------------------------------------------------------------------
// Release coverage
// ---------------------------------------------------------------------------

#[test]
fn install_bash_tumbleweed() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:tumbleweed",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/bash").symlink_metadata().is_ok());
}

#[test]
fn install_bash_leap_15_5() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:15.5",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/bash").symlink_metadata().is_ok());
}

#[test]
fn install_bash_leap() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:15.6",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/bash").symlink_metadata().is_ok());
}

// ---------------------------------------------------------------------------
// Execution verification
// ---------------------------------------------------------------------------

#[test]
fn sandbox_echo_tumbleweed() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:tumbleweed",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "coreutils",
    ])
    .assert()
    .success();

  // openSUSE compat-usrmerge-tools creates circular symlinks for /usr/bin/bash
  // and /bin/sh, so we use /usr/bin/echo (a real coreutils binary) instead.
  let output = Sandbox::new(root.path())
    .run(&["/usr/bin/echo", "opensuse-works"], &[("PATH", "/usr/bin:/bin"), ("HOME", "/root")])
    .expect("Sandbox::run failed");
  assert!(output.status.success(), "exit: {}", output.status);
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "opensuse-works");
}

// ---------------------------------------------------------------------------
// --no-deps isolation
// ---------------------------------------------------------------------------

#[test]
fn install_no_deps_tumbleweed() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:tumbleweed",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--no-deps",
      "curl",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/curl").symlink_metadata().is_ok());
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

// covers: DIST-043
#[test]
fn search_bash_tumbleweed_aarch64_ports() {
  // The aarch64 'ports' path branch: tumbleweed on aarch64 reads
  // ports/aarch64/tumbleweed/repo/oss (distro/opensuse.rs:62-66). A search
  // (index fetch only) must find bash, exercising the ports path end-to-end.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "opensuse:tumbleweed", "--arch", "aarch64", "search", "bash"])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(
    stdout.lines().any(|l| l.ends_with(".name=bash")),
    "aarch64 tumbleweed ports index must contain bash:\n{}",
    stdout
  );
}

#[test]
fn install_nonexistent_package_fails() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:tumbleweed",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "this-package-does-not-exist-xyz123",
    ])
    .assert()
    .failure()
    .stderr(contains("not found"));
}

#[test]
fn install_mix_valid_invalid_fails() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "opensuse:tumbleweed",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
      "this-package-does-not-exist-xyz123",
    ])
    .assert()
    .failure()
    .stderr(contains("not found"));
}
