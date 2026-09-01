//! Graphs, generated from the dataset.
//!
//! ⛔ No graph in this repository is drawn by hand or edited after generation.
//! Each one is a pure function of the results it names, so a reader can
//! regenerate it and get the same picture, and a stale graph is impossible
//! rather than merely discouraged.
//!
//! SVG is written directly: a plotting library would be the heaviest dependency
//! in the tree, and a bar chart is arithmetic.

use crate::rank::Row;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Colour-blind-safe, and the control is deliberately grey so the eye does not
/// read it as another candidate.
const BAR: &str = "#3b7dd8";
const BAR_SLOW: &str = "#d1495b";
const BAR_BASE: &str = "#8a8f98";
const FG: &str = "#1b1f24";
const MUTED: &str = "#5b636d";
const GRID: &str = "#d8dce1";

pub struct Chart<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub unit: &'a str,
    /// (label, value, is_baseline, note)
    pub bars: Vec<(String, f64, bool, String)>,
    /// Drawn as a dashed reference line, e.g. 1.0 for a ratio chart.
    pub reference: Option<f64>,
}

pub fn bar_chart(c: &Chart) -> String {
    let n = c.bars.len().max(1);
    let row_h = 26.0;
    let pad_top = 62.0;
    let pad_bottom = 34.0;
    let label_w = 260.0;
    let plot_w = 460.0;
    let width = label_w + plot_w + 90.0;
    let height = pad_top + row_h * n as f64 + pad_bottom;

    let max = c
        .bars
        .iter()
        .map(|b| b.1)
        .fold(c.reference.unwrap_or(0.0), f64::max)
        .max(f64::MIN_POSITIVE);
    let scale = |v: f64| (v / (max * 1.08)) * plot_w;

    let mut s = String::new();
    s.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}" viewBox="0 0 {w:.0} {h:.0}" font-family="ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif">
<rect width="100%" height="100%" fill="#ffffff"/>
<text x="16" y="26" font-size="15" font-weight="600" fill="{fg}">{title}</text>
<text x="16" y="45" font-size="11" fill="{muted}">{sub}</text>
"##,
        w = width,
        h = height,
        fg = FG,
        muted = MUTED,
        title = esc(c.title),
        sub = esc(c.subtitle)
    ));

    if let Some(r) = c.reference {
        let x = label_w + scale(r);
        s.push_str(&format!(
            r##"<line x1="{x:.1}" y1="{y0:.1}" x2="{x:.1}" y2="{y1:.1}" stroke="{g}" stroke-width="1" stroke-dasharray="4 3"/>
<text x="{x:.1}" y="{ty:.1}" font-size="10" fill="{muted}" text-anchor="middle">control</text>
"##,
            x = x,
            y0 = pad_top - 10.0,
            y1 = pad_top + row_h * n as f64,
            ty = pad_top - 14.0,
            g = GRID,
            muted = MUTED
        ));
    }

    for (i, (label, value, is_base, note)) in c.bars.iter().enumerate() {
        let y = pad_top + row_h * i as f64;
        let bw = scale(*value).max(1.0);
        let colour = if *is_base {
            BAR_BASE
        } else if c.reference.map(|r| *value > r).unwrap_or(false) {
            BAR_SLOW
        } else {
            BAR
        };
        s.push_str(&format!(
            r##"<text x="{lx}" y="{ty:.1}" font-size="11" fill="{fg}" text-anchor="end">{label}</text>
<rect x="{bx}" y="{by:.1}" width="{bw:.1}" height="{bh}" rx="2" fill="{c}"/>
<text x="{vx:.1}" y="{ty:.1}" font-size="10.5" fill="{muted}">{val}</text>
"##,
            lx = label_w - 10.0,
            ty = y + row_h * 0.68,
            fg = FG,
            label = esc(label),
            bx = label_w,
            by = y + 5.0,
            bw = bw,
            bh = row_h - 11.0,
            c = colour,
            vx = label_w + bw + 6.0,
            muted = MUTED,
            val = esc(note)
        ));
    }

    s.push_str(&format!(
        r##"<text x="16" y="{y:.1}" font-size="10" fill="{muted}">{unit}</text></svg>"##,
        y = height - 12.0,
        muted = MUTED,
        unit = esc(c.unit)
    ));
    s
}

/// Relative execution time for one comparison group.
pub fn relative_time_chart(group: &str, rows: &[Row], workload: &str) -> Option<String> {
    let bars: Vec<(String, f64, bool, String)> = rows
        .iter()
        .filter_map(|r| {
            let rel = r.rel_time?;
            let label = if r.integration == "baseline" {
                format!("{} (control)", r.allocator)
            } else {
                format!("{} / {}", r.allocator, r.integration)
            };
            let note = match r.time_s {
                Some(t) => format!("{:.3}x  ({:.3}s)", rel, t),
                None => format!("{:.3}x", rel),
            };
            Some((label, rel, r.is_baseline, note))
        })
        .collect();
    if bars.len() < 2 {
        return None;
    }
    Some(bar_chart(&Chart {
        title: &format!("Execution time relative to the control — {}", group),
        subtitle: &format!(
            "workload `{}`; median of the run's samples; lower is faster; the control is the image's own allocator",
            workload
        ),
        unit: "Generated by `alloc-bench report`. Ratios are within one image, architecture, profile and toolchain.",
        bars,
        reference: Some(1.0),
    }))
}

/// Peak RSS for one comparison group.
pub fn relative_rss_chart(group: &str, rows: &[Row], workload: &str) -> Option<String> {
    let bars: Vec<(String, f64, bool, String)> = rows
        .iter()
        .filter_map(|r| {
            let rel = r.rel_rss?;
            let label = if r.integration == "baseline" {
                format!("{} (control)", r.allocator)
            } else {
                format!("{} / {}", r.allocator, r.integration)
            };
            let note = match r.rss_kb {
                Some(v) => format!("{:.3}x  ({:.1} MiB)", rel, v / 1024.0),
                None => format!("{:.3}x", rel),
            };
            Some((label, rel, r.is_baseline, note))
        })
        .collect();
    if bars.len() < 2 {
        return None;
    }
    Some(bar_chart(&Chart {
        title: &format!("Peak resident set relative to the control — {}", group),
        subtitle: &format!(
            "workload `{}`; ru_maxrss from wait4, median of the run's samples; lower is less memory",
            workload
        ),
        unit: "Peak RSS is what sets a container's memory limit. It is not the allocator's own idea of heap size.",
        bars,
        reference: Some(1.0),
    }))
}

/// Binary size, absolute. A ratio chart would hide that every row is within a
/// few per cent of the same handful of megabytes.
pub fn size_chart(group: &str, rows: &[Row]) -> Option<String> {
    let bars: Vec<(String, f64, bool, String)> = rows
        .iter()
        .filter_map(|r| {
            let b = r.size_bytes?;
            let mib = b as f64 / (1024.0 * 1024.0);
            let label = if r.integration == "baseline" {
                format!("{} (control)", r.allocator)
            } else {
                format!("{} / {}", r.allocator, r.integration)
            };
            Some((label, mib, r.is_baseline, format!("{:.2} MiB", mib)))
        })
        .collect();
    if bars.len() < 2 {
        return None;
    }
    Some(bar_chart(&Chart {
        title: &format!("Binary size — {}", group),
        subtitle: "unstripped, with symbols retained so the identity oracle can read the binary",
        unit:
            "⚠ These binaries keep .symtab on purpose; a shipped build would strip and be smaller.",
        bars,
        reference: None,
    }))
}
