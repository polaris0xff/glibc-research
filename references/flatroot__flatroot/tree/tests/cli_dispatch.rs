//! CLI parsing and dispatch-level validation tests that need no network.
//!
//! These exercise the boundary between clap (the parser in `src/cli.rs`) and
//! the dispatcher (`src/main.rs`): clap rejects for unknown subcommands,
//! unknown flags, missing positionals, missing subcommands, and bad enum/number
//! values; dispatch `bail!`s for an unsupported `--arch`, an empty `--strategy`,
//! a missing `--from`/`--output`, and the order in which those checks fire. Each
//! error substring below is copied verbatim from clap 4.6 (`clap_builder`
//! `error/format.rs`) or from the `anyhow::bail!`/`anyhow!` call sites in
//! `src/main.rs`, `src/cli.rs`, and `src/arch.rs`.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

mod common;
use common::flatroot;

// ============================================================================
// Version, unknown subcommand, unknown flag
// ============================================================================

// covers: CLI-003
#[test]
fn version_flag_prints_crate_version_and_exits_zero() {
  // clap's `version` derive prints `<name> <version>`. The version is the
  // crate's `CARGO_PKG_VERSION`, which an integration test compiled in the
  // same crate reads at build time — so this asserts the real value, not a
  // hardcoded string.
  let (mut cmd, _cache) = flatroot();
  let output = cmd.arg("--version").output().unwrap();
  assert!(output.status.success(), "--version must exit 0");
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(
    stdout.contains(env!("CARGO_PKG_VERSION")),
    "--version stdout must contain the crate version {}: {}",
    env!("CARGO_PKG_VERSION"),
    stdout
  );
}

// covers: CLI-004
#[test]
fn unknown_subcommand_is_rejected() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .arg("frobnicate")
    .assert()
    .failure()
    .stderr(contains("unrecognized subcommand 'frobnicate'"));
}

// covers: CLI-005
#[test]
fn unknown_global_flag_is_rejected() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--nonexistent-flag", "remote", "list"])
    .assert()
    .failure()
    .stderr(contains("unexpected argument '--nonexistent-flag' found"));
}

// ============================================================================
// --arch parsing and dispatch-level validation
// ============================================================================

// covers: CLI-009
#[test]
fn arch_supported_uname_tokens_parse() {
  // clap stores `--arch` as a bare String, so every supported uname token —
  // including the `x86` alias for i686 — must parse. The String→Arch mapping
  // happens in dispatch; this asserts the parse layer accepts the vocabulary.
  for token in ["x86_64", "x86", "i686", "aarch64", "armv7l", "riscv64"] {
    let (mut cmd, _cache) = flatroot();
    cmd.args(["--arch", token, "search", "--help"]).assert().success();
  }
}

// covers: CLI-010
#[test]
fn arch_unsupported_token_is_rejected_before_network() {
  // `Arch::from_uname` (src/arch.rs:137) bails for an unknown token. For the
  // `search` arm this runs after `from_required` but before any fetch, so the
  // error appears with no network. `--from` is supplied so the from-required
  // gate passes and the arch check is what fails.
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian:bookworm", "--arch", "sparc64", "search", "bash"])
    .assert()
    .failure()
    .stderr(contains("Unsupported architecture 'sparc64'"));
}

// covers: CLI-013
#[test]
fn search_arch_comma_list_bails_on_literal_string() {
  // search/query/release feed the whole `--arch` string to `Arch::from_uname`
  // with no comma split (main.rs:79/88), so a comma list is one unknown token.
  // The bail names the literal comma-joined string.
  let (mut cmd, _cache) = flatroot();
  cmd
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64,aarch64",
      "search",
      "bash",
    ])
    .assert()
    .failure()
    .stderr(contains("Unsupported architecture 'x86_64,aarch64'"));
}

// covers: CLI-013
#[test]
fn query_arch_comma_list_bails_on_literal_string() {
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian:bookworm", "--arch", "x86_64,aarch64", "query"])
    .write_stdin("SELECT 1")
    .assert()
    .failure()
    .stderr(contains("Unsupported architecture 'x86_64,aarch64'"));
}

// covers: CLI-014, CLI-067
#[test]
fn arch_env_fallback_supplies_arch_and_loses_to_flag() {
  // `FLATROOT_ARG_ARCH` supplies `--arch` when the flag is absent: an unknown
  // env value reaches `Arch::from_uname` and is named in the bail. This proves
  // the env fallback is wired without needing a valid arch / network.
  let (mut cmd, _cache) = flatroot();
  cmd
    .env("FLATROOT_ARG_ARCH", "envonlyarch")
    .args(["--from", "debian:bookworm", "search", "bash"])
    .assert()
    .failure()
    .stderr(contains("Unsupported architecture 'envonlyarch'"));

  // With both env and flag set, clap precedence is flag > env: the flag value
  // is what reaches the bail, never the env value.
  let (mut cmd, _cache) = flatroot();
  let output = cmd
    .env("FLATROOT_ARG_ARCH", "envonlyarch")
    .args(["--from", "debian:bookworm", "--arch", "flagonlyarch", "search", "bash"])
    .output()
    .unwrap();
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("Unsupported architecture 'flagonlyarch'"), "flag value must win: {}", stderr);
  assert!(!stderr.contains("envonlyarch"), "env value must not leak when flag is set: {}", stderr);
}

// ============================================================================
// --http-retries: default and non-numeric rejection
// ============================================================================

// covers: CLI-015
#[test]
fn http_retries_default_is_three_in_help() {
  // The `default_value_t = 3` (cli.rs:46) renders into the global --help text.
  let (mut cmd, _cache) = flatroot();
  let output = cmd.arg("--help").output().unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(
    stdout.contains("--http-retries") && stdout.contains("[default: 3]"),
    "global --help must show --http-retries default 3:\n{}",
    stdout
  );
}

// covers: CLI-015
#[test]
fn http_retries_non_numeric_is_clap_error() {
  // `--http-retries` is a u32; a non-numeric value fails clap's value parser.
  // `remote list` is the trailing subcommand (no --from, no network) so the
  // value-validation error is what fails the run — no `--help` that could
  // short-circuit to a zero exit before validation.
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--http-retries", "notanumber", "remote", "list"])
    .assert()
    .failure()
    .stderr(contains("invalid value 'notanumber'"));
}

// ============================================================================
// install: --from / --output requirement ordering
// ============================================================================

// covers: CLI-019
#[test]
fn install_missing_output_bails_at_dispatch() {
  // --from present, --output absent: from_required passes, then the dispatch
  // bail for the missing output fires (main.rs:35-37).
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["--from", "debian:bookworm", "install", "bash"])
    .assert()
    .failure()
    .stderr(contains("Install output directory is required"));
}

// covers: CLI-021
#[test]
fn install_missing_from_fires_before_missing_output() {
  // Neither --from nor --output: from_required runs first (main.rs:34), so the
  // --from error fires and the output-required message does NOT appear.
  let (mut cmd, _cache) = flatroot();
  let output = cmd.args(["install", "bash"]).output().unwrap();
  assert!(!output.status.success());
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("--from is required"), "expected the --from error first: {}", stderr);
  assert!(
    !stderr.contains("Install output directory is required"),
    "the output-required message must not fire before the --from check: {}",
    stderr
  );
}

// ============================================================================
// Subcommands that require an action
// ============================================================================

// covers: CLI-041
#[test]
fn remote_without_action_is_rejected() {
  // A non-Optional `#[command(subcommand)]` field makes clap-derive emit
  // `arg_required_else_help`, so a missing action prints the subcommand's help
  // and exits non-zero rather than a terse "requires a subcommand" string.
  let (mut cmd, _cache) = flatroot();
  cmd
    .arg("remote")
    .assert()
    .failure()
    .stderr(contains("Usage:"))
    .stderr(contains("Commands:"));
}

// covers: CLI-045
#[test]
fn release_without_action_is_rejected_before_from_check() {
  // With no action, clap prints the subcommand's help and exits non-zero at
  // PARSE time — before any dispatch — so the `from_required` check never runs
  // even though no --from was supplied.
  let (mut cmd, _cache) = flatroot();
  let output = cmd.arg("release").output().unwrap();
  assert!(!output.status.success());
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("Usage:") && stderr.contains("Commands:"),
    "release with no action must fail at parse with the subcommand help: {}",
    stderr
  );
  assert!(!stderr.contains("--from is required"), "the parse error must fire before the from check: {}", stderr);
}

// covers: CLI-051
#[test]
fn analyze_without_action_is_rejected() {
  // Same clap-derive `arg_required_else_help` behaviour as `remote`/`release`:
  // a missing action prints the subcommand help and exits non-zero.
  let (mut cmd, _cache) = flatroot();
  cmd
    .arg("analyze")
    .assert()
    .failure()
    .stderr(contains("Usage:"))
    .stderr(contains("Commands:"));
}

// ============================================================================
// export: positional and format validation
// ============================================================================

// covers: CLI-046
#[test]
fn export_requires_two_positionals() {
  // export has two required positionals: src_dir and output. With only one,
  // clap reports the missing required `<OUTPUT>`.
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["export", "/tmp/rootfs"])
    .assert()
    .failure()
    .stderr(contains("the following required arguments were not provided:"))
    .stderr(contains("<OUTPUT>"));
}

// covers: CLI-049
#[test]
fn export_unknown_format_is_rejected() {
  // ExportFormat is a value_enum; an unknown --format value fails at parse.
  let (mut cmd, _cache) = flatroot();
  cmd
    .args(["export", "/tmp/rootfs", "/tmp/out.tar", "--format", "frob"])
    .assert()
    .failure()
    .stderr(contains("invalid value 'frob'"));
}

// ============================================================================
// analyze trace: empty --strategy and ordering vs from_required
// ============================================================================

// covers: CLI-056
#[test]
fn analyze_trace_empty_strategy_bails_before_from_required() {
  // `--strategy=` supplies an empty value to the value-enum arg, which clap
  // rejects at PARSE time ("a value is required ... but none was supplied"),
  // before any dispatch — so with no --from the parse error wins over the
  // from-requirement check.
  let (mut cmd, _cache) = flatroot();
  let output = cmd.args(["analyze", "trace", "--strategy=", "bash"]).output().unwrap();
  assert!(!output.status.success());
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("a value is required for '--strategy"),
    "empty --strategy must be rejected at parse time: {}",
    stderr
  );
  assert!(!stderr.contains("--from is required"), "the parse error must fire before the from check: {}", stderr);
}

// ============================================================================
// Global-flag scoping (before vs after the subcommand)
// ============================================================================

// covers: CLI-071
#[test]
fn from_flag_is_top_level_accepted_before_subcommand_only() {
  // `--from` is a top-level flag on `Args`, not a clap `global` flag: it is
  // accepted before the subcommand and reaches dispatch (an unknown distro then
  // fails downstream, naming the value), but placed AFTER the subcommand it is a
  // clap parse error, since `search` declares no `--from` of its own.
  let before = {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["--from", "flagfake:nope", "search", "anything"])
      .output()
      .unwrap()
  };
  let after = {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["search", "--from", "flagfake:nope", "anything"])
      .output()
      .unwrap()
  };

  // Before the subcommand: parsed, reaches dispatch, fails on the unknown distro.
  let before_err = String::from_utf8_lossy(&before.stderr);
  assert!(!before.status.success(), "before: expected failure for the unknown distro");
  assert!(
    !before_err.contains("unexpected argument") && !before_err.contains("unrecognized"),
    "before: --from must be accepted ahead of the subcommand, not a clap error: {before_err}"
  );
  assert!(before_err.contains("flagfake"), "before: the from value must reach dispatch: {before_err}");

  // After the subcommand: clap rejects it as an unexpected argument.
  let after_err = String::from_utf8_lossy(&after.stderr);
  assert!(!after.status.success(), "after: --from after the subcommand must be rejected");
  assert!(
    after_err.contains("unexpected argument '--from'"),
    "after: --from after the subcommand is a clap parse error: {after_err}"
  );
}

// ============================================================================
// Cache-dir resolution failure surfaces as a non-zero exit
// ============================================================================

// covers: CLI-074
#[test]
fn cache_dir_unresolvable_propagates_nonzero() {
  // `Cache::dir_resolve()` (internal/cache.rs:46) does `create_dir_all(root)`.
  // Pointing FLATROOT_CACHE_HOME at a path *under a regular file* makes that
  // call fail (a file cannot have child directories), and the `?` propagates a
  // non-zero exit before any network work. A real file is used as the parent so
  // the failure is the genuine filesystem error, not a contrived one.
  let parent = TempDir::new().unwrap();
  let path_file_blocker = parent.path().join("not-a-dir");
  std::fs::write(&path_file_blocker, b"x").unwrap();
  let path_dir_unmakeable = path_file_blocker.join("cache");

  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  let output = cmd
    .env("FLATROOT_CACHE_HOME", &path_dir_unmakeable)
    .args(["--from", "debian:bookworm", "search", "bash"])
    .output()
    .unwrap();
  assert!(!output.status.success(), "an unresolvable cache dir must fail the run: {:?}", output.status);
}

// ===========================================================================
// CLI-105 — install required-flag check ordering: missing --from reported before
// missing -o, before positional packages (cli.rs:144 clap positionals;
// main.rs:34-37 from_required then output check)
// ===========================================================================

// covers: CLI-105
#[test]
fn install_required_flag_check_ordering() {
  // Missing positional patterns is a clap parse error (parser.rs required=true)
  // and fires before main.rs even runs: `install` with no patterns fails at
  // parse, naming the required arg.
  {
    let (mut cmd, _cache) = flatroot();
    let output = cmd.args(["install"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // clap's required-positional error mentions PATTERNS (the arg name).
    assert!(
      stderr.to_lowercase().contains("patterns") || stderr.contains("required"),
      "clap must flag missing patterns: {stderr}"
    );
  }

  // With packages present but BOTH --from and -o missing, main.rs:34 checks
  // from_required FIRST — so the "--from is required" message wins, not the
  // "-o <PATH>" output message.
  {
    let (mut cmd, _cache) = flatroot();
    let output = cmd.args(["install", "bash"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--from is required"), "from_required must be reported before the -o check: {stderr}");
    assert!(
      !stderr.contains("Install output directory is required"),
      "the -o error must not win while --from is missing: {stderr}"
    );
  }

  // With --from present but -o missing, the output check (main.rs:35-37) is what
  // fails, naming the -o requirement.
  {
    let (mut cmd, _cache) = flatroot();
    let output = cmd
      .args(["--from", "debian:bookworm", "install", "bash"])
      .output()
      .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
      stderr.contains("Install output directory is required"),
      "the -o check fires once --from is satisfied: {stderr}"
    );
    assert!(stderr.contains("FLATROOT_ARG_INSTALL_OUTPUT"), "the -o error names its env fallback: {stderr}");
  }
}
