//! Idempotency tests: verify that running flatroot install twice into the
//! same root directory works correctly — packages and indices are cached on
//! the second run, extraction overwrites cleanly (hard links, symlinks,
//! shared libs), and the rootfs remains functional.
//!
//! Each test gets its own isolated cache directory (via FLATROOT_CACHE_HOME)
//! so there's no cross-contamination between tests or from previous runs.
//! This allows asserting that the first run fetches from the network and
//! the second run serves everything from cache.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

/// Pull the `Downloaded N packages (M already current)` line's M value.
/// flatroot emits this once per arch after the download phase. M counts the
/// packages the short-circuit skipped because their version + checksum
/// matched the existing manifest entry — the signal that the idempotent
/// reuse path actually fired.
fn short_circuit_count(stderr: &str) -> Option<usize> {
  for line in stderr.lines() {
    if let Some(rest) = line.strip_prefix("Downloaded ")
      && let Some(n_str) = rest
        .split_once(" packages (")
        .map(|(_, tail)| tail)
        .and_then(|t| t.strip_suffix(" already current)"))
    {
      return n_str.parse().ok();
    }
  }
  None
}

/// Every flatroot install must produce the three-file manifest layout. This
/// helper encodes the invariants that every distro pipeline must satisfy.
fn assert_manifest_present(root: &Path, label: &str) {
  let meta = root.join(".flatroot");
  assert!(meta.join("manifest").exists(), "{label}: missing .flatroot/manifest");
  assert!(meta.join("packages").exists(), "{label}: missing .flatroot/packages");
  let files_dir = meta.join("files");
  assert!(files_dir.exists(), "{label}: missing .flatroot/files/");

  let entries: Vec<_> = fs::read_dir(&files_dir)
    .unwrap_or_else(|e| panic!("{label}: read_dir failed: {e}"))
    .filter_map(|e| e.ok())
    .collect();
  assert!(!entries.is_empty(), "{label}: no per-package file lists written");

  // Every Url: line must be https and carry no encoded `|` from the DB.
  let packages = fs::read_to_string(meta.join("packages")).unwrap();
  for line in packages.lines().filter(|l| l.starts_with("Url: ")) {
    assert!(line.starts_with("Url: https://"), "{label}: non-https url: {line}");
    assert!(!line.contains('|'), "{label}: encoding leaked: {line}");
  }

  // Per-package file lists never reference .flatroot paths.
  for entry in &entries {
    let content = fs::read_to_string(entry.path()).unwrap();
    for line in content.lines() {
      assert!(!line.starts_with("/.flatroot"), "{label}: leaked metadata path: {line}");
    }
  }
}

/// Assert that the manifest, packages, and every file list are byte-identical
/// to what was captured before a second identical install.
fn assert_manifest_stable_across_reinstall(root: &Path, snapshot: &ManifestSnapshot, label: &str) {
  let current = ManifestSnapshot::capture(root);
  assert_eq!(snapshot.manifest, current.manifest, "{label}: manifest bytes changed");
  assert_eq!(snapshot.packages, current.packages, "{label}: packages bytes changed");
  assert_eq!(snapshot.file_lists, current.file_lists, "{label}: per-package file lists changed");
}

struct ManifestSnapshot {
  manifest: Vec<u8>,
  packages: Vec<u8>,
  file_lists: std::collections::BTreeMap<std::ffi::OsString, Vec<u8>>,
}

impl ManifestSnapshot {
  fn capture(root: &Path) -> Self {
    let meta = root.join(".flatroot");
    let mut file_lists = std::collections::BTreeMap::new();
    for entry in fs::read_dir(meta.join("files")).unwrap().filter_map(|e| e.ok()) {
      file_lists.insert(entry.file_name(), fs::read(entry.path()).unwrap());
    }
    Self {
      manifest: fs::read(meta.join("manifest")).unwrap(),
      packages: fs::read(meta.join("packages")).unwrap(),
      file_lists,
    }
  }
}

/// Run flatroot install with an isolated cache directory.
fn install_with_cache(remote: &str, root: &str, cache: &str, pkg: &str) -> std::process::Output {
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache)
    .args(["--from", remote, "install", "--output", root, pkg])
    .output()
    .expect("flatroot failed to execute")
}

/// Assert that a flatroot install command exited successfully. On failure,
/// includes the captured stderr in the panic message so the real error is
/// visible in `cargo test` output.
fn install_assert_ok(out: &std::process::Output, label: &str) {
  assert!(
    out.status.success(),
    "{}: flatroot install failed (exit={:?})\nstderr:\n{}",
    label,
    out.status.code(),
    String::from_utf8_lossy(&out.stderr),
  );
}

/// Assert that the first run fetches from the network and the second run
/// serves everything from cache (both indices and package archives).
fn assert_caching(first: &std::process::Output, second: &std::process::Output, label: &str) {
  let first_stderr = String::from_utf8_lossy(&first.stderr);
  let second_stderr = String::from_utf8_lossy(&second.stderr);

  // First run must fetch from network (fresh cache dir, nothing cached)
  assert!(first_stderr.contains("fetching"), "{}: first run should fetch from network", label);

  // Second run must serve from cache and not fetch anything
  assert!(second_stderr.contains("cached:"), "{}: second run should have cached entries", label);
  assert!(
    !second_stderr.contains("fetching"),
    "{}: second run should not fetch from network, but found 'fetching' in output:\n{}",
    label,
    second_stderr
      .lines()
      .filter(|l| l.contains("fetching"))
      .collect::<Vec<_>>()
      .join("\n"),
  );
}

// ---------------------------------------------------------------------------
// Re-install behavior (the actual pipeline short-circuit)
// ---------------------------------------------------------------------------

/// The fundamental invariant every "installed-state manifest" must uphold:
/// re-running an install with identical inputs should do no work. We check
/// the two user-visible proofs of work — per-package extraction lines and
/// the post-install header — are both absent the second time.
#[test]
fn debian_reinstall_same_package_does_no_work() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "first install");
  let first_stderr = String::from_utf8_lossy(&first.stderr);
  assert!(first_stderr.contains("Extracted "), "first install missing 'Extracted' marker:\n{first_stderr}");
  assert!(first_stderr.contains("Cache hooks completed"), "first install skipped post-install:\n{first_stderr}");

  let second = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&second, "second install");
  let second_stderr = String::from_utf8_lossy(&second.stderr);

  assert!(!second_stderr.contains("Extracted "), "second install re-extracted packages:\n{second_stderr}");
  assert!(
    !second_stderr.contains("Cache hooks completed"),
    "second install ran post-install despite no changes:\n{second_stderr}"
  );
  assert!(
    second_stderr.contains("Nothing to do"),
    "second install missing the 'Nothing to do' confirmation:\n{second_stderr}"
  );
}

/// Adding a package to an existing rootfs must extract only the genuinely
/// new archives — every package already current must be short-circuited.
/// Without that short-circuit, install_for_arch would extract everything
/// the resolver returns on every run, making the second invocation as
/// expensive as the first.
#[test]
fn debian_adding_new_package_extracts_only_new_deps() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "bash install");
  let first_short_circuit = short_circuit_count(&String::from_utf8_lossy(&first.stderr));
  assert_eq!(first_short_circuit, Some(0), "first install shouldn't have anything already current");

  // nano shares libc6, base-files, and friends with bash. The resolver
  // returns many packages already installed; the short-circuit count on the
  // second install must therefore be > 0.
  let second = install_with_cache("debian:bookworm", root_str, cache_str, "nano");
  install_assert_ok(&second, "nano install");
  let second_stderr = String::from_utf8_lossy(&second.stderr);

  let second_short_circuit = short_circuit_count(&second_stderr).unwrap_or_else(|| {
    panic!("second install missing 'Downloaded N packages (M already current)' line:\n{second_stderr}")
  });
  assert!(
    second_short_circuit > 0,
    "second install didn't short-circuit any packages — nano pulled everything fresh:\n{second_stderr}"
  );
  assert!(second_stderr.contains("Extracted "), "second install didn't extract nano:\n{second_stderr}");
}

// ---------------------------------------------------------------------------
// Debian
// ---------------------------------------------------------------------------

#[test]
fn debian_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first =
    install_with_cache("debian:bookworm", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox-esr");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/bin/firefox-esr").exists());
  assert!(root.path().join("usr/lib/firefox-esr/firefox-esr").exists());
  assert_manifest_present(root.path(), "debian firefox first");
  let snap = ManifestSnapshot::capture(root.path());

  let second =
    install_with_cache("debian:bookworm", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox-esr");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/bin/firefox-esr").exists());
  assert!(root.path().join("usr/lib/firefox-esr/firefox-esr").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "debian firefox");

  assert_caching(&first, &second, "debian firefox");
}

#[test]
fn debian_install_gimp_over_firefox() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_path = root.path().to_str().unwrap();
  let cache_path = cache.path().to_str().unwrap();

  let out = install_with_cache("debian:bookworm", root_path, cache_path, "firefox-esr");
  install_assert_ok(&out, "firefox install failed");
  assert!(root.path().join("usr/bin/firefox-esr").exists());

  let out = install_with_cache("debian:bookworm", root_path, cache_path, "gimp");
  install_assert_ok(&out, "gimp install failed");
  assert!(root.path().join("usr/bin/firefox-esr").exists(), "firefox should still exist");
  assert!(root.path().join("usr/bin/gimp").exists(), "gimp should be installed");
  assert!(
    root.path().join("usr/lib/x86_64-linux-gnu/libgtk-3.so.0").exists()
      || root
        .path()
        .join("usr/lib/x86_64-linux-gnu/libgtk-x11-2.0.so.0")
        .exists(),
    "GTK shared lib should exist"
  );
}

// ---------------------------------------------------------------------------
// Ubuntu
// ---------------------------------------------------------------------------

#[test]
fn ubuntu_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first =
    install_with_cache("ubuntu:noble", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/bin/firefox").exists());
  assert_manifest_present(root.path(), "ubuntu firefox first");
  let snap = ManifestSnapshot::capture(root.path());

  let second =
    install_with_cache("ubuntu:noble", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/bin/firefox").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "ubuntu firefox");

  assert_caching(&first, &second, "ubuntu firefox");
}

#[test]
fn ubuntu_install_gimp_over_firefox() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_path = root.path().to_str().unwrap();
  let cache_path = cache.path().to_str().unwrap();

  let out = install_with_cache("ubuntu:noble", root_path, cache_path, "firefox");
  install_assert_ok(&out, "firefox install failed");
  assert!(root.path().join("usr/bin/firefox").exists());

  let out = install_with_cache("ubuntu:noble", root_path, cache_path, "gimp");
  install_assert_ok(&out, "gimp install failed");
  assert!(root.path().join("usr/bin/firefox").exists(), "firefox should still exist");
  assert!(root.path().join("usr/bin/gimp").exists(), "gimp should be installed");
}

// ---------------------------------------------------------------------------
// Arch
// ---------------------------------------------------------------------------

#[test]
fn arch_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first =
    install_with_cache("arch:rolling", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists());
  assert_manifest_present(root.path(), "arch firefox first");
  let snap = ManifestSnapshot::capture(root.path());

  let second =
    install_with_cache("arch:rolling", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "arch firefox");

  assert_caching(&first, &second, "arch firefox");
}

#[test]
fn arch_install_gimp_over_firefox() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_path = root.path().to_str().unwrap();
  let cache_path = cache.path().to_str().unwrap();

  let out = install_with_cache("arch:rolling", root_path, cache_path, "firefox");
  install_assert_ok(&out, "firefox install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists());

  let out = install_with_cache("arch:rolling", root_path, cache_path, "gimp");
  install_assert_ok(&out, "gimp install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists(), "firefox should still exist");
  assert!(root.path().join("usr/bin/gimp").exists(), "gimp should be installed");
}

// ---------------------------------------------------------------------------
// Alpine
// ---------------------------------------------------------------------------

#[test]
fn alpine_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first =
    install_with_cache("alpine:edge", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists());
  assert_manifest_present(root.path(), "alpine firefox first");
  // Alpine uses q1-sha1 for package checksums; confirm the typed prefix reaches the manifest.
  let packages = fs::read_to_string(root.path().join(".flatroot/packages")).unwrap();
  assert!(
    packages.lines().any(|l| l.starts_with("Checksum: q1-sha1:")),
    "alpine manifest missing q1-sha1 checksums:\n{packages}"
  );
  let snap = ManifestSnapshot::capture(root.path());

  let second =
    install_with_cache("alpine:edge", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "alpine firefox");

  assert_caching(&first, &second, "alpine firefox");
}

#[test]
fn alpine_install_gimp_over_firefox() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_path = root.path().to_str().unwrap();
  let cache_path = cache.path().to_str().unwrap();

  let out = install_with_cache("alpine:edge", root_path, cache_path, "firefox");
  install_assert_ok(&out, "firefox install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists());

  let out = install_with_cache("alpine:edge", root_path, cache_path, "gimp");
  install_assert_ok(&out, "gimp install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists(), "firefox should still exist");
  assert!(
    root.path().join("usr/bin/gimp").exists()
      || root.path().join("usr/bin/gimp-2.10").exists()
      || root.path().join("usr/bin/gimp-3.0").exists(),
    "gimp binary should be installed"
  );
}

// ---------------------------------------------------------------------------
// CentOS
// ---------------------------------------------------------------------------

#[test]
fn centos_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first = install_with_cache("centos:7", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());
  assert_manifest_present(root.path(), "centos firefox first");
  let snap = ManifestSnapshot::capture(root.path());

  let second = install_with_cache("centos:7", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "centos firefox");

  assert_caching(&first, &second, "centos firefox");
}

#[test]
fn centos_install_gimp_over_firefox() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_path = root.path().to_str().unwrap();
  let cache_path = cache.path().to_str().unwrap();

  let out = install_with_cache("centos:7", root_path, cache_path, "firefox");
  install_assert_ok(&out, "firefox install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());

  let out = install_with_cache("centos:7", root_path, cache_path, "gimp");
  install_assert_ok(&out, "gimp install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists(), "firefox should still exist");
  assert!(root.path().join("usr/bin/gimp").exists(), "gimp should be installed");
}

// ---------------------------------------------------------------------------
// Fedora
// ---------------------------------------------------------------------------

#[test]
fn fedora_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first = install_with_cache("fedora:42", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());
  assert_manifest_present(root.path(), "fedora firefox first");
  let snap = ManifestSnapshot::capture(root.path());

  let second =
    install_with_cache("fedora:42", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "fedora firefox");

  assert_caching(&first, &second, "fedora firefox");
}

#[test]
fn fedora_install_gimp_over_firefox() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_path = root.path().to_str().unwrap();
  let cache_path = cache.path().to_str().unwrap();

  let out = install_with_cache("fedora:42", root_path, cache_path, "firefox");
  install_assert_ok(&out, "firefox install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());

  let out = install_with_cache("fedora:42", root_path, cache_path, "gimp");
  install_assert_ok(&out, "gimp install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists(), "firefox should still exist");
  assert!(root.path().join("usr/bin/gimp").exists(), "gimp should be installed");
}

// ---------------------------------------------------------------------------
// AlmaLinux
// ---------------------------------------------------------------------------

#[test]
fn alma_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first = install_with_cache("alma:9", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());
  assert_manifest_present(root.path(), "alma firefox first");
  let snap = ManifestSnapshot::capture(root.path());

  let second = install_with_cache("alma:9", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "alma firefox");

  assert_caching(&first, &second, "alma firefox");
}

// ---------------------------------------------------------------------------
// Rocky Linux
// ---------------------------------------------------------------------------

#[test]
fn rocky_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first = install_with_cache("rocky:9", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());
  assert_manifest_present(root.path(), "rocky firefox first");
  let snap = ManifestSnapshot::capture(root.path());

  let second = install_with_cache("rocky:9", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/lib64/firefox/firefox").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "rocky firefox");

  assert_caching(&first, &second, "rocky firefox");
}

// ---------------------------------------------------------------------------
// openSUSE
// ---------------------------------------------------------------------------

#[test]
fn opensuse_install_gimp_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first =
    install_with_cache("opensuse:tumbleweed", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "gimp");
  install_assert_ok(&first, "First install failed");
  // openSUSE ships /usr/bin/gimp as a symlink; symlink_metadata picks it up
  // even when the target itself isn't followed.
  assert!(root.path().join("usr/bin/gimp").symlink_metadata().is_ok());
  assert_manifest_present(root.path(), "opensuse gimp first");
  let snap = ManifestSnapshot::capture(root.path());

  let second =
    install_with_cache("opensuse:tumbleweed", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "gimp");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/bin/gimp").symlink_metadata().is_ok());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "opensuse gimp");

  assert_caching(&first, &second, "opensuse gimp");
}

// ---------------------------------------------------------------------------
// CachyOS
// ---------------------------------------------------------------------------

#[test]
fn cachyos_install_firefox_twice() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  let first =
    install_with_cache("cachyos:rolling", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&first, "First install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists());
  assert_manifest_present(root.path(), "cachyos firefox first");
  let snap = ManifestSnapshot::capture(root.path());

  let second =
    install_with_cache("cachyos:rolling", root.path().to_str().unwrap(), cache.path().to_str().unwrap(), "firefox");
  install_assert_ok(&second, "Second install failed");
  assert!(root.path().join("usr/lib/firefox/firefox").exists());
  assert_manifest_stable_across_reinstall(root.path(), &snap, "cachyos firefox");

  assert_caching(&first, &second, "cachyos firefox");
}

// ===========================================================================
// Incremental / idempotency edge cases (INST-*)
//
// These drive the install short-circuit, the manifest merge, the cross-arch
// total_extracted gate, and the cache-reuse/checksum-gate paths by mutating an
// existing rootfs's manifest or cached archives between runs and asserting the
// observable re-extract / reuse behavior.
// ===========================================================================

/// Run a fully-flagged install with explicit `--from`, `--output`, packages.
fn install_flags(remote: &str, root: &str, cache: &str, args: &[&str]) -> std::process::Output {
  let mut full: Vec<&str> = vec!["--from", remote, "install", "--output", root];
  full.extend_from_slice(args);
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache)
    .args(&full)
    .output()
    .expect("flatroot failed to execute")
}

/// The `Package:` names recorded in `.flatroot/packages`.
fn manifest_names(root: &Path) -> Vec<String> {
  let packages = fs::read_to_string(root.join(".flatroot/packages")).unwrap();
  packages
    .lines()
    .filter_map(|l| l.strip_prefix("Package: ").map(|s| s.to_string()))
    .collect()
}

/// Capture the permission mode of every regular file under the rootfs
/// (excluding `.flatroot`), keyed by rootfs-relative path.
fn capture_modes(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, u32> {
  fn walk(dir: &Path, root: &Path, out: &mut std::collections::BTreeMap<std::path::PathBuf, u32>) {
    let entries = match fs::read_dir(dir) {
      Ok(e) => e,
      Err(_) => return,
    };
    for entry in entries.flatten() {
      let path = entry.path();
      let rel = path.strip_prefix(root).unwrap().to_path_buf();
      if rel.starts_with(".flatroot") {
        continue;
      }
      let md = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(_) => continue,
      };
      let ft = md.file_type();
      if ft.is_symlink() {
        continue;
      }
      if ft.is_dir() {
        walk(&path, root, out);
      } else if ft.is_file() {
        out.insert(rel, md.permissions().mode() & 0o7777);
      }
    }
  }
  let mut out = std::collections::BTreeMap::new();
  walk(root, root, &mut out);
  out
}

/// Env-var fallbacks (FLATROOT_ARG_INSTALL_OUTPUT + _PARALLEL + _NO_DEPS, plus
/// FLATROOT_ARG_FROM) drive an install identically to flags: the env-configured
/// run produces a manifest with the same package set as the flag-driven
/// baseline, with --no-deps + --parallel honored (cli.rs:32/92/126/132).
// covers: INST-005
#[test]
fn env_fallbacks_drive_install_like_flags() {
  // Flag-driven baseline: --no-deps -p 2 curl.
  let root_flags = TempDir::new().unwrap();
  let cache_flags = TempDir::new().unwrap();
  let out_flags = install_flags(
    "debian:bookworm",
    root_flags.path().to_str().unwrap(),
    cache_flags.path().to_str().unwrap(),
    &["--no-deps", "-p", "2", "curl"],
  );
  install_assert_ok(&out_flags, "flag-driven install");

  // Env-driven: same effect, configured purely through env vars. Only the
  // package positional is passed on the command line.
  let root_env = TempDir::new().unwrap();
  let cache_env = TempDir::new().unwrap();
  let out_env = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache_env.path())
    .env("FLATROOT_ARG_FROM", "debian:bookworm")
    .env("FLATROOT_ARG_INSTALL_OUTPUT", root_env.path())
    .env("FLATROOT_ARG_INSTALL_PARALLEL", "2")
    .env("FLATROOT_ARG_INSTALL_NO_DEPS", "true")
    .args(["install", "curl"])
    .output()
    .expect("flatroot failed to execute");
  install_assert_ok(&out_env, "env-driven install");

  let mut flag_names = manifest_names(root_flags.path());
  let mut env_names = manifest_names(root_env.path());
  flag_names.sort();
  env_names.sort();
  assert_eq!(flag_names, env_names, "env-configured install must equal the flag-driven package set");
  // --no-deps honored: only the named package present.
  assert_eq!(env_names, vec!["curl".to_string()], "env --no-deps must install only curl:\n{env_names:?}");
}

/// A reinstall is "already current" only when BOTH version and checksum match
/// (install.rs:255-258). Corrupting the stored checksum of one package forces
/// it to be re-extracted, and the merge prints the "replacing" diagnostic
/// (state.rs:265-281). We need FIM_DEBUG-free: both markers are unconditional
/// eprintln/RunVoice output.
// covers: INST-033
#[test]
fn checksum_mismatch_forces_reextract_with_replacing_diagnostic() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "first bash install");

  // Corrupt the recorded checksum of the `bash` record so version matches but
  // checksum no longer does → up_to_date is false → re-extract.
  let packages_path = root.path().join(".flatroot/packages");
  let packages = fs::read_to_string(&packages_path).unwrap();
  let mut in_bash = false;
  let mut mutated = String::new();
  for line in packages.lines() {
    if let Some(name) = line.strip_prefix("Package: ") {
      in_bash = name == "bash";
    }
    if in_bash && line.starts_with("Checksum: ") {
      mutated.push_str("Checksum: sha256:0000000000000000000000000000000000000000000000000000000000000000");
    } else {
      mutated.push_str(line);
    }
    mutated.push('\n');
  }
  fs::write(&packages_path, mutated).unwrap();

  let second = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&second, "reinstall after checksum corruption");
  let stderr = String::from_utf8_lossy(&second.stderr);
  assert!(stderr.contains("Extracted "), "bash must be re-extracted after checksum mismatch:\n{stderr}");
  assert!(stderr.contains("replacing bash:"), "merge must print the 'replacing' diagnostic for bash:\n{stderr}");
}

/// A reused up-to-date record carries its prior source/url/files verbatim
/// (install.rs:260-280, especially :266). Mutating the stored Url to a
/// sentinel that differs from what the run would recompute, then reinstalling
/// (everything up-to-date), the prior Url must be retained unchanged.
// covers: INST-034
#[test]
fn reused_record_keeps_prior_url() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "first bash install");

  // Rewrite the `bash` record's Url to a sentinel. Source/Version/Checksum are
  // left intact so the package stays up-to-date on reinstall and the prior
  // record (with our sentinel Url) is carried forward verbatim.
  let packages_path = root.path().join(".flatroot/packages");
  let packages = fs::read_to_string(&packages_path).unwrap();
  let sentinel = "https://sentinel.invalid/prior-url-kept-verbatim.deb";
  let mut in_bash = false;
  let mut mutated = String::new();
  for line in packages.lines() {
    if let Some(name) = line.strip_prefix("Package: ") {
      in_bash = name == "bash";
    }
    if in_bash && line.starts_with("Url: ") {
      mutated.push_str(&format!("Url: {sentinel}"));
    } else {
      mutated.push_str(line);
    }
    mutated.push('\n');
  }
  fs::write(&packages_path, mutated).unwrap();

  let second = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&second, "reinstall reuses prior record");
  let stderr = String::from_utf8_lossy(&second.stderr);
  assert!(stderr.contains("Nothing to do"), "everything must be up-to-date on reinstall:\n{stderr}");

  let after = fs::read_to_string(&packages_path).unwrap();
  // Find the bash stanza's Url line and confirm the sentinel survived.
  let mut in_bash = false;
  let mut found = false;
  for line in after.lines() {
    if let Some(name) = line.strip_prefix("Package: ") {
      in_bash = name == "bash";
    }
    if in_bash && line == format!("Url: {sentinel}") {
      found = true;
      break;
    }
  }
  assert!(found, "prior Url must be retained verbatim in the rewritten manifest:\n{after}");
}

/// Cross-arch post-install + Postfix gate on total_extracted: a multiarch
/// reinstall where both arches are already current reports "Nothing to do"
/// (one summary), and never re-runs post-install (install.rs:117/124-126/140).
// covers: INST-037
#[test]
fn multiarch_reinstall_all_current_nothing_to_do() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache_str)
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64,i686",
      "install",
      "--output",
      root_str,
      "dash",
    ])
    .output()
    .expect("flatroot failed to execute");
  install_assert_ok(&first, "first multiarch dash install");
  assert!(String::from_utf8_lossy(&first.stderr).contains("Done. "), "first multiarch install must finish");

  let second = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache_str)
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64,i686",
      "install",
      "--output",
      root_str,
      "dash",
    ])
    .output()
    .expect("flatroot failed to execute");
  install_assert_ok(&second, "second multiarch dash install");
  let stderr = String::from_utf8_lossy(&second.stderr);
  assert!(
    stderr.contains("Nothing to do"),
    "multiarch reinstall with both arches current must report 'Nothing to do':\n{stderr}"
  );
  assert!(!stderr.contains("Extracted "), "nothing should be extracted on the all-current reinstall:\n{stderr}");
  assert!(!stderr.contains("Cache hooks completed"), "post-install must not re-run when nothing changed:\n{stderr}");
}

/// Postfix is skipped when nothing was extracted: across a no-op reinstall the
/// file modes are byte-identical (install.rs:140-145 gates PermissionFix on
/// total_extracted > 0).
// covers: INST-041
#[test]
fn postfix_skipped_when_nothing_extracted_modes_unchanged() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "first bash install");
  let modes_before = capture_modes(root.path());

  let second = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&second, "no-op reinstall");
  assert!(String::from_utf8_lossy(&second.stderr).contains("Nothing to do"), "second run must be a no-op");
  let modes_after = capture_modes(root.path());
  assert_eq!(modes_before, modes_after, "file modes must be byte-identical across a no-op reinstall");
}

/// The package index db is reused from cache on the second run without a
/// network index fetch: the db cache-hit line "  cached: <key>.db" appears and
/// no "  fetching " of an index body occurs (arch_context.rs:56-60 →
/// db.rs:174-176 fast path). Distinct from archive caching.
// covers: INST-048
#[test]
fn index_db_cache_reused_without_refetch() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "first install populates the index");

  // Second run with a different package: the index db is fresh, so the index
  // is served from the db cache and no index body is fetched.
  let second = install_with_cache("debian:bookworm", root_str, cache_str, "vim");
  install_assert_ok(&second, "second install reuses the index db");
  let stderr = String::from_utf8_lossy(&second.stderr);
  // The db cache-hit line is keyed by the sanitized cache_key
  // (debian/bookworm/amd64 → debian-bookworm-amd64.db).
  assert!(stderr.contains("cached: debian-bookworm-amd64.db"), "second run must hit the index db cache:\n{stderr}");
  assert!(!stderr.contains("  fetching "), "second run must not fetch the package index over the network:\n{stderr}");
}

/// On a zero-extraction reinstall, each arch's pass prints the per-arch
/// "All N resolved packages already current for <arch>." line (install.rs:288)
/// and no post-install runs.
// covers: INST-051
#[test]
fn zero_extraction_reinstall_prints_all_current_line() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "first bash install");

  let second = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&second, "no-op reinstall");
  let stderr = String::from_utf8_lossy(&second.stderr);
  assert!(
    stderr.contains("resolved packages already current for x86_64."),
    "per-arch all-current line missing:\n{stderr}"
  );
}

/// merge supersedes the same (name, arch) record and recomputes headers
/// (state.rs:257-289). Downgrading the stored bash Version+Checksum then
/// reinstalling forces the merge to replace the record with the newer real
/// version, and the manifest's Version header reflects the recomputed value.
// covers: INST-052
#[test]
fn merge_supersedes_record_and_shows_newer_version() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "first bash install");

  // Capture the real bash version, then rewrite the stored Version+Checksum to
  // an obviously-older sentinel so the reinstall sees a mismatch and re-extracts.
  let packages_path = root.path().join(".flatroot/packages");
  let packages = fs::read_to_string(&packages_path).unwrap();
  let real_version = {
    let mut in_bash = false;
    let mut v = None;
    for line in packages.lines() {
      if let Some(name) = line.strip_prefix("Package: ") {
        in_bash = name == "bash";
      }
      if in_bash && let Some(ver) = line.strip_prefix("Version: ") {
        v = Some(ver.to_string());
        break;
      }
    }
    v.expect("bash Version line present")
  };

  let mut in_bash = false;
  let mut mutated = String::new();
  for line in packages.lines() {
    if let Some(name) = line.strip_prefix("Package: ") {
      in_bash = name == "bash";
    }
    if in_bash && line.starts_with("Version: ") {
      mutated.push_str("Version: 0.0.0-old");
    } else if in_bash && line.starts_with("Checksum: ") {
      mutated.push_str("Checksum: sha256:1111111111111111111111111111111111111111111111111111111111111111");
    } else {
      mutated.push_str(line);
    }
    mutated.push('\n');
  }
  fs::write(&packages_path, mutated).unwrap();

  let second = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&second, "reinstall upgrades the downgraded record");

  let after = fs::read_to_string(&packages_path).unwrap();
  // The bash record's Version must now be the real (newer) version again.
  let mut in_bash = false;
  let mut found_real = false;
  let mut found_old = false;
  for line in after.lines() {
    if let Some(name) = line.strip_prefix("Package: ") {
      in_bash = name == "bash";
    }
    if in_bash && line == format!("Version: {real_version}") {
      found_real = true;
    }
    if in_bash && line == "Version: 0.0.0-old" {
      found_old = true;
    }
  }
  assert!(found_real, "merged manifest must show the recomputed (real) bash version {real_version}:\n{after}");
  assert!(!found_old, "the superseded old version must not survive the merge:\n{after}");
}

/// A cached-but-corrupt archive is re-fetched on reinstall — the cache hit is
/// gated on a matching checksum (install.rs:292-294 / downloader.rs:120-128).
/// We corrupt a cached `.deb`, then reinstall the same package after deleting
/// the rootfs so it must re-extract; the run must succeed (proving the corrupt
/// copy was discarded and re-downloaded, not served).
// covers: INST-059
#[test]
fn corrupt_cached_archive_is_refetched() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let root_str = root.path().to_str().unwrap();
  let cache_str = cache.path().to_str().unwrap();

  let first = install_with_cache("debian:bookworm", root_str, cache_str, "bash");
  install_assert_ok(&first, "first bash install");

  // Corrupt every cached `.deb` archive for this source.
  let source_cache = cache.path().join("debian/bookworm/amd64");
  let mut corrupted = 0;
  for entry in fs::read_dir(&source_cache).unwrap().flatten() {
    let path = entry.path();
    if path.extension().map(|e| e == "deb").unwrap_or(false) {
      fs::write(&path, b"corrupt-not-a-real-deb").unwrap();
      corrupted += 1;
    }
  }
  assert!(corrupted > 0, "expected cached .deb archives to corrupt under {source_cache:?}");

  // Remove the rootfs entirely so the next install must download + extract
  // afresh — forcing the cache to be consulted and the corrupt copies rejected.
  let fresh_root = TempDir::new().unwrap();
  let second = install_with_cache("debian:bookworm", fresh_root.path().to_str().unwrap(), cache_str, "bash");
  install_assert_ok(&second, "reinstall after corrupting the cache must re-fetch and succeed");
  assert!(fresh_root.path().join("bin/bash").exists(), "bash must be re-extracted from a re-fetched archive");
}
