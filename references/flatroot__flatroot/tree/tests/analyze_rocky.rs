//! Integration test for `flatroot analyze` against Rocky Linux's live
//! mirror.
//!
//! Rocky Linux 9 is a RHEL 9 rebuild on `dl.rockylinux.org`. Identical
//! soname-resolution shape to Alma's test; both hit the same RPM
//! provides path.

mod analyze;

#[test]
fn analyze_rocky_coreutils_full_closure() {
  let outcome = analyze::run_analyze("rocky:9", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}
