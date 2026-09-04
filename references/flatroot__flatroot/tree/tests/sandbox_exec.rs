//! Sandbox execution semantics (POST area, prefix POST).
//!
//! Exercises `flatroot::sandbox::Sandbox` directly: the availability probe, the
//! empty-command guard (which bails before any fork), the exit-code translation
//! for normal / signalled / exec-failure / setup-failure children, the in-
//! namespace uid/gid-0 mapping with setgroups denied, the /dev device set, and
//! the separate stdout/stderr capture with a wiped-then-provided environment.
//!
//! Every test that actually forks into a namespace is gated on
//! `Sandbox::available()` so a locked-down CI host yields a documented skip
//! rather than a spurious failure. The exit-code and identity tests reuse a
//! single real Debian `bash` rootfs (it ships a working `/bin/sh` + loader the
//! sandbox can pivot into and exec inside).

mod post;

use std::path::Path;

use flatroot::sandbox::Sandbox;
use tempfile::TempDir;

/// Run one command in the sandbox or panic with context.
fn run(root: &Path, cmd: &[&str], env: &[(&str, &str)]) -> std::process::Output {
  Sandbox::new(root)
    .run(cmd, env)
    .expect("Sandbox::run should return Ok(Output)")
}

// covers: POST-010
#[test]
fn sandbox_available_succeeds_on_permissive_host() {
  // The availability probe forks a child that attempts unshare(CLONE_NEWUSER),
  // signals the verdict over a pipe, and is reaped (sandbox.rs:60-91). On a
  // permissive host the observable contract is simply Ok(()).
  match Sandbox::available() {
    Ok(()) => {}
    Err(reason) => {
      // Locked-down host: the probe correctly reports the capability missing.
      assert!(
        reason
          .to_string()
          .contains("unprivileged user namespaces are not available"),
        "unexpected availability failure reason: {reason}"
      );
      eprintln!("skipping deeper sandbox assertions: {reason}");
    }
  }
}

// covers: POST-013
#[test]
fn sandbox_run_empty_command_bails_before_fork() {
  // An empty command slice is rejected with a clear bail BEFORE any fork or
  // namespace work (sandbox.rs:108-110). No rootfs needs to exist.
  let tmp = TempDir::new().unwrap();
  let err = Sandbox::new(tmp.path())
    .run(&[], &[])
    .err()
    .expect("empty command must bail");
  assert!(
    err.to_string().contains("Sandbox::run called with empty command"),
    "expected empty-command bail, got: {err}"
  );
}

// covers: POST-016
#[test]
fn sandbox_run_missing_command_yields_exit_127() {
  // exec of a command that does not exist in the rootfs results in the child
  // printing `exec failed: <cmd>` and exiting 127 (sandbox.rs:185-187). An
  // empty pivotable tempdir is enough — the failure is at exec, after pivot.
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let tmp = TempDir::new().unwrap();
  let out = run(tmp.path(), &["/no-such-binary-xyz"], &[]);
  assert_eq!(out.status.code(), Some(127), "missing command must exit 127");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("exec failed: /no-such-binary-xyz"), "stderr must name the failed exec, got:\n{stderr}");
}

// covers: POST-017
#[test]
fn sandbox_run_mount_setup_failure_yields_exit_126() {
  // When the rootfs cannot be presented (here: the rootfs path does not exist,
  // so the bind-mount of the tree onto itself fails), the child reports the
  // mount/pivot context and exits 126 (sandbox.rs:146-149/161-164/259-306).
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let tmp = TempDir::new().unwrap();
  let missing = tmp.path().join("does-not-exist");
  let out = run(&missing, &["/bin/true"], &[]);
  assert_eq!(out.status.code(), Some(126), "mount-setup failure must exit 126");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("mount setup failed"), "stderr must carry mount-setup context, got:\n{stderr}");
  assert!(stderr.contains("bind-mount rootfs"), "stderr must carry the failing bind-mount context, got:\n{stderr}");
}

// covers: POST-012, POST-014, POST-015, POST-018, POST-019
#[test]
fn sandbox_run_against_real_rootfs_covers_identity_codes_dev_and_capture() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  post::rootfs_debian_bash(cache.path(), root.path());

  let env: Vec<(&str, &str)> = vec![
    ("PATH", "/usr/local/bin:/usr/bin:/bin:/sbin"),
    ("HOME", "/root"),
    ("FLATROOT_PROVIDED", "marker-value"),
  ];

  // POST-012: in-namespace identity is uid 0 / gid 0, and setgroups is denied
  // (sandbox.rs:239-247). bash exposes $EUID/$UID as builtins (no coreutils
  // needed); /proc/self/setgroups reads back "deny".
  let id_out = run(
    root.path(),
    &[
      "/bin/bash",
      "-c",
      "echo uid=$UID euid=$EUID; \
       while read -r k v _; do [ \"$k\" = Gid: ] && echo gid=$v; done </proc/self/status; \
       read -r sg </proc/self/setgroups; echo setgroups=$sg",
    ],
    &env,
  );
  assert!(id_out.status.success(), "identity probe should exit 0: {:?}", id_out.status);
  let id_stdout = String::from_utf8_lossy(&id_out.stdout);
  assert!(id_stdout.contains("uid=0"), "process must see uid 0 inside the namespace:\n{id_stdout}");
  assert!(id_stdout.contains("euid=0"), "process must see euid 0 inside the namespace:\n{id_stdout}");
  assert!(id_stdout.contains("gid=0"), "process must see gid 0 inside the namespace:\n{id_stdout}");
  assert!(id_stdout.contains("setgroups=deny"), "setgroups must be denied inside the namespace:\n{id_stdout}");

  // POST-014: a non-zero child exit is reported (not aborted) — status.code()==7.
  let code_out = run(root.path(), &["/bin/bash", "-c", "exit 7"], &env);
  assert_eq!(code_out.status.code(), Some(7), "non-zero child exit must be reported as 7");

  // POST-015: a signalled child is translated to 128 + signal (sandbox.rs:215-218).
  // SIGKILL (9) -> 137.
  let sig_out = run(root.path(), &["/bin/bash", "-c", "kill -9 $$"], &env);
  assert_eq!(
    sig_out.status.code(),
    Some(137),
    "SIGKILL child must translate to 128+9=137, got {:?}",
    sig_out.status.code()
  );

  // POST-018: /dev is a tmpfs carrying the device set + fd/pts (sandbox.rs:315-361).
  let dev_out = run(
    root.path(),
    &[
      "/bin/bash",
      "-c",
      "for d in null zero full random urandom tty; do [ -e /dev/$d ] && echo have-$d; done; \
       [ -L /dev/fd ] && echo have-fd; [ -d /dev/pts ] && echo have-pts; \
       while read src tgt fstype rest; do [ \"$tgt\" = /dev ] && echo devfs=$fstype; done </proc/mounts; \
       true",
    ],
    &env,
  );
  assert!(dev_out.status.success(), "dev probe should exit 0");
  let dev_stdout = String::from_utf8_lossy(&dev_out.stdout);
  for d in ["null", "zero", "full", "random", "urandom", "tty"] {
    assert!(dev_stdout.contains(&format!("have-{d}")), "/dev/{d} must exist:\n{dev_stdout}");
  }
  assert!(dev_stdout.contains("have-fd"), "/dev/fd symlink must exist:\n{dev_stdout}");
  assert!(dev_stdout.contains("have-pts"), "/dev/pts must exist:\n{dev_stdout}");
  assert!(dev_stdout.contains("devfs=tmpfs"), "/dev must be a tmpfs:\n{dev_stdout}");

  // POST-019: stdout and stderr are captured into distinct streams, the
  // inherited host environment is wiped, and only the provided env is present
  // (sandbox.rs:169-176/207-209). We set a unique host var that must NOT leak.
  unsafe {
    std::env::set_var("FLATROOT_HOST_LEAK", "should-be-wiped");
  }
  let cap_out = run(
    root.path(),
    &[
      "/bin/bash",
      "-c",
      "printf to-stdout; printf to-stderr 1>&2; \
       echo provided=${FLATROOT_PROVIDED:-MISSING}; \
       echo leak=${FLATROOT_HOST_LEAK:-ABSENT}",
    ],
    &env,
  );
  let cap_stdout = String::from_utf8_lossy(&cap_out.stdout);
  let cap_stderr = String::from_utf8_lossy(&cap_out.stderr);
  assert!(cap_stdout.contains("to-stdout"), "stdout stream must carry stdout text:\n{cap_stdout}");
  assert!(cap_stderr.contains("to-stderr"), "stderr stream must carry stderr text:\n{cap_stderr}");
  assert!(!cap_stdout.contains("to-stderr"), "stderr text must NOT appear on the stdout stream:\n{cap_stdout}");
  assert!(
    cap_stdout.contains("provided=marker-value"),
    "the provided env var must be present inside the sandbox:\n{cap_stdout}"
  );
  assert!(cap_stdout.contains("leak=ABSENT"), "inherited host env must be wiped inside the sandbox:\n{cap_stdout}");
}
