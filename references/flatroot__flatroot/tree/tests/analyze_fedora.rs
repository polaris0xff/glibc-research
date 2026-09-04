//! Integration test for `flatroot analyze` against Fedora's live mirror.
//!
//! Fedora 42 is the current stable release. `coreutils`'s libc.so.6
//! resolves through RPM's primary.xml provides — modern Fedora
//! publishes the suffix-free form (`libc.so.6`) directly, so the
//! RPM soname dialect's bare-provides branch hits.

mod analyze;

#[test]
fn analyze_fedora_coreutils_full_closure() {
  let outcome = analyze::run_analyze("fedora:42", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

#[test]
fn analyze_fedora44_coreutils_full_closure() {
  let outcome = analyze::run_analyze("fedora:44", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

#[test]
fn analyze_fedora43_coreutils_full_closure() {
  let outcome = analyze::run_analyze("fedora:43", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

#[test]
fn analyze_fedora41_coreutils_full_closure() {
  let outcome = analyze::run_analyze("fedora:41", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

#[test]
fn analyze_fedora40_coreutils_full_closure() {
  let outcome = analyze::run_analyze("fedora:40", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

#[test]
fn analyze_fedora_rawhide_coreutils_full_closure() {
  let outcome = analyze::run_analyze("fedora:rawhide", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

// covers: ANL-050
#[test]
fn analyze_fedora_all_formats_share_one_package_set() {
  // Cross-distro × cross-format contract: the declared + linker +
  // soname-resolution pipeline succeeds for every `--format` on a
  // non-deb distro (the per-distro full-closure tests above force
  // --format=json; this widens format coverage past debian). Both
  // renderings must surface exactly the same package set — a renderer
  // dropping or inventing entries on an RPM closure would be caught
  // here. `bash` is the smallest stable RPM linker target.
  use assert_cmd::Command;
  use serde_json::Value;
  use std::collections::BTreeSet;
  use tempfile::TempDir;

  fn run(format: &str) -> String {
    let cache = TempDir::new().expect("tempdir for flatroot cache");
    let out = Command::cargo_bin("flatroot")
      .unwrap()
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args([
        "--from",
        "fedora:42",
        "--arch",
        "x86_64",
        "analyze",
        "trace",
        "bash",
        "--format",
        format,
      ])
      .output()
      .expect("flatroot binary failed to spawn");
    assert!(
      out.status.success(),
      "fedora analyze --format {format} exited {:?}\nstderr:\n{}",
      out.status.code(),
      String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
  }

  fn names_json(out: &str) -> BTreeSet<String> {
    serde_json::from_str::<Value>(out)
      .expect("json must parse")
      .as_array()
      .expect("json top must be an array")
      .iter()
      .filter_map(|e| e["name"].as_str().map(String::from))
      .collect()
  }

  fn names_plain(out: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in out.lines() {
      let parts: Vec<&str> = line.split_once('=').map(|(p, _)| p).unwrap_or("").split('.').collect();
      if parts.len() == 4 && parts[0] == "analyze" && parts[1] == "trace" && parts[3] == "name" {
        if let Some((_, v)) = line.split_once('=') {
          names.insert(v.to_string());
        }
      }
    }
    names
  }

  let json = run("json");
  let plain = run("plain");

  let json_names = names_json(&json);
  assert!(json_names.contains("bash"), "fedora json must include the seed");
  assert!(json_names.contains("glibc"), "fedora json must include glibc");

  assert_eq!(json_names, names_plain(&plain), "fedora plain renderer must cover the same package set as json");
}
