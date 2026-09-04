//! `Remote` is the distribution-neutral interface the build pipeline
//! talks to; `RemoteDistro` is its single implementation, dispatching
//! each request to the resolved source's package format.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use crate::db::{Index, IndexWriter};
use crate::distro::{FormatPackage, SourceDistro};
use crate::internal::http::HttpClient;
use crate::manifest::ChecksumAlgorithm;
use crate::package::PackageIdentity;
use crate::postinstall::PostInstall;
use crate::version::VersionCompare;

use crate::remote::soname::Soname;

/// The distribution-neutral interface every source implements; adding a
/// distribution means implementing this, with no consumer change.
pub trait Remote: Send + Sync {
  /// Fetches the source's package index and file-to-package data into
  /// `writer`, so later reads are local.
  fn index_fetch(&self, writer: &mut IndexWriter) -> Result<()>;

  /// Rebuilds an archive's full URL from its index-recorded filename,
  /// so a fetch targets the mirror the index came from. An unprefixed
  /// filename is an index-population invariant violation and is
  /// refused.
  fn download_url(&self, filename: &str) -> Result<String>;

  /// Extracts one package's files into the rootfs, saves its install
  /// scripts under its identity, and returns the files placed.
  fn extract(&self, archive_path: &Path, root: &Path, pkg: &PackageIdentity) -> Result<Vec<PathBuf>>;

  /// The checksum algorithm the source's digests use.
  fn checksum_type(&self) -> ChecksumAlgorithm;

  /// The base packages the resolver always seeds, so the result is a
  /// working system.
  fn base_packages(&self) -> Vec<&str>;

  /// The cache key the source's cached state lives under; only the same
  /// distribution, release, and architecture share it.
  fn cache_key(&self) -> String;

  /// A fresh comparator for the source's version-ordering rules.
  fn version_compare(&self) -> Box<dyn VersionCompare>;

  /// The post-install behavior for the source's family.
  fn post_install(&self) -> PostInstall;

  /// An owned copy from behind the trait object, for parallel stages.
  fn clone_box(&self) -> Box<dyn Remote>;

  /// The `Soname` resolver that maps a shared-library name to its
  /// providing package for this family, bound to `index`.
  fn soname_resolver<'a>(&self, index: &'a Index) -> Soname<'a>;

  /// The package format the source uses, so a caller can refuse a
  /// request that family cannot answer (apk publishes no file lists)
  /// rather than return silently empty output.
  fn format(&self) -> FormatPackage;
}

/// The single `Remote` implementation: holds a resolved `SourceDistro`
/// and dispatches each request to its package format.
#[derive(Clone)]
pub struct RemoteDistro {
  /// Resolved source whose format and metadata drive every dispatch.
  source: SourceDistro,
  /// Shared HTTP client, so index fetching can pin a fresh mirror
  /// against the same client when a family fetches several files from
  /// one origin.
  http: Arc<HttpClient>,
}

impl RemoteDistro {
  /// Pairs a resolved source with the shared HTTP client.
  pub fn new(source: SourceDistro, http: Arc<HttpClient>) -> Self {
    Self { source, http }
  }

  /// A read-only look at the underlying resolved description.
  pub fn source(&self) -> &SourceDistro {
    &self.source
  }

  /// Boxes a resolved source behind the `Remote` interface.
  pub fn from_source(source: SourceDistro, http: Arc<HttpClient>) -> Box<dyn Remote> {
    Box::new(RemoteDistro::new(source, http))
  }

  /// Builds the `Remote` for a `<distro>:<release>` selector and
  /// architecture.
  ///
  /// ```no_run
  /// use std::sync::Arc;
  /// use std::time::Duration;
  /// use flatroot::internal::http::HttpClient;
  /// use flatroot::remote::RemoteDistro;
  /// # use flatroot::arch::Arch;
  /// let http = Arc::new(HttpClient::new("/tmp/cache".into(), 3, Duration::from_secs(3600)));
  /// let _ = RemoteDistro::from_str("debian:bookworm", Arch::X86_64, http);
  /// ```
  pub fn from_str(remote_str: &str, arch: crate::arch::Arch, http: Arc<HttpClient>) -> Result<Box<dyn Remote>> {
    let selector = crate::distro::FromSelector::parse(remote_str)?;
    let source = crate::distro::DistroRegistry::source(&selector, arch, &http)?;
    Ok(Self::from_source(source, http))
  }
}

impl Remote for RemoteDistro {
  fn index_fetch(&self, writer: &mut IndexWriter) -> Result<()> {
    self
      .source
      .kind
      .format()
      .backend()
      .index_fetch(&self.source, writer, &self.http)
  }

  fn download_url(&self, filename: &str) -> Result<String> {
    // A `PackageFormat`'s `index_fetch` encodes every filename as
    // `<url_base>|<relative_path>`; splitting on `|` and rejoining
    // produces the absolute URL pointing at the same mirror that
    // delivered the index. Any filename without the prefix is an
    // index-population invariant violation reported as an error, not a
    // fallback case.
    let (base, rest) = filename.split_once('|').ok_or_else(|| {
      anyhow::anyhow!(
        "download_url received unprefixed filename '{filename}' — index_fetch must encode every filename as '<url_base>|<relative_path>'"
      )
    })?;
    Ok(format!("{}/{}", base.trim_end_matches('/'), rest.trim_start_matches('/')))
  }

  fn extract(&self, archive_path: &Path, root: &Path, pkg: &PackageIdentity) -> Result<Vec<PathBuf>> {
    self.source.kind.format().backend().extract(archive_path, root, pkg)
  }

  fn checksum_type(&self) -> ChecksumAlgorithm {
    self.source.kind.format().checksum_type()
  }

  fn base_packages(&self) -> Vec<&str> {
    self.source.base_packages.iter().map(|s| s.as_str()).collect()
  }

  fn cache_key(&self) -> String {
    self.source.cache_id.clone()
  }

  fn version_compare(&self) -> Box<dyn VersionCompare> {
    self.source.kind.format().version_compare()
  }

  fn post_install(&self) -> PostInstall {
    self.source.kind.post_install()
  }

  fn clone_box(&self) -> Box<dyn Remote> {
    Box::new(self.clone())
  }

  fn soname_resolver<'a>(&self, index: &'a Index) -> Soname<'a> {
    Soname::for_source(self.source.kind.format(), &self.source, index)
  }

  fn format(&self) -> FormatPackage {
    self.source.kind.format()
  }
}
