//! Tests for the Debian backend — current suites, archived suites, and snapshots.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Current suites (deb.debian.org)
// ---------------------------------------------------------------------------

#[test]
fn install_bash_bookworm() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("bin/bash").exists());
}

#[test]
fn install_python3_bookworm() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "python3-minimal",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/python3.11").exists());
}

#[test]
fn bwrap_bash_bookworm() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  let output = std::process::Command::new("bwrap")
    .args([
      "--bind",
      root.path().to_str().unwrap(),
      "/",
      "--dev",
      "/dev",
      "--proc",
      "/proc",
      "/bin/bash",
      "-c",
      "echo hello",
    ])
    .output()
    .expect("bwrap not available");
  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
}

#[test]
fn install_multiarch_amd64_i386() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64,i686",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "dash",
    ])
    .assert()
    .success();

  assert!(root.path().join("bin/dash").exists());
  assert!(
    root.path().join("lib/i386-linux-gnu").exists() || root.path().join("usr/lib/i386-linux-gnu").exists(),
    "Expected i386 multiarch lib directory"
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
      "debian:bookworm",
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
      "debian:bookworm",
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

#[test]
fn install_no_deps() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--no-deps",
      "curl",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/curl").exists());
  assert!(!root.path().join("lib/x86_64-linux-gnu/libc.so.6").exists(), "libc should not be installed with --no-deps");
}

#[test]
fn install_no_deps_nonexistent_fails() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--no-deps",
      "this-package-does-not-exist-xyz123",
    ])
    .assert()
    .failure()
    .stderr(contains("not found"));
}

#[test]
fn install_bash_trixie() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:trixie",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("bin/bash").exists());
}

#[test]
fn install_bash_forky() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:forky",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("bin/bash").exists());
}

#[test]
fn install_bash_bullseye() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bullseye",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("bin/bash").exists());
}

// ---------------------------------------------------------------------------
// Archived suites (archive.debian.org)
// ---------------------------------------------------------------------------

#[test]
fn install_bash_buster() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:buster",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("bin/bash").exists());
}

#[test]
fn bwrap_bash_buster_glibc_version() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:buster",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  let output = std::process::Command::new("bwrap")
    .args([
      "--bind",
      root.path().to_str().unwrap(),
      "/",
      "--dev",
      "/dev",
      "--proc",
      "/proc",
      "/lib/x86_64-linux-gnu/libc.so.6",
    ])
    .output()
    .expect("bwrap not available");
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("2.28"), "Expected glibc 2.28, got: {}", stdout);
}

#[test]
fn install_bash_stretch() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:stretch",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("bin/bash").exists());
}

// ---------------------------------------------------------------------------
// Snapshots (snapshot.debian.org)
// ---------------------------------------------------------------------------

#[test]
fn install_bash_bookworm_pinned() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm@2024-06-15",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("bin/bash").exists());
}

#[test]
fn bwrap_bash_bookworm_pinned() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm@2024-06-15",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  let output = std::process::Command::new("bwrap")
    .args([
      "--bind",
      root.path().to_str().unwrap(),
      "/",
      "--dev",
      "/dev",
      "--proc",
      "/proc",
      "/bin/bash",
      "-c",
      "echo snapshot-works",
    ])
    .output()
    .expect("bwrap not available");
  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "snapshot-works");
}

#[test]
fn install_buster_pinned() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:buster@2023-01-01",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();
  assert!(root.path().join("bin/bash").exists());
}
