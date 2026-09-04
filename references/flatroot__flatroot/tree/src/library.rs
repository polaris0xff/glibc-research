//! Turns a library-name pattern into the packages that provide it,
//! drawing on both places a distribution records a library — provides
//! entries and shipped files — each shown in one normalized form so the
//! answer reads the same across distributions.

use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::db::Index;

/// One library-name query hit: a library and the package that provides
/// it, carrying the version and description a listing shows so the hit
/// can be presented without a second lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryMatch {
  /// Owning-package name as recorded in the index.
  pub package: String,
  /// Library name in canonical form, with known decorations stripped (see
  /// `canonicalize`).
  pub library: String,
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

impl LibraryMatch {
  /// Merges the two ways a distribution records a library — provides
  /// entries and shipped files — into one deduplicated list, since
  /// either source alone is incomplete. Non-library names are filtered
  /// out and each library is shown in one normalized form.
  pub fn glob(index: &Index, pattern: &str) -> Result<Vec<LibraryMatch>> {
    let mut out = Self::matches_from_provides(index, pattern)?;
    out.extend(Self::matches_from_path_index(index, pattern)?);

    out.sort_by(|a, b| (&a.library, &a.package).cmp(&(&b.library, &b.package)));
    out.dedup_by(|a, b| a.library == b.library && a.package == b.package);
    Ok(out)
  }

  /// The provides side: sonames packages declare. Queried twice — raw
  /// (covers Arch/RPM decorations under a trailing `*`) and
  /// `so:`-prefixed (covers Alpine, whose sonames live only there) —
  /// with decorations stripped to one canonical form.
  fn matches_from_provides(index: &Index, pattern: &str) -> Result<Vec<LibraryMatch>> {
    let mut rows_provides = index
      .providers()
      .glob(pattern)
      .context("Failed to query provides table")?;
    let pattern_alpine = format!("so:{}", pattern);
    rows_provides.extend(
      index
        .providers()
        .glob(&pattern_alpine)
        .context("Failed to query provides table (so: namespace)")?,
    );

    let mut out: Vec<LibraryMatch> = Vec::new();
    for row in rows_provides {
      if !Self::is_library_shape(&row.provides_name) {
        continue;
      }
      out.push(LibraryMatch {
        package: row.package_name,
        library: Self::canonicalize(&row.provides_name).to_string(),
        version: row.package_version,
        description: row.package_description,
      });
    }
    Ok(out)
  }

  /// The file side: library files packages place on disk, so a library
  /// with no provides entry is still found. Package facts are joined
  /// through a per-package cache; a missing path index contributes
  /// nothing.
  fn matches_from_path_index(index: &Index, pattern: &str) -> Result<Vec<LibraryMatch>> {
    let Some(path_index) = index.paths().context("Failed to open path index")? else {
      return Ok(Vec::new());
    };
    let matches_path = path_index.query_glob(pattern).context("Failed to query path index")?;

    let mut cache_meta: HashMap<String, PackageMeta> = HashMap::new();
    let mut out: Vec<LibraryMatch> = Vec::new();
    for (filename, package) in matches_path {
      if !Self::is_library_shape(&filename) {
        continue;
      }
      let meta = Self::package_meta(index, &package, &mut cache_meta)?;
      out.push(LibraryMatch {
        package,
        library: filename,
        version: meta.version,
        description: meta.description,
      });
    }
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

  /// Keeps only names shaped like a shared object or static archive,
  /// so a wildcard's command names and other provides entries do not
  /// leak into the answer.
  fn is_library_shape(name: &str) -> bool {
    let stripped = Self::canonicalize(name);
    stripped.ends_with(".so") || stripped.contains(".so.") || stripped.ends_with(".a")
  }

  /// Strips each distribution's decorations — `so:` prefix, `()(64bit)`
  /// marker, `=<version>` suffix — to the bare library name, so every
  /// encoding of one library reads identically.
  fn canonicalize(name: &str) -> &str {
    let s = name.strip_prefix("so:").unwrap_or(name);
    let s = s.strip_suffix("()(64bit)").unwrap_or(s);
    let s = s.strip_suffix("()(32bit)").unwrap_or(s);
    match s.find('=') {
      Some(eq) => &s[..eq],
      None => s,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::package::{DepSpec, Package};
  use crate::path_index::PathIndexBuilder;
  use tempfile::TempDir;

  fn pkg(name: &str, version: &str, description: &str) -> Package {
    Package {
      description: description.into(),
      ..crate::package::fixtures::pkg(name, version)
    }
  }

  /// An on-disk index whose companion path index holds the given
  /// file-ownership rows, so the file side of the search has real data
  /// to consult.
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

  // ── shape_check / strip_decorations ─────────────────────────────────

  #[test]
  fn shape_check_matches_shared_libraries() {
    assert!(LibraryMatch::is_library_shape("libssl.so.3"));
    assert!(LibraryMatch::is_library_shape("libfoo.so"));
    assert!(LibraryMatch::is_library_shape("libfoo.so.1.2.3"));
  }

  #[test]
  fn shape_check_matches_static_archives() {
    assert!(LibraryMatch::is_library_shape("libfoo.a"));
  }

  #[test]
  fn shape_check_rejects_non_library_shapes() {
    assert!(!LibraryMatch::is_library_shape("libssl3"));
    assert!(!LibraryMatch::is_library_shape("bash"));
    assert!(!LibraryMatch::is_library_shape("python3.11"));
    assert!(!LibraryMatch::is_library_shape("awk"));
  }

  #[test]
  fn shape_check_handles_alpine_so_prefix() {
    assert!(LibraryMatch::is_library_shape("so:libc.musl-x86_64.so.1"));
    assert!(!LibraryMatch::is_library_shape("so:bash"));
  }

  #[test]
  fn shape_check_handles_rpm_arch_suffix() {
    assert!(LibraryMatch::is_library_shape("libssl.so.3()(64bit)"));
    assert!(LibraryMatch::is_library_shape("libssl.so.3()(32bit)"));
    assert!(!LibraryMatch::is_library_shape("foo()(64bit)"));
  }

  #[test]
  fn shape_check_handles_arch_provides_version() {
    assert!(LibraryMatch::is_library_shape("libgomp.so=1-64"));
    assert!(LibraryMatch::is_library_shape("libfoo.so.1=2.3.4-5"));
    assert!(!LibraryMatch::is_library_shape("foo=1.0"));
  }

  #[test]
  fn strip_decorations_canonicalises_known_forms() {
    assert_eq!(LibraryMatch::canonicalize("libssl.so.3"), "libssl.so.3");
    assert_eq!(LibraryMatch::canonicalize("so:libc.musl-x86_64.so.1"), "libc.musl-x86_64.so.1");
    assert_eq!(LibraryMatch::canonicalize("libssl.so.3()(64bit)"), "libssl.so.3");
    assert_eq!(LibraryMatch::canonicalize("libssl.so.3()(32bit)"), "libssl.so.3");
    assert_eq!(LibraryMatch::canonicalize("libgomp.so=1-64"), "libgomp.so");
  }

  // ── glob: provides side ──────────────────────────────────────────────

  #[test]
  fn glob_matches_debian_path_index_only() {
    // Debian: provides is the package name (`libssl3`), not a soname.
    // Path index has `libssl.so.3`. Result comes from the file side
    // only.
    let mut libssl = pkg("libssl3", "3.0.11-1", "OpenSSL");
    libssl.provides = vec![DepSpec {
      name: "libssl3".to_string(),
      version_constraint: None,
    }];
    let (_tmp, index) = index_with_paths(vec![libssl], &[("/usr/lib/x86_64-linux-gnu", "libssl.so.3", "libssl3")]);

    let matches = LibraryMatch::glob(&index, "libssl.so.3").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].package, "libssl3");
    assert_eq!(matches[0].library, "libssl.so.3");
    assert_eq!(matches[0].version, "3.0.11-1");
    assert_eq!(matches[0].description, "OpenSSL");
  }

  #[test]
  fn glob_matches_alpine_so_prefix() {
    // Alpine: no path index, soname under `so:` namespace in provides.
    let mut musl = pkg("musl", "1.2.5-r0", "musl libc");
    musl.provides = vec![DepSpec {
      name: "so:libc.musl-x86_64.so.1".to_string(),
      version_constraint: None,
    }];
    let index = Index::in_memory(vec![musl]);

    let matches = LibraryMatch::glob(&index, "libc.musl-x86_64.so.1").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].package, "musl");
    assert_eq!(matches[0].library, "libc.musl-x86_64.so.1");
  }

  #[test]
  fn glob_matches_rpm_64bit_suffix_with_trailing_star() {
    // RPM: provides is mangled `libssl.so.3()(64bit)`. Raw SQL GLOB
    // matches it only if the user's pattern has a trailing `*`.
    let mut libssl = pkg("openssl-libs", "3.0.7-1.fc42", "OpenSSL libs");
    libssl.provides = vec![DepSpec {
      name: "libssl.so.3()(64bit)".to_string(),
      version_constraint: None,
    }];
    let index = Index::in_memory(vec![libssl]);

    let matches = LibraryMatch::glob(&index, "libssl.so.3*").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].package, "openssl-libs");
    assert_eq!(matches[0].library, "libssl.so.3"); // decorations stripped
  }

  #[test]
  fn glob_matches_arch_provides_with_version_suffix() {
    let mut glibc = pkg("glibc", "2.40-1", "GNU C library");
    glibc.provides = vec![DepSpec {
      name: "libc.so".to_string(),
      version_constraint: Some("6-64".into()),
    }];
    // Note: Arch's parser stores provides as `libc.so` with version
    // `6-64` in a separate field, not concatenated. The =-decoration
    // case happens when raw provides text contains `=`. Test it
    // directly by inserting a raw decorated form.

    let mut gcc_libs = pkg("gcc-libs", "15.2.1-1", "GCC libraries");
    gcc_libs.provides = vec![DepSpec {
      name: "libgomp.so=1-64".to_string(),
      version_constraint: None,
    }];
    let index = Index::in_memory(vec![glibc, gcc_libs]);

    let matches = LibraryMatch::glob(&index, "libgomp*").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].package, "gcc-libs");
    assert_eq!(matches[0].library, "libgomp.so"); // = stripped
  }

  // ── glob: dedup across both sides ────────────────────────────────────

  #[test]
  fn glob_dedups_when_provides_and_path_index_yield_same_pair() {
    let mut libfoo = pkg("libfoo3", "3.0", "libfoo");
    libfoo.provides = vec![DepSpec {
      name: "libfoo.so.3".to_string(),
      version_constraint: None,
    }];
    let (_tmp, index) = index_with_paths(vec![libfoo], &[("/usr/lib", "libfoo.so.3", "libfoo3")]);

    let matches = LibraryMatch::glob(&index, "libfoo.so.3").unwrap();
    assert_eq!(matches.len(), 1, "same (package, library) pair must dedup");
    assert_eq!(matches[0].package, "libfoo3");
    assert_eq!(matches[0].library, "libfoo.so.3");
  }

  #[test]
  fn glob_emits_one_entry_per_library_when_package_ships_several() {
    let mut libssl = pkg("libssl3", "3.0", "OpenSSL");
    libssl.provides = vec![
      DepSpec {
        name: "libssl.so.3".into(),
        version_constraint: None,
      },
      DepSpec {
        name: "libcrypto.so.3".into(),
        version_constraint: None,
      },
    ];
    let index = Index::in_memory(vec![libssl]);

    let matches = LibraryMatch::glob(&index, "lib*.so.3").unwrap();
    assert_eq!(matches.len(), 2);
    let libs: Vec<&str> = matches.iter().map(|m| m.library.as_str()).collect();
    assert!(libs.contains(&"libssl.so.3"));
    assert!(libs.contains(&"libcrypto.so.3"));
  }

  // ── glob: empty match ────────────────────────────────────────────────

  #[test]
  fn glob_returns_empty_when_no_match() {
    let index = Index::in_memory(vec![pkg("bash", "5.2", "")]);

    let matches = LibraryMatch::glob(&index, "libnothere*").unwrap();
    assert!(matches.is_empty());
  }

  #[test]
  fn glob_filters_out_non_library_shape_provides() {
    // `awk` is a virtual but not a library; even with a glob match,
    // shape_check rejects it.
    let mut mawk = pkg("mawk", "1.3.4", "AWK");
    mawk.provides = vec![DepSpec {
      name: "awk".to_string(),
      version_constraint: None,
    }];
    let index = Index::in_memory(vec![mawk]);

    let matches = LibraryMatch::glob(&index, "awk").unwrap();
    assert!(matches.is_empty(), "awk is virtual, not library-shape");
  }
}
