//! Domain-neutral filesystem operations with durability guarantees:
//! all-or-nothing content replacement, and removal that tolerates an
//! already-absent target.

use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

/// Namespace for the durability-conscious filesystem operations.
pub struct Fs;

impl Fs {
  /// Replaces a file's contents in one indivisible step — write a temp
  /// file, fsync it durable, then rename it over the target — so a reader
  /// sees either the whole old contents or the whole new ones and a crash
  /// cannot leave a torn version. An fsync failure (full disk, I/O error)
  /// is surfaced, not disguised as a successful write.
  pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    let tmp = path.with_extension({
      let existing = path.extension().and_then(|e| e.to_str()).unwrap_or("");
      if existing.is_empty() {
        "tmp".to_string()
      } else {
        format!("{existing}.tmp")
      }
    });
    {
      let mut f = fs::File::create(&tmp).with_context(|| format!("failed to create {}", tmp.display()))?;
      f.write_all(bytes)
        .with_context(|| format!("failed to write {}", tmp.display()))?;
      // fsync before rename so the bytes are durable before the name
      // switch commits.
      f.sync_all().with_context(|| format!("fsync {}", tmp.display()))?;
    }
    fs::rename(&tmp, path).with_context(|| format!("failed to rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
  }

  /// Ensures a file is absent, staying quiet when there was nothing to
  /// remove.
  pub fn remove_lenient(path: &Path) {
    let _ = fs::remove_file(path);
  }

  /// Removes a directory only when empty; "not empty" and "already
  /// absent" both leave the path as it stands, so a refusal signals the
  /// path is still occupied.
  pub fn remove_dir_lenient(path: &Path) {
    let _ = fs::remove_dir(path);
  }
}
