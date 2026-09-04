//! Integration test for `flatroot analyze` against Alpine's live mirror.
//!
//! Alpine resolves sonames through APKINDEX `p:` lines under the
//! `so:` namespace prefix — `so:libc.musl-x86_64.so.1` from the
//! `musl` package. Alpine's `coreutils` is the busybox bundle on
//! standalone systems, so this test uses `bash` for a stable
//! single-binary linker target.

mod analyze;

#[test]
fn analyze_alpine_bash_full_closure() {
  let outcome = analyze::run_analyze("alpine:edge", "bash");
  analyze::assert_full_closure(&outcome, "musl", "libc.musl-x86_64.so.1");
}

#[test]
fn analyze_alpine_v3_23_bash_full_closure() {
  let outcome = analyze::run_analyze("alpine:v3.23", "bash");
  analyze::assert_full_closure(&outcome, "musl", "libc.musl-x86_64.so.1");
}

#[test]
fn analyze_alpine_v3_22_bash_full_closure() {
  let outcome = analyze::run_analyze("alpine:v3.22", "bash");
  analyze::assert_full_closure(&outcome, "musl", "libc.musl-x86_64.so.1");
}

#[test]
fn analyze_alpine_v3_21_bash_full_closure() {
  let outcome = analyze::run_analyze("alpine:v3.21", "bash");
  analyze::assert_full_closure(&outcome, "musl", "libc.musl-x86_64.so.1");
}

#[test]
fn analyze_alpine_v3_20_bash_full_closure() {
  let outcome = analyze::run_analyze("alpine:v3.20", "bash");
  analyze::assert_full_closure(&outcome, "musl", "libc.musl-x86_64.so.1");
}
