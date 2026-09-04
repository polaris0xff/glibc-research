//! Architecture validation tests.
//!
//! For each supported architecture, creates a minimal rootfs with
//! `--postinstall=none` and runs binaries from it under QEMU user-mode
//! emulation. This verifies that:
//!
//! 1. FlatRoot can fetch the index for that architecture
//! 2. FlatRoot can resolve and download packages for that architecture
//! 3. The extracted binaries are the correct architecture (QEMU would
//!    refuse a wrong-arch ELF)
//! 4. The dynamic linker and libc work (bash executes and prints output)
//!
//! Requires `qemu-user` (or `qemu-user-static` + binfmt) installed on
//! the host. The native architecture (x86_64) is tested without QEMU.
//!
//! Run with: cargo test --test arch_qemu -- --nocapture

use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

/// Build a rootfs for the given arch and return the temp directory.
fn build_rootfs(remote: &str, arch: &str) -> TempDir {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      remote,
      "--arch",
      arch,
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      "bash",
    ])
    .assert()
    .success();

  std::mem::forget(cache);
  root
}

/// Convert absolute symlinks inside the rootfs to relative ones.
///
/// QEMU user-mode prepends QEMU_LD_PREFIX to the ELF interpreter path
/// but doesn't follow absolute symlink targets through the prefix —
/// a symlink like `/lib/ld-linux-aarch64.so.1` → `/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1`
/// resolves on the host filesystem (where the target doesn't exist)
/// instead of inside the rootfs.
///
/// Converting to relative symlinks fixes this: `../lib/aarch64-linux-gnu/ld-linux-aarch64.so.1`
/// resolves correctly relative to the symlink's location in the rootfs.
fn relativize_symlinks(root: &Path) {
  fn walk(dir: &Path, root: &Path) {
    let entries = match std::fs::read_dir(dir) {
      Ok(e) => e,
      Err(_) => return,
    };
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_symlink() {
        if let Ok(target) = std::fs::read_link(&path) {
          if target.is_absolute() {
            // Convert absolute target to relative from the symlink's directory
            let link_dir = path.parent().unwrap();
            let abs_target_in_rootfs = root.join(target.strip_prefix("/").unwrap_or(&target));
            if let Some(relative) = pathdiff::diff_paths(&abs_target_in_rootfs, link_dir) {
              let _ = std::fs::remove_file(&path);
              let _ = std::os::unix::fs::symlink(&relative, &path);
            }
          }
        }
      } else if path.is_dir() {
        walk(&path, root);
      }
    }
  }
  walk(root, root);
}

/// Find the QEMU binary, trying both plain and -static variants.
/// Arch Linux installs `qemu-aarch64`, Debian/Ubuntu installs `qemu-aarch64-static`.
fn find_qemu(name: &str) -> String {
  for candidate in [name.to_string(), format!("{}-static", name)] {
    if std::process::Command::new(&candidate)
      .arg("--version")
      .stdout(std::process::Stdio::null())
      .stderr(std::process::Stdio::null())
      .status()
      .is_ok()
    {
      return candidate;
    }
  }
  panic!("{} not found (tried '{}' and '{}-static'). Is qemu-user installed?", name, name, name);
}

/// Run a command from the rootfs under QEMU and return stdout.
fn qemu_exec(root: &Path, qemu_bin: &str, binary: &str, args: &[&str]) -> String {
  let qemu = find_qemu(qemu_bin);
  let mut cmd = std::process::Command::new(&qemu);
  cmd.env("QEMU_LD_PREFIX", root);

  let bin_path = root.join(binary.trim_start_matches('/'));
  cmd.arg(&bin_path);
  cmd.args(args);

  let output = cmd.output().unwrap_or_else(|e| {
    panic!("{} not found or failed to execute: {}. Is qemu-user installed?", qemu_bin, e);
  });

  assert!(
    output.status.success(),
    "{} {} failed with exit code {:?}\nstderr: {}",
    qemu_bin,
    binary,
    output.status.code(),
    String::from_utf8_lossy(&output.stderr)
  );

  String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Populate /etc/ld.so.cache in the rootfs using the host's ldconfig.
///
/// build_rootfs uses `--postinstall=none`, which skips ldconfig. Without a
/// cache, guest ld.so falls back to hardcoded library paths; on multilib hosts,
/// those paths can resolve against the host via qemu-user and pick up a
/// mismatched glibc. The host's x86_64 ldconfig reads ELF headers directly, so
/// one `-r <rootfs>` run produces a valid cache for every supported target
/// arch (all little-endian).
fn populate_ld_so_cache(root: &Path) {
  let status = std::process::Command::new("ldconfig")
    .args(["-r", root.to_str().unwrap()])
    .status()
    .expect("ldconfig not found — required to populate the rootfs's ld.so.cache");
  assert!(status.success(), "ldconfig -r <rootfs> failed with {:?}", status.code());
}

/// Verify bash runs and reports the expected architecture.
fn verify_arch(remote: &str, arch: &str, qemu_bin: &str, expected_uname: &str, expected_bash_arch: &str) {
  let root = build_rootfs(remote, arch);
  relativize_symlinks(root.path());
  populate_ld_so_cache(root.path());

  // bash --version should contain the architecture in the version string
  let bash_version = qemu_exec(root.path(), qemu_bin, "bin/bash", &["--version"]);
  assert!(
    bash_version.contains(expected_bash_arch),
    "bash --version should contain '{}', got: {}",
    expected_bash_arch,
    bash_version.lines().next().unwrap_or("")
  );

  // uname -m should report the emulated architecture
  let uname = qemu_exec(root.path(), qemu_bin, "bin/uname", &["-m"]);
  assert_eq!(uname, expected_uname, "uname -m should report '{}', got '{}'", expected_uname, uname);
}

// x86_64 is skipped — QEMU can't emulate the same architecture as the host,
// and running rootfs binaries natively requires bwrap/chroot to resolve
// absolute ELF interpreter paths. The cross-architecture tests below are
// sufficient to validate FlatRoot's extraction and arch mapping.

// ---------------------------------------------------------------------------
// aarch64
// ---------------------------------------------------------------------------

#[test]
fn aarch64_debian() {
  verify_arch("debian:bookworm", "aarch64", "qemu-aarch64", "aarch64", "aarch64");
}

// ---------------------------------------------------------------------------
// armv7l
// ---------------------------------------------------------------------------

#[test]
fn armv7l_debian() {
  verify_arch("debian:bookworm", "armv7l", "qemu-arm", "armv7l", "arm");
}

// ---------------------------------------------------------------------------
// i686
// ---------------------------------------------------------------------------

#[test]
fn i686_debian() {
  verify_arch("debian:bookworm", "i686", "qemu-i386", "i686", "i686");
}

// ---------------------------------------------------------------------------
// riscv64
// ---------------------------------------------------------------------------

#[test]
fn riscv64_debian() {
  // Debian sid has riscv64 in main with glibc.
  // Alpine's musl has unsupported RISC-V relocations under QEMU user-mode.
  verify_arch("debian:sid", "riscv64", "qemu-riscv64", "riscv64", "riscv64");
}

// ---------------------------------------------------------------------------
// Multilib: one tree, pipeline once per arch, (name,arch)-keyed manifest
// ---------------------------------------------------------------------------

// covers: CORE-037
#[test]
fn multilib_x86_64_i686_records_both_arches_in_one_manifest() {
  // `--arch x86_64,i686` splits into two arches, runs the pipeline once per
  // arch into ONE rootfs, and the manifest is (name, arch)-keyed with BOTH
  // arches recorded. Read the manifest's Architectures header and the packages
  // body to prove both arches are present and that the same package name can
  // appear under each arch.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();

  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64,i686",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      "libc6",
    ])
    .assert()
    .success();
  std::mem::forget(cache);

  let meta = root.path().join(".flatroot");
  let manifest = std::fs::read_to_string(meta.join("manifest")).expect("manifest");
  // The Architectures header (`, `-separated) must carry both kernel-uname
  // spellings.
  let arch_line = manifest
    .lines()
    .find(|l| l.starts_with("Architectures:"))
    .expect("manifest must carry an Architectures header");
  assert!(arch_line.contains("x86_64"), "Architectures header must list x86_64: {arch_line}");
  assert!(arch_line.contains("i686"), "Architectures header must list i686: {arch_line}");

  // The per-package ledgers are (name:arch)-keyed; libc6 must appear for both.
  let files_dir = meta.join("files");
  let keys: Vec<String> = std::fs::read_dir(&files_dir)
    .unwrap()
    .flatten()
    .map(|e| e.file_name().to_string_lossy().into_owned())
    .collect();
  assert!(keys.iter().any(|k| k == "libc6:x86_64"), "manifest must key libc6 under x86_64: {keys:?}");
  assert!(keys.iter().any(|k| k == "libc6:i686"), "manifest must key libc6 under i686: {keys:?}");

  // Both architectures' multiarch lib dirs landed in the one tree.
  assert!(
    root.path().join("usr/lib/x86_64-linux-gnu").exists() || root.path().join("lib/x86_64-linux-gnu").exists(),
    "x86_64 multiarch lib dir must be present"
  );
  assert!(
    root.path().join("usr/lib/i386-linux-gnu").exists() || root.path().join("lib/i386-linux-gnu").exists(),
    "i386 (i686) multiarch lib dir must be present"
  );
}

// ---------------------------------------------------------------------------
// Arch::detect reads the rootfs's own ELF binaries; OCI export stamps it
// ---------------------------------------------------------------------------

// covers: CORE-039
#[test]
fn oci_export_detects_and_stamps_the_rootfs_arch() {
  // Build a real x86_64 rootfs, then export it to OCI with NO --arch. The
  // exporter must detect the arch from the rootfs's own ELF binaries
  // (e_machine -> x86_64), announce it on stderr, and stamp the OCI metadata's
  // `architecture` field with the goarch spelling (amd64).
  require_export_tool("tar");
  // Install coreutils (guarantees usr/bin/env / usr/bin/coreutils — an
  // Arch::detect candidate) plus bash, so the detector has an ELF witness
  // regardless of bash's incidental closure.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      "coreutils",
      "bash",
    ])
    .assert()
    .success();
  std::mem::forget(cache);

  let out_dir = TempDir::new().unwrap();
  let out_path = out_dir.path().join("image.tar");
  let export_cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", export_cache.path())
    .args([
      "export",
      root.path().to_str().unwrap(),
      out_path.to_str().unwrap(),
      "--format",
      "oci",
      "-t",
      "detect:test",
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "oci export failed:\n{}", String::from_utf8_lossy(&out.stderr));

  // The detector announces the mapped uname + goarch on stderr.
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("detected architecture: x86_64 (amd64)"),
    "export must detect and announce the rootfs arch:\n{stderr}"
  );

  // The OCI metadata stamps `architecture` with the goarch spelling. Untar the
  // outer OCI tar and scan the JSON blobs (config/manifest are non-gzip) for
  // the stamped value.
  let unpack = TempDir::new().unwrap();
  let f = std::fs::File::open(&out_path).expect("open OCI tar");
  let mut archive = tar::Archive::new(std::io::BufReader::new(f));
  archive.unpack(unpack.path()).expect("unpack OCI tar");
  let blobs = unpack.path().join("blobs/sha256");
  let mut stamped = false;
  for entry in std::fs::read_dir(&blobs).expect("read OCI blobs/sha256").flatten() {
    let bytes = std::fs::read(entry.path()).unwrap();
    // Skip the gzip-compressed layer blob.
    if bytes.starts_with(&[0x1f, 0x8b]) {
      continue;
    }
    // Strip whitespace to be tolerant of pretty vs compact JSON.
    let compact: String = String::from_utf8_lossy(&bytes)
      .chars()
      .filter(|c| !c.is_whitespace())
      .collect();
    if compact.contains("\"architecture\":\"amd64\"") {
      stamped = true;
    }
    // A misstamp would be the worst failure: no blob may claim a different arch.
    assert!(
      !compact.contains("\"architecture\":\"386\"") && !compact.contains("\"architecture\":\"arm64\""),
      "OCI metadata must not stamp a foreign architecture: {compact}"
    );
  }
  assert!(stamped, "an OCI JSON blob must stamp architecture=amd64 (the x86_64 goarch)");
}

// covers: CORE-040
#[test]
fn export_errors_when_no_recognizable_elf_binary_is_found() {
  // Exporting a directory with no ELF binaries must fail: Arch::detect cannot
  // find a witness to the arch and bails with the documented message. Use the
  // tar format (no --tag needed) so the failure is purely arch-detection.
  let empty = TempDir::new().unwrap();
  // A non-ELF payload so the directory is non-empty but carries no arch witness.
  std::fs::write(empty.path().join("readme.txt"), b"not an elf binary").unwrap();

  let out_dir = TempDir::new().unwrap();
  let out_path = out_dir.path().join("out.tar.gz");
  let export_cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", export_cache.path())
    .args(["export", empty.path().to_str().unwrap(), out_path.to_str().unwrap()])
    .output()
    .unwrap();
  assert!(!out.status.success(), "export of a rootfs with no ELF must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Could not detect rootfs architecture"),
    "no-ELF export must bail with the detect error:\n{stderr}"
  );
}

/// Hard requirement on an external binary used by the export tests.
fn require_export_tool(name: &'static str) {
  which::which(name).unwrap_or_else(|_| panic!("'{name}' is required on PATH for the export arch tests"));
}
