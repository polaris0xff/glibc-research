//! Download / cache / HTTP behaviour for the rootfs builder. The download
//! pipeline rests on three public primitives — `Checksum::verify` (the
//! integrity gate that drives cache reuse vs re-download), `Cache::dir_resolve`
//! (where the durable cache home lands), and `Fs::write_atomic` (temp-then-
//! rename durability) — plus the `HttpClient` retry/cache/HEAD surface. These
//! tests drive those primitives directly and, where the contract is only
//! observable at the boundary, through the real `flatroot` binary (parallel
//! downloads, the retry budget, missing `-o`, env fallbacks, cross-run cache
//! reuse).
//!
//! `HttpClient::head`/`get_fresh` and the retry budget are exercised against a
//! tiny in-process TCP server (no extra dependencies) so the network behaviour
//! is deterministic and offline.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use assert_cmd::Command;
use base64::Engine;
use flatroot::internal::cache::Cache;
use flatroot::internal::fs::Fs;
use flatroot::internal::http::HttpClient;
use flatroot::manifest::{Checksum, ChecksumAlgorithm};
use serial_test::serial;
use sha2::Digest;
use tempfile::TempDir;

// ===========================================================================
// Checksum::verify — SHA-256 / SHA-512 byte-exact accept, corruption reject
// ===========================================================================

// covers: CORE-024
#[test]
fn checksum_verify_accepts_byte_exact_and_rejects_a_flipped_byte() {
  let dir = TempDir::new().unwrap();
  let payload = b"the genuine package archive bytes, exactly as promised";

  for algo in [ChecksumAlgorithm::Sha256, ChecksumAlgorithm::Sha512] {
    let path = dir.path().join(format!("archive-{}", algo_tag(algo)));
    std::fs::write(&path, payload).unwrap();

    // The honest digest of the exact bytes must verify true.
    let hex = match algo {
      ChecksumAlgorithm::Sha256 => hex::encode(sha2::Sha256::digest(payload)),
      ChecksumAlgorithm::Sha512 => hex::encode(sha2::Sha512::digest(payload)),
      ChecksumAlgorithm::Q1Sha1 => unreachable!(),
    };
    let good = Checksum {
      algorithm: algo,
      hex: hex.clone(),
    };
    assert!(good.verify(&path).unwrap(), "byte-exact archive must verify true ({:?})", algo);

    // Flip one byte on disk: verification must now reject it.
    let mut corrupt = payload.to_vec();
    corrupt[0] ^= 0xFF;
    std::fs::write(&path, &corrupt).unwrap();
    assert!(!good.verify(&path).unwrap(), "one flipped byte must verify false ({:?})", algo);
  }
}

fn algo_tag(algo: ChecksumAlgorithm) -> &'static str {
  match algo {
    ChecksumAlgorithm::Sha256 => "256",
    ChecksumAlgorithm::Sha512 => "512",
    ChecksumAlgorithm::Q1Sha1 => "q1",
  }
}

// ===========================================================================
// Checksum::verify — Alpine Q1-SHA1 hashes the second gzip member only
// ===========================================================================

// covers: CORE-025
#[test]
fn checksum_verify_q1_sha1_hashes_only_the_second_gzip_member() {
  // An `.apk` is three concatenated gzip members (signature / control / data).
  // The Q1 checksum is the base64 SHA-1 of the SECOND member's COMPRESSED
  // bytes. Build a 3-member stream, compute the expected Q1 over member 2, and
  // assert verify() is true iff that hash matches.
  let member1 = gzip_member(b"signature stream contents");
  let member2 = gzip_member(b"control stream contents -- this is the one that is hashed");
  let member3 = gzip_member(b"data stream contents, much larger in a real apk");

  let mut apk = Vec::new();
  apk.extend_from_slice(&member1);
  apk.extend_from_slice(&member2);
  apk.extend_from_slice(&member3);

  let dir = TempDir::new().unwrap();
  let path = dir.path().join("pkg.apk");
  std::fs::write(&path, &apk).unwrap();

  // Expected: SHA-1 of member2's compressed bytes, base64-encoded, "Q1"-prefixed.
  let expected_q1 = q1_of(&member2);
  let good = Checksum {
    algorithm: ChecksumAlgorithm::Q1Sha1,
    hex: expected_q1,
  };
  assert!(good.verify(&path).unwrap(), "Q1-SHA1 over the second member must verify true");

  // A Q1 derived from the WRONG member (member1) must verify false.
  let wrong = Checksum {
    algorithm: ChecksumAlgorithm::Q1Sha1,
    hex: q1_of(&member1),
  };
  assert!(!wrong.verify(&path).unwrap(), "Q1 over the first member must not verify the apk");

  // Boundary fallback: a two-member stream — the second member is everything
  // after the first, so a Q1 over member2 alone must still verify true.
  let mut two = Vec::new();
  two.extend_from_slice(&member1);
  two.extend_from_slice(&member2);
  let path_two = dir.path().join("two.apk");
  std::fs::write(&path_two, &two).unwrap();
  let good_two = Checksum {
    algorithm: ChecksumAlgorithm::Q1Sha1,
    hex: q1_of(&member2),
  };
  assert!(good_two.verify(&path_two).unwrap(), "two-member stream: second member is everything after the first");
}

/// Compress `data` into a single standalone gzip member.
fn gzip_member(data: &[u8]) -> Vec<u8> {
  use flate2::Compression;
  use flate2::write::GzEncoder;
  let mut enc = GzEncoder::new(Vec::new(), Compression::default());
  enc.write_all(data).unwrap();
  enc.finish().unwrap()
}

/// Compute the Alpine Q1 string (`Q1` + base64(SHA-1(compressed-bytes))) for
/// one gzip member's compressed bytes.
fn q1_of(member_compressed: &[u8]) -> String {
  let mut hasher = sha1::Sha1::new();
  hasher.update(member_compressed);
  let digest = hasher.finalize();
  format!("Q1{}", base64::engine::general_purpose::STANDARD.encode(digest))
}

// ===========================================================================
// Checksum::infer shape dispatch + format/parse round-trip (supports CORE-026)
// ===========================================================================

// covers: CORE-026
#[test]
fn checksum_infer_dispatches_on_digest_shape_and_drives_reuse() {
  // The cache-reuse path infers the algorithm from the published digest's shape
  // (downloader.rs uses `Checksum::infer(&pkg.checksum)`). Verify the shape
  // dispatch and that an inferred checksum verifies a real file end-to-end.
  let dir = TempDir::new().unwrap();
  let payload = b"cache-reuse candidate bytes";
  let path = dir.path().join("cached.bin");
  std::fs::write(&path, payload).unwrap();

  // 64-hex -> SHA-256; an inferred SHA-256 of the file verifies true.
  let sha256 = hex::encode(sha2::Sha256::digest(payload));
  assert_eq!(Checksum::infer(&sha256).algorithm, ChecksumAlgorithm::Sha256);
  assert!(Checksum::infer(&sha256).verify(&path).unwrap(), "inferred sha256 must verify the cached file (reuse)");

  // 128-hex -> SHA-512.
  let sha512 = hex::encode(sha2::Sha512::digest(payload));
  assert_eq!(Checksum::infer(&sha512).algorithm, ChecksumAlgorithm::Sha512);
  assert!(Checksum::infer(&sha512).verify(&path).unwrap(), "inferred sha512 must verify the cached file (reuse)");

  // Q1-prefixed -> Q1Sha1; the digest is kept verbatim.
  let q1 = Checksum::infer("Q1abcdefABCDEF==");
  assert_eq!(q1.algorithm, ChecksumAlgorithm::Q1Sha1);
  assert_eq!(q1.hex, "Q1abcdefABCDEF==", "Q1 digest is preserved verbatim");

  // The mismatch branch: a checksum that does NOT match the file's bytes must
  // verify false — this is what forces the downloader to re-download.
  let mismatched = Checksum::infer(&hex::encode(sha2::Sha256::digest(b"different bytes")));
  assert!(!mismatched.verify(&path).unwrap(), "a non-matching cached file must verify false (forces re-download)");
}

// ===========================================================================
// Fs::write_atomic — temp-then-rename, no partial file, fsync surfaced
// ===========================================================================

// covers: CORE-051
#[test]
fn fs_write_atomic_swaps_whole_and_keeps_prior_on_no_write() {
  let dir = TempDir::new().unwrap();
  let path = dir.path().join("record");

  // First write lands the whole file; no `.tmp` is left behind.
  Fs::write_atomic(&path, b"version-1").unwrap();
  assert_eq!(std::fs::read(&path).unwrap(), b"version-1");
  assert!(!path.with_extension("tmp").exists(), "no leftover .tmp after a clean atomic write");

  // A second atomic write replaces the contents whole.
  Fs::write_atomic(&path, b"version-2-longer-contents").unwrap();
  assert_eq!(std::fs::read(&path).unwrap(), b"version-2-longer-contents");
  assert!(!path.with_extension("tmp").exists(), "no leftover .tmp after the second atomic write");

  // write_atomic creates the parent directory when absent.
  let nested = dir.path().join("a/b/c/record");
  Fs::write_atomic(&nested, b"nested").unwrap();
  assert_eq!(std::fs::read(&nested).unwrap(), b"nested", "write_atomic creates missing parents");

  // remove_lenient is quiet whether or not the target exists.
  Fs::remove_lenient(&path);
  assert!(!path.exists(), "remove_lenient removes an existing file");
  Fs::remove_lenient(&path); // already gone — must not panic or error
}

// ===========================================================================
// Cache::dir_resolve — FLATROOT_CACHE_HOME > XDG > HOME > /tmp; dir created
// ===========================================================================
//
// dir_resolve reads process env, so these run serially and save/restore the
// three governing variables around each scenario. set_var is unsafe in edition
// 2024; this is the only place these vars are mutated and the test is
// serialized, so no other thread reads them concurrently.

// covers: CORE-029
#[test]
#[serial]
fn cache_dir_resolve_honors_precedence_chain_and_creates_the_dir() {
  let saved = EnvGuard::save(&["FLATROOT_CACHE_HOME", "XDG_CACHE_HOME", "HOME"]);

  // 1. FLATROOT_CACHE_HOME wins outright and is used verbatim.
  let explicit = TempDir::new().unwrap();
  env_set("FLATROOT_CACHE_HOME", explicit.path().to_str().unwrap());
  env_set("XDG_CACHE_HOME", "/nonexistent/xdg");
  env_set("HOME", "/nonexistent/home");
  let cache = Cache::dir_resolve().unwrap();
  assert_eq!(cache.root(), explicit.path(), "FLATROOT_CACHE_HOME must win and be used verbatim");
  assert!(cache.root().is_dir(), "dir_resolve must create the cache home");

  // 2. Without FLATROOT_CACHE_HOME, XDG_CACHE_HOME/flatroot is used.
  env_remove("FLATROOT_CACHE_HOME");
  let xdg = TempDir::new().unwrap();
  env_set("XDG_CACHE_HOME", xdg.path().to_str().unwrap());
  let cache = Cache::dir_resolve().unwrap();
  assert_eq!(cache.root(), xdg.path().join("flatroot"), "XDG_CACHE_HOME/flatroot is the second choice");
  assert!(cache.root().is_dir(), "the XDG-based cache home must be created");

  // 3. Without XDG either, HOME/.cache/flatroot is used.
  env_remove("XDG_CACHE_HOME");
  let home = TempDir::new().unwrap();
  env_set("HOME", home.path().to_str().unwrap());
  let cache = Cache::dir_resolve().unwrap();
  assert_eq!(cache.root(), home.path().join(".cache/flatroot"), "HOME/.cache/flatroot is the third choice");
  assert!(cache.root().is_dir(), "the HOME-based cache home must be created");

  // The conventional sub-areas are derived beneath the chosen root.
  assert_eq!(cache.dir_index(), home.path().join(".cache/flatroot/index"));
  assert_eq!(cache.dir_source("debian:bookworm"), home.path().join(".cache/flatroot/debian:bookworm"));

  saved.restore();
}

/// Saves and restores a set of environment variables so a serial test that
/// mutates process env leaves the environment untouched for siblings.
struct EnvGuard {
  saved: Vec<(String, Option<String>)>,
}

impl EnvGuard {
  fn save(keys: &[&str]) -> Self {
    let saved = keys.iter().map(|k| (k.to_string(), std::env::var(k).ok())).collect();
    EnvGuard { saved }
  }

  fn restore(self) {
    for (k, v) in &self.saved {
      match v {
        Some(val) => env_set(k, val),
        None => env_remove(k),
      }
    }
  }
}

fn env_set(key: &str, val: &str) {
  // Safety: these vars are mutated only inside #[serial] tests in this file, so
  // no other test thread reads them concurrently.
  unsafe { std::env::set_var(key, val) };
}

fn env_remove(key: &str) {
  // Safety: as above — serialized mutation, no concurrent readers.
  unsafe { std::env::remove_var(key) };
}

// ===========================================================================
// HttpClient::head — 2xx -> true, 404 -> false, 5xx -> retried then errored
// ===========================================================================

// covers: CORE-034
#[test]
fn http_head_classifies_present_absent_and_retries_inconclusive() {
  // 200 -> present (true).
  let server = MockServer::start_status(200);
  let client = HttpClient::new(TempDir::new().unwrap().path().to_path_buf(), 3, Duration::from_secs(3600));
  assert_eq!(client.head(&server.url("/present")).unwrap(), true, "HTTP 200 must classify as present");
  server.shutdown();

  // 404 -> absent (false), answered cheaply with no retry.
  let server = MockServer::start_status(404);
  let client = HttpClient::new(TempDir::new().unwrap().path().to_path_buf(), 3, Duration::from_secs(3600));
  assert_eq!(client.head(&server.url("/missing")).unwrap(), false, "HTTP 404 must classify as absent");
  assert_eq!(server.hits(), 1, "a definitive 404 must not be retried");
  server.shutdown();

  // 503 -> inconclusive, retried up to the budget then errored. With a budget
  // of 3 retries the probe makes one initial try plus 3 retries — 4 attempts.
  let server = MockServer::start_status(503);
  let client = HttpClient::new(TempDir::new().unwrap().path().to_path_buf(), 3, Duration::from_secs(3600));
  let err = client.head(&server.url("/flaky")).err().expect("expected an error");
  assert!(err.to_string().contains("after 4 attempts"), "persistent 5xx HEAD must error after the budget: {err}");
  assert_eq!(server.hits(), 4, "an inconclusive HEAD makes one initial try plus the retry budget");
  server.shutdown();
}

// ===========================================================================
// HttpClient::get_fresh — always hits the network, streams the body
// ===========================================================================

// covers: CORE-033
#[test]
fn http_get_fresh_always_hits_the_network_even_with_a_warm_cache() {
  let body = vec![0x42u8; 20_000]; // > 8192 so chunked streaming runs
  let server = MockServer::start_body(body.clone());
  let cache_dir = TempDir::new().unwrap();
  let client = HttpClient::new(cache_dir.path().to_path_buf(), 3, Duration::from_secs(3600));

  // Even after a prior fetch warmed the cache, get_fresh fetches again.
  let url = server.url("/blob");
  let first = client.get_fresh(&url).unwrap();
  assert_eq!(first, body, "get_fresh returns the full streamed body");
  let _warm = client.get_cached(&url); // best-effort: warm the cache
  let hits_before = server.hits();
  let second = client.get_fresh(&url).unwrap();
  assert_eq!(second, body, "get_fresh returns the body again");
  assert!(server.hits() > hits_before, "get_fresh must hit the network unconditionally, not serve a cache");
  server.shutdown();
}

// ===========================================================================
// HttpClient::get_fresh — retries on failure, hard-errors after the budget
// ===========================================================================

// covers: CORE-031
#[test]
fn http_get_fresh_retries_then_hard_errors_after_the_budget() {
  // Server always answers 500: get_fresh makes one initial try plus `retries`
  // retries — 5 attempts for a budget of 4 — then bails naming the count.
  let server = MockServer::start_status(500);
  let client = HttpClient::new(TempDir::new().unwrap().path().to_path_buf(), 4, Duration::from_secs(3600));
  let err = client
    .get_fresh(&server.url("/broken"))
    .err()
    .expect("expected an error");
  assert!(err.to_string().contains("after 5 attempts"), "get_fresh must hard-error after N attempts: {err}");
  assert_eq!(server.hits(), 5, "one initial try plus the retry budget governs how many attempts are made");
  server.shutdown();

  // The budget is exposed so siblings can borrow the same patience.
  assert_eq!(client.retries(), 4, "retries() reports the configured budget");
}

// ===========================================================================
// get_cached — within-TTL local serve; stale refetch; name == sha256(url)
// ===========================================================================

// covers: CORE-032
#[test]
fn http_get_cached_honors_ttl_and_names_the_file_by_url_sha256() {
  let body = b"published index body".to_vec();
  let server = MockServer::start_body(body.clone());
  let cache_dir = TempDir::new().unwrap();
  let url = server.url("/Packages");

  // Fresh fetch with a long TTL, then a second call served from cache (no
  // second network hit).
  let fresh_client = HttpClient::new(cache_dir.path().to_path_buf(), 3, Duration::from_secs(3600));
  let first = fresh_client.get_cached(&url).unwrap();
  assert_eq!(first, body, "first get_cached fetches and returns the body");
  let hits_after_first = server.hits();
  let second = fresh_client.get_cached(&url).unwrap();
  assert_eq!(second, body, "within TTL the cached copy is served");
  assert_eq!(server.hits(), hits_after_first, "within TTL there must be no second network hit");

  // The cache file is named by sha256(url) — recompute it independently.
  let name_sha256 = hex::encode(sha2::Sha256::digest(url.as_bytes()));
  let cache_file = cache_dir.path().join(&name_sha256);
  assert!(cache_file.exists(), "cache file must be named sha256(url): {name_sha256}");
  assert_eq!(std::fs::read(&cache_file).unwrap(), body, "cache file holds the fetched body");

  // Stale: a zero-TTL client against the SAME url and SAME cache dir treats the
  // existing entry as always stale, so each call refetches over the network and
  // resaves — the hit count must keep climbing.
  let stale_client = HttpClient::new(cache_dir.path().to_path_buf(), 3, Duration::from_secs(0));
  let hits_before_stale = server.hits();
  let refetched = stale_client.get_cached(&url).unwrap();
  assert_eq!(refetched, body, "a stale (TTL=0) entry is refetched and returns the body");
  let again = stale_client.get_cached(&url).unwrap();
  assert_eq!(again, body, "the refetched body is resaved and re-served");
  assert!(
    server.hits() >= hits_before_stale + 2,
    "TTL=0 must refetch on every call (hits before stale: {hits_before_stale}, now: {})",
    server.hits()
  );
  server.shutdown();
}

// ===========================================================================
// A tiny single-purpose HTTP server (no extra deps)
// ===========================================================================

/// A minimal HTTP/1.1 server that answers every request with a fixed status
/// (and optional body), counting how many requests it received. Used to make
/// the retry / HEAD / get_fresh behaviour deterministic and offline.
struct MockServer {
  addr: String,
  hits: Arc<AtomicUsize>,
  handle: Option<std::thread::JoinHandle<()>>,
  shutdown: Arc<AtomicUsize>,
}

impl MockServer {
  fn start_status(status: u16) -> Self {
    Self::start(status, Vec::new())
  }

  fn start_body(body: Vec<u8>) -> Self {
    Self::start(200, body)
  }

  fn start(status: u16, body: Vec<u8>) -> Self {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    listener.set_nonblocking(true).expect("nonblocking listener");
    let addr = listener.local_addr().unwrap().to_string();
    let hits = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicUsize::new(0));
    let hits_thread = hits.clone();
    let shutdown_thread = shutdown.clone();

    let handle = std::thread::spawn(move || {
      loop {
        if shutdown_thread.load(Ordering::SeqCst) != 0 {
          break;
        }
        match listener.accept() {
          Ok((mut stream, _)) => {
            // Accepted sockets are not guaranteed to inherit the listener's
            // nonblocking mode; force blocking so the header read does not
            // return WouldBlock before the request arrives.
            let _ = stream.set_nonblocking(false);
            // Read (and discard) the request headers.
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            hits_thread.fetch_add(1, Ordering::SeqCst);

            let reason = match status {
              200 => "OK",
              404 => "Not Found",
              500 => "Internal Server Error",
              503 => "Service Unavailable",
              _ => "Status",
            };
            let header =
              format!("HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&body);
            let _ = stream.flush();
          }
          Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            std::thread::sleep(Duration::from_millis(5));
          }
          Err(_) => break,
        }
      }
    });

    MockServer {
      addr,
      hits,
      handle: Some(handle),
      shutdown,
    }
  }

  fn url(&self, path: &str) -> String {
    format!("http://{}{}", self.addr, path)
  }

  fn hits(&self) -> usize {
    self.hits.load(Ordering::SeqCst)
  }

  fn shutdown(mut self) {
    self.shutdown.store(1, Ordering::SeqCst);
    if let Some(h) = self.handle.take() {
      let _ = h.join();
    }
  }
}

// ===========================================================================
// Black-box: missing -o bails; env fallbacks; parallel; cross-run cache reuse
// ===========================================================================

/// Build the base `flatroot` command with an isolated cache.
fn flatroot_with_cache(cache: &Path) -> Command {
  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  cmd.env("FLATROOT_CACHE_HOME", cache);
  cmd
}

// covers: CORE-052
#[test]
fn install_without_output_bails_with_the_exact_message() {
  // `install` with no -o/--output and no FLATROOT_ARG_INSTALL_OUTPUT must bail
  // with main.rs's exact wording. Use an unknown distro so the run would only
  // ever fail later — proving the missing-output gate fires first.
  let cache = TempDir::new().unwrap();
  let out = flatroot_with_cache(cache.path())
    .args(["--from", "debian:bookworm", "install", "bash"])
    .output()
    .unwrap();
  assert!(!out.status.success(), "install without an output dir must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Install output directory is required"),
    "missing -o must bail with the documented message:\n{stderr}"
  );
}

// covers: CORE-035
#[test]
fn install_unknown_arch_bails_before_any_work() {
  // `--arch mips64` is parsed via Arch::from_uname before any network/disk work
  // and must bail with the exact unsupported-arch message.
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = flatroot_with_cache(cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "mips64",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .output()
    .unwrap();
  assert!(!out.status.success(), "an unknown --arch must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Unsupported architecture 'mips64'"),
    "unknown arch must bail with the exact message before any work:\n{stderr}"
  );
  // It failed before touching the rootfs.
  assert!(!root.path().join(".flatroot").exists(), "no work should have started for an unknown arch");
}

// covers: CORE-052
#[test]
fn install_env_fallbacks_supply_from_arch_and_output() {
  // FLATROOT_ARG_FROM / FLATROOT_ARG_ARCH / FLATROOT_ARG_INSTALL_OUTPUT supply
  // the values when the flags are absent. Drive a real install (bash) so the
  // env-sourced --from, --arch and -o all have to take effect for the tree to
  // appear at the env-named path with the env-named arch.
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env("FLATROOT_ARG_FROM", "debian:bookworm")
    .env("FLATROOT_ARG_ARCH", "x86_64")
    .env("FLATROOT_ARG_INSTALL_OUTPUT", root.path().to_str().unwrap())
    .args(["install", "--postinstall=none", "bash"])
    .output()
    .unwrap();
  assert!(out.status.success(), "env fallbacks must drive a real install:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(root.path().join("bin/bash").exists(), "FLATROOT_ARG_INSTALL_OUTPUT must place the tree there");
  assert!(
    root.path().join("usr/lib/x86_64-linux-gnu").exists() || root.path().join("lib/x86_64-linux-gnu").exists(),
    "FLATROOT_ARG_ARCH=x86_64 must select the x86_64 multiarch lib dir"
  );
}

// covers: CORE-030
#[test]
fn install_parallel_flag_is_honored_and_succeeds() {
  // `-p 8` runs the parallel download path (buffer_unordered(jobs.max(1))); the
  // install must still complete and lay the package down. A complementary -p 1
  // (serial) run must also succeed — both ends of the jobs range work.
  for jobs in ["8", "1"] {
    let cache = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let out = flatroot_with_cache(cache.path())
      .args([
        "--from",
        "debian:bookworm",
        "install",
        "--output",
        root.path().to_str().unwrap(),
        "--postinstall=none",
        "-p",
        jobs,
        "bash",
      ])
      .output()
      .unwrap();
    assert!(out.status.success(), "-p {jobs} install failed:\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(root.path().join("bin/bash").exists(), "-p {jobs} install must produce bash");
  }
}

// covers: CORE-028, CORE-026
#[test]
fn install_reuses_verified_cache_across_separate_runs() {
  // Two install runs into a fresh rootfs but the SAME cache home. The first
  // populates the cache; the second must reuse the checksum-verified archives
  // (no re-download) — observable as the "cached" marker the downloader emits
  // for a cache hit, and a successful build.
  let cache = TempDir::new().unwrap();

  let root1 = TempDir::new().unwrap();
  let out1 = flatroot_with_cache(cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root1.path().to_str().unwrap(),
      "--postinstall=none",
      "bash",
    ])
    .output()
    .unwrap();
  assert!(out1.status.success(), "first install failed:\n{}", String::from_utf8_lossy(&out1.stderr));

  let root2 = TempDir::new().unwrap();
  let out2 = flatroot_with_cache(cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root2.path().to_str().unwrap(),
      "--postinstall=none",
      "bash",
    ])
    .output()
    .unwrap();
  assert!(out2.status.success(), "second install failed:\n{}", String::from_utf8_lossy(&out2.stderr));
  let stderr2 = String::from_utf8_lossy(&out2.stderr);
  assert!(
    stderr2.contains("cached"),
    "the second run against a warm shared cache must reuse verified archives (cached marker):\n{stderr2}"
  );
  assert!(root2.path().join("bin/bash").exists(), "the cache-reusing run must still produce a working tree");
}

// covers: CORE-027
#[test]
fn install_re_downloads_a_checksum_mismatched_cache_entry() {
  // A cached archive whose bytes no longer match the index checksum must be
  // detected (Checksum::verify -> false) and re-downloaded — the install
  // recovers and still produces a working tree. This drives the downloader's
  // mismatch branch (downloader.rs:120-128 / 154-168) end-to-end: the same
  // verify-false verdict that, on a PERSISTENT mirror mismatch, escalates to
  // the "after retry" hard-fail. We poison every cached `.deb` after a warm
  // first run, then reinstall into a fresh tree against the same cache.
  let cache = TempDir::new().unwrap();

  // Warm the cache with a genuine install.
  let root1 = TempDir::new().unwrap();
  let out1 = flatroot_with_cache(cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root1.path().to_str().unwrap(),
      "--postinstall=none",
      "bash",
    ])
    .output()
    .unwrap();
  assert!(out1.status.success(), "first (warming) install failed:\n{}", String::from_utf8_lossy(&out1.stderr));

  // Corrupt every cached archive so its bytes no longer match the index
  // checksum. Walk the cache tree and overwrite each `.deb` with junk.
  let poisoned = poison_cached_archives(cache.path());
  assert!(poisoned > 0, "expected at least one cached .deb to poison");

  // Reinstall into a fresh tree against the poisoned cache: the downloader must
  // notice each mismatch, re-download the genuine bytes, and succeed.
  let root2 = TempDir::new().unwrap();
  let out2 = flatroot_with_cache(cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root2.path().to_str().unwrap(),
      "--postinstall=none",
      "bash",
    ])
    .output()
    .unwrap();
  assert!(
    out2.status.success(),
    "install must recover from a checksum-mismatched cache by re-downloading:\n{}",
    String::from_utf8_lossy(&out2.stderr)
  );
  assert!(root2.path().join("bin/bash").exists(), "the re-download must produce a working tree");

  // The verify-false verdict that gates the re-download (and, on a persistent
  // mismatch, the hard-fail) is the same primitive proven directly here: junk
  // bytes never satisfy the genuine checksum.
  let dir = TempDir::new().unwrap();
  let path = dir.path().join("corrupt.deb");
  std::fs::write(&path, b"definitely not the promised archive").unwrap();
  let genuine = Checksum::infer(&hex::encode(sha2::Sha256::digest(b"the promised archive bytes")));
  assert!(!genuine.verify(&path).unwrap(), "mismatched bytes must verify false (the retry/hard-fail trigger)");
}

/// Overwrite every `*.deb` archive anywhere under the cache root with junk
/// bytes so its checksum no longer matches the index. Returns how many were
/// poisoned.
fn poison_cached_archives(cache_root: &Path) -> usize {
  fn walk(dir: &Path, count: &mut usize) {
    let entries = match std::fs::read_dir(dir) {
      Ok(e) => e,
      Err(_) => return,
    };
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_dir() {
        walk(&path, count);
      } else if path.extension().and_then(|e| e.to_str()) == Some("deb") {
        std::fs::write(&path, b"corrupted cache contents -- checksum will not match").unwrap();
        *count += 1;
      }
    }
  }
  let mut count = 0;
  walk(cache_root, &mut count);
  count
}
