//! Translates between `ManifestState` and the on-disk manifest text —
//! the header and the package body — with no knowledge of where the
//! files live or how they are written.

use std::path::Path;

use anyhow::{Context, Result, bail};

use super::layout::ManifestLayout;
use super::record::PackageRecord;
use super::state::ManifestState;

/// The manifest's header lines — tool version, sources, architectures,
/// and package count — so a later install can check a rootfs before
/// parsing every package.
pub(crate) struct ManifestHeader {
  /// Version of the flatroot binary that wrote the manifest.
  pub(crate) flatroot_version: String,
  /// `Sources:` line — `, `-separated distro sources, raw text.
  pub(crate) sources: String,
  /// `Architectures:` line — `, `-separated arch list, raw text.
  pub(crate) architectures: String,
  /// `PackageCount:`, cross-checked against the record count.
  pub(crate) package_count: usize,
}

impl ManifestHeader {
  /// Renders the header lines from `state`.
  pub(crate) fn format(state: &ManifestState) -> String {
    let sources: Vec<&str> = state.sources.iter().map(|s| s.as_str()).collect();
    let archs: Vec<&str> = state.architectures.iter().map(|s| s.as_str()).collect();
    let lines = vec![
      format!("FlatrootVersion: {}", state.flatroot_version),
      format!("Sources: {}", sources.join(", ")),
      format!("Architectures: {}", archs.join(", ")),
      format!("PackageCount: {}", state.packages.len()),
    ];
    lines.join("\n")
  }

  /// Parses the header from text; a malformed, missing, or unknown
  /// field is an error naming the file, since later compatibility and
  /// skip decisions read these values.
  pub(crate) fn parse(text: &str, path_file: &Path) -> Result<ManifestHeader> {
    let mut flatroot_version: Option<String> = None;
    let mut sources: Option<String> = None;
    let mut architectures: Option<String> = None;
    let mut package_count: Option<usize> = None;

    for line in text.lines() {
      if line.is_empty() {
        continue;
      }
      let (key, value) = match line.split_once(':') {
        Some((k, v)) => (k, v.strip_prefix(' ').unwrap_or(v)),
        None => bail!("{}: line missing ':': {:?}", path_file.display(), line),
      };
      match key {
        "FlatrootVersion" => flatroot_version = Some(value.to_string()),
        "Sources" => sources = Some(value.to_string()),
        "Architectures" => architectures = Some(value.to_string()),
        "PackageCount" => {
          package_count = Some(
            value
              .parse()
              .with_context(|| format!("{}: PackageCount not an integer: {}", path_file.display(), value))?,
          )
        }
        other => bail!("{}: unknown manifest key '{}'", path_file.display(), other),
      }
    }

    Ok(ManifestHeader {
      flatroot_version: flatroot_version
        .with_context(|| format!("{}: missing FlatrootVersion", path_file.display()))?,
      sources: sources.with_context(|| format!("{}: missing Sources", path_file.display()))?,
      architectures: architectures.with_context(|| format!("{}: missing Architectures", path_file.display()))?,
      package_count: package_count.with_context(|| format!("{}: missing PackageCount", path_file.display()))?,
    })
  }
}

/// Joins and splits the `packages` file's per-package stanzas, leaving
/// one stanza's format to `PackageRecord`.
pub struct ManifestCodec;

impl ManifestCodec {
  /// Renders every package record as blank-line-separated stanzas, in a
  /// stable order so the same state renders identically.
  pub(crate) fn packages_format(state: &ManifestState) -> String {
    let records: Vec<String> = state.packages.values().map(PackageRecord::format).collect();
    records.join("\n\n")
  }

  /// Splits the `packages` text into records, each loading its file
  /// list. An empty body yields no packages; a stanza that fails to
  /// parse aborts the read rather than producing a partial result.
  pub(crate) fn packages_parse(text: &str, layout: &ManifestLayout) -> Result<Vec<PackageRecord>> {
    let mut out = Vec::new();
    if text.trim().is_empty() {
      return Ok(out);
    }
    for chunk in text.split("\n\n") {
      let chunk = chunk.trim_matches('\n');
      if chunk.is_empty() {
        continue;
      }
      out.push(PackageRecord::parse(chunk, layout)?);
    }
    Ok(out)
  }
}
