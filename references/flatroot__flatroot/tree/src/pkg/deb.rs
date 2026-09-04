//! The Debian package format: parses the `Packages.gz` index and the
//! `Contents` file listings, and extracts `.deb` archives.

use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};

use crate::db::IndexWriter;
use crate::distro::SourceDistro;
use crate::internal::http::HttpClient;
use crate::internal::tar::Tar;
use crate::manifest::ManifestLayout;
use crate::package::{DepSpec, Dependency, Package, PackageIdentity};
use crate::pkg::PackageFormat;
use crate::ui::RunVoice;

/// The Debian-family `PackageFormat` implementation.
pub struct DebFormat;

impl PackageFormat for DebFormat {
  /// Builds the package index and the file-to-package data from each of
  /// the source's repositories, fetching every piece through that
  /// repository's own mirror list.
  fn index_fetch(&self, source: &SourceDistro, writer: &mut IndexWriter, _http: &Arc<HttpClient>) -> Result<()> {
    for repo in &source.repos {
      let (path_dir_suite, component, arch_str, paths_file_contents) = match &repo.layout {
        crate::distro::LayoutRepository::Deb {
          path_dir_suite,
          component,
          arch,
          paths_file_contents,
        } => (path_dir_suite, component, arch, paths_file_contents),
        other => bail!("deb index_fetch received non-Deb layout for repository {}: {:?}", repo.label, other),
      };

      let path_packages = format!("{path_dir_suite}/{component}/binary-{arch_str}/Packages.gz");
      let (bytes_packages, mirror_origin) = repo
        .mirrors
        .fetch_with_origin(&path_packages)
        .with_context(|| format!("Packages.gz for {}", repo.label))?;

      let mut decoder = flate2::read::GzDecoder::new(&bytes_packages[..]);
      let mut text_packages = String::new();
      decoder.read_to_string(&mut text_packages)?;
      let packages = DebFormat::catalogue_parse(&text_packages, Some(&mirror_origin))?;
      writer.packages_insert(packages, &crate::version::DpkgVersionCompare)?;

      // Each declared contents listing is fetched and ingested; one that
      // is definitively absent from every mirror is skipped, because the
      // long-term archive folds `Contents-all` into the per-arch listing
      // and drops the separate file. A repo whose every declared listing
      // is absent contributes no ownership data at all — that is never
      // accepted silently.
      let mut count_listings_ingested = 0usize;
      for path_contents in paths_file_contents {
        let fetched = repo
          .mirrors
          .fetch_with_origin_optional(path_contents)
          .with_context(|| format!("Contents file {} for {}", path_contents, repo.label))?;
        let Some((bytes_contents, _)) = fetched else {
          RunVoice::warn(format!(
            "contents listing {} absent on every mirror for {} — skipped",
            path_contents, repo.label
          ));
          continue;
        };
        let reader_contents = std::io::BufReader::new(flate2::read::GzDecoder::new(&bytes_contents[..]));
        DebFormat::contents_ingest(reader_contents, writer)?;
        count_listings_ingested += 1;
      }
      if !paths_file_contents.is_empty() && count_listings_ingested == 0 {
        bail!("every contents listing absent for {} — the repository publishes no file-ownership data", repo.label);
      }
    }

    Ok(())
  }

  /// Extracts one `.deb`: the `data.tar.*` files go into the rootfs,
  /// and the `control.tar.*` maintainer scripts are saved per package
  /// for the post-install pass. Returns the files placed, excluding
  /// `.flatroot/` metadata.
  fn extract(&self, path_file_archive: &Path, root: &Path, pkg: &PackageIdentity) -> Result<Vec<PathBuf>> {
    let file = std::fs::File::open(path_file_archive)
      .with_context(|| format!("Failed to open package archive: {}", path_file_archive.display()))?;
    let mut archive = ar::Archive::new(file);
    let mut found_data = false;
    let mut files: Vec<PathBuf> = Vec::new();

    while let Some(entry) = archive.next_entry() {
      let mut entry = entry?;
      let name = String::from_utf8_lossy(entry.header().identifier()).to_string();

      if name.starts_with("data.tar") {
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        for p in Tar::extract_compressed(&name, &data, root)? {
          if !ManifestLayout::is_metadata(&p) {
            files.push(p);
          }
        }
        found_data = true;
      } else if name.starts_with("control.tar") {
        let scripts_dir = ManifestLayout::new(root).dir_scripts_entry(pkg);
        std::fs::create_dir_all(&scripts_dir)?;

        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        Tar::extract_compressed(&name, &data, &scripts_dir)?;
      }
    }

    if !found_data {
      bail!("No data.tar.* found in {}", path_file_archive.display());
    }
    Ok(files)
  }
}

impl DebFormat {
  /// Parses Debian's blank-line-separated `Packages` text into
  /// `Package` records, stamping each with the mirror it was found on
  /// so a later download can rebuild the archive's full URL.
  fn catalogue_parse(text: &str, url_prefix: Option<&str>) -> Result<Vec<Package>> {
    let mut packages: Vec<Package> = Vec::new();
    let mut current: Option<PackageBuilder> = None;

    for line in text.lines() {
      if line.is_empty() {
        Self::block_flush(&mut current, url_prefix, &mut packages)?;
        continue;
      }

      if let Some(ref mut builder) = current {
        if line.starts_with(' ') || line.starts_with('\t') {
          continue;
        }

        if let Some((key, value)) = line.split_once(": ") {
          builder.set(key, value);
        }
      } else if let Some((key, value)) = line.split_once(": ") {
        let mut builder = PackageBuilder::default();
        builder.set(key, value);
        current = Some(builder);
      }
    }

    Self::block_flush(&mut current, url_prefix, &mut packages)?;
    Ok(packages)
  }

  /// Closes the block being gathered, if any: a complete builder
  /// becomes a finished record with its mirror origin stamped; an
  /// incomplete one yields nothing. Runs at every blank line and once
  /// more for the final block.
  fn block_flush(
    current: &mut Option<PackageBuilder>,
    url_prefix: Option<&str>,
    packages: &mut Vec<Package>,
  ) -> Result<()> {
    let Some(builder) = current.take() else {
      return Ok(());
    };
    let Some(mut pkg) = builder.build()? else {
      return Ok(());
    };
    PackageBuilder::url_prefix_apply(&mut pkg.filename, url_prefix);
    packages.push(pkg);
    Ok(())
  }

  /// Streams the large Contents listing line by line into the writer's
  /// file-ownership half, so it is never held in memory; a file claimed by
  /// several packages simply accumulates all its owners.
  fn contents_ingest(mut reader: impl BufRead, writer: &mut IndexWriter) -> Result<()> {
    let mut bytes_line = Vec::with_capacity(256);
    loop {
      bytes_line.clear();
      let n_read = reader
        .read_until(b'\n', &mut bytes_line)
        .context("Failed to read Contents line")?;
      if n_read == 0 {
        break;
      }
      let line_lossy = String::from_utf8_lossy(&bytes_line);
      let line_trimmed = line_lossy.trim_end_matches(|c| c == '\n' || c == '\r').trim_end();
      if line_trimmed.is_empty() {
        continue;
      }

      // Paths in Debian Contents never contain whitespace; the package
      // list does (comma-space). Splitting at the FIRST whitespace
      // boundary separates the two cleanly even for multi-package lines
      // like `etc/passwd  base/base-passwd, admin/passwd`.
      let split_at = match line_trimmed.find(|c: char| c.is_whitespace()) {
        Some(p) => p,
        None => continue,
      };
      let path = &line_trimmed[..split_at];
      let pkg_list = line_trimmed[split_at..].trim_start();

      if path.is_empty() || !path.contains('/') || pkg_list.is_empty() {
        // Header (`FILE  LOCATION`), preamble lines, blank rows.
        continue;
      }

      // Contents uses no leading slash; the on-disk index stores absolute
      // paths so query and ingest agree on the dir field.
      let path_abs = if path.starts_with('/') {
        path.to_string()
      } else {
        format!("/{}", path)
      };
      let (dir, fname) = match path_abs.rsplit_once('/') {
        Some(pair) => pair,
        None => continue,
      };
      if fname.is_empty() {
        continue;
      }

      for token in pkg_list.split(',') {
        let token = token.trim();
        if token.is_empty() {
          continue;
        }
        // Strip the descriptive `<section>/` prefix; the section is for
        // human readers, not for the path index.
        let pkg = token.rsplit_once('/').map(|(_, n)| n).unwrap_or(token);
        if pkg.is_empty() {
          continue;
        }
        writer.path_push(dir, fname, pkg);
      }
    }
    Ok(())
  }
}

/// One package accumulated field by field from its `Packages` block,
/// assembled into a `Package` only when the block ends.
#[derive(Default)]
struct PackageBuilder {
  name: Option<String>,
  version: Option<String>,
  depends: Option<String>,
  pre_depends: Option<String>,
  recommends: Option<String>,
  suggests: Option<String>,
  provides: Option<String>,
  conflicts: Option<String>,
  breaks: Option<String>,
  essential: bool,
  priority: Option<String>,
  description: Option<String>,
  filename: Option<String>,
  size: Option<String>,
  sha256: Option<String>,
}

impl PackageBuilder {
  /// Records one labelled line against the matching field, ignoring any
  /// label the build has no use for.
  fn set(&mut self, key: &str, value: &str) {
    match key {
      "Package" => self.name = Some(value.to_string()),
      "Version" => self.version = Some(value.to_string()),
      "Depends" => self.depends = Some(value.to_string()),
      "Pre-Depends" => self.pre_depends = Some(value.to_string()),
      "Recommends" => self.recommends = Some(value.to_string()),
      "Suggests" => self.suggests = Some(value.to_string()),
      "Provides" => self.provides = Some(value.to_string()),
      "Conflicts" => self.conflicts = Some(value.to_string()),
      "Breaks" => self.breaks = Some(value.to_string()),
      "Essential" => self.essential = value == "yes",
      "Priority" => self.priority = Some(value.to_string()),
      "Description" => self.description = Some(value.to_string()),
      "Filename" => self.filename = Some(value.to_string()),
      "Size" => self.size = Some(value.to_string()),
      "SHA256" => self.sha256 = Some(value.to_string()),
      _ => {}
    }
  }

  /// Assembles the gathered fields into a `Package`; a block missing
  /// name, version, or filename yields `None` rather than a malformed
  /// record. Dependency text is parsed into structured form.
  fn build(self) -> Result<Option<Package>> {
    let Some(name) = self.name else { return Ok(None) };
    let Some(version) = self.version else { return Ok(None) };
    let Some(filename) = self.filename else { return Ok(None) };

    let size = match self.size {
      Some(raw) => raw
        .parse()
        .with_context(|| format!("Packages index: Size '{raw}' is not a valid integer for {name}"))?,
      None => 0,
    };

    let mut depends = PackageBuilder::depends_parse(self.depends.as_deref().unwrap_or(""));
    let mut pre_depends = PackageBuilder::depends_parse(self.pre_depends.as_deref().unwrap_or(""));
    depends.append(&mut pre_depends);

    Ok(Some(Package {
      name,
      version,
      depends,
      recommends: PackageBuilder::depends_parse(self.recommends.as_deref().unwrap_or("")),
      suggests: PackageBuilder::depends_parse(self.suggests.as_deref().unwrap_or("")),
      install_if: Vec::new(),
      provides: PackageBuilder::provides_parse(self.provides.as_deref().unwrap_or("")),
      conflicts: PackageBuilder::depends_parse(self.conflicts.as_deref().unwrap_or("")),
      breaks: PackageBuilder::depends_parse(self.breaks.as_deref().unwrap_or("")),
      essential: self.essential,
      priority: self.priority.as_deref().and_then(PackageBuilder::priority_ordinal),
      description: self.description.unwrap_or_default(),
      filename,
      size,
      checksum: self.sha256.unwrap_or_default(),
      rich_deps: Vec::new(),
    }))
  }

  /// Stamps the mirror origin onto the archive's relative location, so a
  /// later download can rebuild the full address.
  fn url_prefix_apply(filename: &mut String, url_prefix: Option<&str>) {
    if let Some(prefix) = url_prefix {
      *filename = format!("{}|{}", prefix, filename);
    }
  }

  /// Parses Debian's dependency notation (`a | b, c (>= 1.0)`) into the
  /// groups-and-alternatives the resolver walks, stripping `:arch`
  /// qualifiers.
  fn depends_parse(raw: &str) -> Vec<Dependency> {
    if raw.is_empty() {
      return Vec::new();
    }

    raw
      .split(", ")
      .map(|group| {
        let alternatives = group
          .split(" | ")
          .map(|alt| {
            let alt = alt.trim();
            if let Some(idx) = alt.find(" (") {
              let name = PackageBuilder::arch_qualifier_strip(&alt[..idx]);
              let constraint = alt[idx + 2..].trim_end_matches(')').to_string();
              DepSpec {
                name,
                version_constraint: Some(constraint),
              }
            } else {
              DepSpec {
                name: PackageBuilder::arch_qualifier_strip(alt),
                version_constraint: None,
              }
            }
          })
          .collect();
        Dependency { alternatives }
      })
      .collect()
  }

  /// Parses the `Provides:` list into `DepSpec`s, each optionally
  /// version-pinned.
  fn provides_parse(raw: &str) -> Vec<DepSpec> {
    if raw.is_empty() {
      return Vec::new();
    }

    raw
      .split(", ")
      .map(|s| {
        let s = s.trim();
        if let Some(idx) = s.find(" (") {
          let name = s[..idx].trim().to_string();
          let raw = s[idx + 2..].trim_end_matches(')').trim();
          let version = raw.strip_prefix("= ").or(raw.strip_prefix("=")).unwrap_or(raw);
          DepSpec {
            name,
            version_constraint: Some(version.to_string()),
          }
        } else {
          DepSpec {
            name: s.to_string(),
            version_constraint: None,
          }
        }
      })
      .collect()
  }

  /// Strips a dependency's `:arch` qualifier, since the builder matches
  /// packages by plain name for one chosen architecture.
  fn arch_qualifier_strip(name: &str) -> String {
    if let Some(idx) = name.find(':') {
      name[..idx].to_string()
    } else {
      name.to_string()
    }
  }

  /// Maps Debian's named priority levels to an ordinal used in provider
  /// selection; an unrecognized value maps to `None`.
  fn priority_ordinal(value: &str) -> Option<u8> {
    match value.trim() {
      "required" => Some(0),
      "important" => Some(1),
      "standard" => Some(2),
      "optional" => Some(3),
      "extra" => Some(4),
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::path_index::PathIndex;
  use rusqlite::Connection;

  fn ingest_to_index(body: &[u8]) -> tempfile::TempDir {
    let conn = Connection::open_in_memory().unwrap();
    let mut writer = IndexWriter::new(&conn);
    DebFormat::contents_ingest(body, &mut writer).expect("contents_ingest");
    let tmp = tempfile::tempdir().unwrap();
    writer.paths_finalize(&tmp.path().join("test.pathidx")).unwrap();
    tmp
  }

  fn lookup(tmp: &tempfile::TempDir, p: &str) -> Option<String> {
    PathIndex::open(&tmp.path().join("test.pathidx"))
      .unwrap()
      .expect("index file written by ingest_to_index")
      .query(p)
      .unwrap()
  }

  #[test]
  fn contents_ingest_simple_line() {
    let body = b"bin/bash                                        shells/bash\n";
    let tmp = ingest_to_index(body);
    assert_eq!(lookup(&tmp, "/bin/bash"), Some("bash".to_string()));
  }

  #[test]
  fn contents_ingest_multi_package_line() {
    let body = b"etc/passwd                              base/base-passwd, admin/passwd\n";
    let tmp = ingest_to_index(body);
    // Both packages own the same path — query returns one of them
    // (the binary search picks the first matching tuple after sort).
    let result = lookup(&tmp, "/etc/passwd").expect("must resolve");
    assert!(result == "base-passwd" || result == "passwd", "unexpected provider: {result}");
  }

  #[test]
  fn contents_ingest_skips_header_line() {
    let body = b"FILE                                                    LOCATION\nbin/bash    shells/bash\n";
    let tmp = ingest_to_index(body);
    assert_eq!(lookup(&tmp, "/bin/bash"), Some("bash".to_string()));
    // The header line's "FILE" path has no slash — must not appear.
    assert_eq!(lookup(&tmp, "FILE"), None);
  }

  #[test]
  fn contents_ingest_skips_blank_lines() {
    let body = b"\n\nbin/bash    shells/bash\n\n\nbin/cat    utils/coreutils\n";
    let tmp = ingest_to_index(body);
    assert_eq!(lookup(&tmp, "/bin/bash"), Some("bash".to_string()));
    assert_eq!(lookup(&tmp, "/bin/cat"), Some("coreutils".to_string()));
  }

  #[test]
  fn contents_ingest_skips_preamble() {
    let body = b"This file maps each file to its owning package.\n\
                 The first column is the path.\n\
                 \n\
                 FILE                                                    LOCATION\n\
                 usr/bin/dpkg                          admin/dpkg\n";
    let tmp = ingest_to_index(body);
    assert_eq!(lookup(&tmp, "/usr/bin/dpkg"), Some("dpkg".to_string()));
  }

  #[test]
  fn contents_ingest_handles_pkg_without_section() {
    let body = b"usr/bin/foo                                    foopkg\n";
    let tmp = ingest_to_index(body);
    assert_eq!(lookup(&tmp, "/usr/bin/foo"), Some("foopkg".to_string()));
  }

  #[test]
  fn contents_ingest_unknown_path_returns_none() {
    let body = b"bin/bash    shells/bash\n";
    let tmp = ingest_to_index(body);
    assert_eq!(lookup(&tmp, "/usr/lib/libfoo.so.6"), None);
  }

  #[test]
  fn contents_ingest_tolerates_non_utf8_bytes() {
    // Older Ubuntu suites (xenial and earlier) publish Contents rows
    // whose path bytes are not all valid UTF-8. The ingest must decode
    // lossily and keep going rather than aborting the whole stream.
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(b"usr/share/locale/bad");
    body.push(0xff); // invalid UTF-8 byte mid-path
    body.extend_from_slice(b"/file                  l10n/badpkg\n");
    body.extend_from_slice(b"bin/bash                                        shells/bash\n");
    let tmp = ingest_to_index(&body);
    // The valid row after the bad one must still be ingested.
    assert_eq!(lookup(&tmp, "/bin/bash"), Some("bash".to_string()));
  }

  #[test]
  fn priority_ordinal_maps_known_values() {
    assert_eq!(PackageBuilder::priority_ordinal("required"), Some(0));
    assert_eq!(PackageBuilder::priority_ordinal("important"), Some(1));
    assert_eq!(PackageBuilder::priority_ordinal("standard"), Some(2));
    assert_eq!(PackageBuilder::priority_ordinal("optional"), Some(3));
    assert_eq!(PackageBuilder::priority_ordinal("extra"), Some(4));
  }

  #[test]
  fn priority_ordinal_rejects_unknown_values() {
    assert_eq!(PackageBuilder::priority_ordinal("source"), None);
    assert_eq!(PackageBuilder::priority_ordinal(""), None);
    assert_eq!(PackageBuilder::priority_ordinal("REQUIRED"), None);
  }
}
