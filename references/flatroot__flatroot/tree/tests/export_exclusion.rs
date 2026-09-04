//! Verify every export format excludes the top-level `<root>/.flatroot/`
//! directory and passes through any other `.flatroot` path unchanged.
//!
//! Each test builds a rootfs containing both a realistic top-level
//! `.flatroot/` metadata subtree AND a nested `opt/project/.flatroot/`
//! directory buried in payload. The export is run via the real `flatroot`
//! binary, and the output is inspected with the format's own listing tool
//! — `tar -tzf`, `tar` on the OCI layer blob, `dwarfsck -l`, `unsquashfs -l`.
//!
//! These four tools are required. Missing any of them is a test failure,
//! not a silent skip: an export backend with no verification is strictly
//! worse than no export backend at all.

mod export;

use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;

use crate::export::{fresh_output_path, oci_layer_blob, require_tool, run_export};

/// Build a rootfs with payload, a realistic top-level `.flatroot/`
/// metadata subtree, and a nested `.flatroot` directory buried inside a
/// user payload path. The nested one is what distinguishes "exclusion
/// targeting the flatroot metadata dir" from "exclusion matching any
/// component named .flatroot anywhere in the tree".
///
/// `bin/sh` is a real ELF copied from the host so the OCI exporter's
/// architecture detector has something to parse.
fn build_test_rootfs(dir: &Path) {
  fs::create_dir_all(dir.join("bin")).unwrap();
  fs::create_dir_all(dir.join("etc")).unwrap();
  fs::create_dir_all(dir.join("usr/bin")).unwrap();

  let host_sh = ["/bin/sh", "/usr/bin/sh"]
    .iter()
    .map(PathBuf::from)
    .find(|p| {
      p.exists()
        && fs::read(p)
          .map(|b| b.starts_with(&[0x7f, b'E', b'L', b'F']))
          .unwrap_or(false)
    })
    .expect("host has no ELF /bin/sh to copy into the test rootfs");
  fs::copy(&host_sh, dir.join("bin/sh")).unwrap();

  fs::write(dir.join("etc/hostname"), b"flatroot\n").unwrap();
  fs::write(dir.join("usr/bin/hello"), b"hello").unwrap();

  fs::create_dir_all(dir.join(".flatroot/files")).unwrap();
  fs::create_dir_all(dir.join(".flatroot/scripts/bash:x86_64")).unwrap();
  fs::write(
    dir.join(".flatroot/manifest"),
    b"FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: x86_64\nPackageCount: 0",
  )
  .unwrap();
  fs::write(dir.join(".flatroot/packages"), b"").unwrap();
  fs::write(dir.join(".flatroot/files/bash:x86_64"), b"/bin/sh").unwrap();
  fs::write(dir.join(".flatroot/scripts/bash:x86_64/postinst"), b"#!/bin/sh\n").unwrap();

  fs::create_dir_all(dir.join("opt/project/.flatroot")).unwrap();
  fs::write(dir.join("opt/project/.flatroot/marker"), b"nested-marker\n").unwrap();
  fs::write(dir.join("opt/project/.flatroot/config"), b"nested-config\n").unwrap();
}

fn list_targz_entries(path: &Path) -> Vec<String> {
  let f = fs::File::open(path).unwrap();
  let gz = GzDecoder::new(BufReader::new(f));
  let mut archive = tar::Archive::new(gz);
  archive
    .entries()
    .unwrap()
    .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
    .collect()
}

fn list_tar_entries(path: &Path) -> Vec<String> {
  let f = fs::File::open(path).unwrap();
  let mut archive = tar::Archive::new(BufReader::new(f));
  archive
    .entries()
    .unwrap()
    .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
    .collect()
}

fn list_dwarfs_entries(path: &Path) -> Vec<String> {
  let out = Command::new("dwarfsck")
    .arg("-l")
    .arg(path)
    .output()
    .expect("dwarfsck -l failed to execute");
  assert!(out.status.success(), "dwarfsck -l exited {:?}: {}", out.status.code(), String::from_utf8_lossy(&out.stderr));
  String::from_utf8_lossy(&out.stdout)
    .lines()
    .map(|s| s.trim_start_matches('/').to_string())
    .filter(|s| !s.is_empty())
    .collect()
}

/// `unsquashfs -l` prefixes each entry with `squashfs-root`. Strip that
/// prefix and drop the bare `squashfs-root` root entry so the listing
/// shape matches the other backends.
fn list_sqfs_entries(path: &Path) -> Vec<String> {
  let out = Command::new("unsquashfs")
    .arg("-l")
    .arg(path)
    .output()
    .expect("unsquashfs -l failed to execute");
  assert!(
    out.status.success(),
    "unsquashfs -l exited {:?}: {}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stdout)
    .lines()
    .filter_map(|line| {
      let line = line.trim();
      if line.is_empty() || line == "squashfs-root" {
        return None;
      }
      Some(
        line
          .strip_prefix("squashfs-root/")
          .unwrap_or(line.trim_start_matches('/'))
          .to_string(),
      )
    })
    .collect()
}

fn has_top_level_flatroot(paths: &[String]) -> bool {
  paths.iter().any(|p| {
    let n = p.trim_start_matches("./");
    n == ".flatroot" || n.starts_with(".flatroot/")
  })
}

fn has_nested_flatroot(paths: &[String]) -> bool {
  paths.iter().any(|p| {
    let n = p.trim_start_matches("./");
    n == "opt/project/.flatroot" || n.starts_with("opt/project/.flatroot/")
  })
}

/// Shared assertion body: every export format must exclude the top-level
/// `.flatroot/` subtree and preserve a nested `.flatroot` found deeper in
/// the payload.
fn assert_exclusion_invariants(label: &str, entries: &[String]) {
  // The payload we planted is exactly `bin/sh`. Accept the `./`-prefixed form
  // that tar emits; anything else indicates a path-shape bug in the exporter.
  assert!(
    entries.iter().any(|p| p == "bin/sh" || p == "./bin/sh"),
    "{label}: payload 'bin/sh' missing from export:\n{entries:#?}"
  );
  assert!(!has_top_level_flatroot(entries), "{label}: top-level .flatroot/ leaked:\n{entries:#?}");
  assert!(has_nested_flatroot(entries), "{label}: nested opt/project/.flatroot was removed:\n{entries:#?}");
}

#[test]
fn tar_export_excludes_top_level_flatroot_and_preserves_nested() {
  let rootfs = tempfile::tempdir().unwrap();
  build_test_rootfs(rootfs.path());

  let (_keep, output_path) = fresh_output_path(".tar.gz");
  let out = run_export(rootfs.path(), "tar", &output_path, &[]);
  assert!(out.status.success(), "tar export failed: {}", String::from_utf8_lossy(&out.stderr));

  assert_exclusion_invariants("tar", &list_targz_entries(&output_path));
}

#[test]
fn oci_export_excludes_top_level_flatroot_and_preserves_nested() {
  let rootfs = tempfile::tempdir().unwrap();
  build_test_rootfs(rootfs.path());

  let (_keep, output_path) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output_path, &["--tag", "flatroot-test:v1"]);
  assert!(out.status.success(), "oci export failed: {}", String::from_utf8_lossy(&out.stderr));

  let (_unpack, layer) = oci_layer_blob(&output_path);
  assert_exclusion_invariants("oci layer", &list_targz_entries(&layer));
}

#[test]
fn dwarfs_export_excludes_top_level_flatroot_and_preserves_nested() {
  require_tool("mkdwarfs");
  require_tool("dwarfsck");

  let rootfs = tempfile::tempdir().unwrap();
  build_test_rootfs(rootfs.path());

  let (_keep, output_path) = fresh_output_path(".dwarfs");
  let out = run_export(rootfs.path(), "dwarfs", &output_path, &[]);
  assert!(out.status.success(), "dwarfs export failed: {}", String::from_utf8_lossy(&out.stderr));

  assert_exclusion_invariants("dwarfs", &list_dwarfs_entries(&output_path));
}

#[test]
fn sqfs_export_excludes_top_level_flatroot_and_preserves_nested() {
  require_tool("mksquashfs");
  require_tool("unsquashfs");

  let rootfs = tempfile::tempdir().unwrap();
  build_test_rootfs(rootfs.path());

  let (_keep, output_path) = fresh_output_path(".sqfs");
  let out = run_export(rootfs.path(), "sqfs", &output_path, &[]);
  assert!(out.status.success(), "sqfs export failed: {}", String::from_utf8_lossy(&out.stderr));

  assert_exclusion_invariants("sqfs", &list_sqfs_entries(&output_path));
}

#[test]
fn oci_outer_tar_has_expected_layout() {
  let rootfs = tempfile::tempdir().unwrap();
  build_test_rootfs(rootfs.path());

  let (_keep, output_path) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output_path, &["--tag", "ft:v1"]);
  assert!(out.status.success(), "oci export failed: {}", String::from_utf8_lossy(&out.stderr));

  let entries = list_tar_entries(&output_path);
  // Three single-file fixtures plus one directory prefix under which the
  // blob files live. Exact-match the files and prefix-match the directory;
  // `.contains()` would pass spuriously on similarly-named unrelated paths.
  for exact in ["oci-layout", "index.json", "manifest.json"] {
    assert!(
      entries.iter().any(|p| p == exact || p == &format!("./{exact}")),
      "oci outer tar missing '{exact}':\n{entries:#?}"
    );
  }
  assert!(
    entries.iter().any(|p| {
      let n = p.trim_start_matches("./");
      n.starts_with("blobs/sha256/")
    }),
    "oci outer tar missing any blobs/sha256/* entry:\n{entries:#?}"
  );
}
