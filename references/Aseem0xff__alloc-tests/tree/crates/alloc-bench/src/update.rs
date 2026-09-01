//! Resolve "latest" to an exact commit, once, and write it down.
//!
//! ⭐ THE POINT OF THIS FILE. The project is supposed to benchmark current
//! upstream, not a version frozen in 2022. It is also supposed to be
//! reproducible. Those are only compatible if "latest" is resolved at a known
//! moment and the ANSWER is committed. `alloc-bench update` does the resolving;
//! everything downstream reads `allocators.lock.json` and never asks a network
//! what "latest" means.
//!
//! ⚠ The lock is a normal reviewable diff. A run against a lock file that
//! differs from the committed one is a different experiment, and CI says so.

use crate::exec;
use crate::model::*;
use std::collections::BTreeMap;

/// GitHub's API, reached directly or through the read-only proxy.
///
/// ⚠ The proxy is used when the direct route fails, not instead of it. It
/// carries none of the caller's credentials and is read-only.
const API_DIRECT: &str = "https://api.github.com";
const API_PROXY: &str = "https://api.gh.pkgforge.dev";

fn owner_repo(url: &str) -> Option<String> {
    let s = url.trim_end_matches('/').trim_end_matches(".git");
    let rest = s.split("github.com/").nth(1)?;
    let mut it = rest.split('/');
    Some(format!("{}/{}", it.next()?, it.next()?))
}

fn get_json(path: &str) -> Result<serde_json::Value, String> {
    let mut last = String::new();
    for base in [API_DIRECT, API_PROXY] {
        match exec::http_get(&format!("{}/{}", base, path)) {
            Ok(body) => match serde_json::from_str(&body) {
                Ok(v) => return Ok(v),
                Err(e) => last = format!("{}: bad JSON: {}", base, e),
            },
            Err(e) => last = e,
        }
    }
    Err(format!("could not fetch {}: {}", path, last))
}

pub fn resolve(manifest: &Manifest) -> Result<Lock, String> {
    let mut entries: BTreeMap<String, LockEntry> = BTreeMap::new();

    for a in &manifest.allocators {
        if a.is_baseline() {
            continue;
        }
        let Some(repo_url) = &a.repo else { continue };
        let Some(or) = owner_repo(repo_url) else {
            return Err(format!(
                "{}: repo url is not a github.com URL: {}",
                a.id, repo_url
            ));
        };

        let track = a.track.as_deref().unwrap_or("latest-release");
        let (kind, reference, published) = match track {
            "branch" => {
                let branch = a.branch.clone().unwrap_or_else(|| "main".into());
                ("branch".to_string(), branch, None)
            }
            "latest-any" | "latest-release" => {
                let want_pre = track == "latest-any";
                let rels = get_json(&format!("repos/{}/releases?per_page=30", or))?;
                let arr = rels.as_array().cloned().unwrap_or_default();
                let pick = arr.iter().find(|r| {
                    let draft = r.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
                    let pre = r
                        .get("prerelease")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    !draft && (want_pre || !pre)
                });
                match pick {
                    Some(r) => (
                        if r.get("prerelease")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            "prerelease".to_string()
                        } else {
                            "release".to_string()
                        },
                        r.get("tag_name")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| format!("{}: release has no tag_name", a.id))?
                            .to_string(),
                        r.get("published_at")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    ),
                    None => {
                        // ⛔ No release is not an error to paper over with a
                        // branch head silently: it changes what `track` means,
                        // so the lock records that it happened.
                        let branch = a.branch.clone().unwrap_or_else(|| "main".into());
                        ("branch-fallback".to_string(), branch, None)
                    }
                }
            }
            other => return Err(format!("{}: unknown track {:?}", a.id, other)),
        };

        let commit_doc = get_json(&format!("repos/{}/commits/{}", or, reference))?;
        let commit = commit_doc
            .get("sha")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{}: could not resolve {} to a commit", a.id, reference))?
            .to_string();

        let note = if kind == "branch-fallback" {
            Some(format!(
                "upstream publishes no release; tracking branch {} head",
                reference
            ))
        } else {
            None
        };

        entries.insert(
            a.id.clone(),
            LockEntry {
                repo: repo_url.clone(),
                kind,
                reference,
                commit,
                published_at: published,
                note,
            },
        );
    }

    // ripgrep is pinned in the same file, for the same reason: the application
    // version is as much a part of a result as the allocator version.
    let rg = get_json("repos/BurntSushi/ripgrep/releases/latest")?;
    let tag = rg
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or("ripgrep release has no tag_name")?
        .to_string();
    let rg_commit = get_json(&format!("repos/BurntSushi/ripgrep/commits/{}", tag))?
        .get("sha")
        .and_then(|v| v.as_str())
        .ok_or("could not resolve the ripgrep tag to a commit")?
        .to_string();
    entries.insert(
        "ripgrep".into(),
        LockEntry {
            repo: "https://github.com/BurntSushi/ripgrep".into(),
            kind: "release".into(),
            reference: tag,
            commit: rg_commit,
            published_at: rg
                .get("published_at")
                .and_then(|v| v.as_str())
                .map(|s| s.into()),
            note: Some("the benchmarked application".into()),
        },
    );

    Ok(Lock {
        schema_version: 1,
        resolved_at: crate::envinfo::now_iso8601(),
        entries,
    })
}
