//! Post-install ldconfig + cache-rebuild hooks phase (POST area, prefix POST).
//!
//! The hooks phase walks a fixed 12-entry table of content-derived cache
//! builders, runs each one whose builder binary and source content are present,
//! and treats the rest as not-applicable rather than as failures (hooks.rs).
//! These tests drive the public `PostInstallPlan` with `[Phase::Hooks]` (or
//! `[Phase::Ldconfig]`) against a real bash rootfs into which fake builders are
//! dropped. Each fake records the argv it was invoked with (or its own path, or
//! appends its label to an order log) into the bind-mounted rootfs, so a single
//! sandboxed run yields an observable record of which hooks ran, in what order,
//! with what arguments, and that a failing or absent hook never stops the rest.
//!
//! ldconfig coverage spans the glibc happy path (black-box, distro axis), the
//! musl auto-skip (Alpine), and the non-zero-exit-is-still-Ok contract.
//!
//! Sandbox-touching tests gate on `Sandbox::available()`.

mod post;

use std::path::Path;

use flatroot::postinstall::{Phase, PostInstall, PostInstallPlan};
use serial_test::serial;

/// Drop a fake builder at `usr/bin/<name>` that appends `<label>` to the shared
/// hook order log, then exits 0.
fn fake_order_builder(root: &Path, name: &str, label: &str) {
  post::exec_file(
    root,
    &format!("usr/bin/{name}"),
    &format!("#!/bin/sh\nprintf '%s\\n' '{label}' >> /.flatroot/hook-order.log\nexit 0\n"),
  );
}

/// Drop a fake builder at `usr/bin/<name>` that records the argv it received
/// (one element per line) into `<probe_rel>`, then exits 0.
fn fake_argv_builder(root: &Path, name: &str, probe_rel: &str) {
  post::exec_file(root, &format!("usr/bin/{name}"), &post::probe_builder_body(probe_rel));
}

/// Run the hooks phase against the rootfs (any PostInstall works — the Hooks
/// phase does not consult it). Panics if the plan errors.
fn run_hooks(root: &Path) {
  PostInstallPlan::new(root, &[Phase::Hooks], "x86_64")
    .run(&PostInstall::Debian)
    .expect("hooks plan must complete Ok");
}

// ---------------------------------------------------------------------------
// ldconfig phase
// ---------------------------------------------------------------------------

/// The ldconfig phase locates the rootfs's own `ldconfig`, runs it, populates
/// `/etc/ld.so.cache`, and reports "ldconfig completed" — across glibc distros
/// of both package families (ldconfig.rs:38-57; rootfs.rs:82-100).
// covers: POST-020
#[test]
#[serial]
fn ldconfig_phase_populates_cache_on_glibc_distros() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  for remote in ["debian:bookworm", "fedora:40", "arch:rolling", "opensuse:tumbleweed"] {
    let cache = post::cache_dir();
    let root = post::root_dir();
    let out = post::install(remote, cache.path(), root.path(), &["--postinstall=ldconfig", "bash"]);
    post::ok(&out, &format!("{remote} ldconfig"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("ldconfig completed"), "{remote}: ldconfig phase must report completion:\n{stderr}");
    assert!(root.path().join("etc/ld.so.cache").exists(), "{remote}: ldconfig must populate /etc/ld.so.cache");
  }
}

/// On a musl (Alpine) tree there is no `ldconfig`, so the phase auto-skips with
/// "ldconfig not found, skipping" and is NOT treated as an error
/// (ldconfig.rs:39-45).
// covers: POST-021
#[test]
#[serial]
fn ldconfig_phase_auto_skips_on_musl_alpine() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install("alpine:v3.20", cache.path(), root.path(), &["--postinstall=ldconfig", "busybox"]);
  post::ok(&out, "alpine ldconfig skip");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("ldconfig not found, skipping"), "alpine ldconfig phase must auto-skip:\n{stderr}");
}

/// A non-zero ldconfig exit is reported but does NOT fail the build: the phase
/// still returns Ok (ldconfig.rs:50-56). We overwrite the rootfs's ldconfig with
/// a stub that exits 5, then drive the Ldconfig phase via the library API.
// covers: POST-022
#[test]
#[serial]
fn ldconfig_nonzero_exit_is_reported_but_still_ok() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());

  // Replace the discoverable ldconfig with a failing stub. Debian ships it at
  // usr/sbin/ldconfig (the first BIN_DIRS hit after usr/bin, which it does not
  // use); ensure no usr/bin/ldconfig shadows it.
  let _ = std::fs::remove_file(root.path().join("usr/bin/ldconfig"));
  post::exec_file(root.path(), "usr/sbin/ldconfig", "#!/bin/sh\nexit 5\n");

  PostInstallPlan::new(root.path(), &[Phase::Ldconfig], "x86_64")
    .run(&PostInstall::Debian)
    .expect("a non-zero ldconfig exit must NOT fail the build");
}

// ---------------------------------------------------------------------------
// hooks phase: table order, absent skips, binary_find through a real run
// ---------------------------------------------------------------------------

/// The hooks table runs its present entries in declared order, skips entries
/// whose builder binary is absent, and resolves each present builder via
/// `binary_find` during a real run (hooks.rs:145-167/187-274; rootfs.rs:82-100).
/// We drop fakes for a subset spanning the table and assert they appear in the
/// table's order while an omitted builder (mime-database) does not.
// covers: POST-048, POST-049, POST-056
#[test]
#[serial]
fn hooks_run_present_builders_in_table_order_skipping_absent() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());
  let _ = std::fs::remove_file(root.path().join(".flatroot/hook-order.log"));

  // Present (in TABLE order): ca-certificates(1), locale-gen(2), gdk-pixbuf(3),
  // desktop-database(8), font-cache(9). Deliberately ABSENT: update-mime-database(4).
  fake_order_builder(root.path(), "update-ca-certificates", "ca-certificates");
  fake_order_builder(root.path(), "locale-gen", "locale-gen");
  fake_order_builder(root.path(), "gdk-pixbuf-query-loaders", "gdk-pixbuf");
  fake_order_builder(root.path(), "update-desktop-database", "desktop-database");
  fake_order_builder(root.path(), "fc-cache", "font-cache");
  // Make sure no real / leftover mime builder is present.
  let _ = std::fs::remove_file(root.path().join("usr/bin/update-mime-database"));

  run_hooks(root.path());

  let order: Vec<String> = std::fs::read_to_string(root.path().join(".flatroot/hook-order.log"))
    .expect("hook order log must exist")
    .lines()
    .map(|s| s.to_string())
    .collect();
  assert_eq!(
    order,
    vec![
      "ca-certificates",
      "locale-gen",
      "gdk-pixbuf",
      "desktop-database",
      "font-cache"
    ],
    "present hooks must run in declared table order; absent ones skipped"
  );
  assert!(!order.iter().any(|l| l == "mime-database"), "an absent builder's hook must be skipped");
}

/// Both CA-trust families are listed in the table; whichever the rootfs ships is
/// run, and an absent family is silently skipped (hooks.rs:191-203). With both
/// present, both run.
// covers: POST-054
#[test]
#[serial]
fn ca_trust_both_families_run_when_present() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());

  fake_argv_builder(root.path(), "update-ca-trust", ".flatroot/probe-catrust");
  fake_argv_builder(root.path(), "update-ca-certificates", ".flatroot/probe-cacerts");

  run_hooks(root.path());

  assert!(
    root.path().join(".flatroot/probe-catrust").exists(),
    "update-ca-trust (rpm/arch family) must run when present"
  );
  assert!(
    root.path().join(".flatroot/probe-cacerts").exists(),
    "update-ca-certificates (deb/alpine/opensuse family) must run when present"
  );
}

/// A hook builder exiting non-zero is reported but does not stop the remaining
/// hooks: the loop continues and the phase returns Ok (hooks.rs:163-166/291-295).
/// An early hook (gdk-pixbuf, idx 3) exits non-zero; a later one (font-cache,
/// idx 9) must still run.
// covers: POST-055
#[test]
#[serial]
fn hook_nonzero_exit_does_not_stop_remaining() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());

  post::exec_file(root.path(), "usr/bin/gdk-pixbuf-query-loaders", "#!/bin/sh\nexit 7\n");
  fake_argv_builder(root.path(), "fc-cache", ".flatroot/probe-font");

  // Must NOT error despite the failing early hook.
  run_hooks(root.path());

  assert!(root.path().join(".flatroot/probe-font").exists(), "a later hook must still run after an earlier hook fails");
}

// ---------------------------------------------------------------------------
// hooks phase: per-hook argv shapes and preconditions
// ---------------------------------------------------------------------------

/// The icon-cache hook is skipped unless the hicolor theme index is present;
/// when present it is invoked as `['<bin>','-q','/usr/share/icons/hicolor']`
/// (hooks.rs:91-100/240-245).
// covers: POST-050
#[test]
#[serial]
fn icon_cache_hook_gated_on_hicolor_index() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());
  fake_argv_builder(root.path(), "gtk-update-icon-cache", ".flatroot/probe-icon");

  // (a) Without the hicolor index.theme the hook is skipped: no probe.
  let _ = std::fs::remove_dir_all(root.path().join("usr/share/icons/hicolor"));
  let _ = std::fs::remove_file(root.path().join(".flatroot/probe-icon"));
  run_hooks(root.path());
  assert!(
    post::probe_read(root.path(), ".flatroot/probe-icon").is_none(),
    "icon-cache must be skipped without hicolor/index.theme"
  );

  // (b) With the index present the hook runs with the documented argv.
  post::exec_file(root.path(), "usr/share/icons/hicolor/index.theme", "[Icon Theme]\n");
  run_hooks(root.path());
  let argv = post::probe_read(root.path(), ".flatroot/probe-icon").expect("icon-cache must run once index present");
  let args: Vec<&str> = argv.lines().collect();
  assert_eq!(args, vec!["-q", "/usr/share/icons/hicolor"], "icon-cache argv tail");
}

/// The locale-gen hook seeds `/etc/locale.gen` with `en_US.UTF-8 UTF-8` when it
/// is missing, then runs with no extra arguments; an existing file is left
/// untouched (hooks.rs:101-108/205-210).
// covers: POST-051
#[test]
#[serial]
fn locale_gen_hook_seeds_when_missing_and_leaves_existing() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());
  fake_argv_builder(root.path(), "locale-gen", ".flatroot/probe-locale");

  // (a) Missing -> seeded with the default entry, hook runs with no extra args.
  let _ = std::fs::remove_file(root.path().join("etc/locale.gen"));
  let _ = std::fs::remove_file(root.path().join(".flatroot/probe-locale"));
  run_hooks(root.path());
  let seeded = std::fs::read_to_string(root.path().join("etc/locale.gen")).expect("locale.gen must be seeded");
  assert!(seeded.contains("en_US.UTF-8 UTF-8"), "locale.gen seeded content: {seeded}");
  let argv = post::probe_read(root.path(), ".flatroot/probe-locale").expect("locale-gen must run");
  assert!(argv.trim().is_empty(), "locale-gen takes no extra arguments, got: {argv:?}");

  // (b) Existing file is left untouched.
  std::fs::write(root.path().join("etc/locale.gen"), "de_DE.UTF-8 UTF-8\n").unwrap();
  let _ = std::fs::remove_file(root.path().join(".flatroot/probe-locale"));
  run_hooks(root.path());
  assert_eq!(
    std::fs::read_to_string(root.path().join("etc/locale.gen")).unwrap(),
    "de_DE.UTF-8 UTF-8\n",
    "an existing locale.gen must be left untouched"
  );
}

/// The gio-modules hook points the builder at the resolved `gio/modules` dir
/// when one exists, else the flat default `/usr/lib/gio/modules`; argv is
/// `['<bin>',<dir>]` (hooks.rs:85-90; rootfs.rs:109-117).
// covers: POST-052
#[test]
#[serial]
fn gio_modules_hook_uses_resolved_dir_else_flat_default() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());
  fake_argv_builder(root.path(), "gio-querymodules", ".flatroot/probe-gio");

  // (a) No gio/modules dir anywhere -> the flat default.
  let _ = std::fs::remove_dir_all(root.path().join("usr/lib/x86_64-linux-gnu/gio"));
  let _ = std::fs::remove_dir_all(root.path().join("usr/lib/gio"));
  let _ = std::fs::remove_file(root.path().join(".flatroot/probe-gio"));
  run_hooks(root.path());
  let argv = post::probe_read(root.path(), ".flatroot/probe-gio").expect("gio-querymodules must run");
  assert_eq!(
    argv.lines().collect::<Vec<_>>(),
    vec!["/usr/lib/gio/modules"],
    "gio-modules must fall back to the flat default dir"
  );

  // (b) A multiarch gio/modules dir present -> the resolved dir is used.
  std::fs::create_dir_all(root.path().join("usr/lib/x86_64-linux-gnu/gio/modules")).unwrap();
  let _ = std::fs::remove_file(root.path().join(".flatroot/probe-gio"));
  run_hooks(root.path());
  let argv2 = post::probe_read(root.path(), ".flatroot/probe-gio").expect("gio-querymodules must run again");
  assert_eq!(
    argv2.lines().collect::<Vec<_>>(),
    vec!["/usr/lib/x86_64-linux-gnu/gio/modules"],
    "gio-modules must use the resolved multiarch dir"
  );
}

/// An arch-class-suffixed builder (`-64`/`-32`) is located via the basename
/// preference list, and when both the plain and suffixed names exist the first
/// listed (plain) wins (hooks.rs:27-46; rootfs.rs:82-100). Verified through a
/// real hooks run by having the resolved builder record its own path ($0).
// covers: POST-053
#[test]
#[serial]
fn arch_suffixed_cache_builder_located_first_listed_wins() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());

  // A builder that records its own path ($0), so we learn which name resolved.
  let record_self = "#!/bin/sh\nprintf '%s\\n' \"$0\" > /.flatroot/probe-pixself\nexit 0\n";

  // (a) Only the -64 variant exists -> the suffixed name resolves and runs.
  let _ = std::fs::remove_file(root.path().join("usr/bin/gdk-pixbuf-query-loaders"));
  post::exec_file(root.path(), "usr/bin/gdk-pixbuf-query-loaders-64", record_self);
  let _ = std::fs::remove_file(root.path().join(".flatroot/probe-pixself"));
  run_hooks(root.path());
  assert_eq!(
    post::probe_read(root.path(), ".flatroot/probe-pixself").map(|s| s.trim().to_string()),
    Some("/usr/bin/gdk-pixbuf-query-loaders-64".to_string()),
    "the -64 suffixed builder must be located when it is the only one present"
  );

  // (b) Both plain and -64 present -> the first listed (plain) wins.
  post::exec_file(root.path(), "usr/bin/gdk-pixbuf-query-loaders", record_self);
  let _ = std::fs::remove_file(root.path().join(".flatroot/probe-pixself"));
  run_hooks(root.path());
  assert_eq!(
    post::probe_read(root.path(), ".flatroot/probe-pixself").map(|s| s.trim().to_string()),
    Some("/usr/bin/gdk-pixbuf-query-loaders".to_string()),
    "the plain (first-listed) builder must win over the suffixed variant"
  );
}
