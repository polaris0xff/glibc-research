//! Tests against Alpine Linux mirrors.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn cli_install_busybox_edge() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:edge",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "busybox",
    ])
    .assert()
    .success();

  assert!(root.path().join("bin/busybox").exists(), "Expected /bin/busybox in Alpine rootfs");
}

#[test]
fn cli_install_curl_edge() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:edge",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "curl",
    ])
    .assert()
    .success();

  assert!(root.path().join("usr/bin/curl").exists(), "Expected /usr/bin/curl in Alpine rootfs");
}

#[test]
fn bwrap_busybox_edge() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:edge",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "busybox",
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
      "/bin/busybox",
      "echo",
      "alpine-works",
    ])
    .output()
    .expect("bwrap not available");

  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "alpine-works");
}

#[test]
fn cli_install_busybox_v3_23() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.23",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "busybox",
    ])
    .assert()
    .success();

  assert!(root.path().join("bin/busybox").exists());
}

#[test]
fn cli_install_busybox_v3_22() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.22",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "busybox",
    ])
    .assert()
    .success();

  assert!(root.path().join("bin/busybox").exists());
}

#[test]
fn cli_install_busybox_v3_21() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.21",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "busybox",
    ])
    .assert()
    .success();

  assert!(root.path().join("bin/busybox").exists());
}

#[test]
fn cli_install_busybox_v3_20() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.20",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "busybox",
    ])
    .assert()
    .success();

  assert!(root.path().join("bin/busybox").exists());
}

// covers: DIST-049
#[test]
fn cli_install_busybox_v3_21_x86() {
  // `--arch x86` parses to Arch::I686 and projects to Alpine's native 'x86'
  // (arch.rs:130-139, 97-105; distro/alpine.rs:21-25). End-to-end this fetches
  // the x86 APKINDEX and lays the x86 busybox into the rootfs.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.21",
      "--arch",
      "x86",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "busybox",
    ])
    .assert()
    .success();

  assert!(root.path().join("bin/busybox").exists(), "Expected /bin/busybox in Alpine x86 rootfs");
}

// covers: DIST-049
#[test]
fn search_busybox_v3_21_i686_token() {
  // The `i686` token (the other half of the x86 ↔ I686 mapping) also resolves
  // the Alpine 'x86' source; a search against it returns busybox from the x86
  // index, proving the end-to-end native_arch='x86' resolution.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "alpine:v3.21", "--arch", "i686", "search", "busybox"])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(stdout.contains("busybox"), "i686 search must find busybox in the x86 index:\n{}", stdout);
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
      "alpine:edge",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "this-package-does-not-exist-xyz123",
    ])
    .assert()
    .failure()
    .stderr(contains("not found"));
}
