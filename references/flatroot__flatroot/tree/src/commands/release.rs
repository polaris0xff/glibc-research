//! The `release list` command: lists the releases a named distribution
//! publishes.

use std::collections::BTreeMap;

use anyhow::Result;
use serde::ser::{Serialize, SerializeMap, Serializer};

use flatroot::distro::{DistroRegistry, FromSelector};

use crate::commands::report::Report;
use crate::commands::session::Session;

/// Prints the releases a distribution publishes. Only the distribution
/// prefix of `remote_str` is read; a release named after the `:` is
/// ignored, since every release is listed.
pub fn list(session: &Session, remote_str: &str, format: crate::parser::OutputFormat) -> Result<()> {
  let prefix = FromSelector::prefix_of(remote_str);
  let rt = DistroRegistry::releases(prefix, session.http())?;
  let rows: Vec<ReleaseRow> = rt
    .rows
    .iter()
    .map(|cells| ReleaseRow {
      header: &rt.header,
      cells,
    })
    .collect();
  Report::emit(&["release", "list"], &rows, format)
}

/// One release row: its cells paired with the table's shared column
/// headers.
struct ReleaseRow<'a> {
  /// The table's column headers, shared across every row.
  header: &'a [&'static str],
  /// This row's cells, one per header column in header order.
  cells: &'a [String],
}

impl ReleaseRow<'_> {
  /// Normalizes a column name: lowercased, and a `release` column is
  /// renamed `name`, so the same field has the same key across
  /// distributions.
  fn header_normalize(header: &str) -> String {
    let lower = header.to_ascii_lowercase();
    if lower == "release" { "name".to_string() } else { lower }
  }
}

/// Serializes a release as a map from normalized column name to cell,
/// in sorted key order. A cell with no matching header is dropped,
/// since it would have no key.
impl Serialize for ReleaseRow<'_> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut obj: BTreeMap<String, &str> = BTreeMap::new();
    for (i, cell) in self.cells.iter().enumerate() {
      if let Some(h) = self.header.get(i) {
        obj.insert(ReleaseRow::header_normalize(h), cell.as_str());
      }
    }
    let mut map = serializer.serialize_map(Some(obj.len()))?;
    for (key, value) in obj {
      map.serialize_entry(&key, value)?;
    }
    map.end()
  }
}
