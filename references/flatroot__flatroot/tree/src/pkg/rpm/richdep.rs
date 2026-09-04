//! The one place RPM's boolean dependency formulas are parsed: the
//! recursive grammar that reads a formula into a `RichDep` tree, and
//! the reduction of a tree with no real condition back into a flat
//! dependency list.

use anyhow::{Context, Result, bail};

use crate::package::{DepSpec, Dependency, RichDep};
use crate::pkg::rpm::isa::IsaQualifier;

/// The grammar for reading and reducing RPM rich-dependency formulas, kept
/// in one place.
pub(super) struct RichDepExpr;

impl RichDepExpr {
  /// Reads a parenthesized rich-dependency formula into the `RichDep`
  /// tree the resolver walks and evaluates.
  pub(super) fn parse(expr: &str) -> Result<RichDep> {
    let inner = expr
      .trim()
      .strip_prefix('(')
      .and_then(|s| s.strip_suffix(')'))
      .unwrap_or(expr.trim())
      .trim();

    if inner.is_empty() {
      bail!("Empty rich dependency expression: '{}'", expr);
    }

    if let Some((left, op, right)) = RichDepExpr::op_split(inner) {
      let left_ast = RichDepExpr::parse(&format!("({})", left))
        .with_context(|| format!("Failed to parse left side of '{}': '{}'", op, left))?;
      let right_ast = RichDepExpr::parse(&format!("({})", right))
        .with_context(|| format!("Failed to parse right side of '{}': '{}'", op, right))?;

      match op {
        "if" => Ok(RichDep::If {
          payload: Box::new(left_ast),
          condition: Box::new(right_ast),
        }),
        "unless" => Ok(RichDep::Unless {
          payload: Box::new(left_ast),
          condition: Box::new(right_ast),
        }),
        "and" => Ok(RichDep::And(Box::new(left_ast), Box::new(right_ast))),
        "or" => Ok(RichDep::Or(Box::new(left_ast), Box::new(right_ast))),
        "with" => Ok(RichDep::With(Box::new(left_ast), Box::new(right_ast))),
        "without" => Ok(RichDep::Without(Box::new(left_ast), Box::new(right_ast))),
        unknown => bail!("Unknown rich dep operator '{}' in: '{}'", unknown, expr),
      }
    } else {
      // Leaf node: plain package, possibly with version constraint and arch qualifier
      let (name, constraint) =
        RichDepExpr::leaf_extract(inner).with_context(|| format!("Failed to parse rich dep leaf: '{}'", inner))?;
      Ok(RichDep::Pkg(DepSpec {
        name,
        version_constraint: constraint,
      }))
    }
  }

  /// Reduces a formula that is really plain "all of"/"one of" logic into an
  /// ordinary requirement list, keeping the resolver's common path
  /// special-case-free. A genuinely conditional formula cannot be reduced
  /// and is refused.
  pub(super) fn flatten(ast: &RichDep) -> Result<Vec<Dependency>> {
    match ast {
      RichDep::Pkg(spec) => Ok(vec![Dependency {
        alternatives: vec![spec.clone()],
      }]),
      RichDep::And(l, r) => {
        let mut deps = RichDepExpr::flatten(l)?;
        deps.extend(RichDepExpr::flatten(r)?);
        Ok(deps)
      }
      RichDep::Or(l, r) => {
        let mut alts = RichDepExpr::or_alternatives(l)?;
        alts.extend(RichDepExpr::or_alternatives(r)?);
        if alts.is_empty() {
          bail!("'or' rich dep produced no alternatives");
        }
        Ok(vec![Dependency { alternatives: alts }])
      }
      RichDep::With(l, _) => RichDepExpr::flatten(l),
      RichDep::Without(l, _) => RichDepExpr::flatten(l),
      RichDep::If { .. } | RichDep::Unless { .. } => {
        bail!("Cannot flatten conditional rich dep — use contains_conditional() to check first");
      }
    }
  }

  /// Gathers all branches of an `or` choice into one flat list of
  /// interchangeable candidates.
  fn or_alternatives(ast: &RichDep) -> Result<Vec<DepSpec>> {
    match ast {
      RichDep::Pkg(spec) => Ok(vec![spec.clone()]),
      RichDep::Or(l, r) => {
        let mut alts = RichDepExpr::or_alternatives(l)?;
        alts.extend(RichDepExpr::or_alternatives(r)?);
        Ok(alts)
      }
      other => {
        // Non-Or, non-Pkg inside an Or — flatten to a single dep and extract its name
        let deps = RichDepExpr::flatten(other)?;
        Ok(deps.into_iter().flat_map(|d| d.alternatives).collect())
      }
    }
  }

  /// Finds the top-level operator joining the formula's two halves — the
  /// one not enclosed by inner parentheses — and splits there, so the
  /// formula is read outermost-meaning-first.
  fn op_split(expr: &str) -> Option<(&str, &str, &str)> {
    let operators = [" if ", " unless ", " or ", " and ", " with ", " without "];
    let mut depth = 0;

    for (i, c) in expr.char_indices() {
      match c {
        '(' => depth += 1,
        ')' => depth -= 1,
        _ => {}
      }
      if depth > 0 {
        continue;
      }
      for op in &operators {
        if expr[i..].starts_with(op) {
          let left = &expr[..i];
          let right = &expr[i + op.len()..];
          let op_name = op.trim();
          return Some((left.trim(), op_name, right.trim()));
        }
      }
    }
    None
  }

  /// Splits a leaf into the bare package name and its optional version
  /// constraint, dropping the ISA qualifier the resolver ignores.
  pub(super) fn leaf_extract(expr: &str) -> Result<(String, Option<String>)> {
    let s = expr.trim();

    if s.is_empty() {
      bail!("Cannot extract package name from empty expression");
    }

    // Strip the multilib ISA qualifier ("name(x86-64)" → "name"), preserving
    // anything after it (a trailing version constraint). The same cleaning runs
    // on the plain Requires/Provides paths, so every stored edge is cleaned the
    // same way regardless of which path produced it.
    let s = IsaQualifier::arch_strip(s);
    let s = s.trim();

    // Find the version operator and re-spell it in the dpkg form the rest of
    // the system expects (RPM "<"/">" are dpkg "<<"/">>"), matching the plain
    // Requires path's mapping. Each operator travels with its dpkg spelling
    // so the two can never disagree, and the compound operators precede the
    // single-character ones so " >= " is never mistaken for " > ".
    let version_ops = [
      (" >= ", ">="),
      (" <= ", "<="),
      (" = ", "="),
      (" > ", ">>"),
      (" < ", "<<"),
    ];
    for (op, dpkg_op) in &version_ops {
      if let Some(idx) = s.find(op) {
        let name = s[..idx].trim().to_string();
        if name.is_empty() {
          bail!("Empty package name before version operator in: '{}'", expr);
        }
        let version = s[idx + op.len()..].trim();
        return Ok((name, Some(format!("{} {}", dpkg_op, version))));
      }
    }

    // No version constraint
    let name = s.to_string();
    if name.is_empty() {
      bail!("Extracted empty package name from: '{}'", expr);
    }
    Ok((name, None))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn leaf_extract_respells_rpm_operators_as_dpkg() {
    let eq = |s: &str| RichDepExpr::leaf_extract(s).unwrap();
    assert_eq!(eq("foo < 3"), ("foo".to_string(), Some("<< 3".to_string())));
    assert_eq!(eq("foo > 3"), ("foo".to_string(), Some(">> 3".to_string())));
    assert_eq!(eq("foo >= 3"), ("foo".to_string(), Some(">= 3".to_string())));
    assert_eq!(eq("foo <= 3"), ("foo".to_string(), Some("<= 3".to_string())));
    assert_eq!(eq("foo = 3"), ("foo".to_string(), Some("= 3".to_string())));
    // ISA qualifier stripped, single "<" re-spelled to "<<", version preserved.
    assert_eq!(eq("glibc(x86-64) < 3~~"), ("glibc".to_string(), Some("<< 3~~".to_string())));
  }
}
