//! apk-tools' version ordering, so an Alpine candidate ranks the way
//! apk would and the `~` fuzzy constraint gets its component-prefix
//! meaning.

use std::cmp::Ordering;

/// An Alpine version split into the four parts apk compares in order:
/// the dotted numeric components, the optional trailing letter, the
/// ranked suffix chain, and the packaging revision. The split is what
/// makes `1.0_rc1 < 1.0 < 1.0a < 1.0_p1` come out apk's way.
#[derive(Debug, Clone)]
pub(crate) struct ApkVersion {
  /// Dotted numeric components, kept as written so a leading zero can
  /// fall back to lexical comparison the way apk treats fractions.
  components: Vec<String>,
  /// Single trailing letter after the last numeric component
  /// (`1.0a`). Ranks above the bare version, compared by character.
  letter: Option<char>,
  /// `_word[N]` suffixes in order, each as its rank (pre-release
  /// negative, post-release positive) and numeric tail (default `0`).
  suffixes: Vec<(i8, u64)>,
  /// `-rN` packaging revision (default `0` when absent).
  revision: u64,
}

impl ApkVersion {
  /// apk-tools' ranked suffix table, pre-release below zero and
  /// post-release above: `_alpha < _beta < _pre < _rc <` bare
  /// `< _cvs < _svn < _git < _hg < _p`.
  fn suffix_rank(word: &str) -> Option<i8> {
    match word {
      "alpha" => Some(-4),
      "beta" => Some(-3),
      "pre" => Some(-2),
      "rc" => Some(-1),
      "cvs" => Some(1),
      "svn" => Some(2),
      "git" => Some(3),
      "hg" => Some(4),
      "p" => Some(5),
      _ => None,
    }
  }

  /// Reads a flat version string into its four layers in apk's
  /// grammar order — numeric components, optional letter, `_suffix`
  /// chain, `-rN` revision — anything unrecognised ending the layer it
  /// interrupts, apk's own lenient reading.
  pub(crate) fn parse(version: &str) -> Self {
    let (body, revision) = match version.rsplit_once("-r") {
      Some((body, digits)) if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {
        (body, digits.parse().unwrap_or(0))
      }
      _ => (version, 0),
    };

    let (body, suffixes) = Self::suffixes_split(body);

    let mut components: Vec<String> = Vec::new();
    let mut letter: Option<char> = None;
    for (i, part) in body.split('.').enumerate() {
      let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
      let rest = &part[digits.len()..];
      if digits.is_empty() && i > 0 {
        break;
      }
      components.push(digits);
      letter = rest.chars().next().filter(|c| c.is_ascii_alphabetic());
      if !rest.is_empty() {
        break;
      }
    }

    Self {
      components,
      letter,
      suffixes,
      revision,
    }
  }

  /// Split the `_suffix` chain off the back of the version body. The
  /// chain is read from the first `_` whose word is in the ranked
  /// table; an underscore introducing an unranked word is not a
  /// suffix and stays part of the body.
  fn suffixes_split(body: &str) -> (&str, Vec<(i8, u64)>) {
    let Some(idx) = body.find('_') else {
      return (body, Vec::new());
    };
    let mut suffixes = Vec::new();
    for raw in body[idx + 1..].split('_') {
      let word: String = raw.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
      let Some(rank) = Self::suffix_rank(&word) else {
        return (body, Vec::new());
      };
      let number: u64 = raw[word.len()..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0);
      suffixes.push((rank, number));
    }
    (&body[..idx], suffixes)
  }

  /// Two numeric components compare as magnitudes unless either is
  /// written with a leading zero, where apk reads them as fractions
  /// and compares the digit text lexically.
  fn component_compare(a: &str, b: &str) -> Ordering {
    let fractional = (a.len() > 1 && a.starts_with('0')) || (b.len() > 1 && b.starts_with('0'));
    if fractional {
      return a.cmp(b);
    }
    let a = a.trim_start_matches('0');
    let b = b.trim_start_matches('0');
    match a.len().cmp(&b.len()) {
      Ordering::Equal => a.cmp(b),
      ord => ord,
    }
  }

  /// The suffix chains compare pairwise by rank then number, with a
  /// missing suffix standing at rank zero — the bare release — so a
  /// pre-release chain ranks below no chain at all and a post-release
  /// chain above it.
  fn suffixes_compare(a: &[(i8, u64)], b: &[(i8, u64)]) -> Ordering {
    let len = a.len().max(b.len());
    for i in 0..len {
      let (rank_a, num_a) = a.get(i).copied().unwrap_or((0, 0));
      let (rank_b, num_b) = b.get(i).copied().unwrap_or((0, 0));
      match rank_a.cmp(&rank_b) {
        Ordering::Equal => {}
        ord => return ord,
      }
      match num_a.cmp(&num_b) {
        Ordering::Equal => {}
        ord => return ord,
      }
    }
    Ordering::Equal
  }

  /// The `~` fuzzy constraint: the candidate matches when its numeric
  /// components equal the constraint's for as many components as the
  /// constraint spells out — `~2.1` accepts every `2.1*` release.
  pub(crate) fn fuzzy_matches(&self, prefix: &ApkVersion) -> bool {
    if prefix.components.len() > self.components.len() {
      return false;
    }
    self
      .components
      .iter()
      .zip(prefix.components.iter())
      .all(|(a, b)| Self::component_compare(a, b) == Ordering::Equal)
  }
}

impl PartialEq for ApkVersion {
  fn eq(&self, other: &Self) -> bool {
    self.cmp(other) == Ordering::Equal
  }
}

impl Eq for ApkVersion {}

impl PartialOrd for ApkVersion {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for ApkVersion {
  fn cmp(&self, other: &Self) -> Ordering {
    // Components first — and a longer component list ranks newer even
    // when the extra component is zero (`2.1 < 2.1.0`), which is why
    // the lists compare positionally with absence ranking lowest.
    let len = self.components.len().max(other.components.len());
    for i in 0..len {
      let ord = match (self.components.get(i), other.components.get(i)) {
        (Some(a), Some(b)) => Self::component_compare(a, b),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
      };
      if ord != Ordering::Equal {
        return ord;
      }
    }

    match self.letter.cmp(&other.letter) {
      Ordering::Equal => {}
      ord => return ord,
    }

    match Self::suffixes_compare(&self.suffixes, &other.suffixes) {
      Ordering::Equal => {}
      ord => return ord,
    }

    self.revision.cmp(&other.revision)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::version::{ApkVersionCompare, VersionCompare};

  fn ord(a: &str, b: &str) -> Ordering {
    ApkVersion::parse(a).cmp(&ApkVersion::parse(b))
  }

  /// Every vector verified against `apk version -t` in an alpine container.
  #[test]
  fn apk_ground_truth() {
    assert_eq!(ord("1.0", "1.0"), Ordering::Equal);
    assert_eq!(ord("1.0_alpha1", "1.0"), Ordering::Less);
    assert_eq!(ord("1.0_rc1", "1.0"), Ordering::Less);
    assert_eq!(ord("1.0_p1", "1.0"), Ordering::Greater);
    assert_eq!(ord("1.0_alpha", "1.0_beta"), Ordering::Less);
    assert_eq!(ord("1.0_beta", "1.0_pre"), Ordering::Less);
    assert_eq!(ord("1.0_pre", "1.0_rc"), Ordering::Less);
    assert_eq!(ord("1.0a", "1.0"), Ordering::Greater);
    assert_eq!(ord("1.0b", "1.0a"), Ordering::Greater);
    assert_eq!(ord("1.0-r1", "1.0-r2"), Ordering::Less);
    assert_eq!(ord("1.0.1", "1.0"), Ordering::Greater);
    assert_eq!(ord("2.1", "2.1.0"), Ordering::Less);
    assert_eq!(ord("1.0_git20231201", "1.0"), Ordering::Greater);
    assert_eq!(ord("1.0_cvs", "1.0_p1"), Ordering::Less);
    assert_eq!(ord("3.0.0", "3.0"), Ordering::Greater);
    assert_eq!(ord("1.0_rc1", "1.0_rc2"), Ordering::Less);
  }

  #[test]
  fn numeric_not_lexicographic() {
    assert_eq!(ord("1.10", "1.9"), Ordering::Greater);
    assert_eq!(ord("3.21", "3.9"), Ordering::Greater);
  }

  #[test]
  fn satisfies_plain_operators() {
    assert!(ApkVersionCompare.satisfies("2.0-r1", ">=1.0"));
    assert!(ApkVersionCompare.satisfies("1.0-r0", "=1.0-r0"));
    assert!(ApkVersionCompare.satisfies("0.9", "<1.0"));
    assert!(!ApkVersionCompare.satisfies("1.0", ">1.0"));
  }

  #[test]
  fn satisfies_fuzzy_prefix() {
    assert!(ApkVersionCompare.satisfies("2.1.3", "~2.1"));
    assert!(ApkVersionCompare.satisfies("2.1", "~2.1"));
    assert!(ApkVersionCompare.satisfies("2.1.3-r2", "~=2.1"));
    assert!(!ApkVersionCompare.satisfies("2.2.0", "~2.1"));
    assert!(!ApkVersionCompare.satisfies("2", "~2.1"));
  }

  #[test]
  fn satisfies_unparseable_is_lenient() {
    assert!(ApkVersionCompare.satisfies("1.0", "garbage"));
    assert!(ApkVersionCompare.satisfies("1.0", ""));
  }
}
