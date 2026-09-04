//! Tests against Arch Linux mirrors.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn cli_install_bash_arch() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "arch:rolling",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists(), "Expected /usr/bin/bash in Arch rootfs");
}

#[test]
fn cli_install_curl_arch() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "arch:rolling",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "curl",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/curl").exists(), "Expected /usr/bin/curl in Arch rootfs");
}

#[test]
fn bwrap_bash_arch() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "arch:rolling",
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
      "echo arch-works",
    ])
    .output()
    .expect("bwrap not available");

  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "arch-works");
}

#[test]
fn cli_install_lib32_glibc_multilib() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "arch:multilib",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "lib32-glibc",
    ])
    .assert()
    .success();

  assert!(
    root.path().join("usr/lib32/libc.so.6").symlink_metadata().is_ok(),
    "Expected /usr/lib32/libc.so.6 in Arch multilib rootfs",
  );
}

#[test]
fn cli_install_nonexistent_package_fails() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "arch:rolling",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "this-package-does-not-exist-xyz123",
    ])
    .assert()
    .failure()
    .stderr(contains("not found"));
}
