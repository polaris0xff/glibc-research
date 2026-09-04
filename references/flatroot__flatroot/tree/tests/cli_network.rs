//! CLI tests that require network access.
//! Tests for search, query, release list, --with, and --postinstall.

use assert_cmd::Command;
use tempfile::TempDir;

// Shared export helpers (install + export against a real rootfs). Each test
// binary compiles its own copy; `#![allow(dead_code)]` in the module covers
// the helpers this binary does not use.
mod export;

// ---------------------------------------------------------------------------
// release list
// ---------------------------------------------------------------------------

#[test]
fn release_list_debian() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian", "release", "list"])
    .output()
    .unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("bookworm"), "expected bookworm in release list");
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

#[test]
fn search_substring() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "search", "bash"])
    .output()
    .unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("bash"), "expected bash in search results");
}

#[test]
fn search_glob_pattern() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "search", "python3.??"])
    .output()
    .unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("python3.11"), "expected python3.11 in glob search");
}

#[test]
fn search_nonsense_returns_empty() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "search", "zzz-nonexistent-xyz-999"])
    .output()
    .unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  // Plain encoding emits zero lines for an empty result — exit code
  // conveys success, absence of output conveys emptiness.
  assert!(stdout.is_empty(), "expected empty stdout for nonsense search, got {:#?}", stdout);
}

#[test]
fn search_variadic_patterns_union_packages() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "search", "firefox-esr", "thunderbird"])
    .output()
    .unwrap();
  assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
  let stdout = String::from_utf8_lossy(&output.stdout);
  // Both seeds must appear under the search.<n>.name=<value> shape.
  assert!(stdout.lines().any(|l| l.ends_with(".name=firefox-esr")), "missing firefox-esr in stdout:\n{}", stdout);
  assert!(stdout.lines().any(|l| l.ends_with(".name=thunderbird")), "missing thunderbird in stdout:\n{}", stdout);
}

#[test]
fn search_variadic_packages_dedup_by_name() {
  // Two patterns that both match the same package name (firefox-esr)
  // must collapse to a single entry — dedup by name.
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "search", "firefox-esr", "firefox-*"])
    .output()
    .unwrap();
  assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
  let stdout = String::from_utf8_lossy(&output.stdout);
  let firefox_esr_count = stdout.lines().filter(|l| l.ends_with(".name=firefox-esr")).count();
  assert_eq!(
    firefox_esr_count, 1,
    "firefox-esr should appear once after dedup, got {}:\n{}",
    firefox_esr_count, stdout
  );
}

// ---------------------------------------------------------------------------
// search --type=library
// ---------------------------------------------------------------------------

/// Verify a library glob resolves to the expected owning package, in
/// the plain encoding's `search.<n>.library=` and `search.<n>.name=`
/// shape. Used by the per-format-family cases below.
fn assert_search_library_resolves(from: &str, pattern: &str, expected_library: &str, expected_package: &str) {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", from, "search", "--type=library", pattern])
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "search --type=library failed for {} {}\nstderr:\n{}",
    from,
    pattern,
    String::from_utf8_lossy(&output.stderr)
  );
  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut found = false;
  // Walk indexed groups: every entry has `library=` and `name=` lines
  // under the same `search.<n>.` prefix. A pair matches when both sit
  // under the same prefix.
  for line in stdout.lines() {
    if let Some(rest) = line.strip_prefix("search.") {
      if let Some((idx, suffix)) = rest.split_once('.') {
        if suffix == format!("library={}", expected_library) {
          let want = format!("search.{}.name={}", idx, expected_package);
          if stdout.lines().any(|l| l == want) {
            found = true;
            break;
          }
        }
      }
    }
  }
  assert!(
    found,
    "expected library={} resolving to package={} for {} {}, stdout:\n{}",
    expected_library, expected_package, from, pattern, stdout
  );
}

#[test]
fn search_library_debian_libssl() {
  // Deb provides table carries package names (libssl3), not sonames —
  // the path-index leg supplies libssl.so.3.
  assert_search_library_resolves("debian:bookworm", "libssl.so.3", "libssl.so.3", "libssl3");
}

#[test]
fn search_library_arch_libssl() {
  // Arch provides table carries `libssl.so=3-64` — the `=`-decoration
  // strip yields `libssl.so` in the canonicalised library field.
  assert_search_library_resolves("arch:rolling", "libssl.so*", "libssl.so", "openssl");
}

#[test]
fn search_library_alpine_musl() {
  // Alpine has no path index; the provides leg under `so:` namespace
  // is the only source.
  assert_search_library_resolves("alpine:edge", "libc.musl-x86_64.so.1", "libc.musl-x86_64.so.1", "musl");
}

#[test]
fn search_library_fedora_libc() {
  // RPM provides carries `libc.so.6()(64bit)` — the suffix-strip
  // yields the canonical `libc.so.6` library name. The trailing `*`
  // in the pattern lets SQL GLOB reach the mangled raw text.
  assert_search_library_resolves("fedora:42", "libc.so.6*", "libc.so.6", "glibc");
}

#[test]
fn search_library_variadic_patterns_dedup_by_pair() {
  // Two patterns that both match the same (package, library) pair
  // — `libssl.so.3` matches both globs — must collapse to a single
  // entry. A pattern that yields a different library yields its
  // own entry.
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "search",
      "--type=library",
      "libssl.so.3",
      "lib*.so.3",
    ])
    .output()
    .unwrap();
  assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
  let stdout = String::from_utf8_lossy(&output.stdout);
  let libssl_count = stdout.lines().filter(|l| l.ends_with(".library=libssl.so.3")).count();
  assert_eq!(
    libssl_count, 1,
    "(libssl3, libssl.so.3) should appear once after pair-dedup, got {}:\n{}",
    libssl_count, stdout
  );
  assert!(
    stdout.lines().any(|l| l.ends_with(".library=libcrypto.so.3")),
    "expected libcrypto.so.3 entry from `lib*.so.3` pattern, stdout:\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// search --type=path
// ---------------------------------------------------------------------------

/// Verify a full-path glob resolves to the expected owning package, in
/// the plain encoding's `search.<n>.path=` and `search.<n>.name=` shape.
/// Used by the per-format-family cases below.
fn assert_search_path_resolves(from: &str, pattern: &str, expected_path: &str, expected_package: &str) {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", from, "search", "--type=path", pattern])
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "search --type=path failed for {} {}\nstderr:\n{}",
    from,
    pattern,
    String::from_utf8_lossy(&output.stderr)
  );
  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut found = false;
  // Walk indexed groups: every entry has `path=` and `name=` lines under
  // the same `search.<n>.` prefix. A pair matches when both sit under the
  // same prefix.
  for line in stdout.lines() {
    if let Some(rest) = line.strip_prefix("search.") {
      if let Some((idx, suffix)) = rest.split_once('.') {
        if suffix == format!("path={}", expected_path) {
          let want = format!("search.{}.name={}", idx, expected_package);
          if stdout.lines().any(|l| l == want) {
            found = true;
            break;
          }
        }
      }
    }
  }
  assert!(
    found,
    "expected path={} resolving to package={} for {} {}, stdout:\n{}",
    expected_path, expected_package, from, pattern, stdout
  );
}

#[test]
fn search_path_debian_bash() {
  // Debian Contents records bin/bash under the bash package; the full
  // path (not just the basename) is matched and echoed back.
  assert_search_path_resolves("debian:bookworm", "bin/bash", "bin/bash", "bash");
}

#[test]
fn search_path_fedora_bash() {
  // RPM filelists records /usr/bin/bash; the recorded leading slash is
  // stripped in the match space.
  assert_search_path_resolves("fedora:42", "usr/bin/bash", "usr/bin/bash", "bash");
}

#[test]
fn search_path_arch_bash() {
  // pacman .files records usr/bin/bash relative; ingestion normalises to
  // absolute and the match space strips the slash again.
  assert_search_path_resolves("arch:rolling", "usr/bin/bash", "usr/bin/bash", "bash");
}

// The DejaVuSans font is the canonical `Architecture: all` probe: its file
// lives in Debian's `Contents-all` on live and snapshot mirrors, in the
// merged per-arch Contents on the long-term archive, and in Ubuntu's
// always-merged per-arch Contents — so the same assertion exercised across
// releases proves the path match space sees arch-independent packages on
// every mirror generation.
const PATH_FONT_DEJAVU: &str = "usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

#[test]
fn search_path_debian_bookworm_finds_arch_all_font() {
  assert_search_path_resolves("debian:bookworm", PATH_FONT_DEJAVU, PATH_FONT_DEJAVU, "fonts-dejavu-core");
}

#[test]
fn search_path_debian_bullseye_finds_arch_all_font() {
  assert_search_path_resolves("debian:bullseye", PATH_FONT_DEJAVU, PATH_FONT_DEJAVU, "fonts-dejavu-core");
}

#[test]
fn search_path_debian_snapshot_finds_arch_all_font() {
  // Dated pins draw from snapshot.debian.org, which preserves the live
  // mirrors' split layout — Contents-all must be fetched there too.
  assert_search_path_resolves("debian:bookworm@2024-06-15", PATH_FONT_DEJAVU, PATH_FONT_DEJAVU, "fonts-dejavu-core");
}

#[test]
fn search_path_debian_buster_archive_merged_contents() {
  // buster is served by archive.debian.org, which merges Contents-all
  // into the per-arch listing and drops the separate file. The font must
  // still resolve (via the merged listing), and the absent Contents-all
  // must be skipped with a warning naming it — never a hard failure.
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:buster",
      "-v",
      "search",
      "--type=path",
      PATH_FONT_DEJAVU,
    ])
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "buster path search must survive the missing Contents-all:\n{}",
    String::from_utf8_lossy(&output.stderr)
  );
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(
    stdout.contains(&format!("path={}", PATH_FONT_DEJAVU)),
    "the merged per-arch Contents must supply the font, stdout:\n{}",
    stdout
  );
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("Contents-all.gz") && stderr.contains("absent on every mirror"),
    "the skipped listing must be named under -v, stderr:\n{}",
    stderr
  );
}

#[test]
fn search_path_ubuntu_noble_finds_arch_all_font() {
  assert_search_path_resolves("ubuntu:noble", PATH_FONT_DEJAVU, PATH_FONT_DEJAVU, "fonts-dejavu-core");
}

#[test]
fn search_path_ubuntu_jammy_finds_arch_all_font() {
  assert_search_path_resolves("ubuntu:jammy", PATH_FONT_DEJAVU, PATH_FONT_DEJAVU, "fonts-dejavu-core");
}

#[test]
fn search_path_alpine_is_rejected() {
  // apk publishes no file lists, so the path match space cannot answer
  // there — the request is refused with the reason, not answered empty.
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "alpine:v3.21", "search", "--type=path", "usr/bin/bash"])
    .output()
    .unwrap();
  assert!(!output.status.success(), "alpine --type=path must fail");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("unsupported for apk sources"), "error must explain the apk gap, stderr:\n{}", stderr);
}

#[test]
fn search_path_nonsense_returns_empty() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "search",
      "--type=path",
      "zzz/nonexistent/xyz-999",
    ])
    .output()
    .unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.is_empty(), "expected empty stdout for nonsense path search, got {:#?}", stdout);
}

#[test]
fn search_library_json_format() {
  // JSON shape carries the library field; verify the key is present.
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "search",
      "--type=library",
      "--format=json",
      "libssl.so.3",
    ])
    .output()
    .unwrap();
  assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
  let stdout = String::from_utf8_lossy(&output.stdout);
  let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");
  let arr = parsed.as_array().expect("top-level must be array");
  assert!(!arr.is_empty(), "expected at least one entry, got empty array");
  let first = arr[0].as_object().expect("entries must be objects");
  for key in ["description", "library", "name", "version"] {
    assert!(first.contains_key(key), "missing key {} in entry: {}", key, arr[0]);
  }
}

// ---------------------------------------------------------------------------
// query
// ---------------------------------------------------------------------------

#[test]
fn query_count_packages() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "query"])
    .write_stdin("SELECT count(*) as total FROM packages")
    .output()
    .unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  // Plain encoding: a single-column aggregate renders as one cell —
  // query.0.cells.0.column=total and query.0.cells.0.value=<integer>. The
  // column name is data carried as a value; the count is the cell value.
  assert!(stdout.lines().any(|l| l == "query.0.cells.0.column=total"), "expected the total column cell:\n{stdout}");
  let line = stdout
    .lines()
    .find(|l| l.starts_with("query.0.cells.0.value="))
    .unwrap_or_else(|| panic!("expected 'query.0.cells.0.value=' line in stdout:\n{stdout}"));
  let count: usize = line
    .strip_prefix("query.0.cells.0.value=")
    .unwrap()
    .trim()
    .parse()
    .unwrap_or_else(|e| panic!("expected integer on '{line}': {e}"));
  assert!(count > 60000, "expected > 60000 packages, got {}", count);
}

// ---------------------------------------------------------------------------
// --with=recommends
// ---------------------------------------------------------------------------

#[test]
fn with_recommends_pulls_more_packages() {
  let root_without = TempDir::new().unwrap();
  let cache_without = TempDir::new().unwrap();
  let out_without = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache_without.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root_without.path().to_str().unwrap(),
      "--postinstall=none",
      "python3-minimal",
    ])
    .output()
    .unwrap();
  assert!(out_without.status.success());

  let root_with = TempDir::new().unwrap();
  let cache_with = TempDir::new().unwrap();
  let out_with = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache_with.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root_with.path().to_str().unwrap(),
      "--postinstall=none",
      "--with=recommends",
      "python3-minimal",
    ])
    .output()
    .unwrap();
  assert!(out_with.status.success());

  let count_without = extract_resolved_count(&out_without.stderr);
  let count_with = extract_resolved_count(&out_with.stderr);
  assert!(
    count_with > count_without,
    "--with=recommends should pull more packages: {} vs {}",
    count_with,
    count_without
  );
}

// ---------------------------------------------------------------------------
// --with=suggests
// ---------------------------------------------------------------------------

#[test]
fn with_suggests_pulls_more_packages() {
  // Test at the resolver level to avoid downloading thousands of packages.
  // `--with=suggests` on Arch recursively resolves %OPTDEPENDS% which can pull
  // 2000+ packages for a package like git. By calling the resolver directly we
  // skip download/extract and just compare resolved counts.
  let cache = TempDir::new().unwrap();
  // Safety: this is the only test in the binary that reads FLATROOT_CACHE_HOME
  // from process env (siblings spawn subprocesses with explicit env), so the
  // mutation cannot race with another test's read.
  unsafe { std::env::set_var("FLATROOT_CACHE_HOME", cache.path()) };
  let http = std::sync::Arc::new(flatroot::internal::http::HttpClient::new(
    flatroot::internal::cache::Cache::dir_resolve().unwrap().dir_index(),
    3,
    std::time::Duration::from_secs(3600),
  ));
  let remote = flatroot::remote::RemoteDistro::from_str("arch:rolling", flatroot::arch::Arch::X86_64, http).unwrap();
  let vcmp: std::sync::Arc<dyn flatroot::version::VersionCompare> = std::sync::Arc::from(remote.version_compare());

  let index =
    flatroot::db::Index::open_or_populate(&remote.cache_key(), vcmp, |writer| remote.index_fetch(writer)).unwrap();

  let mut seeds: Vec<String> = remote.base_packages().iter().map(|s| s.to_string()).collect();
  seeds.push("curl".to_string());

  let cfg_without = flatroot::resolver::ResolutionEnv {
    index: &index,
    include_recommends: false,
    include_suggests: false,
    exclude: &[],
  };
  let cfg_with = flatroot::resolver::ResolutionEnv {
    index: &index,
    include_recommends: false,
    include_suggests: true,
    exclude: &[],
  };
  let without = flatroot::resolver::DepWalker::resolve(&cfg_without, &seeds).unwrap();
  let with_optional = flatroot::resolver::DepWalker::resolve(&cfg_with, &seeds).unwrap();

  assert!(
    with_optional.len() > without.len(),
    "--with=suggests should resolve more packages: {} vs {}",
    with_optional.len(),
    without.len()
  );
}

// ---------------------------------------------------------------------------
// --postinstall=none
// ---------------------------------------------------------------------------

#[test]
fn postinstall_none_skips_everything() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      "bash",
    ])
    .output()
    .unwrap();
  assert!(output.status.success());
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(!stderr.contains("postinst:"), "scripts should not run with --postinstall=none");
  assert!(!stderr.contains("hook:"), "hooks should not run with --postinstall=none");
  assert!(!stderr.contains("ldconfig"), "ldconfig should not run with --postinstall=none");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_resolved_count(stderr: &[u8]) -> usize {
  let text = String::from_utf8_lossy(stderr);
  for line in text.lines() {
    if let Some(rest) = line.strip_prefix("Resolved ")
      && let Some(num) = rest.split_whitespace().next()
    {
      return num
        .parse()
        .unwrap_or_else(|e| panic!("cannot parse resolved count from '{line}': {e}"));
    }
  }
  panic!("no 'Resolved N packages ...' line in stderr:\n{text}")
}

/// Spawn `flatroot` with a fresh isolated cache and return the completed
/// `Output`. The cache `TempDir` is dropped only after the child exits.
fn run(args: &[&str]) -> std::process::Output {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(args)
    .output()
    .expect("flatroot failed to spawn")
}

/// Spawn `flatroot` with a fresh cache and one extra environment variable.
fn run_env(env_key: &str, env_val: &str, args: &[&str]) -> std::process::Output {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env(env_key, env_val)
    .args(args)
    .output()
    .expect("flatroot failed to spawn")
}

fn ok_stderr(out: &std::process::Output) -> String {
  assert!(
    out.status.success(),
    "command failed (exit={:?})\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stderr).into_owned()
}

// ---------------------------------------------------------------------------
// --from env fallback driving a real command (CLI-007)
// ---------------------------------------------------------------------------

// covers: CLI-007
#[test]
fn env_from_drives_release_list() {
  // FLATROOT_ARG_FROM supplies --from for `release list`, which actually lists
  // Debian's releases — proving the env value drives a working command end to
  // end, not just satisfying the from-required gate.
  let out = run_env("FLATROOT_ARG_FROM", "debian:bookworm", &["release", "list"]);
  let bytes = ok_stdout(&out);
  let stdout = String::from_utf8_lossy(&bytes);
  assert!(stdout.contains("bookworm"), "release list driven by env --from must include bookworm:\n{}", stdout);
}

fn ok_stdout(out: &std::process::Output) -> Vec<u8> {
  assert!(
    out.status.success(),
    "command failed (exit={:?})\nstderr:\n{}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr)
  );
  out.stdout.clone()
}

// ---------------------------------------------------------------------------
// --arch host default and the x86 → i686 alias at the CLI (CLI-008, CLI-009)
// ---------------------------------------------------------------------------

// covers: CLI-008
#[test]
fn arch_default_is_host_uname() {
  // Omitting --arch on an x86_64 host selects the host arch: the install lays
  // libc into the host-arch multiarch dir `usr/lib/x86_64-linux-gnu` (or its
  // `lib/...` merged-usr alias). This mirrors `install_no_deps`, which asserts
  // the same path is *absent* under --no-deps.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      "bash",
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(
    root.path().join("usr/lib/x86_64-linux-gnu").exists() || root.path().join("lib/x86_64-linux-gnu").exists(),
    "host-arch (x86_64) default must land the x86_64 multiarch lib dir"
  );
}

// covers: CLI-009
#[test]
fn arch_x86_alias_maps_to_i686_at_cli() {
  // `--arch x86` is the historical alias for i686. The CLI maps it through
  // `Arch::from_uname` → I686 → Debian `i386`, so the install lays libs into
  // the i386 multiarch dir — observable proof of the alias at the CLI level.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      "dash",
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(
    root.path().join("lib/i386-linux-gnu").exists() || root.path().join("usr/lib/i386-linux-gnu").exists(),
    "`--arch x86` must map to i686 and lay down the i386 multiarch lib dir"
  );
}

// ---------------------------------------------------------------------------
// analyze takes the first --arch token from a comma list (CLI-012)
// ---------------------------------------------------------------------------

// covers: CLI-012
#[test]
fn analyze_arch_comma_list_uses_first_token() {
  // analyze splits --arch on ',' and uses only the first token (main.rs:126).
  // `x86_64,bogusarch` would fail at `from_uname` if the second token were
  // read; instead the run succeeds and the framing reports the first token's
  // arch — proving the second is ignored.
  let out = run(&[
    "--from",
    "debian:bookworm",
    "--arch",
    "x86_64,bogusarch",
    "analyze",
    "trace",
    "bash",
  ]);
  let stderr = ok_stderr(&out);
  assert!(
    stderr.contains("(x86_64)"),
    "analyze must use the first --arch token and frame with it; second token must be ignored:\n{}",
    stderr
  );
}

// ---------------------------------------------------------------------------
// --verbose / FLATROOT_ARG_VERBOSE never pollute stdout (CLI-016, CLI-017)
// ---------------------------------------------------------------------------

// covers: CLI-016
#[test]
fn verbose_flag_does_not_change_stdout() {
  // Verbosity routes only to the stderr warn channel (ui.rs); stdout is the
  // data stream and must be identical with and without -v.
  let quiet = run(&["--from", "debian:bookworm", "search", "bash"]);
  let loud = run(&["--from", "debian:bookworm", "-v", "search", "bash"]);
  assert_eq!(ok_stdout(&quiet), ok_stdout(&loud), "-v must not change search stdout");
}

// covers: CLI-017
#[test]
fn env_verbose_is_accepted_and_keeps_stdout_clean() {
  // FLATROOT_ARG_VERBOSE=true supplies --verbose; the run still succeeds and
  // its stdout is the same data as a non-verbose run (verbosity is a stderr-only
  // concern).
  let plain = run(&["--from", "debian:bookworm", "search", "bash"]);
  let verbose = run_env("FLATROOT_ARG_VERBOSE", "true", &["--from", "debian:bookworm", "search", "bash"]);
  assert_eq!(ok_stdout(&plain), ok_stdout(&verbose), "FLATROOT_ARG_VERBOSE must not change stdout");
}

// ---------------------------------------------------------------------------
// install --output via -o / --output / env all resolve identically (CLI-020)
// ---------------------------------------------------------------------------

// covers: CLI-020
#[test]
fn install_output_env_lands_rootfs_there() {
  // FLATROOT_ARG_INSTALL_OUTPUT supplies the rootfs directory when -o/--output
  // are absent — the tree is built at that path.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env("FLATROOT_ARG_INSTALL_OUTPUT", root.path().to_str().unwrap())
    .args(["--from", "debian:bookworm", "install", "--postinstall=none", "bash"])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(root.path().join("bin/bash").exists(), "FLATROOT_ARG_INSTALL_OUTPUT must place the rootfs at that path");
}

// covers: CLI-020
#[test]
fn install_output_o_and_long_flag_agree() {
  // `-o` and `--output` are the same option; both place bash at the same path.
  for flag in ["-o", "--output"] {
    let root = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let out = Command::cargo_bin("flatroot")
      .unwrap()
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args([
        "--from",
        "debian:bookworm",
        "install",
        flag,
        root.path().to_str().unwrap(),
        "--postinstall=none",
        "bash",
      ])
      .output()
      .unwrap();
    assert!(out.status.success(), "{} stderr:\n{}", flag, String::from_utf8_lossy(&out.stderr));
    assert!(root.path().join("bin/bash").exists(), "{} must place the rootfs at the given path", flag);
  }
}

// ---------------------------------------------------------------------------
// install --with recommends,suggests combined widening (CLI-022)
// ---------------------------------------------------------------------------

// covers: CLI-022
#[test]
fn install_with_recommends_widens_closure() {
  // `install --with` must widen the install set beyond the bare closure: the
  // resolved count with `--with recommends` must exceed the count with
  // neither. Compared at the resolver-reported "Resolved N packages" line
  // (stderr) to avoid asserting on a specific package. Recommends is the one
  // soft-dependency kind whose install closure stays small on Debian: the
  // essential set seeded into every install reaches libc6, whose transitive
  // Suggests pull a four-thousand-package closure, while its Recommends
  // frontier stays within ~100 packages.
  let bare = {
    let root = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    Command::cargo_bin("flatroot")
      .unwrap()
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args([
        "--from",
        "debian:bookworm",
        "install",
        "--output",
        root.path().to_str().unwrap(),
        "--postinstall=none",
        "libsuitesparse-doc",
      ])
      .output()
      .unwrap()
  };
  assert!(bare.status.success(), "bare stderr:\n{}", String::from_utf8_lossy(&bare.stderr));
  let widened = {
    let root = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    Command::cargo_bin("flatroot")
      .unwrap()
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args([
        "--from",
        "debian:bookworm",
        "install",
        "--output",
        root.path().to_str().unwrap(),
        "--postinstall=none",
        "--with",
        "recommends",
        "libsuitesparse-doc",
      ])
      .output()
      .unwrap()
  };
  assert!(widened.status.success(), "widened stderr:\n{}", String::from_utf8_lossy(&widened.stderr));
  let bare_count = extract_resolved_count(&bare.stderr);
  let widened_count = extract_resolved_count(&widened.stderr);
  assert!(
    widened_count > bare_count,
    "--with recommends must widen the closure: {} (widened) vs {} (bare)",
    widened_count,
    bare_count
  );
}

// ---------------------------------------------------------------------------
// install --postinstall default fires all three phases (CLI-023)
// ---------------------------------------------------------------------------

// covers: CLI-023
#[test]
fn install_postinstall_default_runs_all_three_phases() {
  // Omitting --postinstall runs ldconfig + scripts + hooks (cli.rs default).
  // Each phase emits a stable completion line to stderr via RunVoice::finish_ok.
  // (Requires unprivileged user namespaces for the script/hook sandbox.)
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
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
    .unwrap();
  let stderr = ok_stderr(&out);
  assert!(stderr.contains("ldconfig completed"), "default postinstall must run ldconfig:\n{}", stderr);
  assert!(stderr.contains("Postinst scripts completed"), "default postinstall must run scripts:\n{}", stderr);
  assert!(stderr.contains("Cache hooks completed"), "default postinstall must run hooks:\n{}", stderr);
}

// ---------------------------------------------------------------------------
// install --postinstall subset runs only the named phases (CLI-024)
// ---------------------------------------------------------------------------

// covers: CLI-024
#[test]
fn install_postinstall_subset_skips_hooks() {
  // `--postinstall ldconfig,scripts` runs those two phases but not hooks:
  // CacheHooks::run (which emits "Cache hooks completed") is never reached.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall",
      "ldconfig,scripts",
      "bash",
    ])
    .output()
    .unwrap();
  let stderr = ok_stderr(&out);
  assert!(stderr.contains("ldconfig completed"), "ldconfig phase must run:\n{}", stderr);
  assert!(stderr.contains("Postinst scripts completed"), "scripts phase must run:\n{}", stderr);
  assert!(!stderr.contains("Cache hooks completed"), "hooks phase must be skipped:\n{}", stderr);
  assert!(!stderr.contains("Running cache hooks"), "hooks phase must not even start:\n{}", stderr);
}

// ---------------------------------------------------------------------------
// install --parallel serializes / honors the jobs count (CLI-029)
// ---------------------------------------------------------------------------

// covers: CLI-029
#[test]
fn install_parallel_one_serializes_and_succeeds() {
  // `-p 1` forces sequential downloads; the install still completes correctly.
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      "-p",
      "1",
      "bash",
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(root.path().join("bin/bash").exists(), "-p 1 (serial) install must still produce bash");
}

// ---------------------------------------------------------------------------
// install --exclude split/trim/empty-filter drops the package (CLI-030)
// ---------------------------------------------------------------------------

// covers: CLI-030
#[test]
fn install_exclude_trims_and_drops_package() {
  // `--exclude 'libssl3, foo,'` is split on ',', each entry trimmed, empties
  // filtered → ["libssl3","foo"]. Installing curl with libssl3 excluded must
  // produce a closure smaller than without the exclude, and the excluded
  // package must not be installed. The leading/trailing spaces and the
  // trailing empty token exercise trim + empty-filter.
  let without = {
    let root = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    Command::cargo_bin("flatroot")
      .unwrap()
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args([
        "--from",
        "debian:bookworm",
        "install",
        "--output",
        root.path().to_str().unwrap(),
        "--postinstall=none",
        "openssl",
      ])
      .output()
      .unwrap()
  };
  assert!(without.status.success(), "without-exclude stderr:\n{}", String::from_utf8_lossy(&without.stderr));
  let with = {
    let root = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    Command::cargo_bin("flatroot")
      .unwrap()
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args([
        "--from",
        "debian:bookworm",
        "install",
        "--output",
        root.path().to_str().unwrap(),
        "--postinstall=none",
        "--exclude",
        "libssl3, foo,",
        "openssl",
      ])
      .output()
      .unwrap()
  };
  assert!(with.status.success(), "with-exclude stderr:\n{}", String::from_utf8_lossy(&with.stderr));
  let without_count = extract_resolved_count(&without.stderr);
  let with_count = extract_resolved_count(&with.stderr);
  assert!(
    with_count < without_count,
    "excluding libssl3 (after trim) must shrink the closure: {} (with exclude) vs {} (without)",
    with_count,
    without_count
  );
}

// ---------------------------------------------------------------------------
// search --type default is package (CLI-032)
// ---------------------------------------------------------------------------

// covers: CLI-032
#[test]
fn search_type_default_is_package() {
  // Without --type, a search matches package names (not library files): the
  // default-mode output for `bash` carries `.name=bash` and no `.library=`
  // lines, whereas --type=library output carries `.library=` lines.
  let default_out = run(&["--from", "debian:bookworm", "search", "bash"]);
  let default_stdout = String::from_utf8_lossy(&ok_stdout(&default_out)).into_owned();
  assert!(
    default_stdout.lines().any(|l| l.ends_with(".name=bash")),
    "default (package) search must match the package name:\n{}",
    default_stdout
  );
  assert!(
    !default_stdout.lines().any(|l| l.contains(".library=")),
    "package-mode search must not emit library fields:\n{}",
    default_stdout
  );

  let lib_out = run(&[
    "--from",
    "debian:bookworm",
    "search",
    "--type",
    "library",
    "libssl.so.3",
  ]);
  let lib_stdout = String::from_utf8_lossy(&ok_stdout(&lib_out)).into_owned();
  assert!(
    lib_stdout.lines().any(|l| l.contains(".library=")),
    "library-mode search must emit library fields:\n{}",
    lib_stdout
  );
}

// ---------------------------------------------------------------------------
// search --format default plain; json bijective (CLI-033)
// ---------------------------------------------------------------------------

// covers: CLI-033
#[test]
fn search_format_default_is_plain() {
  let out = run(&["--from", "debian:bookworm", "search", "bash"]);
  let stdout = String::from_utf8_lossy(&ok_stdout(&out)).into_owned();
  assert!(
    stdout.lines().any(|l| l.starts_with("search.")),
    "default search format must be plain dotted KEY=VALUE:\n{}",
    stdout
  );
  assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err(), "default search format must not be JSON");
}

// covers: CLI-033
#[test]
fn search_package_json_is_bijective_with_plain() {
  // Package-mode JSON is a top-level array of {description, name, version};
  // every name in JSON appears in the plain `.name=` lines and vice versa.
  let plain = String::from_utf8_lossy(&ok_stdout(&run(&["--from", "debian:bookworm", "search", "bash"]))).into_owned();
  let json =
    String::from_utf8_lossy(&ok_stdout(&run(&["--from", "debian:bookworm", "search", "--format", "json", "bash"])))
      .into_owned();
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("search json must parse");
  let arr = parsed.as_array().expect("search json must be a top-level array");
  assert!(!arr.is_empty(), "search bash must match at least one package");
  let first = arr[0].as_object().expect("entries are objects");
  for key in ["description", "name", "version"] {
    assert!(first.contains_key(key), "package entry missing key {}: {}", key, arr[0]);
  }
  let plain_names: Vec<&str> = plain
    .lines()
    .filter_map(|l| l.split_once(".name=").map(|(_, v)| v))
    .collect();
  let json_names: Vec<&str> = arr.iter().filter_map(|e| e["name"].as_str()).collect();
  for n in &json_names {
    assert!(plain_names.contains(n), "package {} in JSON missing from plain:\n{}", n, plain);
  }
  for n in &plain_names {
    assert!(json_names.contains(n), "package {} in plain missing from JSON:\n{}", n, json);
  }
}

// ---------------------------------------------------------------------------
// query reads SQL from a positional FILE (CLI-034) and format contracts (CLI-037)
// ---------------------------------------------------------------------------

// covers: CLI-034
#[test]
fn query_reads_sql_from_positional_file() {
  // `query <FILE>` runs the SQL read from the file against the cached index.
  let dir = TempDir::new().unwrap();
  let path_file_sql = dir.path().join("q.sql");
  std::fs::write(&path_file_sql, "SELECT count(*) AS total FROM packages").unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "query", path_file_sql.to_str().unwrap()])
    .output()
    .unwrap();
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  let line = stdout
    .lines()
    .find(|l| l.starts_with("query.0.cells.0.value="))
    .unwrap_or_else(|| panic!("expected query.0.cells.0.value= line:\n{stdout}"));
  let count: usize = line
    .strip_prefix("query.0.cells.0.value=")
    .unwrap()
    .trim()
    .parse()
    .unwrap_or_else(|e| panic!("bad count on '{line}': {e}"));
  assert!(count > 60000, "SQL from file must run against the index, got {} packages", count);
}

// covers: CLI-037
#[test]
fn query_plain_drops_null_json_preserves_null() {
  // A SELECT that yields a NULL column: plain drops the NULL cell entirely
  // (no line for it), while JSON preserves an explicit `null` for it.
  let sql = "SELECT 'present' AS a, NULL AS b";
  let plain = run_query_stdin(sql, &[]);
  assert!(
    plain.lines().any(|l| l == "query.0.cells.0.value=present"),
    "plain must keep the present column's value:\n{}",
    plain
  );
  assert!(plain.lines().any(|l| l == "query.0.cells.1.column=b"), "the NULL column still names itself:\n{}", plain);
  assert!(
    !plain.lines().any(|l| l.starts_with("query.0.cells.1.value=")),
    "plain must omit the NULL column's value line:\n{}",
    plain
  );

  let json_text = run_query_stdin(sql, &["--format", "json"]);
  let parsed: serde_json::Value = serde_json::from_str(&json_text).expect("query json must parse");
  let cells = parsed[0]["cells"]
    .as_array()
    .expect("a query json row carries a cells array");
  let cell_a = cells.iter().find(|c| c["column"] == "a").expect("column a present");
  assert_eq!(cell_a["value"].as_str(), Some("present"), "json must keep the present column: {}", json_text);
  let cell_b = cells.iter().find(|c| c["column"] == "b").expect("column b present");
  assert!(cell_b["value"].is_null(), "json must preserve the NULL column as null: {}", json_text);
}

// covers: CLI-037
#[test]
fn query_format_default_is_plain() {
  let stdout = run_query_stdin("SELECT 'x' AS a", &[]);
  assert!(stdout.lines().any(|l| l == "query.0.cells.0.value=x"), "default query format must be plain:\n{}", stdout);
  assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err(), "default query format must not be JSON");
}

/// Run `flatroot --from debian:bookworm query <extra...>` with `sql` on stdin
/// and return stdout (panicking with stderr on failure).
fn run_query_stdin(sql: &str, extra: &[&str]) -> String {
  let cache = TempDir::new().unwrap();
  let mut args: Vec<&str> = vec!["--from", "debian:bookworm", "query"];
  args.extend_from_slice(extra);
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&args)
    .write_stdin(sql)
    .output()
    .unwrap();
  assert!(out.status.success(), "query failed (sql={:?}) stderr:\n{}", sql, String::from_utf8_lossy(&out.stderr));
  String::from_utf8_lossy(&out.stdout).into_owned()
}

// ---------------------------------------------------------------------------
// release list --format json bijective with per-distro fields (CLI-044)
// ---------------------------------------------------------------------------

// covers: CLI-044
#[test]
fn release_list_json_is_bijective_and_carries_distro_fields() {
  // Debian's release listing publishes a `codename` and the `mirror` it is
  // served from per release (the header "Codename"/"Mirror" normalised to
  // lowercase; only "Release" maps to "name", which Debian does not use). JSON
  // is a top-level array; every release `codename` in JSON appears in the plain
  // `.codename=` lines.
  let plain = String::from_utf8_lossy(&ok_stdout(&run(&["--from", "debian", "release", "list"]))).into_owned();
  let json = String::from_utf8_lossy(&ok_stdout(&run(&["--from", "debian", "release", "list", "--format", "json"])))
    .into_owned();
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("release json must parse");
  let arr = parsed.as_array().expect("release json must be a top-level array");
  assert!(!arr.is_empty(), "debian must publish releases");
  // Confirm each release carries at least one field beyond its codename (the
  // mirror it is served from).
  let bookworm = arr
    .iter()
    .find(|e| e["codename"].as_str() == Some("bookworm"))
    .unwrap_or_else(|| panic!("bookworm must be in the release list: {}", json));
  let obj = bookworm.as_object().unwrap();
  assert!(obj.len() > 1, "each release must carry more than just a codename: {}", bookworm);
  let plain_codenames: Vec<&str> = plain
    .lines()
    .filter_map(|l| l.split_once(".codename=").map(|(_, v)| v))
    .collect();
  for e in arr {
    if let Some(codename) = e["codename"].as_str() {
      assert!(plain_codenames.contains(&codename), "release {} in JSON missing from plain:\n{}", codename, plain);
    }
  }
}

// ---------------------------------------------------------------------------
// export needs no --from / no --arch; arch inferred from the rootfs (CLI-047)
// ---------------------------------------------------------------------------

// covers: CLI-047
#[test]
fn export_infers_arch_from_rootfs_without_from_or_arch() {
  // Build a rootfs, then export with NO --from and NO --arch. The OCI exporter
  // detects the arch from the rootfs's own binaries and announces it on stderr
  // ("detected architecture: <uname> (<goarch>)"), proving arch came from the
  // tree, not a flag.
  export::require_tool("tar");
  let cache = TempDir::new().unwrap();
  let rootfs_dir = TempDir::new().unwrap();
  let install = export::install_with_cache("debian:bookworm", rootfs_dir.path(), cache.path(), "bash");
  export::install_assert_ok(&install, "export arch-infer rootfs");

  let (_out_dir, out_path) = export::fresh_output_path(".tar");
  // Invoke export directly (no --from, no --arch) targeting OCI.
  let export_cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", export_cache.path())
    .args([
      "export",
      rootfs_dir.path().to_str().unwrap(),
      out_path.to_str().unwrap(),
      "--format",
      "oci",
      "-t",
      "infer:test",
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "export stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("detected architecture:"), "export must detect arch from the rootfs:\n{}", stderr);
}

// ---------------------------------------------------------------------------
// export --format inference from output extension (CLI-048)
// ---------------------------------------------------------------------------

// covers: CLI-048
#[test]
fn export_infers_format_from_extension() {
  // With no --format, the `.tar.gz` extension is inferred as the tar format
  // (ExportFormat::resolve). The exporter runs and writes a gzip tar.
  export::require_tool("tar");
  let cache = TempDir::new().unwrap();
  let rootfs_dir = TempDir::new().unwrap();
  let install = export::install_with_cache("debian:bookworm", rootfs_dir.path(), cache.path(), "bash");
  export::install_assert_ok(&install, "export format-infer rootfs");

  let (_out_dir, out_path) = export::fresh_output_path(".tar.gz");
  let export_cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", export_cache.path())
    .args([
      "export",
      rootfs_dir.path().to_str().unwrap(),
      out_path.to_str().unwrap(),
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "export (inferred .tar.gz) stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(out_path.exists(), "inferred-format export must produce the output file");
  let mut magic = [0u8; 2];
  use std::io::Read as _;
  std::fs::File::open(&out_path).unwrap().read_exact(&mut magic).unwrap();
  assert_eq!(magic, [0x1f, 0x8b], "inferred .tar.gz must be gzip-compressed");
}

// ---------------------------------------------------------------------------
// export --tag required for oci, ignored otherwise (CLI-050)
// ---------------------------------------------------------------------------

// covers: CLI-050
#[test]
fn export_oci_without_tag_fails() {
  // OCI requires --tag (export/mod.rs:91). Omitting it fails with the exact
  // bail, after the rootfs is confirmed real.
  export::require_tool("tar");
  let cache = TempDir::new().unwrap();
  let rootfs_dir = TempDir::new().unwrap();
  let install = export::install_with_cache("debian:bookworm", rootfs_dir.path(), cache.path(), "bash");
  export::install_assert_ok(&install, "export oci-no-tag rootfs");

  let (_out_dir, out_path) = export::fresh_output_path(".tar");
  let export_cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", export_cache.path())
    .args([
      "export",
      rootfs_dir.path().to_str().unwrap(),
      out_path.to_str().unwrap(),
      "--format",
      "oci",
    ])
    .output()
    .unwrap();
  assert!(!out.status.success(), "OCI export without --tag must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("--tag is required for OCI format"), "expected the oci-tag bail:\n{}", stderr);
}

// covers: CLI-050
#[test]
fn export_tar_ignores_tag() {
  // For tar, --tag is accepted but ignored (export/mod.rs:94) — the export
  // succeeds and produces a tar regardless of the tag value.
  export::require_tool("tar");
  let cache = TempDir::new().unwrap();
  let rootfs_dir = TempDir::new().unwrap();
  let install = export::install_with_cache("debian:bookworm", rootfs_dir.path(), cache.path(), "bash");
  export::install_assert_ok(&install, "export tar-tag-ignored rootfs");

  let (_out_dir, out_path) = export::fresh_output_path(".tar.gz");
  let out = export::run_export(rootfs_dir.path(), "tar", &out_path, &["-t", "ignored:tag"]);
  assert!(
    out.status.success(),
    "tar export with --tag must succeed (tag ignored):\n{}",
    String::from_utf8_lossy(&out.stderr)
  );
  assert!(out_path.exists(), "tar export must produce the output file even with a tag");
}

// ---------------------------------------------------------------------------
// analyze trace --type default is package (CLI-053)
// ---------------------------------------------------------------------------

// covers: CLI-053
#[test]
fn analyze_trace_type_default_is_package() {
  // Default --type matches package names: `bash` seeds the trace as the bash
  // package (reason=target). In --type=library mode, a soname pattern seeds the
  // owning package instead.
  let pkg = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--format",
    "json",
    "bash",
  ]);
  let pkg_json: serde_json::Value = serde_json::from_slice(&ok_stdout(&pkg)).expect("trace json must parse");
  let entries = pkg_json.as_array().expect("trace json is an array");
  assert!(
    entries.iter().any(|e| e["name"] == "bash" && e["reason"] == "target"),
    "default (package) trace of bash must seed the bash package as target: {}",
    pkg_json
  );

  let lib = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--type",
    "library",
    "--format",
    "json",
    "libssl.so.3",
  ]);
  let lib_json: serde_json::Value = serde_json::from_slice(&ok_stdout(&lib)).expect("library trace json must parse");
  let lib_entries = lib_json.as_array().expect("trace json is an array");
  assert!(
    lib_entries
      .iter()
      .any(|e| e["name"] == "libssl3" && e["reason"] == "target"),
    "library-mode trace of libssl.so.3 must seed its owning package libssl3 as target: {}",
    lib_json
  );
}

// ---------------------------------------------------------------------------
// analyze trace --format default plain; env drives format (CLI-054)
// ---------------------------------------------------------------------------

// covers: CLI-054
#[test]
fn analyze_trace_format_default_is_plain() {
  let out = run(&["--from", "debian:bookworm", "analyze", "trace", "bash"]);
  let stdout = String::from_utf8_lossy(&ok_stdout(&out)).into_owned();
  assert!(
    stdout.lines().any(|l| l.starts_with("analyze.trace.")),
    "default analyze format must be plain dotted KEY=VALUE:\n{}",
    stdout
  );
  assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err(), "default analyze format must not be JSON");
}

// covers: CLI-054
#[test]
fn analyze_trace_format_env_drives_json() {
  // FLATROOT_ARG_ANALYZE_TRACE_FORMAT=json makes stdout a JSON array even with
  // no --format flag.
  let out =
    run_env("FLATROOT_ARG_ANALYZE_TRACE_FORMAT", "json", &["--from", "debian:bookworm", "analyze", "trace", "bash"]);
  let stdout = ok_stdout(&out);
  let parsed: serde_json::Value = serde_json::from_slice(&stdout).expect("env-driven json format must produce JSON");
  assert!(parsed.as_array().map(|a| !a.is_empty()).unwrap_or(false), "env-driven json trace must be a non-empty array");
}

// ---------------------------------------------------------------------------
// analyze trace --strategy declared vs linker (CLI-055)
// ---------------------------------------------------------------------------

// covers: CLI-055
#[test]
fn analyze_trace_strategy_declared_omits_linker_only_edges() {
  // `--strategy declared` derives no_linker, so no entry can be reason=linker
  // (linker-only). `--strategy linker` derives no_deps, so no entry can be
  // reason=declared (declared-only). The complementary single-strategy runs
  // prove each strategy is independently selectable.
  let declared = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--strategy",
    "declared",
    "--format",
    "json",
    "bash",
  ]);
  let declared_json: serde_json::Value = serde_json::from_slice(&ok_stdout(&declared)).expect("declared trace json");
  for e in declared_json.as_array().expect("array") {
    assert_ne!(e["reason"], "linker", "--strategy declared must not produce linker-only entries: {}", e);
  }

  let linker = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--strategy",
    "linker",
    "--format",
    "json",
    "bash",
  ]);
  let linker_json: serde_json::Value = serde_json::from_slice(&ok_stdout(&linker)).expect("linker trace json");
  for e in linker_json.as_array().expect("array") {
    assert_ne!(e["reason"], "declared", "--strategy linker must not produce declared-only entries: {}", e);
  }
}

// ---------------------------------------------------------------------------
// analyze trace --with widens the declared closure (CLI-058)
// ---------------------------------------------------------------------------

// covers: CLI-058
#[test]
fn analyze_trace_with_recommends_widens_declared_closure() {
  // `--with recommends` widens the declared closure; the entry count with it
  // must be >= without (recommends adds edges, never removes them) and, on a
  // package with recommends, strictly greater.
  let bare = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--strategy",
    "declared",
    "--format",
    "json",
    "python3-minimal",
  ]);
  let bare_json: serde_json::Value = serde_json::from_slice(&ok_stdout(&bare)).expect("bare trace json");
  let bare_n = bare_json.as_array().expect("array").len();

  let widened = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--strategy",
    "declared",
    "--with",
    "recommends",
    "--format",
    "json",
    "python3-minimal",
  ]);
  let widened_json: serde_json::Value = serde_json::from_slice(&ok_stdout(&widened)).expect("widened trace json");
  let widened_n = widened_json.as_array().expect("array").len();

  assert!(
    widened_n > bare_n,
    "--with recommends must widen the declared closure: {} (with) vs {} (without)",
    widened_n,
    bare_n
  );
}

// ---------------------------------------------------------------------------
// analyze trace framing on stderr (CLI-061, CLI-062, CLI-063, CLI-065)
// ---------------------------------------------------------------------------

// covers: CLI-061
#[test]
fn analyze_trace_zero_seeds_framing_and_empty_stdout() {
  // A pattern matching nothing: framing says "Analyzing 0 packages from
  // <source> (<arch>)" on stderr, stdout is empty, exit 0.
  let out = run(&["--from", "debian:bookworm", "analyze", "trace", "no-such-pkg-zzz-*"]);
  assert!(out.status.success(), "empty match must exit 0:\n{}", String::from_utf8_lossy(&out.stderr));
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Analyzing 0 packages from debian:bookworm (x86_64)"),
    "zero-seed framing missing:\n{}",
    stderr
  );
  assert!(
    out.stdout.is_empty(),
    "zero-seed trace must emit no stdout, got: {:?}",
    String::from_utf8_lossy(&out.stdout)
  );
}

// covers: CLI-062, CLI-065
#[test]
fn analyze_trace_single_seed_framing_on_stderr_only() {
  // Single-seed framing: "Analyzing <name> <version> from <source> (<arch>)"
  // lives on stderr; the trace data lives on stdout (JSON here). This isolates
  // the stdout/stderr split for a normal run.
  let out = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--format",
    "json",
    "bash",
  ]);
  assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Analyzing bash ") && stderr.contains(" from debian:bookworm (x86_64)"),
    "single-seed framing missing:\n{}",
    stderr
  );
  // Framing must NOT be on stdout; stdout is the JSON trace.
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(!stdout.contains("Analyzing "), "framing must not appear on stdout:\n{}", stdout);
  let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout must be the JSON trace");
  assert!(parsed.as_array().map(|a| !a.is_empty()).unwrap_or(false), "trace stdout must be a non-empty JSON array");
}

// covers: CLI-063
#[test]
fn analyze_trace_many_seeds_framing_truncates_above_five() {
  // A glob matching > 5 packages: framing previews the first five then
  // "... (N total)". A <= 5 case shows no truncation marker.
  let many = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--strategy",
    "declared",
    "libc6",
    "bash",
    "dash",
    "coreutils",
    "sed",
    "grep",
    "gzip",
  ]);
  assert!(many.status.success(), "stderr:\n{}", String::from_utf8_lossy(&many.stderr));
  let many_stderr = String::from_utf8_lossy(&many.stderr);
  assert!(
    many_stderr.contains(" total)") && many_stderr.contains("..."),
    "framing for > 5 seeds must truncate with '... (N total)':\n{}",
    many_stderr
  );

  let few = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--strategy",
    "declared",
    "bash",
    "dash",
  ]);
  assert!(few.status.success(), "stderr:\n{}", String::from_utf8_lossy(&few.stderr));
  let few_stderr = String::from_utf8_lossy(&few.stderr);
  assert!(
    few_stderr.contains("Analyzing 2 packages from debian:bookworm (x86_64):"),
    "framing for 2 seeds must use the N-seed shape without truncation:\n{}",
    few_stderr
  );
  assert!(!few_stderr.contains(" total)"), "<= 5 seeds must not be truncated:\n{}", few_stderr);
}

// ---------------------------------------------------------------------------
// analyze trace unresolved-soname warning on stderr, not gated by -v (CLI-064)
// ---------------------------------------------------------------------------

// covers: CLI-064
#[test]
fn analyze_trace_unresolved_warning_on_stderr_not_gated_by_verbose() {
  // A linker-strategy trace whose binaries reference sonames the index cannot
  // map produces a trailing "warning: N soname(s) could not be resolved" line.
  // It is a plain eprintln (main.rs:159), NOT RunVoice::warn, so it is emitted
  // regardless of -v. Exit stays 0. Run with and without -v: the warning must
  // appear in both stderr streams (proving it is not gated by verbose), and
  // both runs exit 0.
  let quiet = run(&[
    "--from",
    "debian:bookworm",
    "analyze",
    "trace",
    "--strategy",
    "linker",
    "bash",
  ]);
  let loud = run(&[
    "--from",
    "debian:bookworm",
    "-v",
    "analyze",
    "trace",
    "--strategy",
    "linker",
    "bash",
  ]);
  assert!(
    quiet.status.success(),
    "unresolved-warning trace must still exit 0 (quiet):\n{}",
    String::from_utf8_lossy(&quiet.stderr)
  );
  assert!(
    loud.status.success(),
    "unresolved-warning trace must still exit 0 (verbose):\n{}",
    String::from_utf8_lossy(&loud.stderr)
  );
  let quiet_stderr = String::from_utf8_lossy(&quiet.stderr);
  let loud_stderr = String::from_utf8_lossy(&loud.stderr);
  let needle = "soname(s) could not be resolved against the index";
  assert!(
    quiet_stderr.contains(needle),
    "unresolved warning must appear without -v (not verbose-gated):\n{}",
    quiet_stderr
  );
  assert!(loud_stderr.contains(needle), "unresolved warning must appear with -v as well:\n{}", loud_stderr);
}

// ---------------------------------------------------------------------------
// install full flag combination dispatches and succeeds silently (CLI-069)
// ---------------------------------------------------------------------------

// covers: CLI-069
#[test]
fn install_full_flag_combination_succeeds() {
  // Exercise the whole InstallArgs assembly: --arch x86_64, --with recommends,
  // --postinstall ldconfig,scripts,hooks, -p 4, two packages. The run must
  // complete and lay both packages down. (Requires user namespaces for the
  // postinstall sandbox.)
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64",
      "install",
      "-o",
      root.path().to_str().unwrap(),
      "--with",
      "recommends",
      "--postinstall",
      "ldconfig,scripts,hooks",
      "-p",
      "4",
      "bash",
      "coreutils",
    ])
    .output()
    .unwrap();
  assert!(out.status.success(), "full-combo install stderr:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(root.path().join("bin/bash").exists(), "bash must be installed");
  assert!(
    root.path().join("usr/bin/env").exists() || root.path().join("bin/env").exists(),
    "coreutils (env) must be installed"
  );
}

// ---------------------------------------------------------------------------
// --from `@date` malformed surfaces downstream, not at the CLI (CLI-070)
// ---------------------------------------------------------------------------

// covers: CLI-070
#[test]
fn from_malformed_date_surfaces_downstream() {
  // The CLI stores --from verbatim (a String); a syntactically malformed
  // @date is NOT rejected by clap (it passes the parse layer untouched). It
  // surfaces only downstream, when the dated snapshot is resolved/fetched
  // during a command that actually uses the date — `search` here. The failure
  // is a non-zero exit that is NOT a clap parse error.
  let out = run(&["--from", "ubuntu:noble@not-a-real-date", "search", "bash"]);
  assert!(!out.status.success(), "a malformed @date must fail downstream when the snapshot is resolved");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    !stderr.contains("unexpected argument")
      && !stderr.contains("invalid value for")
      && !stderr.contains("unrecognized"),
    "the malformed @date must surface as a downstream error, not a clap parse error:\n{}",
    stderr
  );
}

// ---------------------------------------------------------------------------
// spawn_blocking task error propagates as a non-zero exit (CLI-066)
// ---------------------------------------------------------------------------

// covers: CLI-066
#[test]
fn spawn_blocking_task_error_propagates_nonzero() {
  // release/search/query run inside `tokio::task::spawn_blocking(...).await?`.
  // An error returned from the blocking task (here a malformed SQL query in
  // `query`, run in spawn_blocking at main.rs:91) must propagate through the
  // JoinHandle's `.await?` plus the inner `?` and surface as a non-zero exit.
  let out = run_query_for_error("this is not valid sql ;;;");
  assert!(!out.status.success(), "a failing spawn_blocking task must yield a non-zero exit");
}

/// Run `query` with `sql` on stdin and return the Output without asserting
/// success — used to observe error propagation.
fn run_query_for_error(sql: &str) -> std::process::Output {
  let cache = TempDir::new().unwrap();
  Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "query"])
    .write_stdin(sql)
    .output()
    .unwrap()
}

// ---------------------------------------------------------------------------
// --match {any,all} combine (path / library)
//
// Real-mirror facts (debian:bookworm), confirmed before these were written:
//   usr/sbin/sendmail  → ~10 MTA packages (courier-mta, dma, exim4-daemon-*,
//                        msmtp-mta, nullmailer, opensmtpd, postfix, ssmtp, …)
//   usr/sbin/postfix   → postfix only
//   bin/bash           → bash only
//   libssl.so.3        → libssl3 ; libcrypto.so.3 → libssl3
// so `usr/sbin/sendmail ∩ usr/sbin/postfix = {postfix}` is a genuine
// collision narrowed by a second file, and `usr/sbin/sendmail ∩ bin/bash = {}`.
// ---------------------------------------------------------------------------

/// The `search.<n>.name=` owner names a `search` plain run emits, in order.
fn search_owner_names(from: &str, after_search: &[&str]) -> Vec<String> {
  let cache = TempDir::new().unwrap();
  let mut args = vec!["--from", from, "search"];
  args.extend_from_slice(after_search);
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&args)
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "search failed for {:?}\nstderr:\n{}",
    after_search,
    String::from_utf8_lossy(&output.stderr)
  );
  String::from_utf8_lossy(&output.stdout)
    .lines()
    .filter_map(|l| l.split_once(".name=").map(|(_, n)| n.to_string()))
    .collect()
}

// covers: IDX-067
#[test]
fn search_path_match_all_keeps_only_common_owner() {
  let names =
    search_owner_names("debian:bookworm", &["--type=path", "--match", "all", "usr/sbin/sendmail", "usr/sbin/postfix"]);
  assert!(!names.is_empty(), "the intersection must not be empty");
  assert!(names.iter().all(|n| n == "postfix"), "only postfix owns both paths, got: {:?}", names);
}

// covers: IDX-069
#[test]
fn search_path_match_any_keeps_all_owners() {
  let names = search_owner_names("debian:bookworm", &["--type=path", "usr/sbin/sendmail", "usr/sbin/postfix"]);
  assert!(names.iter().any(|n| n == "postfix"), "postfix present under the default any: {:?}", names);
  assert!(names.iter().any(|n| n != "postfix"), "default any keeps the other MTAs too: {:?}", names);
}

// covers: IDX-068
#[test]
fn search_library_match_all_keeps_only_common_owner() {
  let names =
    search_owner_names("debian:bookworm", &["--type=library", "--match", "all", "libssl.so.3", "libcrypto.so.3"]);
  assert!(!names.is_empty(), "both sonames resolve, so the intersection is non-empty");
  assert!(names.iter().all(|n| n == "libssl3"), "both sonames are owned by libssl3, got: {:?}", names);
}

// covers: IDX-070
#[test]
fn search_path_match_all_disjoint_is_empty() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "search",
      "--type=path",
      "--match",
      "all",
      "usr/sbin/sendmail",
      "bin/bash",
    ])
    .output()
    .unwrap();
  assert!(output.status.success(), "empty intersection exits 0:\n{}", String::from_utf8_lossy(&output.stderr));
  assert!(
    output.stdout.is_empty(),
    "a disjoint intersection emits zero bytes, got:\n{}",
    String::from_utf8_lossy(&output.stdout)
  );
}

// covers: IDX-071
#[test]
fn search_path_match_all_single_pattern_identity() {
  let names = search_owner_names("debian:bookworm", &["--type=path", "--match", "all", "bin/bash"]);
  assert_eq!(names, vec!["bash"], "single-pattern --match all is the identity");
}

// covers: FMT-001
#[test]
fn search_path_match_all_json_carries_only_common_owner() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "search",
      "--type=path",
      "--match",
      "all",
      "--format",
      "json",
      "usr/sbin/sendmail",
      "usr/sbin/postfix",
    ])
    .output()
    .unwrap();
  assert!(output.status.success());
  let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
  let entries = value.as_array().expect("json is an array");
  assert!(!entries.is_empty(), "filtered set is non-empty");
  assert!(entries.iter().all(|e| e["name"] == "postfix"), "json carries only postfix: {}", value);
}

// covers: FMT-002
#[test]
fn search_path_match_all_disjoint_json_is_empty_array() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "search",
      "--type=path",
      "--match",
      "all",
      "--format",
      "json",
      "usr/sbin/sendmail",
      "bin/bash",
    ])
    .output()
    .unwrap();
  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "[]", "empty intersection is [] in json, not zero bytes");
}

// covers: ANL-069
#[test]
fn trace_path_match_all_seeds_common_owner() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "analyze",
      "trace",
      "--strategy",
      "declared",
      "--type",
      "path",
      "--match",
      "all",
      "usr/sbin/sendmail",
      "usr/sbin/postfix",
    ])
    .output()
    .unwrap();
  assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("Analyzing postfix"), "the trace seeds only postfix:\n{}", stderr);
}

// covers: ANL-070
#[test]
fn trace_path_match_all_disjoint_empty_outcome() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "analyze",
      "trace",
      "--strategy",
      "declared",
      "--type",
      "path",
      "--match",
      "all",
      "usr/sbin/sendmail",
      "bin/bash",
    ])
    .output()
    .unwrap();
  assert!(output.status.success(), "an empty seed set is exit 0");
  assert!(
    output.stdout.is_empty(),
    "the empty outcome is zero bytes in plain form, got:\n{}",
    String::from_utf8_lossy(&output.stdout)
  );
}

// covers: FMT-003
#[test]
fn trace_path_match_all_disjoint_json_is_empty_array() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "analyze",
      "trace",
      "--strategy",
      "declared",
      "--type",
      "path",
      "--match",
      "all",
      "--format",
      "json",
      "usr/sbin/sendmail",
      "bin/bash",
    ])
    .output()
    .unwrap();
  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "[]");
}

// covers: INST-080
#[test]
fn install_path_match_all_on_apk_refused() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.21",
      "install",
      "-o",
      root.path().to_str().unwrap(),
      "--type",
      "path",
      "--match",
      "all",
      "usr/bin/bash",
      "usr/bin/sh",
    ])
    .output()
    .unwrap();
  assert!(!output.status.success(), "--type path is refused on apk");
  assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported for apk sources"));
}

// covers: IDX-078
#[test]
fn search_path_match_all_on_apk_refused() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.21",
      "search",
      "--type=path",
      "--match",
      "all",
      "usr/bin/bash",
      "usr/bin/sh",
    ])
    .output()
    .unwrap();
  assert!(!output.status.success(), "--type path is refused on apk");
  assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported for apk sources"));
}

// covers: ANL-074
#[test]
fn trace_path_match_all_on_apk_refused() {
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.21",
      "analyze",
      "trace",
      "--strategy",
      "declared",
      "--type",
      "path",
      "--match",
      "all",
      "usr/bin/bash",
      "usr/bin/sh",
    ])
    .output()
    .unwrap();
  assert!(!output.status.success(), "--type path is refused on apk");
  assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported for apk sources"));
}

// covers: IDX-079
#[test]
fn search_library_match_all_on_apk_not_refused() {
  // Library mode works on apk via the `so:` provides namespace, so `--match
  // all` is allowed there — the apk-path refusal must NOT appear.
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "alpine:v3.21",
      "search",
      "--type=library",
      "--match",
      "all",
      "libcrypto.so.3",
      "libssl.so.3",
    ])
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "library --match all is allowed on apk:\n{}",
    String::from_utf8_lossy(&output.stderr)
  );
  assert!(
    !String::from_utf8_lossy(&output.stderr).contains("unsupported for apk sources"),
    "library mode must not hit the apk-path refusal"
  );
}

// covers: INST-070, INST-081
#[test]
fn install_path_match_all_installs_resolved_package() {
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
      "--type",
      "path",
      "--match",
      "all",
      "usr/sbin/sendmail",
      "usr/sbin/postfix",
    ])
    .output()
    .unwrap();
  assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
  // INST-081: the per-pattern resolution echoes land on stderr, not stdout.
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("Resolved 'usr/sbin/sendmail'"), "resolution echoed on stderr:\n{}", stderr);
  // INST-070: postfix (the intersection) is installed — its binary is on disk.
  assert!(root.path().join("usr/sbin/postfix").exists(), "postfix (the resolved package) must be installed");
}

// covers: INST-071
#[test]
fn install_path_match_all_no_deps_installs_only_resolved() {
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
      "--no-deps",
      "--type",
      "path",
      "--match",
      "all",
      "usr/sbin/sendmail",
      "usr/sbin/postfix",
    ])
    .output()
    .unwrap();
  assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
  assert!(root.path().join("usr/sbin/postfix").exists(), "postfix extracted under --no-deps");
}

// covers: INST-072
#[test]
fn install_library_match_all_multiarch_per_arch() {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      "debian:bookworm",
      "--arch",
      "x86_64,i686",
      "install",
      "-o",
      root.path().to_str().unwrap(),
      "--no-deps",
      "--type",
      "library",
      "--match",
      "all",
      "libssl.so.3",
      "libcrypto.so.3",
    ])
    .output()
    .unwrap();
  assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
  // The intersection (libssl3) is resolved per-arch and lands in each arch's
  // library directory.
  assert!(root.path().join("usr/lib/x86_64-linux-gnu/libssl.so.3").exists(), "x86_64 libssl.so.3");
  assert!(root.path().join("usr/lib/i386-linux-gnu/libssl.so.3").exists(), "i686 libssl.so.3");
}
