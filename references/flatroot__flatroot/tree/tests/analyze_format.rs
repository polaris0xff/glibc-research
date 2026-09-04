//! Format-invariance integration test for `flatroot analyze trace`.
//!
//! Runs the same trace of `bash` from `debian:bookworm` twice — once
//! per `--format` value (`plain`, `json`). Asserts that both
//! renderings cover the same package set: a renderer that drops or
//! duplicates entries would be caught here.
//!
//! `bash` is the smallest meaningful target — it ships one ELF
//! binary that links libc, libtinfo, and a few helpers, producing a
//! ~10-entry outcome that exercises every reason classification
//! without dragging in coreutils' broader closure.

mod analyze;

use assert_cmd::Command;
use serde_json::Value;
use std::collections::BTreeSet;
use tempfile::TempDir;

fn run(remote: &str, target: &str, format: &str) -> String {
  run_multi(remote, &[target], format)
}

/// Variadic-positional runner — used by the multi-seed
/// format-invariance test. Forwards every entry of `targets` to the
/// subcommand verbatim, so a caller can pass real package names
/// (`"bash"`), globs (`"firefox-esr*"`), or extra flags
/// (`"--type=library"`) interleaved with patterns.
fn run_multi(remote: &str, targets: &[&str], format: &str) -> String {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let mut full: Vec<&str> = vec!["--from", remote, "analyze", "trace"];
  full.extend_from_slice(targets);
  full.extend_from_slice(&["--format", format]);
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&full)
    .output()
    .expect("flatroot binary failed to spawn");
  assert!(
    output.status.success(),
    "flatroot --format={} exited {:?}\nstderr:\n{}",
    format,
    output.status.code(),
    String::from_utf8_lossy(&output.stderr)
  );
  String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Extract the set of package names from the JSON-formatted trace.
/// The wire form is a top-level array of entries; each entry's
/// `name` field is the key. This is the canonical ground truth —
/// every other format must reference the same set, no more and no
/// less.
fn names_from_json(out: &str) -> BTreeSet<String> {
  let v: Value = serde_json::from_str(out).expect("json must parse");
  v.as_array()
    .expect("json top must be an array")
    .iter()
    .filter_map(|e| e["name"].as_str().map(|s| s.to_string()))
    .collect()
}

/// Extract the set of package names from the plain-formatted trace.
/// Plain emits dotted `KEY=VALUE` lines, every line prefixed with
/// `analyze.trace.<index>.`. The set of names is recovered by selecting
/// every line of shape `analyze.trace.<index>.name=<value>` and extracting
/// the value.
fn names_from_plain(out: &str) -> BTreeSet<String> {
  let mut names = BTreeSet::new();
  for line in out.lines() {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      continue;
    }
    let (path, value) = match trimmed.split_once('=') {
      Some(pv) => pv,
      None => continue,
    };
    // path is `analyze.trace.<index>.name`; the fourth dot-separated component
    // must be exactly `name` to match the entry's name leaf and not
    // any other field that happens to be a string.
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() == 4 && parts[0] == "analyze" && parts[1] == "trace" && parts[3] == "name" {
      names.insert(value.to_string());
    }
  }
  names
}

#[test]
fn analyze_format_invariance_same_package_set() {
  let json = run("debian:bookworm", "bash", "json");
  let plain = run("debian:bookworm", "bash", "plain");

  let json_names = names_from_json(&json);
  let plain_names = names_from_plain(&plain);

  assert!(json_names.contains("bash"), "JSON must include the target");
  assert!(json_names.contains("libc6"), "JSON must include libc6");

  assert_eq!(
    json_names,
    plain_names,
    "plain renderer dropped or invented entries compared to JSON\nonly in JSON: {:?}\nonly in plain: {:?}",
    json_names.difference(&plain_names).collect::<Vec<_>>(),
    plain_names.difference(&json_names).collect::<Vec<_>>()
  );
}

#[test]
fn analyze_format_invariance_multi_seed() {
  // Multi-seed shape — two real package names; each renderer must
  // surface both seeds plus their merged closure. Plain and JSON
  // carry both as `reason=target` entries. Format invariance still
  // holds: every renderer's name set is identical to the JSON ground
  // truth.
  let json = run_multi("debian:bookworm", &["bash", "coreutils"], "json");
  let plain = run_multi("debian:bookworm", &["bash", "coreutils"], "plain");

  let json_names = names_from_json(&json);
  let plain_names = names_from_plain(&plain);

  assert!(json_names.contains("bash"), "JSON must include first seed");
  assert!(json_names.contains("coreutils"), "JSON must include second seed");
  assert!(json_names.contains("libc6"), "JSON must include shared transitive dep libc6");

  // Both seeds must be reason=target in the JSON.
  let v: Value = serde_json::from_str(&json).unwrap();
  let targets: BTreeSet<String> = v
    .as_array()
    .unwrap()
    .iter()
    .filter(|e| e["reason"] == "target")
    .filter_map(|e| e["name"].as_str().map(String::from))
    .collect();
  assert_eq!(
    targets,
    BTreeSet::from(["bash".to_string(), "coreutils".to_string()]),
    "both bash and coreutils must be reason=target, got: {:?}",
    targets
  );

  assert_eq!(
    json_names,
    plain_names,
    "multi-seed: plain dropped or invented entries\nonly in JSON: {:?}\nonly in plain: {:?}",
    json_names.difference(&plain_names).collect::<Vec<_>>(),
    plain_names.difference(&json_names).collect::<Vec<_>>()
  );
}
