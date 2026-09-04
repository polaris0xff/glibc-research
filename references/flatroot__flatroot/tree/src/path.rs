//! Turns a full-path pattern into the packages that ship matching files,
//! each match carried with the facts a listing needs. Unlike the library
//! search, paths are matched and reported exactly as the file lists
//! record them, unfiltered.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::db::Index;

/// One path-query hit: a matched installed path and the package that
/// ships it, carrying the version and summary a listing shows so the hit
/// can be presented without a second lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMatch {
  /// Owning-package name as recorded in the index.
  pub package: String,
  /// Matched file's installed path, leading slash stripped, e.g.
  /// `usr/bin/bash`.
  pub path: String,
  /// Owning package's version.
  pub version: String,
  /// Owning package's description.
  pub description: String,
}

/// The two descriptive facts a listing shows about an owning package.
#[derive(Clone)]
struct PackageMeta {
  version: String,
  description: String,
}

impl PathMatch {
  /// Matches `pattern` against every recorded file path, pairs each hit
  /// with its owning package's version and description, and returns
  /// them sorted and deduplicated. A source without a path index is an
  /// error rather than an empty answer; the one family where that
  /// absence is by design (apk) is refused by the caller before this is
  /// reached.
  pub fn glob(index: &Index, pattern: &str) -> Result<Vec<PathMatch>> {
    let Some(path_index) = index.paths().context("Failed to open path index")? else {
      bail!("path index missing for this source — the catalogue was populated without file lists");
    };
    let matches_path = path_index
      .query_glob_path(pattern)
      .context("Failed to query path index")?;

    let mut cache_meta: HashMap<String, PackageMeta> = HashMap::new();
    let mut out: Vec<PathMatch> = Vec::new();
    for (path, package) in matches_path {
      let meta = Self::package_meta(index, &package, &mut cache_meta)?;
      out.push(PathMatch {
        package,
        path,
        version: meta.version,
        description: meta.description,
      });
    }

    out.sort_by(|a, b| (&a.path, &a.package).cmp(&(&b.path, &b.package)));
    out.dedup_by(|a, b| a.path == b.path && a.package == b.package);
    Ok(out)
  }

  /// One package's listing facts, looked up at most once per package. A
  /// package named by the path index but absent from the packages table is an
  /// index inconsistency worth failing on, not a hole to skip.
  fn package_meta(index: &Index, package: &str, cache_meta: &mut HashMap<String, PackageMeta>) -> Result<PackageMeta> {
    if let Some(cached) = cache_meta.get(package) {
      return Ok(cached.clone());
    }
    let row = index.packages().get(package)?.ok_or_else(|| {
      anyhow::anyhow!("package '{}' from path index not found in packages table — index inconsistency", package)
    })?;
    let meta = PackageMeta {
      version: row.version,
      description: row.description,
    };
    cache_meta.insert(package.to_string(), meta.clone());
    Ok(meta)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::package::Package;
  use crate::path_index::PathIndexBuilder;
  use tempfile::TempDir;

  fn pkg(name: &str, version: &str, description: &str) -> Package {
    Package {
      description: description.into(),
      ..crate::package::fixtures::pkg(name, version)
    }
  }

  /// An on-disk index whose companion path index holds the given
  /// file-ownership rows, so the path search has real data to consult.
  fn index_with_paths(packages: Vec<Package>, entries: &[(&str, &str, &str)]) -> (TempDir, Index) {
    let tmp = TempDir::new().unwrap();
    let index = Index::on_disk(&tmp.path().join("test.db"), packages);
    let mut builder = PathIndexBuilder::new();
    for (dir, fname, pkg) in entries {
      builder.entry_push(dir, fname, pkg);
    }
    builder.finalize(&tmp.path().join("test.pathidx")).unwrap();
    (tmp, index)
  }

  // ── glob ─────────────────────────────────────────────────────────────

  #[test]
  fn glob_joins_owning_package_metadata() {
    let (_tmp, index) = index_with_paths(
      vec![pkg("bash", "5.2.15-2", "GNU Bourne Again SHell")],
      &[("/usr/bin", "bash", "bash"), ("/usr/bin", "bashbug", "bash")],
    );

    let matches = PathMatch::glob(&index, "usr/bin/bash").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].package, "bash");
    assert_eq!(matches[0].path, "usr/bin/bash");
    assert_eq!(matches[0].version, "5.2.15-2");
    assert_eq!(matches[0].description, "GNU Bourne Again SHell");
  }

  #[test]
  fn glob_keeps_non_library_files_the_library_search_rejects() {
    let (_tmp, index) =
      index_with_paths(vec![pkg("base-files", "12.4", "base system files")], &[("/etc", "fstab", "base-files")]);

    let matches = PathMatch::glob(&index, "etc/fstab").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].path, "etc/fstab");
    assert_eq!(matches[0].package, "base-files");
  }

  #[test]
  fn glob_sorted_by_path_then_package() {
    let (_tmp, index) = index_with_paths(
      vec![pkg("foo-cli", "1.0", ""), pkg("foo-lib", "1.0", "")],
      &[("/usr/lib", "foo", "foo-lib"), ("/usr/bin", "foo", "foo-cli")],
    );

    let matches = PathMatch::glob(&index, "usr/*/foo").unwrap();
    let pairs: Vec<(&str, &str)> = matches.iter().map(|m| (m.path.as_str(), m.package.as_str())).collect();
    assert_eq!(pairs, vec![("usr/bin/foo", "foo-cli"), ("usr/lib/foo", "foo-lib")]);
  }

  #[test]
  fn glob_returns_empty_when_no_match() {
    let (_tmp, index) = index_with_paths(vec![pkg("bash", "5.2", "")], &[("/usr/bin", "bash", "bash")]);

    let matches = PathMatch::glob(&index, "opt/*").unwrap();
    assert!(matches.is_empty());
  }

  #[test]
  fn glob_fails_when_path_index_missing() {
    // An in-memory index has no companion path index on disk. For the
    // path search that absence is an error, never an empty answer — the
    // apk-by-design case is gated by the caller before glob is reached.
    let index = Index::in_memory(vec![pkg("bash", "5.2", "")]);

    let err = PathMatch::glob(&index, "usr/bin/bash").unwrap_err();
    assert!(err.to_string().contains("path index missing"), "got: {err}");
  }

  #[test]
  fn glob_fails_on_package_missing_from_catalogue() {
    let (_tmp, index) = index_with_paths(vec![pkg("bash", "5.2", "")], &[("/usr/bin", "ghost", "ghostpkg")]);

    let err = PathMatch::glob(&index, "usr/bin/ghost").unwrap_err();
    assert!(err.to_string().contains("index inconsistency"), "got: {err}");
  }
}
