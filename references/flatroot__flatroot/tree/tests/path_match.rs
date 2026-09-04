//! Integration tests for the path glob matcher (`flatroot::path`).
//!
//! These build a real on-disk catalogue with `Index::open_or_populate` (the only
//! public path that produces a populated `Index`, since `IndexWriter` cannot be
//! constructed from outside the crate) using a populate closure that inserts a
//! known package set with NO network. The closure also feeds the writer the
//! file-ownership facts whose companion record `PathMatch::glob` consults.
//!
//! `open_or_populate` reads `FLATROOT_CACHE_HOME` from the *process* environment
//! via `Cache::dir_resolve()`, so every test here mutates that env var and is
//! therefore `#[serial]`: tests in one integration binary share a process, and
//! concurrent env mutation would race. Each test points the cache at its own
//! fresh `TempDir`.

#![allow(dead_code)]

use std::sync::Arc;

use flatroot::db::{Index, IndexWriter};
use flatroot::package::Package;
use flatroot::path::PathMatch;
use flatroot::version::{DpkgVersionCompare, VersionCompare};
use serial_test::serial;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A bare package with all the lists empty; tests override only the fields they
/// care about via struct-update syntax.
fn pkg(name: &str, version: &str, description: &str) -> Package {
  Package {
    name: name.into(),
    version: version.into(),
    depends: vec![],
    recommends: vec![],
    suggests: vec![],
    install_if: vec![],
    provides: vec![],
    conflicts: vec![],
    breaks: vec![],
    essential: false,
    priority: None,
    description: description.into(),
    filename: format!("pool/{}.deb", name),
    size: 0,
    checksum: String::new(),
    rich_deps: vec![],
  }
}

/// Point `FLATROOT_CACHE_HOME` at a fresh tempdir for the duration of one test.
/// Returns the `TempDir` (kept alive by the caller).
fn cache_redirect() -> TempDir {
  let tmp = TempDir::new().unwrap();
  // Safety: every test in this binary is `#[serial]`, so no other test reads or
  // writes the process env concurrently with this mutation.
  unsafe { std::env::set_var("FLATROOT_CACHE_HOME", tmp.path()) };
  tmp
}

/// Build a populated catalogue under the current cache home from an in-memory
/// package set and a path-index fact set. The facts land through the writer's
/// own file-ownership half, so the companion record the production code
/// derives — and `PathMatch::glob` consults — is written beside the catalogue.
/// Returns the opened `Index`.
fn build_index(
  cache_key: &str,
  vcmp: Arc<dyn VersionCompare>,
  packages: Vec<Package>,
  path_facts: &[(&str, &str, &str)],
) -> Index {
  let path_facts: Vec<(String, String, String)> = path_facts
    .iter()
    .map(|(d, f, p)| (d.to_string(), f.to_string(), p.to_string()))
    .collect();
  Index::open_or_populate(cache_key, vcmp, move |writer: &mut IndexWriter| {
    for p in &packages {
      writer.insert(p)?;
    }
    for (d, f, p) in &path_facts {
      writer.path_push(d, f, p);
    }
    Ok(())
  })
  .unwrap()
}

// ---------------------------------------------------------------------------
// PathMatch::glob — full-path matching over a populated catalogue
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn path_glob_resolves_full_path_to_owning_package() {
  let _tmp = cache_redirect();
  let index = build_index(
    "idx-test-path-full-path",
    Arc::new(DpkgVersionCompare),
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
#[serial]
fn path_glob_distinguishes_same_filename_by_directory() {
  // The whole point of the path match space over the library one: the same
  // basename in two locations resolves to two different owners, and a
  // full-path pattern picks exactly one.
  let _tmp = cache_redirect();
  let index = build_index(
    "idx-test-path-disambiguate",
    Arc::new(DpkgVersionCompare),
    vec![pkg("foo-cli", "1.0", "cli"), pkg("foo-lib", "1.0", "lib")],
    &[("/usr/bin", "foo", "foo-cli"), ("/usr/lib", "foo", "foo-lib")],
  );

  let matches = PathMatch::glob(&index, "usr/bin/foo").unwrap();
  assert_eq!(matches.len(), 1);
  assert_eq!(matches[0].package, "foo-cli");

  let matches = PathMatch::glob(&index, "usr/*/foo").unwrap();
  let owners: Vec<&str> = matches.iter().map(|m| m.package.as_str()).collect();
  assert_eq!(owners, vec!["foo-cli", "foo-lib"]);
}

#[test]
#[serial]
fn path_glob_matches_non_library_files() {
  // Configuration files are invisible to `--type library` by design; the
  // path match space must reach them.
  let _tmp = cache_redirect();
  let index = build_index(
    "idx-test-path-non-library",
    Arc::new(DpkgVersionCompare),
    vec![pkg("openssl", "3.0.18-1", "TLS toolkit")],
    &[("/etc/ssl", "openssl.cnf", "openssl")],
  );

  let matches = PathMatch::glob(&index, "etc/ssl/*").unwrap();
  assert_eq!(matches.len(), 1);
  assert_eq!(matches[0].path, "etc/ssl/openssl.cnf");
  assert_eq!(matches[0].package, "openssl");
}

// ---------------------------------------------------------------------------
// PathMatch::glob — path-index → packages table inconsistency
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn path_glob_path_index_pkg_missing_from_table_is_inconsistency() {
  // The path index references package "ghostpkg", but the packages table holds
  // only "realpkg". The metadata join's `Packages::get(ghostpkg)` returns None,
  // which path.rs turns into an explicit index-inconsistency error.
  let _tmp = cache_redirect();
  let index = build_index(
    "idx-test-path-inconsistency",
    Arc::new(DpkgVersionCompare),
    vec![pkg("realpkg", "1.0", "real")],
    &[("/usr/bin", "ghost", "ghostpkg")],
  );

  let err = PathMatch::glob(&index, "usr/bin/ghost")
    .err()
    .expect("expected an error");
  let msg = format!("{:#}", err);
  assert!(
    msg.contains("not found in packages table — index inconsistency"),
    "expected the index-inconsistency error, got: {}",
    msg
  );
  assert!(msg.contains("ghostpkg"), "error must name the missing package: {}", msg);
}
