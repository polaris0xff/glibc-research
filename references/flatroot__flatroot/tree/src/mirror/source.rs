//! `Mirror`, the trait every upstream package source implements; the
//! concrete implementations live in the sibling `http` and `list`
//! modules.

use anyhow::Result;

/// One upstream package source, whatever actually serves the bytes.
/// `Send + Sync` because a build fetches in parallel.
pub trait Mirror: Send + Sync {
  /// Fetches the contents at `rel_path` under the mirror's base URL.
  fn fetch(&self, rel_path: &str) -> Result<Vec<u8>>;

  /// Confirms `rel_path` is actually retrievable before the build
  /// depends on it.
  fn probe(&self, rel_path: &str) -> Result<bool>;

  /// Whether `rel_path` exists, without downloading its content.
  fn exists(&self, rel_path: &str) -> Result<bool>;

  /// Fetches the HTML directory listing at `rel_path`, so the build can
  /// discover what a directory holds rather than assume fixed names.
  fn fetch_listing(&self, rel_path: &str) -> Result<String>;

  /// The mirror's base URL, for diagnostics and for pinning a related
  /// sequence of fetches to one origin.
  fn url_base(&self) -> &str;

  /// An owned copy, since a build hands the mirror across stages and
  /// threads behind this trait object.
  fn clone_box(&self) -> Box<dyn Mirror>;
}
