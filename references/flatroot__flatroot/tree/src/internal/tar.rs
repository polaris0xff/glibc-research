//! Domain-neutral archive mechanism: unpack a stream into a directory
//! tree, and pack a directory tree back into a stream.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::internal::codec::Codec;
use crate::internal::fs::Fs;

/// Namespace for the archive extract and pack operations.
pub struct Tar;

impl Tar {
  /// Unpacks an archive into `root`, returning every path placed. Built
  /// for many packages sharing one tree: later contents overwrite
  /// earlier, an obstructing destination is cleared first, and a
  /// malformed entry is skipped with a warning rather than aborting.
  pub fn extract<R: Read>(reader: R, root: &Path) -> Result<Vec<PathBuf>> {
    let mut archive = tar::Archive::new(reader);
    archive.set_preserve_permissions(false);
    archive.set_preserve_ownerships(false);
    archive.set_overwrite(true);

    let mut captured: Vec<PathBuf> = Vec::new();
    for entry in archive.entries()? {
      let mut entry = match entry {
        Ok(e) => e,
        Err(e) => {
          eprintln!("    warning: skipping tar entry: {}", e);
          continue;
        }
      };
      let path = match entry.path() {
        Ok(p) => p.into_owned(),
        Err(e) => {
          eprintln!("    warning: skipping entry with bad path: {}", e);
          continue;
        }
      };
      let dest = root.join(&path);

      // An obstructing plain file or dangling link is cleared so later
      // contents win over earlier ones; an existing directory stays.
      if (dest.exists() || dest.symlink_metadata().is_ok()) && !dest.is_dir() {
        Fs::remove_lenient(&dest);
      }

      match entry.unpack_in(root) {
        Ok(_) => captured.push(path),
        Err(e) => {
          eprintln!("    warning: failed to unpack {}: {}", path.display(), e);
        }
      }
    }

    Ok(captured)
  }

  /// Infers the member's compression from its name, decompresses, and
  /// unpacks the recovered tree into `dest`, returning the paths placed.
  pub fn extract_compressed(name: &str, data: &[u8], dest: &Path) -> Result<Vec<PathBuf>> {
    let reader = Codec::from_suffix(name).reader(data)?;
    Tar::extract(reader, dest)
  }

  /// Packs `dir` into the archive reproducibly — entries in a fixed
  /// name-sorted order at every level, links kept as links — omitting
  /// the top-level entry named by `exclude` so the tool's private
  /// metadata never leaks into the image.
  pub fn append_dir_sorted<W: Write>(archive: &mut tar::Builder<W>, dir: &Path, exclude: &OsStr) -> Result<()> {
    for (name, path) in Self::entries_sorted(dir)? {
      if name == exclude {
        continue;
      }
      Self::entry_append_sorted(archive, &path, Path::new(&name))?;
    }

    Ok(())
  }

  /// Appends one entry: a link or file as-is, a directory as its own
  /// entry then descended in sorted order.
  fn entry_append_sorted<W: Write>(archive: &mut tar::Builder<W>, path: &Path, name: &Path) -> Result<()> {
    let file_type = fs::symlink_metadata(path)
      .with_context(|| format!("failed to stat {}", path.display()))?
      .file_type();
    if file_type.is_symlink() {
      archive
        .append_path_with_name(path, name)
        .with_context(|| format!("failed to append symlink {}", path.display()))?;
      return Ok(());
    }
    if file_type.is_dir() {
      archive
        .append_path_with_name(path, name)
        .with_context(|| format!("failed to append directory {}", path.display()))?;
      for (name_child, path_child) in Self::entries_sorted(path)? {
        Self::entry_append_sorted(archive, &path_child, &name.join(name_child))?;
      }
      return Ok(());
    }
    archive
      .append_path_with_name(path, name)
      .with_context(|| format!("failed to append {}", path.display()))?;
    Ok(())
  }

  /// Packs `dir` like `append_dir_sorted` but records every entry as
  /// owned by numeric id 0 — the one owner a consumer applying the layer
  /// in a single-mapped-id user namespace can always represent.
  pub fn append_dir_sorted_root_owned<W: Write>(
    archive: &mut tar::Builder<W>,
    dir: &Path,
    exclude: &OsStr,
  ) -> Result<()> {
    for (name, path) in Self::entries_sorted(dir)? {
      if name == exclude {
        continue;
      }
      Self::entry_append_root_owned(archive, &path, Path::new(&name))?;
    }
    Ok(())
  }

  /// Reads one directory and returns its entries name-sorted, the shared
  /// order both packers impose for reproducible bytes.
  fn entries_sorted(dir: &Path) -> Result<Vec<(OsString, PathBuf)>> {
    let mut entries: Vec<(OsString, PathBuf)> = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
      let entry = entry?;
      entries.push((entry.file_name(), entry.path()));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
  }

  /// Appends one entry with its owner forced to id 0: header from the
  /// entry's own metadata, so mode and timestamp survive, then written by
  /// file type — link target kept, directory descended sorted, file
  /// contents streamed. Owner name fields stay empty so only the numeric
  /// id shows.
  fn entry_append_root_owned<W: Write>(archive: &mut tar::Builder<W>, path: &Path, name: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    let mut header = tar::Header::new_gnu();
    header.set_metadata(&metadata);
    header.set_uid(0);
    header.set_gid(0);

    let file_type = metadata.file_type();
    if file_type.is_symlink() {
      let target = fs::read_link(path).with_context(|| format!("failed to read link {}", path.display()))?;
      archive
        .append_link(&mut header, name, &target)
        .with_context(|| format!("failed to append symlink {}", path.display()))?;
      return Ok(());
    }
    if file_type.is_dir() {
      archive
        .append_data(&mut header, name, io::empty())
        .with_context(|| format!("failed to append directory {}", path.display()))?;
      for (name_child, path_child) in Self::entries_sorted(path)? {
        Self::entry_append_root_owned(archive, &path_child, &name.join(name_child))?;
      }
      return Ok(());
    }
    if file_type.is_file() {
      let file = fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
      archive
        .append_data(&mut header, name, file)
        .with_context(|| format!("failed to append {}", path.display()))?;
      return Ok(());
    }
    Self::special_append_root_owned(archive, header, path, name, &metadata)
  }

  /// Appends an entry that is neither file, directory, nor link: a fifo
  /// or device node, whose identity lives in the header alone (type flag,
  /// and for devices the major/minor from the on-disk device id). A
  /// socket has no archive representation and is refused.
  fn special_append_root_owned<W: Write>(
    archive: &mut tar::Builder<W>,
    mut header: tar::Header,
    path: &Path,
    name: &Path,
    metadata: &fs::Metadata,
  ) -> Result<()> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    let file_type = metadata.file_type();
    let entry_type = if file_type.is_fifo() {
      tar::EntryType::Fifo
    } else if file_type.is_char_device() {
      tar::EntryType::Char
    } else if file_type.is_block_device() {
      tar::EntryType::Block
    } else {
      anyhow::bail!("{}: unsupported file type for archiving", path.display());
    };
    header.set_entry_type(entry_type);
    header.set_size(0);

    let dev_id = metadata.rdev();
    let dev_major = ((dev_id >> 32) & 0xffff_f000) | ((dev_id >> 8) & 0x0000_0fff);
    let dev_minor = ((dev_id >> 12) & 0xffff_ff00) | (dev_id & 0x0000_00ff);
    header
      .set_device_major(dev_major as u32)
      .with_context(|| format!("failed to record device major for {}", path.display()))?;
    header
      .set_device_minor(dev_minor as u32)
      .with_context(|| format!("failed to record device minor for {}", path.display()))?;

    archive
      .append_data(&mut header, name, io::empty())
      .with_context(|| format!("failed to append {}", path.display()))?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn archive_names(bytes: &[u8]) -> Vec<String> {
    let mut archive = tar::Archive::new(bytes);
    archive
      .entries()
      .unwrap()
      .map(|entry| entry.unwrap().path().unwrap().to_string_lossy().into_owned())
      .collect()
  }

  /// The host-owned packer promises a name-determined sequence at every
  /// level, not just the top: nested entries must come out sorted no
  /// matter what order the filesystem enumerates them in.
  #[test]
  fn append_dir_sorted_orders_nested_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let path_dir_data = tmp.path().join("data");
    fs::create_dir_all(path_dir_data.join("sub")).unwrap();
    for name in ["z2", "m1", "a3", "k0"] {
      fs::write(path_dir_data.join(format!("{name}.txt")), b"x").unwrap();
      fs::write(path_dir_data.join("sub").join(format!("{name}.txt")), b"x").unwrap();
    }
    fs::write(tmp.path().join("top.txt"), b"x").unwrap();

    let mut builder = tar::Builder::new(Vec::new());
    builder.follow_symlinks(false);
    Tar::append_dir_sorted(&mut builder, tmp.path(), OsStr::new(".flatroot")).unwrap();
    let names = archive_names(&builder.into_inner().unwrap());

    let mut names_sorted = names.clone();
    names_sorted.sort();
    assert_eq!(names, names_sorted);
    assert!(names.contains(&"data/sub/a3.txt".to_string()));
  }

  /// The exclusion contract: the named top-level entry stays out of the
  /// archive while everything else is packed.
  #[test]
  fn append_dir_sorted_excludes_named_entry() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".flatroot")).unwrap();
    fs::write(tmp.path().join(".flatroot").join("manifest"), b"x").unwrap();
    fs::write(tmp.path().join("keep.txt"), b"x").unwrap();

    let mut builder = tar::Builder::new(Vec::new());
    builder.follow_symlinks(false);
    Tar::append_dir_sorted(&mut builder, tmp.path(), OsStr::new(".flatroot")).unwrap();
    let names = archive_names(&builder.into_inner().unwrap());

    assert_eq!(names, vec!["keep.txt".to_string()]);
  }
}
