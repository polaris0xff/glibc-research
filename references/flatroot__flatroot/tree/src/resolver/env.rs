//! `ResolutionEnv`: the read-only inputs one resolve reads — the
//! package index and the options that widen or narrow the closure.

use crate::db::Index;

/// The read-only inputs one resolve reads.
pub struct ResolutionEnv<'a> {
  /// The package index dependencies are looked up in; it also supplies
  /// the version comparator and the file-to-package data.
  pub index: &'a Index,
  /// Also follow `Recommends:` dependencies, not only hard ones.
  pub include_recommends: bool,
  /// Also follow `Suggests:` dependencies.
  pub include_suggests: bool,
  /// Package names the resolve drops entirely — the named packages and
  /// anything reachable only through them.
  pub exclude: &'a [String],
}
