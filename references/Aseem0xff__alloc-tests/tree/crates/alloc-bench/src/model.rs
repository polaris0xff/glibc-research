//! The types the whole pipeline agrees on.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// allocators/allocators.toml

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub schema_version: u32,
    #[serde(rename = "allocator")]
    pub allocators: Vec<AllocatorSpec>,
}

/// ⚠ Several fields here are parsed and not read by this binary. That is
/// deliberate and it is not dead weight: `allocators.toml` is the registry a
/// PERSON reads to learn how an allocator is built and why a mechanism is
/// unavailable, and parsing every field means a typo in one is a load error
/// rather than a silently ignored line. `serde` would accept unknown keys
/// otherwise.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct AllocatorSpec {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub track: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub variants: Option<Vec<String>>,
    #[serde(default)]
    pub bazel_version: Option<String>,
    pub integrations: Vec<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub build_notes: BTreeMap<String, String>,
    #[serde(default)]
    pub integration_notes: BTreeMap<String, String>,
    /// Why a mechanism this allocator does not list is impossible. Keyed by
    /// mechanism id. Absent means "no reason recorded", which the planner
    /// reports as such rather than inventing one.
    #[serde(default)]
    pub unsupported_notes: BTreeMap<String, String>,
}

impl AllocatorSpec {
    pub fn is_baseline(&self) -> bool {
        self.kind.as_deref() == Some("builtin")
    }
    pub fn supports(&self, integration: &str) -> bool {
        self.integrations.iter().any(|i| i == integration)
    }
    pub fn why_not(&self, integration: &str) -> String {
        self.unsupported_notes
            .get(integration)
            .cloned()
            .map(|s| s.trim().replace('\n', " "))
            .unwrap_or_else(|| {
                format!(
                    "allocators.toml does not list `{}` among this allocator's integrations, and records no reason. \
                     That absence is itself a gap: add an unsupported_notes entry.",
                    integration
                )
            })
    }
}

// ---------------------------------------------------------------------------
// allocators/allocators.lock.json

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Lock {
    pub schema_version: u32,
    /// When `alloc-bench update` last resolved these. Not the benchmark date.
    pub resolved_at: String,
    pub entries: BTreeMap<String, LockEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockEntry {
    pub repo: String,
    /// `release`, `prerelease` or `branch` -- how the ref below was chosen.
    pub kind: String,
    /// The tag or branch name. `-` when the upstream publishes neither.
    pub reference: String,
    /// ⭐ The one field that makes a result reproducible.
    pub commit: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// benchmarks/matrix.toml

#[derive(Debug, Deserialize, Clone)]
pub struct MatrixFile {
    pub schema_version: u32,
    #[serde(rename = "suite")]
    pub suites: Vec<Suite>,
}

impl MatrixFile {
    /// Every architecture any suite names. This is the universe a `--arch`
    /// filter is checked against — being absent from one suite is legitimate,
    /// being absent from all of them is a typo.
    pub fn all_arches(&self) -> Vec<String> {
        self.suites.iter().flat_map(|s| s.arches.clone()).collect()
    }

    /// Every distribution any suite names.
    ///
    /// ⚠ The aarch64 rename is included deliberately: a suite says `archlinux`
    /// but on aarch64 the cell is built from Arch Linux ARM and is labelled
    /// `archlinuxarm` (see `plan::effective_distro`). Both spellings therefore
    /// have to be accepted, or `--distro archlinuxarm` — which is what the cell
    /// ids actually contain — would be rejected as unknown.
    pub fn all_distros(&self) -> Vec<String> {
        let mut v: Vec<String> = self.suites.iter().flat_map(|s| s.distros.clone()).collect();
        for s in &self.suites {
            for d in &s.distros {
                for a in &s.arches {
                    v.push(crate::plan::effective_distro(d, a));
                }
            }
        }
        v
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Suite {
    pub id: String,
    pub why: String,
    pub distros: Vec<String>,
    pub arches: Vec<String>,
    pub allocators: Vec<String>,
    pub integrations: Vec<String>,
    pub profiles: Vec<String>,
    pub toolchains: Vec<String>,
    #[serde(default = "default_corpus")]
    pub corpus: String,
    #[serde(default = "default_repeat")]
    pub repeat: u32,
}

fn default_corpus() -> String {
    "standard".into()
}
fn default_repeat() -> u32 {
    10
}

// ---------------------------------------------------------------------------
// A matrix cell: one experiment.

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cell {
    pub id: String,
    pub suite: String,
    pub distro: String,
    pub arch: String,
    pub libc: String,
    pub allocator: String,
    pub integration: String,
    pub profile: String,
    pub toolchain: String,
    pub corpus: String,
    pub repeat: u32,
    /// `planned`, or `unsupported` with `reason` filled in. An unsupported cell
    /// is carried through the whole pipeline and printed, never dropped.
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
}

impl Cell {
    // ⚠ The rust target triple, the expected link kind and whether a mechanism
    // is a replacement are all derived by scripts/build/run-cell.sh, which is
    // the single place that decides them. Duplicating them here as Rust helpers
    // would be two places for one rule to drift, and the shell copy is the one
    // that actually configures the build.
    pub fn slug(
        distro: &str,
        arch: &str,
        allocator: &str,
        integration: &str,
        profile: &str,
        toolchain: &str,
    ) -> String {
        format!(
            "{}-{}-{}-{}-{}-{}",
            distro, arch, allocator, integration, profile, toolchain
        )
    }

    pub fn is_baseline(&self) -> bool {
        self.allocator == "system"
    }

    /// The control this cell is compared against: same everything, system
    /// allocator. A ratio against any other row would be comparing two things
    /// that differ in more than the allocator.
    pub fn baseline_id(&self) -> String {
        Cell::slug(
            &self.distro,
            &self.arch,
            "system",
            "baseline",
            &self.profile,
            &self.toolchain,
        )
    }
}

// ---------------------------------------------------------------------------
// Results.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunMeta {
    pub run_id: String,
    pub started_at: String,
    pub suites: Vec<String>,
    pub host: HostInfo,
    pub tool_versions: BTreeMap<String, String>,
    pub lock: Lock,
    pub ci: BTreeMap<String, String>,
    pub corpus_seed: u64,
    pub git_commit: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HostInfo {
    pub kernel: String,
    pub arch: String,
    pub cpu_model: String,
    pub cpu_count: usize,
    pub mem_total_kb: u64,
    pub container_runtime: String,
    pub runtime_version: String,
    /// ⚠ Populated when the host is not the architecture being measured, i.e.
    /// the run went through binfmt emulation. Emulated timings are recorded and
    /// then EXCLUDED from ranking; see docs/methodology.md.
    pub emulated: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CellResult {
    pub cell: Cell,
    /// `ok` | `unsupported` | `build_failed` | `identity_failed`
    /// | `correctness_failed` | `measurement_failed`
    pub outcome: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub image_digest: Option<String>,
    #[serde(default)]
    pub image_env: BTreeMap<String, String>,
    #[serde(default)]
    pub build: Option<serde_json::Value>,
    /// How the allocator itself was built: mode, PIC, libc, and the exact flag
    /// list. Read from the recipe's own files rather than interpolated into
    /// JSON by a shell.
    #[serde(default)]
    pub allocator_build: BTreeMap<String, String>,
    #[serde(default)]
    pub identity: Option<serde_json::Value>,
    #[serde(default)]
    pub correctness: Option<serde_json::Value>,
    #[serde(default)]
    pub aslr: Option<serde_json::Value>,
    #[serde(default)]
    pub binary_bytes: Option<u64>,
    #[serde(default)]
    pub build_seconds: Option<f64>,
    /// workload -> the measurement document `alloc-runner measure` emitted.
    #[serde(default)]
    pub measurements: BTreeMap<String, serde_json::Value>,
}

impl CellResult {
    pub fn rankable(&self) -> bool {
        self.outcome == "ok"
    }
    /// Median wall seconds for one workload, or None when it did not run.
    /// ⛔ None is never coerced to 0: a missing measurement that read as zero
    /// would rank as infinitely fast.
    pub fn median_wall(&self, workload: &str) -> Option<f64> {
        self.measurements
            .get(workload)?
            .get("wall_s")?
            .get("median")?
            .as_f64()
    }
    pub fn median_rss(&self, workload: &str) -> Option<f64> {
        self.measurements
            .get(workload)?
            .get("maxrss_kb")?
            .get("median")?
            .as_f64()
    }
    pub fn rel_mad(&self, workload: &str) -> Option<f64> {
        self.measurements
            .get(workload)?
            .get("wall_s")?
            .get("rel_mad")?
            .as_f64()
    }
}
