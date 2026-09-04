//! Round-trip integration tests for every export format. Each test
//! installs the same realistic reference rootfs (`gimp` on
//! `debian:bookworm`) into a fresh tempdir, runs the export, extracts
//! the resulting artefact with the format's standard extractor into a
//! second tempdir, and asserts the two trees are byte-faithful: same
//! paths, same modes, same regular-file contents, same symlink
//! targets. For OCI the test additionally loads the image into Docker
//! and runs a shell command inside the resulting container.
//!
//! The reference install costs around three minutes per test, four
//! tests, no shared rootfs across them — sharing a process-wide
//! rootfs via `OnceLock` would leak on graceful exit (Rust statics
//! are never dropped) and tests would violate the project's "clean
//! /tmp on every exit path" rule. Slow but correct.
//!
//! Every external tool the tests need is hard-required via
//! `require_tool`. A missing tool fails the test loudly rather than
//! returning early with an `ok` result, matching the
//! `export_exclusion.rs` precedent.

mod export;

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::export::{
  fresh_output_path, install_assert_ok, install_with_cache, oci_layer_blob, require_tool, run_export,
};

/// One entry's worth of information needed to assert a faithful
/// round-trip. Regular files carry mode + content hash; symlinks carry
/// the link target as recorded by `read_link`; directories carry mode.
/// Other file types (devices, fifos, sockets) are not produced by
/// `flatroot install` and would indicate a corrupted rootfs if seen.
#[derive(Debug, PartialEq, Eq)]
enum FileEntry {
  File { mode: u32, sha256: [u8; 32] },
  Symlink { target: PathBuf },
  Dir { mode: u32 },
}

/// Walk `root` recursively and produce a path -> entry map. The top-
/// level `.flatroot/` directory is skipped because every exporter
/// excludes it on purpose, so neither side should ever contain it.
/// Symlinks are recorded as symlinks (not followed) so dangling
/// absolute links in the rootfs do not resolve against the host.
fn collect_tree(root: &Path) -> BTreeMap<PathBuf, FileEntry> {
  let mut out = BTreeMap::new();
  walk(root, root, &mut out);
  out
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, FileEntry>) {
  for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display())) {
    let entry = entry.unwrap();
    let path = entry.path();
    let rel = path
      .strip_prefix(root)
      .unwrap_or_else(|_| panic!("strip_prefix {} from {}", root.display(), path.display()))
      .to_path_buf();

    if rel == Path::new(".flatroot") || rel.starts_with(".flatroot") {
      continue;
    }

    let metadata = fs::symlink_metadata(&path).unwrap_or_else(|e| panic!("symlink_metadata {}: {e}", path.display()));
    let file_type = metadata.file_type();
    let mode = metadata.permissions().mode() & 0o7777;

    if file_type.is_symlink() {
      let target = fs::read_link(&path).unwrap_or_else(|e| panic!("read_link {}: {e}", path.display()));
      out.insert(rel, FileEntry::Symlink { target });
    } else if file_type.is_dir() {
      out.insert(rel.clone(), FileEntry::Dir { mode });
      walk(root, &path, out);
    } else if file_type.is_file() {
      let sha256 = hash_file(&path);
      out.insert(rel, FileEntry::File { mode, sha256 });
    } else {
      panic!("unexpected file type at {}", path.display());
    }
  }
}

fn hash_file(path: &Path) -> [u8; 32] {
  let mut f = fs::File::open(path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
  let mut hasher = Sha256::new();
  let mut buf = [0u8; 64 * 1024];
  loop {
    let n = f
      .read(&mut buf)
      .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    if n == 0 {
      break;
    }
    hasher.update(&buf[..n]);
  }
  hasher.finalize().into()
}

/// Compare two trees. The path set must match exactly; for each shared
/// path the `FileEntry` must match. Mismatch reporter names the first
/// differing element so a failure points at the actual divergence
/// instead of dumping the whole map.
fn assert_trees_equal(label: &str, expected: &BTreeMap<PathBuf, FileEntry>, actual: &BTreeMap<PathBuf, FileEntry>) {
  let mut missing: Vec<&PathBuf> = expected.keys().filter(|k| !actual.contains_key(*k)).collect();
  let mut extra: Vec<&PathBuf> = actual.keys().filter(|k| !expected.contains_key(*k)).collect();
  missing.sort();
  extra.sort();
  assert!(
    missing.is_empty() && extra.is_empty(),
    "{label}: path set mismatch\n  missing from export ({} paths): {:?}\n  extra in export ({} paths): {:?}",
    missing.len(),
    missing.iter().take(20).collect::<Vec<_>>(),
    extra.len(),
    extra.iter().take(20).collect::<Vec<_>>(),
  );

  for (path, expected_entry) in expected {
    let actual_entry = actual.get(path).expect("path-set check passed; key must exist");
    assert!(
      expected_entry == actual_entry,
      "{label}: entry mismatch at {}\n  expected: {:?}\n  actual:   {:?}",
      path.display(),
      expected_entry,
      actual_entry,
    );
  }
}

/// Build the reference rootfs by running `flatroot install gimp` on
/// `debian:bookworm`. Panics with stderr on failure.
fn install_gimp_debian(rootfs: &Path, cache: &Path) {
  let out = install_with_cache("debian:bookworm", rootfs, cache, "gimp");
  install_assert_ok(&out, "gimp install (debian:bookworm)");
}

/// Shell out to `tar -xpzf <archive> -C <dest>` and assert it
/// succeeds. The `-p` flag tells tar to preserve the archived modes
/// instead of masking them through the caller's umask, which is what
/// tar does by default when run as a non-root user. The exporter
/// records faithful modes in the tar header; the extractor's umask
/// behaviour is what would otherwise drop information on the round
/// trip.
fn extract_targz(archive: &Path, dest: &Path) {
  let out = Command::new("tar")
    .args(["-xpzf", archive.to_str().unwrap(), "-C", dest.to_str().unwrap()])
    .output()
    .expect("tar failed to execute");
  assert!(
    out.status.success(),
    "tar -xpzf {} -C {} exited {:?}\nstderr:\n{}",
    archive.display(),
    dest.display(),
    out.status.code(),
    String::from_utf8_lossy(&out.stderr),
  );
}

/// Extract the OCI archive's rootfs layer blob into `dest`. The outer
/// tar is unpacked to a tempdir, the gzip layer blob is located, and
/// then extracted with `extract_targz`.
fn extract_oci(archive: &Path, dest: &Path) -> TempDir {
  let (unpack, blob) = oci_layer_blob(archive);
  extract_targz(&blob, dest);
  unpack
}

/// Shell out to `dwarfsextract -i <archive> -o <dest>` and assert it
/// succeeds.
fn extract_dwarfs(archive: &Path, dest: &Path) {
  let out = Command::new("dwarfsextract")
    .args(["-i", archive.to_str().unwrap(), "-o", dest.to_str().unwrap()])
    .output()
    .expect("dwarfsextract failed to execute");
  assert!(
    out.status.success(),
    "dwarfsextract exited {:?}\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr),
  );
}

/// Shell out to `unsquashfs -f -d <dest> <archive>` and assert it
/// succeeds. The `-f` flag lets unsquashfs overwrite the empty
/// destination tempdir, which exists by the time `extract_sqfs` is
/// called.
fn extract_sqfs(archive: &Path, dest: &Path) {
  let out = Command::new("unsquashfs")
    .args(["-f", "-d", dest.to_str().unwrap(), archive.to_str().unwrap()])
    .output()
    .expect("unsquashfs failed to execute");
  assert!(
    out.status.success(),
    "unsquashfs exited {:?}\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr),
  );
}

#[test]
fn tar_export_roundtrips_gimp_debian() {
  require_tool("tar");

  let rootfs = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  install_gimp_debian(rootfs.path(), cache.path());
  let expected = collect_tree(rootfs.path());

  let (_output_dir, output) = fresh_output_path(".tar.gz");
  let out = run_export(rootfs.path(), "tar", &output, &[]);
  assert!(out.status.success(), "tar export failed: {}", String::from_utf8_lossy(&out.stderr));

  let extracted = TempDir::new().unwrap();
  extract_targz(&output, extracted.path());
  let actual = collect_tree(extracted.path());

  assert_trees_equal("tar.gz", &expected, &actual);
}

// covers: EXP-051 (docker half + host-arch)
#[test]
fn oci_export_roundtrips_gimp_debian_and_docker_loads() {
  require_tool("tar");
  require_tool("docker");

  let rootfs = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  install_gimp_debian(rootfs.path(), cache.path());
  let expected = collect_tree(rootfs.path());

  let tag = format!("flatroot-roundtrip-{}:v1", std::process::id());
  let (_output_dir, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output, &["--tag", &tag]);
  assert!(out.status.success(), "oci export failed: {}", String::from_utf8_lossy(&out.stderr));

  let extracted = TempDir::new().unwrap();
  let _unpack = extract_oci(&output, extracted.path());
  let actual = collect_tree(extracted.path());
  assert_trees_equal("oci layer", &expected, &actual);

  // Loadability: docker accepts the OCI archive and the image lands in
  // the local daemon. `docker load -i` on an OCI archive needs Docker
  // >= 25; older daemons fail here, which is intentional — the test
  // requires real loadability, not a permissive "best effort".
  let load = Command::new("docker")
    .args(["load", "-i", output.to_str().unwrap()])
    .output()
    .expect("docker load failed to execute");
  assert!(
    load.status.success(),
    "docker load failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
    load.status.code(),
    String::from_utf8_lossy(&load.stdout),
    String::from_utf8_lossy(&load.stderr),
  );

  let inspect = Command::new("docker")
    .args(["inspect", &tag])
    .output()
    .expect("docker inspect failed to execute");
  assert!(
    inspect.status.success(),
    "docker inspect {} failed (exit {:?})\nstderr:\n{}",
    tag,
    inspect.status.code(),
    String::from_utf8_lossy(&inspect.stderr),
  );

  // Runnability: the loaded image actually launches a container and the
  // rootfs's /usr/bin/gimp is reachable from inside.
  let run = Command::new("docker")
    .args(["run", "--rm", &tag, "/usr/bin/sh", "-c", "ls -la /usr/bin/gimp"])
    .output()
    .expect("docker run failed to execute");
  let stdout = String::from_utf8_lossy(&run.stdout);
  assert!(
    run.status.success(),
    "docker run failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
    run.status.code(),
    stdout,
    String::from_utf8_lossy(&run.stderr),
  );
  assert!(stdout.contains("gimp"), "docker run stdout does not name /usr/bin/gimp:\n{stdout}",);

  // Best-effort cleanup — exit code intentionally ignored so a
  // background daemon hiccup does not turn a passed test into a
  // failure. Leftover tags surface to a manual `docker rmi $(docker
  // images --filter reference=flatroot-roundtrip-* -q)`.
  let _ = Command::new("docker").args(["rmi", "-f", &tag]).output();
}

#[test]
fn dwarfs_export_roundtrips_gimp_debian() {
  require_tool("mkdwarfs");
  require_tool("dwarfsextract");

  let rootfs = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  install_gimp_debian(rootfs.path(), cache.path());
  let expected = collect_tree(rootfs.path());

  let (_output_dir, output) = fresh_output_path(".dwarfs");
  let out = run_export(rootfs.path(), "dwarfs", &output, &[]);
  assert!(out.status.success(), "dwarfs export failed: {}", String::from_utf8_lossy(&out.stderr));

  let extracted = TempDir::new().unwrap();
  extract_dwarfs(&output, extracted.path());
  let actual = collect_tree(extracted.path());

  assert_trees_equal("dwarfs", &expected, &actual);
}

// covers: EXP-051
#[test]
fn oci_export_loads_and_runs_under_podman() {
  require_tool("podman");

  // A real, minimal install (alpine busybox) gives a self-contained,
  // runnable rootfs — a genuine /bin/sh plus its musl loader, built for the
  // host architecture and carrying nothing copied from the host — which is
  // what lets the exported OCI image actually launch under podman. The docker
  // half of the engine axis is covered by
  // `oci_export_roundtrips_gimp_debian_and_docker_loads`; this is the podman
  // half, kept cheap by installing busybox rather than a full application.
  let rootfs = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let install = install_with_cache("alpine:v3.21", rootfs.path(), cache.path(), "busybox");
  install_assert_ok(&install, "busybox install (alpine:v3.21)");

  let tag = format!("flatroot-podman-{}:v1", std::process::id());
  let (_output_dir, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output, &["--tag", &tag]);
  assert!(out.status.success(), "oci export failed: {}", String::from_utf8_lossy(&out.stderr));

  let load = Command::new("podman")
    .args(["load", "-i", output.to_str().unwrap()])
    .output()
    .expect("podman load failed to execute");
  assert!(
    load.status.success(),
    "podman load failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
    load.status.code(),
    String::from_utf8_lossy(&load.stdout),
    String::from_utf8_lossy(&load.stderr),
  );

  let inspect = Command::new("podman")
    .args(["image", "inspect", &tag])
    .output()
    .expect("podman image inspect failed to execute");
  assert!(
    inspect.status.success(),
    "podman image inspect {} failed (exit {:?})\nstderr:\n{}",
    tag,
    inspect.status.code(),
    String::from_utf8_lossy(&inspect.stderr),
  );

  // Runnability: the loaded image launches and its /bin/sh executes.
  let run = Command::new("podman")
    .args(["run", "--rm", &tag, "/bin/sh", "-c", "echo podman-ok"])
    .output()
    .expect("podman run failed to execute");
  let stdout = String::from_utf8_lossy(&run.stdout);
  assert!(
    run.status.success(),
    "podman run failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
    run.status.code(),
    stdout,
    String::from_utf8_lossy(&run.stderr),
  );
  assert!(stdout.contains("podman-ok"), "podman run did not execute /bin/sh:\n{stdout}");

  let _ = Command::new("podman").args(["rmi", "-f", &tag]).output();
}

#[test]
fn sqfs_export_roundtrips_gimp_debian() {
  require_tool("mksquashfs");
  require_tool("unsquashfs");

  let rootfs = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  install_gimp_debian(rootfs.path(), cache.path());
  let expected = collect_tree(rootfs.path());

  let (_output_dir, output) = fresh_output_path(".sqfs");
  let out = run_export(rootfs.path(), "sqfs", &output, &[]);
  assert!(out.status.success(), "sqfs export failed: {}", String::from_utf8_lossy(&out.stderr));

  let extracted = TempDir::new().unwrap();
  extract_sqfs(&output, extracted.path());
  let actual = collect_tree(extracted.path());

  assert_trees_equal("sqfs", &expected, &actual);
}
