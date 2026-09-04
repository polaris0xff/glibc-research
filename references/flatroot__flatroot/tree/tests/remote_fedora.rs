//! Tests against dl.fedoraproject.org / archives.fedoraproject.org (Fedora releases).

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn cli_install_bash_fedora42() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "fedora:42",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists(), "Expected /usr/bin/bash in Fedora 42 rootfs");
}

#[test]
fn bwrap_bash_fedora42() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "fedora:42",
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
      "echo fedora42-works",
    ])
    .output()
    .expect("bwrap not available");

  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "fedora42-works");
}

#[test]
fn cli_install_bash_fedora44() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "fedora:44",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists());
}

#[test]
fn cli_install_bash_fedora43() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "fedora:43",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists());
}

#[test]
fn cli_install_bash_fedora41() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "fedora:41",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists());
}

#[test]
fn cli_install_bash_fedora40() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "fedora:40",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists());
}

#[test]
fn cli_install_bash_fedora_rawhide() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "fedora:rawhide",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/bash").exists());
}

// covers: DIST-041
#[test]
fn search_bash_fedora42_aarch64() {
  // The aarch64 arch axis for a numbered fedora release: the releases repo path
  // is releases/42/Everything/aarch64/os (distro/fedora.rs:88-95). A search
  // (index fetch only) must find bash.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "fedora:42", "--arch", "aarch64", "search", "bash"])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(stdout.lines().any(|l| l.ends_with(".name=bash")), "aarch64 fedora42 index must contain bash:\n{}", stdout);
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
      "fedora:42",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "this-package-does-not-exist-xyz123",
    ])
    .assert()
    .failure()
    .stderr(contains("not found"));
}
