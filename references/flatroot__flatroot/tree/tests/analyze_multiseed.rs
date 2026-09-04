//! Multi-seed `flatroot analyze trace` integration tests — variadic
//! positional patterns, package globs that fan out to multiple
//! packages, and `--type=library` patterns whose owning packages
//! overlap. Each case verifies the union/dedup discipline: every
//! resolved seed lands as `reason=target`, the shared transitive
//! closure is collapsed into one entry per package, and the wire
//! shape carries no duplicates.

mod analyze;

use serde_json::Value;
use std::collections::BTreeSet;

/// Pulls the names of every entry whose `reason` is `target` out of
/// the parsed JSON outcome — i.e. the resolved seed set as the
/// orchestrator computed it. Sorted into a `BTreeSet` so equality
/// assertions are order-independent.
fn target_names(outcome: &Value) -> BTreeSet<String> {
  outcome
    .as_array()
    .expect("outcome is a top-level JSON array")
    .iter()
    .filter(|e| e["reason"] == "target")
    .filter_map(|e| e["name"].as_str().map(String::from))
    .collect()
}

/// Counts how many entries carry the given name. Multi-seed traces
/// must dedup shared providers; this helper makes the dedup
/// invariant ("libc6 appears once across the union") explicit.
fn entry_count(outcome: &Value, name: &str) -> usize {
  outcome.as_array().unwrap().iter().filter(|e| e["name"] == name).count()
}

#[test]
fn analyze_multiseed_variadic_positionals_dedup_closure() {
  // Two real package names — both must appear as `reason=target`,
  // and the shared transitive dep `libc6` must appear exactly once
  // (the BTreeMap-backed merge collapses duplicates).
  let outcome = analyze::run_analyze_args("debian:bookworm", &["bash", "coreutils"]);

  let targets = target_names(&outcome);
  assert_eq!(
    targets,
    BTreeSet::from(["bash".to_string(), "coreutils".to_string()]),
    "exactly the two seeds must be reason=target, got: {:?}",
    targets
  );

  assert_eq!(entry_count(&outcome, "libc6"), 1, "shared transitive dep libc6 must appear exactly once");
}

#[test]
fn analyze_multiseed_package_glob_fans_out() {
  // A single positional glob matching multiple packages — every
  // matching package becomes a seed, deduped by name. The
  // `firefox-esr-l10n-d?` glob matches at least `firefox-esr-l10n-da`
  // and `firefox-esr-l10n-de`; both must land as `reason=target`.
  let outcome = analyze::run_analyze_args("debian:bookworm", &["firefox-esr-l10n-d?"]);

  let targets = target_names(&outcome);
  assert!(targets.len() >= 2, "expected >=2 seeds from glob fan-out, got: {:?}", targets);
  assert!(
    targets.iter().all(|n| n.starts_with("firefox-esr-l10n-d")),
    "every seed must match the source glob, got: {:?}",
    targets
  );
}

#[test]
fn analyze_multiseed_library_overlap_dedups_seed_set() {
  // Both `libssl.so.3` and `libcrypto.so.3` are shipped by the same
  // Debian package (`libssl3`). The orchestrator unions every
  // pattern's owning packages and dedupes by package name, so the
  // seed set is exactly `{libssl3}` and the trace produces a single
  // `reason=target` entry.
  let outcome = analyze::run_analyze_args("debian:bookworm", &["--type=library", "libssl.so.3", "libcrypto.so.3"]);

  let targets = target_names(&outcome);
  assert_eq!(
    targets,
    BTreeSet::from(["libssl3".to_string()]),
    "overlapping library patterns must dedup to a single seed, got: {:?}",
    targets
  );
}

#[test]
fn analyze_multiseed_empty_match_emits_empty_outcome() {
  // Aggregate-empty match (every pattern hits nothing) — the
  // orchestrator short-circuits to an empty outcome. JSON renders as
  // an empty array; no entries, no seeds. Per the CLI_DESIGN
  // empty-results contract this is success, not error.
  let outcome = analyze::run_analyze_args("debian:bookworm", &["zzz-nonexistent-xyz-999"]);

  let arr = outcome.as_array().expect("top-level array");
  assert!(arr.is_empty(), "empty match must produce empty entries array, got: {:?}", arr);
}
