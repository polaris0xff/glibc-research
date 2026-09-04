//! The CentOS builder: turns a CentOS release request into a
//! `SourceDistro`, reading the numbered releases from the vault host
//! and the stream release from its own host.

use std::sync::Arc;

use anyhow::{Result, bail};

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, TableRelease};

const URL_VAULT: &str = "https://vault.centos.org";
const URL_STREAM: &str = "https://mirror.stream.centos.org";
const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64, Arch::Aarch64];

/// CentOS's `Distro` implementation.
pub struct Centos;

impl Distro for Centos {
  fn kind(&self) -> KindDistro {
    KindDistro::Centos
  }

  fn prefix(&self) -> &'static str {
    KindDistro::Centos.prefix()
  }

  fn display_name(&self) -> &'static str {
    "CentOS"
  }

  fn from_syntax(&self) -> &'static str {
    "centos:<version>"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    false
  }

  /// Builds the `SourceDistro` for one CentOS release: the numbered
  /// releases frozen on the vault host, `stream9` on the stream host.
  /// An unrecognized release is refused rather than guessed.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    let arch_uname = arch.as_uname();
    let mirrors_vault = ListMirror::primary(Box::new(HttpMirror::live(URL_VAULT, http.clone())));
    let mirrors_stream = ListMirror::primary(Box::new(HttpMirror::live(URL_STREAM, http.clone())));

    let entries: Vec<(&str, ListMirror, String)> = match release {
      "7" => vec![
        ("os", mirrors_vault.clone(), format!("centos/7/os/{arch_uname}")),
        ("updates", mirrors_vault, format!("centos/7/updates/{arch_uname}")),
      ],
      "8" => vec![
        ("BaseOS", mirrors_vault.clone(), format!("centos/8.5.2111/BaseOS/{arch_uname}/os")),
        ("AppStream", mirrors_vault, format!("centos/8.5.2111/AppStream/{arch_uname}/os")),
      ],
      "stream9" => vec![
        ("BaseOS", mirrors_stream.clone(), format!("9-stream/BaseOS/{arch_uname}/os")),
        ("AppStream", mirrors_stream, format!("9-stream/AppStream/{arch_uname}/os")),
      ],
      other => bail!("Unknown CentOS release '{}'. Supported: 7, 8, stream9", other),
    };

    let repos = entries
      .into_iter()
      .map(|(name, mirrors, path)| Repository {
        label: format!("centos:{release}/{name}"),
        mirrors,
        layout: LayoutRepository::Rpm { path_dir_repo: path },
      })
      .collect();

    Ok(SourceDistro {
      kind: KindDistro::Centos,
      release: release.to_string(),
      native_arch: arch_uname.to_string(),
      cache_id: format!("centos/{}/{}", release, arch_uname),
      repos,
      base_packages: super::base_packages::from_table(super::base_packages::RHEL),
    })
  }

  /// CentOS's fixed release table — the numbered releases and
  /// `stream9` — since it publishes no walkable listing.
  fn release_source(&self, _http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Static(TableRelease {
      header: vec!["Codename", "Version", "Suite", "Mirror"],
      rows: vec![
        vec!["7".into(), "7.9.2009".into(), "final".into(), "vault.centos.org".into()],
        vec!["8".into(), "8.5.2111".into(), "final".into(), "vault.centos.org".into()],
        vec![
          "stream9".into(),
          "9-stream".into(),
          "stream".into(),
          "mirror.stream.centos.org".into(),
        ],
      ],
    })
  }
}
