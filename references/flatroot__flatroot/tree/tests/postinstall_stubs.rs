//! Post-install stub provisioning (POST area, prefix POST).
//!
//! Every supported distro's `PostInstall::stubs_install` writes a private
//! `.flatroot/bin/` directory of harmless stand-in commands (and a few seeded
//! files) so privileged maintainer scripts can run to completion unprivileged.
//! These tests drive the public `PostInstall` trait impls (`PostInstall::Debian`,
//! `PostInstall::Arch`, `PostInstall::Alpine`) against a throwaway tempdir and
//! assert the on-disk contract: exact bodies, 0755 modes, the *_keep deference,
//! the stub PATH ordering, and — for the two stubs that carry real logic — the
//! actual behaviour of the dpkg and update-alternatives stand-ins by executing
//! them with the host shell.
//!
//! No sandbox / user namespace is required: stub installation is pure
//! filesystem work performed on the host side before the sandbox ever runs.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use flatroot::postinstall::{PostInstall, Rootfs};
use tempfile::TempDir;

/// Absolute path to a stub command inside the rootfs's private stub bin dir.
fn stub_path(root: &Path, name: &str) -> std::path::PathBuf {
  root.join(".flatroot/bin").join(name)
}

/// Mode bits (low 12) of a file.
fn mode(path: &Path) -> u32 {
  std::fs::symlink_metadata(path).unwrap().permissions().mode() & 0o7777
}

// covers: POST-032, POST-057
#[test]
fn debian_noop_stubs_written_0755_under_flatroot_bin() {
  // The Debian NOOP_STUBS roster (debian.rs:80-104) must each land as an
  // executable `#!/bin/sh\nexit 0\n` stand-in under .flatroot/bin (the dir is
  // created 0755 by StubSet::bin_dir, stubs.rs:42-46/53-61).
  let tmp = TempDir::new().unwrap();
  PostInstall::Debian.stubs_install(&Rootfs::new(tmp.path())).unwrap();

  let bin_dir = tmp.path().join(".flatroot/bin");
  assert!(bin_dir.is_dir(), "stub bin dir must exist");

  // A representative spread across the roster (the full set is debian.rs:80-104).
  for name in [
    "chown",
    "chgrp",
    "chmod",
    "systemctl",
    "invoke-rc.d",
    "start-stop-daemon",
    "adduser",
    "dpkg-trigger",
    "dpkg-divert",
    "py3compile",
  ] {
    let p = stub_path(tmp.path(), name);
    let body = std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("missing noop stub {name}"));
    assert_eq!(body, "#!/bin/sh\nexit 0\n", "noop stub {name} body");
    assert_eq!(mode(&p), 0o755, "noop stub {name} mode");
  }
}

// covers: POST-033
#[test]
fn debian_dpkg_stub_answers_compare_versions_and_status() {
  // The dpkg stand-in (debian.rs:114-146) must answer --compare-versions via
  // `sort -V`, report --status as "Status: install ok installed", and exit 0 on
  // anything else. We execute the real stub body with the host /bin/sh.
  let tmp = TempDir::new().unwrap();
  PostInstall::Debian.stubs_install(&Rootfs::new(tmp.path())).unwrap();
  let dpkg = stub_path(tmp.path(), "dpkg");
  assert!(dpkg.exists(), "dpkg stub must be installed");

  // 1.0 lt 2.0  -> true (exit 0)
  let lt = std::process::Command::new(&dpkg)
    .args(["--compare-versions", "1.0", "lt", "2.0"])
    .output()
    .unwrap();
  assert!(lt.status.success(), "1.0 lt 2.0 should succeed (exit 0)");

  // 2.0 lt 1.0  -> false (non-zero)
  let not_lt = std::process::Command::new(&dpkg)
    .args(["--compare-versions", "2.0", "lt", "1.0"])
    .output()
    .unwrap();
  assert!(!not_lt.status.success(), "2.0 lt 1.0 should fail (non-zero)");

  // 1.0 eq 1.0  -> true
  let eq = std::process::Command::new(&dpkg)
    .args(["--compare-versions", "1.0", "eq", "1.0"])
    .output()
    .unwrap();
  assert!(eq.status.success(), "1.0 eq 1.0 should succeed");

  // --status prints the installed line and exits 0
  let status = std::process::Command::new(&dpkg).args(["-s", "bash"]).output().unwrap();
  assert!(status.status.success(), "--status should exit 0");
  assert!(
    String::from_utf8_lossy(&status.stdout).contains("Status: install ok installed"),
    "--status must report installed"
  );

  // Anything else -> exit 0
  let other = std::process::Command::new(&dpkg)
    .args(["--configure", "-a"])
    .output()
    .unwrap();
  assert!(other.status.success(), "unhandled dpkg verb must exit 0");
}

// covers: POST-034
#[test]
fn debian_update_alternatives_stub_creates_primary_and_slave_symlinks() {
  // The update-alternatives stand-in (debian.rs:155-183) must create the
  // primary symlink and each --slave symlink. Run the real stub body and check
  // the links land where requested.
  let tmp = TempDir::new().unwrap();
  PostInstall::Debian.stubs_install(&Rootfs::new(tmp.path())).unwrap();
  let alts = stub_path(tmp.path(), "update-alternatives");
  assert!(alts.exists(), "update-alternatives stub must be installed");

  let work = TempDir::new().unwrap();
  let target_primary = work.path().join("real-editor");
  let link_primary = work.path().join("editor");
  let target_slave = work.path().join("real-editor.1.gz");
  let link_slave = work.path().join("editor.1.gz");
  std::fs::write(&target_primary, "x").unwrap();
  std::fs::write(&target_slave, "x").unwrap();

  let out = std::process::Command::new(&alts)
    .args([
      "--install",
      link_primary.to_str().unwrap(),
      "editor",
      target_primary.to_str().unwrap(),
      "100",
      "--slave",
      link_slave.to_str().unwrap(),
      "editor.1.gz",
      target_slave.to_str().unwrap(),
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "update-alternatives --install must exit 0");

  assert!(
    std::fs::symlink_metadata(&link_primary)
      .unwrap()
      .file_type()
      .is_symlink(),
    "primary symlink must be created"
  );
  assert!(std::fs::symlink_metadata(&link_slave).unwrap().file_type().is_symlink(), "slave symlink must be created");
  assert_eq!(std::fs::read_link(&link_primary).unwrap(), target_primary, "primary symlink target");
  assert_eq!(std::fs::read_link(&link_slave).unwrap(), target_slave, "slave symlink target");
}

// covers: POST-035
#[test]
fn debian_debconf_confmodule_and_frontend_seeded() {
  // The debconf confmodule (no-op db_* functions) and the frontend stub must be
  // seeded so scripts that source `. /usr/share/debconf/confmodule` do not hang
  // (debian.rs:54-56/193-216, stubs.rs:114-124).
  let tmp = TempDir::new().unwrap();
  PostInstall::Debian.stubs_install(&Rootfs::new(tmp.path())).unwrap();

  let confmodule = tmp.path().join("usr/share/debconf/confmodule");
  let body = std::fs::read_to_string(&confmodule).expect("confmodule must be seeded");
  // A spread of the no-op db_* shell functions the stub defines.
  for func in ["db_input()", "db_go()", "db_get()", "db_set()", "db_stop()"] {
    assert!(body.contains(func), "confmodule must define {func}");
  }
  // confmodule is sourced (not executed), so it is seeded non-exec.
  assert_eq!(mode(&confmodule) & 0o111, 0, "confmodule must not be executable");

  let frontend = tmp.path().join("usr/share/debconf/frontend");
  let fbody = std::fs::read_to_string(&frontend).expect("frontend stub must be seeded");
  assert_eq!(fbody, "#!/bin/sh\nexit 0\n", "frontend stub body");
  assert_eq!(mode(&frontend), 0o755, "frontend stub must be executable 0755");
}

// covers: POST-036
#[test]
fn debian_etc_shells_seeded_only_if_missing_add_shell_guarded() {
  // seed_keep must leave a pre-existing etc/shells untouched (debian.rs:59,
  // stubs.rs:131-140), while the add-shell stub appends idempotently behind a
  // `grep -qx` guard (debian.rs:224-230).
  let tmp = TempDir::new().unwrap();

  // (a) Fresh tree: seed_keep writes the default /etc/shells.
  PostInstall::Debian.stubs_install(&Rootfs::new(tmp.path())).unwrap();
  let shells = tmp.path().join("etc/shells");
  let seeded = std::fs::read_to_string(&shells).expect("etc/shells must be seeded when missing");
  assert!(seeded.contains("/bin/bash"), "seeded etc/shells must list /bin/bash");

  // (b) Pre-existing custom content is preserved across a second stubs_install.
  let tmp2 = TempDir::new().unwrap();
  std::fs::create_dir_all(tmp2.path().join("etc")).unwrap();
  std::fs::write(tmp2.path().join("etc/shells"), "/custom/shell\n").unwrap();
  PostInstall::Debian.stubs_install(&Rootfs::new(tmp2.path())).unwrap();
  assert_eq!(
    std::fs::read_to_string(tmp2.path().join("etc/shells")).unwrap(),
    "/custom/shell\n",
    "existing etc/shells must be left untouched by seed_keep"
  );

  // (c) The add-shell stub carries the grep-qx idempotency guard + append.
  let add_shell = std::fs::read_to_string(stub_path(tmp.path(), "add-shell")).unwrap();
  assert!(add_shell.contains("grep -qx"), "add-shell must guard with grep -qx");
  assert!(add_shell.contains(">> /etc/shells"), "add-shell must append to /etc/shells");
}

// covers: POST-039
#[test]
fn arch_stubs_use_noop_keep_and_vercmp_equal_without_cache_regen() {
  // Arch stubs (arch.rs:43-52/66-75) install the noop roster via noop_keep and a
  // vercmp stand-in echoing equal ("0"); crucially they must NOT stub the
  // content cache/index regenerators, or that real content would be lost.
  let tmp = TempDir::new().unwrap();
  PostInstall::Arch.stubs_install(&Rootfs::new(tmp.path())).unwrap();

  for name in [
    "groupadd",
    "useradd",
    "usermod",
    "systemd-tmpfiles",
    "install-info",
    "xdg-icon-resource",
  ] {
    let p = stub_path(tmp.path(), name);
    assert_eq!(
      std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("missing arch noop stub {name}")),
      "#!/bin/sh\nexit 0\n",
      "arch noop stub {name} body"
    );
    assert_eq!(mode(&p), 0o755, "arch noop stub {name} mode");
  }

  let vercmp = std::fs::read_to_string(stub_path(tmp.path(), "vercmp")).expect("vercmp stub");
  assert_eq!(vercmp, "#!/bin/sh\necho 0\n", "vercmp must echo 0 (equal)");
  assert_eq!(mode(&stub_path(tmp.path(), "vercmp")), 0o755, "vercmp mode");

  // Cache/index regenerators must remain real (never stubbed).
  for never in [
    "fc-cache",
    "gtk-update-icon-cache",
    "update-mime-database",
    "gdk-pixbuf-query-loaders",
    "glib-compile-schemas",
  ] {
    assert!(!stub_path(tmp.path(), never).exists(), "arch must NOT stub cache regenerator {never}");
  }
}

// covers: POST-043
#[test]
fn alpine_stubs_neutralize_rc_and_apk_via_noop_keep() {
  // Alpine stubs (alpine.rs:38-42/54) neutralize the service-management and apk
  // commands via noop_keep — present as 0755 `#!/bin/sh\nexit 0\n` stand-ins.
  let tmp = TempDir::new().unwrap();
  PostInstall::Alpine.stubs_install(&Rootfs::new(tmp.path())).unwrap();

  for name in ["rc-update", "rc-service", "openrc", "apk"] {
    let p = stub_path(tmp.path(), name);
    assert_eq!(
      std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("missing alpine noop stub {name}")),
      "#!/bin/sh\nexit 0\n",
      "alpine noop stub {name} body"
    );
    assert_eq!(mode(&p), 0o755, "alpine noop stub {name} mode");
  }
}

// covers: POST-058
#[test]
fn keep_variants_never_overwrite_existing_tailored_stub() {
  // noop_keep / script_keep / seed_keep must defer to an existing file (first
  // intent wins), while the plain variants overwrite. We exercise this through
  // Arch (noop_keep + script_keep) and Debian (seed_keep vs plain script).
  let tmp = TempDir::new().unwrap();
  std::fs::create_dir_all(tmp.path().join(".flatroot/bin")).unwrap();

  // Pre-seed a tailored vercmp and a tailored noop-roster member.
  std::fs::write(stub_path(tmp.path(), "vercmp"), "TAILORED-VERCMP").unwrap();
  std::fs::write(stub_path(tmp.path(), "useradd"), "TAILORED-USERADD").unwrap();

  PostInstall::Arch.stubs_install(&Rootfs::new(tmp.path())).unwrap();

  // script_keep (vercmp) and noop_keep (useradd) must NOT clobber the tailored
  // bodies that were already present.
  assert_eq!(
    std::fs::read_to_string(stub_path(tmp.path(), "vercmp")).unwrap(),
    "TAILORED-VERCMP",
    "script_keep must not overwrite an existing vercmp"
  );
  assert_eq!(
    std::fs::read_to_string(stub_path(tmp.path(), "useradd")).unwrap(),
    "TAILORED-USERADD",
    "noop_keep must not overwrite an existing useradd"
  );

  // Plain `script` (Debian dpkg) DOES overwrite a pre-existing file.
  let tmp2 = TempDir::new().unwrap();
  std::fs::create_dir_all(tmp2.path().join(".flatroot/bin")).unwrap();
  std::fs::write(stub_path(tmp2.path(), "dpkg"), "OLD-DPKG").unwrap();
  PostInstall::Debian.stubs_install(&Rootfs::new(tmp2.path())).unwrap();
  assert_ne!(
    std::fs::read_to_string(stub_path(tmp2.path(), "dpkg")).unwrap(),
    "OLD-DPKG",
    "plain script() must overwrite an existing dpkg stub"
  );

  // seed_keep (etc/shells) must defer to existing content.
  let tmp3 = TempDir::new().unwrap();
  std::fs::create_dir_all(tmp3.path().join("etc")).unwrap();
  std::fs::write(tmp3.path().join("etc/shells"), "KEEP-ME\n").unwrap();
  PostInstall::Debian.stubs_install(&Rootfs::new(tmp3.path())).unwrap();
  assert_eq!(
    std::fs::read_to_string(tmp3.path().join("etc/shells")).unwrap(),
    "KEEP-ME\n",
    "seed_keep must not overwrite an existing etc/shells"
  );
}

/// POST: the registry that replaced the per-distro forwarders maps each
/// supported distribution to its post-install strategy. This is the runnable
/// proof the `enum PostInstall` fold preserved the old forwarding (cachyos→Arch;
/// the RPM family + ubuntu→Debian) — it needs no sandbox or network.
#[test]
fn distro_post_install_strategy_mapping() {
  use flatroot::distro::KindDistro;
  for kind in [
    KindDistro::Debian,
    KindDistro::Ubuntu,
    KindDistro::Centos,
    KindDistro::Fedora,
    KindDistro::Alma,
    KindDistro::Rocky,
    KindDistro::Opensuse,
  ] {
    assert_eq!(kind.post_install(), PostInstall::Debian, "{kind:?} should use the Debian strategy");
  }
  assert_eq!(KindDistro::ArchLinux.post_install(), PostInstall::Arch);
  assert_eq!(KindDistro::Cachyos.post_install(), PostInstall::Arch, "CachyOS folds onto the Arch strategy");
  assert_eq!(KindDistro::Alpine.post_install(), PostInstall::Alpine);
}
