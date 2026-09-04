//! The Alpine builder: turns an Alpine version request into a
//! `SourceDistro` covering the `main` and `community` repositories.

use std::sync::Arc;

use anyhow::Result;

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, WalkStrategy};

const URL_MIRROR: &str = "https://dl-cdn.alpinelinux.org/alpine";
// Both kernel tokens `x86` and `i686` parse to `Arch::I686` at the CLI
// boundary (`Arch::from_uname`), so the six host strings Alpine accepts
// collapse to five `Arch` entries here — `Arch::I686` covers both, and
// either still projects to Alpine's `x86` via `as_alpine`.
const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64, Arch::Aarch64, Arch::I686, Arch::Armv7l, Arch::Riscv64];
const REPOS: &[&str] = &["main", "community"];

/// Alpine Linux's `Distro` implementation.
pub struct Alpine;

impl Distro for Alpine {
  fn kind(&self) -> KindDistro {
    KindDistro::Alpine
  }

  fn prefix(&self) -> &'static str {
    KindDistro::Alpine.prefix()
  }

  fn display_name(&self) -> &'static str {
    "Alpine Linux"
  }

  fn from_syntax(&self) -> &'static str {
    "alpine:<version>"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    false
  }

  /// Builds the `SourceDistro` for one Alpine version, adding the `v`
  /// prefix a bare version number omits and translating the arch to
  /// Alpine's naming.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    let arch_alpine = arch.as_alpine();

    let version_normalised = if release == "edge" || release == "latest-stable" {
      release.to_string()
    } else if release.starts_with('v') {
      release.to_string()
    } else {
      format!("v{}", release)
    };

    // The version is part of the mirror base URL so repo paths stay
    // simple (just `<repo>/<arch>/...`).
    let url_base = format!("{URL_MIRROR}/{version_normalised}");
    let mirrors = ListMirror::primary(Box::new(HttpMirror::live(url_base, http.clone())));

    let repos = REPOS
      .iter()
      .map(|repo| Repository {
        label: format!("alpine:{version_normalised}/{repo}"),
        mirrors: mirrors.clone(),
        layout: LayoutRepository::Apk {
          path_file_apkindex: format!("{repo}/{arch_alpine}/APKINDEX.tar.gz"),
          path_dir_packages: format!("{repo}/{arch_alpine}"),
        },
      })
      .collect();

    Ok(SourceDistro {
      kind: KindDistro::Alpine,
      release: version_normalised.clone(),
      native_arch: arch_alpine.to_string(),
      cache_id: format!("alpine/{}/{}", version_normalised, arch_alpine),
      repos,
      base_packages: vec![
        "musl".into(),
        "busybox".into(),
        "alpine-baselayout".into(),
        "busybox-binsh".into(),
      ],
    })
  }

  /// Walks Alpine's CDN, which carries both its numbered versions and
  /// the rolling `edge`, so the listing reflects what actually exists.
  fn release_source(&self, http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Walk {
      mirrors: ListMirror::primary(Box::new(HttpMirror::live(URL_MIRROR, http.clone()))),
      strategy: WalkStrategy::ApkVersion,
    }
  }
}
