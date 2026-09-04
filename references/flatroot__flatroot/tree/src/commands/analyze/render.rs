//! Prints a finished trace in the requested output format.

use anyhow::Result;

use super::outcome::AnalysisOutcome;
use crate::commands::report::Report;
use crate::parser::OutputFormat;

impl AnalysisOutcome {
  /// Prints the entries under the `analyze trace` scope in the chosen
  /// `format` — plain Shout lines or JSON, like every other reading
  /// command.
  pub fn render(&self, format: OutputFormat) -> Result<()> {
    Report::emit(&["analyze", "trace"], &self.entries, format)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::commands::analyze::outcome::{AnalysisReason, ConsumedSoname, TraceEntry, TraceSeed, UnresolvedSoname};
  use std::path::PathBuf;

  fn make_outcome() -> AnalysisOutcome {
    let seeds = vec![TraceSeed {
      name: "bash".into(),
      version: "5.2".into(),
    }];
    let entries = vec![
      TraceEntry {
        depends_on: vec!["libc6".into(), "libtinfo6".into()],
        name: "bash".into(),
        reason: AnalysisReason::Target,
        sonames_consumed: vec![
          ConsumedSoname {
            binary: PathBuf::from("bin/bash"),
            provider: "libc6".into(),
            soname: "libc.so.6".into(),
          },
          ConsumedSoname {
            binary: PathBuf::from("bin/bash"),
            provider: "libc6".into(),
            soname: "libdl.so.2".into(),
          },
          ConsumedSoname {
            binary: PathBuf::from("bin/bash"),
            provider: "libtinfo6".into(),
            soname: "libtinfo.so.6".into(),
          },
          ConsumedSoname {
            binary: PathBuf::from("bin/bash"),
            provider: "undeclared-lib".into(),
            soname: "libfoo.so.1".into(),
          },
        ],
        sonames_provided: vec![],
        unresolved_sonames: vec![UnresolvedSoname {
          binary: PathBuf::from("bin/bash"),
          soname: "libmystery.so.1".into(),
        }],
        version: "5.2".into(),
      },
      TraceEntry {
        depends_on: vec![],
        name: "libc6".into(),
        reason: AnalysisReason::DeclaredLinker,
        sonames_consumed: vec![],
        sonames_provided: vec!["libc.so.6".into(), "libdl.so.2".into()],
        unresolved_sonames: vec![],
        version: "2.36".into(),
      },
      TraceEntry {
        depends_on: vec!["libc6".into()],
        name: "libtinfo6".into(),
        reason: AnalysisReason::DeclaredLinker,
        sonames_consumed: vec![ConsumedSoname {
          binary: PathBuf::from("usr/lib/x86_64-linux-gnu/libtinfo.so.6.4"),
          provider: "libc6".into(),
          soname: "libc.so.6".into(),
        }],
        sonames_provided: vec!["libtinfo.so.6".into()],
        unresolved_sonames: vec![],
        version: "6.4".into(),
      },
      TraceEntry {
        depends_on: vec![],
        name: "undeclared-lib".into(),
        reason: AnalysisReason::Linker,
        sonames_consumed: vec![],
        sonames_provided: vec!["libfoo.so.1".into()],
        unresolved_sonames: vec![],
        version: "1.0".into(),
      },
    ];
    AnalysisOutcome {
      source: "debian:bookworm".into(),
      arch: "x86_64".into(),
      seeds,
      entries,
    }
  }

  #[test]
  fn render_dispatches_both_formats() {
    let outcome = make_outcome();
    // Smoke-test: each renderer runs to completion. Output assertions
    // live in the format-specific tests below.
    outcome.render(OutputFormat::Plain).unwrap();
    outcome.render(OutputFormat::Json).unwrap();
  }

  #[test]
  fn json_round_trips_through_serde_value() {
    let outcome = make_outcome();
    let s = serde_json::to_string_pretty(&outcome.entries).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let arr = v.as_array().expect("json top must be array");
    assert!(arr.iter().any(|e| e["name"] == "bash"));
    let libc6 = arr.iter().find(|e| e["name"] == "libc6").unwrap();
    assert_eq!(libc6["reason"], "declared_linker");
    assert_eq!(libc6["sonames_provided"][0], "libc.so.6");
  }
}
