//! `Cache`: the on-disk cache directory and the conventional
//! subdirectories beneath it, so downloads survive across separate
//! runs.

use std::path::{Path, PathBuf};

use anyhow::Result;

/// The resolved cache root, from which the shared index directory and
/// the per-source archive directories are named.
pub struct Cache {
  root: PathBuf,
}

impl Cache {
  /// Resolves the cache root and creates it if missing. Highest
  /// priority first:
  /// - `FLATROOT_CACHE_HOME`
  /// - `$XDG_CACHE_HOME/flatroot`
  /// - `~/.cache/flatroot`
  pub fn dir_resolve() -> Result<Self> {
    let root = if let Ok(explicit) = std::env::var("FLATROOT_CACHE_HOME") {
      PathBuf::from(explicit)
    } else {
      let base = std::env::var("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".cache")
      });
      base.join("flatroot")
    };
    std::fs::create_dir_all(&root)?;
    Ok(Self { root })
  }

  /// The `index` subdirectory, for the package index and the raw index
  /// files it is built from, kept apart from downloaded archives.
  pub fn dir_index(&self) -> PathBuf {
    self.root.join("index")
  }

  /// The per-source subdirectory keyed by `cache_key`, so archives from
  /// different distributions and releases never collide.
  pub fn dir_source(&self, cache_key: &str) -> PathBuf {
    self.root.join(cache_key)
  }

  /// The cache root itself, for referring to the cache as a whole.
  pub fn root(&self) -> &Path {
    &self.root
  }
}
