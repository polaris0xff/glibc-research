//! Archive extraction across the four package formats, driven through the
//! public `PackageFormat::extract` trait on the real format backends
//! (`DebFormat`, `RpmFormat`, `ApkFormat`, `PacmanFormat`). Each fixture is
//! a genuine archive of the format under test, hand-built with the `ar`,
//! `tar`, `flate2`, `xz2`, and `zstd` crates, or a deliberately malformed
//! byte slice for the error paths. No network, no Docker — only the
//! crate's own extraction logic against bytes we control.

#![allow(clippy::octal_escapes)]

use std::io::Write;
use std::path::{Path, PathBuf};

use flatroot::package::PackageIdentity;
use flatroot::pkg::{ApkFormat, DebFormat, PackageFormat, PacmanFormat, RpmFormat};

// ===========================================================================
// Shared fixture builders
// ===========================================================================

/// The identity an extraction files its staged scripts under; every fixture
/// here targets the one test architecture.
fn ident(name: &str) -> PackageIdentity {
  PackageIdentity {
    name: name.to_string(),
    arch: "x86_64".to_string(),
  }
}

fn compress_gzip(plain: &[u8]) -> Vec<u8> {
  let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
  enc.write_all(plain).unwrap();
  enc.finish().unwrap()
}

fn compress_xz(plain: &[u8]) -> Vec<u8> {
  let mut enc = xz2::write::XzEncoder::new(Vec::new(), 6);
  enc.write_all(plain).unwrap();
  enc.finish().unwrap()
}

fn compress_zstd(plain: &[u8]) -> Vec<u8> {
  zstd::encode_all(plain, 3).unwrap()
}

/// Build an uncompressed tar from a list of (path, mode, content) regular
/// files plus an optional set of (path, target) symlinks.
fn tar_build(files: &[(&str, u32, &[u8])], symlinks: &[(&str, &str)]) -> Vec<u8> {
  let mut buf: Vec<u8> = Vec::new();
  {
    let mut builder = tar::Builder::new(&mut buf);
    for (path, mode, content) in files {
      let mut header = tar::Header::new_gnu();
      header.set_size(content.len() as u64);
      header.set_mode(*mode);
      header.set_entry_type(tar::EntryType::Regular);
      header.set_cksum();
      builder.append_data(&mut header, path, *content).unwrap();
    }
    for (path, target) in symlinks {
      let mut header = tar::Header::new_gnu();
      header.set_size(0);
      header.set_mode(0o777);
      header.set_entry_type(tar::EntryType::Symlink);
      header.set_cksum();
      builder.append_link(&mut header, path, target).unwrap();
    }
    builder.finish().unwrap();
  }
  buf
}

// ===========================================================================
// DEB extraction (ar container holding data.tar.* + control.tar.*)
// ===========================================================================

/// Build a `.deb`: an `ar` archive carrying `debian-binary`, an optional
/// `control.tar.*` and a `data.tar.*` member. Each named member's bytes are
/// stored verbatim, so the caller chooses the on-name codec.
fn deb_build(members: &[(&str, &[u8])]) -> Vec<u8> {
  let mut buf: Vec<u8> = Vec::new();
  {
    let mut builder = ar::Builder::new(&mut buf);
    for (name, bytes) in members {
      let header = ar::Header::new(name.as_bytes().to_vec(), bytes.len() as u64);
      builder.append(&header, *bytes).unwrap();
    }
  }
  buf
}

fn compress_bzip2(plain: &[u8]) -> Vec<u8> {
  let mut enc = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default());
  enc.write_all(plain).unwrap();
  enc.finish().unwrap()
}

fn deb_data_for_member(member: &str, inner_tar: &[u8]) -> Vec<u8> {
  if member.ends_with(".gz") {
    compress_gzip(inner_tar)
  } else if member.ends_with(".xz") {
    compress_xz(inner_tar)
  } else if member.ends_with(".zst") {
    compress_zstd(inner_tar)
  } else if member.ends_with(".bz2") {
    compress_bzip2(inner_tar)
  } else {
    inner_tar.to_vec()
  }
}

// covers: PKG-013
#[test]
fn deb_extract_auto_detects_data_tar_codec() {
  let inner = tar_build(
    &[
      ("usr/bin/hello", 0o755, b"#!/bin/sh\necho hi\n"),
      ("usr/share/doc/hello/copyright", 0o644, b"GPL"),
    ],
    &[],
  );

  // The four compressed forms plus the uncompressed `data.tar` must all
  // produce identical on-disk output — the codec is auto-detected from the
  // ar member name alone.
  for member in ["data.tar.gz", "data.tar.xz", "data.tar.zst", "data.tar.bz2", "data.tar"] {
    let data = deb_data_for_member(member, &inner);
    let deb = deb_build(&[("debian-binary", b"2.0\n"), (member, data.as_slice())]);

    let dir = tempfile::tempdir().unwrap();
    let path_file_archive = dir.path().join("pkg.deb");
    std::fs::write(&path_file_archive, &deb).unwrap();

    let root = tempfile::tempdir().unwrap();
    let files = DebFormat
      .extract(&path_file_archive, root.path(), &ident("hello"))
      .unwrap_or_else(|e| panic!("{member}: deb extract must succeed: {e}"));

    let on_disk =
      std::fs::read(root.path().join("usr/bin/hello")).unwrap_or_else(|_| panic!("{member}: usr/bin/hello must exist"));
    assert_eq!(on_disk, b"#!/bin/sh\necho hi\n", "{member}: extracted content must match");
    assert!(
      files.iter().any(|p| p == &PathBuf::from("usr/bin/hello")),
      "{member}: returned file list must record the payload path, got {:?}",
      files
    );
  }
}

// covers: PKG-014
#[test]
fn deb_extract_missing_data_tar_is_hard_error() {
  // An ar archive with only control.tar (no data.tar.*) must bail.
  let control_inner = tar_build(&[("postinst", 0o755, b"#!/bin/sh\nexit 0\n")], &[]);
  let control = compress_gzip(&control_inner);
  let deb = deb_build(&[("debian-binary", b"2.0\n"), ("control.tar.gz", control.as_slice())]);

  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("nodata.deb");
  std::fs::write(&path_file_archive, &deb).unwrap();

  let root = tempfile::tempdir().unwrap();
  let err = DebFormat
    .extract(&path_file_archive, root.path(), &ident("nodata"))
    .err()
    .expect("missing data.tar must error");
  let msg = format!("{err:#}");
  assert!(
    msg.contains(&format!("No data.tar.* found in {}", path_file_archive.display())),
    "expected 'No data.tar.* found in <path>', got: {msg}"
  );
}

// covers: PKG-015
#[test]
fn deb_extract_unopenable_archive_errors_with_context() {
  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("does-not-exist.deb");
  let root = tempfile::tempdir().unwrap();

  let err = DebFormat
    .extract(&path_file_archive, root.path(), &ident("ghost"))
    .err()
    .expect("opening a non-existent archive must error");
  let msg = format!("{err:#}");
  assert!(
    msg.contains(&format!("Failed to open package archive: {}", path_file_archive.display())),
    "expected 'Failed to open package archive: <path>', got: {msg}"
  );
}

// ===========================================================================
// APK extraction (three concatenated gzip streams: signature, control, data)
// ===========================================================================

/// Build an `.apk`: signature gzip stream (content irrelevant, discarded),
/// then a control gzip-tar stream, then a data gzip-tar stream. The reader
/// peels them one gzip member at a time off a single cursor.
fn apk_build(control_tar: &[u8], data_tar: &[u8]) -> Vec<u8> {
  let mut out: Vec<u8> = Vec::new();
  out.extend_from_slice(&compress_gzip(b"signature-stream-discarded"));
  out.extend_from_slice(&compress_gzip(control_tar));
  out.extend_from_slice(&compress_gzip(data_tar));
  out
}

// covers: PKG-044
#[test]
fn apk_extract_stages_only_install_scripts() {
  // Control tar carries .post-install (kept), .pre-install (kept),
  // .PKGINFO and a stray control file (both skipped).
  let control = tar_build(
    &[
      (".post-install", 0o755, b"#!/bin/sh\necho post\n"),
      (".pre-install", 0o755, b"#!/bin/sh\necho pre\n"),
      (".PKGINFO", 0o644, b"pkgname = demo\n"),
      (".trigger", 0o644, b"some trigger\n"),
    ],
    &[],
  );
  let data = tar_build(&[("usr/bin/demo", 0o755, b"binary")], &[]);
  let apk = apk_build(&control, &data);

  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("demo.apk");
  std::fs::write(&path_file_archive, &apk).unwrap();

  let root = tempfile::tempdir().unwrap();
  let files = ApkFormat
    .extract(&path_file_archive, root.path(), &ident("demo"))
    .expect("apk extract must succeed");

  let scripts_dir = root.path().join(".flatroot/scripts/demo:x86_64");
  // Leading dot is stripped from the saved script name.
  assert!(scripts_dir.join("post-install").exists(), "post-install must be staged");
  assert!(scripts_dir.join("pre-install").exists(), "pre-install must be staged");
  // .PKGINFO and other control files are NOT staged.
  assert!(!scripts_dir.join("PKGINFO").exists(), ".PKGINFO must not be staged");
  assert!(!scripts_dir.join(".PKGINFO").exists(), ".PKGINFO must not be staged");
  assert!(!scripts_dir.join("trigger").exists(), "other control files must not be staged");

  // The data file is extracted and reported as payload (not metadata).
  assert!(root.path().join("usr/bin/demo").exists(), "data payload must be extracted");
  assert!(
    files.iter().any(|p| p == &PathBuf::from("usr/bin/demo")),
    "returned files must record the payload, got {:?}",
    files
  );
}

// covers: PKG-045
#[test]
fn apk_extract_unreadable_archive_errors_with_context() {
  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("missing.apk");
  let root = tempfile::tempdir().unwrap();

  let err = ApkFormat
    .extract(&path_file_archive, root.path(), &ident("missing"))
    .err()
    .expect("reading a non-existent apk must error");
  let msg = format!("{err:#}");
  assert!(
    msg.contains(&format!("Failed to read {}", path_file_archive.display())),
    "expected 'Failed to read <path>', got: {msg}"
  );
}

// ===========================================================================
// Pacman extraction (single zstd-compressed tar)
// ===========================================================================

/// Build a `.pkg.tar.zst`: a tar (carrying payload files, an optional
/// `.INSTALL`, and metadata dotfiles) compressed with zstd.
fn pacman_build(tar_bytes: &[u8]) -> Vec<u8> {
  compress_zstd(tar_bytes)
}

// covers: PKG-054
#[test]
fn pacman_extract_replaces_plain_file_preserves_directory() {
  // Pre-seed the root: a plain file where a later entry's file lands, and a
  // directory where a later entry's directory lands.
  let root = tempfile::tempdir().unwrap();
  std::fs::create_dir_all(root.path().join("usr/lib")).unwrap();
  // Obstructing plain file at a path the package ships as a regular file.
  std::fs::write(root.path().join("usr/lib/libdemo.so"), b"stale").unwrap();
  // Pre-existing directory that the package also ships as a directory — must
  // be preserved (not removed) so its existing contents survive.
  std::fs::create_dir_all(root.path().join("usr/share/demo")).unwrap();
  std::fs::write(root.path().join("usr/share/demo/keep.txt"), b"preserved").unwrap();

  let inner = tar_build(
    &[
      ("usr/lib/libdemo.so", 0o644, b"fresh-library-bytes"),
      ("usr/share/demo/new.txt", 0o644, b"new-content"),
      (".PKGINFO", 0o644, b"pkgname = demo\n"),
    ],
    &[],
  );
  let pkg = pacman_build(&inner);

  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("demo.pkg.tar.zst");
  std::fs::write(&path_file_archive, &pkg).unwrap();

  let files = PacmanFormat
    .extract(&path_file_archive, root.path(), &ident("demo"))
    .expect("pacman extract must succeed");

  // The plain-file destination is cleanly replaced by the package's content.
  assert_eq!(
    std::fs::read(root.path().join("usr/lib/libdemo.so")).unwrap(),
    b"fresh-library-bytes",
    "an obstructing plain file must be replaced by the package content"
  );
  // The pre-existing directory was preserved: its earlier file is still there
  // alongside the newly-added one.
  assert_eq!(
    std::fs::read(root.path().join("usr/share/demo/keep.txt")).unwrap(),
    b"preserved",
    "a pre-existing directory must be preserved, not clobbered"
  );
  assert!(root.path().join("usr/share/demo/new.txt").exists(), "the new file under it must extract");
  // Metadata dotfile is not reported as payload.
  assert!(!files.iter().any(|p| p == &PathBuf::from(".PKGINFO")), ".PKGINFO must not be reported as payload");
  assert!(files.iter().any(|p| p == &PathBuf::from("usr/lib/libdemo.so")));
}

// covers: PKG-064
#[test]
fn pacman_extract_unopenable_archive_aborts() {
  // Non-existent file → File::open fails and the install aborts.
  let dir = tempfile::tempdir().unwrap();
  let path_missing = dir.path().join("missing.pkg.tar.zst");
  let root = tempfile::tempdir().unwrap();
  assert!(
    PacmanFormat
      .extract(&path_missing, root.path(), &ident("missing"))
      .is_err(),
    "opening a non-existent pacman archive must error"
  );

  // A file that is NOT zstd → zstd::Decoder construction fails and the
  // install aborts rather than silently producing nothing.
  let path_not_zstd = dir.path().join("notzstd.pkg.tar.zst");
  std::fs::write(&path_not_zstd, b"this is plainly not a zstd stream").unwrap();
  assert!(
    PacmanFormat
      .extract(&path_not_zstd, root.path(), &ident("notzstd"))
      .is_err(),
    "a non-zstd archive must error at decoder construction"
  );
}

// ===========================================================================
// RPM extraction (lead + signature header + main header + compressed cpio)
// ===========================================================================

/// Append a big-endian u32.
fn push_be_u32(out: &mut Vec<u8>, v: u32) {
  out.extend_from_slice(&v.to_be_bytes());
}

/// Build an RPM header section (signature or main) with the given index
/// entries and store. Returns the raw bytes (magic + reserved + counts +
/// index + store), 8-byte aligned with trailing padding the caller adds.
///
/// Each entry is (tag, type, offset, count). The store is the concatenated
/// data the offsets point into.
fn rpm_header_section(entries: &[(u32, u32, u32, u32)], store: &[u8]) -> Vec<u8> {
  let mut out: Vec<u8> = Vec::new();
  out.extend_from_slice(&[0x8e, 0xad, 0xe8, 0x01]); // magic
  push_be_u32(&mut out, 0); // reserved
  push_be_u32(&mut out, entries.len() as u32); // nindex
  push_be_u32(&mut out, store.len() as u32); // hsize
  for (tag, ty, offset, count) in entries {
    push_be_u32(&mut out, *tag);
    push_be_u32(&mut out, *ty);
    push_be_u32(&mut out, *offset);
    push_be_u32(&mut out, *count);
  }
  out.extend_from_slice(store);
  out
}

/// Assemble a complete RPM: a 96-byte lead, the signature header padded to
/// 8 bytes, the main header, then the compressed cpio payload appended raw.
fn rpm_build(main_entries: &[(u32, u32, u32, u32)], main_store: &[u8], payload: &[u8]) -> Vec<u8> {
  let mut out: Vec<u8> = vec![0u8; 96]; // lead (96 bytes, contents irrelevant)

  // Signature header: empty (no entries, no store).
  let sig = rpm_header_section(&[], &[]);
  out.extend_from_slice(&sig);
  // Pad the signature region to an 8-byte boundary, measured from offset 96.
  let sig_len = sig.len();
  let pad = (8 - sig_len % 8) % 8;
  out.extend(std::iter::repeat_n(0u8, pad));

  // Main header.
  let main = rpm_header_section(main_entries, main_store);
  out.extend_from_slice(&main);

  // Payload immediately follows the main header.
  out.extend_from_slice(payload);
  out
}

/// Encode one newc cpio entry (header + name + NUL + padding + data + padding).
fn cpio_entry(out: &mut Vec<u8>, name: &str, mode: u32, data: &[u8]) {
  let name_with_nul: Vec<u8> = {
    let mut v = name.as_bytes().to_vec();
    v.push(0);
    v
  };
  let namesize = name_with_nul.len() as u32;
  let filesize = data.len() as u32;

  // 110-byte newc header: 13 hex fields of 8 chars each + 6-char magic.
  // Layout used by the reader: magic[0..6], mode[14..22], filesize[54..62],
  // namesize[94..102]. Fill all fields with valid 8-hex values.
  let hex8 = |v: u32| format!("{v:08X}");
  let mut header = String::new();
  header.push_str("070701"); // magic (6)
  header.push_str(&hex8(0)); // ino [6..14]
  header.push_str(&hex8(mode)); // mode [14..22]
  header.push_str(&hex8(0)); // uid [22..30]
  header.push_str(&hex8(0)); // gid [30..38]
  header.push_str(&hex8(1)); // nlink [38..46]
  header.push_str(&hex8(0)); // mtime [46..54]
  header.push_str(&hex8(filesize)); // filesize [54..62]
  header.push_str(&hex8(0)); // devmajor [62..70]
  header.push_str(&hex8(0)); // devminor [70..78]
  header.push_str(&hex8(0)); // rdevmajor [78..86]
  header.push_str(&hex8(0)); // rdevminor [86..94]
  header.push_str(&hex8(namesize)); // namesize [94..102]
  header.push_str(&hex8(0)); // check [102..110]
  assert_eq!(header.len(), 110, "newc header must be 110 bytes");

  out.extend_from_slice(header.as_bytes());
  out.extend_from_slice(&name_with_nul);
  // Align header+name to 4 bytes.
  let header_plus_name = 110 + name_with_nul.len();
  let name_pad = (4 - header_plus_name % 4) % 4;
  out.extend(std::iter::repeat_n(0u8, name_pad));
  // Data.
  out.extend_from_slice(data);
  let data_pad = (4 - data.len() % 4) % 4;
  out.extend(std::iter::repeat_n(0u8, data_pad));
}

/// Write the cpio TRAILER!!! end marker.
fn cpio_trailer(out: &mut Vec<u8>) {
  cpio_entry(out, "TRAILER!!!", 0, &[]);
}

const S_IFREG: u32 = 0o100000;
const S_IFLNK: u32 = 0o120000;
const S_IFCHR: u32 = 0o020000;

// covers: PKG-027
#[test]
fn rpm_extract_decompresses_each_payload_codec_and_rejects_unknown() {
  // Build one cpio payload with a single regular file, then compress it with
  // each of the three magics RPM payloads may carry.
  let mut cpio: Vec<u8> = Vec::new();
  cpio_entry(&mut cpio, "./usr/bin/tool", S_IFREG | 0o644, b"tool-bytes");
  cpio_trailer(&mut cpio);

  for (label, payload) in [
    ("xz", compress_xz(&cpio)),
    ("gzip", compress_gzip(&cpio)),
    ("zstd", compress_zstd(&cpio)),
  ] {
    let rpm = rpm_build(&[], &[], &payload);
    let dir = tempfile::tempdir().unwrap();
    let path_file_archive = dir.path().join(format!("pkg-{label}.rpm"));
    std::fs::write(&path_file_archive, &rpm).unwrap();

    let root = tempfile::tempdir().unwrap();
    let files = RpmFormat
      .extract(&path_file_archive, root.path(), &ident("tool"))
      .unwrap_or_else(|e| panic!("{label}: rpm extract must succeed: {e}"));

    assert_eq!(
      std::fs::read(root.path().join("usr/bin/tool")).unwrap(),
      b"tool-bytes",
      "{label}: payload must decompress and extract"
    );
    assert!(files.iter().any(|p| p == &PathBuf::from("usr/bin/tool")), "{label}: payload path reported");
  }

  // An unknown payload magic must bail rather than guess.
  let mut unknown_payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01];
  unknown_payload.extend_from_slice(&cpio); // tail content irrelevant
  let rpm = rpm_build(&[], &[], &unknown_payload);
  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("pkg-unknown.rpm");
  std::fs::write(&path_file_archive, &rpm).unwrap();
  let root = tempfile::tempdir().unwrap();
  let err = RpmFormat
    .extract(&path_file_archive, root.path(), &ident("tool"))
    .err()
    .expect("unknown payload magic must error");
  let msg = format!("{err:#}");
  assert!(msg.contains("Unknown compression format in RPM payload"), "expected unknown-compression bail, got: {msg}");
}

// covers: PKG-028
#[test]
fn rpm_extract_no_payload_offset_errors_with_path() {
  // Bytes too short / lacking the signature magic → payload_offset() is None
  // → payload_decompress errors → extract wraps it with the archive path.
  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("broken.rpm");
  std::fs::write(&path_file_archive, b"not an rpm at all, far too short").unwrap();

  let root = tempfile::tempdir().unwrap();
  let err = RpmFormat
    .extract(&path_file_archive, root.path(), &ident("broken"))
    .err()
    .expect("an RPM with no payload offset must error");
  let msg = format!("{err:#}");
  assert!(
    msg.contains(&format!("No compressed payload found in {}", path_file_archive.display())),
    "expected 'No compressed payload found in <path>', got: {msg}"
  );
}

// covers: PKG-029
#[test]
fn rpm_cpio_extract_newc_files_exec_bit_and_symlink() {
  use std::os::unix::fs::PermissionsExt;

  let mut cpio: Vec<u8> = Vec::new();
  // A non-executable regular file.
  cpio_entry(&mut cpio, "./etc/demo.conf", S_IFREG | 0o644, b"key=value\n");
  // An executable regular file — extraction normalises exec to 0755.
  cpio_entry(&mut cpio, "./usr/bin/demo", S_IFREG | 0o755, b"#!/bin/sh\n");
  // A symlink whose data is the link target.
  cpio_entry(&mut cpio, "./usr/bin/demo-link", S_IFLNK | 0o777, b"demo");
  cpio_trailer(&mut cpio);

  let rpm = rpm_build(&[], &[], &compress_gzip(&cpio));
  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("demo.rpm");
  std::fs::write(&path_file_archive, &rpm).unwrap();

  let root = tempfile::tempdir().unwrap();
  let files = RpmFormat
    .extract(&path_file_archive, root.path(), &ident("demo"))
    .expect("rpm extract must succeed");

  // Regular file content preserved.
  assert_eq!(std::fs::read(root.path().join("etc/demo.conf")).unwrap(), b"key=value\n");

  // Executable file: on-disk mode forced to 0755.
  let mode = std::fs::metadata(root.path().join("usr/bin/demo"))
    .unwrap()
    .permissions()
    .mode();
  assert_eq!(mode & 0o7777, 0o755, "exec bit must map to mode 0755, got {:o}", mode & 0o7777);

  // Symlink: type and target.
  let link = root.path().join("usr/bin/demo-link");
  let meta = std::fs::symlink_metadata(&link).unwrap();
  assert!(meta.file_type().is_symlink(), "demo-link must be a symlink");
  assert_eq!(std::fs::read_link(&link).unwrap(), Path::new("demo"), "symlink target must be 'demo'");

  // The TRAILER!!! and the metadata are excluded; the three real entries are
  // reported.
  for expect in ["etc/demo.conf", "usr/bin/demo", "usr/bin/demo-link"] {
    assert!(
      files.iter().any(|p| p == &PathBuf::from(expect)),
      "{expect} must appear in the reported file list: {:?}",
      files
    );
  }
  assert!(
    !files.iter().any(|p| p.to_string_lossy().contains("TRAILER")),
    "the cpio trailer must never be reported as content"
  );
}

// covers: PKG-030
#[test]
fn rpm_cpio_extract_replaces_empty_dir_with_symlink_for_merged_usr() {
  // An empty directory at a destination that a later symlink targets must be
  // replaced by the symlink (merged-/usr). A non-empty directory at a symlink
  // destination must NOT be clobbered.
  let root = tempfile::tempdir().unwrap();
  // Pre-seed an EMPTY directory at /bin (the classic /bin → usr/bin case).
  std::fs::create_dir_all(root.path().join("bin")).unwrap();
  // Pre-seed a NON-EMPTY directory at /lib (must survive).
  std::fs::create_dir_all(root.path().join("lib")).unwrap();
  std::fs::write(root.path().join("lib/preexisting"), b"keep").unwrap();

  let mut cpio: Vec<u8> = Vec::new();
  // Symlink /bin → usr/bin: the empty /bin dir should be replaced.
  cpio_entry(&mut cpio, "./bin", S_IFLNK | 0o777, b"usr/bin");
  // Symlink /lib → usr/lib: the non-empty /lib dir must be preserved.
  cpio_entry(&mut cpio, "./lib", S_IFLNK | 0o777, b"usr/lib");
  cpio_trailer(&mut cpio);

  let rpm = rpm_build(&[], &[], &compress_gzip(&cpio));
  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("fs.rpm");
  std::fs::write(&path_file_archive, &rpm).unwrap();

  RpmFormat
    .extract(&path_file_archive, root.path(), &ident("filesystem"))
    .expect("rpm extract must succeed");

  // The empty /bin became a symlink to usr/bin.
  let bin_meta = std::fs::symlink_metadata(root.path().join("bin")).unwrap();
  assert!(bin_meta.file_type().is_symlink(), "empty /bin must be replaced by a symlink");
  assert_eq!(std::fs::read_link(root.path().join("bin")).unwrap(), Path::new("usr/bin"));

  // The non-empty /lib stays a directory and keeps its content.
  let lib_meta = std::fs::symlink_metadata(root.path().join("lib")).unwrap();
  assert!(lib_meta.file_type().is_dir(), "non-empty /lib must NOT be clobbered by a symlink");
  assert_eq!(std::fs::read(root.path().join("lib/preexisting")).unwrap(), b"keep");
}

// covers: PKG-031
#[test]
fn rpm_cpio_extract_skips_device_nodes_and_symlink_failure_warns() {
  // Pre-seed a NON-EMPTY directory at the destination a later symlink targets,
  // so the symlink creation fails (EEXIST) — extraction must warn and carry on.
  let root = tempfile::tempdir().unwrap();
  std::fs::create_dir_all(root.path().join("opt/keep")).unwrap();
  std::fs::write(root.path().join("opt/keep/inner"), b"survives").unwrap();

  // A character device node cannot be created unprivileged, so it is silently
  // skipped; the regular files around it — and after the failing symlink —
  // still extract.
  let mut cpio: Vec<u8> = Vec::new();
  cpio_entry(&mut cpio, "./before.txt", S_IFREG | 0o644, b"before");
  cpio_entry(&mut cpio, "./dev/null", S_IFCHR | 0o666, &[]); // device node — skipped
  cpio_entry(&mut cpio, "./opt/keep", S_IFLNK | 0o777, b"somewhere"); // symlink onto a non-empty dir → fails
  cpio_entry(&mut cpio, "./after.txt", S_IFREG | 0o644, b"after");
  cpio_trailer(&mut cpio);

  let rpm = rpm_build(&[], &[], &compress_gzip(&cpio));
  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("dev.rpm");
  std::fs::write(&path_file_archive, &rpm).unwrap();

  let files = RpmFormat
    .extract(&path_file_archive, root.path(), &ident("dev"))
    .expect("rpm extract must succeed despite a device node and a failing symlink");

  // Files on both sides of the skipped/failed entries extracted.
  assert_eq!(std::fs::read(root.path().join("before.txt")).unwrap(), b"before");
  assert_eq!(std::fs::read(root.path().join("after.txt")).unwrap(), b"after");
  // The device node was silently skipped — not created, not reported.
  assert!(!root.path().join("dev/null").exists(), "device node must not be created");
  assert!(!files.iter().any(|p| p.to_string_lossy().contains("null")), "device node must not be reported as content");
  // The symlink onto a non-empty directory failed; the directory and its
  // content are untouched and extraction did not abort.
  let keep_meta = std::fs::symlink_metadata(root.path().join("opt/keep")).unwrap();
  assert!(keep_meta.file_type().is_dir(), "a non-empty directory must survive a failed symlink-replace");
  assert_eq!(std::fs::read(root.path().join("opt/keep/inner")).unwrap(), b"survives");
}

// covers: PKG-032
#[test]
fn rpm_cpio_extract_malformed_hex_header_is_hard_error() {
  // Build a valid newc header, then corrupt the namesize field (bytes
  // [94..102]) with non-hex characters. cpio_extract must bail.
  let mut cpio: Vec<u8> = Vec::new();
  cpio_entry(&mut cpio, "./ok.txt", S_IFREG | 0o644, b"ok");
  cpio_trailer(&mut cpio);

  // Corrupt the first entry's namesize field in place. The first header
  // begins at offset 0; namesize occupies bytes [94..102].
  for (i, b) in b"ZZZZZZZZ".iter().enumerate() {
    cpio[94 + i] = *b;
  }

  let rpm = rpm_build(&[], &[], &compress_gzip(&cpio));
  let dir = tempfile::tempdir().unwrap();
  let path_file_archive = dir.path().join("bad.rpm");
  std::fs::write(&path_file_archive, &rpm).unwrap();

  let root = tempfile::tempdir().unwrap();
  let err = RpmFormat
    .extract(&path_file_archive, root.path(), &ident("bad"))
    .err()
    .expect("a non-hex cpio header field must error");
  let msg = format!("{err:#}");
  assert!(
    msg.contains("cpio namesize field 'ZZZZZZZZ' is not a hex integer"),
    "expected hex-integer bail naming the field and raw value, got: {msg}"
  );
}

// covers: PKG-033
#[test]
fn rpm_scripts_stage_shell_lua_and_empty() {
  // POSTIN is tag 1024; POSTINPROG is tag 1086. Build a main header whose
  // store carries the script text and (optionally) the interpreter, then
  // assert the staged file name.
  let cpio = {
    let mut c: Vec<u8> = Vec::new();
    cpio_entry(&mut c, "./usr/bin/x", S_IFREG | 0o644, b"x");
    cpio_trailer(&mut c);
    c
  };
  let payload = compress_gzip(&cpio);

  // --- shell scriptlet (no POSTINPROG → defaults to shell) → postinst ---
  {
    let script = b"#!/bin/sh\necho installed\n\0"; // NUL-terminated in the store
    let store = script.to_vec();
    // entry: (tag=1024 POSTIN, type=6 STRING, offset=0, count=1)
    let rpm = rpm_build(&[(1024, 6, 0, 1)], &store, &payload);
    let dir = tempfile::tempdir().unwrap();
    let path_file_archive = dir.path().join("shell.rpm");
    std::fs::write(&path_file_archive, &rpm).unwrap();
    let root = tempfile::tempdir().unwrap();
    RpmFormat
      .extract(&path_file_archive, root.path(), &ident("shellpkg"))
      .expect("rpm extract must succeed");
    let staged = root.path().join(".flatroot/scripts/shellpkg:x86_64/postinst");
    assert!(staged.exists(), "a shell POSTIN must be staged as postinst");
    let body = std::fs::read_to_string(&staged).unwrap();
    assert!(body.contains("echo installed"), "staged postinst must carry the script body: {body:?}");
    assert!(
      !root
        .path()
        .join(".flatroot/scripts/shellpkg:x86_64/postinst.lua")
        .exists(),
      "a shell scriptlet must not be staged as .lua"
    );
  }

  // --- Lua scriptlet (POSTINPROG == "<lua>") → postinst.lua ---
  {
    // Store: script at offset 0, interpreter "<lua>" right after.
    let script = b"print('hi')\0";
    let interp = b"<lua>\0";
    let mut store: Vec<u8> = Vec::new();
    store.extend_from_slice(script);
    let interp_offset = store.len() as u32;
    store.extend_from_slice(interp);
    let rpm = rpm_build(&[(1024, 6, 0, 1), (1086, 6, interp_offset, 1)], &store, &payload);
    let dir = tempfile::tempdir().unwrap();
    let path_file_archive = dir.path().join("lua.rpm");
    std::fs::write(&path_file_archive, &rpm).unwrap();
    let root = tempfile::tempdir().unwrap();
    RpmFormat
      .extract(&path_file_archive, root.path(), &ident("luapkg"))
      .expect("rpm extract must succeed");
    assert!(
      root
        .path()
        .join(".flatroot/scripts/luapkg:x86_64/postinst.lua")
        .exists(),
      "a Lua POSTINPROG must stage the scriptlet as postinst.lua"
    );
    assert!(
      !root.path().join(".flatroot/scripts/luapkg:x86_64/postinst").exists(),
      "a Lua scriptlet must not be staged as plain postinst"
    );
  }

  // --- empty scriptlet → nothing written ---
  {
    let script = b"   \n\t \0"; // whitespace only — trimmed empty, not written
    let store = script.to_vec();
    let rpm = rpm_build(&[(1024, 6, 0, 1)], &store, &payload);
    let dir = tempfile::tempdir().unwrap();
    let path_file_archive = dir.path().join("empty.rpm");
    std::fs::write(&path_file_archive, &rpm).unwrap();
    let root = tempfile::tempdir().unwrap();
    RpmFormat
      .extract(&path_file_archive, root.path(), &ident("emptypkg"))
      .expect("rpm extract must succeed");
    let scripts_dir = root.path().join(".flatroot/scripts/emptypkg:x86_64");
    assert!(
      !scripts_dir.join("postinst").exists() && !scripts_dir.join("postinst.lua").exists(),
      "an empty/whitespace scriptlet must write no script file"
    );
  }
}
