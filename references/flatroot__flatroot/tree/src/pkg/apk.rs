//! The Alpine (apk) package format: parses `APKINDEX` into `Package`
//! records and extracts `.apk` archives.

use std::io::Read;
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

/// The Alpine (apk) package format.
pub struct ApkFormat;

/// The per-package accumulator for one APKINDEX block, flushed into a
/// `Package` at every blank line and once more at end of input.
/// Flushing takes the whole draft, so a block dropped for lacking its
/// name or version can never leak gathered fields into the next block.
#[derive(Default)]
struct ApkDraft {
  name: Option<String>,
  version: Option<String>,
  description: String,
  depends: Vec<Dependency>,
  provides: Vec<DepSpec>,
  install_if: Vec<String>,
  size: u64,
  checksum: String,
}

impl ApkDraft {
  /// A complete draft becomes a `Package`; one missing its name or
  /// version yields `None`. Either way the draft resets for the next
  /// block.
  fn flush(&mut self) -> Option<Package> {
    let draft = std::mem::take(self);
    let (name, version) = (draft.name?, draft.version?);
    let filename = format!("{}-{}.apk", name, version);
    Some(Package {
      name,
      version,
      depends: draft.depends,
      provides: draft.provides,
      recommends: Vec::new(),
      suggests: Vec::new(),
      install_if: draft.install_if,
      conflicts: Vec::new(),
      breaks: Vec::new(),
      essential: false,
      priority: None,
      description: draft.description,
      filename,
      size: draft.size,
      checksum: draft.checksum,
      rich_deps: Vec::new(),
    })
  }
}

impl PackageFormat for ApkFormat {
  /// Reads each repository's `APKINDEX` from its own mirrors into the
  /// package index, recording where each package can be fetched. Alpine
  /// publishes no file lists, so no file-to-package data is built.
  fn index_fetch(&self, source: &SourceDistro, writer: &mut IndexWriter, _http: &Arc<HttpClient>) -> Result<()> {
    for repo in &source.repos {
      let (path_apkindex, path_dir_packages) = match &repo.layout {
        crate::distro::LayoutRepository::Apk {
          path_file_apkindex,
          path_dir_packages,
        } => (path_file_apkindex, path_dir_packages),
        other => bail!("apk index_fetch received non-Apk layout for repository {}: {:?}", repo.label, other),
      };

      let (bytes, mirror_origin) = repo
        .mirrors
        .fetch_with_origin(path_apkindex)
        .with_context(|| format!("APKINDEX for {}", repo.label))?;

      let decoder = flate2::read::MultiGzDecoder::new(&bytes[..]);
      let mut archive = tar::Archive::new(decoder);

      for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        if path.to_string_lossy() != "APKINDEX" {
          continue;
        }
        let mut content = String::new();
        entry.read_to_string(&mut content)?;

        let packages: Vec<Package> = ApkFormat::apkindex_parse(&content)?
          .into_iter()
          .map(|mut pkg| {
            pkg.filename = format!("{}|{}/{}", mirror_origin.trim_end_matches('/'), path_dir_packages, pkg.filename);
            pkg
          })
          .collect();
        writer.packages_insert(packages, &crate::version::ApkVersionCompare)?;
      }
    }

    Ok(())
  }

  /// Extracts one `.apk` — three concatenated gzip streams: the
  /// signature is discarded, the control scripts are saved per package
  /// for the post-install pass, and the data files go into the rootfs.
  /// Returns the files placed, excluding `.flatroot/` metadata.
  fn extract(&self, path_file_archive: &Path, root: &Path, pkg: &PackageIdentity) -> Result<Vec<PathBuf>> {
    let data =
      std::fs::read(path_file_archive).with_context(|| format!("Failed to read {}", path_file_archive.display()))?;

    // BufReader wrapping a Cursor — needed for bufread::GzDecoder
    let mut cursor = std::io::BufReader::new(std::io::Cursor::new(&data));

    // Stream 0: signature — decompress and discard
    {
      let mut sig = flate2::bufread::GzDecoder::new(&mut cursor);
      std::io::copy(&mut sig, &mut std::io::sink())?;
    }

    // Stream 1: control — extract post-install and pre-install scripts
    {
      let control_gz = flate2::bufread::GzDecoder::new(&mut cursor);
      let mut control_tar = tar::Archive::new(control_gz);

      let scripts_dir = ManifestLayout::new(root).dir_scripts_entry(pkg);

      for entry in control_tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let name = path.to_string_lossy();

        // Save post-install and pre-install scripts
        // ".post-install" → saved as "post-install" (strip leading dot)
        if name == ".post-install" || name == ".pre-install" {
          std::fs::create_dir_all(&scripts_dir)?;
          let script_name = name.trim_start_matches('.');
          let mut content = String::new();
          entry.read_to_string(&mut content)?;
          std::fs::write(scripts_dir.join(script_name), &content)?;
        }
        // .PKGINFO and other control files are skipped
      }

      // tar::Archive stops at the logical tar end (two zero blocks) and leaves
      // the control member's 8-byte gzip trailer unconsumed; draining the
      // decoder to EOF advances the shared cursor to the start of the data
      // member rather than into the middle of that trailer.
      let mut control_gz = control_tar.into_inner();
      std::io::copy(&mut control_gz, &mut std::io::sink())?;
    }

    // Stream 2: data — extract files to rootfs using the shared tar extractor.
    // Only payload paths (non-.flatroot) contribute to the returned list.
    let mut captured = {
      let data_gz = flate2::bufread::GzDecoder::new(&mut cursor);
      Tar::extract(data_gz, root)?
    };
    captured.retain(|p| !ManifestLayout::is_metadata(p));

    Ok(captured)
  }
}

impl ApkFormat {
  /// Parses `APKINDEX` blocks into `Package` records, including the
  /// `i:` install-if rule (auto-install when a set of other packages is
  /// present) that only Alpine has.
  fn apkindex_parse(content: &str) -> Result<Vec<Package>> {
    let mut packages = Vec::new();
    let mut draft = ApkDraft::default();

    for line in content.lines() {
      if line.is_empty() {
        packages.extend(draft.flush());
        continue;
      }

      if let Some(val) = line.strip_prefix("P:") {
        draft.name = Some(val.to_string());
      } else if let Some(val) = line.strip_prefix("V:") {
        draft.version = Some(val.to_string());
      } else if let Some(val) = line.strip_prefix("T:") {
        draft.description = val.to_string();
      } else if let Some(val) = line.strip_prefix("S:") {
        draft.size = val
          .parse()
          .with_context(|| format!("APKINDEX: S: field '{val}' is not a valid integer"))?;
      } else if let Some(val) = line.strip_prefix("D:") {
        draft.depends = ApkFormat::deps_parse(val);
      } else if let Some(val) = line.strip_prefix("p:") {
        draft.provides = ApkFormat::provides_parse(val);
      } else if let Some(val) = line.strip_prefix("i:") {
        draft.install_if = ApkFormat::install_if_parse(val);
      } else if let Some(val) = line.strip_prefix("C:") {
        draft.checksum = val.to_string();
      }
    }
    packages.extend(draft.flush());

    Ok(packages)
  }

  /// Alpine's version-constraint operators, longest first so `>=` is never
  /// misread as `>` (`~=`/`~` are the compatible-release forms).
  const VERSION_OPS: [&'static str; 7] = [">=", "<=", "~=", ">", "<", "=", "~"];

  /// Split one dependency token into the bare name and whatever constraint
  /// is glued onto it: `"libglycin~2.1.0"` → `("libglycin", Some("~2.1.0"))`.
  fn constraint_split(token: &str) -> (&str, Option<&str>) {
    for op in &Self::VERSION_OPS {
      if let Some(idx) = token.find(op) {
        return (&token[..idx], Some(&token[idx..]));
      }
    }
    (token, None)
  }

  /// The packages whose joint presence auto-installs this one; version
  /// constraints are dropped, since only the names matter.
  fn install_if_parse(raw: &str) -> Vec<String> {
    raw
      .split_whitespace()
      .map(|token| Self::constraint_split(token).0.to_string())
      .collect()
  }

  /// Parses Alpine's one-line dependency list into `Dependency`
  /// entries, skipping the `!pkg` conflict entries.
  fn deps_parse(raw: &str) -> Vec<Dependency> {
    let mut deps = Vec::new();
    for token in raw.split_whitespace() {
      // Negative deps (`!pkg`) describe conflicts, not needs.
      if token.starts_with('!') {
        continue;
      }
      let (name, constraint) = Self::constraint_split(token);
      deps.push(Dependency {
        alternatives: vec![DepSpec {
          name: name.to_string(),
          version_constraint: constraint.map(|c| c.to_string()),
        }],
      });
    }
    deps
  }

  /// Parses the `p:` provides list — virtual names and library names,
  /// each optionally versioned — which stands in for the file lists
  /// Alpine never publishes.
  fn provides_parse(raw: &str) -> Vec<DepSpec> {
    raw
      .split_whitespace()
      .map(|token| {
        if let Some(idx) = token.find('=') {
          DepSpec {
            name: token[..idx].to_string(),
            version_constraint: Some(token[idx + 1..].to_string()),
          }
        } else {
          DepSpec {
            name: token.to_string(),
            version_constraint: None,
          }
        }
      })
      .collect()
  }
}
