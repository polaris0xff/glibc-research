//! The CachyOS builder: turns a CachyOS request into a `SourceDistro`.
//! CachyOS builds on Arch, so the repositories combine the upstream
//! Arch mirrors with CachyOS's own optimized ones, in precedence order.

use std::sync::Arc;

use anyhow::{Result, bail};

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, TableRelease};

const URL_CACHYOS: &str = "https://mirror.cachyos.org/repo";
const URL_ARCH: &str = "https://geo.mirror.pkgbuild.com";
const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64];

const REPOS_ARCH: &[&str] = &["core", "extra"];
const REPOS_V3: &[&str] = &["cachyos-core-v3", "cachyos-extra-v3", "cachyos-v3"];

/// CachyOS's `Distro` implementation.
pub struct Cachyos;

impl Distro for Cachyos {
  fn kind(&self) -> KindDistro {
    KindDistro::Cachyos
  }

  fn prefix(&self) -> &'static str {
    KindDistro::Cachyos.prefix()
  }

  fn display_name(&self) -> &'static str {
    "CachyOS"
  }

  fn from_syntax(&self) -> &'static str {
    "cachyos:rolling"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    false
  }

  /// Builds the CachyOS `SourceDistro`: the upstream Arch repositories
  /// combined with CachyOS's optimized ones, ordered so the optimized
  /// packages take precedence. Only `rolling` exists; any other release
  /// is refused.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    if release != "rolling" {
      bail!("CachyOS only supports 'rolling' release. Use: cachyos:rolling");
    }

    let mirrors_arch = ListMirror::primary(Box::new(HttpMirror::live(URL_ARCH, http.clone())));
    let mirrors_cachyos = ListMirror::primary(Box::new(HttpMirror::live(URL_CACHYOS, http.clone())));

    let mut repos: Vec<Repository> = Vec::with_capacity(REPOS_ARCH.len() + REPOS_V3.len());
    for name in REPOS_ARCH {
      repos.push(Repository {
        label: format!("cachyos:arch/{name}"),
        mirrors: mirrors_arch.clone(),
        layout: LayoutRepository::Pacman {
          path_file_db: format!("{name}/os/x86_64/{name}.db.tar.gz"),
          path_file_files: format!("{name}/os/x86_64/{name}.files.tar.gz"),
          path_dir_packages: format!("{name}/os/x86_64"),
        },
      });
    }
    for name in REPOS_V3 {
      repos.push(Repository {
        label: format!("cachyos:{name}"),
        mirrors: mirrors_cachyos.clone(),
        layout: LayoutRepository::Pacman {
          path_file_db: format!("x86_64_v3/{name}/{name}.db"),
          path_file_files: format!("x86_64_v3/{name}/{name}.files"),
          path_dir_packages: format!("x86_64_v3/{name}"),
        },
      });
    }

    Ok(SourceDistro {
      kind: KindDistro::Cachyos,
      release: release.to_string(),
      native_arch: arch.as_uname().to_string(),
      cache_id: format!("cachyos/{}/{}", release, arch.as_uname()),
      repos,
      base_packages: super::base_packages::from_table(super::base_packages::PACMAN),
    })
  }

  /// CachyOS's fixed release table — only `rolling` — since it
  /// publishes no walkable listing.
  fn release_source(&self, _http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Static(TableRelease {
      header: vec!["Release", "Type", "Mirror"],
      rows: vec![vec!["rolling".into(), "rolling".into(), "mirror.cachyos.org".into()]],
    })
  }
}
