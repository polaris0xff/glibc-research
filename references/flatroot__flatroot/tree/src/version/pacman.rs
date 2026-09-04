//! libalpm's `vercmp` ordering, so an Arch or CachyOS candidate ranks
//! the way pacman would.

use std::cmp::Ordering;

/// A pacman version split into the dominating epoch, the upstream
/// version text, and the optional packaging release. The release is
/// held apart because libalpm consults it only when both sides carry
/// one — `1.0-1` and `1.0` rank equal.
#[derive(Debug, Clone)]
pub(crate) struct PacmanVersion {
  /// Integer epoch (default `0`), the digits before the first `:`.
  /// Compared numerically and dominates the comparison absolutely.
  epoch: u64,
  /// Upstream version, between the epoch and the last hyphen.
  /// Compared by the segment-walking algorithm.
  ver: String,
  /// Packaging release after the last hyphen, when present. Compared
  /// by the same walk, but only when both sides carry one, mirroring
  /// `alpm_pkg_vercmp` (`1.0-1` ranks equal to `1.0`).
  rel: Option<String>,
}

impl PacmanVersion {
  /// Splits a flat version string into epoch, upstream, and release at
  /// the first `:` and the last `-` (upstream may itself hold hyphens).
  pub(crate) fn parse(version: &str) -> Self {
    let (epoch, rest) = match version.split_once(':') {
      Some((epoch, rest)) => (epoch.parse().unwrap_or(0), rest),
      None => (0, version),
    };
    let (ver, rel) = match rest.rfind('-') {
      Some(i) => (rest[..i].to_string(), Some(rest[i + 1..].to_string())),
      None => (rest.to_string(), None),
    };
    Self { epoch, ver, rel }
  }

  /// libalpm's rpmvercmp walk: separators delimit maximal digit or
  /// letter runs; digit runs compare as magnitudes (leading zeros
  /// stripped, longer run wins, then lexically), letter runs compare
  /// lexically, and a digit run always outranks a letter run. The
  /// end-of-string rule is libalpm's own: when one side runs out,
  /// leftover *letters* mark the longer side as older (`1.0a` < `1.0`)
  /// while any other leftover marks it as newer (`1.5.0` > `1.5`).
  fn segments_compare(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
      while ai.peek().is_some_and(|c| !c.is_alphanumeric()) {
        ai.next();
      }
      while bi.peek().is_some_and(|c| !c.is_alphanumeric()) {
        bi.next();
      }

      // One or both sides exhausted ends the walk with libalpm's
      // end-of-string rule: leftover letters mark their side older, any
      // other leftover marks it newer.
      let (ca, cb) = match (ai.peek().copied(), bi.peek().copied()) {
        (Some(ca), Some(cb)) => (ca, cb),
        (None, None) => return Ordering::Equal,
        (Some(c), None) => {
          return if c.is_ascii_alphabetic() {
            Ordering::Less
          } else {
            Ordering::Greater
          };
        }
        (None, Some(c)) => {
          return if c.is_ascii_alphabetic() {
            Ordering::Greater
          } else {
            Ordering::Less
          };
        }
      };

      // The segment type is taken from `a`; a type mismatch decides the
      // comparison outright — a digit run always outranks a letter run.
      let is_digit = ca.is_ascii_digit();
      if is_digit != cb.is_ascii_digit() {
        return if is_digit { Ordering::Greater } else { Ordering::Less };
      }

      let seg_a = Self::segment_collect(&mut ai, is_digit);
      let seg_b = Self::segment_collect(&mut bi, is_digit);

      let ord = if is_digit {
        Self::digits_compare(&seg_a, &seg_b)
      } else {
        seg_a.cmp(&seg_b)
      };
      if ord != Ordering::Equal {
        return ord;
      }
    }
  }

  /// Digit runs compare as magnitudes without ever parsing into a fixed
  /// width: leading zeros are stripped, a longer remaining run is the
  /// larger number, and equal lengths decide lexically.
  fn digits_compare(a: &str, b: &str) -> Ordering {
    let a = a.trim_start_matches('0');
    let b = b.trim_start_matches('0');
    match a.len().cmp(&b.len()) {
      Ordering::Equal => a.cmp(b),
      ord => ord,
    }
  }

  /// Collect one maximal run of same-type characters off the front of
  /// the iterator, leaving the first non-matching character in place.
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

impl PartialEq for PacmanVersion {
  fn eq(&self, other: &Self) -> bool {
    self.cmp(other) == Ordering::Equal
  }
}

impl Eq for PacmanVersion {}

impl PartialOrd for PacmanVersion {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for PacmanVersion {
  fn cmp(&self, other: &Self) -> Ordering {
    // Epoch dominates; the upstream text decides next; the release
    // breaks the tie only when both sides carry one — exactly
    // `alpm_pkg_vercmp`. Index rows always carry a `-rel`, so the
    // release-less form only ever meets a full one inside constraint
    // checks, where the pairwise verdict is all that is consulted.
    match self.epoch.cmp(&other.epoch) {
      Ordering::Equal => {}
      ord => return ord,
    }
    match Self::segments_compare(&self.ver, &other.ver) {
      Ordering::Equal => {}
      ord => return ord,
    }
    match (&self.rel, &other.rel) {
      (Some(a), Some(b)) => Self::segments_compare(a, b),
      _ => Ordering::Equal,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::version::{PacmanVersionCompare, VersionCompare};

  fn ord(a: &str, b: &str) -> Ordering {
    PacmanVersion::parse(a).cmp(&PacmanVersion::parse(b))
  }

  /// Every vector verified against `vercmp(8)` in an archlinux container.
  #[test]
  fn vercmp_ground_truth() {
    assert_eq!(ord("1.0", "1.0"), Ordering::Equal);
    assert_eq!(ord("1.0", "2.0"), Ordering::Less);
    assert_eq!(ord("1.0-1", "1.0-2"), Ordering::Less);
    assert_eq!(ord("1.0-1", "1.0"), Ordering::Equal);
    assert_eq!(ord("1.0a", "1.0"), Ordering::Less);
    assert_eq!(ord("1.0", "1.0a"), Ordering::Greater);
    assert_eq!(ord("1.0rc1", "1.0"), Ordering::Less);
    assert_eq!(ord("1.0.1", "1.0"), Ordering::Greater);
    assert_eq!(ord("1:1.0", "2.0"), Ordering::Greater);
    assert_eq!(ord("01.0", "1.0"), Ordering::Equal);
    assert_eq!(ord("1.0.a", "1.0.1"), Ordering::Less);
    assert_eq!(ord("1.0_1", "1.0.1"), Ordering::Equal);
    assert_eq!(ord("1.5.0", "1.5"), Ordering::Greater);
    assert_eq!(ord("1.0b", "1.0a"), Ordering::Greater);
    assert_eq!(ord("2.0a", "2.0.1"), Ordering::Less);
  }

  #[test]
  fn parse_epoch_and_release() {
    let v = PacmanVersion::parse("2:1.0-1");
    assert_eq!(v.epoch, 2);
    assert_eq!(v.ver, "1.0");
    assert_eq!(v.rel.as_deref(), Some("1"));
    let v = PacmanVersion::parse("1.0");
    assert_eq!(v.epoch, 0);
    assert_eq!(v.rel, None);
  }

  #[test]
  fn digits_compare_magnitude() {
    assert_eq!(ord("1.10", "1.9"), Ordering::Greater);
    assert_eq!(ord("2.6.100", "2.6.32"), Ordering::Greater);
  }

  #[test]
  fn satisfies_single_char_operators() {
    assert!(PacmanVersionCompare.satisfies("2.0-1", ">1.0"));
    assert!(!PacmanVersionCompare.satisfies("1.0-1", ">1.0"));
    assert!(PacmanVersionCompare.satisfies("0.9-1", "<1.0"));
    assert!(PacmanVersionCompare.satisfies("1.0-2", "=1.0"));
    assert!(PacmanVersionCompare.satisfies("1.0-1", ">=1.0"));
    assert!(!PacmanVersionCompare.satisfies("0.9-1", ">=1.0"));
  }

  #[test]
  fn satisfies_unparseable_is_lenient() {
    assert!(PacmanVersionCompare.satisfies("1.0-1", "garbage"));
    assert!(PacmanVersionCompare.satisfies("1.0-1", ""));
  }
}
