//! End-to-end manifest integration: exercise install + manifest read
//! through the real pipeline against Debian's live index.
//!
//! These tests require network. They are written to run in a CI with
//! internet access alongside the existing `workflow_idempotent` suite.
//!
//! A second, network-free section drives the manifest subsystem directly
//! through its public API (`ManifestState::{merge,write,read,source_admit}`,
//! `PackageRecord`, `Checksum`, `ManifestLayout::is_metadata`). The manifest's
//! on-disk shape — `.flatroot/{manifest,packages,files/<name>:<arch>}` — is
//! constructed against that layout so the read path's consistency checks
//! (PackageCount, Sources/Architectures headers, files-ledger presence, unknown
//! keys) can be exercised on hand-written trees.

use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use flatroot::manifest::{Checksum, ChecksumAlgorithm, ManifestLayout, ManifestState, PackageRecord};
use flatroot::package::{DepSpec, Dependency, PackageIdentity};
use std::path::Path;
use tempfile::TempDir;

fn install_with_cache(remote: &str, root: &str, cache: &str, pkgs: &[&str]) -> std::process::Output {
  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  cmd
    .env("FLATROOT_CACHE_HOME", cache)
    .args(["--from", remote, "install", "--output", root]);
  for p in pkgs {
    cmd.arg(p);
  }
  cmd.output().expect("flatroot failed to execute")
}

fn assert_success(out: &std::process::Output, label: &str) {
  assert!(
    out.status.success(),
    "{label}: exit={:?}\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr)
  );
}

#[test]
fn debian_install_writes_manifest_and_filelists() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out =
    install_with_cache("debian:bookworm", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), &["bash"]);
  assert_success(&out, "debian install");

  let meta = root.path().join(".flatroot");
  assert!(meta.join("manifest").exists(), "missing manifest");
  assert!(meta.join("packages").exists(), "missing packages");
  let files_dir = meta.join("files");
  let entries: Vec<_> = fs::read_dir(&files_dir)
    .unwrap()
    .filter_map(|e| e.ok())
    .map(|e| e.file_name())
    .collect();
  assert!(!entries.is_empty(), "no per-package file lists");

  let packages = fs::read_to_string(meta.join("packages")).unwrap();
  // Every Url: line is a fully qualified HTTPS URL; no `|` encoding leaks.
  for line in packages.lines().filter(|l| l.starts_with("Url: ")) {
    assert!(line.starts_with("Url: https://"), "not https: {line}");
    assert!(!line.contains('|'), "encoding leaked: {line}");
  }

  // bash's file list must be non-empty, every path absolute, and no .flatroot
  // entries leaked in. Using `all()` catches partial corruption (e.g. three
  // absolute paths plus one garbage line); `any()` would let that slide.
  let bash_list = fs::read_to_string(files_dir.join("bash:x86_64")).expect("bash:x86_64 file list");
  let lines: Vec<&str> = bash_list.lines().collect();
  assert!(!lines.is_empty(), "bash file list is empty");
  assert!(lines.iter().all(|l| l.starts_with('/')), "bash file list has non-absolute entries: {lines:#?}");
  for line in &lines {
    assert!(!line.starts_with("/.flatroot"), "leaked metadata path: {line}");
  }
}

#[test]
fn debian_reinstall_is_byte_identical() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  assert_success(&install_with_cache("debian:bookworm", root_str, cache_str, &["bash"]), "first install");
  let meta = root.path().join(".flatroot");
  let m1 = fs::read(meta.join("manifest")).unwrap();
  let p1 = fs::read(meta.join("packages")).unwrap();

  assert_success(&install_with_cache("debian:bookworm", root_str, cache_str, &["bash"]), "second install");
  let m2 = fs::read(meta.join("manifest")).unwrap();
  let p2 = fs::read(meta.join("packages")).unwrap();
  assert_eq!(m1, m2, "manifest changed between identical installs");
  assert_eq!(p1, p2, "packages file changed between identical installs");

  // Every per-package file list must have at least one entry — a package
  // that wrote zero files is a broken extractor, not a valid install.
  let entries: Vec<_> = fs::read_dir(meta.join("files"))
    .unwrap()
    .filter_map(|e| e.ok())
    .map(|e| e.file_name())
    .collect();
  for name in &entries {
    let path = meta.join("files").join(name);
    let buf = fs::read(&path).unwrap();
    assert!(!buf.is_empty(), "empty file list for {}", path.display());
  }
}

#[test]
fn cross_distro_install_is_refused() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  assert_success(&install_with_cache("debian:bookworm", root_str, cache_str, &["bash"]), "seed debian");
  let meta = root.path().join(".flatroot");
  let before_manifest = fs::read(meta.join("manifest")).unwrap();

  let out = install_with_cache("ubuntu:noble", root_str, cache_str, &["bash"]);
  assert!(!out.status.success(), "ubuntu-into-debian should have failed");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("scoped to one exact source"), "missing strict-source wording: {stderr}");

  let after_manifest = fs::read(meta.join("manifest")).unwrap();
  assert_eq!(before_manifest, after_manifest, "rejected install modified the manifest");
}

#[test]
fn same_release_different_date_is_refused() {
  // Pinning a previously-unpinned rootfs (or changing the pin date) is a
  // source-string change and the strict compatibility check refuses it
  // before any network access.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  assert_success(&install_with_cache("debian:bookworm", root_str, cache_str, &["bash"]), "seed unpinned");
  let meta = root.path().join(".flatroot");
  let before_manifest = fs::read(meta.join("manifest")).unwrap();

  let out = install_with_cache("debian:bookworm@2024-06-15", root_str, cache_str, &["bash"]);
  assert!(!out.status.success(), "pinned-into-unpinned should have failed");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("scoped to one exact source"), "missing strict-source wording: {stderr}");
  assert!(stderr.contains("debian:bookworm@2024-06-15"), "stderr should name the incoming source: {stderr}");

  let after_manifest = fs::read(meta.join("manifest")).unwrap();
  assert_eq!(before_manifest, after_manifest, "rejected install modified the manifest");
}

#[test]
fn malformed_manifest_rejects_install_without_work() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let meta = root.path().join(".flatroot");
  fs::create_dir_all(meta.join("files")).unwrap();
  // A manifest that fails to parse (malformed header).
  fs::write(meta.join("manifest"), "this is not a manifest").unwrap();
  fs::write(meta.join("packages"), "").unwrap();

  let out =
    install_with_cache("debian:bookworm", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), &["bash"]);
  assert!(!out.status.success(), "install should have refused the malformed manifest");
  assert_eq!(
    fs::read_to_string(meta.join("manifest")).unwrap(),
    "this is not a manifest",
    "manifest was overwritten despite the refusal"
  );
}

// covers: CORE-005
#[test]
fn same_distro_different_release_is_refused_unchanged() {
  // Seed a rootfs from debian:bookworm, then attempt a second install from the
  // same distro but a different release (trixie). The cross-distro guardrail
  // refuses it before any network/disk work, naming the strict-source rule, and
  // the manifest is left byte-identical.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  assert_success(&install_with_cache("debian:bookworm", root_str, cache_str, &["bash"]), "seed bookworm");
  let meta = root.path().join(".flatroot");
  let before_manifest = fs::read(meta.join("manifest")).unwrap();
  let before_packages = fs::read(meta.join("packages")).unwrap();

  let out = install_with_cache("debian:trixie", root_str, cache_str, &["bash"]);
  assert!(!out.status.success(), "trixie-into-bookworm (same distro, different release) should have failed");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("scoped to one exact source"), "missing strict-source wording: {stderr}");
  assert!(stderr.contains("debian:trixie"), "stderr must name the incoming release: {stderr}");

  assert_eq!(before_manifest, fs::read(meta.join("manifest")).unwrap(), "rejected install modified the manifest");
  assert_eq!(before_packages, fs::read(meta.join("packages")).unwrap(), "rejected install modified the packages file");
}

// covers: CORE-021
#[test]
fn install_extraction_sweep_skips_only_the_flatroot_subtree() {
  // After a real install, the rootfs holds genuine content (bin/bash) AND the
  // tool's own `.flatroot/` metadata corner. The extraction sweep attributes
  // package content but skips exactly the `.flatroot` subtree per is_metadata:
  // no per-package file ledger may claim a path inside `.flatroot/`, and the
  // genuine top-level content is present.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out =
    install_with_cache("debian:bookworm", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), &["bash"]);
  assert_success(&out, "install for is_metadata sweep");

  // Genuine content is present; the metadata corner exists separately.
  assert!(root.path().join("bin/bash").exists(), "real content must be installed");
  let meta = root.path().join(".flatroot");
  assert!(meta.join("manifest").exists(), "the .flatroot metadata corner must exist");

  // No package ledger attributes any path inside `.flatroot/` — the predicate
  // would have excluded it from the sweep.
  for entry in fs::read_dir(meta.join("files")).unwrap().flatten() {
    let body = fs::read_to_string(entry.path()).unwrap();
    for line in body.lines() {
      let rel = line.trim_start_matches('/');
      assert!(
        !ManifestLayout::is_metadata(Path::new(rel)),
        "package ledger {:?} leaked a .flatroot metadata path: {line}",
        entry.file_name()
      );
      assert!(!rel.starts_with(".flatroot"), "ledger entry must not point inside .flatroot: {line}");
    }
  }

  // The predicate itself: only the .flatroot subtree is metadata; genuine
  // content and a top-level `flatroot/` (no dot) are not.
  assert!(ManifestLayout::is_metadata(Path::new(".flatroot")));
  assert!(ManifestLayout::is_metadata(Path::new(".flatroot/manifest")));
  assert!(!ManifestLayout::is_metadata(Path::new("usr/bin/bash")));
  assert!(!ManifestLayout::is_metadata(Path::new("flatroot/something")));
}

// ===========================================================================
// Manifest subsystem — direct public-API tests (network-free)
// ===========================================================================

/// Build a checksum-bearing record for `(name, arch, version)` with a couple of
/// dependencies and a small file list. All fields are public; this mirrors the
/// shape the install pipeline produces.
fn sample_record(name: &str, arch: &str, version: &str) -> PackageRecord {
  PackageRecord {
    name: name.to_string(),
    version: version.to_string(),
    architecture: arch.to_string(),
    source: "debian:bookworm".to_string(),
    url: format!("https://deb.debian.org/debian/pool/main/{}/{}_{}.deb", &name[..1], name, version),
    checksum: Checksum {
      algorithm: ChecksumAlgorithm::Sha256,
      hex: "a4f8".to_string(),
    },
    size: 1234,
    depends: vec![Dependency {
      alternatives: vec![DepSpec {
        name: "libc6".to_string(),
        version_constraint: Some(">= 2.36".to_string()),
      }],
    }],
    files: vec![
      PathBuf::from(format!("usr/bin/{name}")),
      PathBuf::from(format!("usr/share/doc/{name}/copyright")),
    ],
  }
}

fn write_meta(root: &Path, manifest: &str, packages: &str, ledgers: &[(&str, &str)]) {
  let meta = root.join(".flatroot");
  fs::create_dir_all(meta.join("files")).unwrap();
  fs::write(meta.join("manifest"), manifest).unwrap();
  fs::write(meta.join("packages"), packages).unwrap();
  for (key, body) in ledgers {
    fs::write(meta.join("files").join(key), body).unwrap();
  }
}

// covers: CORE-010
#[test]
fn merge_replaces_a_record_when_version_or_checksum_differs() {
  // A second merge carrying the same (name, arch) but a changed version must
  // replace the prior record; an identical re-merge is a no-op. (The "replacing
  // ..." line merge emits goes to process stderr; the verifiable substance is
  // the record replacement itself.)
  let first = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.1")], "0.1.0");
  let key = PackageIdentity {
    name: "bash".to_string(),
    arch: "x86_64".to_string(),
  };
  assert_eq!(first.packages[&key].version, "5.1");

  let updated = sample_record("bash", "x86_64", "5.2");
  let merged = ManifestState::merge(Some(first.clone()), vec![updated.clone()], "0.1.0");
  assert_eq!(merged.packages[&updated.key()].version, "5.2", "changed version must replace the prior record");

  // A changed checksum (same version) also replaces.
  let mut rechecksummed = sample_record("bash", "x86_64", "5.2");
  rechecksummed.checksum = Checksum {
    algorithm: ChecksumAlgorithm::Sha256,
    hex: "deadbeef".to_string(),
  };
  let merged2 = ManifestState::merge(Some(merged.clone()), vec![rechecksummed.clone()], "0.1.0");
  assert_eq!(merged2.packages[&rechecksummed.key()].checksum.hex, "deadbeef", "changed checksum must replace");

  // An identical re-merge is a no-op.
  let noop = ManifestState::merge(Some(merged2.clone()), vec![rechecksummed], "0.1.0");
  assert_eq!(noop, merged2, "an identical merge must be a no-op");
}

// covers: CORE-011
#[test]
fn merge_recomputes_sources_and_architectures_from_the_final_set() {
  // Two arches of the same package: merge must recompute Architectures as the
  // exact set of the final records ({x86_64, i686}) and Sources from them —
  // recomputed, not patched, so they can never drift.
  let records = vec![
    sample_record("libc6", "x86_64", "2.36"),
    sample_record("libc6", "i686", "2.36"),
  ];
  let state = ManifestState::merge(None, records, "0.1.0");

  use std::collections::BTreeSet;
  let arches: BTreeSet<String> = state.architectures.iter().cloned().collect();
  assert_eq!(arches, BTreeSet::from(["x86_64".to_string(), "i686".to_string()]), "both arches recomputed");
  assert_eq!(state.sources.len(), 1, "one source recomputed from the records");
  assert_eq!(state.packages.len(), 2, "(name, arch)-keyed: two distinct entries");

  // Removing one arch on a fresh merge recomputes the set down — never patches.
  let down = ManifestState::merge(None, vec![sample_record("libc6", "x86_64", "2.36")], "0.1.0");
  assert_eq!(
    down.architectures.iter().cloned().collect::<BTreeSet<_>>(),
    BTreeSet::from(["x86_64".to_string()]),
    "a final set with one arch recomputes to exactly that arch"
  );
}

// covers: CORE-012
#[test]
fn write_prunes_stale_ledgers_and_leftover_tmp_files() {
  // A shrinking package set on rewrite must prune the files/<key> ledger of the
  // package no longer present, and any leftover .tmp from a crashed write.
  let tmp = TempDir::new().unwrap();
  let wide = ManifestState::merge(
    None,
    vec![
      sample_record("bash", "x86_64", "5.2"),
      sample_record("coreutils", "x86_64", "9.6"),
    ],
    "0.1.0",
  );
  wide.write(tmp.path()).unwrap();
  let files_dir = tmp.path().join(".flatroot/files");
  assert!(files_dir.join("bash:x86_64").exists());
  assert!(files_dir.join("coreutils:x86_64").exists());

  // Drop coreutils and add a leftover .tmp; rewrite must prune both.
  fs::write(files_dir.join("ghost:x86_64.tmp"), "/leftover").unwrap();
  let narrow = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.2")], "0.1.0");
  narrow.write(tmp.path()).unwrap();
  assert!(files_dir.join("bash:x86_64").exists(), "the surviving package keeps its ledger");
  assert!(!files_dir.join("coreutils:x86_64").exists(), "the dropped package's stale ledger must be pruned");
  assert!(!files_dir.join("ghost:x86_64.tmp").exists(), "a leftover .tmp must be removed on rewrite");
}

// covers: CORE-013
#[test]
fn read_rejects_packagecount_disagreeing_with_record_count() {
  let tmp = TempDir::new().unwrap();
  // Header declares 2 packages; the body has exactly 1 record (with a ledger).
  write_meta(
    tmp.path(),
    "FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: x86_64\nPackageCount: 2",
    "Package: bash\nVersion: 5.2\nArchitecture: x86_64\nSource: debian:bookworm\nUrl: https://x\nChecksum: sha256:a\nSize: 0\nDepends:",
    &[("bash:x86_64", "/bin/bash")],
  );
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(
    err.to_string().contains("PackageCount 2 does not match 1 records"),
    "mismatched PackageCount must bail with the exact wording: {err}"
  );
}

// covers: CORE-014
#[test]
fn read_rejects_sources_or_architectures_header_disagreeing_with_records() {
  // Sources header lists a value the records do not carry.
  let tmp = TempDir::new().unwrap();
  write_meta(
    tmp.path(),
    "FlatrootVersion: 0.1.0\nSources: ubuntu:noble\nArchitectures: x86_64\nPackageCount: 1",
    "Package: bash\nVersion: 5.2\nArchitecture: x86_64\nSource: debian:bookworm\nUrl: https://x\nChecksum: sha256:a\nSize: 0\nDepends:",
    &[("bash:x86_64", "/bin/bash")],
  );
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(
    err.to_string().contains("Sources header does not match"),
    "a Sources header not derivable from records must bail: {err}"
  );

  // Architectures header lists an arch the records do not carry.
  let tmp = TempDir::new().unwrap();
  write_meta(
    tmp.path(),
    "FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: aarch64\nPackageCount: 1",
    "Package: bash\nVersion: 5.2\nArchitecture: x86_64\nSource: debian:bookworm\nUrl: https://x\nChecksum: sha256:a\nSize: 0\nDepends:",
    &[("bash:x86_64", "/bin/bash")],
  );
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(
    err.to_string().contains("Architectures header does not match"),
    "an Architectures header not derivable from records must bail: {err}"
  );
}

// covers: CORE-015
#[test]
fn read_errors_when_a_record_has_no_matching_files_ledger() {
  // Build a real on-disk manifest, then delete the package's files/ ledger:
  // read() must bail naming the missing ledger for that (name, arch).
  let tmp = TempDir::new().unwrap();
  let state = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.2")], "0.1.0");
  state.write(tmp.path()).unwrap();
  fs::remove_file(tmp.path().join(".flatroot/files/bash:x86_64")).unwrap();

  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  let msg = err.to_string();
  assert!(
    msg.contains("manifest record (bash, x86_64) has no matching"),
    "missing ledger must name (name, arch): {msg}"
  );
  assert!(msg.contains("files/bash:x86_64"), "the error must name the missing ledger path: {msg}");
}

// covers: CORE-016
#[test]
fn read_rejects_missing_keys_unknown_header_key_and_non_integer_count() {
  // Missing required header keys.
  let tmp = TempDir::new().unwrap();
  write_meta(tmp.path(), "FlatrootVersion: 0.1.0", "Package: bash", &[]);
  assert!(ManifestState::read(tmp.path()).is_err(), "a manifest missing required keys must error");

  // Unknown manifest HEADER key.
  let tmp = TempDir::new().unwrap();
  write_meta(
    tmp.path(),
    "FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: x86_64\nPackageCount: 1\nBogus: nope",
    "Package: bash\nVersion: 5.2\nArchitecture: x86_64\nSource: debian:bookworm\nUrl: https://x\nChecksum: sha256:a\nSize: 0\nDepends:",
    &[("bash:x86_64", "/bin/bash")],
  );
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(err.to_string().contains("unknown manifest key 'Bogus'"), "unknown header key must bail: {err}");

  // Non-integer PackageCount.
  let tmp = TempDir::new().unwrap();
  write_meta(
    tmp.path(),
    "FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: x86_64\nPackageCount: lots",
    "Package: bash\nVersion: 5.2\nArchitecture: x86_64\nSource: debian:bookworm\nUrl: https://x\nChecksum: sha256:a\nSize: 0\nDepends:",
    &[("bash:x86_64", "/bin/bash")],
  );
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(err.to_string().contains("PackageCount not an integer"), "non-integer count must bail: {err}");
}

// covers: CORE-017
#[test]
fn read_errors_on_package_data_without_a_manifest_file_and_inverse() {
  // packages file present but no manifest file.
  let tmp = TempDir::new().unwrap();
  let meta = tmp.path().join(".flatroot");
  fs::create_dir_all(meta.join("files")).unwrap();
  fs::write(meta.join("packages"), "Package: bash\n").unwrap();
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(
    err.to_string().contains("has package data but no manifest file"),
    "package data without a manifest must bail: {err}"
  );

  // Inverse: manifest present but no packages file.
  let tmp = TempDir::new().unwrap();
  let meta = tmp.path().join(".flatroot");
  fs::create_dir_all(meta.join("files")).unwrap();
  fs::write(meta.join("manifest"), "FlatrootVersion: 0.1.0\nSources:\nArchitectures:\nPackageCount: 0").unwrap();
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(
    err.to_string().contains("has a manifest but no packages file"),
    "a manifest without a packages file must bail: {err}"
  );
}

// covers: CORE-018
#[test]
fn read_rejects_unknown_record_field_and_missing_required_record_field() {
  // Unknown record field key drives through the real read path.
  let tmp = TempDir::new().unwrap();
  write_meta(
    tmp.path(),
    "FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: x86_64\nPackageCount: 1",
    "Package: bash\nVersion: 5.2\nArchitecture: x86_64\nSource: debian:bookworm\nUrl: https://x\nChecksum: sha256:a\nSize: 0\nWeird: hello\nDepends:",
    &[("bash:x86_64", "/bin/bash")],
  );
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(err.to_string().contains("unknown manifest key 'Weird'"), "unknown record field must bail: {err}");

  // Missing a required record field (no Version).
  let tmp = TempDir::new().unwrap();
  write_meta(
    tmp.path(),
    "FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: x86_64\nPackageCount: 1",
    "Package: bash\nArchitecture: x86_64\nSource: debian:bookworm\nUrl: https://x\nChecksum: sha256:a\nSize: 0\nDepends:",
    &[("bash:x86_64", "/bin/bash")],
  );
  let err = ManifestState::read(tmp.path()).err().expect("expected an error");
  assert!(err.to_string().contains("missing Version"), "a record missing a required field must bail: {err}");
}

// covers: CORE-019
#[test]
fn files_ledger_is_normalized_sorted_deduped_absolute_and_drops_bare_root() {
  // A record whose file list is untidy — ./ prefix, trailing slash, duplicates,
  // out of order, and a bare-root entry — must be written as a normalized,
  // sorted, deduped, absolute ledger with the bare-root entry dropped; reading
  // it back strips the leading slash to rootfs-relative paths.
  let tmp = TempDir::new().unwrap();
  let mut rec = sample_record("bash", "x86_64", "5.2");
  rec.files = vec![
    PathBuf::from("usr/bin/bash"),
    PathBuf::from("./bin/bash"),
    PathBuf::from("etc/bash.bashrc/"),
    PathBuf::from("usr/bin/bash"), // duplicate
    PathBuf::from("/"),            // bare root — must be dropped
  ];
  let state = ManifestState::merge(None, vec![rec], "0.1.0");
  state.write(tmp.path()).unwrap();

  let ledger = fs::read_to_string(tmp.path().join(".flatroot/files/bash:x86_64")).unwrap();
  let lines: Vec<&str> = ledger.lines().collect();
  // Every entry absolute, no bare-root, deduped, lex-sorted.
  assert!(lines.iter().all(|l| l.starts_with('/')), "every ledger entry must be absolute: {lines:?}");
  assert!(!lines.iter().any(|l| *l == "/"), "the bare-root entry must be dropped");
  let mut sorted = lines.clone();
  sorted.sort();
  assert_eq!(lines, sorted, "ledger entries must be sorted");
  assert_eq!(lines.iter().collect::<std::collections::BTreeSet<_>>().len(), lines.len(), "no duplicates in the ledger");
  assert!(lines.contains(&"/bin/bash"), "the ./-prefixed path normalizes to /bin/bash");
  assert!(lines.contains(&"/etc/bash.bashrc"), "the trailing-slash path normalizes without it");

  // Round-trip: reading the manifest back yields rootfs-relative paths.
  let back = ManifestState::read(tmp.path()).unwrap().unwrap();
  let key = PackageIdentity {
    name: "bash".to_string(),
    arch: "x86_64".to_string(),
  };
  let files = &back.packages[&key].files;
  assert!(files.iter().all(|p| !p.to_string_lossy().starts_with('/')), "parsed paths are rootfs-relative: {files:?}");
  assert!(files.contains(&PathBuf::from("bin/bash")), "parsed list contains the rootfs-relative bin/bash");
}

// covers: CORE-020
#[test]
fn depends_field_round_trips_alternatives_constraints_and_empty_without_pipe_leak() {
  // A record carrying a multi-alternative dependency with a version constraint
  // plus a plain one must round-trip through write/read with its structure
  // preserved and no encoding leak; an empty Depends must round-trip to [].
  let tmp = TempDir::new().unwrap();
  let mut rec = sample_record("debconf-helper", "x86_64", "1.0");
  rec.depends = vec![
    Dependency {
      alternatives: vec![
        DepSpec {
          name: "debconf".to_string(),
          version_constraint: Some(">= 0.5".to_string()),
        },
        DepSpec {
          name: "cdebconf".to_string(),
          version_constraint: None,
        },
      ],
    },
    Dependency {
      alternatives: vec![DepSpec {
        name: "libc6".to_string(),
        version_constraint: Some(">= 2.36".to_string()),
      }],
    },
  ];
  let mut empty = sample_record("no-deps-pkg", "x86_64", "1.0");
  empty.depends = Vec::new();

  let state = ManifestState::merge(None, vec![rec.clone(), empty.clone()], "0.1.0");
  state.write(tmp.path()).unwrap();

  // The packages file's Depends line carries `|` only between alternatives, and
  // the round-tripped structure equals the input.
  let packages = fs::read_to_string(tmp.path().join(".flatroot/packages")).unwrap();
  assert!(
    packages.contains("Depends: debconf (>= 0.5) | cdebconf, libc6 (>= 2.36)"),
    "Depends shape on disk:\n{packages}"
  );
  assert!(
    packages.contains("Depends:\n") || packages.ends_with("Depends:"),
    "empty Depends is an explicit empty field:\n{packages}"
  );

  let back = ManifestState::read(tmp.path()).unwrap().unwrap();
  let got = &back.packages[&rec.key()].depends;
  assert_eq!(got, &rec.depends, "dependency structure must round-trip exactly");
  let got_empty = &back.packages[&empty.key()].depends;
  assert!(got_empty.is_empty(), "an empty Depends must round-trip to []");
}

// covers: CORE-022
#[test]
fn checksum_format_parse_round_trips_all_algorithms_and_rejects_bad_input() {
  // All three algorithms round-trip through format()/parse().
  for algo in [
    ChecksumAlgorithm::Sha256,
    ChecksumAlgorithm::Sha512,
    ChecksumAlgorithm::Q1Sha1,
  ] {
    let c = Checksum {
      algorithm: algo,
      hex: "deadbeef".to_string(),
    };
    let encoded = c.format();
    assert_eq!(Checksum::parse(&encoded).unwrap(), c, "round-trip failed for {algo:?}");
  }
  // The persisted forms carry the stable scheme names.
  assert_eq!(
    Checksum {
      algorithm: ChecksumAlgorithm::Sha256,
      hex: "ab".into()
    }
    .format(),
    "sha256:ab"
  );
  assert_eq!(
    Checksum {
      algorithm: ChecksumAlgorithm::Sha512,
      hex: "ab".into()
    }
    .format(),
    "sha512:ab"
  );
  assert_eq!(
    Checksum {
      algorithm: ChecksumAlgorithm::Q1Sha1,
      hex: "Q1ab".into()
    }
    .format(),
    "q1-sha1:Q1ab"
  );

  // An unknown algorithm name is refused.
  assert!(Checksum::parse("sha1:abc").is_err(), "unknown algorithm 'sha1' must be refused");
  // A line missing the ':' separator is refused.
  let err = Checksum::parse("nocolonhere").err().expect("expected an error");
  assert!(err.to_string().contains("missing ':'"), "a checksum with no ':' must bail: {err}");
}

// covers: CORE-023
#[test]
fn checksum_algorithm_infer_dispatches_on_digest_shape() {
  // Q1-prefixed -> Q1Sha1; 128 chars -> Sha512; 64 chars -> Sha256; short ->
  // Sha256 (the default fall-through).
  assert_eq!(ChecksumAlgorithm::infer("Q1abcdef"), ChecksumAlgorithm::Q1Sha1, "Q1 prefix -> Q1Sha1");
  assert_eq!(ChecksumAlgorithm::infer(&"a".repeat(128)), ChecksumAlgorithm::Sha512, "128 hex chars -> Sha512");
  assert_eq!(ChecksumAlgorithm::infer(&"a".repeat(64)), ChecksumAlgorithm::Sha256, "64 hex chars -> Sha256");
  assert_eq!(ChecksumAlgorithm::infer("deadbeef"), ChecksumAlgorithm::Sha256, "short digest -> Sha256 default");
}

// covers: CORE-053
#[test]
fn empty_existing_sources_set_admits_any_incoming_source() {
  // A defensive baseline: a ManifestState whose sources set is empty (e.g. a
  // default/never-populated state) must admit any incoming source — the
  // guardrail only bites when an existing source is recorded.
  let p = Path::new("/tmp/rootfs");

  // None existing: anything passes.
  assert!(ManifestState::source_admit(None, "debian:bookworm", p).is_ok());

  // Empty sources set: anything passes.
  let empty = ManifestState::default();
  assert!(empty.sources.is_empty(), "the default state must have an empty sources set");
  assert!(ManifestState::source_admit(Some(&empty), "debian:bookworm", p).is_ok());
  assert!(ManifestState::source_admit(Some(&empty), "alpine:edge", p).is_ok());
  assert!(ManifestState::source_admit(Some(&empty), "fedora:42@2026-01-01", p).is_ok());

  // But a populated, differing source is refused (sanity that the gate works).
  let mut populated = ManifestState::default();
  populated.sources.insert("debian:bookworm".to_string());
  assert!(ManifestState::source_admit(Some(&populated), "debian:bookworm", p).is_ok(), "identical source passes");
  assert!(ManifestState::source_admit(Some(&populated), "ubuntu:noble", p).is_err(), "differing source is refused");
}

// covers: CORE-054
#[test]
fn files_load_attributes_every_file_and_accepts_an_empty_ledger() {
  // A package that placed files round-trips its non-empty ledger; a package
  // that placed nothing is a legitimate empty ledger (-> []), NOT an error. A
  // record whose ledger is entirely missing IS an error (covered separately).
  let tmp = TempDir::new().unwrap();
  let mut placed = sample_record("bash", "x86_64", "5.2");
  placed.files = vec![PathBuf::from("bin/bash"), PathBuf::from("usr/share/man/man1/bash.1")];
  let mut empty = sample_record("metapackage", "x86_64", "1.0");
  empty.files = Vec::new();

  let state = ManifestState::merge(None, vec![placed.clone(), empty.clone()], "0.1.0");
  state.write(tmp.path()).unwrap();

  // The empty package's ledger is an empty (zero-byte) file — legitimate.
  let empty_ledger = tmp.path().join(".flatroot/files/metapackage:x86_64");
  assert!(empty_ledger.exists(), "an empty ledger file must still be written");
  assert!(fs::read(&empty_ledger).unwrap().is_empty(), "a package that placed nothing has a zero-byte ledger");

  // Read back: the empty package attributes [], the placed package attributes
  // every file.
  let back = ManifestState::read(tmp.path()).unwrap().unwrap();
  assert!(back.packages[&empty.key()].files.is_empty(), "empty ledger -> [] (legitimate, not an error)");
  let placed_files = &back.packages[&placed.key()].files;
  assert!(placed_files.contains(&PathBuf::from("bin/bash")), "every placed file is attributed");
  assert!(placed_files.contains(&PathBuf::from("usr/share/man/man1/bash.1")), "every placed file is attributed");
}

/// A single PackageRecord with an explicit source axis (mirrors the shape used
/// by the src/manifest/mod.rs unit tests).
fn record(name: &str, arch: &str, version: &str, source: &str) -> PackageRecord {
  PackageRecord {
    name: name.to_string(),
    version: version.to_string(),
    architecture: arch.to_string(),
    source: source.to_string(),
    url: format!("https://example.invalid/pool/{}_{}.deb", name, version),
    checksum: Checksum {
      algorithm: ChecksumAlgorithm::Sha256,
      hex: "deadbeef".to_string(),
    },
    size: 1,
    depends: Vec::new(),
    files: vec![PathBuf::from(format!("usr/bin/{}", name))],
  }
}

// ===========================================================================
// CORE-062 — manifest write durability ordering: per-package file lists, then the
// packages file, then the manifest, then prune of stale/`.tmp` entries
// (manifest/state.rs:161-208). Driven via the public ManifestState::write, then
// asserting a planted `.tmp` corpse and a stale files/ entry are both swept and
// every referenced files/ entry exists, with the manifest readable back.
// ===========================================================================

// covers: CORE-062
#[test]
fn manifest_write_orders_files_then_packages_then_manifest_then_prune() {
  let tmp = TempDir::new().unwrap();
  let state = ManifestState::merge(
    None,
    vec![
      record("bash", "x86_64", "5.2", "debian:bookworm"),
      record("coreutils", "x86_64", "9.6", "debian:bookworm"),
    ],
    "0.1.0",
  );
  state.write(tmp.path()).unwrap();

  // Every referenced package has a files/ entry (phase 1 ran before the manifest
  // that points at them).
  for (name, arch) in [("bash", "x86_64"), ("coreutils", "x86_64")] {
    let entry = tmp.path().join(format!(".flatroot/files/{name}:{arch}"));
    assert!(entry.exists(), "files/ entry for {name}:{arch} must exist after write");
  }
  // The manifest and packages files exist and read back consistently (phases 2/3
  // landed; ManifestState::read enforces header↔record agreement).
  let read = ManifestState::read(tmp.path()).unwrap().unwrap();
  assert_eq!(read, state, "the durable record must read back identically");

  // Plant a stale files/ entry and a `.tmp` corpse; a re-write must prune both
  // (phase 4), proving the prune runs last over the final package set.
  let stale = tmp.path().join(".flatroot/files/ghost:x86_64");
  let corpse = tmp.path().join(".flatroot/files/bash:x86_64.tmp");
  fs::write(&stale, "/ghost").unwrap();
  fs::write(&corpse, "leftover").unwrap();
  state.write(tmp.path()).unwrap();
  assert!(!stale.exists(), "a stale files/ entry must be pruned on the next write");
  assert!(!corpse.exists(), "a leftover .tmp corpse must be swept on the next write");
  // The legitimate entries survive the prune.
  assert!(tmp.path().join(".flatroot/files/bash:x86_64").exists(), "live files/ entry must survive the prune");
}
