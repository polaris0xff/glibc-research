//! Parses a downloaded RPM's binary layout: finds the compressed cpio
//! payload behind the variable-length headers, extracts it into the
//! rootfs unprivileged, and recovers the post-install scriptlet from
//! the header.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write as IoWrite};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;

use crate::internal::fs::Fs;
use crate::manifest::ManifestLayout;
use crate::package::PackageIdentity;

/// RPM header tag numbers the script recovery reads (rpmtag.h vocabulary).
const RPMTAG_POSTIN: u32 = 1024;
const RPMTAG_POSTINPROG: u32 = 1086;
/// Magic that opens both the signature header and the main header.
const RPM_HEADER_MAGIC: [u8; 4] = [0x8e, 0xad, 0xe8, 0x01];

/// cpio `newc` header constants: the two accepted magics, the fixed header
/// size, the end-of-archive entry name, and the field offsets (each field is
/// 8 hex characters wide).
const CPIO_MAGIC_NEWC: &str = "070701";
const CPIO_MAGIC_NEWC_CRC: &str = "070702";
const CPIO_HEADER_SIZE: usize = 110;
const CPIO_TRAILER: &str = "TRAILER!!!";
const CPIO_FIELD_INO: usize = 6;
const CPIO_FIELD_MODE: usize = 14;
const CPIO_FIELD_NLINK: usize = 38;
const CPIO_FIELD_FILESIZE: usize = 54;
const CPIO_FIELD_DEVMAJOR: usize = 62;
const CPIO_FIELD_DEVMINOR: usize = 70;
const CPIO_FIELD_NAMESIZE: usize = 94;

/// `c_mode` file-type mask and the three entry types this unprivileged
/// extractor can place; device and special nodes are skipped by design.
const MODE_TYPE_MASK: u32 = 0o170000;
const MODE_TYPE_DIR: u32 = 0o040000;
const MODE_TYPE_SYMLINK: u32 = 0o120000;
const MODE_TYPE_FILE: u32 = 0o100000;
const MODE_EXEC_ANY: u32 = 0o111;

/// One archive member as the `newc` header describes it: where it goes, what
/// kind of thing it is, and the bytes that travel with it (file content, or a
/// symlink's target).
struct CpioEntry {
  filename: String,
  mode: u32,
  ino: u32,
  nlink: u32,
  dev_major: u32,
  dev_minor: u32,
  data: Vec<u8>,
}

impl CpioEntry {
  /// The archive path with the leading `./` or `/` stripped; `None` for the
  /// bare `.` and empty entries that name nothing to place.
  fn rel_path(&self) -> Option<PathBuf> {
    let clean = self
      .filename
      .strip_prefix("./")
      .or_else(|| self.filename.strip_prefix('/'))
      .unwrap_or(&self.filename);
    if clean.is_empty() || clean == "." {
      return None;
    }
    Some(PathBuf::from(clean))
  }
}

/// Identity shared by every archive member naming the same on-disk file:
/// the `newc` device pair plus inode number.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct HardlinkKey {
  dev_major: u32,
  dev_minor: u32,
  ino: u32,
}

impl HardlinkKey {
  fn of(entry: &CpioEntry) -> Self {
    HardlinkKey {
      dev_major: entry.dev_major,
      dev_minor: entry.dev_minor,
      ino: entry.ino,
    }
  }
}

/// Reunites a hardlink group split across archive members: the payload
/// carries the file's bytes on a single member of the group and lists the
/// other names with zero size, so byte-less names wait here until the
/// bytes are written and then become hardlinks to that file.
struct HardlinkGroups {
  written_by_key: HashMap<HardlinkKey, PathBuf>,
  waiting_by_key: HashMap<HardlinkKey, Vec<(PathBuf, u32)>>,
}

impl HardlinkGroups {
  fn new() -> Self {
    HardlinkGroups {
      written_by_key: HashMap::new(),
      waiting_by_key: HashMap::new(),
    }
  }

  /// Places one group member, deferring a byte-less name until the member
  /// carrying the bytes has been written.
  fn member_place(&mut self, entry: &CpioEntry, dest: &Path) -> Result<()> {
    let key = HardlinkKey::of(entry);
    if !entry.data.is_empty() {
      RpmContainer::file_place(dest, entry.mode, &entry.data)?;
      for (waiting, _) in self.waiting_by_key.remove(&key).unwrap_or_default() {
        Self::link_place(dest, &waiting)?;
      }
      self.written_by_key.insert(key, dest.to_path_buf());
      return Ok(());
    }
    if let Some(written) = self.written_by_key.get(&key) {
      return Self::link_place(written, dest);
    }
    self
      .waiting_by_key
      .entry(key)
      .or_default()
      .push((dest.to_path_buf(), entry.mode));
    Ok(())
  }

  /// Settles the groups whose bytes never arrived — the file is genuinely
  /// empty: the first name receives the empty file and the rest link to it.
  fn finish(self) -> Result<()> {
    for names in self.waiting_by_key.into_values() {
      let Some(((first, mode), rest)) = names.split_first() else {
        continue;
      };
      RpmContainer::file_place(first, *mode, &[])?;
      for (name, _) in rest {
        Self::link_place(first, name)?;
      }
    }
    Ok(())
  }

  /// One additional name for an already-written file, replacing whatever
  /// occupies the destination.
  fn link_place(written: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
      std::fs::create_dir_all(parent)?;
    }
    Fs::remove_lenient(dest);
    std::fs::hard_link(written, dest)
      .with_context(|| format!("Failed to hardlink {} to {}", dest.display(), written.display()))?;
    Ok(())
  }
}

/// One RPM's raw bytes held in memory, with the parsing of its binary
/// layout.
pub(super) struct RpmContainer<'a> {
  data: &'a [u8],
}

impl<'a> RpmContainer<'a> {
  /// Wraps the package's raw bytes; nothing is copied or parsed yet.
  pub(super) fn open(data: &'a [u8]) -> Self {
    RpmContainer { data }
  }

  /// Where the compressed payload begins, computed from the
  /// variable-length signature and main headers; `None` when the data
  /// does not match the RPM layout.
  fn payload_offset(&self) -> Option<usize> {
    let data = self.data;
    if data.len() < 112 {
      return None;
    }

    // Signature header at offset 96
    let sig_magic = &data[96..100];
    if sig_magic != RPM_HEADER_MAGIC {
      return None;
    }
    let sig_nindex = Self::u32_be(data, 104) as usize;
    let sig_hsize = Self::u32_be(data, 108) as usize;
    let sig_total = 16 + sig_nindex * 16 + sig_hsize;
    // Signature is padded to 8-byte alignment
    let sig_padded = sig_total + (8 - sig_total % 8) % 8;
    let header_start = 96 + sig_padded;

    if data.len() < header_start + 16 {
      return None;
    }

    // Main header
    let h_magic = &data[header_start..header_start + 4];
    if h_magic != RPM_HEADER_MAGIC {
      return None;
    }
    let h_nindex = Self::u32_be(data, header_start + 8) as usize;
    let h_hsize = Self::u32_be(data, header_start + 12) as usize;
    let h_total = 16 + h_nindex * 16 + h_hsize;

    let payload_start = header_start + h_total;
    if payload_start < data.len() {
      Some(payload_start)
    } else {
      None
    }
  }

  /// Locates and decompresses the payload, recognizing xz, gzip, or
  /// zstd from its leading magic bytes; an unrecognized format is an
  /// error.
  pub(super) fn payload_decompress(&self) -> Result<Vec<u8>> {
    let payload_offset = self.payload_offset().context("No compressed payload found in RPM")?;
    let payload = &self.data[payload_offset..];

    if payload.starts_with(&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]) {
      let mut decoder = xz2::read::XzDecoder::new(payload);
      let mut out = Vec::new();
      decoder.read_to_end(&mut out)?;
      Ok(out)
    } else if payload.starts_with(&[0x1f, 0x8b]) {
      let mut decoder = GzDecoder::new(payload);
      let mut out = Vec::new();
      decoder.read_to_end(&mut out)?;
      Ok(out)
    } else if payload.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
      let mut decoder = zstd::Decoder::new(payload)?;
      let mut out = Vec::new();
      decoder.read_to_end(&mut out)?;
      Ok(out)
    } else {
      bail!("Unknown compression format in RPM payload");
    }
  }

  /// Extracts the decompressed cpio archive into the rootfs:
  /// directories created, symlinks recreated, files written with the
  /// executable bit preserved. Device nodes are skipped (unprivileged
  /// extraction), and an existing empty directory can be replaced by a
  /// symlink for merged-/usr layouts. Returns the files placed.
  pub(super) fn cpio_extract(data: &[u8], root: &Path) -> Result<Vec<PathBuf>> {
    let mut cursor = Cursor::new(data);
    let len = data.len() as u64;
    let mut files: Vec<PathBuf> = Vec::new();
    let mut hardlinks = HardlinkGroups::new();

    while cursor.position() < len {
      let Some(entry) = Self::cpio_entry_read(&mut cursor)? else {
        break;
      };
      let Some(rel_path) = entry.rel_path() else {
        continue;
      };
      let dest = root.join(&rel_path);

      let placed = match entry.mode & MODE_TYPE_MASK {
        MODE_TYPE_DIR => Self::dir_place(&dest)?,
        MODE_TYPE_SYMLINK => Self::symlink_place(&entry, &dest)?,
        MODE_TYPE_FILE if entry.nlink > 1 => {
          hardlinks.member_place(&entry, &dest)?;
          true
        }
        MODE_TYPE_FILE => {
          Self::file_place(&dest, entry.mode, &entry.data)?;
          true
        }
        _ => false,
      };
      if placed && !ManifestLayout::is_metadata(&rel_path) {
        files.push(rel_path);
      }
    }
    hardlinks.finish()?;

    Ok(files)
  }

  /// One `newc` header plus its name and data, with the 4-byte alignment the
  /// format pads to already consumed. `None` marks the end of the archive: a
  /// short read, a non-newc header, or the trailer entry.
  fn cpio_entry_read(cursor: &mut Cursor<&[u8]>) -> Result<Option<CpioEntry>> {
    let mut header = [0u8; CPIO_HEADER_SIZE];
    if cursor.read_exact(&mut header).is_err() {
      return Ok(None);
    }
    let magic = std::str::from_utf8(&header[0..6]).unwrap_or("");
    if magic != CPIO_MAGIC_NEWC && magic != CPIO_MAGIC_NEWC_CRC {
      return Ok(None);
    }

    let namesize = Self::cpio_field(&header, CPIO_FIELD_NAMESIZE, "namesize")? as usize;
    let filesize = Self::cpio_field(&header, CPIO_FIELD_FILESIZE, "filesize")? as usize;
    let mode = Self::cpio_field(&header, CPIO_FIELD_MODE, "mode")?;
    let ino = Self::cpio_field(&header, CPIO_FIELD_INO, "ino")?;
    let nlink = Self::cpio_field(&header, CPIO_FIELD_NLINK, "nlink")?;
    let dev_major = Self::cpio_field(&header, CPIO_FIELD_DEVMAJOR, "devmajor")?;
    let dev_minor = Self::cpio_field(&header, CPIO_FIELD_DEVMINOR, "devminor")?;

    let mut namebuf = vec![0u8; namesize];
    cursor.read_exact(&mut namebuf)?;
    let filename = std::str::from_utf8(&namebuf)
      .unwrap_or("")
      .trim_end_matches('\0')
      .to_string();
    Self::cpio_align(cursor, CPIO_HEADER_SIZE + namesize);

    if filename == CPIO_TRAILER {
      return Ok(None);
    }

    let mut data = vec![0u8; filesize];
    if filesize > 0 {
      cursor.read_exact(&mut data)?;
    }
    Self::cpio_align(cursor, filesize);

    Ok(Some(CpioEntry {
      filename,
      mode,
      ino,
      nlink,
      dev_major,
      dev_minor,
      data,
    }))
  }

  /// One 8-hex-digit `newc` header field at its fixed offset, refused with a
  /// field-naming error when the bytes are not hex.
  fn cpio_field(header: &[u8; CPIO_HEADER_SIZE], offset: usize, field: &str) -> Result<u32> {
    let raw = std::str::from_utf8(&header[offset..offset + 8])
      .with_context(|| format!("cpio {field} field is not valid UTF-8"))?;
    u32::from_str_radix(raw, 16).with_context(|| format!("cpio {field} field '{raw}' is not a hex integer"))
  }

  /// `newc` pads both the name and the data to a 4-byte boundary.
  fn cpio_align(cursor: &mut Cursor<&[u8]>, consumed: usize) {
    let padding = (4 - consumed % 4) % 4;
    cursor.set_position(cursor.position() + padding as u64);
  }

  fn dir_place(dest: &Path) -> Result<bool> {
    std::fs::create_dir_all(dest)?;
    Ok(true)
  }

  /// Recreate one symlink, clearing whatever already occupies the destination.
  /// Distributions that unify their system directories expect an existing
  /// directory to be turned into a link (e.g. /bin → usr/bin), so an empty
  /// directory is removed to make way; a non-empty one cannot be replaced, and
  /// the failed link attempt below reports the obstruction. A failed link is
  /// reported and skipped rather than fatal, so one stubborn path does not
  /// abandon an otherwise good extraction.
  fn symlink_place(entry: &CpioEntry, dest: &Path) -> Result<bool> {
    let target = std::str::from_utf8(&entry.data).unwrap_or("");
    if target.is_empty() {
      return Ok(false);
    }
    if let Some(parent) = dest.parent() {
      std::fs::create_dir_all(parent)?;
    }
    Self::occupant_clear(dest);
    match std::os::unix::fs::symlink(target, dest) {
      Ok(()) => Ok(true),
      Err(e) => {
        eprintln!("    warning: failed to create symlink {}: {}", dest.display(), e);
        Ok(false)
      }
    }
  }

  /// Clear a plain file, dangling link, or empty directory standing where a
  /// new entry must land; a non-empty directory deliberately survives.
  fn occupant_clear(dest: &Path) {
    if dest.symlink_metadata().is_err() {
      return;
    }
    if dest.is_dir() && !dest.is_symlink() {
      Fs::remove_dir_lenient(dest);
      return;
    }
    Fs::remove_lenient(dest);
  }

  fn file_place(dest: &Path, mode: u32, data: &[u8]) -> Result<()> {
    if let Some(parent) = dest.parent() {
      std::fs::create_dir_all(parent)?;
    }
    Fs::remove_lenient(dest);
    let mut f = std::fs::File::create(dest)?;
    f.write_all(data)?;
    if mode & MODE_EXEC_ANY != 0 {
      use std::os::unix::fs::PermissionsExt;
      std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
  }

  /// Recovers the POSTIN scriptlet from the RPM header and saves it per
  /// package for the post-install pass. A Lua scriptlet is saved as
  /// `.lua` so the script runner can detect it and skip it with an
  /// explicit message.
  pub(super) fn scripts_stage(&self, root: &Path, pkg: &PackageIdentity) -> Result<()> {
    let Some((index_start, store_start, nindex)) = self.header_main_locate() else {
      return Ok(());
    };
    let (script_offset, interpreter) = self.postin_locate(index_start, store_start, nindex);
    let Some(offset) = script_offset else {
      return Ok(());
    };
    let Some(script) = self.store_string(store_start + offset) else {
      return Ok(());
    };
    if script.trim().is_empty() {
      return Ok(());
    }

    let scripts_dir = ManifestLayout::new(root).dir_scripts_entry(pkg);
    std::fs::create_dir_all(&scripts_dir)?;

    // Lua scriptlets use RPM's embedded Lua with posix.* and rpm.* extensions
    // not available in standard Lua. Save as .lua so the runner can detect
    // and skip them with an explicit message.
    let is_lua = interpreter
      .as_deref()
      .is_some_and(|i| i == "<lua>" || i.contains("lua"));
    let filename = if is_lua { "postinst.lua" } else { "postinst" };
    std::fs::write(scripts_dir.join(filename), script)?;
    Ok(())
  }

  /// The main header is the second header-magic occurrence (the first
  /// opens the signature header). Returns the index area's start, the
  /// store's start, and the entry count; `None` when the data carries
  /// no readable main header, which just means there is no scriptlet to
  /// recover.
  fn header_main_locate(&self) -> Option<(usize, usize, usize)> {
    let data = self.data;
    let mut found = 0;
    let mut main_header_start = 0;
    for i in 96..data.len().saturating_sub(4) {
      if data[i..i + 4] != RPM_HEADER_MAGIC {
        continue;
      }
      found += 1;
      if found == 2 {
        main_header_start = i;
        break;
      }
    }
    if found < 2 || main_header_start + 16 > data.len() {
      return None;
    }

    // Header layout: magic(4) + reserved(4) + nindex(4) + hsize(4).
    let nindex = Self::u32_be(data, main_header_start + 8) as usize;
    let hsize = Self::u32_be(data, main_header_start + 12) as usize;
    let index_start = main_header_start + 16;
    let store_start = index_start + nindex * 16;
    if store_start + hsize > data.len() {
      return None;
    }
    Some((index_start, store_start, nindex))
  }

  /// Scan the index entries for the post-install scriptlet (POSTIN) and its
  /// interpreter (POSTINPROG). Each index entry is tag(4) + type(4) +
  /// offset(4) + count(4).
  fn postin_locate(&self, index_start: usize, store_start: usize, nindex: usize) -> (Option<usize>, Option<String>) {
    let data = self.data;
    let mut script_offset: Option<usize> = None;
    let mut interpreter: Option<String> = None;

    for i in 0..nindex {
      let entry_offset = index_start + i * 16;
      let tag = Self::u32_be(data, entry_offset);
      let offset = Self::u32_be(data, entry_offset + 8) as usize;
      match tag {
        RPMTAG_POSTIN => script_offset = Some(offset),
        RPMTAG_POSTINPROG => interpreter = self.store_string(store_start + offset),
        _ => {}
      }
      if script_offset.is_some() && interpreter.is_some() {
        break;
      }
    }
    (script_offset, interpreter)
  }

  /// The NUL-terminated string starting at an absolute store position,
  /// or `None` when the position falls outside the data.
  fn store_string(&self, start: usize) -> Option<String> {
    if start >= self.data.len() {
      return None;
    }
    let end = self.data[start..]
      .iter()
      .position(|&b| b == 0)
      .map(|p| start + p)
      .unwrap_or(self.data.len());
    Some(String::from_utf8_lossy(&self.data[start..end]).to_string())
  }

  /// Big-endian u32 at a fixed offset — the encoding every RPM header field uses.
  fn u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::MetadataExt;

  /// One `newc` archive member: magic, thirteen 8-hex-digit fields, the
  /// NUL-terminated name, then the data, each padded to a 4-byte boundary.
  fn newc_entry(name: &str, mode: u32, ino: u32, nlink: u32, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"070701");
    let namesize = (name.len() + 1) as u32;
    let fields = [ino, mode, 0, 0, nlink, 0, data.len() as u32, 0, 0, 0, 0, namesize, 0];
    for field in fields {
      out.extend_from_slice(format!("{field:08x}").as_bytes());
    }
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    while out.len() % 4 != 0 {
      out.push(0);
    }
    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
      out.push(0);
    }
    out
  }

  fn archive(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = entries.concat();
    out.extend_from_slice(&newc_entry("TRAILER!!!", 0, 0, 1, b""));
    out
  }

  #[test]
  fn cpio_extract_plain_file() {
    let tmp = tempfile::tempdir().unwrap();
    let data = archive(&[newc_entry("./usr/bin/tool", 0o100755, 1, 1, b"payload")]);
    let files = RpmContainer::cpio_extract(&data, tmp.path()).unwrap();
    assert_eq!(files, vec![PathBuf::from("usr/bin/tool")]);
    assert_eq!(std::fs::read(tmp.path().join("usr/bin/tool")).unwrap(), b"payload");
  }

  #[test]
  fn cpio_extract_hardlinks_data_on_last_member() {
    let tmp = tempfile::tempdir().unwrap();
    let data = archive(&[
      newc_entry("./usr/bin/gcc", 0o100755, 7, 2, b""),
      newc_entry("./usr/bin/cc-real", 0o100755, 7, 2, b"elf"),
    ]);
    let files = RpmContainer::cpio_extract(&data, tmp.path()).unwrap();
    assert_eq!(files.len(), 2);
    let first = tmp.path().join("usr/bin/gcc");
    let last = tmp.path().join("usr/bin/cc-real");
    assert_eq!(std::fs::read(&first).unwrap(), b"elf");
    assert_eq!(std::fs::read(&last).unwrap(), b"elf");
    assert_eq!(std::fs::metadata(&first).unwrap().ino(), std::fs::metadata(&last).unwrap().ino());
  }

  #[test]
  fn cpio_extract_hardlinks_data_on_first_member() {
    let tmp = tempfile::tempdir().unwrap();
    let data = archive(&[
      newc_entry("./usr/bin/cc-real", 0o100755, 7, 2, b"elf"),
      newc_entry("./usr/bin/gcc", 0o100755, 7, 2, b""),
    ]);
    RpmContainer::cpio_extract(&data, tmp.path()).unwrap();
    let first = tmp.path().join("usr/bin/cc-real");
    let last = tmp.path().join("usr/bin/gcc");
    assert_eq!(std::fs::read(&first).unwrap(), b"elf");
    assert_eq!(std::fs::read(&last).unwrap(), b"elf");
    assert_eq!(std::fs::metadata(&first).unwrap().ino(), std::fs::metadata(&last).unwrap().ino());
  }

  #[test]
  fn cpio_extract_hardlinks_empty_group() {
    let tmp = tempfile::tempdir().unwrap();
    let data = archive(&[
      newc_entry("./etc/one", 0o100644, 9, 2, b""),
      newc_entry("./etc/two", 0o100644, 9, 2, b""),
    ]);
    RpmContainer::cpio_extract(&data, tmp.path()).unwrap();
    let one = std::fs::metadata(tmp.path().join("etc/one")).unwrap();
    let two = std::fs::metadata(tmp.path().join("etc/two")).unwrap();
    assert_eq!(one.len(), 0);
    assert_eq!(two.len(), 0);
    assert_eq!(one.ino(), two.ino());
  }

  #[test]
  fn cpio_extract_distinct_groups_stay_apart() {
    let tmp = tempfile::tempdir().unwrap();
    let data = archive(&[
      newc_entry("./a", 0o100644, 3, 2, b""),
      newc_entry("./b", 0o100644, 4, 2, b""),
      newc_entry("./a2", 0o100644, 3, 2, b"alpha"),
      newc_entry("./b2", 0o100644, 4, 2, b"beta"),
    ]);
    RpmContainer::cpio_extract(&data, tmp.path()).unwrap();
    assert_eq!(std::fs::read(tmp.path().join("a")).unwrap(), b"alpha");
    assert_eq!(std::fs::read(tmp.path().join("b")).unwrap(), b"beta");
  }
}
