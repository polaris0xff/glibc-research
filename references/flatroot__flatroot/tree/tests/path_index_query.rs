//! Integration tests for the binary file→package path index
//! (`flatroot::path_index`). These exercise the public read/write API
//! directly with explicit on-disk paths — `PathIndexBuilder` (entry_push,
//! ingest, finalize) and `PathIndex` (open, of_db, query, query_glob) — plus
//! the format-validation bails the reader raises on a malformed record.
//!
//! None of these tests touch the `Cache`/`FLATROOT_CACHE_HOME` machinery: the
//! path index API takes explicit `&Path`s, so each test builds its own index
//! in a `TempDir`. No network, no process-env mutation.

#![allow(dead_code)]

use std::io::Write;

use flatroot::path_index::{PathIndex, PathIndexBuilder};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a real path index from `entry_push` facts and return its path.
fn build_index(entries: &[(&str, &str, &str)]) -> (TempDir, std::path::PathBuf) {
  let tmp = TempDir::new().unwrap();
  let path = tmp.path().join("test.pathidx");
  let mut builder = PathIndexBuilder::new();
  for (dir, fname, pkg) in entries {
    builder.entry_push(dir, fname, pkg);
  }
  builder.finalize(&path).unwrap();
  (tmp, path)
}

/// Hand-craft a path index byte image so a single field can be corrupted to
/// exercise the reader's validation. `dirs`/`fnames`/`pkgs` are written into
/// the three string tables verbatim; `entries` are `(dir_id, fname_id, pkg_id)`
/// triples written into the fixed-width entries array. The four counts in the
/// header are taken from the slice lengths so a well-formed call round-trips,
/// while a malformed call overrides them.
struct RawImage {
  version: u32,
  dir_count: u32,
  fname_count: u32,
  pkg_count: u32,
  entries_count: u32,
  dirs: Vec<String>,
  fnames: Vec<String>,
  pkgs: Vec<String>,
  entries: Vec<(u32, u32, u32)>,
  truncate_to: Option<usize>,
}

impl RawImage {
  fn well_formed(dirs: &[&str], fnames: &[&str], pkgs: &[&str], entries: &[(u32, u32, u32)]) -> Self {
    Self {
      version: 1,
      dir_count: dirs.len() as u32,
      fname_count: fnames.len() as u32,
      pkg_count: pkgs.len() as u32,
      entries_count: entries.len() as u32,
      dirs: dirs.iter().map(|s| s.to_string()).collect(),
      fnames: fnames.iter().map(|s| s.to_string()).collect(),
      pkgs: pkgs.iter().map(|s| s.to_string()).collect(),
      entries: entries.to_vec(),
      truncate_to: None,
    }
  }

  fn bytes(&self) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"FRPI");
    out.extend_from_slice(&self.version.to_le_bytes());
    out.extend_from_slice(&self.dir_count.to_le_bytes());
    out.extend_from_slice(&self.fname_count.to_le_bytes());
    out.extend_from_slice(&self.pkg_count.to_le_bytes());
    out.extend_from_slice(&self.entries_count.to_le_bytes());
    for table in [&self.dirs, &self.fnames, &self.pkgs] {
      for s in table {
        out.extend_from_slice(&(s.len() as u16).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
      }
    }
    for (d, f, p) in &self.entries {
      out.extend_from_slice(&d.to_le_bytes());
      out.extend_from_slice(&f.to_le_bytes());
      out.extend_from_slice(&p.to_le_bytes());
    }
    if let Some(n) = self.truncate_to {
      out.truncate(n);
    }
    out
  }

  /// Write the image to a fresh `.pathidx` file and return its path.
  fn write(&self) -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("crafted.pathidx");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(&self.bytes()).unwrap();
    f.flush().unwrap();
    (tmp, path)
  }
}

// ---------------------------------------------------------------------------
// query() — single-path → owning package
// ---------------------------------------------------------------------------

// covers: IDX-046
#[test]
fn query_resolves_known_path_and_unknown_returns_none() {
  // Round-trip: a file present in the index resolves to its owning package;
  // a path whose dir/filename is absent returns Ok(None) (not an error).
  let (_tmp, path) = build_index(&[
    ("/usr/lib/x86_64-linux-gnu", "libssl.so.3", "libssl3"),
    ("/usr/lib/x86_64-linux-gnu", "libcrypto.so.3", "libssl3"),
    ("/usr/bin", "bash", "bash"),
  ]);
  let idx = PathIndex::open(&path).unwrap().unwrap();

  // Known file → owning package.
  assert_eq!(idx.query("/usr/lib/x86_64-linux-gnu/libssl.so.3").unwrap(), Some("libssl3".to_string()));
  assert_eq!(idx.query("/usr/bin/bash").unwrap(), Some("bash".to_string()));

  // Unknown filename in a known dir → None.
  assert_eq!(idx.query("/usr/bin/no-such-binary").unwrap(), None);
  // Unknown dir → None.
  assert_eq!(idx.query("/opt/nowhere/libssl.so.3").unwrap(), None);
}

// covers: IDX-047
#[test]
fn query_path_without_slash_returns_none() {
  // file_path.rfind('/') == None → Ok(None) before any table lookup.
  let (_tmp, path) = build_index(&[("/usr/bin", "bash", "bash")]);
  let idx = PathIndex::open(&path).unwrap().unwrap();
  assert_eq!(idx.query("bash").unwrap(), None);
  assert_eq!(idx.query("noslashhere").unwrap(), None);
}

// ---------------------------------------------------------------------------
// query_glob() — basename glob match, union/sort/dedup
// ---------------------------------------------------------------------------

// covers: IDX-048
#[test]
fn query_glob_basename_union_sort_and_empty() {
  let (_tmp, path) = build_index(&[
    ("/usr/lib", "libssl.so.3", "libssl3"),
    ("/usr/lib", "libcrypto.so.3", "libssl3"),
    ("/usr/lib", "libfoo.so.1", "foopkg"),
    ("/usr/bin", "bash", "bash"),
  ]);
  let idx = PathIndex::open(&path).unwrap().unwrap();

  // Multi-match union, sorted by (filename, package).
  let result = idx.query_glob("lib*.so.*").unwrap();
  assert_eq!(
    result,
    vec![
      ("libcrypto.so.3".to_string(), "libssl3".to_string()),
      ("libfoo.so.1".to_string(), "foopkg".to_string()),
      ("libssl.so.3".to_string(), "libssl3".to_string()),
    ]
  );

  // No filename matches → empty Vec (not an error).
  assert!(idx.query_glob("nothere*").unwrap().is_empty());
}

// covers: IDX-049
#[test]
fn query_glob_question_mark_matches_exactly_one_char() {
  let (_tmp, path) = build_index(&[
    ("/usr/lib", "libssl.so.1", "libssl1"),
    ("/usr/lib", "libssl.so.3", "libssl3"),
    ("/usr/lib", "libssl.so.10", "libssl10"),
  ]);
  let idx = PathIndex::open(&path).unwrap().unwrap();
  let result = idx.query_glob("libssl.so.?").unwrap();
  // `?` is exactly one char: .1 and .3 match, .10 is rejected.
  assert_eq!(
    result,
    vec![
      ("libssl.so.1".to_string(), "libssl1".to_string()),
      ("libssl.so.3".to_string(), "libssl3".to_string()),
    ]
  );
}

// covers: IDX-050
#[test]
fn query_glob_dedups_same_filename_pkg_across_dirs() {
  // Same (filename, package) shipped under two different directories must
  // collapse to one pair after the query path's sort+dedup.
  let (_tmp, path) = build_index(&[
    ("/usr/lib", "libfoo.so.1", "foopkg"),
    ("/usr/lib32", "libfoo.so.1", "foopkg"),
  ]);
  let idx = PathIndex::open(&path).unwrap().unwrap();
  let result = idx.query_glob("libfoo*").unwrap();
  assert_eq!(result, vec![("libfoo.so.1".to_string(), "foopkg".to_string())]);
}

// covers: IDX-051
#[test]
fn query_glob_invalid_pattern_errors() {
  // An unparseable glob ([ with no close) surfaces "invalid glob pattern: <p>".
  let (_tmp, path) = build_index(&[("/usr/lib", "libfoo.so.1", "foopkg")]);
  let idx = PathIndex::open(&path).unwrap().unwrap();
  let bad = "lib[";
  let err = idx.query_glob(bad).err().expect("expected an error");
  let msg = format!("{:#}", err);
  assert!(msg.contains(&format!("invalid glob pattern: {}", bad)), "expected invalid-glob message, got: {}", msg);
}

// ---------------------------------------------------------------------------
// open() process cache — parse once per file path
// ---------------------------------------------------------------------------

// covers: IDX-052
#[test]
fn open_is_process_cached_returns_same_arc() {
  // Two opens of the same path hand back the same cached Arc (parsed once).
  let (_tmp, path) = build_index(&[("/usr/lib", "libfoo.so.1", "foopkg")]);
  let a = PathIndex::open(&path).unwrap().unwrap();
  let b = PathIndex::open(&path).unwrap().unwrap();
  assert!(
    std::sync::Arc::ptr_eq(&a, &b),
    "PathIndex::open must return the same cached Arc on a repeat open of one path"
  );
  // Both views answer identically.
  assert_eq!(a.query("/usr/lib/libfoo.so.1").unwrap(), Some("foopkg".to_string()));
  assert_eq!(b.query("/usr/lib/libfoo.so.1").unwrap(), Some("foopkg".to_string()));
}

// covers: IDX-052
#[test]
fn open_missing_index_returns_none() {
  // A path that does not exist → Ok(None); the absence is not an error.
  let tmp = TempDir::new().unwrap();
  let opened = PathIndex::open(&tmp.path().join("does-not-exist.pathidx")).unwrap();
  assert!(opened.is_none());
}

// ---------------------------------------------------------------------------
// of_db() — sibling .pathidx derived from the .db path
// ---------------------------------------------------------------------------

// covers: IDX-058
#[test]
fn of_db_derives_sibling_pathidx_extension() {
  // of_db replaces the .db extension with .pathidx (fixed companion).
  let companion = PathIndex::of_db(std::path::Path::new("/cache/index/debian-bookworm-x86_64.db"));
  assert_eq!(companion, std::path::PathBuf::from("/cache/index/debian-bookworm-x86_64.pathidx"));
  // A path with no extension still gains .pathidx.
  let companion2 = PathIndex::of_db(std::path::Path::new("/cache/index/foo"));
  assert_eq!(companion2, std::path::PathBuf::from("/cache/index/foo.pathidx"));
}

// covers: IDX-058
#[test]
fn of_db_round_trips_through_a_real_index_file() {
  // Build an index at the companion path of_db derives, then open via that
  // derived path — proving the sibling derivation is the path readers consult.
  let tmp = TempDir::new().unwrap();
  let path_file_db = tmp.path().join("debian-bookworm-x86_64.db");
  let path_file_idx = PathIndex::of_db(&path_file_db);
  assert_eq!(path_file_idx, tmp.path().join("debian-bookworm-x86_64.pathidx"));

  let mut builder = PathIndexBuilder::new();
  builder.entry_push("/usr/lib", "libssl.so.3", "libssl3");
  builder.finalize(&path_file_idx).unwrap();

  let idx = PathIndex::open(&path_file_idx).unwrap().unwrap();
  assert_eq!(idx.query("/usr/lib/libssl.so.3").unwrap(), Some("libssl3".to_string()));
}

// ---------------------------------------------------------------------------
// builder finalize — string pooling, id remap, sorted+deduped entries
// ---------------------------------------------------------------------------

// covers: IDX-059
#[test]
fn builder_pools_strings_remaps_ids_and_dedups_entries() {
  // Feed duplicate (dir, fname, pkg) facts and pkg/dir names in a non-sorted
  // order. After finalize the same fact must appear exactly once, and lookups
  // must work despite the remap from insertion order to sorted order.
  let (_tmp, path) = build_index(&[
    // zzz before aaa on purpose; pools sort alphabetically at finalize.
    ("/zzz", "zlib.so.1", "zpkg"),
    ("/aaa", "alib.so.1", "apkg"),
    // exact duplicate fact — must dedup to a single triple.
    ("/aaa", "alib.so.1", "apkg"),
    // same package owning two files under two dirs.
    ("/aaa", "blib.so.1", "apkg"),
  ]);
  let idx = PathIndex::open(&path).unwrap().unwrap();

  // Every distinct fact resolves correctly after the remap.
  assert_eq!(idx.query("/zzz/zlib.so.1").unwrap(), Some("zpkg".to_string()));
  assert_eq!(idx.query("/aaa/alib.so.1").unwrap(), Some("apkg".to_string()));
  assert_eq!(idx.query("/aaa/blib.so.1").unwrap(), Some("apkg".to_string()));

  // The duplicate (aaa, alib.so.1, apkg) fact collapsed: a glob over all the
  // *.so.1 filenames yields each (filename, package) pair exactly once.
  let mut all = idx.query_glob("*.so.1").unwrap();
  all.sort();
  assert_eq!(
    all,
    vec![
      ("alib.so.1".to_string(), "apkg".to_string()),
      ("blib.so.1".to_string(), "apkg".to_string()),
      ("zlib.so.1".to_string(), "zpkg".to_string()),
    ],
    "duplicate fact must dedup and every pool string must remap correctly"
  );
}

// ---------------------------------------------------------------------------
// ingest() — RPM filelists.xml folds packages + files; malformed bails
// ---------------------------------------------------------------------------

// covers: IDX-060
#[test]
fn ingest_xml_filelists_folds_packages_and_files() {
  // A filelists-style XML: each <package> groups <file> entries; the nested
  // file path is split at its last '/' into (dir, filename). The result is a
  // queryable index attributing each file to its owning package.
  let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<filelists>
  <package name="openssl-libs" arch="x86_64">
    <version epoch="1" ver="3.0.7" rel="1.fc42"/>
    <file>/usr/lib64/libssl.so.3</file>
    <file>/usr/lib64/libcrypto.so.3</file>
  </package>
  <package name="bash" arch="x86_64">
    <version epoch="0" ver="5.2.15" rel="1.fc42"/>
    <file>/usr/bin/bash</file>
  </package>
</filelists>"#;

  let tmp = TempDir::new().unwrap();
  let path = tmp.path().join("filelists.pathidx");
  let mut builder = PathIndexBuilder::new();
  builder.ingest(std::io::Cursor::new(xml.as_bytes())).unwrap();
  builder.finalize(&path).unwrap();

  let idx = PathIndex::open(&path).unwrap().unwrap();
  // Files split at the last '/' and attributed to the owning <package>.
  assert_eq!(idx.query("/usr/lib64/libssl.so.3").unwrap(), Some("openssl-libs".to_string()));
  assert_eq!(idx.query("/usr/lib64/libcrypto.so.3").unwrap(), Some("openssl-libs".to_string()));
  assert_eq!(idx.query("/usr/bin/bash").unwrap(), Some("bash".to_string()));
}

// covers: IDX-060
#[test]
fn ingest_malformed_xml_bails() {
  // An unclosed tag makes quick-xml error; ingest surfaces a parse error.
  let bad = r#"<filelists><package name="x"><file>/usr/lib/x.so.1</package>"#;
  let mut builder = PathIndexBuilder::new();
  let err = builder
    .ingest(std::io::Cursor::new(bad.as_bytes()))
    .err()
    .expect("expected an error");
  let msg = format!("{:#}", err);
  assert!(
    msg.contains("XML parse error in filelists.xml"),
    "expected the filelists XML parse-error message, got: {}",
    msg
  );
}

// ---------------------------------------------------------------------------
// parse() validation — magic / version / size / truncation / bad pkg id
// ---------------------------------------------------------------------------

// covers: IDX-055
#[test]
fn parse_rejects_file_smaller_than_header() {
  // Fewer than HEADER_SIZE (24) bytes → "Path index too small: <n> bytes".
  let tmp = TempDir::new().unwrap();
  let path = tmp.path().join("tiny.pathidx");
  std::fs::write(&path, b"FRPI\x01\x00\x00\x00").unwrap(); // 8 bytes
  let err = PathIndex::open(&path).err().expect("expected an error");
  let msg = format!("{:#}", err);
  assert!(msg.contains("Path index too small: 8 bytes"), "got: {}", msg);
}

// covers: IDX-053
#[test]
fn parse_rejects_bad_magic() {
  // Valid size (>=24) but wrong magic → "Invalid path index magic: <bytes>".
  let mut bytes = vec![0u8; 24];
  bytes[0] = 0xDE;
  bytes[1] = 0xAD;
  bytes[2] = 0xBE;
  bytes[3] = 0xEF;
  let tmp = TempDir::new().unwrap();
  let path = tmp.path().join("badmagic.pathidx");
  std::fs::write(&path, &bytes).unwrap();
  let err = PathIndex::open(&path).err().expect("expected an error");
  let msg = format!("{:#}", err);
  // bail!("Invalid path index magic: {:?}", &data[0..4]) → Debug of &[u8].
  assert!(msg.contains(&format!("Invalid path index magic: {:?}", &[0xDEu8, 0xAD, 0xBE, 0xEF][..])), "got: {}", msg);
}

// covers: IDX-054
#[test]
fn parse_rejects_unsupported_version() {
  // Correct magic and size but version != 1 → "Unsupported path index version: <v>".
  let img = RawImage {
    version: 99,
    ..RawImage::well_formed(&[], &[], &[], &[])
  };
  let (_tmp, path) = img.write();
  let err = PathIndex::open(&path).err().expect("expected an error");
  let msg = format!("{:#}", err);
  assert!(msg.contains("Unsupported path index version: 99"), "got: {}", msg);
}

// covers: IDX-056
#[test]
fn parse_rejects_truncated_string_table() {
  // Header promises one dir string but the file ends before that table's
  // length word can be read → "Truncated string table".
  let mut img = RawImage::well_formed(&["/usr/lib"], &[], &[], &[]);
  // Cut the file off right after the 24-byte header, before the dir table.
  img.truncate_to = Some(24);
  let (_tmp, path) = img.write();
  let err = PathIndex::open(&path).err().expect("expected an error");
  let msg = format!("{:#}", err);
  assert!(msg.contains("Truncated string table"), "got: {}", msg);
}

// covers: IDX-056
#[test]
fn parse_rejects_truncated_entries() {
  // Header promises one entry (12 bytes) but the file ends after the string
  // tables → "Truncated path index entries".
  let mut img = RawImage::well_formed(&["/usr/lib"], &["libfoo.so.1"], &["foopkg"], &[(0, 0, 0)]);
  // Header (24) + dir table (2 + 8) + fname table (2 + 11) + pkg table (2 + 6)
  // = 24 + 10 + 13 + 8 = 55. The single entry needs 12 more bytes; cut there.
  img.truncate_to = Some(55);
  let (_tmp, path) = img.write();
  let err = PathIndex::open(&path).err().expect("expected an error");
  let msg = format!("{:#}", err);
  assert!(msg.contains("Truncated path index entries"), "got: {}", msg);
}

// covers: IDX-057
#[test]
fn lookup_rejects_out_of_range_package_id() {
  // An entry references pkg_id 5 while only 1 package exists. Both the
  // single-path query and the glob walk must bail with the same message.
  let img = RawImage::well_formed(
    &["/usr/lib"],
    &["libfoo.so.1"],
    &["foopkg"],
    &[(0, 0, 5)], // pkg_id 5 is out of range (pkg_count == 1)
  );
  let (_tmp, path) = img.write();
  let idx = PathIndex::open(&path).unwrap().unwrap();

  // query() reaches the entry via binary search and dereferences the bad id.
  let err_q = idx.query("/usr/lib/libfoo.so.1").err().expect("expected an error");
  assert!(format!("{:#}", err_q).contains("Invalid package ID 5 in path index entry"), "query: got: {:#}", err_q);

  // query_glob() walks the entries array and dereferences the bad id.
  let err_g = idx.query_glob("libfoo*").err().expect("expected an error");
  assert!(format!("{:#}", err_g).contains("Invalid package ID 5 in path index entry"), "query_glob: got: {:#}", err_g);
}
