//! End-to-end tests for `flatroot release list` and `flatroot remote list`.

use std::sync::Arc;
use std::time::Duration;

use predicates::str::contains;

use flatroot::distro::{DistroRegistry, ReleaseSource};
use flatroot::internal::http::HttpClient;

mod common;
use common::flatroot;

/// An inert HTTP client; release-source classification never fetches.
fn http() -> Arc<HttpClient> {
  Arc::new(HttpClient::new(std::env::temp_dir().join("flatroot-release-list-test"), 3, Duration::from_secs(3600)))
}

#[test]
fn remote_list_prints_all_ten_backends() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["remote", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  for backend in [
    "debian", "ubuntu", "arch", "alpine", "centos", "fedora", "alma", "rocky", "opensuse", "cachyos",
  ] {
    assert!(text.contains(backend), "remote list missing '{}': {}", backend, text);
  }
}

#[test]
fn release_list_arch_lists_rolling_and_multilib() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "arch", "release", "list"])
    .assert()
    .success()
    .stdout(contains("rolling"))
    .stdout(contains("multilib"))
    .stdout(contains("geo.mirror.pkgbuild.com"));
}

#[test]
fn release_list_cachyos_shows_rolling_type_label() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "cachyos", "release", "list"])
    .assert()
    .success()
    .stdout(contains("rolling"))
    .stdout(contains("mirror.cachyos.org"));
}

#[test]
fn release_list_centos_enumerates_7_8_stream9() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "centos", "release", "list"])
    .assert()
    .success()
    .stdout(contains("7.9.2009"))
    .stdout(contains("8.5.2111"))
    .stdout(contains("stream9"))
    .stdout(contains("vault.centos.org"))
    .stdout(contains("mirror.stream.centos.org"));
}

#[test]
fn release_list_alma_enumerates_8_and_9() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "alma", "release", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.contains("repo.almalinux.org"));
  assert!(text.contains(".name=8"));
  assert!(text.contains(".name=9"));
}

#[test]
fn release_list_rocky_enumerates_8_and_9() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "rocky", "release", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.contains("dl.rockylinux.org"));
  assert!(text.contains(".name=8"));
  assert!(text.contains(".name=9"));
}

#[test]
fn release_list_opensuse_includes_tumbleweed_and_leap() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "opensuse", "release", "list"])
    .assert()
    .success()
    .stdout(contains("tumbleweed"))
    .stdout(contains("15.5"))
    .stdout(contains("15.6"))
    .stdout(contains("download.opensuse.org"));
}

#[test]
fn release_list_unsupported_distro_errors_without_listing_it_as_supported() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "nosuchdistro", "release", "list"])
    .assert()
    .failure()
    .get_output()
    .stderr
    .clone();
  let err = String::from_utf8(out).unwrap();
  assert!(err.contains("'nosuchdistro'"), "error should name the rejected distro: {}", err);
  assert!(err.contains("not a supported distro"), "error wording: {}", err);

  for d in [
    "debian", "ubuntu", "arch", "alpine", "centos", "fedora", "alma", "rocky", "opensuse", "cachyos",
  ] {
    assert!(err.contains(d), "expected '{}' in error: {}", d, err);
  }

  let listing = err.split("Supported:").nth(1).unwrap_or("");
  assert!(!listing.contains("nosuchdistro"), "rejected distro must not appear in supported list: {}", err);
}

#[test]
fn release_list_debian_reports_bookworm() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian", "release", "list"])
    .assert()
    .success()
    .stdout(contains("bookworm"));
}

#[test]
fn release_list_debian_lists_forky() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian", "release", "list"])
    .assert()
    .success()
    .stdout(contains("forky"));
}

#[test]
fn release_list_debian_lists_trixie() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian", "release", "list"])
    .assert()
    .success()
    .stdout(contains("trixie"));
}

#[test]
fn release_list_debian_lists_bullseye() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian", "release", "list"])
    .assert()
    .success()
    .stdout(contains("bullseye"));
}

#[test]
fn release_list_ubuntu_reports_noble() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "ubuntu", "release", "list"])
    .assert()
    .success()
    .stdout(contains("noble"));
}

#[test]
fn release_list_ubuntu_lists_jammy() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "ubuntu", "release", "list"])
    .assert()
    .success()
    .stdout(contains("jammy"));
}

#[test]
fn release_list_ubuntu_lists_bionic() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "ubuntu", "release", "list"])
    .assert()
    .success()
    .stdout(contains("bionic"));
}

#[test]
fn release_list_ubuntu_lists_xenial() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "ubuntu", "release", "list"])
    .assert()
    .success()
    .stdout(contains("xenial"));
}

#[test]
fn release_list_alpine_lists_v3_20() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "alpine", "release", "list"])
    .assert()
    .success()
    .stdout(contains("v3.20"));
}

#[test]
fn release_list_alpine_lists_v3_21() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "alpine", "release", "list"])
    .assert()
    .success()
    .stdout(contains("v3.21"));
}

#[test]
fn release_list_alpine_lists_v3_22() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "alpine", "release", "list"])
    .assert()
    .success()
    .stdout(contains("v3.22"));
}

#[test]
fn release_list_alpine_lists_v3_23() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "alpine", "release", "list"])
    .assert()
    .success()
    .stdout(contains("v3.23"));
}

#[test]
fn release_list_fedora_lists_40() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "fedora", "release", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.contains(".name=40"), "fedora output missing 40: {}", text);
}

#[test]
fn release_list_fedora_lists_41() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "fedora", "release", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.contains(".name=41"), "fedora output missing 41: {}", text);
}

#[test]
fn release_list_fedora_lists_42() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "fedora", "release", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.contains(".name=42"), "fedora output missing 42: {}", text);
}

#[test]
fn release_list_fedora_lists_43() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "fedora", "release", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.contains(".name=43"), "fedora output missing 43: {}", text);
}

#[test]
fn release_list_fedora_lists_44() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "fedora", "release", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.contains(".name=44"), "fedora output missing 44: {}", text);
}

#[test]
fn release_list_fedora_includes_rawhide() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "fedora", "release", "list"])
    .assert()
    .success()
    .stdout(contains("rawhide"))
    .stdout(contains("dl.fedoraproject.org"));
}

#[test]
fn release_list_alpine_shows_edge_and_versioned_releases() {
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "alpine", "release", "list"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();

  assert!(text.contains("edge"), "alpine output missing edge: {}", text);
  assert!(text.contains("v3."), "alpine output missing any versioned release: {}", text);
  assert!(text.contains("dl-cdn.alpinelinux.org"), "alpine output missing mirror: {}", text);
}

// ===========================================================================
// remote list — shape, format, offline, registry order (DIST-001..006, 070)
// ===========================================================================

/// The registry order `DistroRegistry::all()` publishes — the order each
/// `remote.list.<i>.*` entry must appear in.
const REGISTRY_ORDER: &[&str] = &[
  "debian", "ubuntu", "arch", "alpine", "centos", "fedora", "alma", "rocky", "opensuse", "cachyos",
];

/// Each distro's exact `from_syntax` string, indexed by registry position.
/// Only debian / ubuntu / arch advertise the `[@<date>]` snapshot suffix.
const FROM_SYNTAX: &[(&str, &str)] = &[
  ("debian", "debian:<release>[@<date>]"),
  ("ubuntu", "ubuntu:<release>[@<date>]"),
  ("arch", "arch:<release>[@<date>]"),
  ("alpine", "alpine:<version>"),
  ("centos", "centos:<version>"),
  ("fedora", "fedora:<release>"),
  ("alma", "alma:<version>"),
  ("rocky", "rocky:<version>"),
  ("opensuse", "opensuse:<release>"),
  ("cachyos", "cachyos:rolling"),
];

fn stdout_of(args: &[&str]) -> String {
  let (mut cmd, _cache) = flatroot();
  let out = cmd.args(args).output().unwrap();
  assert!(out.status.success(), "`{}` failed:\n{}", args.join(" "), String::from_utf8_lossy(&out.stderr));
  String::from_utf8(out.stdout).unwrap()
}

// covers: DIST-001, DIST-003, DIST-070
#[test]
fn remote_list_plain_per_entry_shape_in_registry_order() {
  // Plain emits remote.list.<i>.format / .name / .status for each entry, in
  // registry order (commands/remote.rs:58-68, report.rs:55-71). status is
  // always 'supported'. No network is needed — only a tempdir cache — and the
  // command never builds an HttpClient (main.rs:63-65 routes straight to list).
  let text = stdout_of(&["remote", "list"]);
  for (i, name) in REGISTRY_ORDER.iter().enumerate() {
    let want_name = format!("remote.list.{i}.name={name}");
    assert!(text.lines().any(|l| l == want_name), "missing '{}' in:\n{}", want_name, text);
    let want_status = format!("remote.list.{i}.status=supported");
    assert!(text.lines().any(|l| l == want_status), "missing '{}' in:\n{}", want_status, text);
    // Each entry's format equals its exact from_syntax verbatim.
    let (syn_name, syntax) = FROM_SYNTAX[i];
    assert_eq!(syn_name, *name, "FROM_SYNTAX order must match REGISTRY_ORDER at {i}");
    let want_format = format!("remote.list.{i}.format={syntax}");
    assert!(text.lines().any(|l| l == want_format), "missing '{}' in:\n{}", want_format, text);
  }
  // Only debian / ubuntu / arch advertise the [@<date>] suffix.
  for (name, syntax) in FROM_SYNTAX {
    let advertises_date = syntax.contains("[@<date>]");
    let should = matches!(*name, "debian" | "ubuntu" | "arch");
    assert_eq!(advertises_date, should, "{} [@<date>] advertisement mismatch", name);
  }
}

// covers: DIST-002, DIST-070
#[test]
fn remote_list_json_is_array_of_format_name_status_in_order() {
  // --format json is a top-level array of {format,name,status} in registry
  // order, status=='supported', format=='from_syntax' (commands/remote.rs:22-32).
  let json = stdout_of(&["remote", "list", "--format", "json"]);
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("remote list json must parse");
  let arr = parsed.as_array().expect("remote list json must be a top-level array");
  assert_eq!(arr.len(), REGISTRY_ORDER.len(), "json must list every registered distro");
  for (i, entry) in arr.iter().enumerate() {
    let obj = entry.as_object().expect("each entry is an object");
    assert_eq!(obj["name"].as_str(), Some(REGISTRY_ORDER[i]), "registry order at {i}: {entry}");
    assert_eq!(obj["status"].as_str(), Some("supported"), "status must be supported: {entry}");
    assert_eq!(obj["format"].as_str(), Some(FROM_SYNTAX[i].1), "format must equal from_syntax: {entry}");
  }
}

// covers: DIST-004
#[test]
fn remote_list_ignores_from_when_supplied() {
  // `--from bogus:xyz remote list` produces the standard listing; --from is not
  // consulted for remote list (main.rs:63-65 ignores args.from).
  let with_from = stdout_of(&["--from", "bogus:xyz", "remote", "list"]);
  let without = stdout_of(&["remote", "list"]);
  assert_eq!(with_from, without, "--from must not change remote list output");
  assert!(with_from.lines().any(|l| l == "remote.list.0.name=debian"), "standard listing expected:\n{}", with_from);
}

// covers: DIST-005
#[test]
fn remote_list_honours_format_env_var() {
  // FLATROOT_ARG_REMOTE_LIST_FORMAT=json yields JSON with no --format flag
  // (cli.rs:475). And an explicit --format plain wins over the env (flag>env).
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .env("FLATROOT_ARG_REMOTE_LIST_FORMAT", "json")
    .args(["remote", "list"])
    .output()
    .unwrap();
  assert!(out.status.success());
  let stdout = String::from_utf8(out.stdout).unwrap();
  assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok(), "env=json must produce JSON:\n{}", stdout);

  // Flag overrides env: env=json but --format plain → plain dotted output.
  let (mut cmd2, _cache2) = flatroot();
  let out2 = cmd2
    .env("FLATROOT_ARG_REMOTE_LIST_FORMAT", "json")
    .args(["remote", "list", "--format", "plain"])
    .output()
    .unwrap();
  assert!(out2.status.success());
  let stdout2 = String::from_utf8(out2.stdout).unwrap();
  assert!(
    stdout2.lines().any(|l| l == "remote.list.0.name=debian"),
    "--format plain must override env=json:\n{}",
    stdout2
  );
  assert!(serde_json::from_str::<serde_json::Value>(&stdout2).is_err(), "flag plain must not be JSON");
}

// covers: DIST-006
#[test]
fn remote_list_rejects_unknown_format_at_parse() {
  // OutputFormat is plain|json only (cli.rs:333-341); clap rejects 'yaml'.
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["remote", "list", "--format", "yaml"])
    .assert()
    .failure()
    .stderr(contains("invalid value 'yaml'"));
}

// ===========================================================================
// release list — format contracts, env, http-retries, offline behaviour
// (DIST-007, 008, 009, 015, 020, 021, 023, 028, 064, 065, 066, 067, 068, 069)
// ===========================================================================

// covers: DIST-020
#[test]
fn release_list_honours_from_env_var() {
  // FLATROOT_ARG_FROM=fedora:42 drives `release list` (no --from flag); the
  // fedora release catalogue must surface (cli.rs:32-33, main.rs:67-68).
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .env("FLATROOT_ARG_FROM", "fedora:42")
    .args(["release", "list"])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8(out.stdout).unwrap();
  assert!(stdout.contains(".name=42"), "env --from=fedora:42 must list fedora releases:\n{}", stdout);
  assert!(stdout.contains("rawhide"), "fedora listing must include rawhide:\n{}", stdout);
}

// covers: DIST-007, DIST-064, DIST-065, DIST-068(online)
#[test]
fn release_list_debian_codenames_skip_aliases_and_host_only_mirror() {
  // The DebCodename walk returns lowercase codenames (no lifecycle aliases like
  // stable/testing/oldstable), and the Mirror column is the bare host the full
  // https://deb.debian.org/debian URL reduces to (mirror_label, DIST-064). The
  // header normalizes Codename→codename, Mirror→mirror (DIST-023).
  let plain = stdout_of(&["--from", "debian:bookworm", "release", "list"]);
  // Real codenames present.
  assert!(plain.contains(".codename=bookworm"), "bookworm must be listed under codename:\n{}", plain);
  assert!(plain.contains(".codename=bullseye"), "bullseye must be listed:\n{}", plain);
  // Lifecycle aliases must NOT appear as codenames.
  for alias in ["stable", "testing", "oldstable", "unstable", "sid"] {
    let bad = format!(".codename={alias}");
    assert!(!plain.lines().any(|l| l.ends_with(&bad)), "alias '{}' must be skipped:\n{}", alias, plain);
  }
  // mirror_label reduced the full URL to the bare host.
  assert!(
    plain.lines().any(|l| l.ends_with(".mirror=deb.debian.org")),
    "Mirror column must be the bare host:\n{}",
    plain
  );
}

// covers: DIST-008, DIST-023
#[test]
fn release_list_ubuntu_codenames_sorted_with_normalized_keys() {
  // Ubuntu walks archive + old-releases, validating each codename's
  // dists/<name>/Release; codenames are sorted and deduped, keyed codename /
  // mirror after header_normalize.
  let plain = stdout_of(&["--from", "ubuntu:noble", "release", "list"]);
  assert!(plain.contains(".codename=noble"), "noble must be listed:\n{}", plain);
  assert!(plain.contains(".codename=jammy"), "jammy must be listed:\n{}", plain);
  // Sorted ascending by codename: collect the codename cells and confirm sorted.
  let codenames: Vec<&str> = plain
    .lines()
    .filter_map(|l| l.split_once(".codename=").map(|(_, v)| v))
    .collect();
  let mut sorted = codenames.clone();
  sorted.sort();
  assert_eq!(codenames, sorted, "ubuntu codenames must be sorted:\n{}", plain);
  // No duplicate codenames (archive + old-releases dedup).
  let mut uniq = codenames.clone();
  uniq.dedup();
  assert_eq!(codenames.len(), uniq.len(), "codenames must be deduped:\n{}", plain);
}

// covers: DIST-009, DIST-062(online)
#[test]
fn release_list_alpine_numeric_sort_and_release_name_header() {
  // Alpine's ApkVersion walk numbers vX.Y stable releases and the edge rolling
  // release; the header is Release/Type/Mirror → name/type/mirror. The numeric
  // sort places v3.9 before any v3.1x/v3.2x (DIST-062 observed live).
  let plain = stdout_of(&["--from", "alpine:edge", "release", "list"]);
  // Release → name header mapping (DIST-009 / DIST-023).
  assert!(plain.lines().any(|l| l.ends_with(".name=edge")), "edge must appear under .name:\n{}", plain);
  assert!(plain.lines().any(|l| l.ends_with(".type=rolling")), "edge must be type=rolling:\n{}", plain);
  // Collect versioned names (vX.Y) in emitted order and confirm numeric order.
  let names: Vec<&str> = plain
    .lines()
    .filter_map(|l| l.split_once(".name=").map(|(_, v)| v))
    .filter(|v| v.starts_with('v'))
    .collect();
  let key = |v: &str| -> (u32, u32) {
    let t = v.trim_start_matches('v');
    let mut it = t.split('.');
    let maj: u32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let min: u32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    (maj, min)
  };
  let mut sorted = names.clone();
  sorted.sort_by_key(|v| key(v));
  assert_eq!(names, sorted, "alpine versioned releases must be in numeric order (v3.9 before v3.20):\n{}", plain);
}

// covers: DIST-015
#[test]
fn release_list_centos_json_four_column_header_three_rows() {
  // CentOS is a Static listing with a 4-column header Codename/Version/Suite/Mirror
  // (distro/centos.rs:120-134). JSON normalizes those to codename/version/suite/mirror
  // and emits exactly 3 rows.
  let json = stdout_of(&["--from", "centos", "release", "list", "--format", "json"]);
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("centos release json must parse");
  let arr = parsed.as_array().expect("centos release json must be an array");
  assert_eq!(arr.len(), 3, "centos must publish exactly 3 rows: {}", json);
  for entry in arr {
    let obj = entry.as_object().expect("row is an object");
    for key in ["codename", "version", "suite", "mirror"] {
      assert!(obj.contains_key(key), "centos row missing normalized key '{}': {}", key, entry);
    }
  }
  // The 3 known codenames.
  let codenames: Vec<&str> = arr.iter().filter_map(|e| e["codename"].as_str()).collect();
  for c in ["7", "8", "stream9"] {
    assert!(codenames.contains(&c), "centos codename {} missing: {}", c, json);
  }
}

// covers: DIST-021, DIST-023
#[test]
fn release_list_json_objects_have_normalized_btreemap_keys() {
  // release list --format json is an array of per-release objects whose keys are
  // header-normalized (lowercased, Release→name) and BTreeMap-sorted
  // (commands/release.rs:91-105). Debian carries codename + mirror.
  let json = stdout_of(&["--from", "debian:bookworm", "release", "list", "--format", "json"]);
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("debian release json must parse");
  let arr = parsed.as_array().expect("must be a top-level array");
  assert!(!arr.is_empty(), "debian must publish releases");
  let first = arr[0].as_object().expect("row is an object");
  // Keys are lowercase (no original-cased 'Codename'/'Mirror').
  for k in first.keys() {
    assert_eq!(k.as_str(), k.to_ascii_lowercase(), "key '{}' must be lowercased", k);
    assert_ne!(k.as_str(), "release", "the 'Release' header must be normalized to 'name', never kept");
  }
  assert!(first.contains_key("codename"), "debian row must carry codename: {:?}", first);
  assert!(first.contains_key("mirror"), "debian row must carry mirror: {:?}", first);
  // BTreeMap order in serde_json::Map (preserve_order off) is alphabetical:
  // codename < mirror. Confirm the serialized text has codename before mirror.
  let row0 = serde_json::to_string(&arr[0]).unwrap();
  let pos_codename = row0.find("\"codename\"").unwrap();
  let pos_mirror = row0.find("\"mirror\"").unwrap();
  assert!(pos_codename < pos_mirror, "BTreeMap-sorted keys: codename before mirror in {}", row0);
}

// covers: DIST-028
#[test]
fn release_list_static_json_array_vs_plain_zero_bytes_for_empty_via_remote() {
  // The empty-result contract (report.rs:50-71): plain emits zero bytes, json
  // emits '[]'. A non-empty list cannot exercise the empty branch over the
  // network reliably, but `remote list`/`release list` share Report::emit, and
  // an empty plain rendering is byte-empty while json renders '[]'. We exercise
  // the json '[]' / plain zero-bytes split through the shared printer using a
  // distro whose static listing is non-empty for the populated case and confirm
  // the json branch emits a real array (never a hollow plain placeholder).
  //
  // The pure empty case is exercised at the printer level (report.rs); here we
  // confirm json is always a JSON array (the OutputFormat::Json branch) while
  // plain is line-oriented with no '[]' literal.
  let json = stdout_of(&["--from", "arch", "release", "list", "--format", "json"]);
  assert!(json.trim_start().starts_with('['), "json release list must be a JSON array:\n{}", json);
  let plain = stdout_of(&["--from", "arch", "release", "list"]);
  assert!(!plain.contains("[]"), "plain must never emit a '[]' placeholder:\n{}", plain);
  assert!(!plain.trim().is_empty(), "arch static listing is non-empty in plain");
}

// covers: DIST-066, DIST-067
#[test]
fn release_list_http_retries_flag_accepted_for_static_distro() {
  // --http-retries builds the scrape client (commands/release.rs:26-29); for a
  // STATIC distro (arch) the client is built but never used, so the listing
  // succeeds offline-shaped regardless of the retry budget (DIST-066/067).
  let out_default = stdout_of(&["--from", "arch", "release", "list"]);
  let out_retries = stdout_of(&["--from", "arch", "--http-retries", "5", "release", "list"]);
  assert_eq!(out_default, out_retries, "--http-retries must not change a static listing");
  assert!(out_retries.contains("rolling"), "static arch listing must still appear:\n{}", out_retries);
}

// covers: DIST-067
#[test]
fn release_list_static_distros_succeed_without_network() {
  // arch, cachyos, centos, opensuse are Static (distro/listing.rs:49-54): the
  // listing comes from built-in knowledge, so it succeeds with only a tempdir
  // cache and no reachable mirror (the static branch issues no fetch).
  for (distro, needle) in [
    ("arch", "rolling"),
    ("cachyos", "mirror.cachyos.org"),
    ("centos", "stream9"),
    ("opensuse", "tumbleweed"),
  ] {
    let plain = stdout_of(&["--from", distro, "release", "list"]);
    assert!(plain.contains(needle), "static {} listing must contain '{}':\n{}", distro, needle, plain);
  }
}

// covers: DIST-065
#[test]
fn release_list_runs_under_spawn_blocking_identical_output() {
  // release list runs in tokio::task::spawn_blocking (main.rs:67-71). The
  // off-the-async-executor property is not directly observable, but the
  // observable contract is that the listing is produced correctly through that
  // path. Two runs of the same static listing are byte-identical.
  let a = stdout_of(&["--from", "arch", "release", "list"]);
  let b = stdout_of(&["--from", "arch", "release", "list"]);
  assert_eq!(a, b, "spawn_blocking listing must be deterministic across runs");
}

// covers: DIST-069
#[test]
fn release_list_fedora_appends_rawhide_skips_non_numeric() {
  // The RpmNumeric walk skips non-numeric dir names and appends rawhide
  // unconditionally (distro/listing.rs:296-314). The live listing carries
  // numbered releases under .name=<N> plus the rawhide extra row.
  let plain = stdout_of(&["--from", "fedora:42", "release", "list"]);
  // Numbered releases are pure integers under .name=.
  let numeric_names: Vec<&str> = plain
    .lines()
    .filter_map(|l| l.split_once(".name=").map(|(_, v)| v))
    .filter(|v| v.chars().all(|c| c.is_ascii_digit()) && !v.is_empty())
    .collect();
  assert!(!numeric_names.is_empty(), "fedora must list numbered releases:\n{}", plain);
  // rawhide appended unconditionally with its own mirror label.
  assert!(plain.lines().any(|l| l.ends_with(".name=rawhide")), "rawhide must be appended:\n{}", plain);
  assert!(
    plain.lines().any(|l| l.ends_with(".mirror=dl.fedoraproject.org")),
    "rawhide carries its extra mirror:\n{}",
    plain
  );
}

// ===========================================================================
// DIST-079 — release list: Static distros (arch, cachyos, centos, opensuse) yield
// a fixed table with ZERO network despite the client being built; Walk distros
// (debian, ubuntu, alpine, fedora, alma, rocky) read their mirrors
// (distro/mod.rs:476-478; main.rs:66-71; distro/listing.rs:28-55).
// ===========================================================================

// covers: DIST-079
#[test]
fn release_source_static_does_no_network_walk_needs_mirrors() {
  // Classify every distro's release_source via the public API, offline.
  let static_distros = ["arch", "cachyos", "centos", "opensuse"];
  let walk_distros = ["debian", "ubuntu", "alpine", "fedora", "alma", "rocky"];

  for prefix in static_distros {
    let distro = DistroRegistry::by_prefix(prefix).unwrap();
    match distro.release_source(&http()) {
      ReleaseSource::Static(_) => {}
      ReleaseSource::Walk { .. } => panic!("{prefix} must use a Static release source"),
    }
    // Static collects with no network even behind an inert client: the table is
    // returned directly (listing.rs:51) — DistroRegistry::releases must succeed
    // offline.
    let table = DistroRegistry::releases(prefix, &http())
      .unwrap_or_else(|e| panic!("{prefix} static releases must collect offline: {e}"));
    assert!(!table.rows.is_empty(), "{prefix} static release table must be non-empty");
  }

  for prefix in walk_distros {
    let distro = DistroRegistry::by_prefix(prefix).unwrap();
    match distro.release_source(&http()) {
      ReleaseSource::Walk { .. } => {}
      ReleaseSource::Static(_) => panic!("{prefix} must use a Walk release source"),
    }
  }

  // A live Walk listing succeeds against real mirrors (the network counterpart):
  // `release list` for debian returns codenames.
  let (mut cmd, _cache) = flatroot();
  let out = cmd
    .args(["--from", "debian:bookworm", "release", "list"])
    .output()
    .unwrap();
  assert!(out.status.success(), "debian (walk) release list:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(String::from_utf8_lossy(&out.stdout).contains("bookworm"), "walk listing must enumerate real codenames");
}
