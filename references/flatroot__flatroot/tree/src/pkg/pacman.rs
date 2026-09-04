//! The pacman package format: parses the `.db` package index and the
//! `.files` file listing, and extracts `.pkg.tar.zst` archives.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};

use crate::db::IndexWriter;
use crate::distro::SourceDistro;
use crate::internal::codec::Codec;
use crate::internal::fs::Fs;
use crate::internal::http::HttpClient;
use crate::manifest::ManifestLayout;
use crate::package::{DepSpec, Dependency, Package, PackageIdentity};
use crate::pkg::PackageFormat;

/// The pacman-family `PackageFormat` implementation.
pub struct PacmanFormat;

impl PackageFormat for PacmanFormat {
  /// Reads each repository's `.db` package index and `.files` file
  /// listing from that repository's own mirrors, recording alongside
  /// every package the mirror it can be fetched from.
  fn index_fetch(&self, source: &SourceDistro, writer: &mut IndexWriter, _http: &Arc<HttpClient>) -> Result<()> {
    use std::io::Read as _;

    for repo in &source.repos {
      let (path_file_db, path_file_files, path_dir_packages) = match &repo.layout {
        crate::distro::LayoutRepository::Pacman {
          path_file_db,
          path_file_files,
          path_dir_packages,
        } => (path_file_db, path_file_files, path_dir_packages),
        other => bail!("pacman index_fetch received non-Pacman layout for repository {}: {:?}", repo.label, other),
      };

      let (bytes_db, mirror_origin) = repo
        .mirrors
        .fetch_with_origin(path_file_db)
        .with_context(|| format!(".db archive for {}", repo.label))?;
      let bytes_db_uncompressed = Codec::from_magic(&bytes_db)?.bytes(&bytes_db)?;
      let mut archive_db = tar::Archive::new(&bytes_db_uncompressed[..]);

      // Accumulate every parsed `desc` into a per-repo `Vec<Package>`,
      // sort by (name, version) using the pacman-flavour comparator
      let mut packages: Vec<Package> = Vec::new();
      for entry in archive_db.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let path_str = path.to_string_lossy();
        if !path_str.ends_with("/desc") {
          continue;
        }
        let mut content = String::new();
        entry.read_to_string(&mut content)?;
        if let Some(mut pkg) = PacmanFormat::desc_parse(&content)? {
          pkg.filename = format!("{}|{}/{}", mirror_origin.trim_end_matches('/'), path_dir_packages, pkg.filename);
          packages.push(pkg);
        }
      }
      writer.packages_insert(packages, &crate::version::PacmanVersionCompare)?;

      let (bytes_files, _) = repo
        .mirrors
        .fetch_with_origin(path_file_files)
        .with_context(|| format!(".files archive for {}", repo.label))?;
      let bytes_files_uncompressed = Codec::from_magic(&bytes_files)?.bytes(&bytes_files)?;
      PacmanFormat::files_ingest(&bytes_files_uncompressed[..], writer)?;
    }

    Ok(())
  }

  /// Extracts one pacman archive: files go into the rootfs, the
  /// `.INSTALL` script is saved per package for the post-install pass,
  /// and the other dot-file metadata entries are discarded. Returns the
  /// files placed.
  fn extract(&self, path_file_archive: &Path, root: &Path, pkg: &PackageIdentity) -> Result<Vec<PathBuf>> {
    let file = std::fs::File::open(path_file_archive)?;
    let decoder = zstd::Decoder::new(file)?;
    let mut archive = tar::Archive::new(decoder);
    archive.set_preserve_permissions(false);
    archive.set_preserve_ownerships(false);
    archive.set_overwrite(true);

    let scripts_dir = ManifestLayout::new(root).dir_scripts_entry(pkg);
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in archive.entries()? {
      let mut entry = entry?;
      let path = entry.path()?.into_owned();
      let path_str = path.to_string_lossy();

      // Save .INSTALL for postinstall execution
      if path_str == ".INSTALL" || path_str == ".INSTALL/" {
        std::fs::create_dir_all(&scripts_dir)?;
        let mut content = String::new();
        entry.read_to_string(&mut content)?;
        std::fs::write(scripts_dir.join("install"), &content)?;
        continue;
      }

      // Skip other metadata files (.PKGINFO, .BUILDINFO, .MTREE, .CHANGELOG)
      if path_str.starts_with('.') {
        continue;
      }

      let dest = root.join(&path);

      // Remove existing files before extracting (handles hard link conflicts).
      if (dest.exists() || dest.symlink_metadata().is_ok()) && !dest.is_dir() {
        Fs::remove_lenient(&dest);
      }

      entry.unpack_in(root)?;
      if !ManifestLayout::is_metadata(&path) {
        files.push(path);
      }
    }

    Ok(files)
  }
}

impl PacmanFormat {
  /// Parses one package's `desc` entry into a `Package`; an entry
  /// missing name, version, or filename yields `None` rather than an
  /// unfetchable record.
  fn desc_parse(content: &str) -> Result<Option<Package>> {
    let mut name = None;
    let mut version = None;
    let mut description = String::new();
    let mut depends = Vec::new();
    let mut optdepends = Vec::new();
    let mut provides = Vec::new();
    let mut conflicts = Vec::new();
    let mut filename = None;
    let mut size = 0u64;
    let mut sha256 = String::new();

    // Track which %FIELD% header we're currently under
    let mut current_field: Option<&str> = None;

    for line in content.lines() {
      let line = line.trim();

      if line.is_empty() {
        // Blank line = end of current field
        current_field = None;
        continue;
      }

      // Lines wrapped in % are field headers
      if line.starts_with('%') && line.ends_with('%') {
        current_field = Some(line);
        continue;
      }

      // Value line — dispatch based on the current field header
      match current_field {
        Some("%NAME%") => name = Some(line.to_string()),
        Some("%VERSION%") => version = Some(line.to_string()),
        Some("%DESC%") => description = line.to_string(),
        Some("%DEPENDS%") => {
          // One dependency per line, may include version: "glibc>=2.27"
          depends.push(PacmanFormat::dep_parse(line));
        }
        Some("%OPTDEPENDS%") => {
          // Format: "pkg: description" — strip description, parse just the pkg name
          let pkg_part = line.split_once(": ").map(|(p, _)| p).unwrap_or(line);
          optdepends.push(PacmanFormat::dep_parse(pkg_part));
        }
        Some("%PROVIDES%") => {
          // Parse provides with version: "libgomp.so=1-64" → name="libgomp.so", version="1-64"
          // The version is a plain version string, not a constraint with operator.
          if let Some(idx) = line.find('=') {
            provides.push(DepSpec {
              name: line[..idx].to_string(),
              version_constraint: Some(line[idx + 1..].to_string()),
            });
          } else {
            provides.push(DepSpec {
              name: line.to_string(),
              version_constraint: None,
            });
          }
        }
        Some("%CONFLICTS%") => {
          conflicts.push(PacmanFormat::dep_parse(line));
        }
        Some("%FILENAME%") => filename = Some(line.to_string()),
        Some("%CSIZE%") => {
          size = line
            .parse()
            .with_context(|| format!(".db desc: %CSIZE% '{line}' is not a valid integer"))?;
        }
        Some("%SHA256SUM%") => sha256 = line.to_string(),
        _ => {}
      }
    }

    let (Some(name), Some(version), Some(filename)) = (name, version, filename) else {
      return Ok(None);
    };
    Ok(Some(Package {
      name,
      version,
      depends,
      recommends: Vec::new(), // Arch doesn't have recommends
      suggests: optdepends,   // Arch %OPTDEPENDS% maps to suggests
      install_if: Vec::new(), // Arch doesn't have install-if (Alpine-only)
      provides,
      conflicts,
      breaks: Vec::new(), // Arch doesn't separate breaks from conflicts
      essential: false,   // Arch doesn't have essential (Debian-only)
      priority: None,     // Arch has no Priority field (Debian-only)
      description,
      filename,
      size,
      checksum: sha256,
      rich_deps: Vec::new(),
    }))
  }

  /// Reads the `.files` archive's per-package file lists into the
  /// writer's file-to-package data.
  fn files_ingest<R: Read>(reader: R, writer: &mut IndexWriter) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
      let mut entry = entry?;
      let path = entry.path()?.into_owned();
      let path_str = path.to_string_lossy().to_string();

      let dir_name = match path_str.strip_suffix("/files") {
        Some(d) => d,
        None => continue,
      };
      let pkg_name = match PacmanFormat::pkg_name_from_dir(dir_name) {
        Some(n) => n.to_string(),
        None => continue,
      };

      let mut content = String::new();
      entry.read_to_string(&mut content)?;
      PacmanFormat::files_parse(&content, &pkg_name, writer);
    }
    Ok(())
  }

  /// Records one package's `%FILES%` entries as file-to-package rows,
  /// skipping directory entries (trailing `/`).
  fn files_parse(content: &str, pkg_name: &str, writer: &mut IndexWriter) {
    let mut in_files = false;
    for line in content.lines() {
      if line.starts_with('%') && line.ends_with('%') {
        in_files = line == "%FILES%";
        continue;
      }
      if !in_files || line.is_empty() {
        continue;
      }
      if line.ends_with('/') {
        continue;
      }
      let path_abs = if line.starts_with('/') {
        line.to_string()
      } else {
        format!("/{}", line)
      };
      if let Some((dir, fname)) = path_abs.rsplit_once('/') {
        if !fname.is_empty() {
          writer.path_push(dir, fname, pkg_name);
        }
      }
    }
  }

  /// The bare package name from a `<name>-<pkgver>-<pkgrel>` directory
  /// name; `None` when the name has too few dashes to split.
  fn pkg_name_from_dir(dir_name: &str) -> Option<&str> {
    let last = dir_name.rfind('-')?;
    let rest = &dir_name[..last];
    let second_last = rest.rfind('-')?;
    Some(&dir_name[..second_last])
  }

  /// Splits one dependency line (`glibc>=2.27`) into its name and
  /// optional version constraint.
  fn dep_parse(dep: &str) -> Dependency {
    let ops = [">=", "<=", ">", "<", "="];
    for op in &ops {
      if let Some(idx) = dep.find(op) {
        let name = dep[..idx].to_string();
        let version = dep[idx..].to_string();
        return Dependency {
          alternatives: vec![DepSpec {
            name,
            version_constraint: Some(version),
          }],
        };
      }
    }

    // No version constraint — just a bare package name
    Dependency {
      alternatives: vec![DepSpec {
        name: dep.to_string(),
        version_constraint: None,
      }],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::path_index::PathIndex;
  use rusqlite::Connection;
  use std::io::Cursor;
  use tar::{Builder, Header};

  fn tar_with_files_entries(entries: &[(&str, &str)]) -> Vec<u8> {
    // entries: (tar_path, content)
    let mut buf: Vec<u8> = Vec::new();
    {
      let mut builder = Builder::new(&mut buf);
      for (path, content) in entries {
        let mut header = Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, path, content.as_bytes()).unwrap();
      }
      builder.finish().unwrap();
    }
    buf
  }

  fn run_files_ingest(tar_bytes: Vec<u8>) -> tempfile::TempDir {
    let conn = Connection::open_in_memory().unwrap();
    let mut writer = IndexWriter::new(&conn);
    PacmanFormat::files_ingest(Cursor::new(tar_bytes), &mut writer).expect("files_ingest");
    let tmp = tempfile::tempdir().unwrap();
    writer.paths_finalize(&tmp.path().join("test.pathidx")).unwrap();
    tmp
  }

  fn lookup(tmp: &tempfile::TempDir, p: &str) -> Option<String> {
    PathIndex::open(&tmp.path().join("test.pathidx"))
      .unwrap()
      .expect("index file written by run_files_ingest")
      .query(p)
      .unwrap()
  }

  #[test]
  fn pkg_name_from_dir_strips_pkgver_and_pkgrel() {
    assert_eq!(PacmanFormat::pkg_name_from_dir("glibc-2.43+r22+g8362e8ce10b2-1"), Some("glibc"));
    assert_eq!(PacmanFormat::pkg_name_from_dir("xorg-server-xvfb-21.1.10-1"), Some("xorg-server-xvfb"));
    assert_eq!(PacmanFormat::pkg_name_from_dir("openssl-3.2.1-1"), Some("openssl"));
  }

  #[test]
  fn pkg_name_from_dir_returns_none_when_too_few_dashes() {
    assert_eq!(PacmanFormat::pkg_name_from_dir("nodashes"), None);
    assert_eq!(PacmanFormat::pkg_name_from_dir("one-dash"), None);
  }

  #[test]
  fn files_ingest_indexes_files_section() {
    let glibc_files = "%FILES%\nusr/\nusr/lib/\nusr/lib/libc.so.6\nusr/lib/libm.so.6\n";
    let tar_bytes = tar_with_files_entries(&[("glibc-2.43-1/files", glibc_files)]);
    let tmp = run_files_ingest(tar_bytes);

    assert_eq!(lookup(&tmp, "/usr/lib/libc.so.6"), Some("glibc".to_string()));
    assert_eq!(lookup(&tmp, "/usr/lib/libm.so.6"), Some("glibc".to_string()));
    // Directory entries (trailing `/`) must not produce path-index entries.
    assert_eq!(lookup(&tmp, "/usr/lib"), None);
  }

  #[test]
  fn files_ingest_handles_multiple_packages() {
    let glibc = "%FILES%\nusr/lib/libc.so.6\n";
    let openssl = "%FILES%\nusr/lib/libssl.so.3\nusr/lib/libcrypto.so.3\n";
    let tar_bytes = tar_with_files_entries(&[("glibc-2.43-1/files", glibc), ("openssl-3.2.1-1/files", openssl)]);
    let tmp = run_files_ingest(tar_bytes);

    assert_eq!(lookup(&tmp, "/usr/lib/libc.so.6"), Some("glibc".to_string()));
    assert_eq!(lookup(&tmp, "/usr/lib/libssl.so.3"), Some("openssl".to_string()));
    assert_eq!(lookup(&tmp, "/usr/lib/libcrypto.so.3"), Some("openssl".to_string()));
  }

  #[test]
  fn files_ingest_skips_non_files_entries() {
    // .files repos contain BOTH `<pkg>/desc` AND `<pkg>/files` entries.
    // files_ingest must process only the `files` ones.
    let desc = "%NAME%\nglibc\n";
    let files = "%FILES%\nusr/lib/libc.so.6\n";
    let tar_bytes = tar_with_files_entries(&[("glibc-2.43-1/desc", desc), ("glibc-2.43-1/files", files)]);
    let tmp = run_files_ingest(tar_bytes);

    assert_eq!(lookup(&tmp, "/usr/lib/libc.so.6"), Some("glibc".to_string()));
  }
}
