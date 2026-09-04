//! Integration test for `flatroot analyze` against Debian's live mirror.
//!
//! Asserts that analyze of `coreutils` from `debian:bookworm` produces
//! a libc6 row with `libc.so.6` in `sonames_provided` and a
//! `DeclaredLinker` reason — the canonical declared+linker positive
//! case for the deb family.

mod analyze;

#[test]
fn analyze_debian_coreutils_full_closure() {
  let outcome = analyze::run_analyze("debian:bookworm", "coreutils");
  analyze::assert_full_closure(&outcome, "libc6", "libc.so.6");
}

#[test]
fn analyze_debian_trixie_coreutils_full_closure() {
  let outcome = analyze::run_analyze("debian:trixie", "coreutils");
  analyze::assert_full_closure(&outcome, "libc6", "libc.so.6");
}

#[test]
fn analyze_debian_forky_coreutils_full_closure() {
  let outcome = analyze::run_analyze("debian:forky", "coreutils");
  analyze::assert_full_closure(&outcome, "libc6", "libc.so.6");
}

#[test]
fn analyze_debian_bullseye_coreutils_full_closure() {
  let outcome = analyze::run_analyze("debian:bullseye", "coreutils");
  analyze::assert_full_closure(&outcome, "libc6", "libc.so.6");
}
