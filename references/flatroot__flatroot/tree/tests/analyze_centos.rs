//! Integration test for `flatroot analyze` against CentOS's live mirror.
//!
//! CentOS 7 is frozen on `vault.centos.org` at its last point release.
//! Its `coreutils` links against glibc-2.17's libc.so.6, which RPM's
//! auto-provides records as `libc.so.6()(64bit)` — the suffix-form
//! branch of the RPM soname dialect.

mod analyze;

#[test]
fn analyze_centos_coreutils_full_closure() {
  let outcome = analyze::run_analyze("centos:7", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

#[test]
fn analyze_centos8_coreutils_full_closure() {
  let outcome = analyze::run_analyze("centos:8", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}
