//! Black-box CLI tests for `flatroot ... search` (package and library modes)
//! against live distribution mirrors. These cover the output contracts (plain
//! dotted KEY=VALUE shape, json `[]` for empty, plain↔json bijection), the
//! glob semantics (case-insensitivity, union/dedup, ordering), the per-distro
//! library-encoding canonicalization, env-var fallbacks, and `--format`
//! validation.
//!
//! Every invocation isolates its cache via a fresh `FLATROOT_CACHE_HOME`
//! TempDir kept alive until after the call. Network-backed tests panic with the
//! captured stderr on a non-zero exit.

#![allow(dead_code)]

use std::collections::BTreeMap;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run `flatroot --from <from> [extra...] search <args...>` with an isolated
/// cache and return stdout (panicking with stderr on non-zero exit).
fn search_ok(from: &str, args: &[&str]) -> String {
  let cache = TempDir::new().unwrap();
  let mut full: Vec<&str> = vec!["--from", from, "search"];
  full.extend_from_slice(args);
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&full)
    .output()
    .unwrap();
  assert!(
    out.status.success(),
    "search {:?} (from {}) failed:\nstderr:\n{}",
    args,
    from,
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Run a search with one extra environment variable set, returning stdout.
fn search_ok_env(from: &str, env_key: &str, env_val: &str, args: &[&str]) -> String {
  let cache = TempDir::new().unwrap();
  let mut full: Vec<&str> = vec!["--from", from, "search"];
  full.extend_from_slice(args);
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env(env_key, env_val)
    .args(&full)
    .output()
    .unwrap();
  assert!(
    out.status.success(),
    "search {:?} (env {}={}) failed:\nstderr:\n{}",
    args,
    env_key,
    env_val,
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Parse the plain dotted `search.<i>.<key>=<value>` stream into one map per
/// indexed record, keyed by the column name. Records arrive in emission order.
fn parse_plain_records(stdout: &str) -> Vec<BTreeMap<String, String>> {
  let mut records: Vec<BTreeMap<String, String>> = Vec::new();
  for line in stdout.lines() {
    let rest = match line.strip_prefix("search.") {
      Some(r) => r,
      None => continue,
    };
    // rest == "<i>.<key>=<value>"
    let (idx_str, kv) = match rest.split_once('.') {
      Some(p) => p,
      None => continue,
    };
    let idx: usize = match idx_str.parse() {
      Ok(n) => n,
      Err(_) => continue,
    };
    let (key, value) = match kv.split_once('=') {
      Some(p) => p,
      None => continue,
    };
    while records.len() <= idx {
      records.push(BTreeMap::new());
    }
    records[idx].insert(key.to_string(), value.to_string());
  }
  records
}

// ---------------------------------------------------------------------------
// search package — single literal, plain shape, newest-per-name (IDX-001)
// ---------------------------------------------------------------------------

// covers: IDX-001
#[test]
fn search_package_single_literal_plain_shape() {
  // A literal `bash` against debian yields exactly one entry (newest version
  // per name), rendered as the plain `description`/`name`/`version` triple.
  let stdout = search_ok("debian:bookworm", &["bash"]);
  let records = parse_plain_records(&stdout);
  // Exactly one record for the literal package name.
  let bash: Vec<&BTreeMap<String, String>> = records
    .iter()
    .filter(|r| r.get("name").map(|s| s == "bash").unwrap_or(false))
    .collect();
  assert_eq!(bash.len(), 1, "literal `bash` must yield exactly one entry (newest per name):\n{}", stdout);
  let rec = bash[0];
  assert!(rec.contains_key("description"), "package record must carry description: {:?}", rec);
  assert!(rec.contains_key("version"), "package record must carry version: {:?}", rec);
  // Package mode must NOT carry a library field.
  assert!(!rec.contains_key("library"), "package-mode record must not carry library: {:?}", rec);
  // No record should repeat the name (newest-per-name dedup).
  assert_eq!(records.len(), 1, "a literal match must produce a single record:\n{}", stdout);
}

// ---------------------------------------------------------------------------
// search package — glob is case-insensitive (IDX-003)
// ---------------------------------------------------------------------------

// covers: IDX-003
#[test]
fn search_package_glob_case_insensitive() {
  // Searching the uppercase pattern `BASH` must still match the lowercase
  // package `bash` (case-insensitive glob), and the output carries the real
  // lowercase name.
  let stdout = search_ok("debian:bookworm", &["BASH"]);
  let records = parse_plain_records(&stdout);
  assert!(
    records
      .iter()
      .any(|r| r.get("name").map(|s| s == "bash").unwrap_or(false)),
    "uppercase pattern BASH must match the lowercase package bash:\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// search package — empty match: zero bytes plain / '[]' json (IDX-006)
// ---------------------------------------------------------------------------

// covers: IDX-006
#[test]
fn search_package_empty_match_json_is_empty_array() {
  // `--format json` on a no-match search emits exactly "[]\n" and exits 0.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "search",
      "--format",
      "json",
      "zzz-no-such-pkg-xyz-*",
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "empty json search must exit 0:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert_eq!(stdout, "[]\n", "empty package search in json must be exactly '[]\\n', got {:?}", stdout);
}

// covers: IDX-006
#[test]
fn search_package_empty_match_plain_is_zero_bytes() {
  // Plain encoding emits zero bytes for an empty result.
  let stdout = search_ok("debian:bookworm", &["zzz-no-such-pkg-xyz-999"]);
  assert!(stdout.is_empty(), "empty package search in plain must be zero bytes, got {:?}", stdout);
}

// ---------------------------------------------------------------------------
// search package — plain↔json bijection for non-empty result (IDX-007)
// ---------------------------------------------------------------------------

// covers: IDX-007
#[test]
fn search_package_plain_json_bijection() {
  // Package JSON is an array of {description, name, version}. The set of names
  // (and their versions) must be identical in plain and json.
  let plain = search_ok("debian:bookworm", &["libssl*"]);
  let json = search_ok("debian:bookworm", &["--format", "json", "libssl*"]);

  let parsed: serde_json::Value = serde_json::from_str(&json).expect("package search json must parse");
  let arr = parsed
    .as_array()
    .expect("package search json must be a top-level array");
  assert!(!arr.is_empty(), "libssl* must match at least one package");

  // Every json entry has exactly the three package keys (no library key).
  for e in arr {
    let obj = e.as_object().expect("entries are objects");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["description", "name", "version"], "package json entry keys: {}", e);
  }

  // (name, version) pairs identical across encodings.
  let plain_records = parse_plain_records(&plain);
  let mut plain_pairs: Vec<(String, String)> = plain_records
    .iter()
    .map(|r| (r["name"].clone(), r["version"].clone()))
    .collect();
  let mut json_pairs: Vec<(String, String)> = arr
    .iter()
    .map(|e| (e["name"].as_str().unwrap().to_string(), e["version"].as_str().unwrap().to_string()))
    .collect();
  plain_pairs.sort();
  json_pairs.sort();
  assert_eq!(plain_pairs, json_pairs, "plain and json must carry identical (name, version) data");
}

// ---------------------------------------------------------------------------
// search package — invalid --format rejected by clap (IDX-008)
// ---------------------------------------------------------------------------

// covers: IDX-008
#[test]
fn search_invalid_format_rejected() {
  // `--format=xml` is not a valid OutputFormat (plain|json) → clap rejects it.
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "search", "--format=xml", "bash"])
    .assert()
    .failure()
    .stderr(contains("invalid value 'xml'"));
}

// ---------------------------------------------------------------------------
// search package — FLATROOT_ARG_SEARCH_FORMAT env fallback; flag overrides (IDX-009)
// ---------------------------------------------------------------------------

// covers: IDX-009
#[test]
fn search_format_env_fallback_and_flag_override() {
  // FLATROOT_ARG_SEARCH_FORMAT=json makes search emit json with no --format.
  let json = search_ok_env("debian:bookworm", "FLATROOT_ARG_SEARCH_FORMAT", "json", &["bash"]);
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("env-driven json must parse");
  assert!(parsed.is_array(), "FLATROOT_ARG_SEARCH_FORMAT=json must produce a json array:\n{}", json);

  // An explicit --format plain flag overrides the json env value.
  let plain = search_ok_env("debian:bookworm", "FLATROOT_ARG_SEARCH_FORMAT", "json", &["--format", "plain", "bash"]);
  assert!(plain.lines().any(|l| l.starts_with("search.")), "--format plain must override the json env var:\n{}", plain);
  assert!(serde_json::from_str::<serde_json::Value>(&plain).is_err(), "flag-overridden output must not be json");
}

// ---------------------------------------------------------------------------
// search --type env fallback FLATROOT_ARG_SEARCH_TYPE → library (IDX-010)
// ---------------------------------------------------------------------------

// covers: IDX-010
#[test]
fn search_type_env_fallback_routes_to_library() {
  // FLATROOT_ARG_SEARCH_TYPE=library routes the run to the library matcher:
  // the output carries `library=` lines (absent in package mode).
  let stdout = search_ok_env("debian:bookworm", "FLATROOT_ARG_SEARCH_TYPE", "library", &["libssl.so.3"]);
  let records = parse_plain_records(&stdout);
  assert!(
    records.iter().any(|r| r.contains_key("library")),
    "FLATROOT_ARG_SEARCH_TYPE=library must produce library-field records:\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// search --from env fallback drives the source (IDX-064)
// ---------------------------------------------------------------------------

// covers: IDX-064
#[test]
fn search_from_env_drives_real_search() {
  // FLATROOT_ARG_FROM supplies the source with no --from flag; a real search
  // for bash against the env-supplied debian source returns the bash package.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env("FLATROOT_ARG_FROM", "debian:bookworm")
    .args(["search", "bash"])
    .output()
    .unwrap();
  assert!(out.status.success(), "env --from search failed:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(
    parse_plain_records(&stdout)
      .iter()
      .any(|r| r.get("name").map(|s| s == "bash").unwrap_or(false)),
    "env-supplied --from must drive a real search returning bash:\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// search --arch builds/consults a per-arch index (IDX-065)
// ---------------------------------------------------------------------------

// covers: IDX-065
#[test]
fn search_arch_consults_per_arch_catalogue() {
  // `--arch aarch64 search 'libssl*'` builds a per-arch index (the framing on
  // stderr names the aarch64 arch) and returns aarch64-catalogue results.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "--arch", "aarch64", "search", "libssl*"])
    .output()
    .unwrap();
  assert!(out.status.success(), "aarch64 search failed:\n{}", String::from_utf8_lossy(&out.stderr));
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("(aarch64)"), "per-arch search framing must name the aarch64 arch:\n{}", stderr);
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(
    parse_plain_records(&stdout)
      .iter()
      .any(|r| r.get("name").map(|s| s.starts_with("libssl")).unwrap_or(false)),
    "aarch64 catalogue must still carry a libssl* package:\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// Packages::glob via CLI — newest-per-name single row for libc6 (IDX-061 CLI)
// ---------------------------------------------------------------------------

// covers: IDX-061
#[test]
fn search_libc6_yields_single_newest_row() {
  // `search libc6` (literal) collapses to a single row at the highest version,
  // proving the ROW_NUMBER PARTITION rn=1 newest-per-name behavior end to end.
  let stdout = search_ok("debian:bookworm", &["libc6"]);
  let records = parse_plain_records(&stdout);
  let libc6: Vec<&BTreeMap<String, String>> = records
    .iter()
    .filter(|r| r.get("name").map(|s| s == "libc6").unwrap_or(false))
    .collect();
  assert_eq!(libc6.len(), 1, "`search libc6` must yield a single newest row:\n{}", stdout);
}

// ---------------------------------------------------------------------------
// search library — deb-family path-index leg (debian + ubuntu) (IDX-013)
// ---------------------------------------------------------------------------

/// Assert a library glob resolves to a (library, package) pair in plain output.
fn assert_library_pair(from: &str, pattern: &str, expected_library: &str, expected_package: &str) {
  let stdout = search_ok(from, &["--type=library", pattern]);
  let records = parse_plain_records(&stdout);
  let found = records.iter().any(|r| {
    r.get("library").map(|l| l == expected_library).unwrap_or(false)
      && r.get("name").map(|n| n == expected_package).unwrap_or(false)
  });
  assert!(
    found,
    "expected library={} → package={} for {} {}:\n{}",
    expected_library, expected_package, from, pattern, stdout
  );
}

// covers: IDX-013
#[test]
fn search_library_debian_and_ubuntu_path_index() {
  // Both deb-family distros record sonames only on disk (Contents index), not in
  // the provides table — the path-index leg supplies the soname. `libc.so.6` is
  // owned by `libc6` on both debian and ubuntu (a stable name, unlike the
  // t64-renamed libssl package on noble), so it isolates the deb path-index leg
  // across the family.
  assert_library_pair("debian:bookworm", "libc.so.6", "libc.so.6", "libc6");
  assert_library_pair("ubuntu:noble", "libc.so.6", "libc.so.6", "libc6");
}

// ---------------------------------------------------------------------------
// search library — RPM mangled provides, trailing-* (centos/alma/rocky/opensuse) (IDX-015)
// ---------------------------------------------------------------------------

// covers: IDX-015
#[test]
fn search_library_rpm_family_mangled_provides() {
  // RPM provides carry `libc.so.6()(64bit)`; the trailing `*` lets SQL GLOB
  // reach the mangled raw text, and canonicalize strips the `()(64bit)` suffix
  // so the library field is the bare `libc.so.6`. Run across every rpm distro
  // not already covered by the fedora case in cli_network.rs. The accepted
  // release handles are alma:9 / rocky:9 / centos:stream9 / opensuse:tumbleweed.
  for from in ["centos:stream9", "alma:9", "rocky:9", "opensuse:tumbleweed"] {
    assert_library_pair(from, "libc.so.6*", "libc.so.6", "glibc");
  }
}

// ---------------------------------------------------------------------------
// search library — Arch =-decorated provides (arch + cachyos) (IDX-016)
// ---------------------------------------------------------------------------

// covers: IDX-016
#[test]
fn search_library_pacman_family_eq_decorated_provides() {
  // Pacman provides carry `libgomp.so=1-64`; everything from `=` is stripped so
  // the library field is `libgomp.so`, owned by the standalone `libgomp`
  // package. Cover both pacman-family distros.
  for from in ["arch:rolling", "cachyos:rolling"] {
    assert_library_pair(from, "libgomp*", "libgomp.so", "libgomp");
  }
}

// ---------------------------------------------------------------------------
// search library — non-library-shaped provides filtered out (IDX-017)
// ---------------------------------------------------------------------------

// covers: IDX-017
#[test]
fn search_library_filters_out_non_library_provides() {
  // `awk` is a virtual capability several packages provide, but it is not
  // library-shaped (no .so/.so./.a). A library search for it yields zero entries.
  let stdout = search_ok("debian:bookworm", &["--type=library", "awk"]);
  assert!(
    stdout.is_empty(),
    "library search for the virtual `awk` must yield zero entries (is_library_shape rejects it):\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// search library — dedup same (package, library) across both legs (IDX-018)
// ---------------------------------------------------------------------------

// covers: IDX-018
#[test]
fn search_library_dedups_pair_across_legs() {
  // On Arch, openssl both advertises `libssl.so=3-64` in its provides AND ships
  // the `usr/lib/libssl.so` dev symlink recorded in the path index. With a
  // trailing-`*` pattern both legs match and canonicalize to the same
  // (openssl, libssl.so) pair, which the final sort+dedup must collapse to one.
  let stdout = search_ok("arch:rolling", &["--type=library", "libssl.so*"]);
  let records = parse_plain_records(&stdout);
  let openssl_libssl: usize = records
    .iter()
    .filter(|r| {
      r.get("library").map(|l| l == "libssl.so").unwrap_or(false)
        && r.get("name").map(|n| n == "openssl").unwrap_or(false)
    })
    .count();
  assert_eq!(
    openssl_libssl, 1,
    "the (openssl, libssl.so) pair must appear exactly once after cross-leg sort+dedup:\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// search library — multi-pattern union, ordered by (library, name), dedup (IDX-020)
// ---------------------------------------------------------------------------

// covers: IDX-020
#[test]
fn search_library_multi_pattern_ordered_and_deduped() {
  // Two patterns union into one listing ordered by (library, name) with each
  // pair once. The libcrypto.so.3 entry must sort before libssl.so.3.
  let stdout = search_ok("debian:bookworm", &["--type=library", "libssl.so.3", "libcrypto.so.3"]);
  let records = parse_plain_records(&stdout);

  // Collect (library, name) in emission order; emission order IS the sorted order.
  let pairs: Vec<(String, String)> = records
    .iter()
    .filter_map(|r| match (r.get("library"), r.get("name")) {
      (Some(l), Some(n)) => Some((l.clone(), n.clone())),
      _ => None,
    })
    .collect();
  assert!(!pairs.is_empty(), "multi-pattern library search must yield entries:\n{}", stdout);

  // Ordering contract: the emitted sequence equals its own (library, name) sort.
  let mut sorted = pairs.clone();
  sorted.sort();
  assert_eq!(pairs, sorted, "library results must be emitted sorted by (library, name):\n{}", stdout);

  // Each pair appears once (dedup).
  let mut unique = pairs.clone();
  unique.dedup();
  assert_eq!(unique.len(), pairs.len(), "every (library, name) pair must appear exactly once:\n{}", stdout);

  // Both requested sonames are present.
  assert!(pairs.iter().any(|(l, _)| l == "libssl.so.3"), "libssl.so.3 missing:\n{}", stdout);
  assert!(pairs.iter().any(|(l, _)| l == "libcrypto.so.3"), "libcrypto.so.3 missing:\n{}", stdout);
}

// ---------------------------------------------------------------------------
// search library — path-index leg is case-INSENSITIVE (IDX-021)
// ---------------------------------------------------------------------------

// covers: IDX-021
#[test]
fn search_library_path_index_is_case_insensitive() {
  // The path-index (shipped-files) leg matches case-insensitively, the same
  // dialect the catalogue's `glob_bash` SQL function applies (FR-MATCH-001),
  // so an uppercase soname pattern matches the recorded `libssl.so.3`.
  let stdout = search_ok("debian:bookworm", &["--type=library", "LIBSSL.SO.3"]);
  assert!(
    stdout.contains("libssl.so.3"),
    "uppercase soname must match the case-insensitive path-index leg and resolve libssl.so.3:\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// search library — missing path index contributes nothing, no error (IDX-022)
// ---------------------------------------------------------------------------

// covers: IDX-022
#[test]
fn search_library_alpine_missing_path_index_provides_only() {
  // Alpine publishes no on-disk file-ownership index, so PathIndex::open returns
  // Ok(None) and only the provides leg (under the `so:` namespace) contributes.
  // The run must succeed (no error) and resolve musl's soname.
  assert_library_pair("alpine:edge", "libc.musl-x86_64.so.1", "libc.musl-x86_64.so.1", "musl");
}

// ---------------------------------------------------------------------------
// search library — empty match: zero bytes plain / '[]' json (IDX-023)
// ---------------------------------------------------------------------------

// covers: IDX-023
#[test]
fn search_library_empty_match_formats() {
  // A no-match library pattern: plain emits zero bytes; json emits exactly "[]".
  let plain = search_ok("debian:bookworm", &["--type=library", "libnothere-xyz*"]);
  assert!(plain.is_empty(), "empty library search plain must be zero bytes, got {:?}", plain);

  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "search",
      "--type=library",
      "--format",
      "json",
      "libnothere-xyz*",
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "empty library json search must exit 0:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert_eq!(stdout, "[]\n", "empty library search in json must be exactly '[]\\n', got {:?}", stdout);
}

// ---------------------------------------------------------------------------
// search library — plain↔json bijection includes the library field (IDX-024)
// ---------------------------------------------------------------------------

// covers: IDX-024
#[test]
fn search_library_plain_json_bijection_with_library_field() {
  // Library json entries carry exactly {description, library, name, version};
  // the (library, name, version) triples are identical in plain and json.
  let plain = search_ok("debian:bookworm", &["--type=library", "libssl.so.3"]);
  let json = search_ok("debian:bookworm", &["--format", "json", "--type=library", "libssl.so.3"]);

  let parsed: serde_json::Value = serde_json::from_str(&json).expect("library search json must parse");
  let arr = parsed
    .as_array()
    .expect("library search json must be a top-level array");
  assert!(!arr.is_empty(), "libssl.so.3 must resolve to at least one package");

  for e in arr {
    let obj = e.as_object().expect("entries are objects");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["description", "library", "name", "version"], "library json entry keys: {}", e);
  }

  let plain_records = parse_plain_records(&plain);
  let mut plain_triples: Vec<(String, String, String)> = plain_records
    .iter()
    .map(|r| (r["library"].clone(), r["name"].clone(), r["version"].clone()))
    .collect();
  let mut json_triples: Vec<(String, String, String)> = arr
    .iter()
    .map(|e| {
      (
        e["library"].as_str().unwrap().to_string(),
        e["name"].as_str().unwrap().to_string(),
        e["version"].as_str().unwrap().to_string(),
      )
    })
    .collect();
  plain_triples.sort();
  json_triples.sort();
  assert_eq!(
    plain_triples, json_triples,
    "plain and json must carry identical (library, name, version) data for the library search"
  );
}

// ---------------------------------------------------------------------------
// search library — one canonical form across distro encodings (IDX-066 CLI)
// ---------------------------------------------------------------------------

// covers: IDX-066
#[test]
fn search_library_canonical_form_across_families() {
  // The same pattern `libc.so.6*` run across a deb, an rpm, and a pacman distro
  // must, in each, surface the identical canonical library string `libc.so.6`
  // regardless of how that distro encodes the soname (deb on-disk file, rpm
  // `()(64bit)` provides, pacman `=`-decorated provides).
  for (from, pattern) in [
    ("debian:bookworm", "libc.so.6"),
    ("fedora:42", "libc.so.6*"),
    ("arch:rolling", "libc.so.6*"),
  ] {
    let stdout = search_ok(from, &["--type=library", pattern]);
    let records = parse_plain_records(&stdout);
    assert!(
      records
        .iter()
        .any(|r| r.get("library").map(|l| l == "libc.so.6").unwrap_or(false)),
      "{} must canonicalize its libc soname to the identical `libc.so.6`:\n{}",
      from,
      stdout
    );
  }
}
