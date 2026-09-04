//! `ExportPlan`: resolves which export format to run — from the
//! `--format` flag or the output file's extension — and checks the
//! format's requirements before any work.

use std::path::Path;

use anyhow::{Result, bail};

use crate::parser::ExportFormat;

use super::dwarfs::DwarfsExporter;
use super::exporter::Exporter;
use super::oci::OciExporter;
use super::source::RootfsSource;
use super::sqfs::SqfsExporter;
use super::tar::TarExporter;

/// One requested export: the output format, possibly left to infer from
/// the output path, and the optional OCI tag.
pub struct ExportPlan<'a> {
  format: Option<ExportFormat>,
  tag: Option<&'a str>,
}

impl<'a> ExportPlan<'a> {
  /// Pairs the requested format (`None` to infer it from the output
  /// path) with the optional OCI tag.
  pub fn new(format: Option<ExportFormat>, tag: Option<&'a str>) -> Self {
    Self { format, tag }
  }

  /// Runs the export: validates the source directory, resolves the
  /// format, checks the format's requirements, and writes the artifact
  /// at `output`.
  pub fn run(self, src_dir: &Path, output: &Path) -> Result<()> {
    let source = RootfsSource::open(src_dir)?;
    let format = self.format_resolve(output)?;
    let exporter = self.into_exporter(format)?;
    exporter.export(&source, output)
  }

  /// Resolves the export format: an explicit `--format` wins; otherwise
  /// the output file's extension decides. Only unambiguous extensions
  /// are accepted — `.tar` is rejected because both the tar and OCI
  /// formats produce one, and an unrecognized extension is an error,
  /// never a guess.
  fn format_resolve(&self, output: &Path) -> Result<ExportFormat> {
    if let Some(format) = self.format.clone() {
      return Ok(format);
    }

    let name = output
      .file_name()
      .and_then(|n| n.to_str())
      .ok_or_else(|| anyhow::anyhow!("Output path has no file name: {}", output.display()))?;
    let lower = name.to_ascii_lowercase();

    if lower.ends_with(".tar.gz") {
      return Ok(ExportFormat::Tar);
    }
    if lower.ends_with(".sqfs") || lower.ends_with(".squashfs") {
      return Ok(ExportFormat::Sqfs);
    }
    if lower.ends_with(".dwarfs") {
      return Ok(ExportFormat::Dwarfs);
    }
    if lower.ends_with(".tar") {
      bail!("Ambiguous output extension `.tar` — use `--format tar` or `--format oci` to disambiguate");
    }

    bail!("Cannot infer format from output extension; specify --format explicitly")
  }

  /// The exporter for `format`. OCI requires `--tag` and is refused
  /// here without it; the other formats ignore the tag.
  fn into_exporter(self, format: ExportFormat) -> Result<Box<dyn Exporter + 'a>> {
    match format {
      ExportFormat::Oci => {
        let tag = self
          .tag
          .ok_or_else(|| anyhow::anyhow!("--tag is required for OCI format. Example: --tag myapp:v1.0"))?;
        Ok(Box::new(OciExporter { tag }))
      }
      ExportFormat::Tar => Ok(Box::new(TarExporter)),
      ExportFormat::Dwarfs => Ok(Box::new(DwarfsExporter)),
      ExportFormat::Sqfs => Ok(Box::new(SqfsExporter)),
    }
  }
}
