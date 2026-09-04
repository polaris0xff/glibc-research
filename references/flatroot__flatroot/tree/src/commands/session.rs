//! `Session`: the cache home and the shared HTTP client, built once
//! when a command starts and used by every index open and download.

use std::sync::Arc;

use anyhow::Result;

use flatroot::arch::Arch;
use flatroot::internal::cache::Cache;
use flatroot::internal::http::HttpClient;

use crate::commands::arch_context::ArchContext;

/// The cache home and the shared HTTP client one command run uses for
/// all of its network work.
pub struct Session {
  cache: Cache,
  http: Arc<HttpClient>,
}

impl Session {
  /// Resolves the cache home and builds the HTTP client from it.
  /// Returned in an `Arc` because the executor runs commands on
  /// blocking threads that share the session.
  pub fn open(http_retries: u32) -> Result<Arc<Session>> {
    let cache = Cache::dir_resolve()?;
    let http = Arc::new(HttpClient::new(cache.dir_index(), http_retries, HttpClient::INDEX_CACHE_TTL));
    Ok(Arc::new(Session { cache, http }))
  }

  /// The shared HTTP client, for network work that needs no package
  /// index (release listing, post-install backend).
  pub fn http(&self) -> &Arc<HttpClient> {
    &self.http
  }

  /// Opens an `ArchContext` for this session from async code; the slow
  /// index build runs on a blocking thread.
  pub async fn context_open(&self, remote_str: &str, arch: Arch) -> Result<ArchContext> {
    ArchContext::open(remote_str, arch, &self.cache, &self.http).await
  }

  /// Opens an `ArchContext` for this session from a caller already on a
  /// blocking thread.
  pub fn context_open_blocking(&self, remote_str: &str, arch: Arch) -> Result<ArchContext> {
    ArchContext::open_blocking(remote_str, arch, &self.cache, &self.http)
  }
}
