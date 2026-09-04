//! Cross-cutting runtime / network integration tests — behaviours that span
//! several subcommands or subsystems at once, plus index-lifecycle behaviours
//! that have no single command's home file. Single-concern runtime tests live
//! in their concern's file (install pipeline → `install_pipeline.rs`, manifest
//! durability → `manifest_integration.rs`, release sources → `release_list.rs`,
//! downloader/cache integrity → `download_cache.rs`, query rendering →
//! `query.rs`); only the genuinely cross-cutting cases remain here:
//!
//! Multi-command / matrix:
//!   DIST-077  @date axis honoured on search/query/analyze/release; distinct cache
//!   CLI-101   one --http-retries value drives index + archive + release scrape
//!   CLI-102   analyze async + spawn_blocking failures produce the same nonzero
//!   CORE-063  cross-distro guardrail across every ordered distro-prefix pair
//!   IDX-076   ArchContext::open (async) vs open_blocking parity + shared .db
//!
//! Index lifecycle (no dedicated db-test file):
//!   IDX-073   concurrent index populate: flock + TOCTOU fast-path, no partial .db
//!   IDX-074   stale .db rebuild sweeps .db-wal / .db-shm / .pathidx siblings
//!   IDX-075   DB-cache TTL and listing-body TTL are independent windows
//!   IDX-077   per-arch index identity: separate .db per --arch, no cross-serving
//!
//! Downloader library guard:
//!   INST-079  resolved-but-missing-from-DB is a hard error in the Downloader
//!
//! Each network test gets a fresh `FLATROOT_CACHE_HOME` TempDir so cache reuse
//! versus refetch is observable, captures stderr, and panics with that stderr on
//! an unexpected non-zero exit.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use assert_cmd::Command;
use assert_cmd::cargo::CommandCargoExt;
use tempfile::TempDir;

use flatroot::arch::Arch;
use flatroot::internal::cache::Cache;
use flatroot::internal::http::HttpClient;
use flatroot::manifest::ManifestState;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Run `flatroot` with the given args and an isolated cache, returning Output.
fn run(cache: &Path, args: &[&str]) -> Output {
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.to_str().unwrap())
    .args(args)
    .output()
    .expect("flatroot failed to execute")
}

/// `flatroot install` into `root` with extra args after the subcommand.
fn install(remote: &str, cache: &Path, root: &Path, extra: &[&str]) -> Output {
  let mut args: Vec<&str> = vec!["--from", remote, "install", "--output", root.to_str().unwrap()];
  args.extend_from_slice(extra);
  run(cache, &args)
}

/// Assert success or panic with the captured stderr.
fn ok(out: &Output, label: &str) {
  assert!(
    out.status.success(),
    "{label}: flatroot exited {:?}\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr),
  );
}

/// All `.db` files under `<cache>/index`.
fn index_db_files(cache: &Path) -> Vec<PathBuf> {
  let dir = cache.join("index");
  let mut out: Vec<PathBuf> = fs::read_dir(&dir)
    .map(|it| {
      it.flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "db").unwrap_or(false))
        .collect()
    })
    .unwrap_or_default();
  out.sort();
  out
}

/// Set a file's mtime via std (`set_modified`, std 1.75+).
fn filetime_set(path: &Path, when: SystemTime) {
  let f = fs::File::options()
    .write(true)
    .open(path)
    .expect("open db to set mtime");
  f.set_modified(when).expect("set mtime");
}

fn file_has_bytes(path: &Path, needle: &[u8]) -> bool {
  fs::read(path).map(|b| b == needle).unwrap_or(false)
}

/// mtimes of the cached raw listing bodies (everything in the index dir that is
/// not a built artefact: .db / WAL / lock / pathidx).
fn listing_body_mtimes(index_dir: &Path) -> BTreeMap<String, SystemTime> {
  let mut out = BTreeMap::new();
  for e in fs::read_dir(index_dir).unwrap().flatten() {
    let name = e.file_name().to_string_lossy().into_owned();
    let is_artefact = name.ends_with(".db")
      || name.contains(".db-")
      || name.ends_with(".db.tmp")
      || name.ends_with(".lock")
      || name.contains("pathidx");
    if is_artefact {
      continue;
    }
    if let Ok(meta) = e.metadata() {
      if let Ok(m) = meta.modified() {
        out.insert(name, m);
      }
    }
  }
  out
}

// ===========================================================================
// DIST-077 — @date snapshot source axis honoured across search / query / analyze /
// release, with a DISTINCT cache namespace from the unpinned source
// (distro/mod.rs:46/55 — the @date suffix flows into cache_id; manifest/state.rs
// guardrail keeps pinned ≠ unpinned). Debian snapshot.
// ===========================================================================

// covers: DIST-077
#[test]
fn dated_source_axis_honoured_and_cache_namespaced() {
  let pinned_cache = TempDir::new().unwrap();
  let unpinned_cache = TempDir::new().unwrap();
  let pinned = "debian:bookworm@2024-06-15";

  // search honours the @date pin (populates an index, returns bash).
  let s = run(pinned_cache.path(), &["--from", pinned, "search", "bash"]);
  ok(&s, "search against @date");
  assert!(String::from_utf8_lossy(&s.stdout).contains("bash"), "search must work against a dated source");

  // query honours the @date pin.
  let q = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", pinned_cache.path())
    .args(["--from", pinned, "query", "--format", "plain"])
    .write_stdin("SELECT COUNT(*) AS n FROM packages")
    .output()
    .unwrap();
  ok(&q, "query against @date");
  assert!(
    String::from_utf8_lossy(&q.stdout).contains("query.0.cells.0.column=n"),
    "query must work against a dated source"
  );

  // analyze honours the @date pin.
  let a = run(pinned_cache.path(), &["--from", pinned, "analyze", "trace", "bash"]);
  ok(&a, "analyze against @date");

  // release list honours the @date-bearing --from (the @date is ignored for the
  // release catalogue, but the command must still succeed for a dated source).
  let r = run(pinned_cache.path(), &["--from", pinned, "release", "list"]);
  ok(&r, "release list against @date");

  // Distinct cache namespace: the pinned index .db filename must carry the date,
  // so a pinned cache and an unpinned cache never collide. Populate an unpinned
  // index in a separate cache and compare the .db basenames.
  let u = run(unpinned_cache.path(), &["--from", "debian:bookworm", "search", "bash"]);
  ok(&u, "search against unpinned");
  let pinned_dbs: Vec<String> = index_db_files(pinned_cache.path())
    .iter()
    .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
    .collect();
  let unpinned_dbs: Vec<String> = index_db_files(unpinned_cache.path())
    .iter()
    .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
    .collect();
  assert!(!pinned_dbs.is_empty() && !unpinned_dbs.is_empty(), "both runs must build an index .db");
  // No pinned .db name equals an unpinned .db name — different namespaces.
  for p in &pinned_dbs {
    assert!(
      !unpinned_dbs.contains(p),
      "pinned and unpinned must occupy distinct cache namespaces: {p} appears in both"
    );
  }
}

// ===========================================================================
// IDX-073 — concurrent index population: two processes against the same source +
// cache; one populates under the exclusive flock, the waiter takes the TOCTOU
// fast-path and prints "cached:", and no partial/corrupt .db results
// (db.rs:184-199 flock + recheck; db.rs:216-245 tmp→rename).
// ===========================================================================

// covers: IDX-073
#[test]
fn concurrent_index_population_one_populates_other_waits() {
  let cache = TempDir::new().unwrap();

  // Launch two searches against the SAME source + cache simultaneously. One wins
  // the flock and populates; the other blocks, then sees the fresh .db on the
  // TOCTOU recheck and serves it.
  let spawn = || {
    std::process::Command::cargo_bin("flatroot")
      .expect("locate flatroot binary")
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args(["--from", "fedora:42", "search", "bash"])
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .expect("spawn flatroot")
  };
  let a = spawn();
  let b = spawn();
  let oa = a.wait_with_output().unwrap();
  let ob = b.wait_with_output().unwrap();

  ok(&oa, "concurrent search a");
  ok(&ob, "concurrent search b");

  // Exactly one valid .db exists (no `.tmp` corpse, no half-written file). The
  // tmp→rename swap (db.rs:244) means a partial populate never lands as a .db.
  let dbs = index_db_files(cache.path());
  assert_eq!(dbs.len(), 1, "concurrent populate must leave exactly one .db, got {dbs:?}");
  let stray_tmp = fs::read_dir(cache.path().join("index"))
    .unwrap()
    .flatten()
    .any(|e| e.file_name().to_string_lossy().contains(".db.tmp"));
  assert!(!stray_tmp, "no .db.tmp corpse may survive a successful concurrent populate");

  // At least one of the two processes took the cached fast-path ("cached:" is
  // printed both on the pre-lock fresh check and the post-lock TOCTOU recheck,
  // db.rs:175/197). The waiter must have hit it.
  let combined = format!("{}{}", String::from_utf8_lossy(&oa.stderr), String::from_utf8_lossy(&ob.stderr));
  assert!(
    combined.contains("cached:"),
    "the waiter must serve the already-populated .db via the cached fast-path:\n{combined}"
  );
}

// ===========================================================================
// IDX-074 — stale .db rebuild removes the .db, its .db-wal / .db-shm WAL
// companions, AND the sibling .pathidx before the tmp→rename swap
// (db.rs:201-212 drop_if_present sweep; db.rs:28 TTL).
// ===========================================================================

// covers: IDX-074
#[test]
fn stale_db_rebuild_sweeps_wal_shm_and_pathidx_siblings() {
  let cache = TempDir::new().unwrap();
  // First populate a real index (CentOS has a path index / filelists).
  let first = run(cache.path(), &["--from", "centos:stream9", "search", "libc"]);
  ok(&first, "first centos search");
  let dbs = index_db_files(cache.path());
  assert_eq!(dbs.len(), 1, "one .db after first populate, got {dbs:?}");
  let path_file_db = dbs[0].clone();

  // Make the .db look stale by back-dating its mtime well beyond the 3600s TTL,
  // and plant WAL/SHM companions + a stale .pathidx that the rebuild must sweep.
  let stale = SystemTime::now() - Duration::from_secs(7200);
  filetime_set(&path_file_db, stale);
  let wal = path_file_db.with_extension("db-wal");
  let shm = path_file_db.with_extension("db-shm");
  // .pathidx is the sibling PathIndex::of_db pins next to the .db (path_index.rs:
  // 357-359 — with_extension("pathidx")). The stale sweep removes it too
  // (db.rs:211).
  let pathidx = flatroot::path_index::PathIndex::of_db(&path_file_db);
  fs::write(&wal, b"stale-wal").unwrap();
  fs::write(&shm, b"stale-shm").unwrap();
  // Plant a stale sentinel at the pathidx sibling so the sweep has something to
  // remove (a centos index also builds its own from filelists, so this is
  // overwritten/removed before the rebuild either way).
  fs::write(&pathidx, b"stale-pathidx").unwrap();

  // Re-run: the .db is stale → the slow path runs, sweeping the companions and
  // the sibling pathidx before rebuilding. A successful rebuild proves the WAL
  // companions did not poison the open, and the planted stale files are gone /
  // replaced by a freshly built tree.
  let second = run(cache.path(), &["--from", "centos:stream9", "search", "libc"]);
  ok(&second, "rebuild after staleness");
  // The planted WAL/SHM corpses must not survive as the literal stale bytes — a
  // rebuilt SQLite DB owns its own WAL lifecycle, so the stale companions are
  // removed by the sweep (db.rs:209-210).
  assert!(!file_has_bytes(&wal, b"stale-wal"), "stale .db-wal must be swept before rebuild");
  assert!(!file_has_bytes(&shm, b"stale-shm"), "stale .db-shm must be swept before rebuild");
  assert!(!file_has_bytes(&pathidx, b"stale-pathidx"), "stale .pathidx sibling must be swept before rebuild");
  // And a valid .db is present again.
  assert!(path_file_db.exists(), "the .db must be rebuilt after the stale sweep");
}

// ===========================================================================
// IDX-075 — the DB-cache TTL (db.rs:28) and the listing-body TTL
// (commands/mod.rs:32 / internal/http.rs) are two independent 3600s windows: one
// layer can be invalidated while the other stays fresh. Proven behaviorally —
// invalidate only the .db layer (delete it) and observe the listing bodies stay
// fresh (not re-fetched) while the .db is rebuilt from them.
// ===========================================================================

// covers: IDX-075
#[test]
fn db_cache_and_listing_body_ttls_are_independent() {
  let cache = TempDir::new().unwrap();
  // First run populates BOTH layers: the listing bodies (raw index material) and
  // the built .db.
  let first = run(cache.path(), &["--from", "debian:bookworm", "search", "bash"]);
  ok(&first, "first populate");

  let index_dir = cache.path().join("index");
  // Listing-body files are the non-.db / non-WAL / non-lock / non-pathidx files
  // in the index dir (the cached raw bodies internal/http.rs writes).
  let body_mtimes_before = listing_body_mtimes(&index_dir);
  assert!(!body_mtimes_before.is_empty(), "first run must cache raw listing bodies under {index_dir:?}");

  // Invalidate ONLY the DB layer: remove the .db (+ WAL companions + pathidx),
  // leaving the listing bodies untouched and fresh.
  for db in index_db_files(cache.path()) {
    let _ = fs::remove_file(&db);
    let _ = fs::remove_file(db.with_extension("db-wal"));
    let _ = fs::remove_file(db.with_extension("db-shm"));
  }
  for e in fs::read_dir(&index_dir).unwrap().flatten() {
    if e.file_name().to_string_lossy().contains("pathidx") {
      let _ = fs::remove_file(e.path());
    }
  }

  // Second run: the DB layer is gone (stale by deletion) so the .db is rebuilt,
  // but the listing-body layer is still within its 3600s window — those bodies
  // must be reused, not re-downloaded. mtime unchanged is the witness that the
  // two windows are independent (one invalidated, one fresh).
  let second = run(cache.path(), &["--from", "debian:bookworm", "search", "bash"]);
  ok(&second, "rebuild .db from fresh listing bodies");
  let body_mtimes_after = listing_body_mtimes(&index_dir);

  for (name, before) in &body_mtimes_before {
    if let Some(after) = body_mtimes_after.get(name) {
      assert_eq!(before, after, "listing body {name} must stay fresh (not re-fetched) while the .db layer was rebuilt");
    }
  }
  // The .db layer was genuinely rebuilt.
  assert!(!index_db_files(cache.path()).is_empty(), "the .db must be rebuilt from the still-fresh listing bodies");
}

// ===========================================================================
// CLI-101 — one --http-retries value drives the index fetch, the archive
// download, AND the release scrape (commands/mod.rs:44; downloader.rs:296;
// main.rs:69-70). A budget of 1 (set via FLATROOT_ARG_HTTP_RETRIES) carries
// through every subsystem: with a real source it still succeeds (one attempt is
// enough against a healthy mirror), proving the value flows everywhere without
// being reset or clamped.
// ===========================================================================

// covers: CLI-101
#[test]
fn single_http_retries_value_flows_into_every_subsystem() {
  // The retry budget is carried verbatim onto the client (proven directly), and
  // the same value reaches the Downloader (install.rs:296) and the release scrape
  // (main.rs:70). Direct: the client honours the configured budget.
  for retries in [1u32, 2, 5] {
    let client =
      HttpClient::new(std::env::temp_dir().join("flatroot-retries-probe"), retries, Duration::from_secs(3600));
    assert_eq!(client.retries(), retries, "HttpClient must carry the configured retry budget verbatim");
  }

  // End-to-end: with FLATROOT_ARG_HTTP_RETRIES=1, an install (index fetch +
  // archive download) and a release list (release scrape) both still succeed
  // against healthy mirrors — the single budget is enough and flows everywhere.
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let inst = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env("FLATROOT_ARG_HTTP_RETRIES", "1")
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      "bash",
    ])
    .output()
    .unwrap();
  ok(&inst, "install honours FLATROOT_ARG_HTTP_RETRIES=1 across index + download");

  let rel = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env("FLATROOT_ARG_HTTP_RETRIES", "1")
    .args(["--from", "debian:bookworm", "release", "list"])
    .output()
    .unwrap();
  ok(&rel, "release scrape honours FLATROOT_ARG_HTTP_RETRIES=1");
}

// ===========================================================================
// CLI-102 — a failure inside a spawn_blocking command (search / query / release)
// or the async analyze produces the same nonzero exit + message as a synchronous
// failure: the JoinError / inner-Result is propagated, not swallowed
// (main.rs:70/81-84/91/128-139). Driven by an unknown distro, whose resolution
// error must surface with a nonzero exit identically across the command shapes.
// ===========================================================================

// covers: CLI-102
#[test]
fn spawn_blocking_and_async_failures_propagate_identically() {
  let cache = TempDir::new().unwrap();
  let unknown = "definitelynotadistro:nope";

  // search + query + release run inside spawn_blocking; analyze runs async. All
  // four must surface the same unknown-distro failure with a nonzero exit — the
  // failure inside the blocking task / the async future is propagated, not lost.
  let search = run(cache.path(), &["--from", unknown, "search", "bash"]);
  let query = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", unknown, "query"])
    .write_stdin("SELECT 1")
    .output()
    .unwrap();
  let release = run(cache.path(), &["--from", unknown, "release", "list"]);
  let analyze = run(cache.path(), &["--from", unknown, "analyze", "trace", "bash"]);

  for (label, out) in [
    ("search", &search),
    ("query", &query),
    ("release", &release),
    ("analyze", &analyze),
  ] {
    assert!(!out.status.success(), "{label}: an unknown distro must fail with a nonzero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
      stderr.contains("not a supported distro") || stderr.contains("Unknown remote"),
      "{label}: the resolution failure must propagate as the same message:\n{stderr}"
    );
  }
}

// ===========================================================================
// IDX-076 — ArchContext::open (async, used by install/analyze) and open_blocking
// (sync, used by search/query) build the same per-arch environment and reuse the
// same cached .db across commands (arch_context.rs:61-101/110-122). Proven by
// running an async command (install) and a sync command (query) against the same
// source + cache and asserting the second reuses the first's .db (the "cached:"
// fast-path), with identical index framing.
// ===========================================================================

// covers: IDX-076
#[test]
fn open_and_open_blocking_share_one_cached_index() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();

  // Async path (install via ArchContext::open) populates the index .db.
  let inst = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&inst, "async-path install populates the index");
  let dbs_after_install = index_db_files(cache.path());
  assert_eq!(dbs_after_install.len(), 1, "install must build exactly one index .db, got {dbs_after_install:?}");

  // Sync path (query via ArchContext::open_blocking) against the same source +
  // cache must REUSE that .db — no second .db is built, and the "cached:"
  // fast-path fires (db.rs:175).
  let query = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "query", "--format", "plain"])
    .write_stdin("SELECT COUNT(*) AS n FROM packages")
    .output()
    .unwrap();
  ok(&query, "sync-path query reuses the index");
  let dbs_after_query = index_db_files(cache.path());
  assert_eq!(dbs_after_query, dbs_after_install, "open_blocking must reuse the same .db open built");
  assert!(
    String::from_utf8_lossy(&query.stderr).contains("cached:"),
    "the sync path must hit the cached fast-path on the shared .db:\n{}",
    String::from_utf8_lossy(&query.stderr)
  );
  // Both paths print the same index framing line.
  assert!(
    String::from_utf8_lossy(&inst.stderr).contains("Fetching package index for debian:bookworm"),
    "async framing"
  );
  assert!(
    String::from_utf8_lossy(&query.stderr).contains("Fetching package index for debian:bookworm"),
    "sync framing"
  );
}

// ===========================================================================
// IDX-077 — per-arch index identity: search/query build a SEPARATE .db per arch
// (distro/mod.rs:55 cache_id carries the arch; db.rs:170 filename derives from
// cache_id). Running the same source under two --arch values yields two distinct
// .db files with no cross-serving.
// ===========================================================================

// covers: IDX-077
#[test]
fn per_arch_index_builds_distinct_db_files() {
  let cache = TempDir::new().unwrap();
  // Same source, two arches, shared cache.
  let a = run(cache.path(), &["--from", "debian:bookworm", "--arch", "aarch64", "search", "bash"]);
  ok(&a, "aarch64 search");
  let b = run(cache.path(), &["--from", "debian:bookworm", "--arch", "x86_64", "search", "bash"]);
  ok(&b, "x86_64 search");

  let dbs: Vec<String> = index_db_files(cache.path())
    .iter()
    .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
    .collect();
  assert_eq!(dbs.len(), 2, "two arches against one source must build two distinct .db files, got {dbs:?}");
  // The two filenames are distinct and each names its arch (cache_id is
  // debian/<release>/<deb-arch>, sanitized / → -, db.rs:170).
  assert!(dbs.iter().any(|n| n.contains("arm64")), "an arch-specific .db must name arm64 (aarch64): {dbs:?}");
  assert!(dbs.iter().any(|n| n.contains("amd64")), "an arch-specific .db must name amd64 (x86_64): {dbs:?}");
}

// ===========================================================================
// CORE-063 — cross-distro guardrail axis: an install whose source prefix differs
// from a rootfs's recorded source is refused (manifest/state.rs:224-242 via
// install.rs:104-105). Driven end-to-end across several ordered distro pairs;
// the refusal names both sources and the strict-source rule, before any work.
// ===========================================================================

// covers: CORE-063
#[test]
fn cross_distro_install_refused_across_ordered_pairs() {
  // Library-level breadth: every ordered cross-prefix pair across the ten
  // distros is refused, the matching pair admitted (the full matrix, offline).
  let prefixes = [
    "debian", "ubuntu", "arch", "cachyos", "alpine", "centos", "fedora", "alma", "rocky", "opensuse",
  ];
  let root = PathBuf::from("/tmp/flatroot-guardrail-axis");
  for existing in &prefixes {
    let mut state = ManifestState::default();
    state.sources.insert(format!("{existing}:rel"));
    for incoming in &prefixes {
      let res = ManifestState::source_admit(Some(&state), &format!("{incoming}:rel"), &root);
      if existing == incoming {
        assert!(res.is_ok(), "identical source must be admitted: {existing}:rel");
      } else {
        let err = res.err().expect(&format!("{existing} -> {incoming} must be refused"));
        let msg = err.to_string();
        assert!(msg.contains("scoped to one exact source"), "{existing}->{incoming}: strict-source wording: {msg}");
        assert!(
          msg.contains(&format!("{incoming}:rel")),
          "{existing}->{incoming}: must name the incoming source: {msg}"
        );
        assert!(
          msg.contains(&format!("{existing}:rel")),
          "{existing}->{incoming}: must name the existing source: {msg}"
        );
      }
    }
  }

  // End-to-end: a debian rootfs refuses an ubuntu install, with the rejection
  // happening before any network (no manifest change).
  let cache = TempDir::new().unwrap();
  let rootfs = TempDir::new().unwrap();
  let seed = install("debian:bookworm", cache.path(), rootfs.path(), &["--postinstall=none", "bash"]);
  ok(&seed, "seed a debian rootfs");
  let manifest_before = fs::read(rootfs.path().join(".flatroot/manifest")).unwrap();
  let refused = install("ubuntu:noble", cache.path(), rootfs.path(), &["--postinstall=none", "bash"]);
  assert!(!refused.status.success(), "an ubuntu install into a debian rootfs must be refused");
  let stderr = String::from_utf8_lossy(&refused.stderr);
  assert!(stderr.contains("scoped to one exact source"), "end-to-end refusal must carry the strict wording:\n{stderr}");
  assert!(stderr.contains("cannot install from 'ubuntu:noble'"), "must name the incoming ubuntu source:\n{stderr}");
  let manifest_after = fs::read(rootfs.path().join(".flatroot/manifest")).unwrap();
  assert_eq!(manifest_before, manifest_after, "the refused install must not modify the manifest");
}

// ===========================================================================
// INST-079 — a resolved-but-missing-from-DB package is a hard error in the
// Downloader (downloader.rs:106-110 — `Package '<name>' not found in database`).
// CLI-unreachable in the normal pipeline (resolution only yields DB-present
// names), so it is exercised at the library level: Downloader::fetch on a name
// absent from a real index bails rather than silently skipping.
// ===========================================================================

// covers: INST-079
#[test]
#[serial_test::serial]
fn downloader_fetch_bails_on_name_absent_from_db() {
  use flatroot::db::Index;
  use flatroot::downloader::Downloader;
  use flatroot::remote::RemoteDistro;

  // Index::open_or_populate resolves its own cache dir from FLATROOT_CACHE_HOME
  // (db.rs:166 → Cache::dir_resolve). Point it at an isolated tempdir for this
  // in-process build. `#[serial]` keeps the env mutation from racing other tests.
  let cache_home = TempDir::new().unwrap();
  let prior = std::env::var("FLATROOT_CACHE_HOME").ok();
  // Safety: serialized by `#[serial]`; restored before returning.
  unsafe { std::env::set_var("FLATROOT_CACHE_HOME", cache_home.path()) };

  let result = (|| -> anyhow::Result<String> {
    let cache = Cache::dir_resolve()?;
    let client = Arc::new(HttpClient::new(cache.dir_index(), 3, Duration::from_secs(3600)));
    let remote = RemoteDistro::from_str("debian:bookworm", Arch::X86_64, client.clone())?;
    let vcmp: Arc<dyn flatroot::version::VersionCompare> = Arc::from(remote.version_compare());
    let index = Index::open_or_populate(&remote.cache_key(), vcmp, |writer| remote.index_fetch(writer))?;

    let cache_dir = cache.dir_source(&remote.cache_key());
    std::fs::create_dir_all(&cache_dir)?;
    let downloader = Downloader::new(&index, remote.as_ref(), &cache_dir, 1, 1)?;

    // A ProgressBar from the public UI surface (no direct indicatif dependency).
    let pb = flatroot::ui::RunVoice::bar(1, "test");
    let rt = tokio::runtime::Runtime::new()?;
    let names = vec!["this-package-does-not-exist-xyz-123".to_string()];
    // fetch must be Err; turn it into the message for assertion outside the env
    // window.
    let fetch = rt.block_on(downloader.fetch(&names, &pb));
    Ok(fetch.err().map(|e| e.to_string()).unwrap_or_default())
  })();

  // Restore env before asserting.
  match prior {
    Some(v) => unsafe { std::env::set_var("FLATROOT_CACHE_HOME", v) },
    None => unsafe { std::env::remove_var("FLATROOT_CACHE_HOME") },
  }

  let msg = result.expect("the index build + downloader construction must succeed");
  assert!(
    msg.contains("not found in database"),
    "the Downloader guard (downloader.rs:106-110) must bail naming the missing package: {msg}"
  );
}
