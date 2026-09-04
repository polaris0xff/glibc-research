//! Codec dispatch + generic tar extraction, exercised directly against
//! the public `flatroot::internal::{codec, tar}` API. These are the
//! domain-neutral plumbing units underneath every package format: the
//! compression recogniser (`Codec`) and the obstruction-clearing,
//! last-writer-wins archive unpacker (`Tar`). Building fixtures with the
//! `flate2`/`xz2`/`bzip2`/`zstd`/`tar` crates keeps every case offline.

use std::io::{Read, Write};
use std::path::PathBuf;

use flatroot::internal::codec::Codec;
use flatroot::internal::tar::Tar;

// ---------------------------------------------------------------------------
// Compression helpers — produce a compressed buffer for each codec so the
// round-trip can be checked symmetrically.
// ---------------------------------------------------------------------------

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

fn compress_bzip2(plain: &[u8]) -> Vec<u8> {
  let mut enc = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default());
  enc.write_all(plain).unwrap();
  enc.finish().unwrap()
}

/// Read everything out of a `Codec::reader` over `bytes`.
fn reader_to_vec(codec: Codec, bytes: &[u8]) -> Vec<u8> {
  let mut r = codec.reader(bytes).expect("reader must construct");
  let mut out = Vec::new();
  r.read_to_end(&mut out).expect("reader must decode");
  out
}

/// Stable name for a `Codec` variant. The enum derives nothing, so the
/// only way to assert which variant `from_magic` chose is to match it.
fn codec_name(codec: &Codec) -> &'static str {
  match codec {
    Codec::Gzip => "gzip",
    Codec::Xz => "xz",
    Codec::Zstd => "zstd",
    Codec::Bzip2 => "bzip2",
    Codec::None => "none",
  }
}

// ---------------------------------------------------------------------------
// PKG-059 — every codec decodes consistently through reader() and bytes(),
// with None first-class (the original bytes come straight back).
// ---------------------------------------------------------------------------

// covers: PKG-059
#[test]
fn codec_reader_and_bytes_round_trip_all_five() {
  // A payload long enough that real compression actually happens and the
  // streaming and eager paths both have something to chew through.
  let plain: Vec<u8> = (0..4096u32).flat_map(|i| (i as u8).to_le_bytes()).collect();

  let cases: Vec<(&str, Vec<u8>)> = vec![
    ("gzip", compress_gzip(&plain)),
    ("xz", compress_xz(&plain)),
    ("zstd", compress_zstd(&plain)),
    ("bzip2", compress_bzip2(&plain)),
    ("none", plain.clone()),
  ];

  for (label, compressed) in cases {
    let codec_reader = match label {
      "gzip" => Codec::Gzip,
      "xz" => Codec::Xz,
      "zstd" => Codec::Zstd,
      "bzip2" => Codec::Bzip2,
      "none" => Codec::None,
      _ => unreachable!(),
    };
    let codec_bytes = match label {
      "gzip" => Codec::Gzip,
      "xz" => Codec::Xz,
      "zstd" => Codec::Zstd,
      "bzip2" => Codec::Bzip2,
      "none" => Codec::None,
      _ => unreachable!(),
    };

    let via_reader = reader_to_vec(codec_reader, &compressed);
    let via_bytes = codec_bytes.bytes(&compressed).expect("bytes() must decode");

    assert_eq!(via_reader, plain, "{label}: reader() must recover the original buffer");
    assert_eq!(via_bytes, plain, "{label}: bytes() must recover the original buffer");
    assert_eq!(via_reader, via_bytes, "{label}: reader() and bytes() must agree");
  }

  // None is first-class: an already-plain stream is handed back verbatim,
  // not run through any decoder.
  assert_eq!(Codec::None.bytes(&plain).unwrap(), plain, "None must be a passthrough");
}

// ---------------------------------------------------------------------------
// PKG-060 — from_magic recognises only zstd; everything else is assumed
// gzip. A non-gzip/non-zstd buffer is therefore mis-classified as gzip and
// a subsequent decode fails.
// ---------------------------------------------------------------------------

// covers: PKG-060
#[test]
fn codec_from_magic_recognises_zstd_else_assumes_gzip() {
  let plain = b"the quick brown fox jumps over the lazy dog".repeat(64);

  // A genuine zstd buffer is recognised by its 4-byte magic.
  let zstd_bytes = compress_zstd(&plain);
  let detected_zstd = Codec::from_magic(&zstd_bytes).expect("from_magic must not error");
  assert_eq!(codec_name(&detected_zstd), "zstd", "zstd magic 28 b5 2f fd must classify as Zstd");
  // And it actually decodes through the detected codec.
  assert_eq!(detected_zstd.bytes(&zstd_bytes).unwrap(), plain);

  // A genuine gzip buffer lacks the zstd magic, so it is taken as the only
  // other published form: gzip.
  let gzip_bytes = compress_gzip(&plain);
  let detected_gzip = Codec::from_magic(&gzip_bytes).expect("from_magic must not error");
  assert_eq!(codec_name(&detected_gzip), "gzip", "non-zstd magic must default to Gzip");
  assert_eq!(detected_gzip.bytes(&gzip_bytes).unwrap(), plain);

  // A buffer that is neither gzip nor zstd is still classified as gzip
  // (no suffix, no other magic recognised), and decoding it as gzip fails
  // — the deliberate ambiguity documented in from_magic.
  let garbage = vec![0x00u8, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
  let detected_garbage = Codec::from_magic(&garbage).expect("from_magic itself never errors");
  assert_eq!(codec_name(&detected_garbage), "gzip", "unknown magic must be assumed gzip");
  assert!(
    detected_garbage.bytes(&garbage).is_err(),
    "decoding non-gzip bytes through the assumed-gzip codec must error"
  );
}

// ---------------------------------------------------------------------------
// Tar fixture helpers.
// ---------------------------------------------------------------------------

/// Build an uncompressed tar carrying a sequence of regular-file entries
/// in the given order (later entries appear later in the stream).
fn tar_with_regular_files(entries: &[(&str, &[u8])]) -> Vec<u8> {
  let mut buf: Vec<u8> = Vec::new();
  {
    let mut builder = tar::Builder::new(&mut buf);
    for (path, content) in entries {
      let mut header = tar::Header::new_gnu();
      header.set_size(content.len() as u64);
      header.set_mode(0o644);
      header.set_entry_type(tar::EntryType::Regular);
      header.set_cksum();
      builder.append_data(&mut header, path, *content).unwrap();
    }
    builder.finish().unwrap();
  }
  buf
}

/// Wrap a buffer with the codec the member name implies, so the resulting
/// blob is what `extract_compressed` would be handed.
fn compress_for_suffix(name: &str, plain: &[u8]) -> Vec<u8> {
  if name.ends_with(".gz") {
    compress_gzip(plain)
  } else if name.ends_with(".xz") {
    compress_xz(plain)
  } else if name.ends_with(".zst") {
    compress_zstd(plain)
  } else if name.ends_with(".bz2") {
    compress_bzip2(plain)
  } else {
    plain.to_vec()
  }
}

// ---------------------------------------------------------------------------
// PKG-057 — Tar::extract: later entries win over earlier ones, an
// obstructing destination is cleared, and a corrupt entry is warned about
// and stepped over without aborting the rest of the extraction.
// ---------------------------------------------------------------------------

// covers: PKG-057
#[test]
fn tar_extract_last_writer_wins_and_survives_corrupt_entry() {
  let root = tempfile::tempdir().unwrap();

  // Two entries at the same path: the later content must win on disk.
  let tar_bytes = tar_with_regular_files(&[
    ("opt/app/data.txt", b"first-writer"),
    ("opt/app/keep.txt", b"untouched"),
    ("opt/app/data.txt", b"second-writer-wins"),
  ]);

  let _captured: Vec<PathBuf> = Tar::extract(&tar_bytes[..], root.path()).expect("extract must succeed");

  let on_disk = std::fs::read(root.path().join("opt/app/data.txt")).expect("data.txt must exist");
  assert_eq!(on_disk, b"second-writer-wins", "the last entry at a duplicate path must win");
  let keep = std::fs::read(root.path().join("opt/app/keep.txt")).expect("keep.txt must exist");
  assert_eq!(keep, b"untouched");

  // An obstructing plain file at a destination a later entry targets is
  // cleared before the unpack.
  let root2 = tempfile::tempdir().unwrap();
  std::fs::create_dir_all(root2.path().join("opt/app")).unwrap();
  std::fs::write(root2.path().join("opt/app/data.txt"), b"stale-obstruction").unwrap();
  let tar2 = tar_with_regular_files(&[("opt/app/data.txt", b"fresh-content")]);
  Tar::extract(&tar2[..], root2.path()).expect("extract over obstruction must succeed");
  assert_eq!(
    std::fs::read(root2.path().join("opt/app/data.txt")).unwrap(),
    b"fresh-content",
    "an obstructing file must be cleared and overwritten"
  );

  // A stream whose good entries are followed by a malformed/truncated region:
  // extract must place the good entries and then warn-and-step-over the bad
  // tail rather than aborting (it always returns Ok). We build two real
  // entries, drop the end-of-archive zero blocks, and append a partial header
  // so the trailing bytes cannot parse as a complete entry.
  let root3 = tempfile::tempdir().unwrap();
  let mut good = tar_with_regular_files(&[("first.txt", b"one"), ("second.txt", b"two")]);
  // A finished tar ends in two 512-byte zero blocks; lop them off and tack on
  // a short, non-zero, sub-block run that cannot form a valid 512-byte header.
  if good.len() >= 1024 {
    good.truncate(good.len() - 1024);
  }
  good.extend_from_slice(&[0x42u8; 200]); // partial, malformed trailing header
  // extract must return Ok (never aborts on a bad entry) and the two good
  // entries that preceded the corruption must be on disk.
  Tar::extract(&good[..], root3.path()).expect("extract must not abort on a malformed trailing entry");
  assert!(root3.path().join("first.txt").exists(), "the first good entry must extract");
  assert_eq!(std::fs::read(root3.path().join("first.txt")).unwrap(), b"one");
  assert!(root3.path().join("second.txt").exists(), "the second good entry must extract before the corrupt tail");
}

// ---------------------------------------------------------------------------
// PKG-058 — Tar::extract_compressed infers the codec from the member name
// and extracts identically across gz/xz/zst/bz2/none.
// ---------------------------------------------------------------------------

// covers: PKG-058
#[test]
fn tar_extract_compressed_infers_codec_from_member_name() {
  let inner = tar_with_regular_files(&[
    ("usr/bin/tool", b"#!/bin/sh\necho hi\n"),
    ("usr/share/doc/readme", b"docs"),
  ]);

  for member in ["data.tar.gz", "data.tar.xz", "data.tar.zst", "data.tar.bz2", "data.tar"] {
    let blob = compress_for_suffix(member, &inner);
    let root = tempfile::tempdir().unwrap();
    let captured: Vec<PathBuf> = Tar::extract_compressed(member, &blob, root.path())
      .unwrap_or_else(|e| panic!("{member}: extract_compressed must succeed: {e}"));

    let tool = std::fs::read(root.path().join("usr/bin/tool"))
      .unwrap_or_else(|_| panic!("{member}: usr/bin/tool must be extracted"));
    assert_eq!(tool, b"#!/bin/sh\necho hi\n", "{member}: content must round-trip identically");
    assert!(root.path().join("usr/share/doc/readme").exists(), "{member}: every member must be extracted");
    assert!(
      captured.iter().any(|p| p == &PathBuf::from("usr/bin/tool")),
      "{member}: captured paths must record the extracted file, got {:?}",
      captured
    );
  }
}
