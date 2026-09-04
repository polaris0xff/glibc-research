//! Tests against archive.ubuntu.com (Ubuntu releases).

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn cli_install_bash_noble() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "ubuntu:noble",
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
fn cli_install_python3_noble() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "ubuntu:noble",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "python3-minimal",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/python3").exists() || root.path().join("usr/bin/python3.12").exists(),);
}

#[test]
fn ubuntu_universe_package() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  // htop is in universe, not main — verifies multi-component fetch
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "ubuntu:noble",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "htop",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/htop").exists());
}

#[test]
fn cli_install_bash_jammy() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "ubuntu:jammy",
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
fn cli_install_bash_bionic() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "ubuntu:bionic",
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
fn cli_install_bash_xenial() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "ubuntu:xenial",
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
fn ubuntu_focal_old_lts() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "ubuntu:focal",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(root.path().join("bin/bash").exists());
}

// covers: DIST-046
#[test]
fn search_bash_noble_at_date_snapshot() {
  // ubuntu:noble@2024-06-15 routes both archive and security to
  // snapshot.ubuntu.com (distro/ubuntu.rs:73-90). End-to-end, a search against
  // the dated snapshot reads the pinned index and finds bash — proving the
  // snapshot mirror serves the suite at that day.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "ubuntu:noble@2024-06-15", "search", "bash"])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(stdout.lines().any(|l| l.ends_with(".name=bash")), "snapshot index must contain bash:\n{}", stdout);
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
      "ubuntu:noble",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "this-package-does-not-exist-xyz123",
    ])
    .assert()
    .failure()
    .stderr(contains("not found"));
}
