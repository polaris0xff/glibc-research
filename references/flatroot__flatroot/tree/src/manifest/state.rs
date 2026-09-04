//! `ManifestState`: the in-memory form of a rootfs's manifest —
//! sources, architectures, and installed packages — read before an
//! install acts and written back after.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::internal::fs::Fs;
use crate::package::PackageIdentity;

use super::codec::{ManifestCodec, ManifestHeader};
use super::layout::ManifestLayout;
use super::record::PackageRecord;

/// The in-memory form of one rootfs's manifest, so an install can tell
/// which packages are already current.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestState {
  /// Version of the flatroot binary that wrote the manifest, so a
  /// reader knows which on-disk format to expect.
  pub flatroot_version: String,
  /// The `--from` sources the rootfs's packages came from, as recorded
  /// per package.
  pub sources: BTreeSet<String>,
  /// Architectures the rootfs has packages for (multiple on a multilib
  /// build).
  pub architectures: BTreeSet<String>,
  /// Installed packages keyed by name-plus-architecture, the manifest's
  /// primary key.
  pub packages: BTreeMap<PackageIdentity, PackageRecord>,
}

impl ManifestState {
  /// Reads the manifest from `root`, trusted only if internally
  /// consistent — a header that disagrees with the package records is
  /// an error, never repaired. A rootfs with no manifest data yields
  /// `None`, the ordinary first install.
  pub fn read(root: &Path) -> Result<Option<ManifestState>> {
    let layout = ManifestLayout::new(root);
    let path_file_manifest = layout.file_manifest();
    let path_file_packages = layout.file_packages();
    let path_dir_files = layout.dir_files();

    let manifest_exists = path_file_manifest.exists();
    let packages_exists = path_file_packages.exists();
    let files_dir_populated = path_dir_files
      .read_dir()
      .map(|mut it| it.next().is_some())
      .unwrap_or(false);

    if !manifest_exists && !packages_exists && !files_dir_populated {
      return Ok(None);
    }

    if !manifest_exists {
      bail!("rootfs at {} has package data but no manifest file ({})", root.display(), path_file_manifest.display());
    }
    if !packages_exists {
      bail!("rootfs at {} has a manifest but no packages file ({})", root.display(), path_file_packages.display());
    }

    let manifest_text = fs::read_to_string(&path_file_manifest)
      .with_context(|| format!("failed to read {}", path_file_manifest.display()))?;
    let parsed_manifest = ManifestHeader::parse(&manifest_text, &path_file_manifest)?;

    let packages_text = fs::read_to_string(&path_file_packages)
      .with_context(|| format!("failed to read {}", path_file_packages.display()))?;
    let records = ManifestCodec::packages_parse(&packages_text, &layout)?;

    if records.len() != parsed_manifest.package_count {
      bail!(
        "{}: PackageCount {} does not match {} records in {}",
        path_file_manifest.display(),
        parsed_manifest.package_count,
        records.len(),
        path_file_packages.display()
      );
    }

    let mut state = ManifestState {
      flatroot_version: parsed_manifest.flatroot_version,
      sources: BTreeSet::new(),
      architectures: BTreeSet::new(),
      packages: BTreeMap::new(),
    };
    for rec in records {
      state.sources.insert(rec.source.clone());
      state.architectures.insert(rec.architecture.clone());
      let key = rec.key();
      if state.packages.insert(key.clone(), rec).is_some() {
        bail!("{}: duplicate record for ({}, {})", path_file_packages.display(), key.name, key.arch);
      }
    }

    // The manifest's Sources/Architectures sets must match what we derived
    // from the per-package records — otherwise the file is internally
    // inconsistent.
    let manifest_sources: BTreeSet<String> = parsed_manifest
      .sources
      .split(", ")
      .filter(|s| !s.is_empty())
      .map(|s| s.to_string())
      .collect();
    let manifest_archs: BTreeSet<String> = parsed_manifest
      .architectures
      .split(", ")
      .filter(|s| !s.is_empty())
      .map(|s| s.to_string())
      .collect();
    if manifest_sources != state.sources {
      bail!("{}: Sources header does not match package records", path_file_manifest.display());
    }
    if manifest_archs != state.architectures {
      bail!("{}: Architectures header does not match package records", path_file_manifest.display());
    }

    Ok(Some(state))
  }

  /// Writes the manifest to disk: the per-package file lists first,
  /// then the `packages` file, then the top-level `manifest`, each
  /// atomically swapped into place — so an interrupted run leaves the
  /// previous record intact, never a corrupt half. Stale per-package
  /// entries are pruned last.
  pub fn write(&self, root: &Path) -> Result<()> {
    let layout = ManifestLayout::new(root);
    let path_dir_meta = layout.dir_meta();
    let path_dir_files = layout.dir_files();
    fs::create_dir_all(&path_dir_files).with_context(|| format!("failed to create {}", path_dir_files.display()))?;

    // Phase 1: write every per-package file list.
    for (identity, rec) in &self.packages {
      let path_file_entry = layout.file_entry(identity);
      let text = rec.files_format();
      Fs::write_atomic(&path_file_entry, text.as_bytes())?;
    }

    // Phase 2: write the `packages` file.
    let packages_text = ManifestCodec::packages_format(self);
    Fs::write_atomic(&layout.file_packages(), packages_text.as_bytes())?;

    // Phase 3: write the top-level `manifest`.
    let manifest_text = ManifestHeader::format(self);
    Fs::write_atomic(&layout.file_manifest(), manifest_text.as_bytes())?;

    // Phase 4: prune `files/<key>` entries no longer referenced.
    if let Ok(iter) = fs::read_dir(&path_dir_files) {
      for entry in iter.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        if name_str.ends_with(".tmp") {
          // Leftover from a crashed write — always safe to remove.
          Fs::remove_lenient(&entry.path());
          continue;
        }
        let Some(key) = PackageIdentity::dirname_parse(&name_str) else {
          continue;
        };
        if !self.packages.contains_key(&key) {
          Fs::remove_lenient(&entry.path());
        }
      }
    }

    // Parent-directory fsync is a best-effort durability boost on top of the
    // already-atomic individual file renames. A failure here only means the
    // directory-entry update may not survive a hard crash — the file contents
    // themselves are already durable from `write_atomic`. We log and move on
    // rather than erroring: the caller's install already succeeded.
    match fs::File::open(&path_dir_meta).and_then(|d| d.sync_all()) {
      Ok(_) => {}
      Err(e) => eprintln!("warning: fsync on {} failed: {}", path_dir_meta.display(), e),
    }
    match fs::File::open(&path_dir_files).and_then(|d| d.sync_all()) {
      Ok(_) => {}
      Err(e) => eprintln!("warning: fsync on {} failed: {}", path_dir_files.display(), e),
    }

    Ok(())
  }

  /// Decides whether an install from `remote_str_new` may touch this
  /// rootfs: an empty rootfs accepts anything, a matching recorded
  /// source proceeds, and anything else is refused naming both
  /// sources — a rootfs is bound to one exact source.
  pub fn source_admit(existing: Option<&ManifestState>, remote_str_new: &str, root: &Path) -> Result<()> {
    let existing = match existing {
      Some(s) if !s.sources.is_empty() => s,
      _ => return Ok(()),
    };

    if existing.sources.iter().all(|s| s == remote_str_new) {
      return Ok(());
    }

    let sources_list: Vec<&str> = existing.sources.iter().map(|s| s.as_str()).collect();
    bail!(
      "cannot install from '{}' into rootfs at {}\n  existing source: {}\n  a rootfs is scoped to one exact source — different distros, releases, snapshot dates, and pinned vs unpinned variants are all refused\n  to start fresh, remove {}/.flatroot/ or use a different directory",
      remote_str_new,
      root.display(),
      sources_list.join(", "),
      root.display()
    );
  }

  /// Merges fresh records into the prior state: each replaces an
  /// earlier record of the same name and architecture, untouched
  /// records carry forward, and the source and architecture sets are
  /// recomputed from the final set so they never drift.
  pub fn merge(
    existing: Option<ManifestState>,
    new_records: Vec<PackageRecord>,
    flatroot_version: &str,
  ) -> ManifestState {
    let mut state = existing.unwrap_or_default();
    state.flatroot_version = flatroot_version.to_string();

    for rec in new_records {
      let key = rec.key();
      if let Some(prev) = state.packages.get(&key) {
        if prev.version != rec.version || prev.checksum != rec.checksum {
          eprintln!(
            "replacing {}:{}: {} ({}) -> {} ({}), source {} -> {}",
            key.name,
            key.arch,
            prev.version,
            prev.checksum.format(),
            rec.version,
            rec.checksum.format(),
            prev.source,
            rec.source,
          );
        }
      }
      state.packages.insert(key, rec);
    }

    state.sources = state.packages.values().map(|r| r.source.clone()).collect();
    state.architectures = state.packages.values().map(|r| r.architecture.clone()).collect();

    state
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;
  use crate::manifest::checksum::ChecksumAlgorithm;
  use crate::manifest::record::fixtures::sample_record;

  #[test]
  fn manifest_write_read_roundtrip_public_api() {
    let tmp = tempfile::tempdir().unwrap();
    let records = vec![
      sample_record("bash", "x86_64", "5.2"),
      sample_record("coreutils", "x86_64", "9.6"),
      sample_record("libc6", "i686", "2.36"),
    ];
    let state = ManifestState::merge(None, records.clone(), "0.1.0");
    state.write(tmp.path()).unwrap();
    let read = ManifestState::read(tmp.path()).unwrap().unwrap();
    assert_eq!(read, state);

    let m1 = std::fs::read(tmp.path().join(".flatroot/manifest")).unwrap();
    let p1 = std::fs::read(tmp.path().join(".flatroot/packages")).unwrap();
    state.write(tmp.path()).unwrap();
    let m2 = std::fs::read(tmp.path().join(".flatroot/manifest")).unwrap();
    let p2 = std::fs::read(tmp.path().join(".flatroot/packages")).unwrap();
    assert_eq!(m1, m2);
    assert_eq!(p1, p2);
  }

  #[test]
  fn manifest_read_empty_dir_returns_none() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(ManifestState::read(tmp.path()).unwrap().is_none());
  }

  #[test]
  fn manifest_read_truncated_record_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    let meta = tmp.path().join(".flatroot");
    std::fs::create_dir_all(meta.join("files")).unwrap();
    std::fs::write(
      meta.join("manifest"),
      "FlatrootVersion: 0.1.0\nSources: debian:bookworm\nArchitectures: x86_64\nPackageCount: 1",
    )
    .unwrap();
    std::fs::write(meta.join("packages"), "Package: bash\nVersion: 5.2").unwrap();
    assert!(ManifestState::read(tmp.path()).is_err());
  }

  #[test]
  fn manifest_read_package_without_files_entry_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    let state = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.2")], "0.1.0");
    state.write(tmp.path()).unwrap();
    std::fs::remove_file(tmp.path().join(".flatroot/files/bash:x86_64")).unwrap();
    assert!(ManifestState::read(tmp.path()).is_err());
  }

  #[test]
  fn compatibility_check_strict_equality() {
    let p = PathBuf::from("/tmp/rootfs");

    // No manifest: anything passes.
    assert!(ManifestState::source_admit(None, "debian:bookworm", &p).is_ok());
    assert!(ManifestState::source_admit(None, "debian:bookworm@2026-04-21", &p).is_ok());

    // Empty sources set: anything passes (defensive edge case).
    let empty = ManifestState::default();
    assert!(ManifestState::source_admit(Some(&empty), "debian:bookworm", &p).is_ok());

    fn state_with(source: &str) -> ManifestState {
      let mut s = ManifestState::default();
      s.sources.insert(source.to_string());
      s
    }

    // Identical source: passes.
    let s = state_with("debian:bookworm");
    assert!(ManifestState::source_admit(Some(&s), "debian:bookworm", &p).is_ok());
    let s = state_with("debian:bookworm@2026-04-21");
    assert!(ManifestState::source_admit(Some(&s), "debian:bookworm@2026-04-21", &p).is_ok());

    // Different distro prefix: fails.
    let s = state_with("debian:bookworm");
    assert!(ManifestState::source_admit(Some(&s), "ubuntu:bookworm", &p).is_err());

    // Same distro, different release: fails.
    let s = state_with("debian:bookworm");
    assert!(ManifestState::source_admit(Some(&s), "debian:trixie", &p).is_err());

    // Same release, different @date: fails.
    let s = state_with("debian:bookworm@2026-04-21");
    assert!(ManifestState::source_admit(Some(&s), "debian:bookworm@2025-01-15", &p).is_err());

    // Pinned vs unpinned of same release: fails both directions.
    let s = state_with("debian:bookworm");
    assert!(ManifestState::source_admit(Some(&s), "debian:bookworm@2026-04-21", &p).is_err());
    let s = state_with("debian:bookworm@2026-04-21");
    assert!(ManifestState::source_admit(Some(&s), "debian:bookworm", &p).is_err());
  }

  #[test]
  fn compatibility_check_ten_distro_matrix() {
    let prefixes = [
      "debian", "ubuntu", "arch", "cachyos", "alpine", "centos", "fedora", "alma", "rocky", "opensuse",
    ];
    let p = PathBuf::from("/tmp/rootfs");
    for pfx in &prefixes {
      assert!(ManifestState::source_admit(None, &format!("{pfx}:rel"), &p).is_ok());
    }
    for existing in &prefixes {
      let mut state = ManifestState::default();
      state.sources.insert(format!("{existing}:rel"));
      for incoming in &prefixes {
        let res = ManifestState::source_admit(Some(&state), &format!("{incoming}:rel"), &p);
        if existing == incoming {
          assert!(res.is_ok(), "identical source should pass: {existing}:rel");
        } else {
          assert!(res.is_err(), "cross-prefix should fail: {existing} vs {incoming}");
        }
      }
    }
  }

  #[test]
  fn state_merge_recomputes_sources_and_arches() {
    let records = vec![
      sample_record("bash", "x86_64", "5.2"),
      sample_record("bash", "i686", "5.2"),
    ];
    let state = ManifestState::merge(None, records, "0.1.0");
    assert_eq!(state.architectures.len(), 2);
    assert_eq!(state.sources.len(), 1);
    assert_eq!(state.packages.len(), 2);
  }

  #[test]
  fn state_merge_with_existing_then_rewrite() {
    let tmp = tempfile::tempdir().unwrap();
    let first = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.1")], "0.1.0");
    first.write(tmp.path()).unwrap();
    let loaded = ManifestState::read(tmp.path()).unwrap().unwrap();
    let second = ManifestState::merge(Some(loaded), vec![sample_record("bash", "x86_64", "5.2")], "0.1.0");
    let key = PackageIdentity {
      name: "bash".to_string(),
      arch: "x86_64".to_string(),
    };
    assert_eq!(second.packages[&key].version, "5.2");
  }

  #[test]
  fn manifest_state_default_is_empty() {
    let s = ManifestState::default();
    assert!(s.sources.is_empty());
    assert!(s.architectures.is_empty());
    assert!(s.packages.is_empty());
    assert_eq!(s.flatroot_version, "");
    let _: &BTreeSet<String> = &s.sources;
    let _: &BTreeMap<PackageIdentity, PackageRecord> = &s.packages;
    let _ = ChecksumAlgorithm::Sha256;
  }

  #[test]
  fn state_merge_brand_new_entry_produces_expected_shape() {
    let state = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.2")], "0.1.0");
    assert_eq!(state.packages.len(), 1);
    assert_eq!(state.sources.len(), 1);
    assert_eq!(state.architectures.len(), 1);
    assert_eq!(state.flatroot_version, "0.1.0");
  }

  #[test]
  fn state_merge_identical_replace_is_noop() {
    let first = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.2")], "0.1.0");
    let merged = ManifestState::merge(Some(first.clone()), vec![sample_record("bash", "x86_64", "5.2")], "0.1.0");
    assert_eq!(merged, first);
  }

  #[test]
  fn state_merge_changed_version_replaces_record() {
    let first = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.1")], "0.1.0");
    let updated = sample_record("bash", "x86_64", "5.2");
    let merged = ManifestState::merge(Some(first), vec![updated.clone()], "0.1.0");
    let key = updated.key();
    assert_eq!(merged.packages[&key].version, "5.2");
  }

  #[test]
  fn manifest_write_prunes_stale_files_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let state = ManifestState::merge(None, vec![sample_record("bash", "x86_64", "5.2")], "0.1.0");
    state.write(tmp.path()).unwrap();
    let stale = tmp.path().join(".flatroot/files/ghost:x86_64");
    std::fs::write(&stale, "/ghost").unwrap();
    assert!(stale.exists());
    state.write(tmp.path()).unwrap();
    assert!(!stale.exists(), "stale files/ entry was not pruned");
  }

  #[test]
  fn manifest_read_missing_keys_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    let meta = tmp.path().join(".flatroot");
    std::fs::create_dir_all(meta.join("files")).unwrap();
    std::fs::write(meta.join("manifest"), "FlatrootVersion: 0.1.0").unwrap();
    std::fs::write(meta.join("packages"), "Package: bash").unwrap();
    assert!(ManifestState::read(tmp.path()).is_err());
  }
}
