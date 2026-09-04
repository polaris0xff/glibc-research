//! Library-level tests for index fetching and parsing.
//!
//! Each test opens (and populates) the cached SQLite index via the
//! `Index::open_or_populate` lifecycle — which drives the backend's
//! `index_fetch` against an `IndexWriter` inside the populate
//! transaction — and verifies the resulting package rows by querying the
//! index's connection. `FLATROOT_CACHE_HOME` is pinned to a per-binary
//! tempdir so the `.db` and its sibling `.pathidx` land in isolation.

use std::sync::OnceLock;

use flatroot::db::Index;
use flatroot::remote;
use rusqlite::Connection;
use tempfile::TempDir;

struct Populated {
  index: Index,
}

/// Pin FLATROOT_CACHE_HOME to a per-binary tempdir so the fetcher cache used
/// by `index_fetch` is isolated from the user's real cache and from sibling
/// test binaries. Under nextest each test is its own process, so this fires
/// once per test; under threaded `cargo test` it fires once per binary,
/// avoiding races on `set_var`.
fn cache_isolate() {
  static CACHE: OnceLock<TempDir> = OnceLock::new();
  CACHE.get_or_init(|| {
    let dir = TempDir::new().expect("tempdir for flatroot cache");
    // Safety: `OnceLock::get_or_init` runs the initialiser at most once per
    // process; concurrent callers observe the stored value, never a racing
    // store.
    unsafe { std::env::set_var("FLATROOT_CACHE_HOME", dir.path()) };
    dir
  });
}

fn populate(remote_str: &str, arch: &str) -> Populated {
  cache_isolate();

  // `RemoteDistro::from_str` for `debian:*` performs an async HTTP HEAD probe
  // through `block_in_place`, requiring a multi-threaded Tokio runtime
  // on the calling thread. `index_fetch` on the other hand uses
  // `reqwest::blocking` internally and panics if dropped from inside
  // an async context. Run the construction step on a temporary
  // runtime, drop it before the sync fetch.
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
    let arch_parsed = flatroot::arch::Arch::from_uname(arch).expect("parse arch");
    rt.block_on(async {
      remote::RemoteDistro::from_str(remote_str, arch_parsed, http).expect("RemoteDistro::from_str")
    })
  };
  let vcmp = std::sync::Arc::from(remote.version_compare());
  let index = Index::open_or_populate(&remote.cache_key(), vcmp, |writer| remote.index_fetch(writer))
    .unwrap_or_else(|e| panic!("index_fetch({}, {}) failed: {}", remote_str, arch, e));

  Populated { index }
}

fn has_package(conn: &Connection, name: &str) -> bool {
  conn
    .query_row("SELECT COUNT(*) FROM packages WHERE name = ?1", [name], |r| r.get::<_, i64>(0))
    .unwrap_or(0)
    > 0
}

fn distinct_package_count(conn: &Connection) -> i64 {
  conn
    .query_row("SELECT COUNT(DISTINCT name) FROM packages", [], |r| r.get(0))
    .unwrap()
}

fn first_package_fields(conn: &Connection, name: &str) -> (String, String, String) {
  conn
    .query_row("SELECT filename, version, checksum FROM packages WHERE name = ?1 LIMIT 1", [name], |r| {
      Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    })
    .unwrap_or_else(|e| panic!("no row for package '{}': {}", name, e))
}

/// Borrow the raw read connection from a populated index for the
/// direct-SQL assertions below.
fn conn(p: &Populated) -> &Connection {
  p.index.connection()
}

#[test]
fn debian_bookworm_index_has_bash() {
  let p = populate("debian:bookworm", "x86_64");

  assert!(has_package(conn(&p), "bash"));
  assert!(has_package(conn(&p), "libc6"));
  assert!(has_package(conn(&p), "coreutils"));

  let (filename, version, checksum) = first_package_fields(conn(&p), "bash");
  assert!(!filename.is_empty(), "bash has empty filename");
  assert!(!version.is_empty(), "bash has empty version");
  assert!(!checksum.is_empty(), "bash has empty checksum");
}

#[test]
fn debian_buster_index_has_bash() {
  let p = populate("debian:buster", "x86_64");
  assert!(has_package(conn(&p), "bash"));
  assert!(has_package(conn(&p), "libc6"));
}

#[test]
fn debian_bookworm_index_package_count() {
  let p = populate("debian:bookworm", "x86_64");
  let count = distinct_package_count(conn(&p));
  assert!(count > 50_000, "Expected >50000 packages, got {}", count);
}

#[test]
fn ubuntu_noble_index_has_bash() {
  let p = populate("ubuntu:noble", "x86_64");
  assert!(has_package(conn(&p), "bash"));
  assert!(has_package(conn(&p), "libc6"));
  let count = distinct_package_count(conn(&p));
  assert!(count > 50_000, "Expected >50000 packages, got {}", count);
}

#[test]
fn arch_rolling_index_has_bash() {
  let p = populate("arch:rolling", "x86_64");
  assert!(has_package(conn(&p), "bash"));
  assert!(has_package(conn(&p), "glibc"));
  let count = distinct_package_count(conn(&p));
  assert!(count > 10_000, "Expected >10000 packages, got {}", count);
}

#[test]
fn alpine_edge_index_has_busybox() {
  let p = populate("alpine:edge", "x86_64");
  assert!(has_package(conn(&p), "busybox"));
  assert!(has_package(conn(&p), "musl"));
  let count = distinct_package_count(conn(&p));
  assert!(count > 20_000, "Expected >20000 packages, got {}", count);
}

#[test]
fn centos7_index_has_bash() {
  let p = populate("centos:7", "x86_64");
  assert!(has_package(conn(&p), "bash"));
  assert!(has_package(conn(&p), "glibc"));
  let count = distinct_package_count(conn(&p));
  assert!(count > 5_000, "Expected >5000 packages, got {}", count);
}
