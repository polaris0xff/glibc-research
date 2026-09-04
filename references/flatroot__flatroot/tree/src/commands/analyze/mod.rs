//! The `analyze trace` command: computes the dependencies of a set of
//! seed packages in two ways — what the package metadata declares, and
//! what the packages' ELF binaries load at run time — and merges both
//! into one report so the two can be compared.

pub mod ctx;
pub mod outcome;
pub mod pass;
pub mod render;
mod trace;

#[allow(unused_imports)]
pub use outcome::{AnalysisOutcome, AnalysisReason, ConsumedSoname, TraceEntry, TraceSeed, UnresolvedSoname};
pub use trace::AnalyzeArgs;
