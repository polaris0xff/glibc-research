//! The openSUSE builder: turns an openSUSE request into a
//! `SourceDistro`. Tumbleweed and Leap live under different paths, and
//! non-x86_64 architectures under a separate `ports/` area.

use std::sync::Arc;

use anyhow::Result;

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, TableRelease};

const URL_MIRROR: &str = "https://download.opensuse.org";
const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64, Arch::Aarch64];

/// openSUSE's `Distro` implementation.
pub struct Opensuse;

impl Distro for Opensuse {
  fn kind(&self) -> KindDistro {
    KindDistro::Opensuse
  }

  fn prefix(&self) -> &'static str {
    KindDistro::Opensuse.prefix()
  }

  fn display_name(&self) -> &'static str {
    "openSUSE"
  }

  fn from_syntax(&self) -> &'static str {
    "opensuse:<release>"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    false
  }

  /// Builds the `SourceDistro` for one openSUSE release: `tumbleweed`
  /// and Leap releases sit under different paths, and a non-x86_64
  /// architecture under the `ports/` area.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    let arch_uname = arch.as_uname();
    let path_dir_repo = if release == "tumbleweed" {
      if arch == Arch::X86_64 {
        "tumbleweed/repo/oss".to_string()
      } else {
        format!("ports/{arch_uname}/tumbleweed/repo/oss")
      }
    } else if arch == Arch::X86_64 {
      format!("distribution/leap/{release}/repo/oss")
    } else {
      format!("ports/{arch_uname}/distribution/leap/{release}/repo/oss")
    };

    let mirrors = ListMirror::primary(Box::new(HttpMirror::live(URL_MIRROR, http.clone())));

    let repos = vec![Repository {
      label: format!("opensuse:{release}/oss"),
      mirrors,
      layout: LayoutRepository::Rpm { path_dir_repo },
    }];

    Ok(SourceDistro {
      kind: KindDistro::Opensuse,
      release: release.to_string(),
      native_arch: arch_uname.to_string(),
      cache_id: format!("opensuse/{}/{}", release, arch_uname),
      repos,
      base_packages: vec!["filesystem".into(), "bash".into(), "coreutils".into(), "glibc".into()],
    })
  }

  /// openSUSE's fixed release table — `tumbleweed` and the current Leap
  /// releases — since it publishes no walkable listing.
  fn release_source(&self, _http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Static(TableRelease {
      header: vec!["Release", "Type", "Mirror"],
      rows: vec![
        vec!["tumbleweed".into(), "rolling".into(), "download.opensuse.org".into()],
        vec!["15.5".into(), "stable".into(), "download.opensuse.org".into()],
        vec!["15.6".into(), "stable".into(), "download.opensuse.org".into()],
      ],
    })
  }
}
