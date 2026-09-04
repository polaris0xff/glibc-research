//! The staging area an OCI image is assembled in: `OciImage` is a
//! temporary directory with a `blobs/sha256` store, and `BlobRef` /
//! `LayerRef` are the handles to stored content. Only the finished,
//! packed image ever leaves the staging area.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use flatroot::internal::tar::Tar;

use super::super::source::RootfsSource;

/// A handle to one stored blob: its sha256 digest and size, so later
/// documents can reference it without rereading or rehashing.
pub struct BlobRef {
  pub(super) digest: String,
  pub(super) size: u64,
}

/// The rootfs layer's two digests — the stored compressed blob's, and
/// the `diff_id` of its unpacked tar. The manifest references the
/// first, the config the second.
pub struct LayerRef {
  pub(super) blob: BlobRef,
  pub(super) diff_id: String,
}

/// The temporary directory an image is assembled in, with its
/// `blobs/sha256` store; only the finished archive leaves it.
pub struct OciImage {
  staging: TempDir,
  path_dir_blobs: PathBuf,
}

impl OciImage {
  /// Creates a fresh temporary staging directory with an empty
  /// `blobs/sha256` store; a failure leaves nothing behind outside it.
  pub(super) fn stage() -> Result<Self> {
    let staging = tempfile::tempdir().context("Failed to create staging directory")?;
    let path_dir_blobs = staging.path().join("blobs/sha256");
    fs::create_dir_all(&path_dir_blobs)?;
    Ok(Self {
      staging,
      path_dir_blobs,
    })
  }

  /// Builds the one filesystem layer from the source tree — excluding the
  /// builder's `.flatroot/`, entries name-sorted for reproducibility, and
  /// every entry owned by numeric id 0 (an engine applying the layer in a
  /// single-mapped-id user namespace rejects any other owner). Returns
  /// the layer's blob digest and `diff_id`.
  pub(super) fn layer_append(&self, source: &RootfsSource) -> Result<LayerRef> {
    let mut uncompressed = Vec::new();
    {
      // The root-owned walk builds every header by hand: links are kept as
      // links and entries are plain USTAR, never GNU-sparse (type-S), which
      // docker's containerd parser rejects on load.
      let mut tar_builder = tar::Builder::new(&mut uncompressed);
      Tar::append_dir_sorted_root_owned(&mut tar_builder, source.path(), OsStr::new(".flatroot"))?;
      tar_builder.finish()?;
    }
    let diff_id = hex::encode(Sha256::digest(&uncompressed));

    let mut compressed = Vec::new();
    {
      let mut encoder = GzEncoder::new(&mut compressed, Compression::default());
      encoder.write_all(&uncompressed)?;
      encoder.finish()?;
    }
    let blob = self.blob_write(&compressed)?;

    Ok(LayerRef { blob, diff_id })
  }

  /// Writes `data` into the blob store under its sha256 digest and
  /// returns its `BlobRef`. Every blob enters the store through here.
  fn blob_write(&self, data: &[u8]) -> Result<BlobRef> {
    let digest = hex::encode(Sha256::digest(data));
    fs::write(self.path_dir_blobs.join(&digest), data)?;
    Ok(BlobRef {
      digest,
      size: data.len() as u64,
    })
  }

  /// Serializes `value` to JSON and stores it as a content-addressed
  /// blob, so configs and manifests are addressable like any other blob.
  pub(super) fn blob_write_json<T: Serialize>(&self, value: &T) -> Result<BlobRef> {
    self.blob_write(&serde_json::to_vec(value)?)
  }

  /// Serializes `value` to JSON at a fixed name in the image root — not
  /// the blob store — for entry-point files like `index.json`.
  pub(super) fn file_write_json<T: Serialize>(&self, name: &str, value: &T) -> Result<()> {
    fs::write(self.staging.path().join(name), serde_json::to_vec(value)?)?;
    Ok(())
  }

  /// Packs the staging directory into one uncompressed tar archive at
  /// `output`, consuming the staging area.
  pub(super) fn seal(self, output: &Path) -> Result<()> {
    let out_file = File::create(output).with_context(|| format!("Failed to create {}", output.display()))?;
    let mut archive = tar::Builder::new(out_file);
    // Disable tar-rs's sparse-file detection so the outer tar is plain
    // USTAR and docker's archive parser does not reject blob entries
    // with "unhandled tar header type 83".
    archive.sparse(false);
    archive.append_dir_all(".", self.staging.path())?;
    archive.finish()?;
    Ok(())
  }
}
