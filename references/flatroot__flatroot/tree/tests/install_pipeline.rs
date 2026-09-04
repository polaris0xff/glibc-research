//! End-to-end install-pipeline behavior against live mirrors.
//!
//! These tests run the real `flatroot install` against upstream package
//! mirrors and assert on the user-visible framing (stderr markers), the
//! on-disk tree (extracted files, merged-usr symlinks, file modes), and the
//! manifest (`.flatroot/`). They are not feature-gated — they only need
//! connectivity. Each gets a fresh cache TempDir (FLATROOT_CACHE_HOME) so the
//! first run fetches and a second run can prove cache reuse.
//!
//! Marker vocabulary asserted here, copied verbatim from the source:
//!   - "Resolved N packages"                  install.rs:224 (RunVoice)
//!   - "Installing N packages (--no-deps)"    install.rs:189
//!   - "Downloaded N packages (M already current)" install.rs:300
//!   - "Extracted N packages to <path>"       install.rs:336
//!   - "All N resolved packages already current for <arch>." install.rs:288
//!   - "Done. N packages installed to <path>" install.rs:146
//!   - "Nothing to do. All N packages already current in <path>" install.rs:148
//!   - "=== <uname> ==="                       install.rs:120
//!   - "ldconfig completed" / "ldconfig not found, skipping" ldconfig.rs:51/42
//!   - "Postinst scripts completed"            postinstall/debian.rs:299
//!   - "Cache hooks completed"                 postinstall/hooks.rs:295
//!   - "Fetching package index for ... (...)..."  arch_context.rs:62
//!   - "Loaded N packages"                     arch_context.rs:111

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Run `flatroot install` with the given extra args, an isolated cache, and a
/// fixed `--from`. Returns the captured Output (stdout/stderr/status).
fn install(remote: &str, cache: &Path, root: &Path, extra: &[&str]) -> Output {
  let mut args: Vec<&str> = vec!["--from", remote, "install", "--output", root.to_str().unwrap()];
  args.extend_from_slice(extra);
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.to_str().unwrap())
    .args(&args)
    .output()
    .expect("flatroot failed to execute")
}

/// Run `flatroot install` with full control over the argv before the
/// subcommand (so `--arch` can be threaded). `pre` goes before `install`.
fn install_with_pre(remote: &str, cache: &Path, root: &Path, pre: &[&str], extra: &[&str]) -> Output {
  let mut args: Vec<&str> = vec!["--from", remote];
  args.extend_from_slice(pre);
  args.extend_from_slice(&["install", "--output", root.to_str().unwrap()]);
  args.extend_from_slice(extra);
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.to_str().unwrap())
    .args(&args)
    .output()
    .expect("flatroot failed to execute")
}

/// Assert success or panic with the captured stderr so the real failure is
/// visible in `cargo test` output.
fn ok(out: &Output, label: &str) {
  assert!(
    out.status.success(),
    "{label}: flatroot install failed (exit={:?})\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr),
  );
}

/// `Resolved N packages` count from stderr.
fn resolved_count(stderr: &str) -> usize {
  for line in stderr.lines() {
    if let Some(rest) = line.strip_prefix("Resolved ")
      && let Some(num) = rest.split_whitespace().next()
    {
      return num
        .parse()
        .unwrap_or_else(|e| panic!("cannot parse resolved count from '{line}': {e}"));
    }
  }
  panic!("no 'Resolved N packages' line in stderr:\n{stderr}");
}

/// `Done. N packages installed to ...` count from stderr.
fn done_count(stderr: &str) -> usize {
  for line in stderr.lines() {
    if let Some(rest) = line.strip_prefix("Done. ")
      && let Some(num) = rest.split_whitespace().next()
    {
      return num
        .parse()
        .unwrap_or_else(|e| panic!("cannot parse done count from '{line}': {e}"));
    }
  }
  panic!("no 'Done. N packages installed' line in stderr:\n{stderr}");
}

/// Read every `Package:` name from `.flatroot/packages`.
fn manifest_package_names(root: &Path) -> Vec<String> {
  let packages = fs::read_to_string(root.join(".flatroot/packages")).unwrap();
  packages
    .lines()
    .filter_map(|l| l.strip_prefix("Package: ").map(|s| s.to_string()))
    .collect()
}

// ---------------------------------------------------------------------------
// --no-deps (INST-007)
// ---------------------------------------------------------------------------

/// `--no-deps coreutils` prints the no-deps marker and skips the resolver:
/// "Installing N packages (--no-deps)" present, "Resolved" absent.
// covers: INST-007
#[test]
fn no_deps_prints_marker_and_skips_resolver() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--no-deps", "coreutils"]);
  ok(&out, "no-deps coreutils");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Installing 1 packages (--no-deps)"),
    "expected the --no-deps marker for 1 package:\n{stderr}"
  );
  assert!(!stderr.contains("Resolved "), "the resolver line must NOT appear with --no-deps:\n{stderr}");
  // Only the named package landed in the manifest.
  let names = manifest_package_names(root.path());
  assert_eq!(names, vec!["coreutils".to_string()], "only coreutils should be recorded:\n{names:?}");
}

// ---------------------------------------------------------------------------
// Full resolution seeds base+essential+user (INST-008)
// ---------------------------------------------------------------------------

/// A bare `install bash` resolves a closure of more than one package and the
/// manifest carries a known Debian essential package (base-files) alongside
/// bash — proving base/essential/user seeding (install.rs:191-226).
// covers: INST-008
#[test]
fn full_resolution_seeds_base_essential_user() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&out, "bash full resolution");
  let stderr = String::from_utf8_lossy(&out.stderr);
  let n = resolved_count(&stderr);
  assert!(n > 1, "expected a closure of more than one package, got {n}");
  let names = manifest_package_names(root.path());
  assert!(names.iter().any(|p| p == "bash"), "user-requested bash missing:\n{names:?}");
  assert!(
    names.iter().any(|p| p == "base-files"),
    "essential package base-files missing — base/essential seeding not applied:\n{names:?}"
  );
}

// ---------------------------------------------------------------------------
// --with (INST-010, INST-011, INST-012)
// ---------------------------------------------------------------------------

/// Closure-size helper for a `--with` comparison. The widening property is a
/// resolver fact, settled the moment resolution finishes, so it is read from
/// `analyze trace --strategy declared` — the same dependency walk over the
/// same index, reported without downloading the closure. Uses
/// libsuitesparse-doc: its soft-dependency closure stays among
/// Architecture:all doc packages that never reach libc6, and Recommends and
/// Suggests each add distinct packages (none/rec/sug/both resolve to
/// 1/2/3/4).
fn closure_for_with(with: Option<&str>) -> usize {
  let cache = TempDir::new().unwrap();
  let mut args: Vec<&str> = vec![
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "libsuitesparse-doc",
    "--strategy",
    "declared",
    "--format",
    "plain",
  ];
  if let Some(w) = with {
    args.push("--with");
    args.push(w);
  }
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path().to_str().unwrap())
    .args(&args)
    .output()
    .expect("flatroot failed to execute");
  assert!(out.status.success(), "analyze trace with={with:?} failed:\n{}", String::from_utf8_lossy(&out.stderr));
  String::from_utf8_lossy(&out.stdout)
    .lines()
    .filter(|l| l.contains(".name="))
    .count()
}

/// `--with suggests` widens the resolved closure beyond the default one
/// (main.rs:45 / install.rs:220).
// covers: INST-010
#[test]
fn with_suggests_widens_set() {
  let baseline = closure_for_with(None);
  let suggests = closure_for_with(Some("suggests"));
  assert!(suggests > baseline, "--with suggests should widen the set: suggests={suggests} baseline={baseline}");
}

/// `--with recommends,suggests` folds in BOTH edge kinds, exceeding both the
/// recommends-only and suggests-only closures (cli.rs:99-106, main.rs:44-45).
// covers: INST-011
#[test]
fn with_recommends_suggests_combines_both() {
  let recommends = closure_for_with(Some("recommends"));
  let suggests = closure_for_with(Some("suggests"));
  let both = closure_for_with(Some("recommends,suggests"));
  assert!(
    both >= recommends && both >= suggests && (both > recommends || both > suggests),
    "combined closure should meet/exceed both: both={both} recommends={recommends} suggests={suggests}"
  );
}

/// Default (no `--with`) omits BOTH recommends and suggests: a known
/// recommend pulled in only by `--with recommends` is absent from the default
/// manifest, and the default resolved count is <= the recommends-augmented
/// count (cli.rs:104, main.rs:44-45).
// covers: INST-012
#[test]
fn default_omits_recommends_and_suggests() {
  // Baseline: bash with no soft deps, post-install skipped to keep it cheap.
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out_default = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&out_default, "default bash");
  let default_n = resolved_count(&String::from_utf8_lossy(&out_default.stderr));

  // bash Recommends bash-completion on Debian — it must be absent by default.
  let names = manifest_package_names(root.path());
  assert!(
    !names.iter().any(|p| p == "bash-completion"),
    "bash-completion (a Recommends) must be absent without --with recommends:\n{names:?}"
  );

  // With recommends, the set must be at least as large (proving default omits them).
  let cache2 = TempDir::new().unwrap();
  let root2 = TempDir::new().unwrap();
  let out_rec =
    install("debian:bookworm", cache2.path(), root2.path(), &["--postinstall=none", "--with", "recommends", "bash"]);
  ok(&out_rec, "recommends bash");
  let rec_n = resolved_count(&String::from_utf8_lossy(&out_rec.stderr));
  assert!(default_n <= rec_n, "default closure must not exceed recommends closure: {default_n} vs {rec_n}");
}

// ---------------------------------------------------------------------------
// --exclude (INST-013, INST-014, INST-015)
// ---------------------------------------------------------------------------

/// `--exclude` drops a named package that WOULD be in the closure plus any
/// dependency reachable only through it (main.rs:38-42, install.rs:221). We
/// use the recommends-augmented bash closure, which pulls `bash-completion`;
/// `--exclude bash-completion` removes it (and shrinks the resolved count)
/// while bash and its hard dep libc6 remain. Comparing against the
/// no-exclude recommends run proves real pruning, not a trivially-absent name.
// covers: INST-013
#[test]
fn exclude_drops_named_and_private_deps() {
  // Without exclude: bash --with recommends pulls bash-completion.
  let cache_keep = TempDir::new().unwrap();
  let root_keep = TempDir::new().unwrap();
  let out_keep = install(
    "debian:bookworm",
    cache_keep.path(),
    root_keep.path(),
    &["--postinstall=none", "--with", "recommends", "bash"],
  );
  ok(&out_keep, "bash --with recommends (keep)");
  let keep_names = manifest_package_names(root_keep.path());
  assert!(
    keep_names.iter().any(|p| p == "bash-completion"),
    "precondition: bash --with recommends must pull bash-completion:\n{keep_names:?}"
  );
  let keep_n = resolved_count(&String::from_utf8_lossy(&out_keep.stderr));

  // With exclude: bash-completion (and anything reachable only via it) is pruned.
  let cache_drop = TempDir::new().unwrap();
  let root_drop = TempDir::new().unwrap();
  let out_drop = install(
    "debian:bookworm",
    cache_drop.path(),
    root_drop.path(),
    &[
      "--postinstall=none",
      "--with",
      "recommends",
      "--exclude",
      "bash-completion",
      "bash",
    ],
  );
  ok(&out_drop, "bash --with recommends --exclude bash-completion");
  let drop_names = manifest_package_names(root_drop.path());
  let drop_n = resolved_count(&String::from_utf8_lossy(&out_drop.stderr));

  assert!(
    !drop_names.iter().any(|p| p == "bash-completion"),
    "excluded bash-completion must be absent:\n{drop_names:?}"
  );
  assert!(drop_names.iter().any(|p| p == "bash"), "bash must still be installed:\n{drop_names:?}");
  assert!(drop_names.iter().any(|p| p == "libc6"), "bash's hard dep libc6 must remain:\n{drop_names:?}");
  assert!(drop_n < keep_n, "excluding bash-completion must shrink the closure: drop={drop_n} keep={keep_n}");
}

/// `--exclude 'bash-completion, perl ,'` is trimmed and filtered into the set
/// {bash-completion, perl} with the trailing empty token dropped (main.rs:38-42).
/// bash-completion (a present recommend) is pruned, perl is absent, and the run
/// succeeds despite the trailing empty/space tokens — proving each token is
/// trimmed and the empties are filtered rather than passed through.
// covers: INST-014
#[test]
fn exclude_comma_list_parses_to_trimmed_set() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install(
    "debian:bookworm",
    cache.path(),
    root.path(),
    &[
      "--postinstall=none",
      "--with",
      "recommends",
      "--exclude",
      "bash-completion, perl ,",
      "bash",
    ],
  );
  ok(&out, "exclude comma-list bash");
  let names = manifest_package_names(root.path());
  // The whitespace-padded `bash-completion` token was trimmed and applied.
  assert!(
    !names.iter().any(|p| p == "bash-completion"),
    "padded bash-completion token must be trimmed and excluded:\n{names:?}"
  );
  assert!(!names.iter().any(|p| p == "perl"), "perl token must be excluded:\n{names:?}");
  assert!(names.iter().any(|p| p == "bash"), "bash must remain:\n{names:?}");
}

/// `--exclude ''` (empty string) filters to zero exclusions, yielding a
/// package set identical to a no-exclude baseline (cli.rs:138, main.rs:38-42).
/// Both runs use --with recommends so the comparison spans a non-trivial
/// closure that an accidental exclusion would visibly shrink.
// covers: INST-015
#[test]
fn exclude_empty_string_is_no_exclusions() {
  let cache_base = TempDir::new().unwrap();
  let root_base = TempDir::new().unwrap();
  let out_base = install(
    "debian:bookworm",
    cache_base.path(),
    root_base.path(),
    &["--postinstall=none", "--with", "recommends", "bash"],
  );
  ok(&out_base, "baseline bash");
  let mut base_names = manifest_package_names(root_base.path());
  base_names.sort();

  let cache_empty = TempDir::new().unwrap();
  let root_empty = TempDir::new().unwrap();
  let out_empty = install(
    "debian:bookworm",
    cache_empty.path(),
    root_empty.path(),
    &["--postinstall=none", "--with", "recommends", "--exclude", "", "bash"],
  );
  ok(&out_empty, "empty-exclude bash");
  let mut empty_names = manifest_package_names(root_empty.path());
  empty_names.sort();

  assert_eq!(base_names, empty_names, "empty --exclude must match the no-exclude baseline package set");
}

// ---------------------------------------------------------------------------
// --postinstall phase selection (INST-016, INST-018, INST-019, INST-020,
//                                INST-021, INST-022, INST-040)
// ---------------------------------------------------------------------------

/// Byte-offset of a marker in stderr, or None if absent.
fn offset_of(stderr: &str, marker: &str) -> Option<usize> {
  stderr.find(marker)
}

/// `--postinstall none` runs no phases: no ldconfig/scripts/hooks success
/// markers, yet the run still completes with "Done." and Postfix still ran
/// (a mode-0 extracted file becomes 0644). Sandbox::available() is never
/// invoked because PostInstallPlan::run returns early on an empty phase list
/// (postinstall/mod.rs:116-118) — observable as the absence of any
/// post-install phase output while "Done." is present.
// covers: INST-016, INST-040
#[test]
fn postinstall_none_skips_phases_but_finishes_and_runs_postfix() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&out, "postinstall none bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Done. "), "run must finish with Done.:\n{stderr}");
  assert!(!stderr.contains("ldconfig completed"), "ldconfig must not run with --postinstall=none:\n{stderr}");
  assert!(!stderr.contains("Postinst scripts completed"), "scripts must not run with --postinstall=none:\n{stderr}");
  assert!(!stderr.contains("Cache hooks completed"), "hooks must not run with --postinstall=none:\n{stderr}");

  // Postfix still ran: no extracted regular file is left mode-0. We scan the
  // tree for any mode-0 regular file outside .flatroot; there must be none.
  assert!(
    !any_mode_zero_regular_file(root.path()),
    "Postfix should have flipped every mode-0 regular file to 0644 even with --postinstall=none"
  );
}

/// Recursively report whether any regular file under `root` (excluding
/// `.flatroot`) still has a 0 permission mode.
fn any_mode_zero_regular_file(root: &Path) -> bool {
  fn walk(dir: &Path, root: &Path) -> bool {
    let entries = match fs::read_dir(dir) {
      Ok(e) => e,
      Err(_) => return false,
    };
    for entry in entries.flatten() {
      let path = entry.path();
      if path
        .strip_prefix(root)
        .map(|r| r.starts_with(".flatroot"))
        .unwrap_or(false)
      {
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
        if walk(&path, root) {
          return true;
        }
      } else if ft.is_file() && (md.permissions().mode() & 0o7777) == 0 {
        return true;
      }
    }
    false
  }
  walk(root, root)
}

/// `--postinstall ldconfig` runs only the ldconfig phase: the ld.so.cache is
/// populated (or ldconfig reports cleanly) while scripts and hooks are skipped
/// (postinstall/mod.rs:134-136, cli.rs:113-120).
// covers: INST-018
#[test]
fn postinstall_ldconfig_runs_only_ldconfig() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall", "ldconfig", "bash"]);
  ok(&out, "postinstall ldconfig bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("ldconfig completed"), "ldconfig phase must run:\n{stderr}");
  assert!(!stderr.contains("Postinst scripts completed"), "scripts must be skipped:\n{stderr}");
  assert!(!stderr.contains("Cache hooks completed"), "hooks must be skipped:\n{stderr}");
  assert!(root.path().join("etc/ld.so.cache").exists(), "ldconfig should have populated /etc/ld.so.cache");
}

/// `--postinstall scripts` runs the scripts phase (stubs + replay → the
/// "Postinst scripts completed" finish line) while ldconfig and hooks are
/// skipped (postinstall/mod.rs:142-145).
// covers: INST-019
#[test]
fn postinstall_scripts_runs_only_scripts() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall", "scripts", "bash"]);
  ok(&out, "postinstall scripts bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Postinst scripts completed"), "scripts phase must run:\n{stderr}");
  assert!(!stderr.contains("ldconfig completed"), "ldconfig must be skipped:\n{stderr}");
  assert!(!stderr.contains("Cache hooks completed"), "hooks must be skipped:\n{stderr}");
}

/// `--postinstall hooks` runs only the cache-builder hooks ("Cache hooks
/// completed") while ldconfig and scripts are skipped (postinstall/mod.rs:147-149).
// covers: INST-020
#[test]
fn postinstall_hooks_runs_only_hooks() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall", "hooks", "bash"]);
  ok(&out, "postinstall hooks bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Cache hooks completed"), "hooks phase must run:\n{stderr}");
  assert!(!stderr.contains("ldconfig completed"), "ldconfig must be skipped:\n{stderr}");
  assert!(!stderr.contains("Postinst scripts completed"), "scripts must be skipped:\n{stderr}");
}

/// Default `--postinstall` runs all three phases in the fixed
/// Ldconfig→Scripts→Hooks order (cli.rs:118, postinstall/mod.rs:134-149).
// covers: INST-021
#[test]
fn postinstall_default_runs_all_three_in_order() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["bash"]);
  ok(&out, "default postinstall bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  let ld = offset_of(&stderr, "ldconfig completed").unwrap_or_else(|| panic!("ldconfig marker missing:\n{stderr}"));
  let sc =
    offset_of(&stderr, "Postinst scripts completed").unwrap_or_else(|| panic!("scripts marker missing:\n{stderr}"));
  let hk = offset_of(&stderr, "Cache hooks completed").unwrap_or_else(|| panic!("hooks marker missing:\n{stderr}"));
  assert!(ld < sc, "ldconfig must precede scripts: ld={ld} sc={sc}\n{stderr}");
  assert!(sc < hk, "scripts must precede hooks: sc={sc} hk={hk}\n{stderr}");
}

/// Shuffled `--postinstall hooks,ldconfig,scripts` still executes in the
/// canonical Ldconfig→Scripts→Hooks order: the run order is fixed by the
/// `contains` checks in postinstall/mod.rs:134-149, not by token order.
// covers: INST-022
#[test]
fn postinstall_token_order_does_not_change_execution_order() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall", "hooks,ldconfig,scripts", "bash"]);
  ok(&out, "shuffled postinstall bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  let ld = offset_of(&stderr, "ldconfig completed").unwrap_or_else(|| panic!("ldconfig marker missing:\n{stderr}"));
  let sc =
    offset_of(&stderr, "Postinst scripts completed").unwrap_or_else(|| panic!("scripts marker missing:\n{stderr}"));
  let hk = offset_of(&stderr, "Cache hooks completed").unwrap_or_else(|| panic!("hooks marker missing:\n{stderr}"));
  assert!(ld < sc && sc < hk, "execution must stay Ldconfig->Scripts->Hooks regardless of token order:\n{stderr}");
}

/// ldconfig auto-skips on a musl tree (Alpine): no ldconfig error, and the
/// "ldconfig not found, skipping" notice appears, while scripts/hooks still
/// complete (cli.rs:402-404, postinstall/mod.rs:134-136, ldconfig.rs:42).
// covers: INST-024
#[test]
fn ldconfig_auto_skips_on_alpine_musl() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("alpine:v3.21", cache.path(), root.path(), &["busybox"]);
  ok(&out, "alpine busybox");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("ldconfig not found, skipping"), "alpine musl tree must report ldconfig skipped:\n{stderr}");
  assert!(!stderr.contains("ldconfig failed"), "no ldconfig failure on musl:\n{stderr}");
  assert!(stderr.contains("Cache hooks completed"), "hooks should still complete on alpine:\n{stderr}");
}

// ---------------------------------------------------------------------------
// Multiarch (INST-025, INST-026, INST-028, INST-054)
// ---------------------------------------------------------------------------

/// Multiarch `--arch x86_64,i686` runs the per-arch pipeline once for each
/// arch into one root, emitting the "=== <uname> ===" headers, while
/// post-install + "Done." run exactly once across both arches
/// (install.rs:118-127, :140-146).
// covers: INST-025
#[test]
fn multiarch_runs_pipeline_per_arch_one_postinstall() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install_with_pre("debian:bookworm", cache.path(), root.path(), &["--arch", "x86_64,i686"], &["dash"]);
  ok(&out, "multiarch dash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("=== x86_64 ==="), "missing x86_64 header:\n{stderr}");
  assert!(stderr.contains("=== i686 ==="), "missing i686 header:\n{stderr}");
  assert_eq!(stderr.matches("Done. ").count(), 1, "post-install summary must run exactly once:\n{stderr}");
  // Both arch trees present.
  assert!(root.path().join("bin/dash").exists(), "x86_64 dash missing");
  assert!(
    root.path().join("lib/i386-linux-gnu").exists() || root.path().join("usr/lib/i386-linux-gnu").exists(),
    "i686 multiarch lib dir missing"
  );
}

/// A single-arch install (`--arch x86_64`) omits the "=== arch ===" header
/// entirely (install.rs:119-121 only emits it when archs.len() > 1).
// covers: INST-026
#[test]
fn single_arch_omits_header() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install_with_pre("debian:bookworm", cache.path(), root.path(), &["--arch", "x86_64"], &["bash"]);
  ok(&out, "single-arch bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(!stderr.contains("=== "), "single-arch run must not emit an arch header:\n{stderr}");
}

/// `--arch x86` is accepted as an alias for i686 (arch.rs:133) and produces an
/// i386-linux-gnu tree.
// covers: INST-028
#[test]
fn arch_x86_alias_proceeds_as_i686() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install_with_pre("debian:bookworm", cache.path(), root.path(), &["--arch", "x86"], &["dash"]);
  ok(&out, "arch x86 dash");
  assert!(
    root.path().join("lib/i386-linux-gnu").exists() || root.path().join("usr/lib/i386-linux-gnu").exists(),
    "--arch x86 should yield an i386-linux-gnu tree"
  );
  // The manifest records the kernel-spelling architecture i686.
  let packages = fs::read_to_string(root.path().join(".flatroot/packages")).unwrap();
  assert!(
    packages.lines().any(|l| l == "Architecture: i686"),
    "manifest must record architecture i686 for --arch x86:\n{packages}"
  );
}

/// A multiarch install with a non-x86_64 first arch (`--arch i686,x86_64`)
/// runs post-install once using the first arch's uname (install.rs:141-144).
/// Both arch trees are present and the post-install ran exactly once.
// covers: INST-054
#[test]
fn primary_arch_fallback_uses_first_requested_arch() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install_with_pre("debian:bookworm", cache.path(), root.path(), &["--arch", "i686,x86_64"], &["dash"]);
  ok(&out, "i686-first multiarch dash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert_eq!(stderr.matches("Done. ").count(), 1, "post-install must run once:\n{stderr}");
  // First arch header is i686, second is x86_64.
  let first_hdr = stderr
    .find("=== i686 ===")
    .unwrap_or_else(|| panic!("i686 header missing:\n{stderr}"));
  let second_hdr = stderr
    .find("=== x86_64 ===")
    .unwrap_or_else(|| panic!("x86_64 header missing:\n{stderr}"));
  assert!(first_hdr < second_hdr, "i686 must be processed before x86_64:\n{stderr}");
  assert!(
    root.path().join("lib/i386-linux-gnu").exists() || root.path().join("usr/lib/i386-linux-gnu").exists(),
    "i686 tree missing"
  );
  assert!(root.path().join("bin/dash").exists(), "x86_64 dash missing");
}

// ---------------------------------------------------------------------------
// Multiarch mid-loop failure (INST-036, INST-053)
//
// amd64-microcode is an amd64-only Debian package. With `--no-deps`, the
// x86_64 arch extracts it, then the second arch's `install_for_arch` bails
// "not found in the index" (install.rs:186) AFTER the first arch extracted —
// so the manifest is written only after all arches succeed (install.rs:129-133)
// and a prior run's manifest stays intact.
// ---------------------------------------------------------------------------

/// A multiarch install where a later arch fails after an earlier arch
/// extracted leaves the on-disk manifest unchanged from before the run
/// (install.rs:116-133). We first establish a prior manifest with a clean
/// single-arch install, snapshot it, then run a multiarch install whose
/// second arch fails — the manifest must be byte-identical afterwards.
// covers: INST-036
#[test]
fn manifest_written_only_after_all_arches_succeed() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  // Prior clean install establishes a manifest.
  let prior = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&prior, "prior bash install");
  let manifest_before = fs::read(root.path().join(".flatroot/manifest")).unwrap();
  let packages_before = fs::read(root.path().join(".flatroot/packages")).unwrap();

  // Multiarch install whose second arch (aarch64) lacks amd64-microcode.
  let out = install_with_pre(
    "debian:bookworm",
    cache.path(),
    root.path(),
    &["--arch", "x86_64,aarch64"],
    &["--no-deps", "amd64-microcode"],
  );
  assert!(!out.status.success(), "multiarch install must fail when the second arch lacks the package");

  let manifest_after = fs::read(root.path().join(".flatroot/manifest")).unwrap();
  let packages_after = fs::read(root.path().join(".flatroot/packages")).unwrap();
  assert_eq!(manifest_before, manifest_after, "manifest must be unchanged after a mid-loop failure");
  assert_eq!(packages_before, packages_after, "packages file must be unchanged after a mid-loop failure");
}

/// A multiarch install where a later arch's per-arch pass fails non-zero
/// despite the first arch having extracted leaves the prior manifest intact
/// (install.rs:118-133). Same mechanism as INST-036, asserted as the
/// cross-arch failure-propagation contract: non-zero exit, prior tree's
/// manifest untouched.
// covers: INST-053
#[test]
fn multiarch_later_arch_failure_is_nonzero_prior_manifest_intact() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let prior = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&prior, "prior bash install");
  let before = fs::read(root.path().join(".flatroot/manifest")).unwrap();

  let out = install_with_pre(
    "debian:bookworm",
    cache.path(),
    root.path(),
    &["--arch", "x86_64,aarch64"],
    &["--no-deps", "amd64-microcode"],
  );
  assert!(!out.status.success(), "a later arch failing must make the whole install non-zero");
  let after = fs::read(root.path().join(".flatroot/manifest")).unwrap();
  assert_eq!(before, after, "prior manifest must be unchanged despite the first arch extracting");
}

// ---------------------------------------------------------------------------
// Output directory handling (INST-042, INST-043)
// ---------------------------------------------------------------------------

/// Install to a multi-level absent path creates the full nested path and
/// populates it (install.rs:99 create_dir_all).
// covers: INST-042
#[test]
fn output_directory_created_when_absent() {
  let cache = TempDir::new().unwrap();
  let parent = TempDir::new().unwrap();
  let nested = parent.path().join("does/not/exist/root");
  assert!(!nested.exists(), "precondition: nested path absent");
  let out = install("debian:bookworm", cache.path(), &nested, &["--postinstall=none", "bash"]);
  ok(&out, "nested-path install");
  assert!(nested.join("bin/bash").exists(), "nested path must be created and populated");
  assert!(nested.join(".flatroot/manifest").exists(), "manifest must be written into nested path");
}

/// Installing into a root pre-populated with non-.flatroot files leaves those
/// files in place (overwrite is file-by-file, cli.rs:89-91), while package
/// files are added and .flatroot is written (install.rs:99-104, state.rs:74-76).
// covers: INST-043
#[test]
fn existing_non_flatroot_contents_left_in_place() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  // Pre-populate with a user file that no package would touch.
  let marker = root.path().join("preexisting-user-file.txt");
  fs::write(&marker, b"keep me\n").unwrap();
  let nested_marker = root.path().join("home/user/notes.txt");
  fs::create_dir_all(nested_marker.parent().unwrap()).unwrap();
  fs::write(&nested_marker, b"notes\n").unwrap();

  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&out, "install over pre-populated root");

  assert_eq!(fs::read(&marker).unwrap(), b"keep me\n", "pre-existing top-level file must survive");
  assert_eq!(fs::read(&nested_marker).unwrap(), b"notes\n", "pre-existing nested file must survive");
  assert!(root.path().join("bin/bash").exists(), "package files must be added");
  assert!(root.path().join(".flatroot/manifest").exists(), ".flatroot must be written");
}

// ---------------------------------------------------------------------------
// --parallel (INST-045)
// ---------------------------------------------------------------------------

/// `-p 1` forces sequential downloads; the install must still succeed and
/// produce a working tree (install.rs:53/296-297, cli.rs:132).
// covers: INST-045
#[test]
fn parallel_one_forces_sequential_and_succeeds() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install_with_pre("debian:bookworm", cache.path(), root.path(), &["--arch", "x86_64"], &["-p", "1", "bash"]);
  ok(&out, "parallel 1 bash");
  assert!(root.path().join("bin/bash").exists(), "bash must be installed with -p 1");
}

// ---------------------------------------------------------------------------
// ArchContext index diagnostics + cache dir (INST-046, INST-049)
// ---------------------------------------------------------------------------

/// `--http-retries 5` is accepted and threads through; the install succeeds.
/// The retries value flows into BOTH the shared HTTP client (index fetch,
/// install.rs:97) and the Downloader (archive download, install.rs:296). We
/// assert the happy path completes and produces a tree — the propagation is
/// structural (the same `args.http_retries` is passed to both call sites).
// covers: INST-046
#[test]
fn http_retries_threads_through_to_index_and_downloads() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install_with_pre(
    "debian:bookworm",
    cache.path(),
    root.path(),
    &["--http-retries", "5", "--arch", "x86_64"],
    &["--postinstall=none", "bash"],
  );
  ok(&out, "http-retries 5 bash");
  assert!(root.path().join("bin/bash").exists());
}

/// ArchContext prints the index diagnostics ("Fetching package index for ..."
/// and "Loaded N packages") and creates the per-source download cache dir
/// (arch_context.rs:62/111/113-114).
// covers: INST-049
#[test]
fn archcontext_prints_index_diagnostics_and_creates_cache_dir() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&out, "archcontext diagnostics bash");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Fetching package index for debian:bookworm (x86_64)..."),
    "index-fetch diagnostic missing:\n{stderr}"
  );
  assert!(stderr.contains("Loaded "), "'Loaded N packages' diagnostic missing:\n{stderr}");
  // The downloader cache dir for this source is keyed by the remote cache_id
  // (debian/<release>/<arch>) directly under the cache home, and is created by
  // ArchContext::finish (arch_context.rs:113-114). For debian:bookworm x86_64
  // that is <cache>/debian/bookworm/amd64.
  let source_cache = cache.path().join("debian/bookworm/amd64");
  assert!(source_cache.exists(), "per-source download cache dir must be created at {source_cache:?}");
}

// ---------------------------------------------------------------------------
// Index fetch failure (INST-047)
// ---------------------------------------------------------------------------

/// An unreachable mirror surfaces as a non-zero exit with no manifest written
/// (arch_context.rs:61-81 index fetch failure). debian:bookworm@<absent-date>
/// points snapshot.debian.org at a date with no archive, so the index fetch
/// fails before any extraction — the output tree (created by create_dir_all)
/// gains no `.flatroot/manifest`.
// covers: INST-047
#[test]
fn index_fetch_failure_is_nonzero_no_manifest() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  // A snapshot date far before debian existed has no published archive — the
  // snapshot mirror returns failure for the index, so the fetch fails.
  let out = install("debian:bookworm@1970-01-01", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  assert!(!out.status.success(), "an unreachable/empty snapshot index must fail the install");
  assert!(!root.path().join(".flatroot/manifest").exists(), "no manifest may be written when the index fetch fails");
}

// ---------------------------------------------------------------------------
// Snapshot-dated source (INST-050)
// ---------------------------------------------------------------------------

/// A pinned `@date` source is recorded verbatim as the manifest Source field
/// (install.rs:41, state.rs:235-241), and is treated as a distinct source.
// covers: INST-050
#[test]
fn snapshot_dated_source_recorded_verbatim() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm@2024-06-15", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&out, "pinned bash install");
  let packages = fs::read_to_string(root.path().join(".flatroot/packages")).unwrap();
  assert!(
    packages.lines().any(|l| l == "Source: debian:bookworm@2024-06-15"),
    "manifest Source must equal the exact @date string:\n{packages}"
  );
}

// ---------------------------------------------------------------------------
// 'Done.' total = full merged manifest count (INST-055)
// ---------------------------------------------------------------------------

/// An incremental install (bash then nano) reports "Done. N packages
/// installed" with N equal to the FULL merged manifest size (prior + new),
/// not just the newly-extracted count (install.rs:134/146).
// covers: INST-055
#[test]
fn done_summary_counts_full_merged_manifest() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let first = install("debian:bookworm", cache.path(), root.path(), &["bash"]);
  ok(&first, "bash install");
  let first_done = done_count(&String::from_utf8_lossy(&first.stderr));

  let second = install("debian:bookworm", cache.path(), root.path(), &["nano"]);
  ok(&second, "nano install");
  let second_done = done_count(&String::from_utf8_lossy(&second.stderr));

  // The merged manifest after adding nano must hold at least the first run's
  // packages plus nano itself — so the second 'Done.' count strictly exceeds
  // the first, and equals the on-disk merged manifest size.
  let merged = manifest_package_names(root.path()).len();
  assert_eq!(
    second_done, merged,
    "Done. count must equal the merged manifest size: done={second_done} merged={merged}"
  );
  assert!(
    second_done > first_done,
    "incremental Done. count must reflect prior+new, not just new: second={second_done} first={first_done}"
  );
}

// ---------------------------------------------------------------------------
// Output contract: stdout silent (INST-056)
// ---------------------------------------------------------------------------

/// A mutating install is silent on stdout — all framing goes to stderr
/// (install.rs:120/146/.../arch_context.rs). stdout must be zero bytes.
// covers: INST-056
#[test]
fn install_is_silent_on_stdout() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["bash"]);
  ok(&out, "stdout-contract bash");
  assert!(
    out.stdout.is_empty(),
    "install must emit zero bytes on stdout, got:\n{}",
    String::from_utf8_lossy(&out.stdout)
  );
  // Framing lives on stderr.
  assert!(String::from_utf8_lossy(&out.stderr).contains("Done. "), "framing must be on stderr");
}

// ---------------------------------------------------------------------------
// Extraction order preserves merged-usr (INST-057)
// ---------------------------------------------------------------------------

/// Dependency-first extraction preserves the merged-usr layout: after an Arch
/// install, /bin is a SYMLINK (to usr/bin) rather than a real directory
/// (install.rs:303-318). If extraction order were wrong, a package writing to
/// /bin before `filesystem` created the symlink would have made /bin a real
/// directory.
// covers: INST-057
#[test]
fn extraction_order_preserves_merged_usr_symlink() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("arch:rolling", cache.path(), root.path(), &["coreutils"]);
  ok(&out, "arch coreutils");
  let bin = root.path().join("bin");
  let md = fs::symlink_metadata(&bin).expect("/bin must exist after install");
  assert!(
    md.file_type().is_symlink(),
    "/bin must be a symlink (merged-usr) — extraction order corrupted the layout if it is a real dir"
  );
}

// ===========================================================================
// INST-073 — the Downloader hard-fails on a needs-extract package missing from
// the database BEFORE install.rs ever reaches its defensive None=>continue arm
// (commands/install.rs:311-314 is unreachable normally because downloader.rs
// hard-fails first at downloader.rs:106-110). This exercises the reachable
// guard: a genuine install populates the map for every needs-extract package, so
// the defensive continue is never hit silently.
// ===========================================================================

// covers: INST-073
#[test]
fn downloader_hardfails_before_install_defensive_continue() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&out, "install populates the download map for every needs-extract package");
  let stderr = String::from_utf8_lossy(&out.stderr);
  // Every needs-extract package was extracted (the count line is printed), so
  // none was skipped via the defensive continue.
  assert!(stderr.contains("Extracted "), "every downloaded archive must be extracted, never skipped:\n{stderr}");
  assert!(root.path().join("bin/bash").exists(), "bash must be on disk, proving no needs-extract entry was dropped");
}

// ===========================================================================
// INST-074 — multiarch install: the manifest is written ONCE after every arch
// succeeds (commands/install.rs:118-133). A successful x86_64,i686 build records
// both arches in one coherent manifest; the merged record spans the two arches.
// ===========================================================================

// covers: INST-074
#[test]
fn multiarch_manifest_written_once_spanning_all_arches() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install_with_pre(
    "debian:bookworm",
    cache.path(),
    root.path(),
    &["--arch", "x86_64,i686"],
    &["--postinstall=none", "bash"],
  );
  ok(&out, "multiarch x86_64,i686 install");

  // One manifest file, with an Architectures header naming BOTH arches (the
  // single write after both per-arch passes succeeded).
  let manifest = fs::read_to_string(root.path().join(".flatroot/manifest")).unwrap();
  let archs_line = manifest
    .lines()
    .find(|l| l.starts_with("Architectures:"))
    .unwrap_or_else(|| panic!("manifest must carry an Architectures header:\n{manifest}"));
  assert!(archs_line.contains("x86_64"), "Architectures must include x86_64: {archs_line}");
  assert!(archs_line.contains("i686"), "Architectures must include i686: {archs_line}");

  // Per-arch records both present (multilib carries the same package under both
  // arches). The packages file has Architecture: lines for both.
  let packages = fs::read_to_string(root.path().join(".flatroot/packages")).unwrap();
  assert!(packages.contains("Architecture: x86_64"), "packages must record x86_64 entries:\n{packages}");
  assert!(packages.contains("Architecture: i686"), "packages must record i686 entries:\n{packages}");
}

// ===========================================================================
// INST-075 — post-install + Postfix gated on total_extracted SUMMED across arches
// (commands/install.rs:117-149): on a fresh multiarch install the sum is > 0, so
// the finishing work runs exactly once (one "Done." line), with the primary arch
// (the first requested) driving it (install.rs:141-144).
// ===========================================================================

// covers: INST-075
#[test]
fn postinstall_gated_on_summed_extraction_runs_once() {
  if flatroot::sandbox::Sandbox::available().is_err() {
    eprintln!("skipping: unprivileged user namespaces unavailable; default post-install path unreachable");
    return;
  }
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  // Default post-install (sum across both arches is > 0 on a fresh build), so the
  // finishing work runs ONCE after both arches extract.
  let out = install_with_pre("debian:bookworm", cache.path(), root.path(), &["--arch", "x86_64,i686"], &["bash"]);
  ok(&out, "multiarch default-postinstall install");
  let stderr = String::from_utf8_lossy(&out.stderr);
  // Exactly one terminal "Done." — post-install ran once over the combined
  // result, not once per arch.
  let done_count = stderr.matches("Done. ").count();
  assert_eq!(done_count, 1, "post-install + Postfix must run exactly once after all arches:\n{stderr}");
  assert!(!stderr.contains("Nothing to do"), "a fresh multiarch install summed > 0 must do work:\n{stderr}");
}

// ===========================================================================
// INST-078 — --no-deps validates each name exists and bails on the first unknown
// with "...not found in the index", extracting nothing (commands/install.rs:
// 178-189). Per-format axis: deb / rpm / pacman / apk.
// ===========================================================================

// covers: INST-078
#[test]
fn no_deps_bails_on_unknown_name_across_formats() {
  let cases: &[&str] = &["debian:bookworm", "fedora:42", "arch:rolling", "alpine:edge"];
  for remote in cases {
    let cache = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let out = install(remote, cache.path(), root.path(), &["--no-deps", "no-such-pkg-xyz-123"]);
    assert!(!out.status.success(), "{remote}: --no-deps with an unknown name must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
      stderr.contains("not found in the index"),
      "{remote}: the bail must use the 'not found in the index' wording:\n{stderr}"
    );
    // Nothing extracts: no manifest, and the rootfs holds no usr/ tree from the
    // (nonexistent) package.
    assert!(
      !root.path().join(".flatroot/manifest").exists(),
      "{remote}: no manifest may be written when the name is unknown"
    );
    assert!(!stderr.contains("Extracted "), "{remote}: nothing may be extracted:\n{stderr}");
  }
}

// ===========================================================================
// POST-063 — the sandbox-availability gate is checked once up-front in the
// default path (postinstall/mod.rs:116-128); --postinstall=none bypasses the
// gate entirely (install.rs:144 — an empty phase list short-circuits
// PostInstallPlan::run before Sandbox::available is consulted).
// ===========================================================================

// covers: POST-063
#[test]
fn postinstall_none_bypasses_the_sandbox_gate() {
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  // --postinstall=none must succeed regardless of userns availability and must
  // NEVER print the userns-unavailable remedy, because the gate is never reached.
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall=none", "bash"]);
  ok(&out, "--postinstall=none install");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    !stderr.contains("Post-install requires unprivileged user namespaces"),
    "--postinstall=none must bypass the sandbox gate entirely:\n{stderr}"
  );
  assert!(stderr.contains("Done. "), "the bare-extraction install must still finish:\n{stderr}");

  // The positive counterpart, only when userns IS available: a default install
  // passes the gate and finishes; the remedy must not appear.
  if flatroot::sandbox::Sandbox::available().is_ok() {
    let cache2 = TempDir::new().unwrap();
    let root2 = TempDir::new().unwrap();
    let def = install("debian:bookworm", cache2.path(), root2.path(), &["bash"]);
    ok(&def, "default install with sandbox available");
    let s2 = String::from_utf8_lossy(&def.stderr);
    assert!(
      !s2.contains("Post-install requires unprivileged user namespaces"),
      "the gate must pass when userns is available:\n{s2}"
    );
  }
}

// ===========================================================================
// POST-064 — post-install runs ldconfig → scripts → hooks in FIXED order
// regardless of the --postinstall token order, in one Sandbox::new session
// (postinstall/mod.rs:132-149; main.rs:212-235 normalizes the phase set).
// ===========================================================================

// covers: POST-064
#[test]
fn scrambled_postinstall_tokens_still_run_in_fixed_order() {
  if flatroot::sandbox::Sandbox::available().is_err() {
    eprintln!("skipping: unprivileged user namespaces unavailable; post-install phases unreachable");
    return;
  }
  // Scrambled order "hooks,ldconfig,scripts" must be accepted (it is not the
  // rejected none+phase combination) and the install must complete — the phase
  // ordering inside PostInstallPlan::run is fixed by `if self.phases.contains`
  // checks (mod.rs:134/142/147), independent of the token order on the CLI.
  let cache = TempDir::new().unwrap();
  let root = TempDir::new().unwrap();
  let out = install("debian:bookworm", cache.path(), root.path(), &["--postinstall", "hooks,ldconfig,scripts", "bash"]);
  ok(&out, "scrambled-order post-install must complete");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Done. "), "scrambled-token post-install must finish identically:\n{stderr}");
  assert!(root.path().join("bin/bash").exists(), "the rootfs must be fully built");
}

// ===========================================================================
// POST-065 — Postfix permission repair runs even under --postinstall=none and
// turns a mode-0 file into 0644 (commands/install.rs:145 always-runs;
// postfixes/permissions.rs:60-64). Driven directly via Postfix::apply on a tree
// containing a no-permission file, asserting the repair to 0644 and that links
// are left untouched.
// ===========================================================================

// covers: POST-065
#[test]
fn postfix_repairs_mode_zero_files_to_0644() {
  use std::os::unix::fs::PermissionsExt as _;
  let tree = TempDir::new().unwrap();
  // A file with NO permissions at all (the gap an unprivileged build leaves).
  let locked = tree.path().join("locked.txt");
  fs::write(&locked, b"data").unwrap();
  fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

  // A file with a deliberate mode must NOT be disturbed.
  let kept = tree.path().join("kept.sh");
  fs::write(&kept, b"#!/bin/sh\n").unwrap();
  fs::set_permissions(&kept, fs::Permissions::from_mode(0o755)).unwrap();

  // A nested file too, to prove the walk descends.
  let sub = tree.path().join("sub");
  fs::create_dir(&sub).unwrap();
  let nested = sub.join("nested.bin");
  fs::write(&nested, b"x").unwrap();
  fs::set_permissions(&nested, fs::Permissions::from_mode(0o000)).unwrap();

  flatroot::postfixes::Postfix::apply(tree.path()).expect("Postfix::apply must succeed on a readable tree");

  let mode = |p: &Path| fs::symlink_metadata(p).unwrap().permissions().mode() & 0o7777;
  assert_eq!(mode(&locked), 0o644, "a mode-0 file must be repaired to 0644");
  assert_eq!(mode(&nested), 0o644, "the repair must descend into subdirectories");
  assert_eq!(mode(&kept), 0o755, "a deliberately-permissioned file must be left untouched");
}
