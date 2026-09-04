//! Subsystem tests for the mirror layer (`flatroot::mirror`) and the
//! release-listing engine (`flatroot::distro::ReleaseScraper`), driven against
//! a local in-process HTTP server so the ordered-fallback, probe/exists, and
//! UTF-8/url-join behaviours can be exercised deterministically without the
//! real network. The listing engine's private family filters (accept,
//! sort_key, listing_parse, mirror_label, codename/apk validation) are reached
//! through the public `ReleaseScraper::collect` by feeding crafted directory
//! HTML and asserting the resulting `TableRelease`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use flatroot::distro::{ReleaseScraper, WalkStrategy};
use flatroot::internal::http::HttpClient;
use flatroot::mirror::{HttpMirror, ListMirror, Mirror};

// ---------------------------------------------------------------------------
// In-process HTTP test server
// ---------------------------------------------------------------------------

/// A canned reply for one request path: status line, optional body, and
/// whether the body is sent on a HEAD (it never is — HEAD carries no body, but
/// the Content-Length header still reflects the would-be body length).
#[derive(Clone)]
struct Reply {
  status: u16,
  reason: &'static str,
  body: Vec<u8>,
  /// When true, the connection is abruptly closed instead of replying — used
  /// to simulate an unreachable/erroring mirror that is not a clean 404.
  hangup: bool,
}

impl Reply {
  fn ok(body: impl Into<Vec<u8>>) -> Reply {
    Reply {
      status: 200,
      reason: "OK",
      body: body.into(),
      hangup: false,
    }
  }
  fn not_found() -> Reply {
    Reply {
      status: 404,
      reason: "Not Found",
      body: b"missing".to_vec(),
      hangup: false,
    }
  }
  fn server_error() -> Reply {
    Reply {
      status: 500,
      reason: "Internal Server Error",
      body: b"boom".to_vec(),
      hangup: false,
    }
  }
  fn hangup() -> Reply {
    Reply {
      status: 0,
      reason: "",
      body: Vec::new(),
      hangup: true,
    }
  }
}

/// One recorded request: the HTTP method and the request-target path.
#[derive(Clone, Debug, PartialEq)]
struct Hit {
  method: String,
  path: String,
}

/// A minimal single-host HTTP/1.1 server that serves a fixed route table and
/// records every request. Lives for the duration of the test; the listener
/// thread exits when `stop` flips and a final self-connect unblocks `accept`.
struct TestServer {
  base: String,
  hits: Arc<Mutex<Vec<Hit>>>,
  stop: Arc<AtomicBool>,
  addr: std::net::SocketAddr,
}

impl TestServer {
  /// Start a server with an exact-path route table. Any path not in the table
  /// gets `default`.
  fn start(routes: HashMap<String, Reply>, default: Reply) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let routes = Arc::new(routes);
    let hits = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));

    let routes_t = Arc::clone(&routes);
    let hits_t = Arc::clone(&hits);
    let stop_t = Arc::clone(&stop);
    thread::spawn(move || {
      for stream in listener.incoming() {
        if stop_t.load(Ordering::SeqCst) {
          break;
        }
        let Ok(stream) = stream else { continue };
        handle(stream, &routes_t, &default, &hits_t);
      }
    });

    TestServer { base, hits, stop, addr }
  }

  fn base(&self) -> &str {
    &self.base
  }

  fn hits(&self) -> Vec<Hit> {
    self.hits.lock().unwrap().clone()
  }
}

impl Drop for TestServer {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::SeqCst);
    // Unblock the accept loop with a final throwaway connection.
    let _ = TcpStream::connect(self.addr);
  }
}

/// Read one request line + headers, record the hit, and write the canned reply.
fn handle(mut stream: TcpStream, routes: &HashMap<String, Reply>, default: &Reply, hits: &Arc<Mutex<Vec<Hit>>>) {
  let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
  let mut request_line = String::new();
  if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
    return;
  }
  let mut parts = request_line.split_whitespace();
  let method = parts.next().unwrap_or("").to_string();
  let path = parts.next().unwrap_or("").to_string();

  // Drain the remaining request headers up to the blank line.
  let mut line = String::new();
  loop {
    line.clear();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
      break;
    }
    if line == "\r\n" || line == "\n" {
      break;
    }
  }

  hits.lock().unwrap().push(Hit {
    method: method.clone(),
    path: path.clone(),
  });

  let reply = routes.get(&path).cloned().unwrap_or_else(|| default.clone());
  if reply.hangup {
    // Close without responding.
    let _ = stream.shutdown(std::net::Shutdown::Both);
    return;
  }

  let is_head = method.eq_ignore_ascii_case("HEAD");
  let header = format!(
    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
    reply.status,
    reply.reason,
    reply.body.len()
  );
  let _ = stream.write_all(header.as_bytes());
  if !is_head {
    let _ = stream.write_all(&reply.body);
  }
  let _ = stream.flush();
  let _ = stream.shutdown(std::net::Shutdown::Both);
}

/// A short-retry HTTP client with an isolated temp cache and a near-zero TTL,
/// so error paths fail fast (one attempt at `retries=1`) and per-test caching
/// never masks a second mirror's involvement or collides with the real cache.
fn http_client_no_cache(retries: u32) -> Arc<HttpClient> {
  let dir = std::env::temp_dir().join(format!("flatroot-mirror-nocache-{}", uniq()));
  Arc::new(HttpClient::new(dir, retries, Duration::from_millis(0)))
}

fn uniq() -> String {
  use std::sync::atomic::AtomicU64;
  static N: AtomicU64 = AtomicU64::new(0);
  format!("{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))
}

fn routes(pairs: Vec<(&str, Reply)>) -> HashMap<String, Reply> {
  pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

// ---------------------------------------------------------------------------
// HttpMirror::url_join slash normalization (DIST-059)
// ---------------------------------------------------------------------------

// covers: DIST-059
#[test]
fn url_join_normalizes_to_exactly_one_slash() {
  // mirror/http.rs:95-97 trims a trailing slash off the base and a leading
  // slash off the rel, joining with exactly one '/'. Across the four
  // punctuation combinations the server must always see the same path.
  for (base_suffix, rel) in [("", "foo/bar"), ("/", "foo/bar"), ("", "/foo/bar"), ("/", "/foo/bar")] {
    let server = TestServer::start(routes(vec![("/foo/bar", Reply::ok("hit"))]), Reply::not_found());
    let http = http_client_no_cache(1);
    let base = format!("{}{}", server.base(), base_suffix);
    let mirror = HttpMirror::live(base, http);
    let bytes = mirror.fetch(rel).expect("fetch must reach the normalized path");
    assert_eq!(bytes, b"hit", "base_suffix={base_suffix:?} rel={rel:?}");
    let hits = server.hits();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "/foo/bar", "base_suffix={base_suffix:?} rel={rel:?} must normalize to one slash");
  }
}

// ---------------------------------------------------------------------------
// HttpMirror::probe 404 → false, other error → Err (DIST-056)
// ---------------------------------------------------------------------------

// covers: DIST-056
#[test]
fn probe_maps_404_to_false_and_other_errors_to_err() {
  // mirror/http.rs:109-122. A 404 (via get_cached's "HTTP 404" error string)
  // maps to Ok(false); a present file is Ok(true); a 500 (non-404) propagates
  // as Err carrying the 'probe <url>' context.
  let server = TestServer::start(
    routes(vec![
      ("/present", Reply::ok("here")),
      ("/gone", Reply::not_found()),
      ("/broken", Reply::server_error()),
    ]),
    Reply::not_found(),
  );
  let mirror = HttpMirror::live(server.base().to_string(), http_client_no_cache(1));

  assert!(mirror.probe("present").unwrap(), "a present file probes true");
  assert!(!mirror.probe("gone").unwrap(), "a 404 probes false");

  let err = mirror.probe("broken").err().expect("expected an error");
  let msg = format!("{err:#}");
  assert!(msg.contains("probe "), "a non-404 error must carry the 'probe <url>' context: {msg}");
}

// ---------------------------------------------------------------------------
// HttpMirror::exists uses HEAD (no body) vs fetch's GET (DIST-057)
// ---------------------------------------------------------------------------

// covers: DIST-057
#[test]
fn exists_uses_head_while_fetch_uses_get() {
  // exists() goes through HttpClient::head (mirror/http.rs:124-130); fetch()
  // goes through get_cached → GET. The server records the method per request.
  let server = TestServer::start(routes(vec![("/file", Reply::ok("body-bytes"))]), Reply::not_found());
  let mirror = HttpMirror::live(server.base().to_string(), http_client_no_cache(1));

  assert!(mirror.exists("file").unwrap(), "exists must be true for a present file");
  let bytes = mirror.fetch("file").unwrap();
  assert_eq!(bytes, b"body-bytes", "fetch returns the GET body");

  let hits = server.hits();
  assert!(
    hits.contains(&Hit {
      method: "HEAD".into(),
      path: "/file".into()
    }),
    "exists must issue a HEAD: {hits:?}"
  );
  assert!(
    hits.contains(&Hit {
      method: "GET".into(),
      path: "/file".into()
    }),
    "fetch must issue a GET: {hits:?}"
  );
}

// covers: DIST-057
#[test]
fn exists_missing_is_false_and_error_carries_context() {
  // A 404 HEAD is Ok(false); a 500 HEAD (after the retry budget) is an Err
  // carrying the 'exists <url>' context.
  let server = TestServer::start(
    routes(vec![("/nope", Reply::not_found()), ("/broken", Reply::server_error())]),
    Reply::not_found(),
  );
  let mirror = HttpMirror::live(server.base().to_string(), http_client_no_cache(1));
  assert!(!mirror.exists("nope").unwrap(), "a 404 HEAD is Ok(false)");
  let err = mirror.exists("broken").err().expect("expected an error");
  assert!(format!("{err:#}").contains("exists "), "a non-404 HEAD error must carry 'exists <url>' context: {err:#}");
}

// ---------------------------------------------------------------------------
// HttpMirror::fetch_listing rejects non-UTF-8 bodies (DIST-058)
// ---------------------------------------------------------------------------

// covers: DIST-058
#[test]
fn fetch_listing_rejects_non_utf8_body() {
  // mirror/http.rs:132-139 turns the fetched bytes into a String and errors
  // with 'listing at <url> not utf-8' when they are not valid UTF-8.
  let server = TestServer::start(routes(vec![("/dir/", Reply::ok(vec![0xff, 0xfe, 0x00, 0x80]))]), Reply::not_found());
  let mirror = HttpMirror::live(server.base().to_string(), http_client_no_cache(1));
  let err = mirror.fetch_listing("dir/").err().expect("expected an error");
  assert!(format!("{err:#}").contains("not utf-8"), "non-UTF-8 listing must be rejected: {err:#}");
}

// ---------------------------------------------------------------------------
// ListMirror ordered fallback (DIST-052, DIST-053)
// ---------------------------------------------------------------------------

// covers: DIST-052
#[test]
fn list_mirror_fetch_falls_over_to_second_on_first_failure() {
  // mirror/list.rs:61-75: the first mirror's failure is swallowed and the
  // second mirror's bytes (and its origin) are returned.
  let first = TestServer::start(routes(vec![("/idx", Reply::hangup())]), Reply::server_error());
  let second = TestServer::start(routes(vec![("/idx", Reply::ok("second-body"))]), Reply::not_found());
  let http = http_client_no_cache(1);
  let mirrors = ListMirror::new(vec![
    Box::new(HttpMirror::live(first.base().to_string(), Arc::clone(&http))),
    Box::new(HttpMirror::live(second.base().to_string(), Arc::clone(&http))),
  ]);

  let (bytes, origin) = mirrors
    .fetch_with_origin("idx")
    .expect("fallback to second mirror must succeed");
  assert_eq!(bytes, b"second-body", "second mirror's bytes must be returned");
  assert_eq!(origin, second.base(), "the reported origin must be the second mirror");

  let plain = mirrors
    .fetch("idx")
    .expect("fetch (origin discarded) must also succeed");
  assert_eq!(plain, b"second-body");
}

// covers: DIST-053
#[test]
fn list_mirror_fetch_bails_combined_error_when_all_fail() {
  // mirror/list.rs:69-74: with every mirror failing, the bail names the mirror
  // count, the path, and the last error.
  let first = TestServer::start(routes(vec![]), Reply::server_error());
  let second = TestServer::start(routes(vec![]), Reply::server_error());
  let http = http_client_no_cache(1);
  let mirrors = ListMirror::new(vec![
    Box::new(HttpMirror::live(first.base().to_string(), Arc::clone(&http))),
    Box::new(HttpMirror::live(second.base().to_string(), Arc::clone(&http))),
  ]);
  let err = mirrors
    .fetch_with_origin("pkgs/x.deb")
    .err()
    .expect("expected an error");
  let msg = err.to_string();
  assert!(msg.starts_with("all 2 mirror(s) failed for pkgs/x.deb:"), "combined-failure bail text: {msg}");
}

// ---------------------------------------------------------------------------
// ListMirror::select (DIST-054)
// ---------------------------------------------------------------------------

// covers: DIST-054
#[test]
fn list_mirror_select_returns_first_probe_true_else_bails() {
  // mirror/list.rs:121-128 returns the first mirror whose probe is true.
  let first = TestServer::start(routes(vec![("/item", Reply::not_found())]), Reply::not_found());
  let second = TestServer::start(routes(vec![("/item", Reply::ok("yes"))]), Reply::not_found());
  let http = http_client_no_cache(1);
  let mirrors = ListMirror::new(vec![
    Box::new(HttpMirror::live(first.base().to_string(), Arc::clone(&http))),
    Box::new(HttpMirror::live(second.base().to_string(), Arc::clone(&http))),
  ]);
  let chosen = mirrors.select("item").expect("select must find the second mirror");
  assert_eq!(chosen.url_base(), second.base(), "select returns the first probe-true mirror");

  // When none probe true (all 404), select bails 'no mirror serves <path>'.
  let none1 = TestServer::start(routes(vec![]), Reply::not_found());
  let none2 = TestServer::start(routes(vec![]), Reply::not_found());
  let mirrors_none = ListMirror::new(vec![
    Box::new(HttpMirror::live(none1.base().to_string(), Arc::clone(&http))),
    Box::new(HttpMirror::live(none2.base().to_string(), Arc::clone(&http))),
  ]);
  let err = mirrors_none.select("item").err().expect("expected an error");
  assert_eq!(err.to_string(), "no mirror serves item", "select with none serving must bail");
}

// covers: DIST-054
#[test]
fn list_mirror_select_propagates_probe_error() {
  // A probe Err (non-404) propagates out of select rather than being treated
  // as "not serving" (mirror/list.rs:121-128 uses `?` on probe).
  let broken = TestServer::start(routes(vec![("/item", Reply::server_error())]), Reply::server_error());
  let http = http_client_no_cache(1);
  let mirrors = ListMirror::new(vec![Box::new(HttpMirror::live(broken.base().to_string(), http))]);
  let err = mirrors.select("item").err().expect("expected an error");
  assert!(format!("{err:#}").contains("probe "), "a probe error must propagate from select: {err:#}");
}

// ---------------------------------------------------------------------------
// ListMirror::fetch_all / fetch_listings aggregate-and-abort (DIST-055)
// ---------------------------------------------------------------------------

// covers: DIST-055
#[test]
fn list_mirror_fetch_all_visits_every_mirror_and_aborts_on_failure() {
  // mirror/list.rs:83-94: fetch_all visits EVERY mirror (not first-wins) and
  // aborts on any failure with 'listing <path> at <url>' context.
  let m1 = TestServer::start(routes(vec![("/data", Reply::ok("one"))]), Reply::not_found());
  let m2 = TestServer::start(routes(vec![("/data", Reply::ok("two"))]), Reply::not_found());
  let http = http_client_no_cache(1);
  let mirrors = ListMirror::new(vec![
    Box::new(HttpMirror::live(m1.base().to_string(), Arc::clone(&http))),
    Box::new(HttpMirror::live(m2.base().to_string(), Arc::clone(&http))),
  ]);
  let mut bodies: Vec<Vec<u8>> = Vec::new();
  mirrors
    .fetch_all("data", |_m, bytes| {
      bodies.push(bytes);
      Ok(())
    })
    .expect("all-ok fetch_all must succeed");
  assert_eq!(bodies, vec![b"one".to_vec(), b"two".to_vec()], "fetch_all must gather from every mirror in order");

  // Now make the second mirror fail: the whole call aborts with context.
  let g1 = TestServer::start(routes(vec![("/data", Reply::ok("one"))]), Reply::not_found());
  let g2 = TestServer::start(routes(vec![]), Reply::server_error());
  let mirrors_fail = ListMirror::new(vec![
    Box::new(HttpMirror::live(g1.base().to_string(), Arc::clone(&http))),
    Box::new(HttpMirror::live(g2.base().to_string(), Arc::clone(&http))),
  ]);
  let err = mirrors_fail
    .fetch_all("data", |_m, _bytes| Ok(()))
    .err()
    .expect("expected an error");
  assert!(
    format!("{err:#}").contains("listing data at "),
    "fetch_all abort must carry 'listing <path> at <url>': {err:#}"
  );
}

// covers: DIST-055
#[test]
fn list_mirror_fetch_listings_aborts_on_any_failure() {
  // mirror/list.rs:102-113: fetch_listings (string listings) likewise visits
  // every mirror and aborts on failure with the same context.
  let m1 = TestServer::start(routes(vec![("/dir/", Reply::ok("<a href=\"x/\">x</a>"))]), Reply::not_found());
  let m2 = TestServer::start(routes(vec![]), Reply::server_error());
  let http = http_client_no_cache(1);
  let mirrors = ListMirror::new(vec![
    Box::new(HttpMirror::live(m1.base().to_string(), Arc::clone(&http))),
    Box::new(HttpMirror::live(m2.base().to_string(), Arc::clone(&http))),
  ]);
  let err = mirrors
    .fetch_listings("dir/", |_m, _text| Ok(()))
    .err()
    .expect("expected an error");
  assert!(format!("{err:#}").contains("listing dir/ at "), "fetch_listings abort must carry context: {err:#}");
}

// ---------------------------------------------------------------------------
// ReleaseScraper / WalkStrategy through crafted directory HTML
// (DIST-060, DIST-061, DIST-062, DIST-063, DIST-069, DIST-024, DIST-025,
//  DIST-027)
// ---------------------------------------------------------------------------

/// An Apache-style directory index listing the given directory names as
/// `<a href="<name>/">`, interleaved with noise the parser must ignore.
fn dir_index(names: &[&str]) -> String {
  let mut html = String::from("<html><head><title>Index</title></head><body><pre>\n");
  html.push_str("<a href=\"../\">Parent Directory</a>\n");
  for n in names {
    html.push_str(&format!("<a href=\"{n}/\">{n}/</a>   01-Jan-2024 00:00   -\n"));
  }
  // A plain-file href (no trailing slash) the parser must skip.
  html.push_str("<a href=\"README\">README</a>\n");
  html.push_str("</pre></body></html>\n");
  html
}

// covers: DIST-060, DIST-061
#[test]
fn deb_codename_walk_filters_aliases_dedups_and_sorts() {
  // listing_parse extracts href dir names, accept() rejects lifecycle aliases
  // and uppercase, and the result is dedup/sorted (listing.rs:92-116, 371-387).
  // codename_validate confirms dists/<name>/Release exists. The two genuine
  // codenames survive; aliases (stable/testing/oldstable/sid), uppercase
  // (Debian), and a non-existent codename (its Release HEAD 404s) drop.
  let listing = dir_index(&[
    "stable",
    "testing",
    "oldstable",
    "sid",
    "Debian",
    "bookworm",
    "trixie",
    "ghost",
  ]);
  let server = TestServer::start(
    routes(vec![
      ("/dists/", Reply::ok(listing)),
      ("/dists/bookworm/Release", Reply::ok("Suite: stable")),
      ("/dists/trixie/Release", Reply::ok("Suite: testing")),
      ("/dists/ghost/Release", Reply::not_found()),
    ]),
    Reply::not_found(),
  );
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let table = ReleaseScraper::new(&mirrors)
    .collect(&WalkStrategy::DebCodename)
    .expect("deb walk");

  assert_eq!(table.header, vec!["Codename", "Mirror"]);
  let codenames: Vec<&str> = table.rows.iter().map(|r| r[0].as_str()).collect();
  assert_eq!(codenames, vec!["bookworm", "trixie"], "only real lowercase codenames with a Release survive, sorted");
}

// covers: DIST-024
#[test]
fn deb_codename_walk_bails_when_every_mirror_empty() {
  // listing.rs:221-223: if no codename survives across all mirrors, the walk
  // bails. An empty dists/ listing yields no candidates.
  let server = TestServer::start(routes(vec![("/dists/", Reply::ok(dir_index(&[])))]), Reply::not_found());
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let err = ReleaseScraper::new(&mirrors)
    .collect(&WalkStrategy::DebCodename)
    .err()
    .expect("expected an error");
  assert_eq!(err.to_string(), "no Debian-family releases discovered — every mirror returned an empty listing");
}

// covers: DIST-025, DIST-063
#[test]
fn rhel_major_walk_numeric_only_first_mirror_and_bail_when_none() {
  // RhelMajor accepts only all-digit names (listing.rs:117-119) and scrapes the
  // FIRST mirror only (listing.rs:246-250). Non-numeric dirs drop; the rows are
  // sorted+deduped majors against the supplied short label.
  let listing = dir_index(&["8", "9", "kickstart", "SIGS", "9"]);
  let server = TestServer::start(routes(vec![("/", Reply::ok(listing))]), Reply::not_found());
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let strat = WalkStrategy::RhelMajor {
    mirror_short: "repo.example.org",
  };
  let table = ReleaseScraper::new(&mirrors).collect(&strat).expect("rhel walk");
  assert_eq!(table.header, vec!["Release", "Mirror"]);
  let majors: Vec<&str> = table.rows.iter().map(|r| r[0].as_str()).collect();
  assert_eq!(majors, vec!["8", "9"], "only numeric majors, sorted + deduped");
  assert!(table.rows.iter().all(|r| r[1] == "repo.example.org"), "every row carries the short label");

  // No numeric dirs → bail '<url>: no major-release directories discovered'.
  let empty = TestServer::start(routes(vec![("/", Reply::ok(dir_index(&["kickstart", "SIGS"])))]), Reply::not_found());
  let mirrors_empty =
    ListMirror::primary(Box::new(HttpMirror::live(empty.base().to_string(), http_client_no_cache(1))));
  let err = ReleaseScraper::new(&mirrors_empty)
    .collect(&strat)
    .err()
    .expect("expected an error");
  assert_eq!(err.to_string(), format!("{}: no major-release directories discovered", empty.base()));
}

// covers: DIST-069
#[test]
fn rpm_numeric_walk_skips_non_numeric_and_appends_extras() {
  // RpmNumeric parses numeric dir names, skips non-numeric (continue at
  // listing.rs:296-305), then appends the `extras` rows unconditionally
  // (listing.rs:312-314).
  let listing = dir_index(&["40", "41", "test", "42"]);
  let server = TestServer::start(routes(vec![("/", Reply::ok(listing))]), Reply::not_found());
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let strat = WalkStrategy::RpmNumeric {
    extras: &[("rawhide", "dl.example.org")],
  };
  let table = ReleaseScraper::new(&mirrors).collect(&strat).expect("rpm numeric walk");
  let releases: Vec<&str> = table.rows.iter().map(|r| r[0].as_str()).collect();
  assert_eq!(releases, vec!["40", "41", "42", "rawhide"], "numeric sorted, then rawhide appended; 'test' skipped");
  let rawhide_row = table.rows.iter().find(|r| r[0] == "rawhide").unwrap();
  assert_eq!(rawhide_row[1], "dl.example.org", "extras carry their own mirror label");
}

// covers: DIST-062
#[test]
fn apk_version_walk_sorts_numerically_not_lexically() {
  // ApkVersion accept() takes vX.Y and edge; sort_key orders numerically so
  // v3.9 precedes v3.10 and v3.20 (listing.rs:120-147). apk_validate requires
  // both main and community APKINDEX for x86_64.
  let listing = dir_index(&["v3.2", "v3.9", "v3.10", "v3.20", "edge", "latest-stable"]);
  let mut rt = routes(vec![("/", Reply::ok(listing))]);
  for v in ["v3.2", "v3.9", "v3.10", "v3.20", "edge"] {
    rt.insert(format!("/{v}/main/x86_64/APKINDEX.tar.gz"), Reply::ok("idx"));
    rt.insert(format!("/{v}/community/x86_64/APKINDEX.tar.gz"), Reply::ok("idx"));
  }
  let server = TestServer::start(rt, Reply::not_found());
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let table = ReleaseScraper::new(&mirrors)
    .collect(&WalkStrategy::ApkVersion)
    .expect("apk walk");
  assert_eq!(table.header, vec!["Release", "Type", "Mirror"]);

  // Versioned rows first in numeric order, then edge (rolling) last.
  let releases: Vec<&str> = table.rows.iter().map(|r| r[0].as_str()).collect();
  assert_eq!(releases, vec!["v3.2", "v3.9", "v3.10", "v3.20", "edge"], "v3.9 must precede v3.10/v3.20 numerically");
  // 'latest-stable' is not a vX.Y nor edge → rejected by accept().
  assert!(!releases.contains(&"latest-stable"), "non-version dirs must be rejected");

  // Type column: versioned → stable, edge → rolling.
  let edge_row = table.rows.iter().find(|r| r[0] == "edge").unwrap();
  assert_eq!(edge_row[1], "rolling");
  let v32_row = table.rows.iter().find(|r| r[0] == "v3.2").unwrap();
  assert_eq!(v32_row[1], "stable");
}

// covers: DIST-027
#[test]
fn apk_version_walk_drops_release_missing_community_index() {
  // apk_validate (listing.rs:412-428) requires BOTH main and community
  // APKINDEX; a release serving main but not community is silently dropped.
  let listing = dir_index(&["v3.21", "v3.22"]);
  let mut rt = routes(vec![("/", Reply::ok(listing))]);
  // v3.21 has both → kept.
  rt.insert("/v3.21/main/x86_64/APKINDEX.tar.gz".into(), Reply::ok("idx"));
  rt.insert("/v3.21/community/x86_64/APKINDEX.tar.gz".into(), Reply::ok("idx"));
  // v3.22 has main but NOT community → dropped (community 404s).
  rt.insert("/v3.22/main/x86_64/APKINDEX.tar.gz".into(), Reply::ok("idx"));
  rt.insert("/v3.22/community/x86_64/APKINDEX.tar.gz".into(), Reply::not_found());
  let server = TestServer::start(rt, Reply::not_found());
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let table = ReleaseScraper::new(&mirrors)
    .collect(&WalkStrategy::ApkVersion)
    .expect("apk walk");
  let releases: Vec<&str> = table.rows.iter().map(|r| r[0].as_str()).collect();
  assert_eq!(releases, vec!["v3.21"], "the release missing a community index must be dropped silently");
}

// covers: DIST-068
#[test]
fn deb_walk_fails_when_listing_unreachable() {
  // A Walk listing requires the network; when the dists/ listing fetch fails
  // (server 500), collect_deb propagates the error with the 'listing dists/ at
  // <url>' context (listing.rs:203-205).
  let server = TestServer::start(routes(vec![("/dists/", Reply::server_error())]), Reply::server_error());
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let err = ReleaseScraper::new(&mirrors)
    .collect(&WalkStrategy::DebCodename)
    .err()
    .expect("expected an error");
  assert!(format!("{err:#}").contains("listing dists/ at "), "unreachable listing must carry context: {err:#}");
}

// covers: DIST-026
#[test]
fn deb_walk_propagates_head_failure_during_validation() {
  // codename_validate uses exists() (a HEAD); a non-404 HEAD error is logged via
  // RunVoice::warn (verbose-gated) and then propagated as Err out of the walk
  // (listing.rs:395-404). The listing succeeds but the Release HEAD 500s.
  flatroot::ui::RunVoice::verbose_set(true);
  let server = TestServer::start(
    routes(vec![
      ("/dists/", Reply::ok(dir_index(&["bookworm"]))),
      ("/dists/bookworm/Release", Reply::server_error()),
    ]),
    Reply::server_error(),
  );
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let err = ReleaseScraper::new(&mirrors)
    .collect(&WalkStrategy::DebCodename)
    .err()
    .expect("expected an error");
  assert!(format!("{err:#}").contains("exists "), "a HEAD failure during validation must propagate: {err:#}");
}

// ---------------------------------------------------------------------------
// release-list row rendering: extra cells lacking a header column are dropped
// (DIST-022)
// ---------------------------------------------------------------------------

/// The exact rendering `commands/release.rs`'s private `ReleaseRow` applies to a
/// single release, reproduced here over an arbitrary `(header, cells)` pair so
/// the drop-extra-cell branch — unreachable through the real CLI because every
/// `TableRelease` the scrapers and static tables build has `cells.len() ==
/// header.len()` — can be exercised with a deliberately ragged row.
///
/// `header_normalize` (release.rs:63-66): "Release" → "name", everything else
/// ASCII-lowercased.
fn header_normalize(header: &str) -> String {
  let lower = header.to_ascii_lowercase();
  if lower == "release" { "name".to_string() } else { lower }
}

/// Plain rendering, the Shout projection of `ReleaseRow`'s `Serialize`
/// (release.rs): the `release list` call site renders a one-row array under the
/// `release.list` command-path scope, so a header-less cell is dropped exactly
/// as the serialized form drops it — it never becomes a key in the first place.
fn release_row_plain(header: &[&'static str], cells: &[String]) -> String {
  let value = serde_json::Value::Array(vec![release_row_json(header, cells)]);
  flatroot::internal::shout::Document::from_value(&value, "release.list")
    .render()
    .expect("render plain")
}

/// JSON rendering, mirroring `ReleaseRow::serialize` (release.rs:91-98): each
/// cell is keyed by its normalized header in a BTreeMap (sorted keys); a cell
/// with no header column (`header.get(i)` is `None`) is dropped via the
/// `if let Some(h)` gate.
fn release_row_json(header: &[&'static str], cells: &[String]) -> serde_json::Value {
  use std::collections::BTreeMap;
  let mut obj: BTreeMap<String, &str> = BTreeMap::new();
  for (i, cell) in cells.iter().enumerate() {
    if let Some(h) = header.get(i) {
      obj.insert(header_normalize(h), cell.as_str());
    }
  }
  serde_json::to_value(&obj).expect("serialize row to json")
}

// covers: DIST-022
#[test]
fn release_row_drops_cells_lacking_a_header_column() {
  // A row with MORE cells than the header has — the case no real scrape or
  // static table produces, so the drop-extra-cell branch (release.rs:80 plain
  // `None => continue`, release.rs:95 json `if let Some(h)`) is reached only by
  // a deliberately ragged row. A two-column header ("Release", "Mirror") meets a
  // four-cell row; the third and fourth cells have no column and must be dropped
  // from BOTH renderings, while "Release" still normalizes to "name".
  let header: Vec<&'static str> = vec!["Release", "Mirror"];
  let cells: Vec<String> = vec![
    "bookworm".into(),
    "deb.debian.org".into(),
    "stray-extra".into(),
    "another-orphan".into(),
  ];

  // Plain: exactly the two labelled cells survive, the two header-less cells are
  // dropped, and "Release" is normalized to "name" (others lowercased).
  let plain = release_row_plain(&header, &cells);
  let lines: Vec<&str> = plain.lines().collect();
  assert_eq!(
    lines,
    vec!["release.list.0.mirror=deb.debian.org", "release.list.0.name=bookworm"],
    "plain must keep only header-backed cells (sorted), dropping the two extras"
  );
  assert!(!plain.contains("stray-extra"), "the header-less 3rd cell must not appear in plain output: {plain}");
  assert!(!plain.contains("another-orphan"), "the header-less 4th cell must not appear in plain output: {plain}");

  // JSON: the same two keys, no key for the dropped extras, "Release" → "name".
  let json = release_row_json(&header, &cells);
  let obj = json.as_object().expect("row serializes to a json object");
  assert_eq!(obj.len(), 2, "json must carry exactly the two header-backed cells: {json}");
  assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("bookworm"), "Release→name keyed cell");
  assert_eq!(obj.get("mirror").and_then(|v| v.as_str()), Some("deb.debian.org"), "Mirror→mirror keyed cell");
  let serialized = json.to_string();
  assert!(!serialized.contains("stray-extra"), "the header-less 3rd cell must not appear in json: {serialized}");
  assert!(!serialized.contains("another-orphan"), "the header-less 4th cell must not appear in json: {serialized}");

  // The two views agree on which cells survived: every plain leaf has a json key.
  assert_eq!(obj.len(), lines.len(), "plain and json must surface the same set of header-backed cells");
}

// covers: DIST-064
#[test]
fn deb_walk_mirror_label_is_the_host_only() {
  // mirror_label strips the scheme and keeps only the host (listing.rs:436-439).
  // The local server base is http://127.0.0.1:<port>; the Mirror column must be
  // exactly that host:port (everything after the first '/' of the authority
  // dropped — here there is no path).
  let listing = dir_index(&["bookworm"]);
  let server = TestServer::start(
    routes(vec![
      ("/dists/", Reply::ok(listing)),
      ("/dists/bookworm/Release", Reply::ok("Suite: stable")),
    ]),
    Reply::not_found(),
  );
  let mirrors = ListMirror::primary(Box::new(HttpMirror::live(server.base().to_string(), http_client_no_cache(1))));
  let table = ReleaseScraper::new(&mirrors)
    .collect(&WalkStrategy::DebCodename)
    .expect("deb walk");
  let label = table
    .rows
    .iter()
    .find(|r| r[0] == "bookworm")
    .map(|r| r[1].clone())
    .unwrap();
  // server.addr is the bare host:port the label must reduce to.
  assert_eq!(label, server.addr.to_string(), "mirror label must be the bare host:port, scheme stripped");
}
