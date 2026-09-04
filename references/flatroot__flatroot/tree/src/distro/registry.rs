//! The registry of supported distributions. Every answer — matching a
//! `--from` prefix, checking an architecture, listing releases or
//! distributions — comes from this one list, so they can never disagree
//! about what is supported. `Distro` is the trait each distribution
//! implements; `DistroRegistry` is the list of them.

use std::sync::Arc;

use anyhow::{Result, bail};

use crate::arch::Arch;
use crate::internal::http::HttpClient;

use super::listing::ReleaseSource;
use super::{FromSelector, KindDistro, SourceDistro, TableRelease};
use super::{alma, alpine, arch, cachyos, centos, debian, fedora, opensuse, rocky, ubuntu};

/// The trait every supported distribution implements; callers hold a
/// `dyn Distro` and never know which distribution it is.
pub trait Distro: Send + Sync {
  /// The distribution family, which decides the package format and the
  /// post-install behavior.
  fn kind(&self) -> KindDistro;

  /// The `--from` prefix a user types to select this distribution
  /// (e.g. `debian`).
  fn prefix(&self) -> &'static str;

  /// The distribution's proper name, used in error messages
  /// (e.g. `Debian`).
  fn display_name(&self) -> &'static str;

  /// The documented `--from` syntax for this distribution
  /// (e.g. `debian:<release>[@<date>]`).
  fn from_syntax(&self) -> &'static str;

  /// The architectures this distribution can build a rootfs for; a
  /// request outside this set is refused before any fetching.
  fn archs_supported(&self) -> &'static [Arch];

  /// Whether this distribution serves `@<date>` snapshots. A
  /// distribution without a dated archive (like snapshot.debian.org)
  /// cannot resolve the suffix, so a dated request for it is refused
  /// early.
  fn snapshot_supported(&self) -> bool;

  /// Builds the `SourceDistro` for one release and architecture: the
  /// mirrors, repository layout, and base packages the build runs from.
  fn source(&self, release: &str, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro>;

  /// How this distribution's releases are listed: scraped live from
  /// mirrors, or served from built-in knowledge.
  fn release_source(&self, http: &Arc<HttpClient>) -> ReleaseSource;
}

/// The list of supported distributions every lookup goes through.
pub struct DistroRegistry;

impl DistroRegistry {
  /// Every supported distribution, in display order.
  pub fn all() -> &'static [&'static dyn Distro] {
    &[
      &debian::Debian,
      &ubuntu::Ubuntu,
      &arch::ArchLinux,
      &alpine::Alpine,
      &centos::Centos,
      &fedora::Fedora,
      &alma::Alma,
      &rocky::Rocky,
      &opensuse::Opensuse,
      &cachyos::Cachyos,
    ]
  }

  /// The supported prefixes as a comma-joined list for error messages,
  /// built from `all()` so it never drifts from what is accepted.
  fn supported_names() -> String {
    Self::all()
      .iter()
      .map(|distro| distro.prefix())
      .collect::<Vec<_>>()
      .join(", ")
  }

  /// The distribution named by `prefix`; an unknown prefix is an error
  /// listing every supported one.
  pub fn by_prefix(prefix: &str) -> Result<&'static dyn Distro> {
    match DistroRegistry::all().iter().find(|d| d.prefix() == prefix) {
      Some(distro) => Ok(*distro),
      None => bail!("'{}' is not a supported distro. Supported: {}", prefix, Self::supported_names()),
    }
  }

  /// Builds the `SourceDistro` for a request — the one place that
  /// happens. Checks that the distribution is supported, the
  /// architecture is buildable, and an `@<date>` pin is allowed, each
  /// refused here rather than mid-fetch.
  pub fn source(selector: &FromSelector, arch: Arch, http: &Arc<HttpClient>) -> Result<SourceDistro> {
    let distro = match DistroRegistry::all().iter().find(|d| d.prefix() == selector.prefix()) {
      Some(distro) => *distro,
      None => {
        bail!("Unknown remote '{}:{}'. Supported: {}", selector.prefix(), selector.release(), Self::supported_names())
      }
    };
    if !distro.archs_supported().contains(&arch) {
      let supported: Vec<&str> = distro.archs_supported().iter().map(|a| a.as_uname()).collect();
      bail!(
        "{} does not support architecture '{}'. Supported: {}",
        distro.display_name(),
        arch.as_uname(),
        supported.join(", ")
      );
    }
    if selector.release().contains('@') && !distro.snapshot_supported() {
      bail!("{} does not support @<date> snapshots. Expected: {}", distro.display_name(), distro.from_syntax());
    }
    distro.source(selector.release(), arch, http)
  }

  /// The releases the distribution named by `prefix` offers, as one
  /// `TableRelease`; an unknown prefix is an error listing the
  /// supported ones.
  pub fn releases(prefix: &str, http: &Arc<HttpClient>) -> Result<TableRelease> {
    DistroRegistry::by_prefix(prefix)?.release_source(http).collect()
  }
}
