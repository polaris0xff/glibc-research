//! Version-comparator semantics exercised directly through the public
//! `flatroot::version` API. The dpkg and rpm comparators each implement the
//! `VersionCompare` trait (`satisfies` + `compare`); the resolver leans on
//! those two methods for every "is this new enough" / "which is newer" decision
//! it makes. These tests pin down the full ordering semantics — epoch,
//! upstream/release segments, revision tiebreak, the tilde pre-release rule,
//! the dpkg vs rpm operator dialects, and the deliberate leniency on malformed
//! constraints — at the public surface, where the unit tests live behind
//! `pub(crate)` parse helpers an integration test cannot reach.
//!
//! The comparator chooses a verdict from `Ordering`; `compare(a, b)` is the
//! direct ordering and `satisfies(candidate, constraint)` glues an operator to
//! a version. Every assertion below is derived from the documented operator
//! semantics in `src/version/mod.rs` and the ordering rules in
//! `src/version/{dpkg,rpm}.rs`.

use std::cmp::Ordering;

use flatroot::version::{DpkgVersionCompare, RpmVersionCompare, VersionCompare};

// ---------------------------------------------------------------------------
// dpkg comparator — epoch / upstream / revision / tilde (CORE-041)
// ---------------------------------------------------------------------------

// covers: CORE-041
#[test]
fn dpkg_compare_honors_epoch_upstream_revision_and_tilde() {
  let cmp = DpkgVersionCompare;

  // Epoch dominates absolutely: a higher epoch wins regardless of upstream.
  assert_eq!(cmp.compare("2:1.0", "1:999.0"), Ordering::Greater, "epoch must dominate upstream");
  assert_eq!(cmp.compare("1:0", "999"), Ordering::Greater, "any epoch beats epoch-0");

  // Upstream decides next, numerically (not lexicographically).
  assert_eq!(cmp.compare("1.9", "1.10"), Ordering::Less, "1.9 < 1.10 numerically");
  assert_eq!(cmp.compare("2.6.32", "2.6.100"), Ordering::Less, "2.6.32 < 2.6.100 numerically");

  // Revision breaks ties on identical upstream.
  assert_eq!(cmp.compare("1.0-1", "1.0-2"), Ordering::Less, "revision tiebreak: -1 < -2");
  assert_eq!(cmp.compare("1.0-2", "1.0-1"), Ordering::Greater, "revision tiebreak: -2 > -1");

  // Tilde sorts a pre-release below the release it precedes.
  assert_eq!(cmp.compare("1.0~beta1", "1.0"), Ordering::Less, "tilde pre-release sorts below release");
  assert_eq!(cmp.compare("1.0~alpha", "1.0~beta"), Ordering::Less, "tilde then alpha ordering");
  assert_eq!(cmp.compare("1.0~~", "1.0~"), Ordering::Less, "more tildes sort lower");

  // A letter suffix sorts above the bare release (dpkg char weighting).
  assert_eq!(cmp.compare("1.0", "1.0a"), Ordering::Less, "1.0 < 1.0a");
  assert_eq!(cmp.compare("1.0a", "1.0b"), Ordering::Less, "1.0a < 1.0b");

  // Real Debian version strings.
  assert_eq!(cmp.compare("2.36-9+deb12u8", "2.36-9+deb12u9"), Ordering::Less, "deb12u8 < deb12u9");
  assert_eq!(cmp.compare("1:3.6.1-13+deb12u1", "3.6.1-13+deb12u1"), Ordering::Greater, "epoch-1 beats epoch-0");

  // Equality is symmetric and reflexive.
  assert_eq!(cmp.compare("1.0", "1.0"), Ordering::Equal, "identical versions are equal");
  assert_eq!(cmp.compare("1.0", "1.0-0"), cmp.compare("1.0", "1.0-0"), "compare is deterministic");
}

// covers: CORE-041
#[test]
fn dpkg_satisfies_honors_operators_against_constraints() {
  let cmp = DpkgVersionCompare;

  // `>=` boundary.
  assert!(cmp.satisfies("2.0", ">= 1.5"));
  assert!(cmp.satisfies("1.5", ">= 1.5"));
  assert!(!cmp.satisfies("1.4", ">= 1.5"));

  // `<<` strict-less.
  assert!(cmp.satisfies("1.0", "<< 2.0"));
  assert!(!cmp.satisfies("2.0", "<< 2.0"));

  // `<=` boundary includes equality.
  assert!(cmp.satisfies("1.0", "<= 1.0"));
  assert!(!cmp.satisfies("1.1", "<= 1.0"));

  // `>>` strict-greater excludes equality.
  assert!(!cmp.satisfies("1.0", ">> 1.0"));
  assert!(cmp.satisfies("1.1", ">> 1.0"));

  // `=` exact.
  assert!(cmp.satisfies("1.0", "= 1.0"));
  assert!(!cmp.satisfies("1.1", "= 1.0"));

  // Epoch/revision/tilde semantics drive a real constraint outcome.
  assert!(cmp.satisfies("2.36-9+deb12u9", ">= 2.34"), "revision-bearing version clears upstream bar");
  assert!(cmp.satisfies("2.36-9+deb12u9", "<< 3"), "upstream below the next major bound");
  assert!(!cmp.satisfies("1.0~beta1", ">= 1.0"), "tilde pre-release is older than the release");
}

// ---------------------------------------------------------------------------
// rpm comparator — epoch / segments / tilde / single-char ops (CORE-042)
// ---------------------------------------------------------------------------

// covers: CORE-042
#[test]
fn rpm_compare_honors_epoch_segments_and_tilde() {
  let cmp = RpmVersionCompare;

  // Epoch dominates.
  assert_eq!(cmp.compare("2:1.0-1", "1:999.0-1"), Ordering::Greater, "epoch dominates the release tail");
  assert_eq!(cmp.compare("1:0-1", "999-1"), Ordering::Greater, "any epoch beats epoch-0");

  // Release-tail decides next, numerically.
  assert_eq!(cmp.compare("2.0-1", "1.0-1"), Ordering::Greater);
  assert_eq!(cmp.compare("1.10-1", "1.9-1"), Ordering::Greater, "1.10 > 1.9 numerically");
  assert_eq!(cmp.compare("2.6.100-1", "2.6.32-1"), Ordering::Greater, "2.6.100 > 2.6.32 numerically");

  // Release segment as tiebreak.
  assert_eq!(cmp.compare("1.0-2", "1.0-1"), Ordering::Greater);

  // Tilde sorts a pre-release below the release.
  assert_eq!(cmp.compare("1.0~beta", "1.0"), Ordering::Less, "rpm tilde pre-release below release");
  assert_eq!(cmp.compare("1.0~alpha", "1.0~beta"), Ordering::Less, "rpm tilde then alpha ordering");

  // Alpha segments compare lexicographically.
  assert_eq!(cmp.compare("abc", "abd"), Ordering::Less);
  assert_eq!(cmp.compare("abc", "abc"), Ordering::Equal);

  // Real CentOS-style versions.
  assert_eq!(cmp.compare("115.12.0-1.el7.centos", "102.15.1-1.el7.centos"), Ordering::Greater, "115.x > 102.x");
  assert_eq!(cmp.compare("4.2.46-35.el7_9", "4.2.46-34.el7"), Ordering::Greater, "release 35 > release 34");
}

// covers: CORE-042
#[test]
fn rpm_satisfies_accepts_single_char_and_two_char_operators() {
  let cmp = RpmVersionCompare;

  // RPM accepts the single-character `<` / `>` forms the dpkg side does not.
  assert!(cmp.satisfies("2.0", "> 1.0"));
  assert!(!cmp.satisfies("1.0", "> 1.0"));
  assert!(cmp.satisfies("0.9", "< 1.0"));
  assert!(!cmp.satisfies("1.0", "< 1.0"));

  // Two-character forms still work.
  assert!(cmp.satisfies("2.0", ">= 1.0"));
  assert!(cmp.satisfies("1.0", "<= 1.0"));

  // `=` exact, epoch-aware.
  assert!(cmp.satisfies("2:1.0-1", "= 2:1.0-1"));
  assert!(!cmp.satisfies("1:1.0-1", "= 2:1.0-1"), "differing epoch breaks exact match");

  // Tilde pre-release fails a `>=` against the release.
  assert!(!cmp.satisfies("1.0~beta", ">= 1.0"), "rpm tilde pre-release is older than the release");
}

// ---------------------------------------------------------------------------
// Lenient on malformed / empty / operator-only constraints (CORE-043)
// ---------------------------------------------------------------------------

// covers: CORE-043
#[test]
fn both_families_are_lenient_on_malformed_constraints() {
  let dpkg = DpkgVersionCompare;
  let rpm = RpmVersionCompare;

  // An unrecognised operator, an empty constraint, and an operator with no
  // version must all be treated as satisfied so a malformed index entry never
  // blocks the resolver's pick — for both families.
  for cmp in [&dpkg as &dyn VersionCompare, &rpm as &dyn VersionCompare] {
    assert!(cmp.satisfies("1.0", "garbage"), "unrecognised operator is lenient");
    assert!(cmp.satisfies("1.0", ""), "empty constraint is lenient");
    assert!(cmp.satisfies("1.0", ">= "), "operator-only constraint is lenient");
    assert!(cmp.satisfies("anything-at-all", "   "), "whitespace-only constraint is lenient");
  }
}
