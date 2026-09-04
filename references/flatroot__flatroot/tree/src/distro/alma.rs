//! The AlmaLinux builder: turns an AlmaLinux release request into a
//! `SourceDistro` covering the BaseOS and AppStream repositories.

use std::sync::Arc;

use anyhow::{Result, bail};

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, WalkStrategy};

const URL_MIRROR: &str = "https://repo.almalinux.org/almalinux";
const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64, Arch::Aarch64];

/// AlmaLinux's `Distro` implementation.
pub struct Alma;

impl Distro for Alma {
  fn kind(&self) -> KindDistro {
    KindDistro::Alma
  }

  fn prefix(&self) -> &'static str {
    KindDistro::Alma.prefix()
  }

  fn display_name(&self) -> &'static str {
    "AlmaLinux"
  }

  fn from_syntax(&self) -> &'static str {
    "alma:<version>"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    false
  }

  /// Builds the `SourceDistro` for one AlmaLinux release, refusing an
  /// unrecognized release up front rather than building mirror URLs
  /// that point at nothing.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    match release {
      "8" | "9" => {}
      other => bail!("Unknown AlmaLinux release '{}'. Supported: 8, 9", other),
    }

    let arch_uname = arch.as_uname();
    let mirrors = ListMirror::primary(Box::new(HttpMirror::live(URL_MIRROR, http.clone())));

    let repos = vec![
      Repository {
        label: format!("alma:{release}/BaseOS"),
        mirrors: mirrors.clone(),
        layout: LayoutRepository::Rpm {
          path_dir_repo: format!("{release}/BaseOS/{arch_uname}/os"),
        },
      },
      Repository {
        label: format!("alma:{release}/AppStream"),
        mirrors,
        layout: LayoutRepository::Rpm {
          path_dir_repo: format!("{release}/AppStream/{arch_uname}/os"),
        },
      },
    ];

    Ok(SourceDistro {
      kind: KindDistro::Alma,
      release: release.to_string(),
      native_arch: arch_uname.to_string(),
      cache_id: format!("alma/{}/{}", release, arch_uname),
      repos,
      base_packages: super::base_packages::from_table(super::base_packages::RHEL),
    })
  }

  /// Walks AlmaLinux's mirror for its major-version releases.
  fn release_source(&self, http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Walk {
      mirrors: ListMirror::primary(Box::new(HttpMirror::live(URL_MIRROR, http.clone()))),
      strategy: WalkStrategy::RhelMajor {
        mirror_short: "repo.almalinux.org",
      },
    }
  }
}
