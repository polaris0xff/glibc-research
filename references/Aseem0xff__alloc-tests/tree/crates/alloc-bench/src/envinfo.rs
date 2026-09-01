//! What the measurement ran on.
//!
//! ⛔ A number without its conditions is worse than no number. This module
//! collects them once per run so the report can print them beside every table
//! and a reader can tell whether two runs are comparable at all.

use crate::exec;
use crate::model::HostInfo;
use std::collections::BTreeMap;

/// ISO-8601 UTC, computed from the clock without pulling in a date library.
pub fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // Civil-from-days (Howard Hinnant's algorithm), so this is correct across
    // leap years rather than approximately right.
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, h, mi, s)
}

fn first_field(path: &str, key: &str) -> Option<String> {
    let s = std::fs::read_to_string(path).ok()?;
    for line in s.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim() == key {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

pub fn host(runtime: &exec::Runtime, target_arch: Option<&str>) -> HostInfo {
    let kernel = exec::run("uname", &["-sr".into()], None)
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();
    let arch = exec::run("uname", &["-m".into()], None)
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();

    // /proc/cpuinfo names the field differently per architecture: x86 has
    // "model name", aarch64 usually has none at all and only "CPU part".
    let cpu_model = first_field("/proc/cpuinfo", "model name")
        .or_else(|| first_field("/proc/cpuinfo", "Model"))
        .or_else(|| {
            first_field("/proc/cpuinfo", "CPU part").map(|p| format!("aarch64 CPU part {}", p))
        })
        .unwrap_or_else(|| "unknown".into());

    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    let mem_total_kb = first_field("/proc/meminfo", "MemTotal")
        .and_then(|v| v.split_whitespace().next().and_then(|n| n.parse().ok()))
        .unwrap_or(0);

    // ⚠ Emulation. A run whose target architecture is not the host's went
    // through binfmt/QEMU. Those timings are recorded and then excluded from
    // ranking: user-mode emulation changes the instruction mix and the memory
    // behaviour, so an allocator comparison under it measures the emulator as
    // much as the allocator.
    let emulated = match target_arch {
        Some(t) => !arch.is_empty() && t != arch,
        None => false,
    };

    HostInfo {
        kernel,
        arch,
        cpu_model,
        cpu_count,
        mem_total_kb,
        container_runtime: runtime.bin.clone(),
        runtime_version: runtime.version.clone(),
        emulated,
    }
}

pub fn tool_versions() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for (name, args) in [
        ("git", vec!["--version"]),
        ("curl", vec!["--version"]),
        ("docker", vec!["--version"]),
        ("podman", vec!["--version"]),
    ] {
        if let Ok(o) = exec::run(
            name,
            &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            None,
        ) {
            if o.ok() {
                m.insert(
                    name.to_string(),
                    o.stdout.lines().next().unwrap_or("").to_string(),
                );
            }
        }
    }
    m
}

/// CI identity, so a published result can be traced to the run that made it.
pub fn ci_info() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for k in [
        "GITHUB_RUN_ID",
        "GITHUB_RUN_NUMBER",
        "GITHUB_RUN_ATTEMPT",
        "GITHUB_WORKFLOW",
        "GITHUB_SHA",
        "GITHUB_REF",
        "GITHUB_REPOSITORY",
        "RUNNER_NAME",
        "RUNNER_OS",
        "RUNNER_ARCH",
        "ImageOS",
        "ImageVersion",
    ] {
        if let Ok(v) = std::env::var(k) {
            if !v.is_empty() {
                m.insert(k.to_string(), v);
            }
        }
    }
    m
}

pub fn git_commit() -> String {
    exec::run("git", &["rev-parse".into(), "HEAD".into()], None)
        .ok()
        .filter(|o| o.ok())
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}
