//! Verify the export destination contract across every shape: a failed
//! export — whether a hard SIGKILL mid-write or a graceful error — leaves
//! nothing at the destination that could be mistaken for a finished
//! artefact. The destination name is created only by the final atomic
//! rename, so the only thing an interrupted run may strand is the
//! unmistakable `.partial` staging sibling.
//!
//! Every shape is driven through its real packer (`mksquashfs`/`mkdwarfs`
//! for sqfs/dwarfs, the in-process tar/oci writers otherwise). A missing
//! tool is an error, not a skip: the export simply fails with the tool's
//! own "not found" message, which surfaces through the assertions below.

mod export;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;

use crate::export::{build_rootfs, run_export};

/// The four export shapes and the destination extension each is driven by.
const FORMATS: &[(&str, &str)] = &[
  ("tar", ".tar.gz"),
  ("oci", ".tar"),
  ("sqfs", ".sqfs"),
  ("dwarfs", ".dwarfs"),
];

/// The OCI shape needs a tag; the others take no extra flags.
fn extra_args(format: &str) -> Vec<&'static str> {
  if format == "oci" {
    vec!["--tag", "app:1"]
  } else {
    Vec::new()
  }
}

/// The `.partial` staging sibling an interrupted export may strand.
fn partial_of(output: &Path) -> PathBuf {
  let mut name = output.as_os_str().to_owned();
  name.push(".partial");
  PathBuf::from(name)
}

/// Grow the fixture rootfs with enough payload that packing takes a
/// measurable amount of time, giving the progressive kill a real window
/// to land inside the write.
fn rootfs_payload_grow(dir: &Path) {
  let path_dir_data = dir.join("usr/share/payload");
  fs::create_dir_all(&path_dir_data).unwrap();
  // Mildly incompressible content so the packer spends time per file.
  let block: Vec<u8> = (0..65536u32)
    .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
    .collect();
  for i in 0..192 {
    fs::write(path_dir_data.join(format!("blob-{i:03}.bin")), &block).unwrap();
  }
}

/// One export attempt killed after `delay`: spawn the real binary, wait
/// out the delay, and SIGKILL it unless it already finished. Returns
/// whether the run exited successfully before the kill.
fn export_run_killed(rootfs: &Path, output: &Path, format: &str, delay: Duration) -> bool {
  let cache = tempfile::tempdir().expect("tempdir for flatroot cache");
  let mut args = vec![
    "export",
    rootfs.to_str().unwrap(),
    output.to_str().unwrap(),
    "--format",
    format,
  ];
  args.extend(extra_args(format));
  let mut child = Command::new(cargo_bin("flatroot"))
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("flatroot failed to spawn");

  std::thread::sleep(delay);
  match child.try_wait().expect("try_wait on flatroot") {
    Some(status) => status.success(),
    None => {
      child.kill().expect("SIGKILL flatroot");
      child.wait().expect("reap killed flatroot");
      false
    }
  }
}

#[test]
fn killed_export_leaves_no_destination_artifact() {
  for (format, suffix) in FORMATS {
    let rootfs_dir = tempfile::tempdir().unwrap();
    build_rootfs(rootfs_dir.path());
    rootfs_payload_grow(rootfs_dir.path());

    let out_dir = tempfile::tempdir().unwrap();
    let path_file_out = out_dir.path().join(format!("out{suffix}"));
    let path_file_partial = partial_of(&path_file_out);

    let mut completed = false;
    for delay_ms in [1u64, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192] {
      if export_run_killed(rootfs_dir.path(), &path_file_out, format, Duration::from_millis(delay_ms)) {
        completed = true;
        break;
      }
      assert!(
        !path_file_out.exists(),
        "{format}: killed export (delay {delay_ms}ms) left an artifact at the destination"
      );
      // A SIGKILL may strand the clearly-marked sibling; clear it so the
      // next iteration starts clean.
      if path_file_partial.exists() {
        fs::remove_file(&path_file_partial).unwrap();
      }
    }

    // The final, unkilled run proves the loop ends in a working export.
    if !completed {
      let out = run_export(rootfs_dir.path(), format, &path_file_out, &extra_args(format));
      assert!(out.status.success(), "{format}: clean export failed: {}", String::from_utf8_lossy(&out.stderr));
    }

    assert!(path_file_out.exists(), "{format}: completed export must land at the destination");
    assert!(
      fs::metadata(&path_file_out).unwrap().len() > 0,
      "{format}: completed export must produce a non-empty artefact"
    );
    assert!(!path_file_partial.exists(), "{format}: completed export must leave no .partial sibling");
  }
}

#[test]
fn failed_export_creates_no_destination_artifact() {
  // A destination whose parent directory does not exist makes every shape
  // fail to create its output — uid-independent, unlike a permission
  // trick — and the failure must leave neither the destination nor a
  // `.partial` staging sibling.
  for (format, suffix) in FORMATS {
    let rootfs_dir = tempfile::tempdir().unwrap();
    build_rootfs(rootfs_dir.path());

    let parent = tempfile::tempdir().unwrap();
    let path_file_out = parent.path().join("missing-subdir").join(format!("out{suffix}"));

    let out = run_export(rootfs_dir.path(), format, &path_file_out, &extra_args(format));
    assert!(!out.status.success(), "{format}: export into a nonexistent directory must fail");
    assert!(!path_file_out.exists(), "{format}: failed export must create no destination");
    assert!(!partial_of(&path_file_out).exists(), "{format}: failed export must leave no .partial sibling");
  }
}
