//! Resolver integration tests that fetch real indices from Debian repos.
//!
//! Synthetic resolver unit tests live inline in `src/resolver.rs`; these
//! tests validate the resolver against a real index fetched over the
//! network, which is why they belong in the integration layer.

use flatroot::db::Index;
use flatroot::package::DepKind;
use flatroot::remote;
use flatroot::resolver::{DepWalker, ResolutionEnv};
use flatroot::version::{DpkgVersionCompare, VersionCompare};
use serial_test::serial;

/// Open or populate a package index for the given remote.
///
/// The Debian backend's constructor performs an async HTTP HEAD probe
/// through `block_in_place` to pick a mirror, which requires a
/// multi-threaded Tokio runtime on the calling thread. The blocking
/// fetch path inside `Index::open_or_populate` uses `reqwest::blocking`
/// internally and panics if dropped from inside an async context, so
/// the runtime is built only for the construction step and dropped
/// before the populate runs.
fn remote_db(remote_str: &str) -> Index {
  let remote = {
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .expect("build tokio runtime for remote construction");
    let http = std::sync::Arc::new(flatroot::internal::http::HttpClient::new(
      flatroot::internal::cache::Cache::dir_resolve().unwrap().dir_index(),
      3,
      std::time::Duration::from_secs(3600),
    ));
    rt.block_on(async { remote::RemoteDistro::from_str(remote_str, flatroot::arch::Arch::X86_64, http).unwrap() })
  };
  let vcmp = std::sync::Arc::from(remote.version_compare());
  Index::open_or_populate(&remote.cache_key(), vcmp, |writer| remote.index_fetch(writer)).unwrap()
}

fn walk_cfg(index: &Index) -> ResolutionEnv<'_> {
  ResolutionEnv {
    index,
    include_recommends: false,
    include_suggests: false,
    exclude: &[],
  }
}

#[test]
#[serial(flatroot_cache)]
fn resolves_bash_dependencies() {
  let conn = remote_db("debian:bookworm");

  let resolved = DepWalker::resolve(&walk_cfg(&conn), &["bash".to_string()]).unwrap();

  assert!(resolved.contains(&"bash".to_string()));
  assert!(resolved.contains(&"libc6".to_string()));
  assert!(resolved.contains(&"libtinfo6".to_string()));
}

#[test]
#[serial(flatroot_cache)]
fn resolves_no_duplicates() {
  let conn = remote_db("debian:bookworm");

  let resolved = DepWalker::resolve(&walk_cfg(&conn), &["bash".to_string(), "coreutils".to_string()]).unwrap();

  let libc_count = resolved.iter().filter(|p| *p == "libc6").count();
  assert_eq!(libc_count, 1, "libc6 should appear exactly once");
}

#[test]
#[serial(flatroot_cache)]
fn resolves_virtual_packages() {
  let conn = remote_db("debian:bookworm");

  let resolved = DepWalker::resolve(&walk_cfg(&conn), &["bash".to_string()]).unwrap();

  let has_awk_provider = resolved
    .iter()
    .any(|p| p == "gawk" || p == "mawk" || p == "original-awk");
  assert!(has_awk_provider, "Expected an awk provider in: {:?}", resolved);
}

#[test]
#[serial(flatroot_cache)]
fn resolves_with_version_constraints_bookworm() {
  let conn = remote_db("debian:bookworm");

  let resolved = DepWalker::resolve(&walk_cfg(&conn), &["bash".to_string()]).unwrap();
  assert!(!resolved.is_empty());

  for pkg_name in &resolved {
    assert!(conn.packages().exists(pkg_name.as_str()).unwrap(), "Resolved package '{}' not found in index", pkg_name);
  }
}

#[test]
#[serial(flatroot_cache)]
fn version_constraints_satisfied_in_bookworm() {
  let conn = remote_db("debian:bookworm");

  let _bash = conn.packages().get("bash").unwrap().expect("bash not found");
  let libc6 = conn.packages().get("libc6").unwrap().expect("libc6 not found");

  let bash_deps = conn.dependencies().of("bash", DepKind::Depends).unwrap();
  let libc_dep = bash_deps
    .iter()
    .find(|d| d.alternatives.iter().any(|a| a.name == "libc6"));
  assert!(libc_dep.is_some(), "bash should depend on libc6");

  let libc_spec = libc_dep
    .unwrap()
    .alternatives
    .iter()
    .find(|a| a.name == "libc6")
    .unwrap();

  if let Some(ref constraint) = libc_spec.version_constraint {
    assert!(
      DpkgVersionCompare.satisfies(&libc6.version, constraint),
      "bookworm libc6 {} should satisfy bash's constraint {}",
      libc6.version,
      constraint
    );
  }
}

#[test]
#[serial(flatroot_cache)]
fn conflicts_parsed_in_real_index() {
  let conn = remote_db("debian:bookworm");

  let with_conflicts: i64 = conn
    .connection()
    .query_row("SELECT count(DISTINCT package_id) FROM dependencies WHERE kind = 'conflicts'", [], |r| r.get(0))
    .unwrap();
  let with_breaks: i64 = conn
    .connection()
    .query_row("SELECT count(DISTINCT package_id) FROM dependencies WHERE kind = 'breaks'", [], |r| r.get(0))
    .unwrap();

  assert!(with_conflicts > 0, "Expected some packages with Conflicts in bookworm");
  assert!(with_breaks > 0, "Expected some packages with Breaks in bookworm");
}
