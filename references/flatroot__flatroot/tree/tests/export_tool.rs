//! Tool-driven export paths: the DwarFS and SquashFS exporters that
//! shell out to `mkdwarfs`/`mksquashfs`, plus the failure surfaces the
//! whole export pipeline owns — a missing external tool, a tool that
//! exits non-zero, non-UTF-8 source/output paths, and `File::create`
//! failures for the in-process formats.
//!
//! The missing-tool and tool-failure cases are driven by manipulating
//! `PATH`: either clearing it so `which` cannot find the tool, or
//! pointing it at a fake script that records its argv / exits non-zero.
//! `flatroot` itself is invoked by absolute path, so a cleared `PATH`
//! does not stop the binary from running.
//!
//! Every error string is copied verbatim from the cited source:
//! `src/commands/export/{tool,source,dwarfs,sqfs,tar}.rs`.

#![allow(dead_code)]

mod export;

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use assert_cmd::cargo::cargo_bin;

use crate::export::{build_rootfs, fake_tool_dir, fresh_output_path};

fn stderr_of(out: &Output) -> String {
  String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Run `flatroot export` with the given OsStr args and an explicit
/// environment overlay, under an isolated cache. Accepts OsStr args so
/// non-UTF-8 paths can be exercised. Returns the raw output.
fn run_export_os(args: &[&OsStr], envs: &[(&str, &OsStr)]) -> Output {
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

/// A PATH value with only an empty directory in it, so `which` finds no
/// external tool. The tempdir is returned so the caller keeps it alive.
fn empty_path() -> (tempfile::TempDir, OsString) {
  let dir = tempfile::tempdir().expect("tempdir for empty PATH");
  let path = dir.path().as_os_str().to_os_string();
  (dir, path)
}

// ---------------------------------------------------------------------------
// Missing external tool.
// ---------------------------------------------------------------------------

// covers: EXP-017
#[test]
fn dwarfs_missing_tool_is_reported() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".dwarfs");
  let (_path_keep, path) = empty_path();

  let out = run_export_os(
    &[
      rootfs.path().as_os_str(),
      output.as_os_str(),
      OsStr::new("--format"),
      OsStr::new("dwarfs"),
    ],
    &[("PATH", path.as_os_str())],
  );
  assert!(!out.status.success(), "dwarfs export with no mkdwarfs unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("mkdwarfs not found in PATH. Install it with your system package manager."),
    "wrong/missing mkdwarfs-missing error:\n{}",
    stderr_of(&out)
  );
  assert!(!output.exists(), "no output should be written when mkdwarfs is missing");
}

// covers: EXP-018
#[test]
fn sqfs_missing_tool_is_reported() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".sqfs");
  let (_path_keep, path) = empty_path();

  let out = run_export_os(
    &[
      rootfs.path().as_os_str(),
      output.as_os_str(),
      OsStr::new("--format"),
      OsStr::new("sqfs"),
    ],
    &[("PATH", path.as_os_str())],
  );
  assert!(!out.status.success(), "sqfs export with no mksquashfs unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("mksquashfs not found in PATH. Install it with your system package manager."),
    "wrong/missing mksquashfs-missing error:\n{}",
    stderr_of(&out)
  );
  assert!(!output.exists(), "no output should be written when mksquashfs is missing");
}

// ---------------------------------------------------------------------------
// Tool exits non-zero.
// ---------------------------------------------------------------------------

// covers: EXP-019
#[test]
fn dwarfs_tool_nonzero_exit_is_reported() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".dwarfs");
  let (_tool_keep, tool_dir) = fake_tool_dir("mkdwarfs", "exit 7");

  let path_env = tool_dir.display().to_string();
  let out = run_export_os(
    &[
      rootfs.path().as_os_str(),
      output.as_os_str(),
      OsStr::new("--format"),
      OsStr::new("dwarfs"),
    ],
    &[("PATH", OsStr::new(&path_env))],
  );
  assert!(!out.status.success(), "dwarfs export with a failing mkdwarfs unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("mkdwarfs failed (exit 7)"),
    "wrong/missing mkdwarfs-failed error:\n{}",
    stderr_of(&out)
  );
}

// covers: EXP-020
#[test]
fn sqfs_tool_nonzero_exit_is_reported() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".sqfs");
  let (_tool_keep, tool_dir) = fake_tool_dir("mksquashfs", "exit 5");

  let path_env = tool_dir.display().to_string();
  let out = run_export_os(
    &[
      rootfs.path().as_os_str(),
      output.as_os_str(),
      OsStr::new("--format"),
      OsStr::new("sqfs"),
    ],
    &[("PATH", OsStr::new(&path_env))],
  );
  assert!(!out.status.success(), "sqfs export with a failing mksquashfs unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("mksquashfs failed (exit 5)"),
    "wrong/missing mksquashfs-failed error:\n{}",
    stderr_of(&out)
  );
}

// ---------------------------------------------------------------------------
// Non-UTF-8 paths.
// ---------------------------------------------------------------------------

// covers: EXP-046
#[test]
fn dwarfs_sqfs_reject_non_utf8_output_path() {
  // Source is valid and the tool is present (intercepted), so the only
  // gate left is the output `to_str()` check inside dwarfs/sqfs.
  for (tool, format, suffix) in [
    ("mkdwarfs", "dwarfs", b".dwarfs".as_slice()),
    ("mksquashfs", "sqfs", b".sqfs".as_slice()),
  ] {
    let rootfs = tempfile::tempdir().unwrap();
    build_rootfs(rootfs.path());
    let (_tool_keep, tool_dir) = fake_tool_dir(tool, "exit 0");

    // Build a non-UTF-8 output path inside a fresh tempdir.
    let out_dir = tempfile::tempdir().unwrap();
    let mut name = b"out\xff".to_vec();
    name.extend_from_slice(suffix);
    let output: PathBuf = out_dir.path().join(OsStr::from_bytes(&name));

    let path_env = tool_dir.display().to_string();
    let out = run_export_os(
      &[
        rootfs.path().as_os_str(),
        output.as_os_str(),
        OsStr::new("--format"),
        OsStr::new(format),
      ],
      &[("PATH", OsStr::new(&path_env))],
    );
    assert!(!out.status.success(), "{format}: non-UTF-8 output unexpectedly succeeded");
    assert!(
      stderr_of(&out).contains("Output path is not valid UTF-8"),
      "{format}: wrong/missing non-UTF-8-output error:\n{}",
      stderr_of(&out)
    );
  }
}

// covers: EXP-047
#[test]
fn dwarfs_sqfs_reject_non_utf8_source_path() {
  // Source directory has a non-UTF-8 name but is a real, arch-detectable
  // rootfs and the tool is present, so the failure is the source
  // `path_str()` check inside dwarfs/sqfs.
  for (tool, format) in [("mkdwarfs", "dwarfs"), ("mksquashfs", "sqfs")] {
    let parent = tempfile::tempdir().unwrap();
    let src: PathBuf = parent.path().join(OsStr::from_bytes(b"root\xff"));
    fs::create_dir(&src).unwrap();
    build_rootfs(&src);

    let (_tool_keep, tool_dir) = fake_tool_dir(tool, "exit 0");
    let (_keep, output) = fresh_output_path(if format == "dwarfs" { ".dwarfs" } else { ".sqfs" });

    let path_env = tool_dir.display().to_string();
    let out = run_export_os(
      &[
        src.as_os_str(),
        output.as_os_str(),
        OsStr::new("--format"),
        OsStr::new(format),
      ],
      &[("PATH", OsStr::new(&path_env))],
    );
    assert!(!out.status.success(), "{format}: non-UTF-8 source unexpectedly succeeded");
    assert!(
      stderr_of(&out).contains("Source directory path is not valid UTF-8"),
      "{format}: wrong/missing non-UTF-8-source error:\n{}",
      stderr_of(&out)
    );
  }
}

// ---------------------------------------------------------------------------
// File::create failure for the in-process formats.
// ---------------------------------------------------------------------------

// covers: EXP-048
#[test]
fn tar_and_oci_output_create_failure_is_contextualised() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());

  // Parent directory does not exist -> File::create fails for both.
  let dir = tempfile::tempdir().unwrap();

  let out_tar = dir.path().join("missing-parent/out.tar.gz");
  let r_tar = run_export_os(
    &[
      rootfs.path().as_os_str(),
      out_tar.as_os_str(),
      OsStr::new("--format"),
      OsStr::new("tar"),
    ],
    &[],
  );
  assert!(!r_tar.status.success(), "tar export to a missing parent dir unexpectedly succeeded");
  assert!(
    stderr_of(&r_tar).contains(&format!("Failed to create {}", out_tar.display())),
    "tar: missing 'Failed to create' context:\n{}",
    stderr_of(&r_tar)
  );

  let out_oci = dir.path().join("missing-parent/out.tar");
  let r_oci = run_export_os(
    &[
      rootfs.path().as_os_str(),
      out_oci.as_os_str(),
      OsStr::new("--format"),
      OsStr::new("oci"),
      OsStr::new("--tag"),
      OsStr::new("app:1"),
    ],
    &[],
  );
  assert!(!r_oci.status.success(), "oci export to a missing parent dir unexpectedly succeeded");
  assert!(
    stderr_of(&r_oci).contains(&format!("Failed to create {}", out_oci.display())),
    "oci: missing 'Failed to create' context:\n{}",
    stderr_of(&r_oci)
  );
}
