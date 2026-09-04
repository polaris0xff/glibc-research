//! Shared helpers for export-related integration tests. Both
//! `export_exclusion.rs` and `export_roundtrip.rs` invoke the real
//! `flatroot` binary against a rootfs under a per-test cache and need
//! the same handful of utilities — tool probing, subprocess invocation,
//! fresh output paths, and OCI outer-tar dissection. Keeping them in
//! one place avoids the per-test-binary copy-paste this codebase
//! otherwise accumulates.
//!
//! Each test binary compiles its own copy of this module and uses a
//! subset of the helpers, so the unused-function lint would fire per
//! binary against helpers another binary needs.

#![allow(dead_code)]

use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

/// Hard requirement on an external binary. Panics with a specific
/// message so a missing tool fails loudly at the top of every test
/// rather than returning early with an `ok` result.
pub fn require_tool(name: &'static str) {
  which::which(name).unwrap_or_else(|_| panic!("'{name}' is required on PATH to run the export integration tests"));
}

/// Invoke `flatroot export --format <format> <rootfs> <output>` with
/// the per-call cache rooted in a fresh tempdir.
pub fn run_export(rootfs: &Path, format: &str, output: &Path, extra: &[&str]) -> Output {
  let cache = tempfile::tempdir().expect("tempdir for flatroot cache");
  let bin = cargo_bin("flatroot");
  let mut cmd = Command::new(bin);
  cmd.env("FLATROOT_CACHE_HOME", cache.path());
  cmd.args([
    "export",
    rootfs.to_str().unwrap(),
    output.to_str().unwrap(),
    "--format",
    format,
  ]);
  cmd.args(extra);
  cmd.output().expect("flatroot failed to execute")
}

/// Invoke `flatroot export <rootfs> <output>` with NO `--format` flag,
/// so the exporter must infer the format from the output extension (or
/// refuse). Used by the extension-inference and ambiguity tests.
pub fn run_export_no_format(rootfs: &Path, output: &Path, extra: &[&str]) -> Output {
  let cache = tempfile::tempdir().expect("tempdir for flatroot cache");
  let bin = cargo_bin("flatroot");
  let mut cmd = Command::new(bin);
  cmd.env("FLATROOT_CACHE_HOME", cache.path());
  cmd.args(["export", rootfs.to_str().unwrap(), output.to_str().unwrap()]);
  cmd.args(extra);
  cmd.output().expect("flatroot failed to execute")
}

/// Invoke `flatroot export` with caller-controlled positional and flag
/// args plus an arbitrary set of extra environment variables, on top of
/// the isolated cache. Returns the raw subprocess output. Used by the
/// env-fallback (`FLATROOT_ARG_EXPORT_*`) and PATH-manipulation tests.
pub fn run_export_raw(args: &[&str], envs: &[(&str, &std::ffi::OsStr)]) -> Output {
  let cache = tempfile::tempdir().expect("tempdir for flatroot cache");
  let bin = cargo_bin("flatroot");
  let mut cmd = Command::new(bin);
  cmd.env("FLATROOT_CACHE_HOME", cache.path());
  for (k, v) in envs {
    cmd.env(k, v);
  }
  cmd.arg("export");
  cmd.args(args);
  cmd.output().expect("flatroot failed to execute")
}

/// Build a minimal but valid rootfs at `dir`: a synthetic ELF `bin/sh`
/// (an `e_machine`-bearing header written here, never copied from the
/// host, so the fixture is byte-identical on every machine the suite runs
/// on while still giving the exporter's architecture detector an ELF to
/// parse), a small payload file, and a realistic top-level `.flatroot/`
/// metadata subtree the exporters must exclude.
///
/// The detector reads only the first 20 bytes of `bin/sh`, so a header is
/// enough for the format/tool/metadata tests that never execute it; the one
/// test that actually runs the rootfs (the podman OCI test) installs a real
/// busybox rootfs instead. Centralised here because every export test binary
/// needs the same fixture shape and the architecture-detector's ELF
/// requirement is a trap each one would otherwise rediscover.
pub fn build_rootfs(dir: &Path) {
  fs::create_dir_all(dir.join("bin")).unwrap();
  fs::create_dir_all(dir.join("usr/bin")).unwrap();

  // 64-bit little-endian ELF header naming x86_64 in its `e_machine` field
  // (bytes 18-19) — enough to drive `Arch::detect` without copying any
  // binary from the host.
  let mut header = vec![0u8; 32];
  header[0..4].copy_from_slice(b"\x7fELF");
  header[4] = 2; // EI_CLASS = ELFCLASS64
  header[5] = 1; // EI_DATA  = little-endian
  header[18] = 62; // EM_X86_64
  fs::write(dir.join("bin/sh"), &header).unwrap();

  fs::write(dir.join("usr/bin/hello"), b"hello").unwrap();

  fs::create_dir_all(dir.join(".flatroot/files")).unwrap();
  fs::write(
    dir.join(".flatroot/manifest"),
    b"FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: x86_64\nPackageCount: 0",
  )
  .unwrap();
  fs::write(dir.join(".flatroot/packages"), b"").unwrap();
}

/// Create a directory holding a single fake executable named `tool` that
/// runs the supplied shell `body`, and return both the TempDir (kept
/// alive by the caller) and its path. Putting this directory at the
/// front of `PATH` lets a test intercept `mkdwarfs`/`mksquashfs` to
/// record their argv or force a non-zero exit, without touching the
/// crate source. The body runs under `/bin/sh`.
pub fn fake_tool_dir(tool: &str, body: &str) -> (TempDir, PathBuf) {
  use std::os::unix::fs::PermissionsExt;
  let dir = tempfile::tempdir().expect("tempdir for fake tool");
  let path_bin = dir.path().join(tool);
  fs::write(&path_bin, format!("#!/bin/sh\n{body}\n")).expect("write fake tool");
  let mut perms = fs::metadata(&path_bin).unwrap().permissions();
  perms.set_mode(0o755);
  fs::set_permissions(&path_bin, perms).unwrap();
  let path = dir.path().to_path_buf();
  (dir, path)
}

/// Invoke `flatroot --from <remote> install --output <rootfs> <pkg>`
/// against a caller-supplied cache directory so callers can choose
/// whether to reuse downloads across calls.
pub fn install_with_cache(remote: &str, rootfs: &Path, cache: &Path, pkg: &str) -> Output {
  let bin = cargo_bin("flatroot");
  Command::new(bin)
    .env("FLATROOT_CACHE_HOME", cache)
    .args(["--from", remote, "install", "--output", rootfs.to_str().unwrap(), pkg])
    .output()
    .expect("flatroot failed to execute")
}

/// Panic on a non-zero install exit, with the captured stderr in the
/// message so the real upstream failure is visible in test output.
pub fn install_assert_ok(out: &Output, label: &str) {
  assert!(
    out.status.success(),
    "{}: flatroot install failed (exit={:?})\nstderr:\n{}",
    label,
    out.status.code(),
    String::from_utf8_lossy(&out.stderr),
  );
}

/// Produce a path inside a fresh tempdir with the file not yet
/// created. Some exporters refuse to overwrite an existing file, so
/// the path must be unused on entry.
pub fn fresh_output_path(suffix: &str) -> (TempDir, PathBuf) {
  let dir = tempfile::tempdir().expect("tempdir for export output");
  let path = dir.path().join(format!("out{suffix}"));
  (dir, path)
}

/// Extract the outer OCI tar and return the path of its gzip-compressed
/// layer blob (identified by the `1f 8b` gzip magic).
pub fn oci_layer_blob(outer_tar: &Path) -> (TempDir, PathBuf) {
  let unpack = tempfile::tempdir().expect("tempdir for OCI unpack");
  let f = fs::File::open(outer_tar).expect("open OCI outer tar");
  let mut archive = tar::Archive::new(BufReader::new(f));
  archive.unpack(unpack.path()).expect("unpack OCI outer tar");

  let blobs = unpack.path().join("blobs/sha256");
  for entry in fs::read_dir(&blobs).expect("read OCI blobs/sha256") {
    let entry = entry.unwrap();
    let mut head = [0u8; 2];
    let mut f = fs::File::open(entry.path()).unwrap();
    if f.read_exact(&mut head).is_ok() && head == [0x1f, 0x8b] {
      let p = entry.path();
      return (unpack, p);
    }
  }
  panic!("no gzip layer blob in OCI output");
}
