//! Process execution and the container runtime.
//!
//! Every external command goes through here so that what ran, and what it
//! printed, is capturable and loggable. A command whose output is thrown away
//! is a failure nobody can diagnose from an artefact.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn ok(&self) -> bool {
        self.code == 0
    }
    /// The runner's convention: 3 means "this configuration is unsupported",
    /// which is a result rather than a failure.
    pub fn unsupported(&self) -> bool {
        self.code == 3
    }
    pub fn tail(&self, n: usize) -> String {
        let combined = format!("{}\n{}", self.stdout.trim_end(), self.stderr.trim_end());
        combined
            .lines()
            .filter(|l| !l.trim().is_empty())
            .rev()
            .take(n)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn run(program: &str, args: &[String], log: Option<&Path>) -> Result<Output, String> {
    let t0 = Instant::now();
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("running {}: {}", program, e))?;
    let seconds = t0.elapsed().as_secs_f64();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    if let Some(p) = log {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            let _ = writeln!(
                f,
                "\n$ {} {}\n--- exit {} in {:.1}s ---\n{}\n{}",
                program,
                args.join(" "),
                out.status.code().unwrap_or(-1),
                seconds,
                stdout,
                stderr
            );
        }
    }

    Ok(Output {
        code: out.status.code().unwrap_or(-1),
        stdout,
        stderr,
    })
}

/// `curl`, used for every HTTP fetch.
///
/// ⚠ It honours the caller's proxy environment, which is what makes this work
/// on a restricted network where a linked TLS stack would not. `--fail` so an
/// HTTP error is an error rather than an HTML body parsed as JSON.
pub fn http_get(url: &str) -> Result<String, String> {
    let args: Vec<String> = vec![
        "-sSL".into(),
        "--fail".into(),
        "--retry".into(),
        "3".into(),
        "--max-time".into(),
        "120".into(),
        url.into(),
    ];
    let out = run("curl", &args, None)?;
    if !out.ok() {
        return Err(format!("GET {} failed: {}", url, out.stderr.trim()));
    }
    Ok(out.stdout)
}

// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Runtime {
    pub bin: String,
    pub version: String,
}

impl Runtime {
    /// Prefer whatever the caller asked for; otherwise take docker, then
    /// podman.
    ///
    /// ⚠ Presence on PATH is not the probe. A `docker` binary with no daemon
    /// behind it answers `--version` happily and fails on the first real
    /// command, so `info` is used: it needs the daemon.
    pub fn detect(preferred: Option<&str>) -> Result<Runtime, String> {
        let candidates: Vec<&str> = match preferred {
            Some(p) => vec![p],
            None => vec!["docker", "podman"],
        };
        let mut tried = Vec::new();
        for c in candidates {
            match run(
                c,
                &[
                    "info".into(),
                    "--format".into(),
                    "{{.ServerVersion}}".into(),
                ],
                None,
            ) {
                Ok(o) if o.ok() => {
                    let v = o.stdout.trim().to_string();
                    return Ok(Runtime {
                        bin: c.to_string(),
                        version: if v.is_empty() { "unknown".into() } else { v },
                    });
                }
                Ok(o) => tried.push(format!("{}: {}", c, o.tail(2))),
                Err(e) => tried.push(format!("{}: {}", c, e)),
            }
        }
        Err(format!(
            "no working container runtime. Tried:\n  {}",
            tried.join("\n  ")
        ))
    }

    pub fn cmd(&self, args: &[String], log: Option<&Path>) -> Result<Output, String> {
        run(&self.bin, args, log)
    }

    /// The digest of an image, so a result names an immutable thing.
    pub fn digest(&self, image: &str) -> Option<String> {
        let out = run(
            &self.bin,
            &[
                "image".into(),
                "inspect".into(),
                "--format".into(),
                "{{index .RepoDigests 0}}".into(),
                image.into(),
            ],
            None,
        )
        .ok()?;
        if out.ok() {
            let d = out.stdout.trim().to_string();
            if !d.is_empty() && d != "<no value>" {
                return Some(d);
            }
        }
        // A locally built image has no RepoDigest. Its content Id still pins
        // it for this run, and is labelled as such rather than as a registry
        // digest.
        let out = run(
            &self.bin,
            &[
                "image".into(),
                "inspect".into(),
                "--format".into(),
                "{{.Id}}".into(),
                image.into(),
            ],
            None,
        )
        .ok()?;
        if out.ok() {
            let d = out.stdout.trim().to_string();
            if !d.is_empty() {
                return Some(format!("local:{}", d));
            }
        }
        None
    }
}
