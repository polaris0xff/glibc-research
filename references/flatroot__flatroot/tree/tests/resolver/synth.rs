//! Synthetic in-process resolver fixtures.
//!
//! `Index::in_memory` is `#[cfg(test)]` and therefore unreachable from the
//! integration-test crate, so these helpers build a real on-disk `Index`
//! the public way — `Index::open_or_populate` with a private `FLATROOT_CACHE_HOME`
//! TempDir and a unique cache key — then insert hand-authored `Package`
//! fixtures through the public `IndexWriter::insert`. The result is a fully
//! functional catalogue the public resolver API (`DepWalker::resolve`,
//! `DepTree::walk`) can be exercised against deterministically and offline,
//! exactly as the inline `src/resolver/*` unit tests do against the
//! test-only `in_memory` builder.
//!
//! Each fixture owns its cache `TempDir` for the lifetime of the `SynthIndex`
//! so the catalogue file is not garbage-collected mid-test; the index keeps
//! its `Connection` open against that file.

#![allow(dead_code)]

use std::sync::Arc;

use flatroot::db::{Index, IndexWriter};
use flatroot::package::{DepSpec, Dependency, Package, RichDep};
use flatroot::version::{DpkgVersionCompare, RpmVersionCompare, VersionCompare};
use tempfile::TempDir;

/// A live synthetic catalogue plus the cache directory that backs it.
/// Holding the `TempDir` here keeps the `.db` file alive for as long as the
/// caller keeps the `SynthIndex`, which the open `Connection` inside `index`
/// requires. The companion path-ownership record sits beside the `.db` and
/// the catalogue vends it itself.
pub struct SynthIndex {
  pub index: Index,
  _cache: TempDir,
}

/// The version-ordering family a synthetic catalogue should reason under,
/// chosen to match the distro family a scenario is framed against.
#[derive(Clone, Copy)]
pub enum Family {
  /// Debian/Ubuntu dpkg ordering.
  Dpkg,
  /// RPM-family ordering (Fedora/CentOS/openSUSE/...).
  Rpm,
}

impl Family {
  fn comparator(self) -> Arc<dyn VersionCompare> {
    match self {
      Family::Dpkg => Arc::new(DpkgVersionCompare),
      Family::Rpm => Arc::new(RpmVersionCompare),
    }
  }
}

/// Build a synthetic catalogue from the given packages under the given
/// version-ordering family. The cache is isolated to a fresh TempDir via
/// `FLATROOT_CACHE_HOME`, and the cache key is made unique per call so no
/// two fixtures ever collide on the shared index directory.
pub fn synth_index(family: Family, packages: Vec<Package>) -> SynthIndex {
  synth_index_with_paths(family, packages, &[])
}

/// As [`synth_index`], but also records a companion path-ownership index
/// containing the supplied `(absolute_path, owning_package)` facts, written
/// beside the catalogue where the resolver's file-path dependency resolution
/// finds it through the catalogue itself.
pub fn synth_index_with_paths(family: Family, packages: Vec<Package>, path_facts: &[(&str, &str)]) -> SynthIndex {
  let cache = TempDir::new().expect("tempdir for synthetic flatroot cache");
  // SAFETY: integration tests in this binary that build synthetic indices
  // are `#[serial]`, so the process-global env mutation does not race.
  unsafe {
    std::env::set_var("FLATROOT_CACHE_HOME", cache.path());
  }

  let cache_key = format!(
    "synth-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  );

  let path_facts: Vec<(String, String)> = path_facts
    .iter()
    .map(|(p, pkg)| (p.to_string(), pkg.to_string()))
    .collect();

  let index = Index::open_or_populate(&cache_key, family.comparator(), |writer: &mut IndexWriter| {
    for pkg in &packages {
      writer.insert(pkg)?;
    }
    for (full, pkg) in &path_facts {
      let (dir, filename) = match full.rfind('/') {
        Some(pos) => (&full[..pos], &full[pos + 1..]),
        None => ("", full.as_str()),
      };
      writer.path_push(dir, filename, pkg);
    }
    Ok(())
  })
  .expect("populate synthetic index");

  SynthIndex { index, _cache: cache }
}

// ---------------------------------------------------------------------------
// Package fixture constructors — mirror src/resolver/mod.rs::tests helpers.
// ---------------------------------------------------------------------------

/// A leaf package: name + version, no relationships.
pub fn pkg(name: &str, version: &str) -> Package {
  Package {
    name: name.to_string(),
    version: version.to_string(),
    depends: vec![],
    recommends: vec![],
    suggests: vec![],
    install_if: vec![],
    provides: vec![],
    conflicts: vec![],
    breaks: vec![],
    essential: false,
    priority: None,
    description: String::new(),
    filename: format!("pool/{}.deb", name),
    size: 0,
    checksum: String::new(),
    rich_deps: vec![],
  }
}

/// A package with explicit hard-dependency / conflicts / breaks lists.
pub fn make_pkg(
  name: &str,
  version: &str,
  depends: Vec<Dependency>,
  conflicts: Vec<Dependency>,
  breaks: Vec<Dependency>,
) -> Package {
  Package {
    depends,
    conflicts,
    breaks,
    ..pkg(name, version)
  }
}

/// A single unconstrained hard dependency on `name`.
pub fn dep(name: &str) -> Dependency {
  Dependency {
    alternatives: vec![DepSpec {
      name: name.to_string(),
      version_constraint: None,
    }],
  }
}

/// A single version-constrained hard dependency on `name`.
pub fn dep_versioned(name: &str, constraint: &str) -> Dependency {
  Dependency {
    alternatives: vec![DepSpec {
      name: name.to_string(),
      version_constraint: Some(constraint.to_string()),
    }],
  }
}

/// An alternatives group (`a | b | ...`), each alternative optionally
/// carrying a version constraint, in author (left-to-right) order.
pub fn dep_alternatives(alts: &[(&str, Option<&str>)]) -> Dependency {
  Dependency {
    alternatives: alts
      .iter()
      .map(|(name, constraint)| DepSpec {
        name: name.to_string(),
        version_constraint: constraint.map(|s| s.to_string()),
      })
      .collect(),
  }
}

/// A bare `provides` capability (no advertised version).
pub fn provides(name: &str) -> DepSpec {
  DepSpec {
    name: name.to_string(),
    version_constraint: None,
  }
}

/// A `provides` capability that advertises a specific version.
pub fn provides_versioned(name: &str, version: &str) -> DepSpec {
  DepSpec {
    name: name.to_string(),
    version_constraint: Some(version.to_string()),
  }
}

/// A rich-dep leaf naming one package (no version constraint).
pub fn rich_pkg(name: &str) -> RichDep {
  RichDep::Pkg(DepSpec {
    name: name.to_string(),
    version_constraint: None,
  })
}

/// A rich-dep leaf naming one package with a version constraint.
pub fn rich_pkg_versioned(name: &str, constraint: &str) -> RichDep {
  RichDep::Pkg(DepSpec {
    name: name.to_string(),
    version_constraint: Some(constraint.to_string()),
  })
}
