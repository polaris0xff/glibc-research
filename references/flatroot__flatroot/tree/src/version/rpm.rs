//! RPM's epoch/version-release ordering, so an RPM-family candidate
//! ranks the way the upstream tooling would.

use std::cmp::Ordering;

/// An RPM version split into the dominating epoch and the
/// version-release tail, compared in that order.
#[derive(Debug, Clone)]
pub(crate) struct RpmVersion {
  /// Integer epoch (default `0`), the prefix before the first `:`.
  /// Compared numerically and dominates the comparison absolutely.
  epoch: u32,
  /// Version-release tail, everything after the epoch `:`.
  /// Compared by the segment-walking algorithm.
  tail: String,
}

impl RpmVersion {
  /// Splits a flat version string into epoch and tail at the first `:`,
  /// defaulting the epoch to `0` when none is written.
  pub(crate) fn parse(version: &str) -> Self {
    let (epoch, tail) = match version.split_once(':') {
      Some((epoch, rest)) => (epoch.parse().unwrap_or(0), rest.to_string()),
      None => (0, version.to_string()),
    };
    Self { epoch, tail }
  }

  /// RPM's segment walk: punctuation is a boundary, digit runs compare
  /// by magnitude and letter runs alphabetically, and a `~` pre-release
  /// marker ranks below everything including the bare version, so
  /// `1.0~beta < 1.0`.
  fn segments_compare(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
      // Skip separators (`.`, `-`, `_`, …) but not tilde, which carries
      // pre-release meaning of its own.
      while ai.peek().is_some_and(|c| !c.is_alphanumeric() && *c != '~') {
        ai.next();
      }
      while bi.peek().is_some_and(|c| !c.is_alphanumeric() && *c != '~') {
        bi.next();
      }

      // Tilde and end-of-string are decided here; otherwise the segment type
      // is read from a's next character, which the peek proved present.
      // Tilde sorts before everything, end-of-string included, so
      // `1.0~beta` ranks below `1.0`.
      let is_digit = match (ai.peek(), bi.peek()) {
        (Some(&'~'), Some(&'~')) => {
          ai.next();
          bi.next();
          continue;
        }
        (Some(&'~'), _) => return Ordering::Less,
        (_, Some(&'~')) => return Ordering::Greater,
        (None, None) => return Ordering::Equal,
        (None, _) => return Ordering::Less,    // a exhausted first, so older
        (_, None) => return Ordering::Greater, // b exhausted first, so a is newer
        (Some(&c), Some(_)) => c.is_ascii_digit(),
      };

      let seg_a = Self::segment_collect(&mut ai, is_digit);
      let seg_b = Self::segment_collect(&mut bi, is_digit);

      if is_digit {
        // Parse as integers so `9` < `10` rather than losing on text length.
        let na: u64 = seg_a.parse().unwrap_or(0);
        let nb: u64 = seg_b.parse().unwrap_or(0);
        if na != nb {
          return na.cmp(&nb);
        }
      } else if seg_a != seg_b {
        return seg_a.cmp(&seg_b);
      }
    }
  }

  /// Collect one maximal run of same-type characters (digits or letters)
  /// off the front of the iterator. A manual loop rather than `take_while`,
  /// which would consume the failing character and eat the next separator
  /// or tilde.
  fn segment_collect(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, is_digit: bool) -> String {
    let mut segment = String::new();
    while let Some(&c) = chars.peek() {
      let same_type = if is_digit {
        c.is_ascii_digit()
      } else {
        c.is_ascii_alphabetic()
      };
      if !same_type {
        break;
      }
      chars.next();
      segment.push(c);
    }
    segment
  }
}

impl PartialEq for RpmVersion {
  fn eq(&self, other: &Self) -> bool {
    self.cmp(other) == Ordering::Equal
  }
}

impl Eq for RpmVersion {}

impl PartialOrd for RpmVersion {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for RpmVersion {
  fn cmp(&self, other: &Self) -> Ordering {
    // Epoch dominates absolutely; on a tie the version-release tail
    // decides under the segment-walking algorithm.
    match self.epoch.cmp(&other.epoch) {
      Ordering::Equal => Self::segments_compare(&self.tail, &other.tail),
      ord => ord,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::version::{ComparisonOp, RpmVersionCompare, VersionCompare};

  fn rpm_version_newer(a: &str, b: &str) -> bool {
    RpmVersion::parse(a).cmp(&RpmVersion::parse(b)) == Ordering::Greater
  }

  #[test]
  fn simple_ordering() {
    assert!(rpm_version_newer("2.0-1", "1.0-1"));
    assert!(!rpm_version_newer("1.0-1", "2.0-1"));
    assert!(!rpm_version_newer("1.0-1", "1.0-1"));
  }

  #[test]
  fn epoch_dominance() {
    assert!(rpm_version_newer("2:1.0-1", "1:999.0-1"));
    assert!(rpm_version_newer("1:0-1", "999-1"));
  }

  #[test]
  fn release_comparison() {
    assert!(rpm_version_newer("1.0-2", "1.0-1"));
    assert!(!rpm_version_newer("1.0-1", "1.0-2"));
  }

  #[test]
  fn numeric_not_lexicographic() {
    assert!(rpm_version_newer("1.10-1", "1.9-1"));
    assert!(rpm_version_newer("2.6.100-1", "2.6.32-1"));
  }

  #[test]
  fn tilde_sorts_before() {
    assert_eq!(RpmVersion::segments_compare("1.0~beta", "1.0"), Ordering::Less);
    assert_eq!(RpmVersion::segments_compare("1.0~alpha", "1.0~beta"), Ordering::Less);
  }

  #[test]
  fn parse_epoch() {
    let v = RpmVersion::parse("2:1.0-1");
    assert_eq!(v.epoch, 2);
    assert_eq!(v.tail, "1.0-1");
    let v = RpmVersion::parse("1.0-1");
    assert_eq!(v.epoch, 0);
    assert_eq!(v.tail, "1.0-1");
    let v = RpmVersion::parse("0:5.0");
    assert_eq!(v.epoch, 0);
    assert_eq!(v.tail, "5.0");
  }

  #[test]
  fn real_centos_versions() {
    assert!(rpm_version_newer("115.12.0-1.el7.centos", "102.15.1-1.el7.centos"));
    assert!(rpm_version_newer("4.2.46-35.el7_9", "4.2.46-34.el7"));
  }

  #[test]
  fn alpha_segments() {
    assert_eq!(RpmVersion::segments_compare("abc", "abd"), Ordering::Less);
    assert_eq!(RpmVersion::segments_compare("abc", "abc"), Ordering::Equal);
  }

  #[test]
  fn satisfies_single_char_operators() {
    assert!(RpmVersionCompare.satisfies("2.0", "> 1.0"));
    assert!(!RpmVersionCompare.satisfies("1.0", "> 1.0"));
    assert!(RpmVersionCompare.satisfies("0.9", "< 1.0"));
    assert!(!RpmVersionCompare.satisfies("1.0", "< 1.0"));
  }

  #[test]
  fn satisfies_unparseable_is_lenient() {
    assert!(RpmVersionCompare.satisfies("1.0", "garbage"));
    assert!(RpmVersionCompare.satisfies("1.0", ""));
  }

  #[test]
  fn parse_constraint_rpm_single_char() {
    assert_eq!(ComparisonOp::parse("< 1.0", true).map(|(op, _)| op), Some(ComparisonOp::Lt));
    assert_eq!(ComparisonOp::parse("> 1.0", true).map(|(op, _)| op), Some(ComparisonOp::Gt));
    assert!(ComparisonOp::parse("< 1.0", false).is_none());
  }
}
