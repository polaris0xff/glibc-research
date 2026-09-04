//! The Fedora builder: turns a Fedora release request into a
//! `SourceDistro`; current, retired, and `rawhide` releases each read
//! different download locations.

use std::sync::Arc;

use anyhow::Result;

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, WalkStrategy};

const URL_PRIMARY: &str = "https://dl.fedoraproject.org/pub/fedora/linux";
const URL_ARCHIVE: &str = "https://archives.fedoraproject.org/pub/archive/fedora/linux";
const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64, Arch::Aarch64, Arch::I686];

/// Fedora's `Distro` implementation.
pub struct Fedora;

impl Distro for Fedora {
  fn kind(&self) -> KindDistro {
    KindDistro::Fedora
  }

  fn prefix(&self) -> &'static str {
    KindDistro::Fedora.prefix()
  }

  fn display_name(&self) -> &'static str {
    "Fedora"
  }

  fn from_syntax(&self) -> &'static str {
    "fedora:<release>"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    false
  }

  /// Builds the `SourceDistro` for one Fedora release: the primary
  /// server with the archive server as fallback, so a retired release
  /// stays reachable; `rawhide` reads the development tree.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    let arch_uname = arch.as_uname();
    let mirrors = if release == "rawhide" {
      ListMirror::primary(Box::new(HttpMirror::live(URL_PRIMARY, http.clone())))
    } else {
      ListMirror::new(vec![
        Box::new(HttpMirror::live(URL_PRIMARY, http.clone())),
        Box::new(HttpMirror::live(URL_ARCHIVE, http.clone())),
      ])
    };

    let repos = if release == "rawhide" {
      vec![Repository {
        label: "fedora:rawhide/Everything".to_string(),
        mirrors,
        layout: LayoutRepository::Rpm {
          path_dir_repo: format!("development/rawhide/Everything/{arch_uname}/os"),
        },
      }]
    } else {
      vec![
        Repository {
          label: format!("fedora:{release}/releases"),
          mirrors: mirrors.clone(),
          layout: LayoutRepository::Rpm {
            path_dir_repo: format!("releases/{release}/Everything/{arch_uname}/os"),
          },
        },
        Repository {
          label: format!("fedora:{release}/updates"),
          mirrors,
          layout: LayoutRepository::Rpm {
            path_dir_repo: format!("updates/{release}/Everything/{arch_uname}"),
          },
        },
      ]
    };

    Ok(SourceDistro {
      kind: KindDistro::Fedora,
      release: release.to_string(),
      native_arch: arch_uname.to_string(),
      cache_id: format!("fedora/{}/{}", release, arch_uname),
      repos,
      base_packages: super::base_packages::from_table(super::base_packages::RHEL),
    })
  }

  /// Walks Fedora's live and archive servers for numbered releases and
  /// appends the unnumbered rolling `rawhide` by name, so no
  /// still-installable release drops out.
  fn release_source(&self, http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Walk {
      mirrors: ListMirror::new(vec![
        Box::new(HttpMirror::live(format!("{URL_PRIMARY}/releases"), http.clone())),
        Box::new(HttpMirror::live(format!("{URL_ARCHIVE}/releases"), http.clone())),
      ]),
      strategy: WalkStrategy::RpmNumeric {
        extras: &[("rawhide", "dl.fedoraproject.org")],
      },
    }
  }
}
