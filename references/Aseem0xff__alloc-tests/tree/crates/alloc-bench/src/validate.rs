//! Dataset validation. The gate between "we have files" and "we have results".
//!
//! ⛔ THIS IS THE FILE THAT STOPS A BROKEN RUN FROM BECOMING A TABLE. Every
//! check here exists because the alternative is a report that looks complete
//! and is not:
//!
//!   - a cell that never ran, read as a cell that ran and scored nothing;
//!   - a workload whose runs all failed, whose absent median becomes a zero and
//!     therefore the fastest row in the table;
//!   - a binary that passed the build but is not the allocator it is filed
//!     under;
//!   - two cells that were measured against different corpora;
//!   - a run under emulation ranked against a native one.
//!
//! ⚠ `error` blocks reporting a ranking. `warn` is printed in the report,
//! beside the numbers, where a reader cannot miss it.

use crate::model::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warn,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "ERROR",
            Severity::Warn => "warn",
            Severity::Info => "info",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub check: String,
    pub cell: Option<String>,
    pub message: String,
}

/// Above this, the run's own spread is large enough that small differences
/// between allocators cannot be told from the machine. Flagged, not dropped:
/// the number is still real, it just cannot carry a fine comparison.
pub const NOISE_REL_MAD_WARN: f64 = 0.05;

/// The workload the primary ranking uses.
pub const PRIMARY_WORKLOAD: &str = "literal";

pub fn validate(plan: &[Cell], results: &[CellResult], meta: &RunMeta) -> Vec<Finding> {
    let mut f = Vec::new();
    let mut add = |severity: Severity, check: &str, cell: Option<&str>, message: String| {
        f.push(Finding {
            severity,
            check: check.to_string(),
            cell: cell.map(|s| s.to_string()),
            message,
        });
    };

    // --- the dataset covers the plan ---------------------------------------
    let have: BTreeSet<&str> = results.iter().map(|r| r.cell.id.as_str()).collect();
    if have.len() != results.len() {
        add(
            Severity::Error,
            "duplicate-cells",
            None,
            "two result documents carry the same cell id; one measurement would be counted twice"
                .into(),
        );
    }
    for c in plan {
        if !have.contains(c.id.as_str()) {
            add(Severity::Error, "missing-cell", Some(&c.id),
                "planned but no result document exists. A cell that never ran must not be absent from the report; it is a gap, not a zero.".into());
        }
    }
    let planned: BTreeSet<&str> = plan.iter().map(|c| c.id.as_str()).collect();
    for r in results {
        if !planned.contains(r.cell.id.as_str()) {
            add(Severity::Warn, "extra-cell", Some(&r.cell.id),
                "a result exists for a cell this plan did not contain; it is from a different plan and is excluded from ranking".into());
        }
    }

    // --- every `ok` cell earned it -----------------------------------------
    for r in results {
        let id = r.cell.id.as_str();
        match r.outcome.as_str() {
            "ok" => {}
            "unsupported" => {
                if r.detail.as_deref().unwrap_or("").trim().is_empty() {
                    add(Severity::Error, "unsupported-without-reason", Some(id),
                        "marked unsupported with no reason recorded. An unexplained absence is the failure this project exists to avoid.".into());
                }
                continue;
            }
            other => {
                add(
                    Severity::Info,
                    "cell-failed",
                    Some(id),
                    format!(
                        "outcome {}: {}",
                        other,
                        r.detail.as_deref().unwrap_or("no detail recorded")
                    ),
                );
                continue;
            }
        }

        // Identity, which is the check a benchmark most often skips.
        match r.identity.as_ref().and_then(|v| v.get("ok")).and_then(|v| v.as_bool()) {
            Some(true) => {}
            Some(false) => add(Severity::Error, "identity", Some(id),
                format!("marked ok but the ELF evidence rejects it: {}",
                    r.identity.as_ref().and_then(|v| v.get("reasons")).map(|v| v.to_string()).unwrap_or_default())),
            None => add(Severity::Error, "identity-missing", Some(id),
                "marked ok but carries no identity evidence; which allocator produced these numbers is unestablished".into()),
        }

        match r.correctness.as_ref().and_then(|v| v.get("ok")).and_then(|v| v.as_bool()) {
            Some(true) => {}
            Some(false) => add(Severity::Error, "correctness", Some(id),
                "marked ok but the correctness gate failed; performance of a wrong answer is not a result".into()),
            None => add(Severity::Error, "correctness-missing", Some(id),
                "marked ok but carries no correctness evidence".into()),
        }

        // ⛔ Build metadata. A cell whose build.json failed to parse still
        // measured something, so it looked `ok` and simply lost its
        // binary-size column. Observed here on 2026-09-01. A silently blank
        // cell in a results table is exactly the class of failure this file
        // exists to make impossible.
        if r.build.is_none() {
            add(Severity::Error, "build-metadata-missing", Some(id),
                "marked ok but build.json is absent or unparseable; the flags and tool versions that produced this binary are unrecorded".into());
        }
        if r.binary_bytes.is_none() {
            add(Severity::Error, "binary-size-missing", Some(id),
                "marked ok but no binary size was recorded; the size column would be blank and the composite score silently absent".into());
        }
        if !r.cell.is_baseline() && r.allocator_build.is_empty() {
            add(Severity::Warn, "allocator-build-record-missing", Some(id),
                "no allocator build record (mode, PIC, flags); the archive that went into this binary is not fully described".into());
        }

        // Measurements.
        if r.measurements.is_empty() {
            add(
                Severity::Error,
                "no-measurements",
                Some(id),
                "marked ok but has no measurement documents".into(),
            );
        }
        for (w, m) in &r.measurements {
            let failures = m.get("failures").and_then(|v| v.as_u64()).unwrap_or(0);
            let samples = m.get("samples_ok").and_then(|v| v.as_u64()).unwrap_or(0);
            if failures > 0 {
                add(Severity::Error, "workload-failures", Some(id),
                    format!("workload {} recorded {} failed run(s); its timings describe runs that did not complete", w, failures));
            }
            if samples == 0 {
                add(Severity::Error, "workload-no-samples", Some(id),
                    format!("workload {} produced no usable samples. ⛔ Its median is absent, and absent must never be read as zero.", w));
            } else if samples < u64::from(r.cell.repeat) {
                add(
                    Severity::Warn,
                    "workload-short",
                    Some(id),
                    format!(
                        "workload {} produced {} samples, fewer than the {} asked for",
                        w, samples, r.cell.repeat
                    ),
                );
            }
            // A median that exists but is implausible.
            if let Some(med) = m
                .get("wall_s")
                .and_then(|v| v.get("median"))
                .and_then(|v| v.as_f64())
            {
                if !(med.is_finite() && med > 0.0) {
                    add(
                        Severity::Error,
                        "implausible-median",
                        Some(id),
                        format!(
                            "workload {} has a non-positive or non-finite median ({})",
                            w, med
                        ),
                    );
                }
            } else if samples > 0 {
                add(
                    Severity::Error,
                    "median-missing",
                    Some(id),
                    format!("workload {} has samples but no median", w),
                );
            }
            if let Some(rm) = m
                .get("wall_s")
                .and_then(|v| v.get("rel_mad"))
                .and_then(|v| v.as_f64())
            {
                if rm > NOISE_REL_MAD_WARN {
                    add(Severity::Warn, "noisy", Some(id),
                        format!("workload {} has a relative MAD of {:.1}%, above the {:.0}% threshold; differences smaller than that cannot be attributed to the allocator on this host",
                            w, rm * 100.0, NOISE_REL_MAD_WARN * 100.0));
                }
            }
        }

        // Every rankable cell needs its own control.
        if !r.cell.is_baseline() {
            let base = r.cell.baseline_id();
            let ok_base = results.iter().any(|b| b.cell.id == base && b.rankable());
            if !ok_base {
                add(Severity::Warn, "no-baseline", Some(id),
                    format!("no usable control at {}; this cell can be reported absolutely but not as a ratio", base));
            }
        }
    }

    // --- one corpus for the whole run --------------------------------------
    let mut digests: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in results.iter().filter(|r| r.rankable()) {
        if let Some(d) = r
            .correctness
            .as_ref()
            .and_then(|v| v.get("output_digest"))
            .and_then(|v| v.as_str())
        {
            digests
                .entry(d.to_string())
                .or_default()
                .push(r.cell.id.clone());
        }
    }
    // Grouped by corpus profile, since different profiles legitimately differ.
    let profiles: BTreeSet<&str> = results.iter().map(|r| r.cell.corpus.as_str()).collect();
    if profiles.len() == 1 && digests.len() > 1 {
        add(Severity::Error, "corpus-disagreement", None,
            format!("cells on one corpus profile produced {} different match-set digests: {:?}. \
                     Either the corpus differed between cells or some binaries found different files; \
                     either way the timings are not comparable.",
                digests.len(),
                digests.iter().map(|(d, c)| format!("{}={}", d, c.len())).collect::<Vec<_>>()));
    }

    // --- conditions --------------------------------------------------------
    if meta.host.emulated {
        add(Severity::Warn, "emulated", None,
            "this run's target architecture is not the host's, so it went through binfmt emulation. \
             ⛔ Emulated timings are recorded but EXCLUDED from ranking: user-mode emulation changes the \
             instruction mix and the memory behaviour, so a comparison under it measures the emulator too.".into());
    }
    if meta.host.cpu_count > 0 && meta.host.cpu_count < 2 {
        add(Severity::Warn, "single-cpu", None,
            "the host reports one CPU; the multi-threaded workloads cannot show thread-caching behaviour".into());
    }

    // --- the run is worth ranking at all -----------------------------------
    let ok_count = results.iter().filter(|r| r.rankable()).count();
    if ok_count < 2 {
        add(Severity::Error, "nothing-to-compare", None,
            format!("only {} cell(s) produced a usable result; a ranking needs at least a control and one candidate", ok_count));
    }

    f.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.check.cmp(&b.check)));
    f
}

pub fn has_errors(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity == Severity::Error)
}
