//! Applies RPM rich (conditional) dependencies after the main walk:
//! every conditional is re-evaluated against the current package set,
//! adding what newly-satisfied conditions call for, until a pass adds
//! nothing.

use anyhow::Result;

use crate::package::{DepKind, RichDep};
use crate::resolver::edge::DepEdge;

use super::DepWalker;

impl DepWalker<'_> {
  /// Re-evaluates every rich dependency against the current set until a
  /// full pass changes nothing. Each satisfied condition adds its
  /// payload with its full closure and records the edge.
  pub(super) fn rich_dep_fixpoint(&mut self) -> Result<()> {
    loop {
      let mut changed = false;
      let pkgs_with_rich = self.cfg.index.packages().with_rich_deps()?;
      for pkg_name in &pkgs_with_rich {
        if !self.state.visited.contains(pkg_name) {
          continue;
        }
        changed |= self.rich_deps_apply(pkg_name)?;
      }
      if !changed {
        return Ok(());
      }
    }
  }

  /// Re-evaluates one package's rich dependencies, adding whatever now
  /// applies. `true` when anything new joined — the signal for another
  /// fixpoint round.
  fn rich_deps_apply(&mut self, pkg_name: &str) -> Result<bool> {
    let mut changed = false;
    for rich_dep in self.cfg.index.packages().rich_deps(pkg_name)? {
      let kind = rich_dep.edge_kind();
      for name in self.rich_dep_evaluate(&rich_dep)? {
        changed |= self.payload_admit(pkg_name, &name, kind.clone())?;
      }
    }
    Ok(changed)
  }

  /// Adds one payload package together with its full hard-dependency
  /// closure and records the edge. A name that resolves to nothing, or
  /// one already present, changes nothing.
  fn payload_admit(&mut self, pkg_name: &str, name: &str, kind: DepKind) -> Result<bool> {
    let Some(real) = self.name_resolve(name)? else {
      return Ok(false);
    };
    if self.state.visited.contains(&real) {
      return Ok(false);
    }
    self.transitive_resolve(&real)?;
    self.state.edges.push((
      pkg_name.to_string(),
      DepEdge {
        child: real,
        kind,
        picked_alternative: None,
        original_constraint: None,
      },
    ));
    Ok(true)
  }

  /// Evaluates one rich-dep expression into the packages it calls for
  /// given the current set: an `Or` prefers the already-satisfied side,
  /// and `If`/`Unless` contribute their payload only when the condition
  /// holds.
  fn rich_dep_evaluate(&self, dep: &RichDep) -> Result<Vec<String>> {
    match dep {
      RichDep::Pkg(spec) => Ok(vec![spec.name.clone()]),

      RichDep::And(l, r) => {
        let mut result = self.rich_dep_evaluate(l)?;
        result.extend(self.rich_dep_evaluate(r)?);
        Ok(result)
      }

      RichDep::Or(l, r) => {
        let left = self.rich_dep_evaluate(l)?;
        if left
          .iter()
          .try_fold(false, |acc, n| self.is_resolved(n).map(|r| acc || r))?
        {
          Ok(left)
        } else {
          self.rich_dep_evaluate(r)
        }
      }

      RichDep::If { payload, condition } => {
        if self.condition_evaluate(condition)? {
          self.rich_dep_evaluate(payload)
        } else {
          Ok(Vec::new())
        }
      }

      RichDep::Unless { payload, condition } => {
        if !self.condition_evaluate(condition)? {
          self.rich_dep_evaluate(payload)
        } else {
          Ok(Vec::new())
        }
      }

      RichDep::With(l, _) => self.rich_dep_evaluate(l),
      RichDep::Without(l, _) => self.rich_dep_evaluate(l),
    }
  }

  /// Evaluates a condition against the current set: a named package
  /// counts only when already present and, when constrained, at a
  /// satisfying version; a nested `If` whose own condition fails counts
  /// as true.
  fn condition_evaluate(&self, dep: &RichDep) -> Result<bool> {
    match dep {
      RichDep::Pkg(spec) => {
        if !self.is_resolved(&spec.name)? {
          return Ok(false);
        }
        if let Some(ref constraint) = spec.version_constraint {
          let real_name = match self.name_resolve(&spec.name)? {
            Some(r) => r,
            None => return Ok(false),
          };
          if let Some(pkg) = self.cfg.index.packages().get(&real_name)? {
            return Ok(self.cfg.index.version_cmp().satisfies(&pkg.version, constraint));
          }
          Ok(false)
        } else {
          Ok(true)
        }
      }
      RichDep::And(l, r) => Ok(self.condition_evaluate(l)? && self.condition_evaluate(r)?),
      RichDep::Or(l, r) => Ok(self.condition_evaluate(l)? || self.condition_evaluate(r)?),
      RichDep::If { payload, condition } => {
        if self.condition_evaluate(condition)? {
          self.condition_evaluate(payload)
        } else {
          Ok(true) // Condition not met → vacuously true
        }
      }
      RichDep::Unless { payload, condition } => {
        if !self.condition_evaluate(condition)? {
          self.condition_evaluate(payload)
        } else {
          Ok(true)
        }
      }
      RichDep::With(l, _) => self.condition_evaluate(l),
      RichDep::Without(l, _) => self.condition_evaluate(l),
    }
  }

  /// Whether a name is in the closure, either directly or through a
  /// package that provides it.
  fn is_resolved(&self, name: &str) -> Result<bool> {
    if self.state.visited.contains(name) {
      return Ok(true);
    }
    let providers = self.cfg.index.providers().of(name)?;
    Ok(providers.iter().any(|p| self.state.visited.contains(&p.package_name)))
  }
}
