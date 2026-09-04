//! Tests against mirror.cachyos.org (CachyOS rolling).

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Heavy dependency resolution (270+ packages)
// ---------------------------------------------------------------------------

#[test]
fn install_thunderbird_cachyos() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "cachyos:rolling",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "thunderbird",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/lib/thunderbird").exists());
}

#[test]
fn install_git_cachyos() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "cachyos:rolling",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "git",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/git").exists());
}

// ---------------------------------------------------------------------------
// Execution verification
// ---------------------------------------------------------------------------

#[test]
fn bwrap_bash_cachyos() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "cachyos:rolling",
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
      "/usr/bin/bash",
      "-c",
      "echo cachyos-works",
    ])
    .output()
    .expect("bwrap not available");
  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "cachyos-works");
}

// ---------------------------------------------------------------------------
// --no-deps isolation
// ---------------------------------------------------------------------------

#[test]
fn install_no_deps_cachyos() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "cachyos:rolling",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--no-deps",
      "curl",
    ])
    .assert()
    .success();
  assert!(root.path().join("usr/bin/curl").exists());
  assert!(!root.path().join("usr/lib/libc.so.6").exists(), "libc should not be installed with --no-deps");
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn install_nonexistent_package_fails() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "cachyos:rolling",
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
      "cachyos:rolling",
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
