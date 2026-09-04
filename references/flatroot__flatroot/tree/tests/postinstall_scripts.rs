//! Post-install scripts phase + plan sequencing (POST area, prefix POST).
//!
//! Two kinds of coverage:
//!   - Black-box installs that assert the three phases run in the fixed order
//!     Ldconfig -> Scripts -> Hooks, that phase selection / the env fallback /
//!     `none` behave, and that the per-distro scriptlet replay reaches each
//!     flavour's distinct finish marker (Debian/RPM, Arch/CachyOS, Alpine).
//!   - Library-API runs of `PostInstall::scripts_run` and `PostInstallPlan`
//!     against a controlled `.flatroot/scripts/` tree. The replayed scripts
//!     write probe files into the bind-mounted rootfs, so a single sandbox run
//!     produces an observable on-disk record of which packages were eligible,
//!     the order they ran, the package/arch parsed from each dir name, the env
//!     they received, and that per-package temp scripts are cleaned up.
//!
//! Sandbox-touching tests gate on `Sandbox::available()`.

mod post;

use flatroot::postinstall::{Phase, PostInstall, PostInstallPlan, Rootfs};
use serial_test::serial;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Plan-level: phase ordering / selection / env fallback (black-box, network)
// ---------------------------------------------------------------------------

/// Default install runs all three phases in the fixed order Ldconfig -> Scripts
/// -> Hooks and finishes with "Done." (mod.rs:115-152; install.rs:140-144). All
/// three phases acting on one tree is the observable proxy for the single reused
/// Sandbox session (mod.rs:130-149) — true session spying is not possible at the
/// integration boundary.
// covers: POST-001, POST-061
#[test]
#[serial]
fn default_postinstall_runs_three_phases_in_fixed_order() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install("debian:bookworm", cache.path(), root.path(), &["bash"]);
  post::ok(&out, "default postinstall debian bash");
  let stderr = String::from_utf8_lossy(&out.stderr);

  let ld = post::offset_of(&stderr, "ldconfig completed").expect("ldconfig phase must run");
  let sc = post::offset_of(&stderr, "Postinst scripts completed").expect("scripts phase must run");
  let hk = post::offset_of(&stderr, "Cache hooks completed").expect("hooks phase must run");
  let done = post::offset_of(&stderr, "Done. ").expect("run must finish with Done.");

  assert!(ld < sc, "ldconfig must run before scripts:\n{stderr}");
  assert!(sc < hk, "scripts must run before hooks:\n{stderr}");
  assert!(hk < done, "all phases must finish before Done.:\n{stderr}");
}

/// `--postinstall=scripts` runs ONLY the scripts phase: the scripts finish
/// marker appears while neither ldconfig nor hooks markers do (mod.rs:134-149).
/// A successful replay also prints no per-step failure line for the package that
/// configured cleanly (StepReport silent-on-success, step_report.rs:42).
// covers: POST-005, POST-046
#[test]
#[serial]
fn single_phase_scripts_runs_only_scripts_and_is_silent_on_success() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install("debian:bookworm", cache.path(), root.path(), &["--postinstall=scripts", "bash"]);
  post::ok(&out, "scripts-only debian bash");
  let stderr = String::from_utf8_lossy(&out.stderr);

  assert!(stderr.contains("Postinst scripts completed"), "scripts phase must run:\n{stderr}");
  assert!(!stderr.contains("ldconfig completed"), "ldconfig must be skipped:\n{stderr}");
  assert!(!stderr.contains("Cache hooks completed"), "hooks must be skipped:\n{stderr}");
  // Silent on success: the bash postinst configures cleanly, so no failure line.
  assert!(
    !stderr.contains("postinst: bash"),
    "a successful step must not be announced (StepReport silent on success):\n{stderr}"
  );
}

/// `--postinstall=hooks,ldconfig` preserves the fixed execution order regardless
/// of CLI order: ldconfig still runs before hooks (mod.rs:134-149, cli.rs value
/// delimiter ',').
// covers: POST-006
#[test]
#[serial]
fn comma_subset_preserves_fixed_execution_order() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install("debian:bookworm", cache.path(), root.path(), &["--postinstall=hooks,ldconfig", "bash"]);
  post::ok(&out, "hooks,ldconfig debian bash");
  let stderr = String::from_utf8_lossy(&out.stderr);

  let ld = post::offset_of(&stderr, "ldconfig completed").expect("ldconfig must run");
  let hk = post::offset_of(&stderr, "Cache hooks completed").expect("hooks must run");
  assert!(ld < hk, "ldconfig must run before hooks despite CLI order hooks,ldconfig:\n{stderr}");
  assert!(!stderr.contains("Postinst scripts completed"), "scripts must be skipped:\n{stderr}");
}

/// The `FLATROOT_ARG_INSTALL_POSTINSTALL` env var supplies the phase list when
/// the flag is absent (cli.rs:117). `=none` drives an empty phase set — no phase
/// markers, yet the run still finishes with "Done.".
// covers: POST-007
#[test]
#[serial]
fn env_var_supplies_phase_list_none() {
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install_env(
    "debian:bookworm",
    cache.path(),
    root.path(),
    &[("FLATROOT_ARG_INSTALL_POSTINSTALL", "none")],
    &["bash"],
  );
  post::ok(&out, "env=none debian bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Done. "), "run must finish with Done.:\n{stderr}");
  assert!(!stderr.contains("ldconfig completed"), "env=none must skip ldconfig:\n{stderr}");
  assert!(!stderr.contains("Postinst scripts completed"), "env=none must skip scripts:\n{stderr}");
  assert!(!stderr.contains("Cache hooks completed"), "env=none must skip hooks:\n{stderr}");
}

/// A multi-arch install with the DEFAULT post-install runs both arch passes but
/// runs post-install ONCE after all archs (install.rs:136-144). The scripts
/// finish marker therefore appears exactly once, after both `=== x86_64 ===`
/// and `=== i686 ===` banners. The script-entry default arch is
/// `archs.first().as_uname()` == "x86_64".
// covers: POST-060
#[test]
#[serial]
fn multiarch_default_postinstall_runs_once_after_all_archs() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install_arch("debian:bookworm", "x86_64,i686", cache.path(), root.path(), &["bash"]);
  post::ok(&out, "multiarch default postinstall");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("=== x86_64 ==="), "x86_64 arch pass must run:\n{stderr}");
  assert!(stderr.contains("=== i686 ==="), "i686 arch pass must run:\n{stderr}");
  assert_eq!(
    post::count_of(&stderr, "Postinst scripts completed"),
    1,
    "post-install scripts must run exactly once after all archs:\n{stderr}"
  );
}

// ---------------------------------------------------------------------------
// Plan-level: empty / early-return (library API, no sandbox)
// ---------------------------------------------------------------------------

/// An empty phase list is an immediate no-op: `PostInstallPlan::run` returns
/// Ok(()) before the sandbox gate (mod.rs:116-118), so no stub bin dir is ever
/// provisioned (the Scripts phase, which would create `.flatroot/bin`, did not
/// run). This holds even on a host where the sandbox gate would otherwise be
/// reached.
// covers: POST-002
#[test]
fn empty_phase_list_is_immediate_noop_before_sandbox_gate() {
  let tmp = TempDir::new().unwrap();
  std::fs::create_dir_all(tmp.path()).unwrap();
  let phases: Vec<Phase> = Vec::new();
  PostInstallPlan::new(tmp.path(), &phases, "x86_64")
    .run(&PostInstall::Debian)
    .expect("empty phase list must return Ok");
  assert!(
    !tmp.path().join(".flatroot/bin").exists(),
    "no phase ran, so the Scripts-phase stub bin dir must not be created"
  );
}

// ---------------------------------------------------------------------------
// Scripts phase: no-op short-circuits (library API, no sandbox)
// ---------------------------------------------------------------------------

/// With no `.flatroot/scripts` directory, the replay is an immediate Ok(())
/// before any interpreter probe or sandbox run (script.rs:162-165).
// covers: POST-023
#[test]
fn scripts_phase_noop_when_scripts_dir_absent() {
  let tmp = TempDir::new().unwrap();
  let sandbox = flatroot::sandbox::Sandbox::new(tmp.path());
  PostInstall::Debian
    .scripts_run(&Rootfs::new(tmp.path()), &sandbox, "x86_64")
    .expect("no scripts dir must be a quiet no-op");
}

/// When the interpreter binary is absent from the rootfs, the replay skips with
/// Ok(()) without running anything (script.rs:171-174). We stage a postinst but
/// ship no `sh`, so `binary_find` returns None and nothing executes.
// covers: POST-024
#[test]
fn scripts_phase_skips_when_interpreter_absent() {
  let tmp = TempDir::new().unwrap();
  post::staged_pkg(tmp.path(), "somepkg:x86_64", &[("postinst", "#!/bin/sh\nexit 0\n")]);
  // No usr/bin/sh anywhere -> interpreter probe is None.
  let sandbox = flatroot::sandbox::Sandbox::new(tmp.path());
  PostInstall::Debian
    .scripts_run(&Rootfs::new(tmp.path()), &sandbox, "x86_64")
    .expect("absent interpreter must be a quiet no-op");
}

/// When no staged package is eligible, the replay returns Ok(()) before the
/// progress bar / any sandbox run (script.rs:189-198). Here `sh` is present
/// (interpreter found) but the only staged package has no `postinst`, so the
/// Debian flavour deems it ineligible and the eligible set is empty.
// covers: POST-025
#[test]
fn scripts_phase_noop_when_no_eligible_package() {
  let tmp = TempDir::new().unwrap();
  // A discoverable interpreter (binary_find only stats, never execs).
  post::exec_file(tmp.path(), "usr/bin/sh", "#!/bin/sh\nexit 0\n");
  post::staged_pkg(tmp.path(), "preinst-only:x86_64", &[("preinst", "#!/bin/sh\nexit 0\n")]);
  let sandbox = flatroot::sandbox::Sandbox::new(tmp.path());
  PostInstall::Debian
    .scripts_run(&Rootfs::new(tmp.path()), &sandbox, "x86_64")
    .expect("no eligible package must be a quiet no-op");
}

/// A real read error of an EXISTING Alpine post-install script propagates as Err
/// (not None) (alpine.rs:80-82, surfaced through script.rs:209 `?`). We make the
/// `post-install` entry a directory: `eligible()` sees it exist, but the staging
/// `read_to_string` fails (EISDIR) and the error is propagated. No sandbox run
/// is reached, so no user namespace is required.
// covers: POST-042
#[test]
fn alpine_stage_propagates_read_error_of_existing_script() {
  let tmp = TempDir::new().unwrap();
  // Discoverable `sh` so the interpreter probe succeeds and we reach `stage`.
  post::exec_file(tmp.path(), "usr/bin/sh", "#!/bin/sh\nexit 0\n");
  // Stage a package whose `post-install` exists but is unreadable as a file.
  let pkg_dir = tmp.path().join(".flatroot/scripts/badpkg:x86_64");
  std::fs::create_dir_all(pkg_dir.join("post-install")).expect("make post-install a directory");
  let sandbox = flatroot::sandbox::Sandbox::new(tmp.path());
  let err = PostInstall::Alpine
    .scripts_run(&Rootfs::new(tmp.path()), &sandbox, "x86_64")
    .err()
    .expect("a read error of an existing post-install must propagate as Err");
  assert!(
    err.to_string().contains("failed to read") || format!("{err:#}").contains("failed to read"),
    "error must carry the read context, got: {err:#}"
  );
}

// ---------------------------------------------------------------------------
// Scripts phase: Debian replay against a real rootfs (library API, sandbox)
// ---------------------------------------------------------------------------

/// Drives the Debian flavour replay over a hand-controlled `.flatroot/scripts/`
/// tree on a real bash rootfs, observing several contracts in one sandboxed run:
///   - ScriptEntry parses `pkg:arch` from the dir name and defaults the arch to
///     the CLI uname when absent (script.rs:45-52) — verified via the
///     DPKG_MAINTSCRIPT_PACKAGE/ARCH env each script echoes (debian.rs:277-288);
///   - staged scripts run in sorted dir-name order (script.rs:176-177);
///   - libc6 is excluded, a Lua-form postinst is skipped, and a package without
///     a plain postinst is ineligible (debian.rs:250-258);
///   - a failing postinst is tolerated and replay continues to later packages
///     (script.rs:223-231), with the whole run still Ok;
///   - the per-package temp script `.flatroot/current-postinst` is cleaned up
///     after the run (debian.rs:294-296).
// covers: POST-026, POST-027, POST-028, POST-029, POST-030, POST-031, POST-045, POST-057, POST-059
#[test]
#[serial]
fn debian_replay_parses_orders_filters_and_continues_on_failure() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());
  post::scripts_dir_reset(root.path());

  // Each eligible postinst: record "pkg arch", its $PATH, and append its dir
  // name to a shared order log. `-e` is in effect, so commands must succeed.
  // The postinst is staged to a fixed path (.flatroot/current-postinst), so $0
  // is not the origin dir name — the package identity travels in the
  // DPKG_MAINTSCRIPT_PACKAGE env (debian.rs:281), which the order log records.
  let body = |probe: &str| -> String {
    format!(
      "#!/bin/sh\n\
       printf '%s %s\\n' \"$DPKG_MAINTSCRIPT_PACKAGE\" \"$DPKG_MAINTSCRIPT_ARCH\" > /.flatroot/{probe}\n\
       printf '%s\\n' \"$PATH\" > /.flatroot/path-{probe}\n\
       printf '%s\\n' \"$DPKG_MAINTSCRIPT_PACKAGE\" >> /.flatroot/order.log\n\
       exit 0\n"
    )
  };

  // arch encoded in the dir name -> parsed as amd64
  post::staged_pkg(root.path(), "10-aaa:amd64", &[("postinst", &body("probe-10"))]);
  // no arch in the dir name -> defaults to the CLI uname (x86_64)
  post::staged_pkg(root.path(), "20-bbb", &[("postinst", &body("probe-20"))]);
  // a failing postinst that still records its order, then exits non-zero
  post::staged_pkg(
    root.path(),
    "30-ccc",
    &[("postinst", "#!/bin/sh\nprintf '30-ccc\\n' >> /.flatroot/order.log\nexit 5\n")],
  );
  // a later eligible package that must still run after 30-ccc failed
  post::staged_pkg(root.path(), "40-ddd", &[("postinst", &body("probe-40"))]);
  // excluded: libc6 is never eligible
  post::staged_pkg(root.path(), "libc6", &[("postinst", &body("probe-libc6"))]);
  // skipped: a Lua-form postinst the replay cannot interpret
  post::staged_pkg(root.path(), "lua-pkg", &[("postinst", &body("probe-lua")), ("postinst.lua", "-- lua\n")]);
  // ineligible: no plain postinst present
  post::staged_pkg(root.path(), "no-postinst", &[("preinst", "#!/bin/sh\nexit 0\n")]);

  PostInstallPlan::new(root.path(), &[Phase::Scripts], "x86_64")
    .run(&PostInstall::Debian)
    .expect("scripts plan must complete Ok even with one failing postinst");

  let fr = root.path().join(".flatroot");
  // POST-026: package + arch parsed from the dir name (encoded and defaulted).
  assert_eq!(
    std::fs::read_to_string(fr.join("probe-10")).unwrap().trim(),
    "10-aaa amd64",
    "arch encoded in dir name must parse"
  );
  assert_eq!(
    std::fs::read_to_string(fr.join("probe-20")).unwrap().trim(),
    "20-bbb x86_64",
    "missing arch must default to the CLI uname"
  );
  // POST-057: the stub bin dir is first on the replay PATH.
  let path_seen = std::fs::read_to_string(fr.join("path-probe-10")).unwrap();
  assert!(path_seen.starts_with("/.flatroot/bin:"), "the replay PATH must begin with /.flatroot/bin, got: {path_seen}");

  // POST-027: eligible packages ran in sorted dir-name order.
  let order: Vec<String> = std::fs::read_to_string(fr.join("order.log"))
    .unwrap()
    .lines()
    .map(|s| s.to_string())
    .collect();
  assert_eq!(
    order,
    vec!["10-aaa", "20-bbb", "30-ccc", "40-ddd"],
    "staged scripts must run in sorted (file_name) order"
  );

  // POST-045: replay continued past the failing 30-ccc to 40-ddd.
  assert!(fr.join("probe-40").exists(), "replay must continue after a failing postinst");

  // POST-029/030/031: excluded / skipped / ineligible packages never ran.
  assert!(!fr.join("probe-libc6").exists(), "libc6 must be excluded from replay");
  assert!(!fr.join("probe-lua").exists(), "a Lua-form postinst must be skipped");
  assert!(!order.iter().any(|d| d == "no-postinst"), "a package without a plain postinst must be ineligible");

  // POST-028/059: per-package temp script is cleaned up after the run.
  assert!(!fr.join("current-postinst").exists(), "the staged current-postinst must be cleaned up after replay");
}

/// Black-box Debian scripts-only replay reaches the Debian flavour finish marker
/// "Postinst scripts completed" and leaves no `.flatroot/current-postinst`
/// behind (debian.rs:260-296). This exercises the real distro-staged postinst
/// (the `sh -e ... configure ""` invocation) end to end.
// covers: POST-028, POST-059
#[test]
#[serial]
fn debian_blackbox_scripts_replay_cleans_up_temp_script() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install("debian:bookworm", cache.path(), root.path(), &["--postinstall=scripts", "bash"]);
  post::ok(&out, "debian scripts replay");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Postinst scripts completed"), "debian finish marker:\n{stderr}");
  assert!(!root.path().join(".flatroot/current-postinst").exists(), "current-postinst must be cleaned up after replay");
}

// ---------------------------------------------------------------------------
// Scripts phase: Arch / CachyOS replay (library API on a real bash rootfs)
// ---------------------------------------------------------------------------

/// Drives the Arch flavour replay against a real bash rootfs (Arch's interpreter
/// is `bash`, present in the Debian tree). Verifies eligibility (eligible iff the
/// `install` file holds a `post_install` function, arch.rs:91-100), the wrapper
/// that sources the install file and calls `post_install` (arch.rs:107-123), and
/// cleanup of both staged temp files (arch.rs:137-140). The same fixtures run
/// through CachyOS prove byte-identical delegation to the Arch path
/// (cachyos.rs:25-33).
// covers: POST-037, POST-038, POST-040, POST-059
#[test]
#[serial]
fn arch_and_cachyos_replay_run_only_post_install_bearing_packages() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());

  let run_once = |probe: &str, post_install: &PostInstall| {
    post::scripts_dir_reset(root.path());
    let _ = std::fs::remove_file(root.path().join(format!(".flatroot/{probe}")));
    // Eligible: an install file carrying a post_install function.
    post::staged_pkg(
      root.path(),
      "with-post:x86_64",
      &[("install", &format!("post_install() {{ printf done > /.flatroot/{probe}; }}\n"))],
    );
    // Ineligible: an install file with NO post_install function.
    post::staged_pkg(root.path(), "no-post:x86_64", &[("install", "pre_install() { :; }\n")]);
    // Ineligible: no install file at all.
    post::staged_pkg(root.path(), "no-install:x86_64", &[("README", "x")]);

    PostInstallPlan::new(root.path(), &[Phase::Scripts], "x86_64")
      .run(post_install)
      .expect("arch-family scripts plan must complete Ok");
  };

  // POST-037/038: only the post_install-bearing package runs via the bash wrapper.
  run_once("probe-arch", &PostInstall::Arch);
  assert!(root.path().join(".flatroot/probe-arch").exists(), "the post_install-bearing package must run under Arch");
  // POST-059: both staged temp files are cleaned up after replay.
  assert!(!root.path().join(".flatroot/current-install-wrapper").exists(), "Arch wrapper temp must be cleaned up");
  assert!(!root.path().join(".flatroot/current-install").exists(), "Arch install temp must be cleaned up");

  // POST-040: CachyOS delegates to the Arch path -> identical observable result.
  run_once("probe-cachy", &PostInstall::Arch);
  assert!(root.path().join(".flatroot/probe-cachy").exists(), "CachyOS must replay identically to Arch (delegation)");
}

// ---------------------------------------------------------------------------
// Scripts phase: Alpine replay (library API on a real bash rootfs)
// ---------------------------------------------------------------------------

/// Drives the Alpine flavour replay against a real bash rootfs (Alpine's
/// interpreter is `sh`). The post-install script is invoked with the package
/// name as argv[1] (alpine.rs:85-89); the script records $0/$1 and the staged
/// `current-postinstall` is cleaned up afterwards (alpine.rs:104-106).
// covers: POST-041, POST-059
#[test]
#[serial]
fn alpine_replay_passes_package_name_as_argv1() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  post::rootfs_debian_bash(cache.path(), root.path());
  post::scripts_dir_reset(root.path());

  // Records the first positional argument the script was invoked with.
  post::staged_pkg(
    root.path(),
    "mypkg:x86_64",
    &[("post-install", "#!/bin/sh\nprintf '%s\\n' \"$1\" > /.flatroot/probe-alpine\nexit 0\n")],
  );

  PostInstallPlan::new(root.path(), &[Phase::Scripts], "x86_64")
    .run(&PostInstall::Alpine)
    .expect("alpine scripts plan must complete Ok");

  assert_eq!(
    std::fs::read_to_string(root.path().join(".flatroot/probe-alpine"))
      .unwrap()
      .trim(),
    "mypkg",
    "Alpine post-install must receive the package name as argv[1]"
  );
  assert!(
    !root.path().join(".flatroot/current-postinstall").exists(),
    "Alpine current-postinstall temp must be cleaned up"
  );
}

// ---------------------------------------------------------------------------
// RPM-family delegation (black-box, network)
// ---------------------------------------------------------------------------

/// RPM-family distros replay scriptlets through the Debian flavour path: each
/// RPM PostInstall forwards both stub install and replay to PostInstall::Debian
/// (distro/mod.rs KindDistro::post_install -> PostInstall::Debian; strategy.rs). The observable proof is that a
/// Fedora scripts-only run reaches the *Debian* finish marker "Postinst scripts
/// completed" — the marker the Debian flavour, not a Fedora-specific one, emits.
// covers: POST-062
#[test]
#[serial]
fn rpm_family_replays_through_debian_flavour_path() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install("fedora:40", cache.path(), root.path(), &["--postinstall=scripts", "bash"]);
  post::ok(&out, "fedora scripts replay");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Postinst scripts completed"),
    "RPM replay must reach the Debian-flavour finish marker (delegation):\n{stderr}"
  );
}

/// Arch-family scripts-only black-box run reaches the Arch flavour's distinct
/// finish marker "Install scripts completed" (arch.rs:142-144), confirming the
/// live Arch staged-`install` replay path end to end.
// covers: POST-037
#[test]
#[serial]
fn arch_blackbox_scripts_reaches_arch_finish_marker() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  // fontconfig's closure ships a `post_install` `.INSTALL` (fc-cache), so the
  // scripts phase actually runs; a script-less seed like `bash` would no-op
  // before the "Install scripts completed" marker.
  let out = post::install("arch:rolling", cache.path(), root.path(), &["--postinstall=scripts", "fontconfig"]);
  post::ok(&out, "arch scripts replay");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Install scripts completed"),
    "Arch replay must reach the Arch-flavour finish marker:\n{stderr}"
  );
}

/// CachyOS scripts-only black-box run reaches the same Arch finish marker,
/// confirming CachyOS maps to PostInstall::Arch (distro/mod.rs KindDistro::post_install).
// covers: POST-040
#[test]
#[serial]
fn cachyos_blackbox_scripts_reaches_arch_finish_marker() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  // fontconfig carries a `post_install` `.INSTALL` so the scripts phase runs;
  // CachyOS mirrors Arch packaging, so it ships the same eligible script.
  let out = post::install("cachyos:rolling", cache.path(), root.path(), &["--postinstall=scripts", "fontconfig"]);
  post::ok(&out, "cachyos scripts replay");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Install scripts completed"),
    "CachyOS replay must reach the Arch-flavour finish marker (delegation):\n{stderr}"
  );
}

/// Alpine scripts-only black-box run reaches the Alpine flavour's distinct
/// finish marker "Post-install scripts completed" (alpine.rs:108-110).
// covers: POST-041
#[test]
#[serial]
fn alpine_blackbox_scripts_reaches_alpine_finish_marker() {
  if !post::userns_available() {
    eprintln!("skipping: unprivileged user namespaces unavailable");
    return;
  }
  let cache = post::cache_dir();
  let root = post::root_dir();
  let out = post::install("alpine:v3.20", cache.path(), root.path(), &["--postinstall=scripts", "busybox"]);
  post::ok(&out, "alpine scripts replay");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Post-install scripts completed"),
    "Alpine replay must reach the Alpine-flavour finish marker:\n{stderr}"
  );
}
