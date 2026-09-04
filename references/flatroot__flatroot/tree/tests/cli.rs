//! CLI argument parsing and error handling tests.
//! These do not require network access.

use predicates::boolean::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::TempDir;

mod common;
use common::flatroot;

#[test]
fn help_flag() {
  let (mut cmd, _cache) = flatroot();
  cmd.arg("--help").assert().success();
}

#[test]
fn install_subcommand_help() {
  let (mut cmd, _cache) = flatroot();
  cmd.args(["install", "--help"]).assert().success();
}

#[test]
fn install_requires_packages() {
  let root = TempDir::new().unwrap();
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["install", "--output", root.path().to_str().unwrap()])
    .assert()
    .failure();
}

#[test]
fn invalid_remote_format() {
  let root = TempDir::new().unwrap();
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "notaremote",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .failure();
}

#[test]
fn no_subcommand() {
  let (mut cmd, _cache) = flatroot();
  cmd.assert().failure();
}

#[test]
fn snapshot_date_rejected_for_distros_without_a_dated_archive() {
  // Only distributions backed by a dated upstream archive (Debian, Ubuntu,
  // Arch) can honour an `@<date>` pin; for every other one the suffix would
  // otherwise ride verbatim into mirror URLs and cache keys and surface as a
  // mangled-URL fetch failure. The registry must refuse the request up
  // front, naming the distribution and its expected syntax, before any
  // network contact — proven here by the absence of any per-URL fetch line.
  let cases = [
    ("opensuse:tumbleweed@2026-01-01", "openSUSE"),
    ("fedora:42@2026-01-01", "Fedora"),
    ("alpine:v3.21@2026-01-01", "Alpine Linux"),
    ("centos:7@2026-01-01", "CentOS"),
    ("alma:9@2026-01-01", "AlmaLinux"),
    ("rocky:9@2026-01-01", "Rocky Linux"),
    ("cachyos:rolling@2026-01-01", "CachyOS"),
  ];
  for (from, display_name) in cases {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["--from", from, "search", "bash"])
      .assert()
      .failure()
      .stderr(contains(format!("{display_name} does not support @<date> snapshots. Expected: ")))
      .stderr(contains("fetching http").not());
  }
}

#[test]
fn comma_separated_arch_accepted() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--arch", "x86_64,i686", "install", "--help"])
    .assert()
    .success();
}

#[test]
fn remote_list() {
  let (mut cmd, _cache) = flatroot();
  let output = cmd.args(["remote", "list"]).output().unwrap();
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(output.status.success());
  for distro in [
    "debian", "ubuntu", "arch", "alpine", "centos", "fedora", "alma", "rocky", "opensuse", "cachyos",
  ] {
    assert!(stdout.contains(distro), "missing {} in remote list output", distro);
  }
}

// covers: CLI-040
#[test]
fn remote_list_default_format_is_plain() {
  // No --format → plain. Plain output is dotted KEY=VALUE; the default must
  // not be JSON, so the output must not parse as a JSON array.
  let (mut cmd, _cache) = flatroot();
  let output = cmd.args(["remote", "list"]).output().unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("remote.list.0.name="), "default plain output must use dotted KEY=VALUE:\n{}", stdout);
  assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err(), "default format must be plain, not JSON");
}

// covers: CLI-040
#[test]
fn remote_list_json_format_is_bijective_with_plain() {
  // `remote list` needs no network and no --from. The JSON form is a
  // top-level array of {format, name, status}; every distro name in JSON must
  // also appear in the plain form, and vice versa — the two encodings carry
  // the same data.
  let plain = {
    let (mut cmd, _cache) = flatroot();
    let out = cmd.args(["remote", "list"]).output().unwrap();
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).into_owned()
  };
  let json = {
    let (mut cmd, _cache) = flatroot();
    let out = cmd.args(["remote", "list", "--format", "json"]).output().unwrap();
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).into_owned()
  };
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("json --format must be valid JSON");
  let arr = parsed.as_array().expect("remote list json must be a top-level array");
  assert!(!arr.is_empty(), "remote list must list at least one distro");

  // Every distro name in JSON appears as a `.name=<name>` line in plain.
  let plain_names: Vec<&str> = plain
    .lines()
    .filter_map(|l| l.split_once(".name=").map(|(_, v)| v))
    .collect();
  for entry in arr {
    let obj = entry.as_object().expect("each entry must be an object");
    for key in ["format", "name", "status"] {
      assert!(obj.contains_key(key), "remote entry missing key {}: {}", key, entry);
    }
    let name = obj["name"].as_str().unwrap();
    assert!(plain_names.contains(&name), "distro {} present in JSON must appear in plain output:\n{}", name, plain);
  }
  // ...and every distro name in plain appears in JSON — the two are bijective.
  let json_names: Vec<&str> = arr.iter().filter_map(|e| e["name"].as_str()).collect();
  for name in &plain_names {
    assert!(json_names.contains(name), "distro {} present in plain must appear in JSON:\n{}", name, json);
  }
}

#[test]
fn remote_list_rejects_unknown_format() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["remote", "list", "--format", "frob"])
    .assert()
    .failure()
    .stderr(contains("invalid value 'frob'"));
}

#[test]
fn parallel_flag_accepted() {
  let (mut cmd, _cache) = flatroot();
  cmd.args(["install", "-p", "8", "--help"]).assert().success();
}

#[test]
fn http_retries_flag_accepted() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--http-retries", "5", "install", "--help"])
    .assert()
    .success();
}

#[test]
fn release_list_requires_from() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["release", "list"])
    .assert()
    .failure()
    .stderr(contains("--from"));
}

#[test]
fn search_requires_from() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["search", "firefox"])
    .assert()
    .failure()
    .stderr(contains("--from"));
}

#[test]
fn query_requires_from() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["query"])
    .write_stdin("SELECT 1")
    .assert()
    .failure()
    .stderr(contains("--from"));
}

#[test]
fn query_missing_file_errors() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "query",
      "/tmp/flatroot-nonexistent-sql-file.sql",
    ])
    .assert()
    .failure()
    .stderr(contains("Failed to read SQL file"));
}

#[test]
fn analyze_trace_help_lists_strategy_and_with() {
  let (mut cmd, _cache) = flatroot();
  let output = cmd.args(["analyze", "trace", "--help"]).output().unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("--strategy"), "trace --help must list --strategy");
  assert!(stdout.contains("--with"), "trace --help must list --with");
  assert!(!stdout.contains("--no-deps"), "--no-deps must not appear in trace --help");
  assert!(!stdout.contains("--no-linker"), "--no-linker must not appear in trace --help");
  assert!(!stdout.contains("--include-recommends"), "--include-recommends must not appear in trace --help");
  assert!(!stdout.contains("--include-suggests"), "--include-suggests must not appear in trace --help");
}

#[test]
fn analyze_trace_strategy_accepts_valid_values() {
  for value in ["declared", "linker", "declared,linker", "linker,declared"] {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["analyze", "trace", "--strategy", value, "--help"])
      .assert()
      .success();
  }
}

#[test]
fn analyze_trace_with_accepts_valid_values() {
  for value in ["recommends", "suggests", "recommends,suggests"] {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["analyze", "trace", "--with", value, "--help"])
      .assert()
      .success();
  }
}

#[test]
fn analyze_trace_strategy_rejects_unknown_value() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "analyze",
      "trace",
      "--strategy",
      "foobar",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("invalid value"));
}

#[test]
fn analyze_trace_with_rejects_unknown_value() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "analyze",
      "trace",
      "--with",
      "essential",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("invalid value"));
}

#[test]
fn analyze_trace_type_accepts_valid_values() {
  for value in ["package", "library", "path"] {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["analyze", "trace", "--type", value, "--help"])
      .assert()
      .success();
  }
}

#[test]
fn analyze_trace_type_rejects_unknown_value() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "analyze",
      "trace",
      "--type",
      "soname",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("invalid value"));
}

#[test]
fn search_type_accepts_valid_values() {
  for value in ["package", "library", "path"] {
    let (mut cmd, _cache) = flatroot();
    cmd.args(["search", "--type", value, "--help"]).assert().success();
  }
}

#[test]
fn search_type_rejects_unknown_value() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian:bookworm", "search", "--type", "soname", "bash"])
    .assert()
    .failure()
    .stderr(contains("invalid value"));
}

#[test]
fn install_type_accepts_valid_values() {
  for value in ["package", "library", "path"] {
    let (mut cmd, _cache) = flatroot();
    cmd.args(["install", "--type", value, "--help"]).assert().success();
  }
}

#[test]
fn install_type_rejects_unknown_value() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "-o",
      "/tmp/x",
      "--type",
      "soname",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("invalid value"));
}

#[test]
fn analyze_trace_requires_at_least_one_pattern() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian:bookworm", "analyze", "trace"])
    .assert()
    .failure();
}

#[test]
fn search_requires_at_least_one_pattern() {
  let (mut cmd, _cache) = flatroot();
  cmd.args(["--from", "debian:bookworm", "search"]).assert().failure();
}

// ============================================================================
// Environment variable fallbacks
// ============================================================================
//
// Every CLI flag has a `FLATROOT_ARG_*` env-var fallback. clap renders the
// var name into `--help` output when its `env = "..."` attribute is set, so
// asserting the var name appears in the relevant subcommand's help text
// confirms the wiring without needing network. One precedence test exercises
// flag-wins-over-env via `--from`, whose distinct values surface in the
// distro-not-found error path before any network work begins.

fn assert_help_contains_envs(args: &[&str], expected: &[&str]) {
  let (mut cmd, _cache) = flatroot();
  let output = cmd.args(args).output().unwrap();
  assert!(output.status.success(), "`{}` --help failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr));
  let stdout = String::from_utf8_lossy(&output.stdout);
  for var in expected {
    assert!(stdout.contains(var), "{} missing from `{}` help output:\n{}", var, args.join(" "), stdout);
  }
}

#[test]
fn env_global_vars_appear_in_help() {
  assert_help_contains_envs(
    &["--help"],
    &[
      "FLATROOT_ARG_FROM",
      "FLATROOT_ARG_ARCH",
      "FLATROOT_ARG_HTTP_RETRIES",
      "FLATROOT_ARG_VERBOSE",
    ],
  );
}

#[test]
fn env_install_vars_appear_in_help() {
  assert_help_contains_envs(
    &["install", "--help"],
    &[
      "FLATROOT_ARG_INSTALL_OUTPUT",
      "FLATROOT_ARG_INSTALL_WITH",
      "FLATROOT_ARG_INSTALL_POSTINSTALL",
      "FLATROOT_ARG_INSTALL_NO_DEPS",
      "FLATROOT_ARG_INSTALL_PARALLEL",
      "FLATROOT_ARG_INSTALL_EXCLUDE",
      "FLATROOT_ARG_INSTALL_TYPE",
    ],
  );
}

#[test]
fn env_search_vars_appear_in_help() {
  assert_help_contains_envs(&["search", "--help"], &["FLATROOT_ARG_SEARCH_TYPE", "FLATROOT_ARG_SEARCH_FORMAT"]);
}

#[test]
fn env_query_vars_appear_in_help() {
  assert_help_contains_envs(&["query", "--help"], &["FLATROOT_ARG_QUERY_FORMAT"]);
}

#[test]
fn env_export_vars_appear_in_help() {
  assert_help_contains_envs(&["export", "--help"], &["FLATROOT_ARG_EXPORT_FORMAT", "FLATROOT_ARG_EXPORT_TAG"]);
}

#[test]
fn env_remote_list_var_appears_in_help() {
  assert_help_contains_envs(&["remote", "list", "--help"], &["FLATROOT_ARG_REMOTE_LIST_FORMAT"]);
}

#[test]
fn env_release_list_var_appears_in_help() {
  assert_help_contains_envs(&["release", "list", "--help"], &["FLATROOT_ARG_RELEASE_LIST_FORMAT"]);
}

#[test]
fn env_analyze_trace_vars_appear_in_help() {
  assert_help_contains_envs(
    &["analyze", "trace", "--help"],
    &[
      "FLATROOT_ARG_ANALYZE_TRACE_TYPE",
      "FLATROOT_ARG_ANALYZE_TRACE_FORMAT",
      "FLATROOT_ARG_ANALYZE_TRACE_STRATEGY",
      "FLATROOT_ARG_ANALYZE_TRACE_WITH",
    ],
  );
}

#[test]
fn env_arg_from_satisfies_from_required() {
  // FLATROOT_ARG_FROM should satisfy `Args::from_required` so a no-flag
  // invocation gets past the "missing --from" gate. We point at an unknown
  // distro so the command fails *after* the from-required check with a
  // distro-resolution error — that distinguishes "env didn't supply the flag"
  // (would say "--from is required") from "env supplied the flag and resolution
  // failed" (says "Unknown remote").
  let (mut cmd, _cache) = flatroot();
  let output = cmd
    .env("FLATROOT_ARG_FROM", "envfake:nope")
    .args(["search", "anything"])
    .output()
    .unwrap();
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(!stderr.contains("--from is required"), "FLATROOT_ARG_FROM did not reach from_required: {}", stderr);
  assert!(
    stderr.contains("envfake") || stderr.contains("Unknown remote"),
    "expected unknown-distro error mentioning the env value: {}",
    stderr
  );
}

#[test]
fn env_arg_from_loses_to_cli_flag() {
  // Both env and CLI flag set with distinct fake distros. clap's documented
  // precedence is flag > env > default, so the unknown-distro error must
  // mention the flag value, never the env value. A misconfigured env
  // attribute that overrides the flag would surface the env value here.
  let (mut cmd, _cache) = flatroot();
  let output = cmd
    .env("FLATROOT_ARG_FROM", "envfake:nope")
    .args(["--from", "flagfake:nope", "search", "anything"])
    .output()
    .unwrap();
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("flagfake"), "expected flag value in error: {}", stderr);
  assert!(!stderr.contains("envfake"), "env value leaked into error: {}", stderr);
}

// ============================================================================
// `--with` / `--postinstall` flag-shape tests
// ============================================================================

#[test]
fn install_with_accepts_recommends_and_suggests() {
  for value in ["recommends", "suggests", "recommends,suggests"] {
    let (mut cmd, _cache) = flatroot();
    cmd.args(["install", "--with", value, "--help"]).assert().success();
  }
}

#[test]
fn install_with_rejects_unknown_value() {
  let root = TempDir::new().unwrap();
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--with",
      "essential",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("invalid value"));
}

#[test]
fn install_postinstall_accepts_phase_values() {
  for value in [
    "ldconfig",
    "scripts",
    "hooks",
    "ldconfig,scripts",
    "ldconfig,hooks",
    "scripts,hooks",
    "ldconfig,scripts,hooks",
    "none",
  ] {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["install", "--postinstall", value, "--help"])
      .assert()
      .success();
  }
}

#[test]
fn install_postinstall_rejects_unknown_value() {
  let root = TempDir::new().unwrap();
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall",
      "ldconfig,profile",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("invalid value"));
}

#[test]
fn install_postinstall_rejects_none_with_phases() {
  // `--postinstall=none,ldconfig` parses through clap (both are valid
  // ValueEnum variants) but the dispatch site rejects mixing.
  let root = TempDir::new().unwrap();
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall",
      "none,ldconfig",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("--postinstall=none cannot be combined"));
}

#[test]
fn install_help_surface_only_lists_with_and_postinstall() {
  // install's soft-dep surface is `--with`; its post-install surface is
  // `--postinstall`. No bare flag exposes either selection — soft-deps are
  // values of `--with`, post-install steps are values of `--postinstall`.
  let (mut cmd, _cache) = flatroot();
  let output = cmd.args(["install", "--help"]).output().unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("--with"), "install --help must list --with");
  assert!(stdout.contains("--postinstall"), "install --help must list --postinstall");
  assert!(!stdout.contains("--recommends"), "--recommends must not appear in install --help");
  assert!(!stdout.contains("--optional"), "--optional must not appear in install --help");
  assert!(!stdout.contains("--no-post"), "--no-post must not appear in install --help");
}

// ── --match {any,all} combine flag ─────────────────────────────────────────

const MATCH_ALL_REFUSAL: &str = "--match all is meaningful only with --type path or --type library";

// covers: CLI-085, CLI-086, CLI-087
#[test]
fn match_all_rejected_for_type_package() {
  // The refusal is the same for all three commands, and lands before any
  // catalogue is fetched (no `fetching http` line).
  let root = TempDir::new().unwrap();
  let out = root.path().to_str().unwrap();

  let cases: [Vec<&str>; 3] = [
    vec![
      "--from",
      "debian:bookworm",
      "install",
      "-o",
      out,
      "--type",
      "package",
      "--match",
      "all",
      "bash",
    ],
    vec![
      "--from",
      "debian:bookworm",
      "search",
      "--type",
      "package",
      "--match",
      "all",
      "bash",
    ],
    vec![
      "--from",
      "debian:bookworm",
      "analyze",
      "trace",
      "--type",
      "package",
      "--match",
      "all",
      "bash",
    ],
  ];
  for args in cases {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(&args)
      .assert()
      .failure()
      .stderr(contains(MATCH_ALL_REFUSAL))
      .stderr(contains("fetching http").not());
  }
}

// covers: CLI-088, CLI-089, CLI-090
#[test]
fn match_guard_is_narrow() {
  // The refusal fires only for `--type package --match all`. `package` + the
  // default `any`, and `library`/`path` + `all`, all pass the guard — proven
  // with an invalid `--from` so the run fails later (remote parsing) and never
  // emits the match-all message, without touching the network.
  let root = TempDir::new().unwrap();
  let out = root.path().to_str().unwrap();

  let cases: [Vec<&str>; 3] = [
    vec![
      "--from",
      "notaremote",
      "install",
      "-o",
      out,
      "--type",
      "package",
      "bash",
    ],
    vec![
      "--from",
      "notaremote",
      "install",
      "-o",
      out,
      "--type",
      "library",
      "--match",
      "all",
      "libfoo.so.1",
    ],
    vec![
      "--from",
      "notaremote",
      "install",
      "-o",
      out,
      "--type",
      "path",
      "--match",
      "all",
      "usr/bin/foo",
    ],
  ];
  for args in cases {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(&args)
      .assert()
      .failure()
      .stderr(contains(MATCH_ALL_REFUSAL).not());
  }
}

// covers: CLI-091, CLI-092
#[test]
fn match_flag_in_help_with_possible_values() {
  let subs: [Vec<&str>; 3] = [
    vec!["install", "--help"],
    vec!["search", "--help"],
    vec!["analyze", "trace", "--help"],
  ];
  for sub in subs {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(&sub)
      .assert()
      .success()
      .stdout(contains("--match"))
      .stdout(contains("any"))
      .stdout(contains("all"));
  }
}

// covers: CLI-093
#[test]
fn match_invalid_value_rejected() {
  let root = TempDir::new().unwrap();
  let out = root.path().to_str().unwrap();
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "-o",
      out,
      "--match",
      "bogus",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("bogus"));
}

// covers: CLI-094, CLI-095, CLI-096
#[test]
fn match_env_fallback_is_read() {
  // Setting the env equivalent to `all` with `--type package` trips the same
  // refusal the flag would — proof the env fallback is honoured — for each of
  // the three commands' env vars, and before any network contact.
  let root = TempDir::new().unwrap();
  let out = root.path().to_str().unwrap();

  let cases: [(&str, Vec<&str>); 3] = [
    (
      "FLATROOT_ARG_INSTALL_MATCH",
      vec![
        "--from",
        "debian:bookworm",
        "install",
        "-o",
        out,
        "--type",
        "package",
        "bash",
      ],
    ),
    ("FLATROOT_ARG_SEARCH_MATCH", vec!["--from", "debian:bookworm", "search", "--type", "package", "bash"]),
    (
      "FLATROOT_ARG_ANALYZE_TRACE_MATCH",
      vec![
        "--from",
        "debian:bookworm",
        "analyze",
        "trace",
        "--type",
        "package",
        "bash",
      ],
    ),
  ];
  for (env_key, args) in cases {
    let (mut cmd, _cache) = flatroot();
    cmd
      .env(env_key, "all")
      .args(&args)
      .assert()
      .failure()
      .stderr(contains(MATCH_ALL_REFUSAL))
      .stderr(contains("fetching http").not());
  }
}

// covers: CLI-097
#[test]
fn match_flag_overrides_env() {
  // Env says `all`, the explicit flag says `any`; the flag wins, so the
  // package-mode refusal does NOT fire (clap precedence: CLI over env).
  let root = TempDir::new().unwrap();
  let out = root.path().to_str().unwrap();
  let (mut cmd, _cache) = flatroot();
  cmd
    .env("FLATROOT_ARG_INSTALL_MATCH", "all")
    .args([
      "--from",
      "notaremote",
      "install",
      "-o",
      out,
      "--type",
      "package",
      "--match",
      "any",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains(MATCH_ALL_REFUSAL).not());
}
