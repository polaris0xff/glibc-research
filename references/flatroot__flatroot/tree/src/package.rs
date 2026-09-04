//! The crate's shared vocabulary for packages and their dependency
//! relationships, independent of distribution, archive format, or how a
//! package was fetched. Each packaging family is translated into these
//! shapes once, at parse time.

use anyhow::{Context, Result};

/// One package described in the crate's shared shape, carrying everything
/// later stages need to decide whether it belongs in the install set,
/// where to obtain it, and how to verify it.
#[derive(Debug, Clone)]
pub struct Package {
  /// Package name as the source's index reports it.
  pub name: String,
  /// Package version as the source's index reports it.
  pub version: String,
  /// Hard dependency list — every group must be satisfied.
  pub depends: Vec<Dependency>,
  /// Virtual names this package provides.
  pub provides: Vec<DepSpec>,
  /// Recommended packages (Debian-style soft dependencies);
  /// empty when the source format does not publish any.
  pub recommends: Vec<Dependency>,
  /// Suggested packages — Debian's soft dependencies or
  /// pacman's optional dependencies; empty when the source
  /// format does not publish any.
  pub suggests: Vec<Dependency>,
  /// Alpine conjunctive triggers — install this package when
  /// *all* listed packages are in the resolved set.
  pub install_if: Vec<String>,
  /// Packages this one cannot coexist with.
  pub conflicts: Vec<Dependency>,
  /// Packages this one breaks (a Debian-family notion).
  pub breaks: Vec<Dependency>,
  /// Marks a package the Debian family declares essential to a
  /// working system; always `false` for every other family.
  pub essential: bool,
  /// Debian install priority as a sort ordinal: `0` required, `1`
  /// important, `2` standard, `3` optional, `4` extra. Absent for any
  /// source that publishes no priority; provider selection treats
  /// absent as lowest preference.
  pub priority: Option<u8>,
  /// Free-form description (typically the index's first-line
  /// summary).
  pub description: String,
  /// Archive filename or relative path, rewritten at parse time to encode
  /// the mirror and the path within it, so a download can reconstruct the
  /// absolute address.
  pub filename: String,
  /// Archive size in bytes, as the source's index reports it.
  pub size: u64,
  /// Raw digest in the algorithm-specific encoding; the algorithm is
  /// reported separately by the source.
  pub checksum: String,
  /// Rich-dependency trees whose evaluation depends on the resolved set.
  /// Unconditional ones are folded into `depends` at parse time; only
  /// genuinely conditional trees survive here for the deferred pass.
  pub rich_deps: Vec<RichDep>,
}

impl Package {
  /// Index insertion relies on packages arriving name-grouped with releases in
  /// ascending version order, so the per-name newest row wins deterministic
  /// reads. Every format parser sorts its parsed batch through this one rule;
  /// the family's own comparator decides version order, exactly as the
  /// index's collation later will.
  pub fn list_sort(packages: &mut [Package], vcmp: &dyn crate::version::VersionCompare) {
    packages.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| vcmp.compare(&a.version, &b.version)));
  }
}

/// One installed package's identity — its name pinned to the architecture
/// it was installed for, since a multilib tree holds the same name once
/// per architecture. Owns the `name:arch` on-disk spelling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageIdentity {
  /// Package name as the upstream package metadata reports it.
  pub name: String,
  /// Architecture this package is installed for, in kernel
  /// `uname -m` form (`x86_64`, `aarch64`, …).
  pub arch: String,
}

impl PackageIdentity {
  /// The `name:arch` dirname this identity is filed under on disk —
  /// the spelling shared by the saved-scripts directory and the
  /// manifest's per-package file lists.
  pub fn dirname(&self) -> String {
    format!("{}:{}", self.name, self.arch)
  }

  /// Strict decode of a `name:arch` dirname; `None` when the text carries
  /// no separator, for readers that must skip what they cannot name.
  pub fn dirname_parse(text: &str) -> Option<PackageIdentity> {
    let (name, arch) = text.split_once(':')?;
    Some(PackageIdentity {
      name: name.to_string(),
      arch: arch.to_string(),
    })
  }

  /// Lenient decode — a dirname without a separator adopts the
  /// supplied default architecture.
  pub fn dirname_parse_or(text: &str, arch_default: &str) -> PackageIdentity {
    let (name, arch) = text.split_once(':').unwrap_or((text, arch_default));
    PackageIdentity {
      name: name.to_string(),
      arch: arch.to_string(),
    }
  }
}

/// The kind of relationship one package declares on another.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DepKind {
  /// Universal "must be installed" edge — every distribution
  /// publishes hard dependencies of this kind.
  Depends,
  /// Recommended-package edge that widens the closure when the
  /// user opts in to recommended dependencies. A Debian-family
  /// notion.
  Recommends,
  /// Suggested-package edge that widens the closure when the
  /// user opts in to suggested dependencies. Sourced from
  /// Debian's suggestions and pacman's optional dependencies.
  Suggests,
  /// Negative edge — the source declares it cannot coexist
  /// with the named package; the resolver warns (without
  /// blocking) when such a conflict fires.
  Conflicts,
  /// Negative edge — the source (Debian family) declares it
  /// breaks the named package; the resolver warns (without
  /// blocking) when it applies.
  Breaks,
  /// Alpine's conjunctive trigger — pulls a package in when
  /// every package it lists is already in the resolved set.
  InstallIf,
  /// Conditional rich dependency — payload is added to the
  /// closure when its condition is satisfied.
  RichIf,
  /// Inverse conditional rich dependency — payload is added
  /// when its condition is *not* satisfied.
  RichUnless,
}

impl DepKind {
  /// The label the deps table stores for the durable dependency kinds;
  /// `None` for the conditional and trigger kinds, which are
  /// re-evaluated each run and not stored.
  pub fn as_dep_table_kind(&self) -> Option<&'static str> {
    match self {
      DepKind::Depends => Some("depends"),
      DepKind::Recommends => Some("recommends"),
      DepKind::Suggests => Some("suggests"),
      DepKind::Conflicts => Some("conflicts"),
      DepKind::Breaks => Some("breaks"),
      DepKind::InstallIf | DepKind::RichIf | DepKind::RichUnless => None,
    }
  }
}

/// One dependency group: a disjunction of alternatives, any one of
/// which satisfies it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
  /// Disjunctive alternatives in source order — the first
  /// satisfiable entry wins.
  pub alternatives: Vec<DepSpec>,
}

/// One dependency alternative: a package name and an optional version
/// constraint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DepSpec {
  /// Target package name as the source's index reports it.
  pub name: String,
  /// Raw constraint string (such as `">= 2.36"`); absent when
  /// unconstrained.
  pub version_constraint: Option<String>,
}

impl Dependency {
  /// Renders dependencies — alternatives and version constraints
  /// included — into the conventional one-line dependency text for
  /// storage.
  pub fn list_format(deps: &[Dependency]) -> String {
    deps
      .iter()
      .map(|d| {
        d.alternatives
          .iter()
          .map(|a| match &a.version_constraint {
            Some(c) => format!("{} ({})", a.name, c),
            None => a.name.clone(),
          })
          .collect::<Vec<_>>()
          .join(" | ")
      })
      .collect::<Vec<_>>()
      .join(", ")
  }

  /// Parses the conventional one-line dependency text back into
  /// structured form, rejecting malformed input rather than guessing,
  /// since a misread dependency would corrupt every later decision.
  pub fn list_parse(raw: &str) -> Result<Vec<Dependency>> {
    let raw = raw.trim();
    if raw.is_empty() {
      return Ok(Vec::new());
    }
    let mut groups = Vec::new();
    for group in raw.split(", ") {
      let mut alternatives = Vec::new();
      for alt in group.split(" | ") {
        let alt = alt.trim();
        if let Some(idx) = alt.find(" (") {
          let name = alt[..idx].to_string();
          let constraint = alt[idx + 2..]
            .strip_suffix(')')
            .with_context(|| format!("unterminated version constraint in '{alt}'"))?
            .to_string();
          alternatives.push(DepSpec {
            name,
            version_constraint: Some(constraint),
          });
        } else {
          alternatives.push(DepSpec {
            name: alt.to_string(),
            version_constraint: None,
          });
        }
      }
      groups.push(Dependency { alternatives });
    }
    Ok(groups)
  }
}

/// A dependency expressed as a logical expression — packages joined by
/// and/or or made conditional — kept as a nestable tree until the
/// resolver can evaluate it against the chosen packages.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RichDep {
  /// Leaf — one named package with an optional version
  /// constraint.
  Pkg(DepSpec),
  /// Boolean conjunction: both subtrees must hold for the
  /// expression to be satisfied.
  And(Box<RichDep>, Box<RichDep>),
  /// Boolean disjunction: at least one subtree must hold.
  Or(Box<RichDep>, Box<RichDep>),
  /// Conditional — the payload is required only when its
  /// condition is satisfied. Because it depends on the resolved
  /// set, it is held back from early flattening and evaluated
  /// once the main resolve has finished.
  If {
    /// The dependency to add when the condition holds.
    payload: Box<RichDep>,
    /// The condition that gates the payload.
    condition: Box<RichDep>,
  },
  /// Inverse conditional — the payload is required only when
  /// its condition is *not* satisfied.
  Unless {
    /// The dependency to add when the condition does not hold.
    payload: Box<RichDep>,
    /// The condition whose absence gates the payload.
    condition: Box<RichDep>,
  },
  /// Companion-version shorthand — left depends on right and
  /// the right side narrows the version range. Reduces to the
  /// left package at flatten time.
  With(Box<RichDep>, Box<RichDep>),
  /// Feature-exclusion shorthand — left depends on right and
  /// the right side excludes a variant. Reduces to the left
  /// package at flatten time.
  Without(Box<RichDep>, Box<RichDep>),
}

impl RichDep {
  /// The `DepKind` a rich expression contributes, fixed from its
  /// shape — an `Unless` is `RichUnless`, everything else `RichIf` —
  /// before the condition is evaluated.
  pub fn edge_kind(&self) -> DepKind {
    match self {
      RichDep::Unless { .. } => DepKind::RichUnless,
      _ => DepKind::RichIf,
    }
  }

  /// Whether any part of the expression depends on the chosen set, which
  /// decides early flattening versus the deferred pass.
  pub fn contains_conditional(&self) -> bool {
    match self {
      RichDep::If { .. } | RichDep::Unless { .. } => true,
      RichDep::And(l, r) | RichDep::Or(l, r) | RichDep::With(l, r) | RichDep::Without(l, r) => {
        l.contains_conditional() || r.contains_conditional()
      }
      RichDep::Pkg(_) => false,
    }
  }
}

/// Test-only constructors for the shared package vocabulary. Every inline
/// `#[cfg(test)]` module builds its fixtures through these, so the fixture
/// shape lives in one place and a new `Package` field is added here once.
#[cfg(test)]
pub mod fixtures {
  use super::{DepSpec, Dependency, Package};

  pub fn pkg(name: &str, version: &str) -> Package {
    Package {
      name: name.to_string(),
      version: version.to_string(),
      depends: vec![],
      recommends: vec![],
      suggests: vec![],
      install_if: vec![],
      provides: vec![],
      conflicts: vec![],
      breaks: vec![],
      essential: false,
      priority: None,
      description: String::new(),
      filename: format!("pool/{}.deb", name),
      size: 0,
      checksum: String::new(),
      rich_deps: vec![],
    }
  }

  pub fn pkg_with_deps(name: &str, version: &str, depends: Vec<Dependency>) -> Package {
    Package {
      depends,
      ..pkg(name, version)
    }
  }

  pub fn dep(name: &str) -> Dependency {
    dep_alts(&[(name, None)])
  }

  pub fn dep_ver(name: &str, constraint: &str) -> Dependency {
    dep_alts(&[(name, Some(constraint))])
  }

  pub fn dep_alts(alts: &[(&str, Option<&str>)]) -> Dependency {
    Dependency {
      alternatives: alts
        .iter()
        .map(|(name, constraint)| DepSpec {
          name: name.to_string(),
          version_constraint: constraint.map(|s| s.to_string()),
        })
        .collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::fixtures::{dep, dep_alts, dep_ver};
  use super::*;

  #[test]
  fn dependency_roundtrip_covers_distro_flavours() {
    let deps = vec![
      dep_ver("libc6", ">= 2.36"),
      dep_alts(&[("debconf", Some(">= 0.5")), ("cdebconf", None)]),
      dep("so:libc.musl-x86_64.so.1"),
      dep("/bin/sh"),
      dep("libc.so.6()(64bit)"),
    ];
    let s = Dependency::list_format(&deps);
    let parsed = Dependency::list_parse(&s).unwrap();
    assert_eq!(parsed, deps);
    assert!(Dependency::list_parse("").unwrap().is_empty());
  }
}
