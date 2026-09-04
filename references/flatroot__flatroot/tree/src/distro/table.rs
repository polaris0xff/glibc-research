//! `TableRelease`: a distribution's releases as one uniform table, so
//! any distribution's releases are printed the same way.

/// A distribution's available releases as column headers plus rows,
/// whatever form the distribution publishes them in.
#[derive(Debug, Clone)]
pub struct TableRelease {
  /// Column headers as the per-distro builder published
  /// them.
  pub header: Vec<&'static str>,
  /// Release rows — one inner vector per release, with one
  /// cell per column matching the header order.
  pub rows: Vec<Vec<String>>,
}
