//! Integration test for `flatroot analyze` against Ubuntu's live mirror.
//!
//! Same shape as the Debian test: declared+linker resolution of
//! `libc.so.6` to `libc6` confirms the deb-family path-index code
//! path works on both backends.

mod analyze;

#[test]
fn analyze_ubuntu_coreutils_full_closure() {
  let outcome = analyze::run_analyze("ubuntu:noble", "coreutils");
  analyze::assert_full_closure(&outcome, "libc6", "libc.so.6");
}

#[test]
fn analyze_ubuntu_jammy_coreutils_full_closure() {
  let outcome = analyze::run_analyze("ubuntu:jammy", "coreutils");
  analyze::assert_full_closure(&outcome, "libc6", "libc.so.6");
}

#[test]
fn analyze_ubuntu_bionic_coreutils_full_closure() {
  let outcome = analyze::run_analyze("ubuntu:bionic", "coreutils");
  analyze::assert_full_closure(&outcome, "libc6", "libc.so.6");
}

#[test]
fn analyze_ubuntu_xenial_coreutils_full_closure() {
  let outcome = analyze::run_analyze("ubuntu:xenial", "coreutils");
  analyze::assert_full_closure(&outcome, "libc6", "libc.so.6");
}
