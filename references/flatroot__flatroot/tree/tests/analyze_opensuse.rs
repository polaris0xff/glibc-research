//! Integration test for `flatroot analyze` against openSUSE's live
//! mirror.
//!
//! openSUSE Tumbleweed is the rolling release on
//! `download.opensuse.org`. Its repo metadata signs with SHA-512
//! (the only backend in this tree that does), so this test also
//! exercises the SHA-512 path through the fetcher's checksum
//! validation when the linker walk downloads provider archives.

mod analyze;

#[test]
fn analyze_opensuse_coreutils_full_closure() {
  let outcome = analyze::run_analyze("opensuse:tumbleweed", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

#[test]
fn analyze_opensuse_leap_15_5_coreutils_full_closure() {
  let outcome = analyze::run_analyze("opensuse:15.5", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}
