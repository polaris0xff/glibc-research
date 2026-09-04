//! Integration test for `flatroot analyze` against AlmaLinux's live
//! mirror.
//!
//! AlmaLinux 9 is a RHEL 9 rebuild on `repo.almalinux.org`. Same
//! soname-resolution shape as Fedora — primary.xml provides covers
//! `libc.so.6` directly.

mod analyze;

#[test]
fn analyze_alma_coreutils_full_closure() {
  let outcome = analyze::run_analyze("alma:9", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}
