//! `VersionCompare` is the trait the resolver calls for version
//! ordering and constraint checks; `DpkgVersionCompare` /
//! `RpmVersionCompare` / `PacmanVersionCompare` / `ApkVersionCompare`
//! are the four family implementations; `ComparisonOp` parses a
//! constraint operator and decides whether a version satisfies it.

use crate::version::apk::ApkVersion;
use crate::version::dpkg::DebianVersion;
use crate::version::pacman::PacmanVersion;
use crate::version::rpm::RpmVersion;

/// Version ordering and constraint checking under one distribution
/// family's rules; the resolver calls this without knowing which family
/// is active.
pub trait VersionCompare: Send + Sync {
  /// Whether `candidate_version` satisfies `constraint` under this
  /// family's ordering.
  fn satisfies(&self, candidate_version: &str, constraint: &str) -> bool;

  /// Orders two version strings under this family's rules.
  fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering;
}

/// Version comparison for the Debian family, with its epoch-and-revision
/// grammar and ordering.
pub struct DpkgVersionCompare;

impl VersionCompare for DpkgVersionCompare {
  fn satisfies(&self, candidate_version: &str, constraint: &str) -> bool {
    ComparisonOp::satisfies_with(candidate_version, constraint, false, DebianVersion::parse)
  }

  fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering {
    DebianVersion::parse(a).cmp(&DebianVersion::parse(b))
  }
}

/// Version comparison for the RPM family, with its segment ordering and
/// looser operator spelling.
pub struct RpmVersionCompare;

impl VersionCompare for RpmVersionCompare {
  fn satisfies(&self, candidate_version: &str, constraint: &str) -> bool {
    // RPM also accepts the single-character `<` / `>` operator forms.
    ComparisonOp::satisfies_with(candidate_version, constraint, true, RpmVersion::parse)
  }

  fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering {
    RpmVersion::parse(a).cmp(&RpmVersion::parse(b))
  }
}

/// Version comparison for the pacman family (Arch, CachyOS), using
/// libalpm's `vercmp` convention and bare single-char operators.
pub struct PacmanVersionCompare;

impl VersionCompare for PacmanVersionCompare {
  fn satisfies(&self, candidate_version: &str, constraint: &str) -> bool {
    // pacman writes its strict operators as bare `<` / `>`.
    ComparisonOp::satisfies_with(candidate_version, constraint, true, PacmanVersion::parse)
  }

  fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering {
    PacmanVersion::parse(a).cmp(&PacmanVersion::parse(b))
  }
}

/// Version comparison for Alpine, using apk-tools' convention plus the `~`
/// fuzzy operator (any release under a component prefix).
pub struct ApkVersionCompare;

impl VersionCompare for ApkVersionCompare {
  fn satisfies(&self, candidate_version: &str, constraint: &str) -> bool {
    // The fuzzy forms (`~=`, `~`) ask for a component-prefix match, a
    // relation the shared operator set cannot express; everything else
    // goes through the common path with bare `<` / `>` accepted.
    let trimmed = constraint.trim();
    if let Some(rest) = trimmed.strip_prefix("~=").or_else(|| trimmed.strip_prefix('~')) {
      let rest = rest.trim();
      if rest.is_empty() {
        return true;
      }
      return ApkVersion::parse(candidate_version).fuzzy_matches(&ApkVersion::parse(rest));
    }
    ComparisonOp::satisfies_with(candidate_version, trimmed, true, ApkVersion::parse)
  }

  fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering {
    ApkVersion::parse(a).cmp(&ApkVersion::parse(b))
  }
}

/// The closed set of version-constraint operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
  /// Strict less-than (`<<`). Satisfied when the candidate is
  /// strictly older than the constraint's version.
  Lt,
  /// Less-than-or-equal (`<=`). Satisfied when the candidate is
  /// older than the constraint's version or equal to it.
  Le,
  /// Exact equality (`=`). Satisfied only when the candidate
  /// orders equal to the constraint's version.
  Eq,
  /// Greater-than-or-equal (`>=`). Satisfied when the candidate
  /// is newer than the constraint's version or equal to it.
  Ge,
  /// Strict greater-than (`>>`). Satisfied when the candidate is
  /// strictly newer than the constraint's version.
  Gt,
}

impl ComparisonOp {
  /// Splits the operator off the front of a constraint, returning it
  /// with the remaining version text. Two-character spellings are tried
  /// before the one-character ones they begin with, and bare `<`/`>`
  /// are accepted only when `accept_single`. `None` means no operator
  /// matched.
  pub(crate) fn parse(constraint: &str, accept_single: bool) -> Option<(ComparisonOp, &str)> {
    // Two-character operators first so `<<`/`>>` are not misread as
    // `<`/`>`; single-character `=` always, single `<`/`>` only for RPM.
    if let Some(rest) = constraint.strip_prefix("<<") {
      Some((ComparisonOp::Lt, rest))
    } else if let Some(rest) = constraint.strip_prefix("<=") {
      Some((ComparisonOp::Le, rest))
    } else if let Some(rest) = constraint.strip_prefix(">=") {
      Some((ComparisonOp::Ge, rest))
    } else if let Some(rest) = constraint.strip_prefix(">>") {
      Some((ComparisonOp::Gt, rest))
    } else if accept_single && let Some(rest) = constraint.strip_prefix('<') {
      Some((ComparisonOp::Lt, rest))
    } else if accept_single && let Some(rest) = constraint.strip_prefix('>') {
      Some((ComparisonOp::Gt, rest))
    } else if let Some(rest) = constraint.strip_prefix('=') {
      Some((ComparisonOp::Eq, rest))
    } else {
      None
    }
  }

  /// Whether `candidate_version` satisfies `constraint` under a
  /// family's ordering. `accept_single` also accepts bare `<`/`>`;
  /// `parse_family` turns a raw version string into that family's
  /// comparable value. A constraint with no recognizable operator or no
  /// version counts as satisfied, so a malformed index entry never
  /// blocks a pick.
  pub fn satisfies_with<V: Ord>(
    candidate_version: &str,
    constraint: &str,
    accept_single: bool,
    parse_family: fn(&str) -> V,
  ) -> bool {
    let Some((op, rest)) = ComparisonOp::parse(constraint.trim(), accept_single) else {
      return true;
    };
    let rest = rest.trim();
    if rest.is_empty() {
      return true;
    }
    op.evaluate(parse_family(candidate_version).cmp(&parse_family(rest)))
  }

  /// Whether the computed `ordering` satisfies this operator.
  fn evaluate(self, ordering: std::cmp::Ordering) -> bool {
    use std::cmp::Ordering;
    match self {
      ComparisonOp::Lt => ordering == Ordering::Less,
      ComparisonOp::Le => ordering != Ordering::Greater,
      ComparisonOp::Eq => ordering == Ordering::Equal,
      ComparisonOp::Ge => ordering != Ordering::Less,
      ComparisonOp::Gt => ordering == Ordering::Greater,
    }
  }
}
