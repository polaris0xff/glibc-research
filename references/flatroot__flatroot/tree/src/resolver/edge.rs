//! `DepEdge`: one edge in the resolved dependency graph — one package
//! drawing in another, with the kind, the chosen alternative, and the
//! declared version constraint.

use crate::package::DepKind;

/// One edge in the resolved dependency graph: a package drew in
/// `child`, with the detail needed to explain why.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DepEdge {
  /// Resolved name of the child package this edge points to.
  pub child: String,
  /// How the child was pulled in — hard, recommended/suggested, install-if,
  /// or a conditional rich-dep form.
  pub kind: DepKind,
  /// The alternative chosen when the dependency group offered several;
  /// `None` for a single-alternative group.
  pub picked_alternative: Option<String>,
  /// The version constraint the index declared on the chosen alternative,
  /// verbatim; `None` when unconstrained.
  pub original_constraint: Option<String>,
}
