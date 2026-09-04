//! Integration test for `flatroot analyze` against Arch's live mirror.
//!
//! Arch is the path-index path that Phase 8 added: `glibc` ships no
//! `%PROVIDES%` for `libc.so` at all, so the only signal that
//! resolves `libc.so.6` to `glibc` is the `.files` repo ingested
//! into the path index. This test confirms that ingest works end to
//! end.

mod analyze;

#[test]
fn analyze_arch_coreutils_full_closure() {
  let outcome = analyze::run_analyze("arch:rolling", "coreutils");
  analyze::assert_full_closure(&outcome, "glibc", "libc.so.6");
}

#[test]
fn analyze_arch_multilib_lib32_glibc_target() {
  let outcome = analyze::run_analyze("arch:multilib", "lib32-glibc");
  analyze::assert_seed_target(&outcome, "lib32-glibc");
}
