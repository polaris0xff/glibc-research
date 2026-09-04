//! Install-command flag-shape, validation, and targeted-subsystem tests.
//!
//! Two kinds of test live here, both deliberately offline:
//!   1. Pure-validation / error paths that bail before any network or disk
//!      work (missing --from, missing -o, unsupported --arch). These assert
//!      the EXACT bail wording from the source so a future message change is
//!      caught.
//!   2. Targeted subsystem tests that drive a public API directly with a
//!      hand-built tree (the Postfix permission fixup, the Sandbox userns
//!      remedy text). These need no mirror at all.
//!
//! Every black-box invocation isolates its cache via FLATROOT_CACHE_HOME and
//! keeps the TempDir alive across the call.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

use assert_cmd::Command;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Error validation — bails before any network/disk work
// ---------------------------------------------------------------------------

/// `install -o <tmp> bash` with --from absent must bail with the
/// from-required example wording (cli.rs:68-74 via main.rs:34) and must NOT
/// have created the output tree or done any work. The output dir is named but
/// never created because the from-required gate fires before
/// `create_dir_all` at install.rs:99.
// covers: INST-002
#[test]
fn missing_from_rejected_before_any_work() {
  let root_parent = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  // A path that does not yet exist — we assert flatroot never creates it.
  let root = root_parent.path().join("would-be-root");
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env_remove("FLATROOT_ARG_FROM")
    .args(["install", "-o", root.to_str().unwrap(), "bash"])
    .output()
    .unwrap();
  assert!(!output.status.success(), "missing --from must fail");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("--from is required. Example: --from debian:bookworm"),
    "expected from-required example wording, got:\n{stderr}"
  );
  assert!(!root.exists(), "output dir must not be created when --from is missing");
}

/// `--from debian:bookworm install bash` with -o omitted must bail with the
/// output-required guidance (main.rs:35-37) and create no dir. The
/// from-required check passes, then the output `ok_or_else` fires before
/// `create_dir_all`.
// covers: INST-003
#[test]
fn missing_output_rejected_with_o_guidance() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env_remove("FLATROOT_ARG_INSTALL_OUTPUT")
    .args(["--from", "debian:bookworm", "install", "bash"])
    .output()
    .unwrap();
  assert!(!output.status.success(), "missing -o must fail");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("Install output directory is required. Use -o <PATH> or set FLATROOT_ARG_INSTALL_OUTPUT"),
    "expected output-required guidance, got:\n{stderr}"
  );
}

/// `--arch sparc64` must bail "Unsupported architecture 'sparc64'"
/// (arch.rs:130-138 via main.rs:43). The arch list is parsed before any
/// network work, so this fails fast with no output tree.
// covers: INST-027
#[test]
fn unsupported_arch_token_rejected() {
  let root_parent = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root = root_parent.path().join("root");
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "sparc64",
      "install",
      "-o",
      root.to_str().unwrap(),
      "bash",
    ])
    .output()
    .unwrap();
  assert!(!output.status.success(), "unsupported arch must fail");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("Unsupported architecture 'sparc64'"),
    "expected unsupported-architecture bail, got:\n{stderr}"
  );
  assert!(!root.exists(), "output dir must not be created for a bad arch");
}

// ---------------------------------------------------------------------------
// --exclude string parsing (main.rs:38-42)
//
// The exclude string is split on ',', each token trimmed, empties filtered.
// We can observe the *parsed* set's effect only through a real install
// (install_pipeline.rs) — but the trim/filter contract is pure string work
// in main.rs. The empty-string contract (INST-015) is exercised live in
// install_pipeline.rs; here we confirm the flag shape parses through clap.
// ---------------------------------------------------------------------------

/// The `--exclude` flag accepts a comma-list with surrounding whitespace and
/// a trailing empty token; clap parses it as a single String (split happens
/// in main.rs, not clap). Confirm the flag shape parses (no validation
/// error) via --help-gated parse.
// covers: INST-014
#[test]
fn exclude_comma_list_with_spaces_parses() {
  let cache = TempDir::new().unwrap();
  // `--help` short-circuits before any work but after flag parsing, proving
  // the `--exclude` value shape is accepted. The trim+filter semantics are
  // asserted behaviorally in install_pipeline.rs::exclude_drops_named_and_private_deps.
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["install", "--exclude", "perl, debconf ,", "--help"])
    .assert()
    .success();
}

// ---------------------------------------------------------------------------
// Postfix permission fixup (postfixes/mod.rs + postfixes/permissions.rs)
//
// Driven directly via the public `flatroot::postfixes::Postfix::apply` and
// `PermissionFix::apply` against a hand-built tree — no network, no mirror.
// ---------------------------------------------------------------------------

/// `Postfix::apply` walks the tree and flips mode-0 regular files to 0644,
/// leaves symlinks untouched (they are skipped, not chased), and never
/// disturbs a deliberately-set mode like 0700 (permissions.rs:47-49 skip
/// symlinks; :60-64 only act when mode == 0).
// covers: INST-038
#[test]
fn postfix_fixes_mode_zero_files_only() {
  let tree = TempDir::new().unwrap();
  let root = tree.path();

  // A mode-0 regular file (the unprivileged-build access gap).
  let locked = root.join("usr/bin/locked");
  fs::create_dir_all(locked.parent().unwrap()).unwrap();
  fs::write(&locked, b"#!/bin/true\n").unwrap();
  fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

  // A file with a deliberate restrictive mode — must be left exactly as-is.
  let deliberate = root.join("etc/secret");
  fs::create_dir_all(deliberate.parent().unwrap()).unwrap();
  fs::write(&deliberate, b"top secret\n").unwrap();
  fs::set_permissions(&deliberate, fs::Permissions::from_mode(0o700)).unwrap();

  // A symlink pointing at a mode-0 target — the LINK itself must be skipped.
  let link = root.join("usr/bin/locked-link");
  symlink("locked", &link).unwrap();

  flatroot::postfixes::Postfix::apply(root).expect("Postfix::apply must succeed");

  let locked_mode = fs::symlink_metadata(&locked).unwrap().permissions().mode() & 0o7777;
  assert_eq!(locked_mode, 0o644, "mode-0 regular file should become 0644");

  let deliberate_mode = fs::symlink_metadata(&deliberate).unwrap().permissions().mode() & 0o7777;
  assert_eq!(deliberate_mode, 0o700, "deliberate 0700 mode must be untouched");

  // The symlink's own metadata mode is not 0644 — it was never chmodded.
  // (We assert the link is still a symlink; chasing it would have failed to
  // find the target relative to cwd anyway.)
  assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink(), "symlink must remain a symlink, untouched");
}

/// `Postfix::apply` (via `PermissionFix::apply`) fails the whole run when a
/// correction cannot complete: it walks with `?`-propagation, so an
/// unreadable directory it cannot enumerate (read_dir fails) returns Err with
/// the "failed to read" context (permissions.rs:41), and the tree is never
/// handed back finished (postfixes/mod.rs:38 propagates).
// covers: INST-039
#[test]
fn postfix_fails_when_correction_cannot_complete() {
  let tree = TempDir::new().unwrap();
  let root = tree.path();

  // A subdirectory the walk must descend into, made unreadable+unsearchable
  // so `fs::read_dir` on it fails. Running as a non-root user, mode 0 on a
  // directory denies the read_dir, surfacing the error PermissionFix
  // propagates.
  let blocked = root.join("usr/blocked");
  fs::create_dir_all(&blocked).unwrap();
  fs::write(blocked.join("inner"), b"x").unwrap();
  fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();

  let result = flatroot::postfixes::Postfix::apply(root);

  // Restore perms so the TempDir can be cleaned up regardless of outcome.
  let _ = fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755));

  assert!(result.is_err(), "Postfix::apply must return Err when a directory it must walk is unreadable");
  let err = format!("{:#}", result.err().expect("expected an error"));
  assert!(err.contains("failed to read"), "error should carry the 'failed to read' context, got: {err}");
}

// ---------------------------------------------------------------------------
// Post-install userns remedy (postinstall/mod.rs:120-128)
//
// The post-install stage refuses to begin unless unprivileged user
// namespaces are available, surfacing a concrete remedy. We cannot disable
// userns from within a test, so this asserts the remedy path ONLY when the
// host genuinely lacks userns; otherwise it documents that the precondition
// holds and exits. The manifest-already-written invariant is covered live in
// install_pipeline.rs (the manifest is written at install.rs:133, strictly
// before PostInstallPlan::run at install.rs:144).
// ---------------------------------------------------------------------------

/// When unprivileged user namespaces are unavailable, a default install must
/// bail with the sysctl / --postinstall=none remedy (postinstall/mod.rs:120-128),
/// and because the manifest is written before post-install runs (install.rs:133
/// precedes :144) the rootfs already carries its `.flatroot/` record. On a
/// host that DOES have userns this branch is unreachable, so the assertion is
/// gated on `Sandbox::available()` reporting the capability missing.
// covers: INST-023
#[test]
fn postinstall_userns_unavailable_bails_with_remedy_manifest_already_written() {
  if flatroot::sandbox::Sandbox::available().is_ok() {
    // Userns IS available here — the failure path is unreachable. The
    // happy-path post-install behavior is covered by the live pipeline
    // tests. Nothing to assert in this environment.
    eprintln!("skipping: unprivileged user namespaces are available, remedy path unreachable");
    return;
  }

  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "-o",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .output()
    .unwrap();
  assert!(!output.status.success(), "default install must fail when userns is unavailable");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("Post-install requires unprivileged user namespaces"),
    "expected userns remedy, got:\n{stderr}"
  );
  assert!(stderr.contains("--postinstall=none"), "remedy must mention --postinstall=none, got:\n{stderr}");
  // The manifest was written before post-install ran.
  assert!(
    root.path().join(".flatroot/manifest").exists(),
    "manifest must already be written before the post-install failure"
  );
}
