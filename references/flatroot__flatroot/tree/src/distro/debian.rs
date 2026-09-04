//! The Debian builder: turns a Debian release request into a
//! `SourceDistro` — the live archive with the long-term archive behind
//! it, or a frozen snapshot mirror for a dated request. `base_packages`
//! stays empty: Debian's `Essential:` flag marks the base set instead.

use std::sync::Arc;

use anyhow::Result;

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, WalkStrategy};

const URL_PRIMARY: &str = "https://deb.debian.org/debian";
const URL_ARCHIVE: &str = "https://archive.debian.org/debian";
const URL_SNAPSHOT_BASE: &str = "https://snapshot.debian.org/archive/debian";

const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64, Arch::Aarch64, Arch::I686, Arch::Armv7l, Arch::Riscv64];

/// Debian's `Distro` implementation.
pub struct Debian;

impl Distro for Debian {
  fn kind(&self) -> KindDistro {
    KindDistro::Debian
  }

  fn prefix(&self) -> &'static str {
    KindDistro::Debian.prefix()
  }

  fn display_name(&self) -> &'static str {
    "Debian"
  }

  fn from_syntax(&self) -> &'static str {
    "debian:<release>[@<date>]"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    true
  }

  /// Builds the `SourceDistro` for one Debian release: a plain release
  /// reads the live archive with the long-term archive behind it; a
  /// dated `@<date>` request reads a frozen snapshot mirror.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    let arch_deb = arch.as_debian();

    let (suite, mirrors) = if let Some((suite_name, date)) = release.split_once('@') {
      let stamp = date.replace('-', "");
      (
        suite_name.to_string(),
        ListMirror::primary(Box::new(HttpMirror::snapshot(
          format!("{URL_SNAPSHOT_BASE}/{stamp}T000000Z"),
          date.to_string(),
          http.clone(),
        ))),
      )
    } else {
      (
        release.to_string(),
        ListMirror::new(vec![
          Box::new(HttpMirror::live(URL_PRIMARY, http.clone())),
          Box::new(HttpMirror::live(URL_ARCHIVE, http.clone())),
        ]),
      )
    };

    Ok(SourceDistro {
      kind: KindDistro::Debian,
      release: release.to_string(),
      native_arch: arch_deb.to_string(),
      cache_id: format!("debian/{}/{}", release, arch_deb),
      repos: vec![Repository {
        label: format!("debian:{suite}/main"),
        mirrors,
        layout: LayoutRepository::Deb {
          path_dir_suite: format!("dists/{suite}"),
          component: "main".to_string(),
          arch: arch_deb.to_string(),
          // Live and snapshot mirrors split file ownership: the per-arch
          // listing carries architecture-specific packages only, and
          // `Contents-all` carries every `Architecture: all` package
          // (fonts, icons, zoneinfo, pure-Python libraries). The
          // long-term archive merges the two into the per-arch file on
          // import and drops `Contents-all`, so its absence there is
          // expected and tolerated at fetch time.
          paths_file_contents: vec![
            format!("dists/{suite}/main/Contents-{arch_deb}.gz"),
            format!("dists/{suite}/main/Contents-all.gz"),
          ],
        },
      }],
      base_packages: Vec::new(),
    })
  }

  /// Both Debian archives in preference order — live first, the long-term
  /// home of aged-out releases behind it — so the discovery pass sees the
  /// full set, not a recent slice.
  fn release_source(&self, http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Walk {
      mirrors: ListMirror::new(vec![
        Box::new(HttpMirror::live(URL_PRIMARY, http.clone())),
        Box::new(HttpMirror::live(URL_ARCHIVE, http.clone())),
      ]),
      strategy: WalkStrategy::DebCodename,
    }
  }
}
