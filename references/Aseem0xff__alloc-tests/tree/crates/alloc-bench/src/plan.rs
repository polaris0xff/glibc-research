//! Expand the matrix into cells, and decide which of them can exist.

use crate::model::*;
use std::collections::BTreeMap;

pub fn libc_for(distro: &str) -> &'static str {
    match distro {
        "alpine" => "musl",
        _ => "glibc",
    }
}

/// Arch publishes no aarch64 image. Rather than silently building something
/// else and calling it `archlinux`, the distribution is RENAMED for that
/// architecture, so no table can merge two distributions under one label.
pub fn effective_distro(distro: &str, arch: &str) -> String {
    if distro == "archlinux" && arch != "x86_64" {
        "archlinuxarm".to_string()
    } else {
        distro.to_string()
    }
}

pub struct Planner<'a> {
    pub manifest: &'a Manifest,
    pub matrix: &'a MatrixFile,
}

impl<'a> Planner<'a> {
    fn spec(&self, id: &str) -> Option<&AllocatorSpec> {
        self.manifest.allocators.iter().find(|a| a.id == id)
    }

    fn expand_allocators(&self, list: &[String]) -> Vec<String> {
        if list.iter().any(|s| s == "*") {
            self.manifest
                .allocators
                .iter()
                .map(|a| a.id.clone())
                .collect()
        } else {
            list.to_vec()
        }
    }

    /// Expand the named suites. `suites` may contain `all`.
    ///
    /// De-duplication is by cell id: two suites asking for the same
    /// configuration must produce ONE experiment, or the same binary would be
    /// measured twice and appear as two rows.
    pub fn plan(&self, suites: &[String]) -> Result<Vec<Cell>, String> {
        let wanted: Vec<&Suite> = if suites.iter().any(|s| s == "all") {
            self.matrix.suites.iter().collect()
        } else {
            let mut v = Vec::new();
            for want in suites {
                let s = self
                    .matrix
                    .suites
                    .iter()
                    .find(|s| &s.id == want)
                    .ok_or_else(|| {
                        format!(
                            "unknown suite {:?}; known: {}",
                            want,
                            self.matrix
                                .suites
                                .iter()
                                .map(|s| s.id.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })?;
                v.push(s);
            }
            v
        };

        let mut seen: BTreeMap<String, Cell> = BTreeMap::new();
        for suite in wanted {
            for distro_raw in &suite.distros {
                for arch in &suite.arches {
                    let distro = effective_distro(distro_raw, arch);
                    let libc = libc_for(&distro).to_string();
                    for alloc_id in self.expand_allocators(&suite.allocators) {
                        let Some(spec) = self.spec(&alloc_id) else {
                            return Err(format!(
                                "suite {:?} names allocator {:?}, which is not in allocators.toml",
                                suite.id, alloc_id
                            ));
                        };
                        for integration_raw in &suite.integrations {
                            // The control has nothing to integrate; whatever a
                            // suite asks for, `system` is always `baseline`.
                            let integration = if spec.is_baseline() {
                                "baseline".to_string()
                            } else {
                                integration_raw.clone()
                            };
                            for profile in &suite.profiles {
                                for toolchain in &suite.toolchains {
                                    let id = Cell::slug(
                                        &distro,
                                        arch,
                                        &alloc_id,
                                        &integration,
                                        profile,
                                        toolchain,
                                    );
                                    if seen.contains_key(&id) {
                                        continue;
                                    }
                                    let (status, reason) =
                                        self.judge(spec, &integration, profile, &libc, &distro);
                                    seen.insert(
                                        id.clone(),
                                        Cell {
                                            id,
                                            suite: suite.id.clone(),
                                            distro: distro.clone(),
                                            arch: arch.clone(),
                                            libc: libc.clone(),
                                            allocator: alloc_id.clone(),
                                            integration: integration.clone(),
                                            profile: profile.clone(),
                                            toolchain: toolchain.clone(),
                                            corpus: suite.corpus.clone(),
                                            repeat: suite.repeat,
                                            status,
                                            reason,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(seen.into_values().collect())
    }

    /// Can this cell exist? A `no` carries the concrete technical reason,
    /// which is the deliverable for that row.
    fn judge(
        &self,
        spec: &AllocatorSpec,
        integration: &str,
        profile: &str,
        libc: &str,
        distro: &str,
    ) -> (String, Option<String>) {
        let no = |r: String| ("unsupported".to_string(), Some(r));

        if spec.is_baseline() {
            // The control exists everywhere except where the profile itself
            // cannot exist.
            if profile == "dynamic" && libc == "musl" && integration == "preload" {
                // Still fine: Alpine can build dynamic musl binaries.
            }
            return ("planned".to_string(), None);
        }

        if !spec.supports(integration) {
            return no(spec.why_not(integration));
        }

        // A static binary has no dynamic loader, so there is nothing to
        // interpose. This is a property of the profile, not of the allocator.
        if integration == "preload" && profile != "dynamic" {
            return no(format!(
                "LD_PRELOAD needs a dynamic loader to interpose, and the `{}` profile produces a binary with no PT_INTERP. \
                 Preload is measured on the `dynamic` profile only.",
                profile
            ));
        }

        // Replacing malloc inside a *static* glibc is the case that does not
        // work, and it is worth stating precisely rather than discovering it
        // as a link error every run.
        if libc == "glibc"
            && matches!(integration, "libc-surgery" | "link-override")
            && profile.starts_with("static")
        {
            return no(format!(
                "glibc's malloc object in libc.a also defines symbols the rest of glibc references internally \
                 (__libc_malloc and the arena hooks), so removing it breaks the archive and leaving it in place \
                 gives two definitions of malloc. Statically replacing glibc's allocator is therefore not \
                 supported on {}; the same allocator IS measured there through `rust-global`, and through \
                 `preload` on the dynamic profile.",
                distro
            ));
        }

        ("planned".to_string(), None)
    }
}
