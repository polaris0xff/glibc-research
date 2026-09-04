//! The `remote list` command: lists the supported distributions and the
//! `--from` syntax each accepts.

use anyhow::Result;

use flatroot::distro::DistroRegistry;

use crate::commands::report::Report;

/// One supported distribution in the listing.
#[derive(serde::Serialize)]
struct RemoteEntry {
  /// The `--from` syntax this distro documents (e.g.
  /// `debian:<release>[@<date>]`).
  format: &'static str,
  /// The `--from` prefix the user types to select this distro.
  name: &'static str,
  /// Support status, always `supported` for a registered distro;
  /// emitted so consumers can filter on it.
  status: &'static str,
}

/// Prints the supported distributions and how to select each, answered
/// from the built-in registry with no network access.
pub fn list(format: crate::parser::OutputFormat) -> Result<()> {
  let entries: Vec<RemoteEntry> = DistroRegistry::all()
    .iter()
    .map(|distro| RemoteEntry {
      format: distro.from_syntax(),
      name: distro.prefix(),
      status: "supported",
    })
    .collect();
  Report::emit(&["remote", "list"], &entries, format)
}
