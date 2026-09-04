//! The Arch Linux builder: turns an Arch request — the rolling release,
//! its `multilib` variant, or a dated snapshot — into a `SourceDistro`.

use std::sync::Arc;

use anyhow::{Result, bail};

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, TableRelease};

const URL_LIVE: &str = "https://geo.mirror.pkgbuild.com";
const URL_ARCHIVE_BASE: &str = "https://archive.archlinux.org/repos";
const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64];

/// Arch Linux's `Distro` implementation.
pub struct ArchLinux;

impl Distro for ArchLinux {
  fn kind(&self) -> KindDistro {
    KindDistro::ArchLinux
  }

  fn prefix(&self) -> &'static str {
    KindDistro::ArchLinux.prefix()
  }

  fn display_name(&self) -> &'static str {
    "Arch Linux"
  }

  fn from_syntax(&self) -> &'static str {
    "arch:<release>[@<date>]"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    true
  }

  /// Builds the `SourceDistro` for one Arch request: the live mirror or
  /// a dated snapshot, with the `multilib` repository added for the
  /// `multilib` release. A malformed date is refused here, not deep in
  /// a fetch.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    let arch_uname = arch.as_uname();
    let cache_id = format!("arch/{}/{}", release, arch_uname);
    let (release_clean, mirror): (String, Box<dyn crate::mirror::Mirror>) =
      if let Some((rel, date)) = release.split_once('@') {
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() != 3 {
          bail!("Invalid date format '{}'. Expected YYYY-MM-DD", date);
        }
        (
          rel.to_string(),
          Box::new(HttpMirror::snapshot(
            format!("{URL_ARCHIVE_BASE}/{}/{}/{}", parts[0], parts[1], parts[2]),
            date.to_string(),
            http.clone(),
          )),
        )
      } else {
        (release.to_string(), Box::new(HttpMirror::live(URL_LIVE, http.clone())))
      };

    let repo_names = if release_clean == "multilib" {
      vec!["core", "extra", "multilib"]
    } else {
      vec!["core", "extra"]
    };

    let mirrors = ListMirror::primary(mirror);

    let repos = repo_names
      .iter()
      .map(|name| Repository {
        label: format!("arch:{name}"),
        mirrors: mirrors.clone(),
        layout: LayoutRepository::Pacman {
          path_file_db: format!("{name}/os/x86_64/{name}.db.tar.gz"),
          path_file_files: format!("{name}/os/x86_64/{name}.files.tar.gz"),
          path_dir_packages: format!("{name}/os/x86_64"),
        },
      })
      .collect();

    Ok(SourceDistro {
      kind: KindDistro::ArchLinux,
      release: release.to_string(),
      native_arch: arch_uname.to_string(),
      cache_id,
      repos,
      base_packages: super::base_packages::from_table(super::base_packages::PACMAN),
    })
  }

  /// Arch's fixed release table — `rolling` and `multilib` — since Arch
  /// publishes no walkable release listing.
  fn release_source(&self, _http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Static(TableRelease {
      header: vec!["Release", "Type", "Mirror"],
      rows: vec![
        vec!["rolling".into(), "rolling".into(), "geo.mirror.pkgbuild.com".into()],
        vec!["multilib".into(), "rolling".into(), "geo.mirror.pkgbuild.com".into()],
      ],
    })
  }
}
