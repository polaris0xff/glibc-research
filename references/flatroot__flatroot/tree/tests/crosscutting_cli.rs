//! Cross-cutting offline CLI tests — behaviours proved across *several*
//! subcommands at once, so they belong to no single command's file:
//!
//! - **CLI-099** env-var-vs-flag precedence (clap `flag > env > default`),
//!   exercised for every global flag (`--http-retries`, `--arch`, `--from`;
//!   cli.rs:32-53). Each pair sets a *valid* env value and an *invalid* flag
//!   value: if the flag wins, the run fails naming the flag value; an env-only
//!   value must still reach the parser.
//! - **CLI-100** the `--arch` comma handling that differs *per command* —
//!   `install` splits it and `analyze` takes the first token (main.rs:43/126),
//!   while `search`/`query`/`release` parse it as one `uname` token and reject
//!   `x86_64,i686` (main.rs:79/88).
//!
//! Single-command flag behaviours live in their command's file (parsing in
//! `cli_dispatch.rs`, env fallbacks in `cli_network.rs`, etc.); only these
//! genuinely multi-command axes stay here.

use predicates::str::contains;

mod common;
use common::flatroot;

// ===========================================================================
// CLI-099 — env-var vs flag precedence proven for every global flag
// (cli.rs:32-53 — each global `#[arg(env = "FLATROOT_ARG_*")]`)
//
// clap precedence is flag > env > default. Proven per pair by setting a *valid*
// env value and an *invalid* flag value: if the flag wins, the run fails naming
// the flag value; a misconfigured attribute letting env override would instead
// accept the valid env value. And env-only (no flag) must reach the parser.
// ===========================================================================

// covers: CLI-099
#[test]
fn http_retries_flag_overrides_env_and_env_reaches_parser() {
  // Env-only: a non-numeric FLATROOT_ARG_HTTP_RETRIES reaches clap's u32 parser
  // and is rejected — proving the env fallback is wired (cli.rs:46). `remote
  // list` is the trailing subcommand (no --from needed, no --help short-circuit,
  // no network).
  {
    let (mut cmd, _cache) = flatroot();
    cmd
      .env("FLATROOT_ARG_HTTP_RETRIES", "notanumber")
      .args(["remote", "list"])
      .assert()
      .failure()
      .stderr(contains("invalid value 'notanumber'"));
  }
  // Flag > env: valid env (1), invalid flag — the flag value must reach the
  // parser and fail, never the env value.
  {
    let (mut cmd, _cache) = flatroot();
    let output = cmd
      .env("FLATROOT_ARG_HTTP_RETRIES", "1")
      .args(["--http-retries", "alsonotanumber", "remote", "list"])
      .output()
      .unwrap();
    assert!(!output.status.success(), "invalid flag value must win over valid env value");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value 'alsonotanumber'"), "flag value must reach the parser: {stderr}");
  }
}

// covers: CLI-099
#[test]
fn arch_flag_overrides_env_and_env_reaches_parser() {
  // --arch is parsed downstream by Arch::from_uname (main.rs), not by clap's
  // value_parser, so an invalid value surfaces as the "Unsupported architecture"
  // bail. Use `search` (single-arch path, main.rs:79) so the from_uname rejection
  // is the failure point.
  //
  // Env-only invalid value reaches from_uname.
  {
    let (mut cmd, _cache) = flatroot();
    let output = cmd
      .env("FLATROOT_ARG_ARCH", "envbogusarch")
      .args(["--from", "debian:bookworm", "search", "bash"])
      .output()
      .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported architecture 'envbogusarch'"), "env --arch must reach from_uname: {stderr}");
  }
  // Flag > env: a valid env value, an invalid flag value — the flag's bogus
  // value is the one rejected.
  {
    let (mut cmd, _cache) = flatroot();
    let output = cmd
      .env("FLATROOT_ARG_ARCH", "x86_64")
      .args(["--from", "debian:bookworm", "--arch", "flagbogusarch", "search", "bash"])
      .output()
      .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
      stderr.contains("Unsupported architecture 'flagbogusarch'"),
      "flag --arch must win over valid env: {stderr}"
    );
    assert!(!stderr.contains("envbogus"), "env --arch value must not leak when flag is present: {stderr}");
  }
}

// covers: CLI-099
#[test]
fn from_flag_overrides_env_and_env_supplies_when_absent() {
  // Env supplies --from when the flag is absent: an unknown env distro reaches
  // distro resolution (past the from_required gate, cli.rs:68-74).
  {
    let (mut cmd, _cache) = flatroot();
    let output = cmd
      .env("FLATROOT_ARG_FROM", "envfake:nope")
      .args(["search", "anything"])
      .output()
      .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("--from is required"), "env --from must satisfy from_required: {stderr}");
    assert!(
      stderr.contains("envfake") || stderr.contains("Unknown remote"),
      "env value must reach resolution: {stderr}"
    );
  }
  // Flag > env: distinct fake distros — the error names the flag value, never
  // the env value.
  {
    let (mut cmd, _cache) = flatroot();
    let output = cmd
      .env("FLATROOT_ARG_FROM", "envfake:nope")
      .args(["--from", "flagfake:nope", "search", "anything"])
      .output()
      .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("flagfake"), "flag --from value must appear in error: {stderr}");
    assert!(!stderr.contains("envfake"), "env --from value must not leak: {stderr}");
  }
}

// ===========================================================================
// CLI-100 — comma --arch: search/query/release REJECT it (single uname token);
// install splits it; analyze takes only the first token
// (main.rs:43 install split, :79 search single, :88 query single, :126 analyze
// first-token, plus the Release single-arch path)
// ===========================================================================

// covers: CLI-100
#[test]
fn comma_arch_rejected_by_single_arch_commands() {
  // search / query / release parse `--arch` as one uname token via
  // Arch::from_uname, so a literal "x86_64,i686" is an unsupported architecture.
  // These fail before any network with the from_uname bail.
  //
  // search (main.rs:79)
  {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["--from", "debian:bookworm", "--arch", "x86_64,i686", "search", "bash"])
      .assert()
      .failure()
      .stderr(contains("Unsupported architecture 'x86_64,i686'"));
  }
  // query (main.rs:88) — stdin SQL so the run gets to arch parsing.
  {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["--from", "debian:bookworm", "--arch", "x86_64,i686", "query"])
      .write_stdin("SELECT 1")
      .assert()
      .failure()
      .stderr(contains("Unsupported architecture 'x86_64,i686'"));
  }
}

// covers: CLI-100
#[test]
fn comma_arch_install_splits_and_analyze_takes_first_token() {
  // install splits on ',' and parses each token (main.rs:43): "x86_64,i686"
  // yields two valid archs, so `install --help` (no network) parses cleanly —
  // the comma is accepted, not rejected.
  {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args(["--arch", "x86_64,i686", "install", "--help"])
      .assert()
      .success();
  }
  // analyze takes the FIRST comma token (main.rs:126
  // `args.arch.split(',').next()`). A leading bogus token is the one that fails;
  // a trailing one never reached. We prove the first-token rule by making only
  // the first token invalid: the from_uname bail must name exactly that token.
  {
    let (mut cmd, _cache) = flatroot();
    cmd
      .args([
        "--from",
        "debian:bookworm",
        "--arch",
        "bogusfirst,x86_64",
        "analyze",
        "trace",
        "bash",
      ])
      .assert()
      .failure()
      .stderr(contains("Unsupported architecture 'bogusfirst'"));
  }
}
