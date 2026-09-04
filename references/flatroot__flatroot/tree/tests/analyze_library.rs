//! Per-format-family `flatroot analyze trace --type=library`
//! integration tests. Each case verifies that a soname pattern
//! resolves to its owning package, the trace seeds with that package
//! (`reason=target`), and the closure is non-empty. The four cases
//! cover the four provides-encoding shapes the library matcher
//! handles:
//!
//!   - **Debian / Ubuntu** (deb family) — provides table carries
//!     package names (`libssl3`), not sonames; resolution comes from
//!     the path-index leg.
//!   - **Arch Linux** — provides carry `libfoo.so=N-arch`; the
//!     pacman parser splits on `=` at parse time, so the matcher
//!     reads a clean soname out of the provides table.
//!   - **Alpine Linux** — sonames provided under the `so:` namespace
//!     (`so:libc.musl-x86_64.so.1`); the matcher queries that branch
//!     unconditionally.
//!   - **Fedora** (RPM family) — provides carries
//!     `libfoo.so()(64bit)` / `()(32bit)`; suffix-strip yields the
//!     canonical name.

mod analyze;

#[test]
fn analyze_library_debian_libssl() {
  // Deb provides table is package-name-keyed. Path-index leg supplies
  // the soname → owning package mapping.
  let outcome = analyze::run_analyze_args("debian:bookworm", &["--type=library", "libssl.so.3"]);
  analyze::assert_seed_target(&outcome, "libssl3");
}

#[test]
fn analyze_library_arch_libalpm() {
  // Arch's pacman parser splits `name=version-arch` provides on `=`
  // at parse time, so the stored `provides.name` is the clean
  // soname and an exact-match pattern resolves to the owning
  // package via the provides leg.
  let outcome = analyze::run_analyze_args("arch:rolling", &["--type=library", "libalpm.so"]);
  analyze::assert_seed_target(&outcome, "pacman");
}

#[test]
fn analyze_library_alpine_musl() {
  // Alpine sonames live exclusively under the `so:` namespace.
  // `library::glob` queries with both raw and `so:`-prefixed forms;
  // here only the prefixed form matches.
  let outcome = analyze::run_analyze_args("alpine:edge", &["--type=library", "libc.musl-x86_64.so.1"]);
  analyze::assert_seed_target(&outcome, "musl");
}

#[test]
fn analyze_library_fedora_libc() {
  // RPM mangled provides — `libc.so.6()(64bit)`. SQL GLOB on the raw
  // text needs a trailing wildcard to reach past the `()(64bit)`
  // suffix; the matcher's shape-check then strips the decoration to
  // `libc.so.6`.
  let outcome = analyze::run_analyze_args("fedora:42", &["--type=library", "libc.so.6*"]);
  analyze::assert_seed_target(&outcome, "glibc");
}
