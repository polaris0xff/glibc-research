//! Ranking, computed from the recorded samples and from nothing else.
//!
//! ⛔ NO ROW IS PLACED BY JUDGEMENT. Everything here is arithmetic over the
//! measurement documents; the interpretation belongs in the report's prose and
//! in docs/, where a reader can disagree with it while still trusting the
//! table.
//!
//! ⚠ WHAT "BEST" MEANS HERE, stated because a single number always hides
//! something. The PRIMARY ranking is execution time on one named workload,
//! relative to the control in the same image and profile. That is the project's
//! stated practical goal. Peak RSS, binary size and startup are reported beside
//! it, never folded into it silently.
//!
//! A composite IS offered, because "which one should I ship" is a real
//! question, and its formula is printed with it every time. It is a secondary
//! column, not the ranking.

use crate::model::CellResult;
use crate::validate::PRIMARY_WORKLOAD;
use std::collections::BTreeMap;

/// The composite's weights.
///
/// RATIONALE, so this is arguable rather than arbitrary: for a container binary
/// the dominant cost is how long the command takes, so time carries most of the
/// weight. Peak RSS is next, because it is what sets a container's memory limit
/// and therefore what it costs to run. Binary size is last and small: it is
/// paid once at pull time, not per invocation.
///
/// ⛔ These weights are a CHOICE. Every component is published separately so a
/// reader who weighs them differently can re-rank from the same data.
pub const W_TIME: f64 = 0.60;
pub const W_RSS: f64 = 0.30;
pub const W_SIZE: f64 = 0.10;

pub const COMPOSITE_FORMULA: &str =
    "composite = 0.60 x (time / baseline_time) + 0.30 x (peak_rss / baseline_peak_rss) + 0.10 x (binary_bytes / baseline_binary_bytes); lower is better, 1.000 is the control";

#[derive(Debug, Clone)]
pub struct Row {
    pub cell_id: String,
    pub distro: String,
    pub arch: String,
    pub allocator: String,
    pub integration: String,
    pub profile: String,
    pub toolchain: String,
    pub time_s: Option<f64>,
    pub rel_time: Option<f64>,
    pub rel_mad: Option<f64>,
    pub rss_kb: Option<f64>,
    pub rel_rss: Option<f64>,
    pub size_bytes: Option<u64>,
    pub rel_size: Option<f64>,
    pub startup_s: Option<f64>,
    pub composite: Option<f64>,
    pub is_baseline: bool,
}

/// A comparison group: everything held constant except the allocator and the
/// mechanism. Rows from different groups are never ranked against each other.
pub fn group_key(r: &CellResult) -> String {
    format!(
        "{} / {} / {} / {}",
        r.cell.distro, r.cell.arch, r.cell.profile, r.cell.toolchain
    )
}

pub fn build_rows(results: &[CellResult], workload: &str) -> BTreeMap<String, Vec<Row>> {
    let by_id: BTreeMap<&str, &CellResult> =
        results.iter().map(|r| (r.cell.id.as_str(), r)).collect();
    let mut groups: BTreeMap<String, Vec<Row>> = BTreeMap::new();

    for r in results.iter().filter(|r| r.rankable()) {
        let base = by_id
            .get(r.cell.baseline_id().as_str())
            .copied()
            .filter(|b| b.rankable());

        let time_s = r.median_wall(workload);
        let rss_kb = r.median_rss(workload);
        let size_bytes = r.binary_bytes;

        // ⛔ Every ratio is Option all the way down. A missing baseline gives a
        // missing ratio, never a 1.0 and never a 0.0. A `-` in a table is the
        // honest rendering of an unknown.
        let rel_time = match (time_s, base.and_then(|b| b.median_wall(workload))) {
            (Some(t), Some(bt)) if bt > 0.0 => Some(t / bt),
            _ => None,
        };
        let rel_rss = match (rss_kb, base.and_then(|b| b.median_rss(workload))) {
            (Some(v), Some(bv)) if bv > 0.0 => Some(v / bv),
            _ => None,
        };
        let rel_size = match (size_bytes, base.and_then(|b| b.binary_bytes)) {
            (Some(v), Some(bv)) if bv > 0 => Some(v as f64 / bv as f64),
            _ => None,
        };
        let composite = match (rel_time, rel_rss, rel_size) {
            (Some(t), Some(m), Some(s)) => Some(W_TIME * t + W_RSS * m + W_SIZE * s),
            _ => None,
        };

        groups.entry(group_key(r)).or_default().push(Row {
            cell_id: r.cell.id.clone(),
            distro: r.cell.distro.clone(),
            arch: r.cell.arch.clone(),
            allocator: r.cell.allocator.clone(),
            integration: r.cell.integration.clone(),
            profile: r.cell.profile.clone(),
            toolchain: r.cell.toolchain.clone(),
            time_s,
            rel_time,
            rel_mad: r.rel_mad(workload),
            rss_kb,
            rel_rss,
            size_bytes,
            rel_size,
            startup_s: r.median_wall("startup"),
            composite,
            is_baseline: r.cell.is_baseline(),
        });
    }

    // Fastest first; rows with no time sort last rather than first, which is
    // what a naive `sort_by(partial_cmp)` over `Option` would do.
    for rows in groups.values_mut() {
        rows.sort_by(|a, b| match (a.time_s, b.time_s) {
            (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.cell_id.cmp(&b.cell_id),
        });
    }
    groups
}

/// The overall statement, per group: which allocator was fastest and by how
/// much, or why no statement can be made.
pub struct Verdict {
    pub group: String,
    pub winner: Option<String>,
    pub margin_pct: Option<f64>,
    /// Set when the lead is within the run's own noise, in which case the
    /// ordering is not evidence of anything.
    pub within_noise: bool,
    pub note: String,
}

pub fn verdicts(groups: &BTreeMap<String, Vec<Row>>) -> Vec<Verdict> {
    let mut out = Vec::new();
    for (g, rows) in groups {
        let ranked: Vec<&Row> = rows.iter().filter(|r| r.time_s.is_some()).collect();
        if ranked.len() < 2 {
            out.push(Verdict {
                group: g.clone(),
                winner: None,
                margin_pct: None,
                within_noise: false,
                note: format!(
                    "{} usable row(s): not enough to rank. A group needs a control and at least one candidate.",
                    ranked.len()
                ),
            });
            continue;
        }
        let first = ranked[0];
        let second = ranked[1];
        let margin = match (first.time_s, second.time_s) {
            (Some(a), Some(b)) if a > 0.0 => Some((b - a) / a * 100.0),
            _ => None,
        };

        // ⚠ A lead smaller than the measurement's own spread is not a lead.
        // Reporting it as one is how a benchmark manufactures a winner.
        let noise = first
            .rel_mad
            .unwrap_or(0.0)
            .max(second.rel_mad.unwrap_or(0.0))
            * 100.0;
        let within_noise = margin.map(|m| m <= noise).unwrap_or(true);

        let note = if within_noise {
            format!(
                "the lead of {:.1}% is within this run's own spread ({:.1}% relative MAD), so the ordering between the top rows is not established here",
                margin.unwrap_or(0.0),
                noise
            )
        } else {
            format!(
                "{} is {:.1}% faster than the next row, which is outside the run's spread of {:.1}%",
                first.allocator,
                margin.unwrap_or(0.0),
                noise
            )
        };

        out.push(Verdict {
            group: g.clone(),
            winner: Some(format!("{} ({})", first.allocator, first.integration)),
            margin_pct: margin,
            within_noise,
            note,
        });
    }
    out
}

pub fn primary_workload() -> &'static str {
    PRIMARY_WORKLOAD
}
