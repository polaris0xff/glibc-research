//! Debian's epoch/upstream/revision version ordering, so a candidate
//! ranks the same way dpkg's own comparison would.

use std::cmp::Ordering;

/// A Debian version split into the three parts its ordering compares
/// in turn: the dominating epoch, the upstream version, and the
/// packaging revision.
#[derive(Debug, Clone)]
pub(crate) struct DebianVersion {
  /// Integer epoch (default `0`), the prefix before the first `:`.
  /// Dominates the comparison absolutely.
  epoch: u32,
  /// Upstream version, between the epoch and the final hyphen.
  /// Compared by the fragment algorithm.
  upstream: String,
  /// Packaging revision after the last hyphen (empty when none).
  /// The tiebreaker once epoch and upstream match.
  revision: String,
}

impl DebianVersion {
  /// Splits a flat version string into epoch, upstream, and revision
  /// at the first `:` and the last `-`.
  pub(crate) fn parse(version: &str) -> Self {
    let (epoch, rest) = match version.find(':') {
      Some(i) => (version[..i].parse::<u32>().unwrap_or(0), &version[i + 1..]),
      None => (0, version),
    };

    // `rfind` of the last hyphen — upstream segments can themselves
    // contain `-`, so the right-most occurrence is the one that opens
    // the revision tail.
    let (upstream, revision) = match rest.rfind('-') {
      Some(i) => (rest[..i].to_string(), rest[i + 1..].to_string()),
      None => (rest.to_string(), String::new()),
    };

    Self {
      epoch,
      upstream,
      revision,
    }
  }

  /// Compares one version part under dpkg's rule: digit runs compared
  /// as numbers, the surrounding text character by character under
  /// `char_order`, so a `~` pre-release ranks below the release it
  /// precedes.
  fn fragment_compare(a: &str, b: &str) -> Ordering {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut ai = 0;
    let mut bi = 0;

    loop {
      // Drain non-digit runs in lockstep, comparing each pair under
      // dpkg's character sort weight. `None` represents "ran out of
      // non-digits" (hit a digit or end-of-string) and is itself a
      // valid sort position via `char_order`.
      loop {
        let ac = if ai < a.len() && !a[ai].is_ascii_digit() {
          Some(a[ai])
        } else {
          None
        };
        let bc = if bi < b.len() && !b[bi].is_ascii_digit() {
          Some(b[bi])
        } else {
          None
        };

        if ac.is_none() && bc.is_none() {
          break;
        }

        match Self::char_order(ac).cmp(&Self::char_order(bc)) {
          Ordering::Equal => {}
          ord => return ord,
        }

        if ac.is_some() {
          ai += 1;
        }
        if bc.is_some() {
          bi += 1;
        }
      }

      if ai >= a.len() && bi >= b.len() {
        return Ordering::Equal;
      }

      // Numeric run — comparison happens as `u64`, not as text, so the
      // shorter run never wins on lexicographic grounds (`9` vs `10`).
      let an = Self::number_extract(&a, &mut ai);
      let bn = Self::number_extract(&b, &mut bi);
      match an.cmp(&bn) {
        Ordering::Equal => {}
        ord => return ord,
      }
    }
  }

  /// dpkg's per-character sort weight: end-of-part at 0, `~` below it,
  /// letters at their plain value, every other symbol above the
  /// letters, so `1.0a` ranks under `1.0+`.
  fn char_order(c: Option<char>) -> i32 {
    match c {
      None => 0,
      Some('~') => -1,
      Some(c) if c.is_ascii_alphabetic() => c as i32,
      Some(c) => c as i32 + 256,
    }
  }

  /// Reads a digit run as its numeric value and advances `pos` past
  /// it, so a longer number never loses to a shorter one on text
  /// length.
  fn number_extract(chars: &[char], pos: &mut usize) -> u64 {
    let mut n: u64 = 0;
    while *pos < chars.len() && chars[*pos].is_ascii_digit() {
      n = n * 10 + (chars[*pos] as u64 - '0' as u64);
      *pos += 1;
    }
    n
  }
}

impl PartialEq for DebianVersion {
  fn eq(&self, other: &Self) -> bool {
    self.cmp(other) == Ordering::Equal
  }
}

impl Eq for DebianVersion {}

impl PartialOrd for DebianVersion {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for DebianVersion {
  fn cmp(&self, other: &Self) -> Ordering {
    // Epoch dominates absolutely; upstream decides next under the
    // fragment algorithm; revision breaks ties on identical upstreams.
    match self.epoch.cmp(&other.epoch) {
      Ordering::Equal => {}
      ord => return ord,
    }
    match Self::fragment_compare(&self.upstream, &other.upstream) {
      Ordering::Equal => {}
      ord => return ord,
    }
    Self::fragment_compare(&self.revision, &other.revision)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::version::{ComparisonOp, DpkgVersionCompare, VersionCompare};

  #[test]
  fn parse_simple() {
    let v = DebianVersion::parse("1.2.3");
    assert_eq!(v.epoch, 0);
    assert_eq!(v.upstream, "1.2.3");
    assert_eq!(v.revision, "");
  }

  #[test]
  fn parse_with_epoch() {
    let v = DebianVersion::parse("2:1.0.0-4");
    assert_eq!(v.epoch, 2);
    assert_eq!(v.upstream, "1.0.0");
    assert_eq!(v.revision, "4");
  }

  #[test]
  fn parse_with_revision() {
    let v = DebianVersion::parse("1.0-3");
    assert_eq!(v.epoch, 0);
    assert_eq!(v.upstream, "1.0");
    assert_eq!(v.revision, "3");
  }

  #[test]
  fn parse_epoch_only() {
    let v = DebianVersion::parse("1:0");
    assert_eq!(v.epoch, 1);
    assert_eq!(v.upstream, "0");
    assert_eq!(v.revision, "");
  }

  #[test]
  fn parse_complex_revision() {
    let v = DebianVersion::parse("2.36-9+deb12u4");
    assert_eq!(v.epoch, 0);
    assert_eq!(v.upstream, "2.36");
    assert_eq!(v.revision, "9+deb12u4");
  }

  #[test]
  fn compare_simple() {
    assert!(DebianVersion::parse("1.0") < DebianVersion::parse("1.1"));
    assert!(DebianVersion::parse("1.1") < DebianVersion::parse("1.2"));
    assert_eq!(DebianVersion::parse("1.0"), DebianVersion::parse("1.0"));
  }

  #[test]
  fn compare_epoch_dominance() {
    assert!(DebianVersion::parse("2:1.0") > DebianVersion::parse("1:999.0"));
    assert!(DebianVersion::parse("1:0") > DebianVersion::parse("999"));
  }

  #[test]
  fn compare_revision() {
    assert!(DebianVersion::parse("1.0-1") < DebianVersion::parse("1.0-2"));
    assert!(DebianVersion::parse("1.0-1") < DebianVersion::parse("1.1-0"));
  }

  #[test]
  fn compare_tilde() {
    assert!(DebianVersion::parse("1.0~beta1") < DebianVersion::parse("1.0"));
    assert!(DebianVersion::parse("1.0~alpha") < DebianVersion::parse("1.0~beta"));
    assert!(DebianVersion::parse("1.0~~") < DebianVersion::parse("1.0~"));
  }

  #[test]
  fn compare_numeric_not_lexicographic() {
    assert!(DebianVersion::parse("1.9") < DebianVersion::parse("1.10"));
    assert!(DebianVersion::parse("2.6.32") < DebianVersion::parse("2.6.100"));
  }

  #[test]
  fn compare_alpha_suffix() {
    assert!(DebianVersion::parse("1.0") < DebianVersion::parse("1.0a"));
    assert!(DebianVersion::parse("1.0a") < DebianVersion::parse("1.0b"));
  }

  /// Vectors verified against `dpkg --compare-versions` in debian:bookworm:
  /// letters rank below non-letter punctuation, and `+` ranks above
  /// end-of-part but below `.` and below any letter.
  #[test]
  fn compare_letters_before_punctuation() {
    assert!(DebianVersion::parse("1.0a") < DebianVersion::parse("1.0+"));
    assert!(DebianVersion::parse("1.0a") < DebianVersion::parse("1.0."));
    assert!(DebianVersion::parse("1.0") < DebianVersion::parse("1.0+dfsg"));
    assert!(DebianVersion::parse("1.0+1") < DebianVersion::parse("1.0.1"));
  }

  #[test]
  fn compare_real_debian_versions() {
    assert!(DebianVersion::parse("5.2.15-2+b6") < DebianVersion::parse("5.2.15-2+b7"));
    assert!(DebianVersion::parse("2.36-9+deb12u8") < DebianVersion::parse("2.36-9+deb12u9"));
    assert!(DebianVersion::parse("1:3.6.1-13+deb12u1") > DebianVersion::parse("3.6.1-13+deb12u1"));
  }

  #[test]
  fn compare_empty_revision_equals_no_revision() {
    let a = DebianVersion::parse("1.0");
    let b = DebianVersion {
      epoch: 0,
      upstream: "1.0".to_string(),
      revision: String::new(),
    };
    assert_eq!(a, b);
  }

  #[test]
  fn parse_constraint_operators() {
    assert_eq!(ComparisonOp::parse("<< 1.0", false).map(|(op, _)| op), Some(ComparisonOp::Lt));
    assert_eq!(ComparisonOp::parse("<= 1.0", false).map(|(op, _)| op), Some(ComparisonOp::Le));
    assert_eq!(ComparisonOp::parse("= 1.0", false).map(|(op, _)| op), Some(ComparisonOp::Eq));
    assert_eq!(ComparisonOp::parse(">= 1.0", false).map(|(op, _)| op), Some(ComparisonOp::Ge));
    assert_eq!(ComparisonOp::parse(">> 1.0", false).map(|(op, _)| op), Some(ComparisonOp::Gt));
  }

  #[test]
  fn parse_constraint_invalid() {
    assert!(ComparisonOp::parse("", false).is_none());
    assert!(ComparisonOp::parse("garbage", false).is_none());
  }

  #[test]
  fn satisfies_ge() {
    assert!(DpkgVersionCompare.satisfies("2.0", ">= 1.5"));
    assert!(DpkgVersionCompare.satisfies("1.5", ">= 1.5"));
    assert!(!DpkgVersionCompare.satisfies("1.4", ">= 1.5"));
  }

  #[test]
  fn satisfies_lt() {
    assert!(DpkgVersionCompare.satisfies("1.0", "<< 2.0"));
    assert!(!DpkgVersionCompare.satisfies("2.0", "<< 2.0"));
    assert!(!DpkgVersionCompare.satisfies("3.0", "<< 2.0"));
  }

  #[test]
  fn satisfies_eq() {
    assert!(DpkgVersionCompare.satisfies("1.0", "= 1.0"));
    assert!(!DpkgVersionCompare.satisfies("1.1", "= 1.0"));
  }

  #[test]
  fn satisfies_le_boundary() {
    assert!(DpkgVersionCompare.satisfies("1.0", "<= 1.0"));
    assert!(DpkgVersionCompare.satisfies("0.9", "<= 1.0"));
    assert!(!DpkgVersionCompare.satisfies("1.1", "<= 1.0"));
  }

  #[test]
  fn satisfies_gt_boundary() {
    assert!(!DpkgVersionCompare.satisfies("1.0", ">> 1.0"));
    assert!(DpkgVersionCompare.satisfies("1.1", ">> 1.0"));
    assert!(!DpkgVersionCompare.satisfies("0.9", ">> 1.0"));
  }

  #[test]
  fn satisfies_unparseable_is_lenient() {
    assert!(DpkgVersionCompare.satisfies("1.0", "garbage"));
    assert!(DpkgVersionCompare.satisfies("1.0", ""));
    assert!(DpkgVersionCompare.satisfies("1.0", ">= "));
  }

  #[test]
  fn satisfies_real_debian_constraint() {
    assert!(DpkgVersionCompare.satisfies("2.36-9+deb12u9", ">= 2.34"));
    assert!(DpkgVersionCompare.satisfies("2.36-9+deb12u9", "<< 3"));
  }
}
