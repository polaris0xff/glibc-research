//! `DestPartial`: every export writes into a `.partial` sibling, and
//! the destination name appears only through one atomic rename after
//! the artifact is complete — a failed or killed export never leaves a
//! file that looks finished.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use flatroot::internal::fs::Fs;

/// The destination of one export in progress: the `.partial` sibling
/// written into and the final name that appears at commit. Dropping it
/// uncommitted removes the sibling.
pub struct DestPartial {
  /// The `.partial` sibling every byte of the artifact is written to.
  path_file_partial: PathBuf,
  /// The requested destination, created only by the commit rename.
  path_file_final: PathBuf,
  /// Set by `commit` so the drop cleanup knows the partial file has
  /// already become the finished artifact.
  committed: bool,
}

impl DestPartial {
  /// Opens the working destination: the `.partial` sibling, clearing any
  /// stale one a killed run left, so the new run starts clean.
  pub fn begin(output: &Path) -> Result<DestPartial> {
    let mut name = output.as_os_str().to_owned();
    name.push(".partial");
    let path_file_partial = PathBuf::from(name);
    if path_file_partial.symlink_metadata().is_ok() {
      std::fs::remove_file(&path_file_partial)
        .with_context(|| format!("Failed to remove stale {}", path_file_partial.display()))?;
    }
    Ok(DestPartial {
      path_file_partial,
      path_file_final: output.to_path_buf(),
      committed: false,
    })
  }

  /// Where the export writes: the `.partial` sibling, never the final
  /// name.
  pub fn path(&self) -> &Path {
    &self.path_file_partial
  }

  /// Renames the `.partial` file to the final name in one atomic
  /// rename — the only operation that ever creates the destination.
  pub fn commit(mut self) -> Result<()> {
    std::fs::rename(&self.path_file_partial, &self.path_file_final).with_context(|| {
      format!(
        "Failed to move finished export {} to {}",
        self.path_file_partial.display(),
        self.path_file_final.display()
      )
    })?;
    self.committed = true;
    Ok(())
  }
}

impl Drop for DestPartial {
  fn drop(&mut self) {
    if !self.committed {
      Fs::remove_lenient(&self.path_file_partial);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn commit_renames_partial_to_final() {
    let tmp = tempfile::tempdir().unwrap();
    let path_file_out = tmp.path().join("out.tar.gz");
    let dest = DestPartial::begin(&path_file_out).unwrap();
    std::fs::write(dest.path(), b"artefact").unwrap();
    dest.commit().unwrap();
    assert_eq!(std::fs::read(&path_file_out).unwrap(), b"artefact");
    assert!(!path_file_out.with_extension("gz.partial").exists());
  }

  #[test]
  fn drop_without_commit_removes_partial_and_never_creates_final() {
    let tmp = tempfile::tempdir().unwrap();
    let path_file_out = tmp.path().join("out.tar.gz");
    {
      let dest = DestPartial::begin(&path_file_out).unwrap();
      std::fs::write(dest.path(), b"half").unwrap();
    }
    assert!(!path_file_out.exists());
    assert!(std::fs::read_dir(tmp.path()).unwrap().next().is_none());
  }

  #[test]
  fn begin_clears_stale_partial_from_a_killed_run() {
    let tmp = tempfile::tempdir().unwrap();
    let path_file_out = tmp.path().join("out.sqfs");
    let mut name = path_file_out.as_os_str().to_owned();
    name.push(".partial");
    let path_file_stale = PathBuf::from(name);
    std::fs::write(&path_file_stale, b"stranded").unwrap();
    let dest = DestPartial::begin(&path_file_out).unwrap();
    assert!(!dest.path().exists());
  }
}
