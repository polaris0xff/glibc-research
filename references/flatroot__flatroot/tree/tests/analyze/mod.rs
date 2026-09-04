//! Shared helpers for the `flatroot analyze` integration tests.
//! Each test runs the binary as a subprocess against a live distro
//! mirror, parses the JSON output, and asserts on the
//! [`AnalysisOutcome`] shape — a top-level array of `TraceEntry`
//! objects, one per package in the closure.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// Spawns `flatroot --from <remote> analyze trace <target>
/// --format=json` against a fresh per-call cache directory and
/// returns the parsed JSON body. Panics with the full stderr on a
/// non-zero exit so failed runs surface the upstream error.
pub fn run_analyze(remote: &str, target: &str) -> Value {
  run_analyze_args(remote, &[target])
}

/// Variadic-form runner for tests that need extra flags
/// (`--type=library`, multiple positional patterns) on top of the
/// default single-seed shape. Every argument in `extra` is forwarded
/// to the subcommand verbatim — patterns and option flags interleave
/// freely. `--format=json` is always appended so the caller gets the
/// JSON body back. The cache `TempDir` lives until the function
/// returns; nextest can run callers in parallel without contending
/// on a shared cache flock.
pub fn run_analyze_args(remote: &str, extra: &[&str]) -> Value {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let mut full: Vec<&str> = vec!["--from", remote, "analyze", "trace"];
  full.extend_from_slice(extra);
  full.push("--format=json");
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&full)
    .output()
    .expect("flatroot binary failed to spawn");

  assert!(
    output.status.success(),
    "flatroot exited {:?} for {} {:?}\nstderr:\n{}",
    output.status.code(),
    remote,
    extra,
    String::from_utf8_lossy(&output.stderr)
  );
  serde_json::from_slice(&output.stdout)
    .unwrap_or_else(|e| panic!("flatroot stdout for {} {:?} is not valid JSON: {e}", remote, extra))
}

/// Asserts the four invariants every per-distro full-closure test
/// shares:
///
///   1. **Pass 1 sanity** — the declared closure includes
///      `expected_provider`.
///   2. **Pass 2 sanity** — that provider's row carries
///      `expected_soname` in `sonames_provided`, confirming the
///      linker walk mapped the soname back to the expected package.
///   3. **Reason classification** — that provider is `declared_linker`
///      (in both passes), not `declared` or `linker` alone.
///   4. **Clean target** — `unresolved_sonames` may carry distro-
///      specific dlopen'd names (gconv plugins, etc.) that are
///      legitimately not resolvable via DT_NEEDED, but the requested
///      `expected_soname` itself must NOT be among them.
pub fn assert_full_closure(outcome: &Value, expected_provider: &str, expected_soname: &str) {
  let entries = outcome
    .as_array()
    .unwrap_or_else(|| panic!("outcome must be a top-level JSON array, got: {}", outcome));

  let provider_entry = entries
    .iter()
    .find(|e| e["name"].as_str() == Some(expected_provider))
    .unwrap_or_else(|| {
      panic!(
        "expected provider '{}' missing from entries (names present: {:?})",
        expected_provider,
        entries.iter().filter_map(|e| e["name"].as_str()).collect::<Vec<_>>()
      )
    });

  let sonames = provider_entry["sonames_provided"]
    .as_array()
    .unwrap_or_else(|| panic!("entry[{}].sonames_provided is not an array", expected_provider));
  assert!(
    sonames.iter().any(|v| v.as_str() == Some(expected_soname)),
    "entry[{}].sonames_provided must contain '{}': {:?}",
    expected_provider,
    expected_soname,
    sonames
  );

  let reason = provider_entry["reason"].as_str().unwrap_or("");
  assert_eq!(
    reason, "declared_linker",
    "entry[{}].reason must be declared_linker (declared + linker), got {:?}",
    expected_provider, reason
  );

  let unresolved_with_soname: Vec<String> = entries
    .iter()
    .flat_map(|e| {
      e["unresolved_sonames"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|u| u["soname"].as_str().map(String::from))
    })
    .collect();
  assert!(
    !unresolved_with_soname.iter().any(|s| s == expected_soname),
    "expected soname '{}' must resolve, but appears in unresolved_sonames: {:?}",
    expected_soname,
    unresolved_with_soname
  );
}

/// Asserts that `expected_seed` appears in the trace as
/// `reason=target`. Used by tests whose only invariant is "the
/// pattern's seed-resolution put the right package at the root of
/// the trace" — e.g. `analyze_library.rs` cases where the soname
/// stored in `sonames_provided` may not match the user's pattern
/// verbatim (Arch's `libc.so` pattern resolves to `glibc`, but the
/// linker pass records `libc.so.6` from the binaries' DT_NEEDED).
pub fn assert_seed_target(outcome: &Value, expected_seed: &str) {
  let entries = outcome
    .as_array()
    .unwrap_or_else(|| panic!("outcome must be a top-level JSON array, got: {}", outcome));
  let seed_entry = entries
    .iter()
    .find(|e| e["name"].as_str() == Some(expected_seed))
    .unwrap_or_else(|| {
      panic!(
        "expected seed '{}' missing from entries (names present: {:?})",
        expected_seed,
        entries.iter().filter_map(|e| e["name"].as_str()).collect::<Vec<_>>()
      )
    });
  let reason = seed_entry["reason"].as_str().unwrap_or("");
  assert_eq!(reason, "target", "entry[{}].reason must be target (resolved seed), got {:?}", expected_seed, reason);
}
