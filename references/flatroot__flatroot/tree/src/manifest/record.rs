//! `PackageRecord`: one installed package, pinned to its
//! architecture — its readable metadata record plus, stored beside it,
//! the list of files the package placed.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::package::{Dependency, PackageIdentity};

use super::checksum::Checksum;
use super::layout::ManifestLayout;

/// Everything the manifest records about one installed package: where
/// it came from, what to fetch to reproduce it, and what it placed on
/// disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRecord {
  /// Package name as the upstream package metadata reports it.
  pub name: String,
  /// Package version as the upstream package metadata reports it.
  pub version: String,
  /// Architecture installed for, in kernel `uname -m` form (`x86_64`, …).
  pub architecture: String,
  /// Source (`<distro>:<release>[@<date>]`) this install came from,
  /// checked by `ManifestState::source_admit` against a later install's
  /// source.
  pub source: String,
  /// Fully qualified URL of the archive on the mirror.
  pub url: String,
  /// Checksum that verifies the downloaded archive.
  pub checksum: Checksum,
  /// Archive size in bytes, as the upstream package metadata reports it.
  pub size: u64,
  /// Hard dependency list, replayed by consumers without re-walking
  /// resolution.
  pub depends: Vec<Dependency>,
  /// Rootfs-relative paths (no leading `/`) captured at extraction; the
  /// on-disk format adds the leading `/`.
  pub files: Vec<PathBuf>,
}

impl PackageRecord {
  /// The name-and-architecture identity, so a multilib tree's per-arch
  /// entries stay distinct instead of shadowing each other.
  pub fn key(&self) -> PackageIdentity {
    PackageIdentity {
      name: self.name.clone(),
      arch: self.architecture.clone(),
    }
  }

  /// Renders the record as the familiar line-per-field package-metadata
  /// text, writing absent dependencies as an explicit empty `Depends:`
  /// rather than an omission a reader could misread.
  pub(crate) fn format(&self) -> String {
    let mut lines = Vec::with_capacity(8);
    lines.push(format!("Package: {}", self.name));
    lines.push(format!("Version: {}", self.version));
    lines.push(format!("Architecture: {}", self.architecture));
    lines.push(format!("Source: {}", self.source));
    lines.push(format!("Url: {}", self.url));
    lines.push(format!("Checksum: {}", self.checksum.format()));
    lines.push(format!("Size: {}", self.size));
    let deps = Dependency::list_format(&self.depends);
    if deps.is_empty() {
      lines.push("Depends:".to_string());
    } else {
      lines.push(format!("Depends: {}", deps));
    }
    lines.join("\n")
  }

  /// Parses one record from its text and loads its file list.
  /// Everything later trusts this, so a malformed line, unknown field,
  /// non-numeric size, missing field, or missing file list is an error,
  /// never guessed past.
  pub(crate) fn parse(text: &str, layout: &ManifestLayout) -> Result<PackageRecord> {
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut architecture: Option<String> = None;
    let mut source: Option<String> = None;
    let mut url: Option<String> = None;
    let mut checksum: Option<Checksum> = None;
    let mut size: Option<u64> = None;
    let mut depends_raw: Option<String> = None;

    for line in text.lines() {
      let (key, value) = match line.split_once(':') {
        Some((k, v)) => (k, v.strip_prefix(' ').unwrap_or(v)),
        None => bail!("manifest record line missing ':': {:?}", line),
      };
      match key {
        "Package" => name = Some(value.to_string()),
        "Version" => version = Some(value.to_string()),
        "Architecture" => architecture = Some(value.to_string()),
        "Source" => source = Some(value.to_string()),
        "Url" => url = Some(value.to_string()),
        "Checksum" => checksum = Some(Checksum::parse(value)?),
        "Size" => size = Some(value.parse().with_context(|| format!("Size not an integer: {value}"))?),
        "Depends" => depends_raw = Some(value.to_string()),
        other => bail!("unknown manifest key '{}'", other),
      }
    }

    let name = name.context("manifest record missing Package")?;
    let architecture = architecture.context("manifest record missing Architecture")?;
    let identity = PackageIdentity {
      name: name.clone(),
      arch: architecture.clone(),
    };
    let files = PackageRecord::files_load(&identity, layout)?;

    Ok(PackageRecord {
      name,
      version: version.context("manifest record missing Version")?,
      architecture,
      source: source.context("manifest record missing Source")?,
      url: url.context("manifest record missing Url")?,
      checksum: checksum.context("manifest record missing Checksum")?,
      size: size.context("manifest record missing Size")?,
      depends: Dependency::list_parse(depends_raw.as_deref().unwrap_or(""))?,
      files,
    })
  }

  /// Renders the package's file list canonically: every path in one
  /// absolute form, the bare-root entry dropped, sorted and
  /// deduplicated, so two runs over the same install produce identical
  /// listings.
  pub(crate) fn files_format(&self) -> String {
    let mut normalized: Vec<String> = self
      .files
      .iter()
      .filter_map(|p| {
        let s = p.to_string_lossy();
        // Strip `./` prefix, trailing `/`, and drop the bare-root entry.
        let s = s.trim_start_matches("./").trim_end_matches('/');
        if s.is_empty() {
          return None;
        }
        Some(if s.starts_with('/') {
          s.to_string()
        } else {
          format!("/{}", s)
        })
      })
      .collect();
    normalized.sort();
    normalized.dedup();
    normalized.join("\n")
  }

  /// Reads the file list back as rootfs-relative paths; an empty list
  /// is a package that placed nothing, a legitimate state.
  pub(crate) fn files_parse(text: &str) -> Vec<PathBuf> {
    if text.is_empty() {
      return Vec::new();
    }
    text.lines().map(|l| PathBuf::from(l.trim_start_matches('/'))).collect()
  }

  /// Loads a package's file list, stored in its own file so large
  /// listings do not bloat the record. A record with no matching file
  /// list is a contradiction and is an error, not an empty list.
  pub(crate) fn files_load(identity: &PackageIdentity, layout: &ManifestLayout) -> Result<Vec<PathBuf>> {
    let path_file_entry = layout.file_entry(identity);
    if !path_file_entry.exists() {
      bail!("manifest record ({}, {}) has no matching {}", identity.name, identity.arch, path_file_entry.display());
    }
    let text = std::fs::read_to_string(&path_file_entry)
      .with_context(|| format!("failed to read {}", path_file_entry.display()))?;
    Ok(PackageRecord::files_parse(&text))
  }
}

/// Test-only constructor for a representative installed-package record. The
/// inline test modules across the manifest build their records through this,
/// so the fixture shape lives in one place and a new `PackageRecord` field is
/// added here once.
#[cfg(test)]
pub(crate) mod fixtures {
  use std::path::PathBuf;

  use crate::manifest::checksum::{Checksum, ChecksumAlgorithm};
  use crate::package::fixtures::dep_ver;

  use super::PackageRecord;

  pub(crate) fn sample_record(name: &str, arch: &str, version: &str) -> PackageRecord {
    PackageRecord {
      name: name.to_string(),
      version: version.to_string(),
      architecture: arch.to_string(),
      source: "debian:bookworm@2026-04-21".to_string(),
      url: format!(
        "https://snapshot.debian.org/archive/debian/20260421T000000Z/pool/main/{}/{}_{}.deb",
        &name[..1],
        name,
        version
      ),
      checksum: Checksum {
        algorithm: ChecksumAlgorithm::Sha256,
        hex: "a4f8".to_string(),
      },
      size: 1234,
      depends: vec![dep_ver("libc6", ">= 2.36")],
      files: vec![
        PathBuf::from(format!("usr/bin/{}", name)),
        PathBuf::from(format!("usr/share/doc/{}/copyright", name)),
      ],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::fixtures::sample_record;
  use super::*;

  #[test]
  fn files_format_parse_roundtrip_absolute_paths() {
    let files = vec![
      PathBuf::from("usr/bin/bash"),
      PathBuf::from("bin/bash"),
      PathBuf::from("etc/bash.bashrc"),
    ];
    let rec = PackageRecord {
      files,
      ..sample_record("bash", "x86_64", "5.2")
    };
    let s = rec.files_format();
    for line in s.lines() {
      assert!(line.starts_with('/'), "non-absolute: {line}");
    }
    let back = PackageRecord::files_parse(&s);
    for p in &back {
      assert!(!p.to_string_lossy().starts_with('/'));
    }
  }

  #[test]
  fn record_format_is_byte_deterministic() {
    let r = sample_record("bash", "x86_64", "5.2");
    let s1 = r.format();
    let s2 = r.format();
    assert_eq!(s1, s2);
    let expected_keys = [
      "Package",
      "Version",
      "Architecture",
      "Source",
      "Url",
      "Checksum",
      "Size",
      "Depends",
    ];
    let actual_keys: Vec<&str> = s1.lines().filter_map(|l| l.split_once(':').map(|(k, _)| k)).collect();
    assert_eq!(actual_keys, expected_keys);
  }

  #[test]
  fn url_field_never_contains_pipe_and_is_https() {
    let r = sample_record("bash", "x86_64", "5.2");
    let s = r.format();
    let url = s.lines().find(|l| l.starts_with("Url: ")).unwrap();
    assert!(url.starts_with("Url: https://"), "not https: {url}");
    assert!(!url.contains('|'), "encoding leaked: {url}");
  }

  #[test]
  fn record_parse_rejects_unknown_key() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".flatroot/files")).unwrap();
    std::fs::write(tmp.path().join(".flatroot/files/bash:x86_64"), "/bin/bash").unwrap();
    let bad = "Package: bash\nVersion: 5\nArchitecture: x86_64\nSource: debian:bookworm\nUrl: https://x\nChecksum: sha256:a\nSize: 0\nWeird: hello\nDepends:";
    let layout = ManifestLayout::new(tmp.path());
    assert!(PackageRecord::parse(bad, &layout).is_err());
  }
}
