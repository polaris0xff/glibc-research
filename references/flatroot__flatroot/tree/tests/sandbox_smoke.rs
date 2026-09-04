//! Sandbox integration smoke test — installs a fresh debian:bookworm bash
//! rootfs into a tempdir and exercises the full sandbox dance against it.
//! Self-contained: no pre-built fixture required.

use assert_cmd::Command;
use flatroot::sandbox::Sandbox;
use tempfile::TempDir;

#[test]
fn sandbox_run_echo() {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let root = TempDir::new().expect("tempdir for sandbox rootfs");

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .assert()
    .success();

  assert!(
    root.path().join("bin/bash").exists(),
    "flatroot install completed but bin/bash is missing under {}",
    root.path().display()
  );

  let env: Vec<(&str, &str)> = vec![("PATH", "/usr/local/bin:/usr/bin:/bin:/sbin"), ("HOME", "/root")];
  let cmd = vec![
    "/bin/bash",
    "-c",
    "echo hello-from-sandbox && ls /dev/ && echo proc-test && ls /proc/self/ | head -3",
  ];

  let output = Sandbox::new(root.path())
    .run(&cmd, &env)
    .expect("Sandbox::run should succeed");

  let stdout = String::from_utf8_lossy(&output.stdout);
  let stderr = String::from_utf8_lossy(&output.stderr);
  eprintln!("exit: {}", output.status);
  eprintln!("stdout:\n{}", stdout);
  eprintln!("stderr:\n{}", stderr);

  assert!(output.status.success(), "command should exit 0, got: {}", output.status);
  assert!(stdout.contains("hello-from-sandbox"), "should see echo output in stdout");
  assert!(stdout.contains("null"), "/dev/null should exist");
  assert!(stdout.contains("proc-test"), "/proc should be mounted");
}

/// When unprivileged user namespaces are unavailable, a default install aborts
/// the post-install stage with a concrete remediation message: the "requires
/// unprivileged user namespaces" reason, the sysctl hint, and the
/// `--postinstall=none` bypass (postinstall/mod.rs:120-128, sandbox.rs:60-91).
/// On a permissive host this failure branch is unreachable, so the assertion is
/// gated on `Sandbox::available()` reporting the capability missing.
// covers: POST-009
#[test]
fn postinstall_aborts_with_remediation_when_userns_unavailable() {
  if Sandbox::available().is_ok() {
    eprintln!("skipping: unprivileged user namespaces are available, abort path unreachable");
    return;
  }

  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let root = TempDir::new().expect("tempdir for sandbox rootfs");

  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "bash",
    ])
    .output()
    .expect("flatroot failed to execute");

  assert!(!output.status.success(), "default install must fail when userns is unavailable");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("Post-install requires unprivileged user namespaces"),
    "expected the userns remediation, got:\n{stderr}"
  );
  assert!(
    stderr.contains("sysctl kernel.unprivileged_userns_clone=1"),
    "remediation must include the sysctl hint:\n{stderr}"
  );
  assert!(stderr.contains("--postinstall=none"), "remediation must include the --postinstall=none bypass:\n{stderr}");
}
