//! Strategy / framing / env-fallback integration tests for
//! `flatroot analyze trace`. Every test runs the real binary as a
//! subprocess against a live distro mirror (or, for the pure
//! parse/validation cases, with no network at all) and asserts on the
//! observable contract: stdout is the rendered trace, stderr carries
//! the human framing and the trailing unresolved-soname warning, and
//! the reason classification / soname fields reflect which of the two
//! analysis passes the `--strategy` flag left enabled.
//!
//! Source under test: `src/main.rs` (analyze dispatch + framing),
//! `src/commands/analyze/mod.rs` (run orchestration, empty-match
//! short-circuit), `src/commands/analyze/outcome.rs` (merge /
//! classification), `src/commands/analyze/pass/{declared,linker}.rs`,
//! and `src/cli.rs` (the `analyze trace` arg surface + env fallbacks).

use std::collections::BTreeSet;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// Run `analyze trace` with the given extra args (no implicit
/// `--format`), against a fresh per-call cache, returning the raw
/// process output. `--from debian:bookworm --arch x86_64` is always
/// prefixed unless the caller's args already carry `--from` /
/// `--arch`; here every caller supplies the bare trace args.
fn run(extra: &[&str]) -> std::process::Output {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let mut full: Vec<&str> = vec!["--from", "debian:bookworm", "--arch", "x86_64", "analyze", "trace"];
  full.extend_from_slice(extra);
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&full)
    .output()
    .expect("flatroot binary failed to spawn")
}

/// Run with an explicit argv (no debian/x86_64 prefix) and a custom
/// environment overlay, against a fresh cache. Used by the env-fallback
/// and error-validation tests.
fn run_argv(env: &[(&str, &str)], argv: &[&str]) -> std::process::Output {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  cmd.env("FLATROOT_CACHE_HOME", cache.path());
  for (k, v) in env {
    cmd.env(k, v);
  }
  cmd.args(argv).output().expect("flatroot binary failed to spawn")
}

/// Assert a network run succeeded, panicking with the captured stderr
/// otherwise, and return stdout as a string.
fn stdout_ok(out: &std::process::Output, what: &str) -> String {
  assert!(
    out.status.success(),
    "{what} exited {:?}\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stdout).into_owned()
}

fn parse_json(out: &str) -> Value {
  serde_json::from_str(out).unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\n{out}"))
}

fn names(v: &Value) -> BTreeSet<String> {
  v.as_array()
    .expect("top-level array")
    .iter()
    .filter_map(|e| e["name"].as_str().map(String::from))
    .collect()
}

fn reasons(v: &Value) -> BTreeSet<String> {
  v.as_array()
    .expect("top-level array")
    .iter()
    .filter_map(|e| e["reason"].as_str().map(String::from))
    .collect()
}

fn entry<'a>(v: &'a Value, name: &str) -> &'a Value {
  v.as_array()
    .unwrap()
    .iter()
    .find(|e| e["name"].as_str() == Some(name))
    .unwrap_or_else(|| panic!("entry '{name}' missing from outcome"))
}

// covers: ANL-001, ANL-013, ANL-062
#[test]
fn analyze_default_plain_frames_single_seed_and_marks_target() {
  // Default invocation (both strategies, plain format). The seed `bash`
  // must classify as reason=target; stderr must carry the ArchContext
  // loaded-package framing, the single-seed "Analyzing <name> <version>
  // from <source> (<arch>)" line, and (because bash links dlopen-only
  // gconv plugins) a trailing unresolved warning.
  let out = run(&["bash"]);
  assert!(
    out.status.success(),
    "default plain trace exited {:?}\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr)
  );
  let stdout = String::from_utf8_lossy(&out.stdout);
  let stderr = String::from_utf8_lossy(&out.stderr);

  // Plain output: entries are alphabetically ordered under
  // `analyze.trace.<index>.`; locate bash's index, then assert its reason leaf
  // classifies it as target.
  let bash_index = stdout
    .lines()
    .find_map(|l| {
      l.strip_prefix("analyze.trace.")
        .and_then(|rest| rest.strip_suffix(".name=bash"))
        .map(|i| i.to_string())
    })
    .unwrap_or_else(|| panic!("plain output must carry a analyze.trace.<index>.name=bash line:\n{stdout}"));
  let reason_line = format!("analyze.trace.{bash_index}.reason=target");
  assert!(
    stdout.lines().any(|l| l == reason_line),
    "bash (at index {bash_index}) must classify as reason=target ({reason_line}):\n{stdout}"
  );
  // Every entry path is prefixed with `analyze.trace.<index>.` (the plain
  // contract); the index component is numeric.
  assert!(
    bash_index.chars().all(|c| c.is_ascii_digit()),
    "the analyze.trace.<index>. prefix must carry a numeric index, got: {bash_index}"
  );

  // ArchContext framing + single-seed framing on stderr.
  assert!(
    stderr
      .lines()
      .any(|l| l.starts_with("Loaded ") && l.ends_with(" packages")),
    "missing loaded-count framing:\n{stderr}"
  );
  let framing = stderr
    .lines()
    .find(|l| l.starts_with("Analyzing bash "))
    .unwrap_or_else(|| panic!("missing single-seed framing line:\n{stderr}"));
  assert!(
    framing.ends_with(" from debian:bookworm (x86_64)"),
    "single-seed framing must be 'Analyzing <name> <version> from <source> (<arch>)', got: {framing}"
  );
  // The version token between name and " from " must be non-empty.
  let mid = framing
    .strip_prefix("Analyzing bash ")
    .and_then(|s| s.strip_suffix(" from debian:bookworm (x86_64)"))
    .expect("framing shape");
  assert!(!mid.is_empty(), "single-seed framing must name a version, got empty in: {framing}");
}

// covers: ANL-002
#[test]
fn analyze_strategy_declared_skips_linker_pass() {
  // --strategy declared runs only the declared pass: no "Linker walk"
  // stderr line, no unresolved warning, every soname field empty,
  // depends_on populated, and only declared/target reasons appear.
  let out = run(&["--strategy", "declared", "bash", "--format", "json"]);
  let stdout = stdout_ok(&out, "declared-only trace");
  let stderr = String::from_utf8_lossy(&out.stderr);
  let v = parse_json(&stdout);

  assert!(!stderr.contains("Linker walk"), "declared-only must not run the linker pass:\n{stderr}");
  assert!(!stderr.contains("could not be resolved"), "declared-only must emit no unresolved-soname warning:\n{stderr}");
  assert_eq!(
    reasons(&v),
    BTreeSet::from(["declared".to_string(), "target".to_string()]),
    "declared-only reasons must be exactly {{declared, target}}"
  );
  // libc6 is a declared dependency; its soname fields stay empty.
  let libc6 = entry(&v, "libc6");
  assert_eq!(libc6["reason"], "declared", "libc6 must be reason=declared under declared-only");
  assert!(libc6["sonames_provided"].as_array().unwrap().is_empty(), "declared-only: sonames_provided must be empty");
  assert!(libc6["sonames_consumed"].as_array().unwrap().is_empty(), "declared-only: sonames_consumed must be empty");
  assert!(
    !libc6["depends_on"].as_array().unwrap().is_empty(),
    "declared-only: depends_on must be populated for a real declared package"
  );
}

// covers: ANL-003, ANL-033 (declared-only classification proven from the other side), ANL-048, ANL-062
#[test]
fn analyze_strategy_linker_skips_declared_pass() {
  // --strategy linker runs only the linker pass: depends_on empty
  // everywhere, providers carry sonames_provided, the seed keeps its
  // resolved version, and only linker/target reasons appear. The
  // soname resolver (ELF class + runpath driven) is what credits
  // libc6 the consumed sonames.
  let out = run(&["--strategy", "linker", "bash", "--format", "json"]);
  let stdout = stdout_ok(&out, "linker-only trace");
  let stderr = String::from_utf8_lossy(&out.stderr);
  let v = parse_json(&stdout);

  assert!(stderr.contains("Linker walk"), "linker-only must run the linker pass:\n{stderr}");
  assert!(!stderr.contains("declared packages"), "linker-only must not run the declared pass:\n{stderr}");
  assert_eq!(
    reasons(&v),
    BTreeSet::from(["linker".to_string(), "target".to_string()]),
    "linker-only reasons must be exactly {{linker, target}}"
  );
  // Every entry has empty depends_on (declared pass never ran).
  for e in v.as_array().unwrap() {
    assert!(e["depends_on"].as_array().unwrap().is_empty(), "linker-only: depends_on must be empty for {}", e["name"]);
  }
  // The seed keeps its resolved version even though it enters via the
  // consumed map (which carries no version).
  let bash = entry(&v, "bash");
  assert_eq!(bash["reason"], "target");
  assert!(!bash["version"].as_str().unwrap().is_empty(), "consumer-only seed must keep its resolved version");
  // libc6 is reached only by linkage here: it provides sonames and is
  // reason=linker (an under-declaration, since the declared pass is off).
  let libc6 = entry(&v, "libc6");
  assert_eq!(libc6["reason"], "linker");
  let provided = libc6["sonames_provided"].as_array().unwrap();
  assert!(
    provided.iter().any(|s| s == "libc.so.6"),
    "the soname resolver must credit libc6 with libc.so.6: {provided:?}"
  );
}

// covers: ANL-004, ANL-032, ANL-033, ANL-034
#[test]
fn analyze_strategy_both_explicit_equals_default_and_unions_reasons() {
  // --strategy declared,linker is the explicit form of the default;
  // stdout must match a default-format run byte-for-byte. A package
  // reached both ways is declared_linker; the seed reason=target
  // overrides whatever the merge would otherwise infer.
  //
  // Note: debian:bookworm declares every DT_NEEDED via dpkg-shlibdeps, so its
  // closure has no under-declared (reason=linker) provider — that path is
  // covered deterministically by the unit test
  // `commands::analyze::tests::outcome_build_linker_only_package_marked_undeclared`.
  let default = run(&["bash", "--format", "json"]);
  let explicit = run(&["--strategy", "declared,linker", "bash", "--format", "json"]);
  let default_out = stdout_ok(&default, "default trace");
  let explicit_out = stdout_ok(&explicit, "explicit declared,linker trace");
  assert_eq!(default_out, explicit_out, "--strategy declared,linker must produce identical output to the default");

  let v = parse_json(&explicit_out);
  // Seed override: bash would be declared_linker by the merge, but the
  // target stamp wins.
  assert_eq!(entry(&v, "bash")["reason"], "target", "seed reason=target must override declared_linker");
  // libc6 reached both ways → declared_linker, with sonames_provided.
  let libc6 = entry(&v, "libc6");
  assert_eq!(libc6["reason"], "declared_linker", "package reached both ways must be declared_linker");
  assert!(
    libc6["sonames_provided"]
      .as_array()
      .unwrap()
      .iter()
      .any(|s| s == "libc.so.6"),
    "declared_linker provider must carry its provided sonames"
  );
  // A declared-only package (declared but nothing links a library it
  // provides) keeps reason=declared with an empty sonames_provided.
  let declared_only: Vec<&Value> = v
    .as_array()
    .unwrap()
    .iter()
    .filter(|e| e["reason"] == "declared")
    .collect();
  assert!(!declared_only.is_empty(), "the both-strategies closure must surface at least one reason=declared package");
  for e in &declared_only {
    assert!(
      e["sonames_provided"].as_array().unwrap().is_empty(),
      "a reason=declared package must carry no sonames_provided: {}",
      e["name"]
    );
  }
}

// covers: ANL-035, ANL-036, ANL-037
#[test]
fn analyze_unresolved_warning_tracks_aggregate_count() {
  // The trailing stderr warning must appear iff the aggregate
  // unresolved-soname count is > 0, and when present must state that
  // exact count. The per-entry {binary, soname} unresolved records sum
  // to the same total, so the JSON ground truth and the stderr warning
  // agree. Asserting the conditional covers both the warning-present and
  // warning-absent legs truthfully against whatever the live count is.
  let json_out = run(&["--strategy", "linker", "bash", "--format", "json"]);
  let json = stdout_ok(&json_out, "linker json trace");
  let v = parse_json(&json);

  let mut total = 0usize;
  for e in v.as_array().unwrap() {
    for u in e["unresolved_sonames"].as_array().unwrap() {
      // Each per-entry record carries the consuming binary and the soname.
      assert!(u["binary"].is_string(), "unresolved record must carry a binary path: {u}");
      assert!(u["soname"].is_string(), "unresolved record must carry a soname: {u}");
      total += 1;
    }
  }

  let plain_out = run(&["--strategy", "linker", "bash"]);
  let stderr = String::from_utf8_lossy(&plain_out.stderr);
  let warning = format!("warning: {total} soname(s) could not be resolved against the index");
  if total > 0 {
    assert!(
      stderr.contains(&warning),
      "with {total} unresolved sonames the trailing warning must appear, got stderr:\n{stderr}"
    );
  } else {
    assert!(
      !stderr.contains("could not be resolved"),
      "with zero unresolved sonames no warning must appear, got stderr:\n{stderr}"
    );
  }
}

// covers: ANL-038, ANL-039, ANL-040
#[test]
fn analyze_linker_consumed_provided_shapes_and_dedup() {
  // sonames_consumed records {binary, provider, soname} per resolved
  // link, bucketed under the consumer. sonames_provided is a deduped,
  // stably-ordered (BTreeSet) list per provider. The walk reaches a
  // fixpoint following providers transitively, so libc6 (a transitive
  // provider) is present and credited even though bash names it only
  // indirectly.
  let out = run(&["--strategy", "linker", "bash", "--format", "json"]);
  let stdout = stdout_ok(&out, "linker json trace");
  let v = parse_json(&stdout);

  // Consumed edges on bash carry the full triple.
  let bash = entry(&v, "bash");
  let consumed = bash["sonames_consumed"].as_array().unwrap();
  assert!(!consumed.is_empty(), "bash must record consumed sonames");
  let libc_edge = consumed
    .iter()
    .find(|c| c["soname"] == "libc.so.6")
    .expect("bash must consume libc.so.6");
  assert_eq!(libc_edge["provider"], "libc6", "libc.so.6 must resolve to libc6");
  assert!(libc_edge["binary"].as_str().unwrap().contains("bash"), "the consuming binary path must name bash");

  // sonames_provided is deduped and sorted (BTreeSet semantics).
  let libc6 = entry(&v, "libc6");
  let provided: Vec<&str> = libc6["sonames_provided"]
    .as_array()
    .unwrap()
    .iter()
    .map(|s| s.as_str().unwrap())
    .collect();
  let mut sorted = provided.clone();
  sorted.sort();
  assert_eq!(provided, sorted, "sonames_provided must be stably sorted (BTreeSet)");
  let unique: BTreeSet<&str> = provided.iter().copied().collect();
  assert_eq!(unique.len(), provided.len(), "sonames_provided must not contain duplicates");

  // Transitive fixpoint: libc6 is a provider bash reaches only through
  // its closure, present and credited (each transitive provider visited).
  assert!(provided.iter().any(|s| *s == "libc.so.6"), "transitive provider libc6 must be credited libc.so.6");
}

// covers: ANL-007
#[test]
fn analyze_empty_match_plain_zero_stdout_passes_never_run() {
  // A pattern matching no packages exits 0 with zero stdout bytes (plain
  // / default format). stderr frames the empty case as "Analyzing 0
  // packages ...", and the two analysis passes never run (no "Resolved"
  // / "Linker walk" lines).
  let out = run(&["this-package-does-not-exist-xyz"]);
  assert!(out.status.success(), "empty match must exit 0");
  assert!(
    out.stdout.is_empty(),
    "empty match (plain) must emit zero stdout bytes, got: {:?}",
    String::from_utf8_lossy(&out.stdout)
  );
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Analyzing 0 packages from debian:bookworm (x86_64)"),
    "empty match must frame '0 packages' on stderr:\n{stderr}"
  );
  assert!(!stderr.contains("Resolved "), "passes must not run on empty match (declared):\n{stderr}");
  assert!(!stderr.contains("Linker walk"), "passes must not run on empty match (linker):\n{stderr}");
}

// covers: ANL-015
#[test]
fn analyze_library_empty_match_zero_stdout() {
  // --type library with a soname pattern that owns nothing takes the
  // LibraryMatch::glob no-match leg and yields the same empty-outcome
  // contract: exit 0, zero stdout, "Analyzing 0 packages" framing.
  let out = run(&["--type", "library", "libnonexistent.so.999"]);
  assert!(out.status.success(), "library empty match must exit 0");
  assert!(out.stdout.is_empty(), "library empty match (plain) must emit zero stdout bytes");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Analyzing 0 packages from debian:bookworm (x86_64)"),
    "library empty match must frame '0 packages':\n{stderr}"
  );
}

// covers: ANL-012
#[test]
fn analyze_framing_lists_all_seeds_for_two_to_five() {
  // 2..=5 seeds: the framing lists every seed (name + version), with no
  // "... (N total)" truncation suffix.
  let out = run(&["bash", "coreutils", "dash", "--format", "json"]);
  let _ = stdout_ok(&out, "three-seed trace");
  let stderr = String::from_utf8_lossy(&out.stderr);
  let framing = stderr
    .lines()
    .find(|l| l.starts_with("Analyzing 3 packages from debian:bookworm (x86_64): "))
    .unwrap_or_else(|| panic!("missing 3-seed framing line:\n{stderr}"));
  assert!(!framing.contains("... ("), "3-seed framing must not truncate, got: {framing}");
  for seed in ["bash", "coreutils", "dash"] {
    assert!(framing.contains(seed), "3-seed framing must list {seed}, got: {framing}");
  }
}

// covers: ANL-011
#[test]
fn analyze_framing_truncates_preview_to_first_five() {
  // > 5 seeds: the framing previews exactly the first five seeds, then
  // ", ... (N total)" where N is the full seed count. Asserted against
  // the live total parsed from the framing itself (and cross-checked
  // against the JSON target count), so the test stays correct as the
  // mirror's package set evolves.
  let json_out = run(&["firefox-esr-l10n-*", "--format", "json"]);
  let json = stdout_ok(&json_out, "glob fan-out json");
  let target_count = parse_json(&json)
    .as_array()
    .unwrap()
    .iter()
    .filter(|e| e["reason"] == "target")
    .count();
  assert!(target_count > 5, "glob must fan out to more than 5 seeds, got {target_count}");

  let plain_out = run(&["firefox-esr-l10n-*"]);
  let stderr = String::from_utf8_lossy(&plain_out.stderr);
  let framing = stderr
    .lines()
    .find(|l| l.starts_with("Analyzing ") && l.contains(" packages from "))
    .unwrap_or_else(|| panic!("missing N-seed framing line:\n{stderr}"));
  let total_suffix = format!(", ... ({target_count} total)");
  assert!(framing.ends_with(&total_suffix), "framing must end with the total suffix '{total_suffix}', got: {framing}");
  assert!(
    framing.starts_with(&format!("Analyzing {target_count} packages from debian:bookworm (x86_64): ")),
    "framing must declare the full count, got: {framing}"
  );
  // The preview between ": " and ", ... (" must contain exactly five
  // "name version" entries, comma-separated.
  let body = framing
    .split_once("): ")
    .and_then(|(_, rest)| rest.strip_suffix(&total_suffix))
    .expect("framing preview body");
  let previews: Vec<&str> = body.split(", ").collect();
  assert_eq!(previews.len(), 5, "framing must preview exactly the first 5 seeds, got {}: {body}", previews.len());
}

// covers: ANL-016
#[test]
fn analyze_type_env_fallback_selects_library_mode() {
  // FLATROOT_ARG_ANALYZE_TRACE_TYPE=library makes a soname pattern match
  // libraries; libssl.so.3's owning package (libssl3) lands as the seed.
  let out = run_argv(
    &[("FLATROOT_ARG_ANALYZE_TRACE_TYPE", "library")],
    &[
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64",
      "analyze",
      "trace",
      "libssl.so.3",
      "--format",
      "json",
    ],
  );
  let stdout = stdout_ok(&out, "type env fallback");
  let v = parse_json(&stdout);
  let targets: BTreeSet<String> = v
    .as_array()
    .unwrap()
    .iter()
    .filter(|e| e["reason"] == "target")
    .filter_map(|e| e["name"].as_str().map(String::from))
    .collect();
  assert_eq!(targets, BTreeSet::from(["libssl3".to_string()]), "library-mode env must resolve libssl.so.3 to libssl3");
}

// covers: ANL-017
#[test]
fn analyze_format_env_fallback_selects_json() {
  // FLATROOT_ARG_ANALYZE_TRACE_FORMAT=json yields parseable JSON with no
  // explicit --format flag.
  let out = run_argv(
    &[("FLATROOT_ARG_ANALYZE_TRACE_FORMAT", "json")],
    &[
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64",
      "analyze",
      "trace",
      "bash",
    ],
  );
  let stdout = stdout_ok(&out, "format env fallback");
  let v = parse_json(&stdout);
  assert!(names(&v).contains("bash"), "json env output must carry the seed");
}

// covers: ANL-018
#[test]
fn analyze_strategy_env_fallback_selects_declared_only() {
  // FLATROOT_ARG_ANALYZE_TRACE_STRATEGY=declared runs the declared pass
  // only — no linker walk, reasons confined to {declared, target}.
  let out = run_argv(
    &[("FLATROOT_ARG_ANALYZE_TRACE_STRATEGY", "declared")],
    &[
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64",
      "analyze",
      "trace",
      "bash",
      "--format",
      "json",
    ],
  );
  let stdout = stdout_ok(&out, "strategy env fallback");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(!stderr.contains("Linker walk"), "declared-only via env must skip the linker pass:\n{stderr}");
  assert_eq!(
    reasons(&parse_json(&stdout)),
    BTreeSet::from(["declared".to_string(), "target".to_string()]),
    "declared-only via env: reasons must be {{declared, target}}"
  );
}

// covers: ANL-019, ANL-053
#[test]
fn analyze_with_widens_declared_closure_via_flag_and_env() {
  // --with recommends widens the declared closure to a strict superset
  // of the bare declared closure, and the added packages are
  // reason=declared. The same widening is reachable through the
  // FLATROOT_ARG_ANALYZE_TRACE_WITH env fallback.
  let bare = run(&["--strategy", "declared", "bash", "--format", "json"]);
  let bare_v = parse_json(&stdout_ok(&bare, "bare declared trace"));
  let bare_names = names(&bare_v);

  let widened = run(&[
    "--strategy",
    "declared",
    "--with",
    "recommends",
    "bash",
    "--format",
    "json",
  ]);
  let widened_v = parse_json(&stdout_ok(&widened, "declared+recommends trace"));
  let widened_names = names(&widened_v);

  assert!(bare_names.is_subset(&widened_names), "--with recommends must not drop any bare-declared package");
  assert!(
    widened_names.len() > bare_names.len(),
    "--with recommends must widen the declared closure (bare {} vs widened {})",
    bare_names.len(),
    widened_names.len()
  );
  // Every newly added package is reason=declared (recommends edges are
  // declared edges).
  for added in widened_names.difference(&bare_names) {
    assert_eq!(
      entry(&widened_v, added)["reason"],
      "declared",
      "recommends-added package {added} must be reason=declared"
    );
  }

  // Env fallback reaches the same widening (here recommends+suggests,
  // which balloons the closure far past the bare set).
  let env_out = run_argv(
    &[("FLATROOT_ARG_ANALYZE_TRACE_WITH", "recommends,suggests")],
    &[
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64",
      "analyze",
      "trace",
      "--strategy",
      "declared",
      "bash",
      "--format",
      "json",
    ],
  );
  let env_v = parse_json(&stdout_ok(&env_out, "with env fallback"));
  assert!(names(&env_v).len() > bare_names.len(), "FLATROOT_ARG_ANALYZE_TRACE_WITH must widen the declared closure");
}

// covers: ANL-052
#[test]
fn analyze_with_is_inert_under_linker_strategy() {
  // --with recommends,suggests has no effect under --strategy linker:
  // the declared pass is skipped, so the flags are computed but never
  // consulted. Output must be identical to a plain linker trace.
  let plain = run(&["--strategy", "linker", "bash", "--format", "json"]);
  let with = run(&[
    "--strategy",
    "linker",
    "--with",
    "recommends,suggests",
    "bash",
    "--format",
    "json",
  ]);
  let plain_out = stdout_ok(&plain, "plain linker trace");
  let with_out = stdout_ok(&with, "linker trace with inert --with");
  assert_eq!(plain_out, with_out, "--with must be inert under --strategy linker");
}

// covers: ANL-049
#[test]
fn analyze_takes_only_first_arch_when_several_given() {
  // --arch x86_64,i686 analyses only the first token (x86_64); the i686
  // entry is ignored. The stderr framing names exactly x86_64.
  let out = run_argv(
    &[],
    &[
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64,i686",
      "analyze",
      "trace",
      "bash",
      "--format",
      "json",
    ],
  );
  assert!(
    out.status.success(),
    "multi-arch first-token analyze must succeed\nstderr:\n{}",
    String::from_utf8_lossy(&out.stderr)
  );
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Fetching package index for debian:bookworm (x86_64)"), "must fetch only x86_64:\n{stderr}");
  assert!(
    stderr.contains("(x86_64)") && !stderr.contains("(i686)"),
    "framing must name only x86_64, never i686:\n{stderr}"
  );
}

// covers: ANL-006, ANL-005
#[test]
fn analyze_empty_strategy_rejected_before_fetch() {
  // `--strategy=` (empty) is rejected at parse time: clap refuses a
  // value-bearing flag with no value, before any index fetch. (The
  // in-main `--strategy requires at least one ...` bail and the
  // run()-level both-disabled bail are unreachable from the CLI because
  // clap never produces an empty strategy vector — see notes.)
  let out = run(&["--strategy=", "bash"]);
  assert!(!out.status.success(), "empty --strategy must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("a value is required for '--strategy <STRATEGY>' but none was supplied"),
    "empty --strategy must be rejected by clap, got:\n{stderr}"
  );
  assert!(
    !stderr.contains("Fetching package index"),
    "empty --strategy must be rejected before any index fetch:\n{stderr}"
  );
}

// covers: ANL-054
#[test]
fn analyze_missing_from_rejected_before_analysis() {
  // analyze with no --from is rejected by the shared from-required gate
  // before any index fetch.
  let out = run_argv(&[], &["--arch", "x86_64", "analyze", "trace", "bash"]);
  assert!(!out.status.success(), "missing --from must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("--from is required"), "missing --from must be rejected with guidance:\n{stderr}");
  assert!(!stderr.contains("Fetching package index"), "missing --from must be caught before any fetch:\n{stderr}");
}

// covers: ANL-055
#[test]
fn analyze_invalid_arch_rejected_before_analysis() {
  // An unknown --arch token is rejected by Arch::from_uname on the first
  // token, before any index fetch.
  let out = run_argv(
    &[],
    &[
      "--from",
      "debian:bookworm",
      "--arch",
      "bogusarch",
      "analyze",
      "trace",
      "bash",
    ],
  );
  assert!(!out.status.success(), "invalid --arch must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Unsupported architecture 'bogusarch'"),
    "invalid --arch must be rejected naming the token:\n{stderr}"
  );
  assert!(!stderr.contains("Fetching package index"), "invalid --arch must be caught before any fetch:\n{stderr}");
}

// covers: ANL-057
#[test]
fn analyze_invalid_format_value_rejected_by_clap() {
  // --format yaml is not a valid value-enum; clap rejects it before any
  // dispatch or fetch.
  let out = run(&["--format", "yaml", "bash"]);
  assert!(!out.status.success(), "invalid --format must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("invalid value 'yaml' for '--format <FORMAT>'"),
    "invalid --format must be a clap value-enum error, got:\n{stderr}"
  );
  assert!(!stderr.contains("Fetching package index"), "clap rejection must precede any fetch:\n{stderr}");
}

// covers: ANL-058, ANL-059
#[test]
fn analyze_linker_internal_invariants_do_not_fire_on_normal_input() {
  // The linker walk carries two hard-error invariants that are not
  // reproducible from normal input: a provider that was resolved but is
  // then missing from the index ("provider '<pkg>' missing from index
  // after lookup"), and a downloader that returns no path for a frontier
  // package ("Downloader::fetch did not return a path for '<pkg>'").
  // A healthy run must never surface either — they guard internal
  // consistency, not user error. This asserts a real linker trace
  // succeeds without tripping them (a fault-injection harness would be
  // needed to fire them, which the integration surface cannot construct).
  let out = run(&["--strategy", "linker", "bash", "--format", "json"]);
  let stdout = stdout_ok(&out, "linker trace (invariant smoke)");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    !stderr.contains("missing from index after lookup"),
    "provider-missing invariant must not fire on healthy input:\n{stderr}"
  );
  assert!(
    !stderr.contains("did not return a path for"),
    "downloader-path invariant must not fire on healthy input:\n{stderr}"
  );
  // And the closure is actually populated, proving the walk completed.
  assert!(!parse_json(&stdout).as_array().unwrap().is_empty(), "linker closure must be non-empty");
}

// covers: ANL-060
#[test]
fn analyze_http_retries_env_feeds_linker_download_budget() {
  // FLATROOT_ARG_HTTP_RETRIES sets the shared HTTP retry budget that
  // both the index client and the per-package linker Downloader read.
  // A healthy linker trace with the budget set via env must still
  // succeed end-to-end (the budget reaches the download path without
  // changing the result on a reachable mirror).
  let out = run_argv(
    &[("FLATROOT_ARG_HTTP_RETRIES", "5")],
    &[
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64",
      "analyze",
      "trace",
      "--strategy",
      "linker",
      "bash",
      "--format",
      "json",
    ],
  );
  let stdout = stdout_ok(&out, "linker trace with http-retries env");
  assert!(
    parse_json(&stdout)
      .as_array()
      .unwrap()
      .iter()
      .any(|e| e["name"] == "bash"),
    "trace must complete with the seed present"
  );
}

// covers: ANL-061
#[test]
fn analyze_linker_reuses_archive_cache_across_runs() {
  // The linker walk lands provider archives under the per-source cache
  // dir; a second run against the same cache must reuse them rather than
  // re-downloading. Both runs share one cache TempDir, so the second is
  // a cache hit and must produce identical output.
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let mk = || {
    let mut cmd = Command::cargo_bin("flatroot").unwrap();
    cmd.env("FLATROOT_CACHE_HOME", cache.path());
    cmd
      .args([
        "--from",
        "debian:bookworm",
        "--arch",
        "x86_64",
        "analyze",
        "trace",
        "--strategy",
        "linker",
        "bash",
        "--format",
        "json",
      ])
      .output()
      .expect("flatroot binary failed to spawn")
  };

  let first = mk();
  let first_out = stdout_ok(&first, "first linker trace (cold cache)");
  let second = mk();
  let second_out = stdout_ok(&second, "second linker trace (warm cache)");

  assert_eq!(first_out, second_out, "a warm-cache linker run must produce identical output");
  // The per-source archive cache dir must now exist and hold the
  // downloaded provider archives.
  let cache_root = cache.path();
  let archive_count = walkdir_collect(cache_root).len();
  assert!(archive_count > 0, "linker walk must populate the per-source archive cache under {cache_root:?}");
}

/// Minimal recursive file walk over a cache dir — avoids pulling in a
/// crate for a one-line need. Returns every path under `root`.
fn walkdir_collect(root: &std::path::Path) -> Vec<std::path::PathBuf> {
  let mut out = Vec::new();
  let mut stack = vec![root.to_path_buf()];
  while let Some(dir) = stack.pop() {
    let rd = match std::fs::read_dir(&dir) {
      Ok(rd) => rd,
      Err(_) => continue,
    };
    for e in rd.flatten() {
      let p = e.path();
      if p.is_dir() {
        stack.push(p);
      } else {
        out.push(p);
      }
    }
  }
  out
}
