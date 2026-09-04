//! The path index: which package ships which file, compacted into a
//! searchable on-disk form and read back, so any installed file can be
//! traced to its owning package.

use std::collections::HashMap;
use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result, bail};

// ---------------------------------------------------------------------------
// Build: stream-parse source format → write binary index
// ---------------------------------------------------------------------------

/// The write side: accumulates file-ownership rows across a release's
/// repositories, interning recurring strings, and writes the result as
/// one compact on-disk file.
pub struct PathIndexBuilder {
  /// Directory string pool, interned once per distinct directory.
  dirs: HashMap<String, u32>,
  /// Filename string pool, the basename half of each path.
  filenames: HashMap<String, u32>,
  /// Package-name pool; each entry references a package by its id here.
  pkgs: HashMap<String, u32>,
  /// Rows as (dir, filename, package) insertion-order ids, remapped to
  /// sorted positions and deduplicated at finalize.
  entries: Vec<(u32, u32, u32)>,
}

impl PathIndexBuilder {
  /// An empty builder, filled from each repository's listing before
  /// `finalize`.
  pub fn new() -> Self {
    Self {
      dirs: HashMap::new(),
      filenames: HashMap::new(),
      pkgs: HashMap::new(),
      entries: Vec::new(),
    }
  }

  /// Records one row — a package ships one file in one directory —
  /// interning each string so recurring directories and names are
  /// stored once.
  pub fn entry_push(&mut self, dir: &str, filename: &str, pkg: &str) {
    let dir_id = Self::pool_intern(&mut self.dirs, dir);
    let fname_id = Self::pool_intern(&mut self.filenames, filename);
    let pkg_id = Self::pool_intern(&mut self.pkgs, pkg);
    self.entries.push((dir_id, fname_id, pkg_id));
  }

  /// Reads a whole repository's `filelists.xml`, recording each
  /// package's files — the document-shaped counterpart to `entry_push`.
  pub fn ingest(&mut self, reader: impl BufRead) -> Result<()> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut xml_reader = Reader::from_reader(reader);
    let mut xml_buf = Vec::new();
    let mut current_pkg_id: u32 = 0;
    let mut in_package = false;
    let mut in_file = false;
    let mut file_text = String::new();

    loop {
      match xml_reader.read_event_into(&mut xml_buf) {
        Ok(Event::Start(ref e)) => {
          let local = e.local_name();
          if local.as_ref() == b"package" {
            if let Some(pkg_id) = self.package_intern(e) {
              current_pkg_id = pkg_id;
            }
            in_package = true;
          } else if in_package && local.as_ref() == b"file" {
            in_file = true;
            file_text.clear();
          }
        }
        Ok(Event::Text(ref e)) if in_file => {
          if let Ok(text) = e.unescape() {
            file_text.push_str(&text);
          }
        }
        Ok(Event::End(ref e)) => {
          let local = e.local_name();
          if local.as_ref() == b"file" && in_file {
            self.file_record(&file_text, current_pkg_id);
            in_file = false;
          } else if local.as_ref() == b"package" {
            in_package = false;
          }
        }
        Ok(Event::Eof) => break,
        Err(e) => bail!("XML parse error in filelists.xml: {}", e),
        _ => {}
      }
      xml_buf.clear();
    }

    Ok(())
  }

  /// A `<package name=..>` opening tag: intern the package's name and yield
  /// the pool id every `<file>` inside the element is recorded under. A tag
  /// without a name attribute names no package and changes nothing.
  fn package_intern(&mut self, e: &quick_xml::events::BytesStart) -> Option<u32> {
    let attr_name = e
      .attributes()
      .filter_map(|a| a.ok())
      .find(|a| a.key.local_name().as_ref() == b"name")?;
    let name_pkg = String::from_utf8_lossy(&attr_name.value).to_string();
    Some(Self::pool_intern(&mut self.pkgs, &name_pkg))
  }

  /// One closed `<file>` element: splits its path at the last `/` into
  /// directory and filename and records the row. Text without a
  /// separator contributes nothing.
  fn file_record(&mut self, file_text: &str, pkg_id: u32) {
    if file_text.is_empty() {
      return;
    }
    let Some(pos) = file_text.rfind('/') else {
      return;
    };
    let dir_id = Self::pool_intern(&mut self.dirs, &file_text[..pos]);
    let fname_id = Self::pool_intern(&mut self.filenames, &file_text[pos + 1..]);
    self.entries.push((dir_id, fname_id, pkg_id));
  }

  /// Intern one piece of repeated text in its pool: the existing id when the
  /// pool has seen the text before, a freshly assigned insertion-order id
  /// otherwise.
  fn pool_intern(pool: &mut HashMap<String, u32>, text: &str) -> u32 {
    let next_id = pool.len() as u32;
    *pool.entry(text.to_string()).or_insert(next_id)
  }

  /// Writes everything gathered into the compact on-disk form: sorts
  /// the string pools, remaps and sorts the rows for binary search,
  /// drops duplicates, and writes the file.
  pub fn finalize(self, path: &Path) -> Result<()> {
    let mut entries = self.entries;

    // Sort string pools alphabetically and build remapping tables
    let dir_remap = Self::pool_remap(&self.dirs);
    let fname_remap = Self::pool_remap(&self.filenames);
    let pkg_remap = Self::pool_remap(&self.pkgs);

    // Remap entry IDs from insertion order to sorted order
    for entry in &mut entries {
      entry.0 = dir_remap[entry.0 as usize];
      entry.1 = fname_remap[entry.1 as usize];
      entry.2 = pkg_remap[entry.2 as usize];
    }

    // Sort entries by (dir_id, filename_id, pkg_id) for binary search and dedup
    entries.sort_unstable();
    entries.dedup();

    // Build sorted string lists
    let dir_strings = Self::pool_strings(&self.dirs, &dir_remap);
    let fname_strings = Self::pool_strings(&self.filenames, &fname_remap);
    let pkg_strings = Self::pool_strings(&self.pkgs, &pkg_remap);
    // Write binary file
    let file =
      std::fs::File::create(path).with_context(|| format!("Failed to create path index at {}", path.display()))?;
    let mut w = BufWriter::new(file);

    // Header
    w.write_all(&PathIndex::MAGIC)?;
    w.write_all(&PathIndex::VERSION.to_le_bytes())?;
    w.write_all(&(dir_strings.len() as u32).to_le_bytes())?;
    w.write_all(&(fname_strings.len() as u32).to_le_bytes())?;
    w.write_all(&(pkg_strings.len() as u32).to_le_bytes())?;
    w.write_all(&(entries.len() as u32).to_le_bytes())?;

    // String tables
    Self::string_table_write(&mut w, &dir_strings)?;
    Self::string_table_write(&mut w, &fname_strings)?;
    Self::string_table_write(&mut w, &pkg_strings)?;

    // Entries array
    for (dir_id, fname_id, pkg_id) in &entries {
      w.write_all(&dir_id.to_le_bytes())?;
      w.write_all(&fname_id.to_le_bytes())?;
      w.write_all(&pkg_id.to_le_bytes())?;
    }

    w.flush()?;
    Ok(())
  }

  /// Sorts one string pool and returns the old-id → new-id remap, so
  /// references made in insertion order can be redirected to sorted
  /// positions.
  fn pool_remap(pool: &HashMap<String, u32>) -> Vec<u32> {
    let mut sorted: Vec<(&String, &u32)> = pool.iter().collect();
    sorted.sort_by_key(|(s, _)| s.as_str());

    // remap[old_id] = new_id
    let mut remap = vec![0u32; pool.len()];
    for (new_id, (_, old_id)) in sorted.iter().enumerate() {
      remap[**old_id as usize] = new_id as u32;
    }
    remap
  }

  /// The pool's strings arranged into their final sorted positions,
  /// matching the ids the remap redirected references to.
  fn pool_strings(pool: &HashMap<String, u32>, remap: &[u32]) -> Vec<String> {
    let mut result = vec![String::new(); pool.len()];
    for (s, old_id) in pool {
      result[remap[*old_id as usize] as usize] = s.clone();
    }
    result
  }

  /// Writes one string collection, each string length-framed so its
  /// boundaries can be recovered on read-back.
  fn string_table_write(w: &mut impl Write, strings: &[String]) -> Result<()> {
    for s in strings {
      let len = s.len() as u16;
      w.write_all(&len.to_le_bytes())?;
      w.write_all(s.as_bytes())?;
    }
    Ok(())
  }
}

// ---------------------------------------------------------------------------
// Query: look up a file path → package name
// ---------------------------------------------------------------------------

/// Process-global cache mapping each on-disk index's path to its
/// parsed form. Access is serialized on every lookup; contention is
/// negligible because the linker walk that drives these lookups is
/// single-threaded, and the parse cost amortizes after the first
/// lookup per file.
static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<PathIndex>>>> = OnceLock::new();

/// The read side: a searchable view of one on-disk index, holding the
/// raw bytes plus the offsets needed to answer lookups quickly.
pub struct PathIndex {
  /// The on-disk bytes in memory; the offset tables and entries point
  /// into this single buffer.
  data: Vec<u8>,
  /// (offset, len) of each directory string, in sorted order.
  dir_offsets: Vec<(usize, usize)>,
  /// (offset, len) of each filename string, in sorted order.
  fname_offsets: Vec<(usize, usize)>,
  /// (offset, len) of each package name, in sorted order.
  pkg_offsets: Vec<(usize, usize)>,
  /// Number of package names, to bounds-check an entry's package id.
  pkg_count: usize,
  /// Byte offset where the run of fixed-width entries begins.
  entries_start: usize,
  /// Number of entries in that run.
  entries_count: usize,
}

impl PathIndex {
  /// Signature at the start of every index file, checked on open so an
  /// unrelated or corrupt file is rejected immediately.
  const MAGIC: [u8; 4] = *b"FRPI";
  /// On-disk layout version; a reader refuses any value it does not
  /// recognise.
  const VERSION: u32 = 1;
  /// Size of the opening block: signature, version, and the four counts.
  const HEADER_SIZE: usize = 24;
  /// Size of one stored entry: three little-endian u32 ids.
  const ENTRY_SIZE: usize = 12;

  /// Field offsets inside one fixed-width entry: directory id,
  /// filename id, package id — three little-endian u32s (`ENTRY_SIZE`
  /// bytes total).
  const ENTRY_FIELD_DIR: usize = 0;
  const ENTRY_FIELD_FNAME: usize = 4;
  const ENTRY_FIELD_PKG: usize = 8;

  /// Returns the parsed index for `path`, parsing once and caching it for
  /// the run. A file that does not exist yields `None` — not every
  /// distribution publishes an ownership listing, and its absence is not
  /// an error.
  pub fn open(path: &Path) -> Result<Option<Arc<PathIndex>>> {
    if !path.exists() {
      return Ok(None);
    }
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache
      .lock()
      .map_err(|_| anyhow::anyhow!("path-index cache mutex poisoned"))?;
    if let Some(parsed) = guard.get(path) {
      return Ok(Some(parsed.clone()));
    }
    let parsed = Arc::new(Self::parse(path)?);
    guard.insert(path.to_path_buf(), parsed.clone());
    Ok(Some(parsed))
  }

  /// The path index's file path, derived from the database's path so
  /// the pair never drift apart.
  pub fn of_db(path_file_db: &Path) -> PathBuf {
    path_file_db.with_extension("pathidx")
  }

  /// The single package that ships one exact file path, or `None` when no
  /// package claims it — never an invented owner.
  pub fn query(&self, file_path: &str) -> Result<Option<String>> {
    let (query_dir, query_fname) = match file_path.rfind('/') {
      Some(pos) => (&file_path[..pos], &file_path[pos + 1..]),
      None => return Ok(None),
    };

    let dir_id = match self.string_search(&self.dir_offsets, query_dir)? {
      Some(id) => id,
      None => return Ok(None),
    };
    let fname_id = match self.string_search(&self.fname_offsets, query_fname)? {
      Some(id) => id,
      None => return Ok(None),
    };

    let target_dir = dir_id as u32;
    let target_fname = fname_id as u32;

    let mut lo = 0usize;
    let mut hi = self.entries_count;
    while lo < hi {
      let mid = lo + (hi - lo) / 2;
      let (e_dir, e_fname, e_pkg) = self.entry_read(mid);
      match (e_dir, e_fname).cmp(&(target_dir, target_fname)) {
        std::cmp::Ordering::Less => lo = mid + 1,
        std::cmp::Ordering::Greater => hi = mid,
        std::cmp::Ordering::Equal => return Ok(Some(self.pkg_name_at(e_pkg as usize)?.to_string())),
      }
    }

    Ok(None)
  }

  /// The (dir, filename, package) ids of the entry at one position in
  /// the run of fixed-width entries.
  fn entry_read(&self, index: usize) -> (u32, u32, u32) {
    let base = self.entries_start + index * Self::ENTRY_SIZE;
    (
      Self::u32_le_at(&self.data, base + Self::ENTRY_FIELD_DIR),
      Self::u32_le_at(&self.data, base + Self::ENTRY_FIELD_FNAME),
      Self::u32_le_at(&self.data, base + Self::ENTRY_FIELD_PKG),
    )
  }

  fn u32_le_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
  }

  /// The package name an entry references, refusing an id outside the
  /// pool — a corrupt index — rather than reading past it.
  fn pkg_name_at(&self, pkg_id: usize) -> Result<&str> {
    if pkg_id >= self.pkg_count {
      bail!("Invalid package ID {} in path index entry", pkg_id);
    }
    let (str_offset, str_len) = self.pkg_offsets[pkg_id];
    std::str::from_utf8(&self.data[str_offset..str_offset + str_len]).context("Invalid UTF-8 in package name")
  }

  /// The directory string an entry references, refusing an id outside
  /// the pool — a corrupt index — rather than reading past it.
  fn dir_name_at(&self, dir_id: usize) -> Result<&str> {
    if dir_id >= self.dir_offsets.len() {
      bail!("Invalid directory ID {} in path index entry", dir_id);
    }
    let (str_offset, str_len) = self.dir_offsets[dir_id];
    std::str::from_utf8(&self.data[str_offset..str_offset + str_len]).context("Invalid UTF-8 in directory name")
  }

  /// The filename string an entry references, refusing an id outside
  /// the pool — a corrupt index — rather than reading past it.
  fn fname_name_at(&self, fname_id: usize) -> Result<&str> {
    if fname_id >= self.fname_offsets.len() {
      bail!("Invalid filename ID {} in path index entry", fname_id);
    }
    let (str_offset, str_len) = self.fname_offsets[fname_id];
    std::str::from_utf8(&self.data[str_offset..str_offset + str_len]).context("Invalid UTF-8 in filename")
  }

  /// Every package shipping a file whose basename matches the pattern,
  /// complementing the provides entries, so a library with no provides
  /// entry is still found. Returned sorted and deduplicated for
  /// merging.
  pub fn query_glob(&self, filename_pattern: &str) -> Result<Vec<(String, String)>> {
    // Case-insensitive to match the database's `glob_bash` SQL
    // function, so every pattern matches the same way everywhere.
    let matcher = globset::GlobBuilder::new(filename_pattern)
      .case_insensitive(true)
      .build()
      .with_context(|| format!("invalid glob pattern: {}", filename_pattern))?
      .compile_matcher();

    // Walk the filename pool linearly, recording every (idx, name) whose
    // basename matches the glob. The pool is sorted but glob matching is
    // not range-shaped, so a linear scan is the simplest correct shape.
    let mut matched: HashMap<u32, String> = HashMap::new();
    for (fname_id, &(str_offset, str_len)) in self.fname_offsets.iter().enumerate() {
      let fname =
        std::str::from_utf8(&self.data[str_offset..str_offset + str_len]).context("Invalid UTF-8 in filename pool")?;
      if matcher.is_match(fname) {
        matched.insert(fname_id as u32, fname.to_string());
      }
    }

    if matched.is_empty() {
      return Ok(Vec::new());
    }

    // Walk the entries array; for each entry whose fname_id is in `matched`,
    // dereference the package name and emit a `(filename, package)` pair.
    let mut out: Vec<(String, String)> = Vec::new();
    for i in 0..self.entries_count {
      let (_, fname_id, pkg_id) = self.entry_read(i);
      let Some(fname) = matched.get(&fname_id) else {
        continue;
      };
      let pkg = self.pkg_name_at(pkg_id as usize)?;
      out.push((fname.clone(), pkg.to_string()));
    }

    out.sort();
    out.dedup();
    Ok(out)
  }

  /// Every package shipping a file whose full installed path matches
  /// `pattern`, the leading slash dropped so patterns read as the tool
  /// prints paths (`usr/bin/bash`). Returned sorted and deduplicated.
  pub fn query_glob_path(&self, pattern: &str) -> Result<Vec<(String, String)>> {
    // Case-insensitive to match the database's `glob_bash` SQL
    // function, so every pattern matches the same way everywhere.
    let matcher = globset::GlobBuilder::new(pattern)
      .case_insensitive(true)
      .build()
      .with_context(|| format!("invalid glob pattern: {}", pattern))?
      .compile_matcher();

    // Entries are sorted by (dir_id, fname_id), so consecutive entries share
    // a directory. The joined path lives in one reused buffer: the directory
    // prefix is laid down once per run of same-dir entries and only the
    // filename tail is rewritten per entry.
    let mut out: Vec<(String, String)> = Vec::new();
    let mut dir_current: Option<u32> = None;
    let mut buf_path = String::new();
    let mut len_prefix = 0usize;
    for i in 0..self.entries_count {
      let (dir_id, fname_id, pkg_id) = self.entry_read(i);
      if dir_current != Some(dir_id) {
        let dir = self.dir_name_at(dir_id as usize)?;
        buf_path.clear();
        buf_path.push_str(dir.strip_prefix('/').unwrap_or(dir));
        if !buf_path.is_empty() {
          buf_path.push('/');
        }
        len_prefix = buf_path.len();
        dir_current = Some(dir_id);
      }
      buf_path.truncate(len_prefix);
      buf_path.push_str(self.fname_name_at(fname_id as usize)?);
      if matcher.is_match(&buf_path) {
        let pkg = self.pkg_name_at(pkg_id as usize)?;
        out.push((buf_path.clone(), pkg.to_string()));
      }
    }

    out.sort();
    out.dedup();
    Ok(out)
  }

  /// Interprets the on-disk bytes into a searchable `PathIndex`,
  /// confirming the magic, version, and length first so a truncated or
  /// unrelated file is refused rather than misread deep in a later search.
  fn parse(path: &Path) -> Result<PathIndex> {
    let data = std::fs::read(path).with_context(|| format!("Failed to read path index at {}", path.display()))?;

    if data.len() < Self::HEADER_SIZE {
      bail!("Path index too small: {} bytes", data.len());
    }
    if data[0..4] != Self::MAGIC {
      bail!("Invalid path index magic: {:?}", &data[0..4]);
    }
    let version = Self::u32_le_at(&data, 4);
    if version != Self::VERSION {
      bail!("Unsupported path index version: {}", version);
    }
    let dir_count = Self::u32_le_at(&data, 8) as usize;
    let fname_count = Self::u32_le_at(&data, 12) as usize;
    let pkg_count = Self::u32_le_at(&data, 16) as usize;
    let entries_count = Self::u32_le_at(&data, 20) as usize;

    let mut offset = Self::HEADER_SIZE;
    let dir_offsets = Self::offsets_read(&data, &mut offset, dir_count)?;
    let fname_offsets = Self::offsets_read(&data, &mut offset, fname_count)?;
    let pkg_offsets = Self::offsets_read(&data, &mut offset, pkg_count)?;
    let entries_start = offset;

    if entries_start + entries_count * Self::ENTRY_SIZE > data.len() {
      bail!("Truncated path index entries");
    }

    Ok(PathIndex {
      data,
      dir_offsets,
      fname_offsets,
      pkg_offsets,
      pkg_count,
      entries_start,
      entries_count,
    })
  }

  /// Walks one string table's length framing to map each string's
  /// (offset, len); a table that runs out before its promised text is
  /// refused rather than read past.
  fn offsets_read(data: &[u8], offset: &mut usize, count: usize) -> Result<Vec<(usize, usize)>> {
    let mut offsets = Vec::with_capacity(count);
    let mut pos = *offset;
    for _ in 0..count {
      if pos + 2 > data.len() {
        bail!("Truncated string table");
      }
      let len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
      offsets.push((pos + 2, len));
      pos += 2 + len;
    }
    *offset = pos;
    Ok(offsets)
  }

  /// Binary-searches one sorted string table for `target`, returning its
  /// index or `None`.
  fn string_search(&self, offsets: &[(usize, usize)], target: &str) -> Result<Option<usize>> {
    let mut lo = 0usize;
    let mut hi = offsets.len();
    while lo < hi {
      let mid = lo + (hi - lo) / 2;
      let (str_offset, str_len) = offsets[mid];
      let s =
        std::str::from_utf8(&self.data[str_offset..str_offset + str_len]).context("Invalid UTF-8 in string table")?;

      match s.cmp(target) {
        std::cmp::Ordering::Less => lo = mid + 1,
        std::cmp::Ordering::Greater => hi = mid,
        std::cmp::Ordering::Equal => return Ok(Some(mid)),
      }
    }
    Ok(None)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  fn build_test_index(entries: &[(&str, &str, &str)]) -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("test.pathidx");
    let mut builder = PathIndexBuilder::new();
    for (dir, fname, pkg) in entries {
      builder.entry_push(dir, fname, pkg);
    }
    builder.finalize(&path).unwrap();
    (tmp, path)
  }

  // ── query_glob ────────────────────────────────────────────────────────

  #[test]
  fn query_glob_returns_empty_for_missing_index() {
    let tmp = TempDir::new().unwrap();
    let opened = PathIndex::open(&tmp.path().join("does-not-exist.pathidx")).unwrap();
    assert!(opened.is_none());
  }

  #[test]
  fn query_glob_matches_filename_basename() {
    let (_tmp, path) = build_test_index(&[
      ("/usr/lib/x86_64-linux-gnu", "libssl.so.3", "libssl3"),
      ("/usr/lib/x86_64-linux-gnu", "libcrypto.so.3", "libssl3"),
      ("/usr/lib", "libfoo.so.1", "foopkg"),
      ("/usr/bin", "bash", "bash"),
    ]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob("libssl.so.3")
      .unwrap();
    assert_eq!(result, vec![("libssl.so.3".to_string(), "libssl3".to_string())]);
  }

  /// FR-MATCH-001: every pattern surface matches case-insensitively,
  /// the same way the database's `glob_bash` SQL function does.
  #[test]
  fn query_glob_is_case_insensitive() {
    let (_tmp, path) = build_test_index(&[("/usr/lib", "libSSL.so.3", "libssl3")]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob("libssl.so.*")
      .unwrap();
    assert_eq!(result, vec![("libSSL.so.3".to_string(), "libssl3".to_string())]);
  }

  /// FR-MATCH-001 for the full-path surface: mixed-case patterns find
  /// the recorded path regardless of case.
  #[test]
  fn query_glob_path_is_case_insensitive() {
    let (_tmp, path) = build_test_index(&[("/usr/bin", "bash", "bash")]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob_path("USR/BIN/BASH")
      .unwrap();
    assert_eq!(result, vec![("usr/bin/bash".to_string(), "bash".to_string())]);
  }

  #[test]
  fn query_glob_returns_multiple_matches_unioned_and_sorted() {
    let (_tmp, path) = build_test_index(&[
      ("/usr/lib", "libssl.so.3", "libssl3"),
      ("/usr/lib", "libcrypto.so.3", "libssl3"),
      ("/usr/lib", "libfoo.so.1", "foopkg"),
    ]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob("lib*.so.*")
      .unwrap();
    assert_eq!(
      result,
      vec![
        ("libcrypto.so.3".to_string(), "libssl3".to_string()),
        ("libfoo.so.1".to_string(), "foopkg".to_string()),
        ("libssl.so.3".to_string(), "libssl3".to_string()),
      ]
    );
  }

  #[test]
  fn query_glob_empty_when_pattern_matches_no_filenames() {
    let (_tmp, path) = build_test_index(&[("/usr/bin", "bash", "bash")]);
    let result = PathIndex::open(&path).unwrap().unwrap().query_glob("lib*").unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn query_glob_question_mark() {
    let (_tmp, path) = build_test_index(&[
      ("/usr/lib", "libssl.so.1", "libssl1"),
      ("/usr/lib", "libssl.so.3", "libssl3"),
      ("/usr/lib", "libssl.so.10", "libssl10"),
    ]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob("libssl.so.?")
      .unwrap();
    // Only single-char trailing matches; the .10 case is rejected.
    assert_eq!(
      result,
      vec![
        ("libssl.so.1".to_string(), "libssl1".to_string()),
        ("libssl.so.3".to_string(), "libssl3".to_string()),
      ]
    );
  }

  #[test]
  fn query_glob_dedups_when_same_filename_pkg_pair_appears_twice() {
    // Builder dedups (dir, fname, pkg) at finalize time; we're testing
    // the query path's own dedup discipline by ingesting under two dirs
    // with the same (filename, package).
    let (_tmp, path) = build_test_index(&[
      ("/usr/lib", "libfoo.so.1", "foopkg"),
      ("/usr/lib32", "libfoo.so.1", "foopkg"),
    ]);
    let result = PathIndex::open(&path).unwrap().unwrap().query_glob("libfoo*").unwrap();
    assert_eq!(result, vec![("libfoo.so.1".to_string(), "foopkg".to_string())]);
  }

  // ── query_glob_path ───────────────────────────────────────────────────

  #[test]
  fn query_glob_path_matches_exact_full_path() {
    let (_tmp, path) = build_test_index(&[("/usr/bin", "bash", "bash"), ("/usr/lib", "libssl.so.3", "libssl3")]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob_path("usr/bin/bash")
      .unwrap();
    assert_eq!(result, vec![("usr/bin/bash".to_string(), "bash".to_string())]);
  }

  #[test]
  fn query_glob_path_distinguishes_same_filename_in_different_dirs() {
    let (_tmp, path) = build_test_index(&[("/usr/bin", "foo", "foo-cli"), ("/usr/lib", "foo", "foo-lib")]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob_path("usr/bin/foo")
      .unwrap();
    assert_eq!(result, vec![("usr/bin/foo".to_string(), "foo-cli".to_string())]);
  }

  #[test]
  fn query_glob_path_star_crosses_directory_separators() {
    // globset's default (no literal_separator) lets `*` match `/`, so a
    // bare `*.so*` pattern reaches into every directory. This pins that
    // behaviour: the path search and its documentation must move together
    // if the glob construction ever changes.
    let (_tmp, path) = build_test_index(&[
      ("/usr/lib", "libssl.so.3", "libssl3"),
      ("/usr/lib/x86_64-linux-gnu", "libcrypto.so.3", "libssl3"),
      ("/usr/bin", "bash", "bash"),
    ]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob_path("*.so*")
      .unwrap();
    assert_eq!(
      result,
      vec![
        ("usr/lib/libssl.so.3".to_string(), "libssl3".to_string()),
        ("usr/lib/x86_64-linux-gnu/libcrypto.so.3".to_string(), "libssl3".to_string()),
      ]
    );
  }

  #[test]
  fn query_glob_path_subtree_pattern() {
    let (_tmp, path) = build_test_index(&[
      ("/etc/ssl", "openssl.cnf", "openssl"),
      ("/etc/ssl/certs", "ca-certificates.crt", "ca-certificates"),
      ("/etc", "fstab", "base-files"),
    ]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob_path("etc/ssl/*")
      .unwrap();
    assert_eq!(
      result,
      vec![
        ("etc/ssl/certs/ca-certificates.crt".to_string(), "ca-certificates".to_string()),
        ("etc/ssl/openssl.cnf".to_string(), "openssl".to_string()),
      ]
    );
  }

  #[test]
  fn query_glob_path_strips_leading_slash_from_recorded_dirs() {
    // Every ingester records absolute dirs; the match space presents
    // paths without the leading slash, so a pattern spelled with one
    // never matches.
    let (_tmp, path) = build_test_index(&[("/usr/bin", "bash", "bash")]);
    let index = PathIndex::open(&path).unwrap().unwrap();
    assert!(index.query_glob_path("/usr/bin/bash").unwrap().is_empty());
    assert_eq!(index.query_glob_path("usr/bin/bash").unwrap().len(), 1);
  }

  #[test]
  fn query_glob_path_empty_when_no_match() {
    let (_tmp, path) = build_test_index(&[("/usr/bin", "bash", "bash")]);
    let result = PathIndex::open(&path)
      .unwrap()
      .unwrap()
      .query_glob_path("opt/**")
      .unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn query_glob_path_rejects_invalid_pattern() {
    let (_tmp, path) = build_test_index(&[("/usr/bin", "bash", "bash")]);
    assert!(PathIndex::open(&path).unwrap().unwrap().query_glob_path("[").is_err());
  }
}
