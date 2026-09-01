//! Build the images, run every cell, collect the evidence.

use crate::exec::Runtime;
use crate::model::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Runner {
    pub rt: Runtime,
    pub repo_root: PathBuf,
    pub out_root: PathBuf,
    pub cache_root: PathBuf,
    pub corpus_seed: u64,
    pub keep_going: bool,
}

fn dockerfile_for(distro: &str) -> &'static str {
    match distro {
        "alpine" => "images/alpine.Dockerfile",
        "debian" => "images/debian.Dockerfile",
        _ => "images/arch.Dockerfile",
    }
}

/// The upstream image a distribution is built from, per architecture.
///
/// ⚠ Arch is the exception and it is the whole reason this function exists
/// rather than a format string: upstream Arch publishes amd64 only.
fn base_image(distro: &str, arch: &str) -> Result<String, String> {
    Ok(match (distro, arch) {
        ("alpine", _) => "alpine:latest".into(),
        ("debian", _) => "debian:latest".into(),
        ("archlinux", "x86_64") => "archlinux:latest".into(),
        ("archlinux", other) => {
            return Err(format!(
                "upstream Arch publishes no {} image; the planner should have renamed this cell to archlinuxarm",
                other
            ))
        }
        ("archlinuxarm", _) => "menci/archlinuxarm:base-devel".into(),
        (d, _) => return Err(format!("no base image known for distribution {:?}", d)),
    })
}

/// An HTTPS proxy for the image build and for the cell containers.
///
/// ⚠ Deliberately its own variable rather than the ambient `HTTPS_PROXY`. A
/// host proxy is usually on `127.0.0.1`, which inside a container is the
/// container. Inheriting it silently gives every network call in the build a
/// connection refused that looks like an upstream outage. The operator names a
/// container-reachable address or gets none.
fn proxy_env() -> Vec<(String, String)> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("ALLOC_TESTS_HTTPS_PROXY") {
        if !p.is_empty() {
            v.push(("HTTPS_PROXY".to_string(), p.clone()));
            v.push(("https_proxy".to_string(), p));
        }
    }
    if let Ok(n) = std::env::var("ALLOC_TESTS_NO_PROXY") {
        if !n.is_empty() {
            v.push(("NO_PROXY".to_string(), n.clone()));
            v.push(("no_proxy".to_string(), n));
        }
    }
    v
}

fn platform_for(arch: &str) -> &'static str {
    match arch {
        "aarch64" => "linux/arm64",
        _ => "linux/amd64",
    }
}

impl Runner {
    pub fn image_tag(&self, distro: &str, arch: &str) -> String {
        format!("alloc-tests/{}-{}:local", distro, arch)
    }

    /// Build (or reuse) the image for one (distribution, architecture).
    pub fn ensure_image(
        &self,
        distro: &str,
        arch: &str,
    ) -> Result<(String, BTreeMap<String, String>), String> {
        let tag = self.image_tag(distro, arch);
        let log = self
            .out_root
            .join("logs")
            .join(format!("image-{}-{}.log", distro, arch));
        let base = base_image(distro, arch)?;

        let mut args: Vec<String> = vec![
            "build".into(),
            "--platform".into(),
            platform_for(arch).into(),
            "--build-arg".into(),
            format!("BASE_IMAGE={}", base),
            "-f".into(),
            self.repo_root
                .join(dockerfile_for(distro))
                .display()
                .to_string(),
            "-t".into(),
            tag.clone(),
        ];
        for (k, v) in proxy_env() {
            args.push("--build-arg".into());
            args.push(format!("{}={}", k, v));
        }
        args.push(self.repo_root.display().to_string());
        let out = self.rt.cmd(&args, Some(&log))?;
        if !out.ok() {
            return Err(format!(
                "image build failed for {}/{} (see {}):\n{}",
                distro,
                arch,
                log.display(),
                out.tail(25)
            ));
        }

        // The image records its own environment during the build; read it back
        // rather than re-deriving it here, so the recorded values are the ones
        // that were actually present when it was built.
        let env = self.image_env(&tag, arch).unwrap_or_default();
        Ok((tag, env))
    }

    fn image_env(&self, tag: &str, arch: &str) -> Option<BTreeMap<String, String>> {
        let out = self
            .rt
            .cmd(
                &[
                    "run".into(),
                    "--rm".into(),
                    "--platform".into(),
                    platform_for(arch).into(),
                    tag.into(),
                    "cat".into(),
                    "/opt/alloc-tests/image-env.txt".into(),
                ],
                None,
            )
            .ok()?;
        if !out.ok() {
            return None;
        }
        let mut m = BTreeMap::new();
        for line in out.stdout.lines() {
            if let Some((k, v)) = line.split_once('=') {
                m.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        Some(m)
    }

    /// Run one cell in its image and read back everything it wrote.
    pub fn run_cell(
        &self,
        cell: &Cell,
        tag: &str,
        image_digest: Option<String>,
        image_env: BTreeMap<String, String>,
        lock: &Lock,
    ) -> CellResult {
        let mut result = CellResult {
            cell: cell.clone(),
            outcome: "unsupported".into(),
            detail: cell.reason.clone(),
            image_digest,
            image_env,
            build: None,
            allocator_build: BTreeMap::new(),
            identity: None,
            correctness: None,
            aslr: None,
            binary_bytes: None,
            build_seconds: None,
            measurements: BTreeMap::new(),
        };

        if cell.status == "unsupported" {
            // Planned-unsupported cells are carried through untouched. The
            // reason came from allocators.toml and is the deliverable.
            return result;
        }

        let cell_out = self.out_root.join("cells").join(&cell.id);
        let _ = std::fs::create_dir_all(&cell_out);

        let rg = match lock.entries.get("ripgrep") {
            Some(e) => e.clone(),
            None => {
                result.outcome = "build_failed".into();
                result.detail = Some(
                    "allocators.lock.json has no ripgrep entry; run `alloc-bench update`".into(),
                );
                return result;
            }
        };

        let mut envs: Vec<(String, String)> = vec![
            ("CELL_ID".into(), cell.id.clone()),
            ("OUTDIR".into(), "/out".into()),
            ("CACHE".into(), "/cache".into()),
            ("ALLOCATOR".into(), cell.allocator.clone()),
            ("INTEGRATION".into(), cell.integration.clone()),
            ("PROFILE".into(), cell.profile.clone()),
            ("TOOLCHAIN".into(), cell.toolchain.clone()),
            ("LIBC".into(), cell.libc.clone()),
            ("TARGET_ARCH".into(), cell.arch.clone()),
            ("CORPUS_PROFILE".into(), cell.corpus.clone()),
            ("CORPUS_SEED".into(), self.corpus_seed.to_string()),
            ("REPEAT".into(), cell.repeat.to_string()),
            ("RG_REPO".into(), rg.repo.clone()),
            ("RG_COMMIT".into(), rg.commit.clone()),
        ];
        if cell.allocator != "system" {
            match lock.entries.get(&cell.allocator) {
                Some(e) => {
                    envs.push(("ALLOC_REPO".into(), e.repo.clone()));
                    envs.push(("ALLOC_COMMIT".into(), e.commit.clone()));
                }
                None => {
                    result.outcome = "build_failed".into();
                    result.detail = Some(format!(
                        "allocators.lock.json has no entry for {}; run `alloc-bench update`",
                        cell.allocator
                    ));
                    return result;
                }
            }
        }

        // Cache is per (distribution, architecture): an archive built against
        // musl must never be handed to a glibc cell.
        let cache = self
            .cache_root
            .join(format!("{}-{}", cell.distro, cell.arch));
        let _ = std::fs::create_dir_all(&cache);

        let mut args: Vec<String> = vec![
            "run".into(),
            "--rm".into(),
            "--platform".into(),
            platform_for(&cell.arch).into(),
            // ⚠ The measurement must not compete with other cells for CPU, so
            // cells are run one at a time and the container gets the whole
            // host. Any cgroup limit here would become part of the result and
            // would differ between a laptop and a runner.
            "-v".into(),
            format!("{}:/out", cell_out.display()),
            "-v".into(),
            format!("{}:/cache", cache.display()),
        ];
        for (k, v) in envs.iter().cloned().chain(proxy_env()) {
            args.push("-e".into());
            args.push(format!("{}={}", k, v));
        }
        args.push(tag.to_string());
        args.push("sh".into());
        args.push("/opt/alloc-tests/scripts/build/run-cell.sh".into());

        let log = self.out_root.join("logs").join(format!("{}.log", cell.id));
        let out = match self.rt.cmd(&args, Some(&log)) {
            Ok(o) => o,
            Err(e) => {
                result.outcome = "build_failed".into();
                result.detail = Some(e);
                return result;
            }
        };

        // Read whatever the cell wrote, whether it succeeded or not: a failed
        // cell's identity and build metadata are exactly what makes the failure
        // diagnosable later.
        let read_json = |name: &str| -> Option<serde_json::Value> {
            let s = std::fs::read_to_string(cell_out.join(name)).ok()?;
            serde_json::from_str(&s).ok()
        };
        result.build = read_json("build.json");
        // The allocator's own build record, kept as files by the recipe.
        let mut alloc_meta: BTreeMap<String, String> = BTreeMap::new();
        if let Ok(t) = std::fs::read_to_string(cell_out.join("alloc-meta.env")) {
            for line in t.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    alloc_meta.insert(k.trim().into(), v.trim().trim_matches('\'').into());
                }
            }
        }
        if let Ok(t) = std::fs::read_to_string(cell_out.join("alloc-build-flags.txt")) {
            alloc_meta.insert("build_flags".into(), t.trim().to_string());
        }
        if !alloc_meta.is_empty() {
            result.allocator_build = alloc_meta;
        }
        result.identity = read_json("identity.json");
        result.correctness = read_json("correctness.json");
        result.aslr = read_json("aslr.json");
        result.binary_bytes = result
            .build
            .as_ref()
            .and_then(|b| b.get("binary_bytes"))
            .and_then(|v| v.as_u64());
        result.build_seconds = std::fs::read_to_string(cell_out.join("build_seconds"))
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok());

        for entry in std::fs::read_dir(&cell_out).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(w) = name
                .strip_prefix("measure-")
                .and_then(|n| n.strip_suffix(".json"))
            {
                if let Some(v) = read_json(&name) {
                    result.measurements.insert(w.to_string(), v);
                }
            }
        }

        let status = std::fs::read_to_string(cell_out.join("status"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let reason = std::fs::read_to_string(cell_out.join("reason"))
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty());

        result.outcome = if !status.is_empty() {
            status
        } else if out.unsupported() {
            "unsupported".into()
        } else if out.ok() {
            "ok".into()
        } else {
            "build_failed".into()
        };
        result.detail = reason.or_else(|| {
            if result.outcome == "ok" {
                None
            } else {
                Some(out.tail(10))
            }
        });

        result
    }

    pub fn write_result(&self, r: &CellResult) -> Result<(), String> {
        let dir = self.out_root.join("results");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let p = dir.join(format!("{}.json", r.cell.id));
        let s = serde_json::to_string_pretty(r).map_err(|e| e.to_string())?;
        std::fs::write(&p, s).map_err(|e| format!("{}: {}", p.display(), e))
    }
}

pub fn load_results(dir: &Path) -> Result<Vec<CellResult>, String> {
    let mut out = Vec::new();
    let rd = std::fs::read_dir(dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {}", p.display(), e))?;
        match serde_json::from_str::<CellResult>(&s) {
            Ok(r) => out.push(r),
            Err(err) => return Err(format!("{}: not a CellResult: {}", p.display(), err)),
        }
    }
    out.sort_by(|a, b| a.cell.id.cmp(&b.cell.id));
    Ok(out)
}
