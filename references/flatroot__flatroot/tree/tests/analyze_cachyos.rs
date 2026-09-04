//! Integration test for `flatroot analyze` against CachyOS's live mirror.
//!
//! CachyOS layers Arch's repos under its own x86_64-v3 rebuilds. The
//! path index ingests `.files` from all five repos so `libc.so.6`
//! resolves regardless of whether `glibc` came from Arch's `core` or
//! a CachyOS-specific override.

mod analyze;

#[test]
fn analyze_cachyos_coreutils_full_closure() {
  let outcome = analyze::run_analyze("cachyos:rolling", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}
