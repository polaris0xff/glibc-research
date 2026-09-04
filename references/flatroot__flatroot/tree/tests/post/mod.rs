//! Shared helpers for the post-install & sandbox integration tests (POST area).
//!
//! Each test binary compiles its own copy of this module via `mod post;`, so
//! `#![allow(dead_code)]` keeps the per-binary unused-symbol warnings quiet —
//! one file may use only a subset of these helpers.
//!
//! The helpers fall into three groups:
//!   - black-box install runners (drive the `flatroot` binary, parse stderr);
//!   - a "build a real bash rootfs once" helper that yields a working `/bin/sh`
//!     + dynamic loader the sandbox can pivot into and exec inside;
//!   - small filesystem fixtures (fake builders / probe scripts) layered on top
//!     of that real rootfs so a single sandbox run produces an observable,
//!     on-disk side effect we can assert against.
#![allow(dead_code)]

use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use tempfile::TempDir;

/// Run `flatroot --from <remote> install --output <root> <extra...>` with an
/// isolated cache directory. Mirrors the established install idiom: the cache
/// TempDir is owned by the caller and must outlive the call.
pub fn install(remote: &str, cache: &Path, root: &Path, extra: &[&str]) -> Output {
  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  cmd.env("FLATROOT_CACHE_HOME", cache);
  cmd.args(["--from", remote, "install", "--output", root.to_str().unwrap()]);
  cmd.args(extra);
  cmd.output().expect("flatroot failed to execute")
}

/// Run an install with an explicit `--arch <spec>` (a global flag that must
/// precede the `install` subcommand). `arch_spec` may be comma-separated for a
/// multiarch build, e.g. "x86_64,i686".
pub fn install_arch(remote: &str, arch_spec: &str, cache: &Path, root: &Path, extra: &[&str]) -> Output {
  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  cmd.env("FLATROOT_CACHE_HOME", cache);
  cmd.args([
    "--from",
    remote,
    "--arch",
    arch_spec,
    "install",
    "--output",
    root.to_str().unwrap(),
  ]);
  cmd.args(extra);
  cmd.output().expect("flatroot failed to execute")
}

/// Run an install while also setting one environment variable. Used to prove the
/// `FLATROOT_ARG_INSTALL_POSTINSTALL` fallback drives the phase list.
pub fn install_env(remote: &str, cache: &Path, root: &Path, env: &[(&str, &str)], extra: &[&str]) -> Output {
  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  cmd.env("FLATROOT_CACHE_HOME", cache);
  for (k, v) in env {
    cmd.env(k, v);
  }
  cmd.args(["--from", remote, "install", "--output", root.to_str().unwrap()]);
  cmd.args(extra);
  cmd.output().expect("flatroot failed to execute")
}

/// Panic with the captured stderr when an install did not exit 0 — the network
/// convention for surfacing the real error inside `cargo test` output.
pub fn ok(out: &Output, label: &str) {
  assert!(
    out.status.success(),
    "{label}: flatroot install failed (exit={:?})\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr),
  );
}

/// Byte offset of `marker` within `text`, or `None` when absent. Used to assert
/// the relative ordering of the post-install phase finish lines.
pub fn offset_of(text: &str, marker: &str) -> Option<usize> {
  text.find(marker)
}

/// Count non-overlapping occurrences of `marker` in `text`.
pub fn count_of(text: &str, marker: &str) -> usize {
  text.matches(marker).count()
}

/// Build a real, functional Debian `bash` rootfs into `root` with post-install
/// skipped (`--postinstall=none`), so the tree carries a working `/bin/sh`, the
/// dynamic loader, and a staged `.flatroot/scripts/` — everything the sandbox
/// needs to pivot in and actually exec a shell — without yet having replayed
/// anything. The caller then layers its own fixtures on top and drives a single
/// post-install phase via the public `PostInstallPlan` / `PostInstall` API.
///
/// Panics (with stderr) on failure so a fixture build never silently no-ops.
pub fn rootfs_debian_bash(cache: &Path, root: &Path) {
  let out = install("debian:bookworm", cache, root, &["--postinstall=none", "bash"]);
  ok(&out, "rootfs_debian_bash base install");
  assert!(root.join("bin/bash").exists(), "base install completed but bin/bash missing under {}", root.display());
}

/// Replace the staged scripts directory with an empty one, so a scripts-phase
/// replay sees only the entries a test deliberately plants.
pub fn scripts_dir_reset(root: &Path) {
  let scripts = root.join(".flatroot/scripts");
  let _ = std::fs::remove_dir_all(&scripts);
  std::fs::create_dir_all(&scripts).expect("recreate .flatroot/scripts");
}

/// Plant a Debian-style staged package under `.flatroot/scripts/<dir_name>/`
/// holding the named files. `files` is `(filename, body)` pairs; e.g.
/// `("postinst", "#!/bin/sh\n...")`.
pub fn staged_pkg(root: &Path, dir_name: &str, files: &[(&str, &str)]) {
  let dir = root.join(".flatroot/scripts").join(dir_name);
  std::fs::create_dir_all(&dir).expect("create staged pkg dir");
  for (name, body) in files {
    std::fs::write(dir.join(name), body).expect("write staged file");
  }
}

/// Install an executable `#!/bin/sh` builder at `rel` (relative to the rootfs),
/// chmod 0755. Used both for fake post-install scripts and fake cache builders
/// whose only job is to leave a probe behind so the test can observe the run.
pub fn exec_file(root: &Path, rel: &str, body: &str) {
  use std::os::unix::fs::PermissionsExt;
  let path = root.join(rel);
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent).expect("create exec parent");
  }
  std::fs::write(&path, body).expect("write exec file");
  std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod exec file");
}

/// A fake cache-builder body that records the absolute argv it was invoked with
/// into `/<probe_rel>` inside the rootfs, then exits 0. Because the rootfs is
/// bind-mounted into the sandbox, the probe survives back to the host tempdir.
pub fn probe_builder_body(probe_rel: &str) -> String {
  format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > /{probe_rel}\nexit 0\n")
}

/// Read a probe file written inside the rootfs by a fake builder/script, or
/// `None` when the builder never ran (the hook/script was skipped).
pub fn probe_read(root: &Path, probe_rel: &str) -> Option<String> {
  std::fs::read_to_string(root.join(probe_rel)).ok()
}

/// True iff the host provides unprivileged user namespaces. Sandbox-touching
/// tests gate on this so they degrade to a documented skip on a locked-down CI
/// host rather than producing a spurious failure.
pub fn userns_available() -> bool {
  flatroot::sandbox::Sandbox::available().is_ok()
}

/// Convenience: a fresh isolated cache dir.
pub fn cache_dir() -> TempDir {
  TempDir::new().expect("tempdir for flatroot cache")
}

/// Convenience: a fresh rootfs target dir.
pub fn root_dir() -> TempDir {
  TempDir::new().expect("tempdir for rootfs")
}
