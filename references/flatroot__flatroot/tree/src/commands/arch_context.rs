//! Builds `ArchContext`, the per-architecture working set every command
//! runs against.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;

use flatroot::arch::Arch;
use flatroot::db::Index;
use flatroot::internal::cache::Cache;
use flatroot::internal::http::HttpClient;
use flatroot::remote::{Remote, RemoteDistro};

/// Everything a command needs to work against one distribution release
/// and architecture.
pub struct ArchContext {
  /// Architecture this context targets.
  pub arch: Arch,
  /// The populated package index for this architecture; the path
  /// index's location derives from it.
  pub index: Index,
  /// The backend for this distro-and-architecture pair.
  pub remote: Box<dyn Remote>,
  /// Directory downloaded archives are cached in, keyed by the
  /// backend's cache key.
  pub cache_dir: PathBuf,
}

impl ArchContext {
  /// Opens the context from async code, running the blocking index
  /// fetch on a `tokio` blocking thread.
  pub async fn open(remote_str: &str, arch: Arch, cache: &Cache, http: &Arc<HttpClient>) -> Result<Self> {
    eprintln!("Fetching package index for {} ({})...", remote_str, arch.as_uname());

    let remote_str_owned = remote_str.to_string();
    let http_owned = http.clone();
    // The spawned closure builds the backend and the index and returns
    // both, so the backend is not built twice.
    let (index, remote) = tokio::task::spawn_blocking(move || -> Result<(Index, Box<dyn Remote>)> {
      let remote = RemoteDistro::from_str(&remote_str_owned, arch, http_owned)?;
      let vcmp = Arc::from(remote.version_compare());
      let index = Index::open_or_populate(&remote.cache_key(), vcmp, |writer| remote.index_fetch(writer))?;
      Ok((index, remote))
    })
    .await??;

    Self::finish(arch, index, remote, cache)
  }

  /// Opens the context from blocking code, doing the index fetch on the
  /// calling thread.
  pub fn open_blocking(remote_str: &str, arch: Arch, cache: &Cache, http: &Arc<HttpClient>) -> Result<Self> {
    eprintln!("Fetching package index for {} ({})...", remote_str, arch.as_uname());

    let remote = RemoteDistro::from_str(remote_str, arch, http.clone())?;
    let vcmp = Arc::from(remote.version_compare());
    let index = Index::open_or_populate(&remote.cache_key(), vcmp, |writer| remote.index_fetch(writer))?;

    Self::finish(arch, index, remote, cache)
  }

  /// Shared tail of `open` and `open_blocking`: prints the loaded
  /// package count and creates the cache directory.
  fn finish(arch: Arch, index: Index, remote: Box<dyn Remote>, cache: &Cache) -> Result<Self> {
    eprintln!("Loaded {} packages", index.packages().count()?);

    let cache_dir = cache.dir_source(&remote.cache_key());
    std::fs::create_dir_all(&cache_dir)?;

    Ok(Self {
      arch,
      index,
      remote,
      cache_dir,
    })
  }
}
