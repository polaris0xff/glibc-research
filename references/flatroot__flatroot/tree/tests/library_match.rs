//! Integration tests for the library glob matcher (`flatroot::library`) and the
//! SQLite index cache lifecycle (`flatroot::db::Index`).
//!
//! These build a real on-disk catalogue with `Index::open_or_populate` (the only
//! public path that produces a populated `Index`, since `IndexWriter` cannot be
//! constructed from outside the crate) using a populate closure that inserts a
//! known package set with NO network. The closure also feeds the writer the
//! file-ownership facts whose companion record `LibraryMatch::glob` consults.
//!
//! `open_or_populate` reads `FLATROOT_CACHE_HOME` from the *process* environment
//! via `Cache::dir_resolve()`, so every test here mutates that env var and is
//! therefore `#[serial]`: tests in one integration binary share a process, and
//! concurrent env mutation would race. Each test points the cache at its own
//! fresh `TempDir`.

#![allow(dead_code)]

use std::sync::Arc;

use flatroot::db::{Index, IndexWriter};
use flatroot::library::LibraryMatch;
use flatroot::package::{DepSpec, Package};
use flatroot::version::{DpkgVersionCompare, RpmVersionCompare, VersionCompare};
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

/// A package advertising one or more provides capabilities (sonames or virtuals).
fn pkg_provides(name: &str, version: &str, description: &str, provides: &[&str]) -> Package {
  let mut p = pkg(name, version, description);
  p.provides = provides
    .iter()
    .map(|n| DepSpec {
      name: (*n).to_string(),
      version_constraint: None,
    })
    .collect();
  p
}

/// Point `FLATROOT_CACHE_HOME` at a fresh tempdir for the duration of one test.
/// Returns the `TempDir` (kept alive by the caller) and the resolved index dir
/// path so a test can assert where the `.db` landed.
fn cache_redirect() -> (TempDir, std::path::PathBuf) {
  let tmp = TempDir::new().unwrap();
  // Safety: every test in this binary is `#[serial]`, so no other test reads or
  // writes the process env concurrently with this mutation.
  unsafe { std::env::set_var("FLATROOT_CACHE_HOME", tmp.path()) };
  let index_dir = tmp.path().join("index");
  (tmp, index_dir)
}

/// Build a populated catalogue under the current cache home from an in-memory
/// package set and an optional path-index fact set. The facts land through the
/// writer's own file-ownership half, so the companion record the production
/// code derives — and `LibraryMatch::glob` consults — is written beside the
/// catalogue. Returns the opened `Index`.
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
// LibraryMatch::glob — path-index leg → packages table inconsistency (IDX-025)
// ---------------------------------------------------------------------------

// covers: IDX-025
#[test]
#[serial]
fn library_glob_path_index_pkg_missing_from_table_is_inconsistency() {
  // The path index references package "ghostpkg", but the packages table holds
  // only "realpkg". The path-index leg's `Packages::get(ghostpkg)` returns None,
  // which library.rs turns into an explicit index-inconsistency error.
  let (_tmp, _index_dir) = cache_redirect();
  let index = build_index(
    "idx-test-inconsistency",
    Arc::new(DpkgVersionCompare),
    vec![pkg("realpkg", "1.0", "real")],
    &[("/usr/lib", "libghost.so.1", "ghostpkg")],
  );
  let err = LibraryMatch::glob(&index, "libghost.so.1")
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

// ---------------------------------------------------------------------------
// LibraryMatch::glob — dedup across legs / single canonical form (IDX-066 lib leg)
// ---------------------------------------------------------------------------

// covers: IDX-066
#[test]
#[serial]
fn library_glob_canonicalizes_each_encoding_to_one_form() {
  // The same soname `libssl.so.3` is recorded in four distro encodings across
  // four packages: a deb-style path-index file, an Alpine `so:` provides, an
  // RPM `()(64bit)` provides, and an Arch `=`-decorated provides. After
  // canonicalize, every match's `library` field is the identical bare soname.
  let (_tmp, _index_dir) = cache_redirect();
  let index = build_index(
    "idx-test-canonical",
    Arc::new(DpkgVersionCompare),
    vec![
      pkg_provides("musl-style", "1.0", "alpine", &["so:libssl.so.3"]),
      pkg_provides("rpm-style", "1.0", "rpm", &["libssl.so.3()(64bit)"]),
      pkg_provides("arch-style", "1.0", "arch", &["libssl.so.3=3-64"]),
      pkg("deb-style", "1.0", "deb"),
    ],
    // deb-style ships the file on disk (provides carries only the package name).
    &[("/usr/lib/x86_64-linux-gnu", "libssl.so.3", "deb-style")],
  );
  // The trailing `*` lets the raw SQL GLOB reach the decorated RPM/Arch text.
  let matches = LibraryMatch::glob(&index, "libssl.so.3*").unwrap();
  assert!(!matches.is_empty(), "at least the canonical libssl.so.3 must match");
  for m in &matches {
    assert_eq!(
      m.library, "libssl.so.3",
      "every encoding must reduce to the identical canonical library string, got {:?}",
      m
    );
  }
  // All four packages surface, each under the one canonical library form.
  let pkgs: std::collections::BTreeSet<&str> = matches.iter().map(|m| m.package.as_str()).collect();
  for want in ["musl-style", "rpm-style", "arch-style", "deb-style"] {
    assert!(pkgs.contains(want), "package {} missing from canonical matches: {:?}", want, matches);
  }
  // Sort contract: ordered by (library, package). With one library value, the
  // packages are alphabetical.
  let ordered: Vec<&str> = matches.iter().map(|m| m.package.as_str()).collect();
  let mut expected = ordered.clone();
  expected.sort();
  assert_eq!(ordered, expected, "matches must be sorted by (library, package)");
}

// ---------------------------------------------------------------------------
// Packages::glob — newest version per name, not last-walked row (IDX-061)
// ---------------------------------------------------------------------------

// covers: IDX-061
#[test]
#[serial]
fn packages_glob_picks_highest_version_per_name() {
  // Three releases of libc6 inserted out of version order. The glob must return
  // a single row at the highest version (ROW_NUMBER PARTITION rn=1), not the
  // last-inserted row.
  let (_tmp, _index_dir) = cache_redirect();
  let index = build_index(
    "idx-test-newest",
    Arc::new(DpkgVersionCompare),
    vec![
      pkg("libc6", "2.36-9", "GNU C Library"),
      pkg("libc6", "2.36-9+deb12u10", "GNU C Library"),
      pkg("libc6", "2.36-9+deb12u4", "GNU C Library"),
    ],
    &[],
  );
  let rows = index.packages().glob("libc6").unwrap();
  assert_eq!(rows.len(), 1, "newest-per-name collapses to a single row");
  assert_eq!(rows[0].name, "libc6");
  assert_eq!(rows[0].version, "2.36-9+deb12u10", "glob must pick the dpkg-highest version, not the last-inserted row");
}

// ---------------------------------------------------------------------------
// Providers::glob — soname attributed to newest package release (IDX-062)
// ---------------------------------------------------------------------------

// covers: IDX-062
#[test]
#[serial]
fn providers_glob_attributes_soname_to_newest_release() {
  // Two releases of one package both advertise libfoo.so.3. The provides/library
  // leg must report the soname once, attributed to the newest release via the
  // latest_pkgs CTE.
  let (_tmp, _index_dir) = cache_redirect();
  let index = build_index(
    "idx-test-provides-newest",
    Arc::new(DpkgVersionCompare),
    vec![
      pkg_provides("libfoo3", "3.0.0-1", "old", &["libfoo.so.3"]),
      pkg_provides("libfoo3", "3.0.2-1", "new", &["libfoo.so.3"]),
    ],
    &[],
  );
  let rows = index.providers().glob("libfoo.so*").unwrap();
  assert_eq!(rows.len(), 1, "soname must be reported once at the newest release");
  assert_eq!(rows[0].package_name, "libfoo3");
  assert_eq!(rows[0].provides_name, "libfoo.so.3");
  assert_eq!(
    rows[0].package_version, "3.0.2-1",
    "latest_pkgs CTE must attribute the soname to the newest package release"
  );
  assert_eq!(rows[0].package_description, "new");

  // Through the library matcher, the same single newest-attributed pair surfaces.
  let matches = LibraryMatch::glob(&index, "libfoo.so*").unwrap();
  assert_eq!(matches.len(), 1, "library matcher must also dedup to one newest pair");
  assert_eq!(matches[0].version, "3.0.2-1");
}

// ---------------------------------------------------------------------------
// Index cache: filename sanitization, fresh fast-path reuse (IDX-040, IDX-044, IDX-045)
// ---------------------------------------------------------------------------

// covers: IDX-044, IDX-045
#[test]
#[serial]
fn cache_key_slash_sanitized_and_db_lands_under_index_dir() {
  // A cache_key containing '/' is sanitized to a flat filename with '/' → '-',
  // and the .db is written under <FLATROOT_CACHE_HOME>/index/ — never escaping
  // the index dir.
  let (tmp, index_dir) = cache_redirect();
  let _index =
    build_index("debian/bookworm/x86_64", Arc::new(DpkgVersionCompare), vec![pkg("bash", "5.2", "shell")], &[]);
  // FLATROOT_CACHE_HOME redirect: the .db lives under <cache_home>/index/.
  let expected_db = index_dir.join("debian-bookworm-x86_64.db");
  assert!(
    expected_db.exists(),
    "sanitized .db must land at {} under the redirected cache home; index dir contents: {:?}",
    expected_db.display(),
    std::fs::read_dir(&index_dir).map(|d| d.flatten().map(|e| e.file_name()).collect::<Vec<_>>())
  );
  // It must not have escaped the index dir into the cache root.
  assert!(!tmp.path().join("debian-bookworm-x86_64.db").exists(), "the .db must not escape <cache_home>/index/");
}

// covers: IDX-040
#[test]
#[serial]
fn cache_fresh_fast_path_reuses_without_repopulating() {
  // First populate runs the closure. A second open_or_populate within the TTL
  // takes the fresh fast path: the closure must NOT run again, and the returned
  // index still answers from the cached .db.
  let (_tmp, _index_dir) = cache_redirect();
  let key = "idx-test-fresh-reuse";

  let _first = build_index(key, Arc::new(DpkgVersionCompare), vec![pkg("bash", "5.2", "shell")], &[]);

  let ran = Arc::new(std::sync::atomic::AtomicUsize::new(0));
  let ran_c = ran.clone();
  let second = Index::open_or_populate(key, Arc::new(DpkgVersionCompare), move |writer| {
    ran_c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    writer.insert(&pkg("should-not-appear", "1.0", "")).unwrap();
    Ok(())
  })
  .unwrap();

  assert_eq!(
    ran.load(std::sync::atomic::Ordering::SeqCst),
    0,
    "fresh fast-path must not re-run the populate closure within the TTL"
  );
  // The reused cache holds the FIRST populate's contents, not the second's.
  assert!(second.packages().exists("bash").unwrap(), "reused cache must hold bash");
  assert!(
    !second.packages().exists("should-not-appear").unwrap(),
    "the second closure must not have run / mutated the cache"
  );
}

// ---------------------------------------------------------------------------
// Index cache: TTL expiry rebuild (IDX-041)
// ---------------------------------------------------------------------------

// covers: IDX-041
#[test]
#[serial]
fn cache_rebuilds_after_ttl_expiry() {
  // Populate, then backdate the .db mtime well past the 1h TTL. The next
  // open_or_populate must take the slow path: the closure re-runs (rebuild),
  // dropping the stale .db and renaming a fresh one into place.
  let (_tmp, index_dir) = cache_redirect();
  let key = "idx-test-ttl-rebuild";

  let _first = build_index(key, Arc::new(DpkgVersionCompare), vec![pkg("oldpkg", "1.0", "")], &[]);
  let path_file_db = index_dir.join(format!("{key}.db"));
  assert!(path_file_db.exists());

  // Backdate the mtime far past the TTL (no filetime crate available; `touch`
  // is the portable way to set an old mtime).
  let status = std::process::Command::new("touch")
    .args(["-d", "2000-01-01T00:00:00", path_file_db.to_str().unwrap()])
    .status()
    .expect("touch must be available to backdate the cache mtime");
  assert!(status.success(), "failed to backdate the .db mtime");

  let ran = Arc::new(std::sync::atomic::AtomicUsize::new(0));
  let ran_c = ran.clone();
  let rebuilt = Index::open_or_populate(key, Arc::new(DpkgVersionCompare), move |writer| {
    ran_c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    writer.insert(&pkg("newpkg", "2.0", "")).unwrap();
    Ok(())
  })
  .unwrap();

  assert_eq!(
    ran.load(std::sync::atomic::Ordering::SeqCst),
    1,
    "a stale (past-TTL) cache must trigger exactly one repopulate"
  );
  // The rebuilt cache holds the NEW closure's contents, and the stale row is gone.
  assert!(rebuilt.packages().exists("newpkg").unwrap(), "rebuild must hold newpkg");
  assert!(!rebuilt.packages().exists("oldpkg").unwrap(), "the stale pre-TTL contents must be dropped on rebuild");
  // A fresh, valid .db remains (rename completed); no leftover .tmp.
  assert!(path_file_db.exists(), "a fresh .db must exist after rebuild");
  assert!(!index_dir.join(format!("{key}.db.tmp")).exists(), "no .tmp must linger after a successful rebuild");
}

// ---------------------------------------------------------------------------
// Index cache: populate failure → tmp removed, error propagated, no stale .db (IDX-043)
// ---------------------------------------------------------------------------

// covers: IDX-043
#[test]
#[serial]
fn cache_populate_failure_leaves_no_usable_cache() {
  // A populate closure that errors must: propagate the error, remove the .tmp
  // working file, and leave no final .db behind (rename never happens).
  let (_tmp, index_dir) = cache_redirect();
  let key = "idx-test-populate-fail";

  let result = Index::open_or_populate(key, Arc::new(DpkgVersionCompare), |writer| -> anyhow::Result<()> {
    writer.insert(&pkg("partial", "1.0", ""))?;
    Err(anyhow::anyhow!("simulated populate failure"))
  });
  let err = result
    .err()
    .expect("a failing populate closure must propagate its error");
  assert!(
    format!("{:#}", err).contains("simulated populate failure"),
    "the populate error must propagate, got: {:#}",
    err
  );

  // No final .db (rename never ran) and no leftover .tmp.
  assert!(!index_dir.join(format!("{key}.db")).exists(), "a failed populate must leave no usable .db");
  assert!(!index_dir.join(format!("{key}.db.tmp")).exists(), "a failed populate must remove its .tmp working file");
}

// ---------------------------------------------------------------------------
// Index cache: concurrent populate serialized by per-cache-key flock (IDX-042)
// ---------------------------------------------------------------------------

// covers: IDX-042
#[test]
#[serial]
fn cache_concurrent_populate_serialized_by_flock() {
  // Two threads race open_or_populate on the same cache key. The per-cache-key
  // flock serializes them: exactly one closure runs (the winner), the loser
  // rechecks fresh() under the lock and reuses the just-built cache.
  // cache_redirect() sets FLATROOT_CACHE_HOME once, before any thread spawns;
  // the spawned threads only *read* the env (Cache::dir_resolve), so there is no
  // concurrent env write.
  let (_tmp, _index_dir) = cache_redirect();
  let key = "idx-test-concurrent";

  let runs = Arc::new(std::sync::atomic::AtomicUsize::new(0));

  let mut handles = Vec::new();
  for _ in 0..2 {
    let runs_c = runs.clone();
    handles.push(std::thread::spawn(move || {
      Index::open_or_populate(key, Arc::new(DpkgVersionCompare), move |writer| {
        runs_c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Slow the populate so the second thread is forced to wait on the flock.
        std::thread::sleep(std::time::Duration::from_millis(300));
        writer.insert(&pkg("bash", "5.2", "shell"))?;
        Ok(())
      })
      .map(|idx| idx.packages().exists("bash").unwrap_or(false))
    }));
  }

  let mut both_have_bash = true;
  for h in handles {
    let has_bash = h
      .join()
      .expect("populate thread panicked")
      .expect("open_or_populate failed");
    both_have_bash &= has_bash;
  }

  assert_eq!(
    runs.load(std::sync::atomic::Ordering::SeqCst),
    1,
    "the per-cache-key flock must let exactly one populate closure run"
  );
  assert!(both_have_bash, "both racers must end up with the same fully-built cache (winner built it, loser reused it)");
}

// ---------------------------------------------------------------------------
// Index::connection — glob_bash function + version_compare collation registered (IDX-038, IDX-039)
// at the library level (the read connection handed to `query`)
// ---------------------------------------------------------------------------

// covers: IDX-039
#[test]
#[serial]
fn read_connection_has_version_compare_collation_rpm() {
  // The collation is built from the catalogue's own comparator. With the RPM
  // comparator, an ORDER BY ... COLLATE version_compare query must order RPM
  // versions natively (1.10 newer than 1.9), proving the collation is on the
  // connection `Index::open` returns.
  let (_tmp, _index_dir) = cache_redirect();
  let index = build_index(
    "idx-test-rpm-collation",
    Arc::new(RpmVersionCompare),
    vec![
      pkg("zlib", "1.9-1", "z"),
      pkg("zlib", "1.10-1", "z"),
      pkg("zlib", "1.2-1", "z"),
    ],
    &[],
  );
  let mut stmt = index
    .connection()
    .prepare("SELECT version FROM packages WHERE name='zlib' ORDER BY version COLLATE version_compare DESC")
    .unwrap();
  let versions: Vec<String> = stmt
    .query_map([], |r| r.get::<_, String>(0))
    .unwrap()
    .map(|r| r.unwrap())
    .collect();
  assert_eq!(
    versions,
    vec!["1.10-1".to_string(), "1.9-1".to_string(), "1.2-1".to_string()],
    "RPM version_compare collation must order 1.10 above 1.9 (not lexically)"
  );
}

// covers: IDX-038
#[test]
#[serial]
fn read_connection_has_glob_bash_scalar_function() {
  // glob_bash(target, pattern) is registered on the read connection and is
  // case-insensitive — calling it from user SQL must compile and match.
  let (_tmp, _index_dir) = cache_redirect();
  let index = build_index(
    "idx-test-globbash",
    Arc::new(DpkgVersionCompare),
    vec![
      pkg("libssl3", "3.0", "ssl"),
      pkg("libcrypto3", "3.0", "crypto"),
      pkg("bash", "5.2", "shell"),
    ],
    &[],
  );
  let mut stmt = index
    .connection()
    .prepare("SELECT name FROM packages WHERE glob_bash(name, 'LIBSSL*') ORDER BY name")
    .unwrap();
  let names: Vec<String> = stmt
    .query_map([], |r| r.get::<_, String>(0))
    .unwrap()
    .map(|r| r.unwrap())
    .collect();
  assert_eq!(
    names,
    vec!["libssl3".to_string()],
    "glob_bash must be registered and case-insensitive (LIBSSL* matches libssl3)"
  );
}

// ---------------------------------------------------------------------------
// Index::open re-opens an existing .db and answers from it (IDX-045 redirect proof)
// ---------------------------------------------------------------------------

// covers: IDX-045
#[test]
#[serial]
fn open_reads_back_db_written_under_redirected_cache_home() {
  // Build under a redirected cache home, then re-open the exact .db path with
  // Index::open and confirm it answers — proving the redirect controls the
  // store location end to end.
  let (_tmp, index_dir) = cache_redirect();
  let key = "idx-test-reopen";
  let _index = build_index(key, Arc::new(DpkgVersionCompare), vec![pkg("bash", "5.2", "shell")], &[]);

  let path_file_db = index_dir.join(format!("{key}.db"));
  assert!(path_file_db.exists(), "db must exist under the redirected index dir");

  let reopened: Index = Index::open(&path_file_db, Arc::new(DpkgVersionCompare)).unwrap();
  assert!(reopened.packages().exists("bash").unwrap(), "re-opened db must hold bash");
  // The companion file-ownership record was committed beside this same db
  // path by the populate, and the re-opened catalogue vends it from there.
  assert!(
    reopened.paths().unwrap().is_some(),
    "the companion path index must sit beside the re-opened db under the redirected cache home"
  );
}
