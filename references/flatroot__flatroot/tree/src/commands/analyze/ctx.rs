//! The read-only context both analysis passes share.

use std::path::Path;

use flatroot::db::{Index, PackageRow};
use flatroot::remote::Remote;

/// Read-only data both analysis passes read.
pub struct AnalysisContext<'a> {
  /// The populated package index every package lookup runs against.
  pub index: &'a Index,

  /// The active distribution backend, for download URLs, extraction, and
  /// soname-to-provider resolution.
  pub remote: &'a dyn Remote,

  /// Directory downloaded archives are cached in — the same cache every
  /// other command's downloads use.
  pub path_dir_cache: &'a Path,

  /// Package rows of every seed package, resolved up front so the passes
  /// do not re-query them.
  pub targets: &'a [PackageRow],

  /// Target architecture as a uname string (e.g. `x86_64`).
  pub arch: &'a str,

  /// How many times a failed download is retried in the linker pass.
  pub http_retries: u32,
}
