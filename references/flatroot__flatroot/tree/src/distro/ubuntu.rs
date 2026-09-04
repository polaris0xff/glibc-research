//! The Ubuntu builder: turns an Ubuntu release request into a
//! `SourceDistro` covering the release, its updates, and its security
//! repositories across the `main` and `universe` components.

use std::sync::Arc;

use anyhow::Result;

use crate::arch::Arch;
use crate::internal::http::HttpClient;
use crate::mirror::{HttpMirror, ListMirror};

use super::{Distro, KindDistro, LayoutRepository, ReleaseSource, Repository, SourceDistro, WalkStrategy};

const URL_ARCHIVE: &str = "https://archive.ubuntu.com/ubuntu";
const URL_OLD_RELEASES: &str = "https://old-releases.ubuntu.com/ubuntu";
const URL_SECURITY: &str = "https://security.ubuntu.com/ubuntu";
const URL_SNAPSHOT_BASE: &str = "https://snapshot.ubuntu.com/ubuntu";

const ARCHS_SUPPORTED: &[Arch] = &[Arch::X86_64, Arch::Aarch64, Arch::I686, Arch::Armv7l, Arch::Riscv64];
const COMPONENTS: &[&str] = &["main", "universe"];

/// Ubuntu's `Distro` implementation.
pub struct Ubuntu;

impl Distro for Ubuntu {
  fn kind(&self) -> KindDistro {
    KindDistro::Ubuntu
  }

  fn prefix(&self) -> &'static str {
    KindDistro::Ubuntu.prefix()
  }

  fn display_name(&self) -> &'static str {
    "Ubuntu"
  }

  fn from_syntax(&self) -> &'static str {
    "ubuntu:<release>[@<date>]"
  }

  fn archs_supported(&self) -> &'static [Arch] {
    ARCHS_SUPPORTED
  }

  fn snapshot_supported(&self) -> bool {
    true
  }

  /// Builds the `SourceDistro` for one Ubuntu release, spanning the
  /// release, its `-updates`, and its `-security` repositories across
  /// both components. A dated `@<date>` request reads a point-in-time
  /// snapshot; a plain one reads the live archive with the old-releases
  /// archive behind it.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    let arch_deb = arch.as_debian();

    let (suite, mirrors_archive, mirrors_security) = if let Some((suite_name, date)) = release.split_once('@') {
      let stamp = date.replace('-', "");
      let mirrors_pinned = ListMirror::primary(Box::new(HttpMirror::snapshot(
        format!("{URL_SNAPSHOT_BASE}/{stamp}T000000Z"),
        date.to_string(),
        http.clone(),
      )));
      (suite_name.to_string(), mirrors_pinned.clone(), mirrors_pinned)
    } else {
      (
        release.to_string(),
        ListMirror::new(vec![
          Box::new(HttpMirror::live(URL_ARCHIVE, http.clone())),
          Box::new(HttpMirror::live(URL_OLD_RELEASES, http.clone())),
        ]),
        ListMirror::primary(Box::new(HttpMirror::live(URL_SECURITY, http.clone()))),
      )
    };

    let mut repos = Vec::new();
    for variant in [suite.clone(), format!("{suite}-updates")] {
      for (idx_component, component) in COMPONENTS.iter().enumerate() {
        // Only the first repo (base suite, main component) carries the
        // suite-level Contents path; all others contribute none. Ubuntu
        // never splits out a `Contents-all`: its per-arch listing
        // already folds `Architecture: all` packages in.
        let paths_file_contents = if variant == suite && idx_component == 0 {
          vec![format!("dists/{suite}/Contents-{arch_deb}.gz")]
        } else {
          Vec::new()
        };
        repos.push(Repository {
          label: format!("ubuntu:{variant}/{component}"),
          mirrors: mirrors_archive.clone(),
          layout: LayoutRepository::Deb {
            path_dir_suite: format!("dists/{variant}"),
            component: (*component).to_string(),
            arch: arch_deb.to_string(),
            paths_file_contents,
          },
        });
      }
    }
    for component in COMPONENTS {
      repos.push(Repository {
        label: format!("ubuntu:{suite}-security/{component}"),
        mirrors: mirrors_security.clone(),
        layout: LayoutRepository::Deb {
          path_dir_suite: format!("dists/{suite}-security"),
          component: (*component).to_string(),
          arch: arch_deb.to_string(),
          paths_file_contents: Vec::new(),
        },
      });
    }

    Ok(SourceDistro {
      kind: KindDistro::Ubuntu,
      release: release.to_string(),
      native_arch: arch_deb.to_string(),
      cache_id: format!("ubuntu/{}/{}", release, arch_deb),
      repos,
      base_packages: Vec::new(),
    })
  }

  /// Walks Ubuntu's archive (with the old-releases archive behind it)
  /// for its release codenames, published the same way Debian's are.
  fn release_source(&self, http: &Arc<HttpClient>) -> ReleaseSource {
    ReleaseSource::Walk {
      mirrors: ListMirror::new(vec![
        Box::new(HttpMirror::live(URL_ARCHIVE, http.clone())),
        Box::new(HttpMirror::live(URL_OLD_RELEASES, http.clone())),
      ]),
      strategy: WalkStrategy::DebCodename,
    }
  }
}
