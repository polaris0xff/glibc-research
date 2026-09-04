//! Tests against vault.centos.org (CentOS releases).

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn cli_install_bash_centos7() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "centos:7",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists(), "Expected /usr/bin/bash in CentOS 7 rootfs");
}

#[test]
fn cli_install_bash_centos8() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "centos:8",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists(), "Expected /usr/bin/bash in CentOS 8 rootfs");
}

#[test]
fn cli_install_bash_centos_stream9() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "centos:stream9",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists(), "Expected /usr/bin/bash in CentOS Stream 9 rootfs");
}

#[test]
fn bwrap_bash_centos7() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "centos:7",
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
      "echo centos7-works",
    ])
    .output()
    .expect("bwrap not available");

  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "centos7-works");
}

// covers: DIST-039
#[test]
fn search_bash_stream9_aarch64() {
  // The aarch64 arch axis for the stream host: centos:stream9 on aarch64 reads
  // 9-stream/BaseOS/aarch64/os repodata (distro/centos.rs:76-78). A search
  // (index fetch only, no download) must find bash.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "centos:stream9", "--arch", "aarch64", "search", "bash"])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(stdout.lines().any(|l| l.ends_with(".name=bash")), "aarch64 stream9 index must contain bash:\n{}", stdout);
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
      "centos:7",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "this-package-does-not-exist-xyz123",
    ])
    .assert()
    .failure()
    .stderr(contains("not found"));
}
