//! The local package index of a distribution: an SQLite database built
//! once from the parsed upstream metadata and read for the rest of the
//! run, with the writing side and the reading sides each behind its own
//! narrow view.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use crate::package::{DepKind, DepSpec, Dependency, Package, RichDep};
use crate::path_index::{PathIndex, PathIndexBuilder};
use crate::version::{DpkgVersionCompare, VersionCompare};

/// TTL for cached database files: one hour keeps repeat invocations
/// near-instant while bounding staleness for a fast-moving rolling release.
const DB_CACHE_TTL_SECS: u64 = 3600;

// ---------------------------------------------------------------------------
// Row types returned by queries
// ---------------------------------------------------------------------------

/// The facts the index holds about one package — the single row shape
/// every package read returns.
pub struct PackageRow {
  /// Package name as the index reports it.
  pub name: String,
  /// Package version as the index reports it.
  pub version: String,
  /// Free-form description (often the index's first-line
  /// summary).
  pub description: String,
  /// Archive filename relative to the mirror's pool — combined
  /// with the remote's base URL to yield a download URL.
  pub filename: String,
  /// Archive size in bytes, as the index reports it.
  pub size: u64,
  /// Checksum string, encoded in the algorithm-specific form
  /// the index publishes (hex SHA-256, hex SHA-512, or
  /// `Q1`-prefixed base64 SHA-1).
  pub checksum: String,
  /// Whether the distribution designates this package as one that
  /// must always be present — set only on the Debian family, and
  /// false on every other distribution.
  pub essential: bool,
}

/// One package that provides a virtual name, with the version it
/// declares for it when the source states one — both facts the resolver
/// needs to check a version-constrained dependency.
pub struct ProviderRow {
  /// Name of the package that provides the virtual name.
  pub package_name: String,
  /// Version the package declares for the virtual name, when the
  /// source states one; most provides entries state none.
  pub provides_version: Option<String>,
}

/// One provides entry matching a glob pattern, paired with its owning
/// package's row facts so the hit can be shown without a second lookup.
pub struct ProvidesGlobRow {
  /// Owning-package name.
  pub package_name: String,
  /// The provides name that matched the pattern.
  pub provides_name: String,
  /// Latest version of the owning package under the active
  /// version-ordering rule.
  pub package_version: String,
  /// Description of the owning package, taken from its package
  /// row.
  pub package_description: String,
}

// ---------------------------------------------------------------------------
// Index — the owned package-index entity
// ---------------------------------------------------------------------------

/// The local package index for one distribution, release, and
/// architecture, carrying the source's version-ordering rule so every
/// reader answers the way the source would. All reading and building of
/// the index passes through here.
pub struct Index {
  /// Open connection with the version-ordering collation and the wildcard
  /// matcher registered.
  conn: Connection,
  /// The active version-ordering rule that "newest version" reads depend
  /// on.
  vcmp: Arc<dyn VersionCompare>,
  /// Path of the store, from which the companion path-index's sibling path
  /// is derived.
  path_file_db: PathBuf,
}

impl Index {
  /// Opens a stored index database for querying with the
  /// distribution's version ordering registered, so "newest version" is
  /// answered as the source would and duplicate releases collapse to
  /// one best per name.
  pub fn open(path_file_db: &Path, vcmp: Arc<dyn VersionCompare>) -> Result<Self> {
    let conn = Connection::open(path_file_db)
      .with_context(|| format!("Failed to open database at {}", path_file_db.display()))?;
    Self::functions_register(&conn)?;
    Self::collation_register(&conn, vcmp.clone())?;
    Ok(Self {
      conn,
      vcmp,
      path_file_db: path_file_db.to_path_buf(),
    })
  }

  /// Ensures a fresh local index exists, paying the network cost at
  /// most once: a recent copy is reused, otherwise a fresh build is
  /// prepared in a `.tmp` sibling and swapped in whole, so concurrent
  /// runs never collide and a crash leaves nothing half-written behind.
  pub fn open_or_populate(
    cache_key: &str,
    vcmp: Arc<dyn VersionCompare>,
    populate: impl FnOnce(&mut IndexWriter) -> Result<()>,
  ) -> Result<Self> {
    let index_dir = crate::internal::cache::Cache::dir_resolve()?.dir_index();
    std::fs::create_dir_all(&index_dir)?;

    // Sanitize cache_key for use as filename: replace / with -
    let db_filename = format!("{}.db", cache_key.replace('/', "-"));
    let path_file_db = index_dir.join(&db_filename);

    // Fast path: existing fresh `.db`. Readers don't contend on the lock.
    if let Some(index) = Self::open_if_fresh(&path_file_db, &db_filename, vcmp.clone())? {
      return Ok(index);
    }

    let _lock_guard = Self::populate_lock(&path_file_db)?;

    // TOCTOU: another process may have populated while we waited on the
    // lock. Recheck before tearing down what is now valid cache.
    if let Some(index) = Self::open_if_fresh(&path_file_db, &db_filename, vcmp.clone())? {
      return Ok(index);
    }

    Self::stale_remove(&path_file_db);
    Self::populate_swap(&path_file_db, populate)?;
    Self::open(&path_file_db, vcmp)
  }

  /// The cheap reuse path: opens the existing index database when it is
  /// still within the TTL, announcing the cache hit. `None` means a
  /// fresh build is needed.
  fn open_if_fresh(path_file_db: &Path, db_filename: &str, vcmp: Arc<dyn VersionCompare>) -> Result<Option<Self>> {
    if !Self::fresh(path_file_db) {
      return Ok(None);
    }
    eprintln!("  cached: {}", db_filename);
    Ok(Some(Self::open(path_file_db, vcmp)?))
  }

  /// Exclusive flock on a per-cache-key sentinel so concurrent processes
  /// don't race on the shared `.tmp` populate path. The lock file itself is
  /// reusable across runs and intentionally never removed — flock semantics
  /// rely on dirent stability across processes. The guard releases on scope
  /// exit (success or error).
  fn populate_lock(path_file_db: &Path) -> Result<nix::fcntl::Flock<std::fs::File>> {
    let lock_path = path_file_db.with_extension("lock");
    let lock_file = std::fs::OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .open(&lock_path)
      .with_context(|| format!("Failed to open lock file {}", lock_path.display()))?;
    nix::fcntl::Flock::lock(lock_file, nix::fcntl::FlockArg::LockExclusive)
      .map_err(|(_, errno)| anyhow::anyhow!("Failed to acquire lock {}: {}", lock_path.display(), errno))
  }

  /// Clears a stale index database and its companions — WAL files and
  /// the path index — so the rebuild starts empty. Removal is
  /// best-effort: a file may legitimately be absent, and a real
  /// obstruction surfaces when the rebuild writes to the same path.
  fn stale_remove(path_file_db: &Path) {
    if !path_file_db.exists() {
      return;
    }
    Self::drop_if_present(path_file_db);
    Self::drop_if_present(&path_file_db.with_extension("db-wal"));
    Self::drop_if_present(&path_file_db.with_extension("db-shm"));
    Self::drop_if_present(&PathIndex::of_db(path_file_db));
  }

  /// Builds the fresh index into a `.tmp` sibling and renames it over
  /// the final name only once whole, so a populate that dies partway
  /// leaves no half-written file to be mistaken for a valid index on
  /// the next run.
  fn populate_swap(path_file_db: &Path, populate: impl FnOnce(&mut IndexWriter) -> Result<()>) -> Result<()> {
    let tmp_path = path_file_db.with_extension("db.tmp");
    Self::tmp_remove(&tmp_path);

    if let Err(e) = Self::tmp_populate(&tmp_path, path_file_db, populate) {
      Self::tmp_remove(&tmp_path);
      return Err(e);
    }
    std::fs::rename(&tmp_path, path_file_db)
      .with_context(|| format!("Failed to rename {} to {}", tmp_path.display(), path_file_db.display()))
  }

  /// Remove a tmp build and its WAL companions, before a build (leftovers of
  /// a crashed run) and after a failed one.
  fn tmp_remove(tmp_path: &Path) {
    Self::drop_if_present(tmp_path);
    Self::drop_if_present(&PathBuf::from(format!("{}-wal", tmp_path.display())));
    Self::drop_if_present(&PathBuf::from(format!("{}-shm", tmp_path.display())));
  }

  /// One populate attempt into the tmp file: builds the schema, runs
  /// the caller's inserts in one transaction that rolls back unless
  /// committed, finalizes the companion path index, and commits.
  fn tmp_populate(
    tmp_path: &Path,
    path_file_db: &Path,
    populate: impl FnOnce(&mut IndexWriter) -> Result<()>,
  ) -> Result<()> {
    let mut conn =
      Connection::open(tmp_path).with_context(|| format!("Failed to create database at {}", tmp_path.display()))?;
    Self::schema(&conn)?;

    let path_file_path_index = PathIndex::of_db(path_file_db);
    let tx = conn.transaction().context("Failed to begin transaction")?;
    {
      let mut writer = IndexWriter::new(&tx);
      populate(&mut writer)?;
      writer.paths_finalize(&path_file_path_index)?;
    }
    tx.commit().context("Failed to commit transaction")?;

    conn
      .close()
      .map_err(|(_, e)| anyhow::anyhow!("Failed to close database: {}", e))?;
    Ok(())
  }

  /// The companion path index, kept as a matched pair with the
  /// database, or `None` for a source that publishes no file lists.
  pub fn paths(&self) -> Result<Option<Arc<PathIndex>>> {
    PathIndex::open(&PathIndex::of_db(&self.path_file_db))
  }

  /// Raw connection for the one command that runs an arbitrary read query;
  /// every other reader goes through a narrow view.
  pub fn connection(&self) -> &Connection {
    &self.conn
  }

  /// The version-ordering rule, so the resolver's constraint checks use
  /// exactly the order the index's collation uses internally.
  pub fn version_cmp(&self) -> &dyn VersionCompare {
    &*self.vcmp
  }

  /// A view onto the packages themselves — identities, versions, facts.
  pub fn packages(&self) -> Packages<'_> {
    Packages(&self.conn)
  }

  /// A view onto the provides entries, for finding which packages
  /// provide a virtual name.
  pub fn providers(&self) -> Providers<'_> {
    Providers(&self.conn)
  }

  /// A view onto the dependencies packages declare.
  pub fn dependencies(&self) -> Dependencies<'_> {
    Dependencies(&self.conn)
  }

  /// An in-memory index preloaded with `packages`, using the default
  /// ordering, so the views can be exercised without the on-disk
  /// rebuild lifecycle.
  #[cfg(test)]
  pub fn in_memory(packages: Vec<Package>) -> Self {
    let conn = Connection::open_in_memory().unwrap();
    Self::schema(&conn).unwrap();
    {
      let writer = IndexWriter::new(&conn);
      for pkg in &packages {
        writer.insert(pkg).unwrap();
      }
    }
    Self {
      conn,
      vcmp: Arc::new(DpkgVersionCompare),
      path_file_db: PathBuf::from(":memory:"),
    }
  }

  /// An on-disk index preloaded with `packages`, so the companion path
  /// index resolves to a real sibling file a test can write —
  /// exercising the shipped-files search paths an in-memory one cannot.
  #[cfg(test)]
  pub fn on_disk(path_file_db: &Path, packages: Vec<Package>) -> Self {
    let conn = Connection::open(path_file_db).unwrap();
    Self::schema(&conn).unwrap();
    {
      let writer = IndexWriter::new(&conn);
      for pkg in &packages {
        writer.insert(pkg).unwrap();
      }
    }
    Self {
      conn,
      vcmp: Arc::new(DpkgVersionCompare),
      path_file_db: path_file_db.to_path_buf(),
    }
  }

  /// Whether a cached index database is still within the TTL — the
  /// gate between cheap reuse and a fresh build.
  fn fresh(path_file_db: &Path) -> bool {
    let Ok(metadata) = path_file_db.metadata() else {
      return false;
    };
    let Ok(modified) = metadata.modified() else {
      return false;
    };
    let Ok(age) = modified.elapsed() else {
      return false;
    };
    age.as_secs() < DB_CACHE_TTL_SECS
  }

  /// Removes a leftover file if present; a genuine failure warns but does
  /// not derail the rebuild, which surfaces a real obstruction when it
  /// next writes the same path.
  fn drop_if_present(path: &Path) {
    match std::fs::remove_file(path) {
      Ok(_) => {}
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
      Err(e) => eprintln!("warning: could not remove stale {}: {}", path.display(), e),
    }
  }

  /// Creates the empty database schema, with foreign keys on so
  /// relationships cannot dangle.
  fn schema(conn: &Connection) -> Result<()> {
    // Performance: reduced sync — every populate runs as one large
    // transaction with a single commit at the end, so the journal mode
    // matters little while `synchronous=NORMAL` lets SQLite skip
    // intermediate fsyncs. The default rollback journal (DELETE) is
    // used rather than WAL: `Index::open_or_populate` writes the `.tmp`
    // DB and then renames it to the final `.db`; WAL would leave a
    // companion `.tmp-wal/.tmp-shm` pair behind that the rename never
    // moves, stranding the schema in an unread WAL alongside an empty
    // `.db`. Foreign keys are on so stray `package_id` rows are
    // rejected at insert time rather than silently orphaned.
    conn.execute_batch("PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")?;

    conn
      .execute_batch(
        "CREATE TABLE packages (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL,
        version     TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        filename    TEXT NOT NULL,
        size        INTEGER NOT NULL DEFAULT 0,
        checksum    TEXT NOT NULL DEFAULT '',
        essential   INTEGER NOT NULL DEFAULT 0,
        priority    INTEGER
      );
      CREATE INDEX idx_packages_name ON packages(name);

      CREATE TABLE dependencies (
        package_id         INTEGER NOT NULL,
        kind               TEXT NOT NULL,
        group_id           INTEGER NOT NULL,
        position           INTEGER NOT NULL,
        name               TEXT NOT NULL,
        version_constraint TEXT,
        FOREIGN KEY (package_id) REFERENCES packages(id)
      );
      CREATE INDEX idx_deps_pkg ON dependencies(package_id, kind);
      CREATE INDEX idx_deps_name ON dependencies(name);

      CREATE TABLE provides (
        package_id INTEGER NOT NULL,
        name       TEXT NOT NULL,
        version    TEXT,
        FOREIGN KEY (package_id) REFERENCES packages(id)
      );
      CREATE INDEX idx_provides_name ON provides(name);

      CREATE TABLE install_if (
        package_id INTEGER NOT NULL,
        trigger    TEXT NOT NULL,
        FOREIGN KEY (package_id) REFERENCES packages(id)
      );
      CREATE INDEX idx_install_if_pkg ON install_if(package_id);

      CREATE TABLE rich_deps (
        package_id INTEGER NOT NULL,
        ast        TEXT NOT NULL,
        FOREIGN KEY (package_id) REFERENCES packages(id)
      );
      CREATE INDEX idx_rich_deps_pkg ON rich_deps(package_id);",
      )
      .context("Failed to create database schema")?;

    Self::functions_register(conn)?;
    // Default `version_compare` collation so callers that use this
    // connection directly (test fixtures, the populate-phase
    // transaction) can run queries that contain
    // `ORDER BY version COLLATE version_compare` without further
    // setup. The production read path goes through [`Index::open`],
    // which opens a fresh connection and re-registers the collation
    // against whichever comparator the caller hands in.
    Self::collation_register(conn, Arc::new(DpkgVersionCompare))?;
    Ok(())
  }

  /// Registers the distribution's version ordering on the connection, so
  /// any query asking for the latest version resolves it the source's way.
  fn collation_register(conn: &Connection, vcmp: Arc<dyn VersionCompare>) -> Result<()> {
    conn
      .create_collation("version_compare", move |a, b| vcmp.compare(a, b))
      .context("Failed to register version_compare collation")?;
    Ok(())
  }

  /// Registers the shell-style wildcard function so glob searches match
  /// the way a user expects, consistently everywhere a pattern is matched.
  fn functions_register(conn: &Connection) -> Result<()> {
    use rusqlite::functions::FunctionFlags;

    conn
      .create_scalar_function(
        "glob_bash",
        2,
        FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_UTF8,
        |ctx| -> rusqlite::Result<bool> {
          // Cache the compiled matcher against the pattern argument so
          // the glob compiles once per query, not once per row. SQLite
          // calls `glob_bash` with the same pattern for every row in
          // the scan; aux-data persists across those calls.
          let matcher: Arc<globset::GlobMatcher> = match ctx.get_aux::<globset::GlobMatcher>(1)? {
            Some(arc) => arc,
            None => {
              let pattern: String = ctx.get(1)?;
              let m = globset::GlobBuilder::new(&pattern)
                .case_insensitive(true)
                .build()
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?
                .compile_matcher();
              ctx.set_aux::<globset::GlobMatcher>(1, m)?
            }
          };
          let target: String = ctx.get(0)?;
          Ok(matcher.is_match(&target))
        },
      )
      .context("Failed to register glob_bash scalar function")?;
    Ok(())
  }
}

// ---------------------------------------------------------------------------
// IndexWriter — the only INSERT/DELETE surface (populate phase)
// ---------------------------------------------------------------------------

/// The one write surface the index is populated through, owning both
/// halves of the pair: package rows into the database and
/// file-ownership rows into the path index, committed together so a
/// reader never sees the pair half-formed.
pub struct IndexWriter<'a> {
  conn: &'a Connection,
  paths: PathIndexBuilder,
}

impl<'a> IndexWriter<'a> {
  /// A writer over one connection, with an empty path-index
  /// accumulator.
  pub fn new(conn: &'a Connection) -> Self {
    IndexWriter {
      conn,
      paths: PathIndexBuilder::new(),
    }
  }

  /// Records one parsed batch of packages, sorted under the family's
  /// own version ordering — the rule index insertion relies on, stated
  /// once instead of repeated by every format parser.
  pub fn packages_insert(&mut self, mut packages: Vec<Package>, vcmp: &dyn VersionCompare) -> Result<()> {
    Package::list_sort(&mut packages, vcmp);
    for pkg in &packages {
      self.insert(pkg)?;
    }
    Ok(())
  }

  /// Records one file-ownership row — one package ships one file in
  /// one directory — into the path-index accumulator.
  pub fn path_push(&mut self, dir: &str, filename: &str, pkg: &str) {
    self.paths.entry_push(dir, filename, pkg);
  }

  /// Reads an entire repository's file listing into the path-index
  /// accumulator.
  pub fn paths_ingest(&mut self, reader: impl std::io::BufRead) -> Result<()> {
    self.paths.ingest(reader)
  }

  /// Writes the accumulated path index to disk once the database is
  /// populated, so the pair lands together and no format can forget its
  /// half.
  pub(crate) fn paths_finalize(self, path_file_path_index: &Path) -> Result<()> {
    self.paths.finalize(path_file_path_index)
  }

  /// Records one package in full — its row plus its dependencies,
  /// provides, install-if triggers, and rich deps — so a package is
  /// wholly present or wholly absent, never half-recorded.
  pub fn insert(&self, pkg: &Package) -> Result<i64> {
    self
      .conn
      .prepare_cached(
        "INSERT INTO packages (name, version, description, filename, size, checksum, essential, priority)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      )?
      .execute(params![
        pkg.name,
        pkg.version,
        pkg.description,
        pkg.filename,
        pkg.size as i64,
        pkg.checksum,
        pkg.essential as i32,
        pkg.priority.map(|p| p as i32),
      ])?;

    let id_pkg = self.conn.last_insert_rowid();

    self.deps_insert(id_pkg, DepKind::Depends, &pkg.depends)?;
    self.deps_insert(id_pkg, DepKind::Recommends, &pkg.recommends)?;
    self.deps_insert(id_pkg, DepKind::Suggests, &pkg.suggests)?;
    self.deps_insert(id_pkg, DepKind::Conflicts, &pkg.conflicts)?;
    self.deps_insert(id_pkg, DepKind::Breaks, &pkg.breaks)?;

    for prov in &pkg.provides {
      self
        .conn
        .prepare_cached("INSERT INTO provides (package_id, name, version) VALUES (?1, ?2, ?3)")?
        .execute(params![id_pkg, prov.name, prov.version_constraint])?;
    }

    for trigger in &pkg.install_if {
      self
        .conn
        .prepare_cached("INSERT INTO install_if (package_id, trigger) VALUES (?1, ?2)")?
        .execute(params![id_pkg, trigger])?;
    }

    for rich_dep in &pkg.rich_deps {
      let json = serde_json::to_string(rich_dep)?;
      self
        .conn
        .prepare_cached("INSERT INTO rich_deps (package_id, ast) VALUES (?1, ?2)")?
        .execute(params![id_pkg, json])?;
    }

    Ok(id_pkg)
  }

  /// Empties the database, deleting relationship rows before the
  /// packages they reference so the foreign keys are never violated.
  pub fn clear(&self) -> Result<()> {
    self.conn.execute_batch(
      "DELETE FROM dependencies;
       DELETE FROM provides;
       DELETE FROM install_if;
       DELETE FROM rich_deps;
       DELETE FROM packages;",
    )?;
    Ok(())
  }

  /// Records one dependency kind, preserving the group-and-alternative
  /// structure the author drew, labelled from the same `DepKind`
  /// vocabulary the read path uses so the two cannot disagree.
  fn deps_insert(&self, id_pkg: i64, kind: DepKind, deps: &[Dependency]) -> Result<()> {
    let Some(kind) = kind.as_dep_table_kind() else {
      anyhow::bail!("dependency kind {:?} has no dependencies-table representation", kind);
    };
    let mut stmt = self.conn.prepare_cached(
      "INSERT INTO dependencies (package_id, kind, group_id, position, name, version_constraint)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (group_idx, dep) in deps.iter().enumerate() {
      for (alt_idx, alt) in dep.alternatives.iter().enumerate() {
        stmt.execute(params![
          id_pkg,
          kind,
          group_idx as i32,
          alt_idx as i32,
          alt.name,
          alt.version_constraint
        ])?;
      }
    }
    Ok(())
  }
}

/// The id of the newest release of `name` under the version ordering,
/// so every fact is read from the release treated as current, or `None`
/// when the index holds no such name.
fn package_id_latest(conn: &Connection, name: &str) -> Result<Option<i64>> {
  let mut stmt = conn
    .prepare_cached("SELECT id FROM packages WHERE name = ?1 ORDER BY version COLLATE version_compare DESC LIMIT 1")?;
  let mut rows = stmt.query(params![name])?;
  match rows.next()? {
    Some(row) => Ok(Some(row.get(0)?)),
    None => Ok(None),
  }
}

// ---------------------------------------------------------------------------
// Packages — read view of the packages table
// ---------------------------------------------------------------------------

/// A borrow-scoped read view of the packages table.
pub struct Packages<'a>(&'a Connection);

impl Packages<'_> {
  /// One package's row, always the newest release under the version
  /// ordering, so the answer lines up with what resolution decided.
  /// `None` when the name is unknown.
  pub fn get(&self, name: &str) -> Result<Option<PackageRow>> {
    let mut stmt = self.0.prepare_cached(
      "SELECT name, version, description, filename, size, checksum, essential
       FROM packages WHERE name = ?1 ORDER BY version COLLATE version_compare DESC LIMIT 1",
    )?;
    let mut rows = stmt.query(params![name])?;
    match rows.next()? {
      Some(row) => Ok(Some(Self::row(row)?)),
      None => Ok(None),
    }
  }

  /// Whether a real package by this name exists — the cheapest check,
  /// and the first interpretation tried for a dependency name, before
  /// the rarer virtual-name and file-owner readings.
  pub fn exists(&self, name: &str) -> Result<bool> {
    let mut stmt = self
      .0
      .prepare_cached("SELECT 1 FROM packages WHERE name = ?1 LIMIT 1")?;
    let mut rows = stmt.query(params![name])?;
    Ok(rows.next()?.is_some())
  }

  /// The number of distinct packages, a cheap sanity check on the build
  /// and the scale a search or trace works against.
  pub fn count(&self) -> Result<usize> {
    let count: i64 = self
      .0
      .query_row("SELECT COUNT(DISTINCT name) FROM packages", [], |row| row.get(0))?;
    Ok(count as usize)
  }

  /// Matches a name pattern, collapsing each package to its newest release
  /// with a short description, so a search or trace seed yields one
  /// candidate per package rather than every historical release.
  pub fn glob(&self, pattern: &str) -> Result<Vec<PackageRow>> {
    // The inner subquery picks one id per name — the row with the
    // highest version under the `version_compare` collation.
    // `ROW_NUMBER() OVER (PARTITION BY name ORDER BY ...)` ranks rows
    // within each name; row 1 wins. The outer SELECT then filters the
    // glob and shapes the output. A `MAX(id)` would silently pick the
    // latest-walked repo's row, which is not the same as the
    // highest-version row when the index splits across base /
    // updates / security suites.
    let mut stmt = self.0.prepare_cached(
      "SELECT name, version, description, filename, size, checksum, essential
       FROM (
         SELECT
           id, name, version, description, filename, size, checksum, essential,
           ROW_NUMBER() OVER (PARTITION BY name ORDER BY version COLLATE version_compare DESC) AS rn
         FROM packages
       )
       WHERE rn = 1 AND glob_bash(name, ?1)
       ORDER BY name",
    )?;
    let rows = stmt.query_map(params![pattern], Self::row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
  }

  /// The packages the distribution designates as always-present, seeding
  /// the install set; empty where the distribution has no such notion.
  pub fn essential(&self) -> Result<Vec<String>> {
    let mut stmt = self
      .0
      .prepare_cached("SELECT DISTINCT name FROM packages WHERE essential = 1")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
  }

  /// Every package with install-if triggers, each with the trigger
  /// packages it waits on, for the install-if fixpoint to re-check each
  /// round.
  pub fn install_if(&self) -> Result<Vec<(String, Vec<String>)>> {
    let mut stmt = self.0.prepare_cached(
      "SELECT p.name, i.trigger
       FROM install_if i
       JOIN packages p ON p.id = i.package_id
       ORDER BY p.name",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;

    // Rows arrive sorted by package name, so consecutive rows for the same
    // package extend the entry just opened; a new name opens the next one.
    let mut result: Vec<(String, Vec<String>)> = Vec::new();

    for row in rows {
      let (name_pkg, trigger) = row?;
      match result.last_mut() {
        Some((current, triggers)) if *current == name_pkg => triggers.push(trigger),
        _ => result.push((name_pkg, vec![trigger])),
      }
    }

    Ok(result)
  }

  /// The packages that carry rich deps at all, narrowing the rich-dep
  /// fixpoint to candidates worth re-evaluating.
  pub fn with_rich_deps(&self) -> Result<Vec<String>> {
    let mut stmt = self
      .0
      .prepare_cached("SELECT DISTINCT p.name FROM rich_deps r JOIN packages p ON p.id = r.package_id")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
  }

  /// The rich deps of `name`'s newest release, as `RichDep` trees.
  pub fn rich_deps(&self, name: &str) -> Result<Vec<RichDep>> {
    let id_pkg = match package_id_latest(self.0, name)? {
      Some(id) => id,
      None => return Ok(vec![]),
    };

    let mut stmt = self
      .0
      .prepare_cached("SELECT ast FROM rich_deps WHERE package_id = ?1")?;
    let rows = stmt.query_map(params![id_pkg], |row| row.get::<_, String>(0))?;

    let mut result = Vec::new();
    for row in rows {
      let json = row?;
      let rich_dep: RichDep = serde_json::from_str(&json)?;
      result.push(rich_dep);
    }
    Ok(result)
  }

  /// Assembles one row into the `PackageRow` every reader returns, so the
  /// shape stays uniform whichever query surfaced the package.
  fn row(row: &rusqlite::Row) -> rusqlite::Result<PackageRow> {
    Ok(PackageRow {
      name: row.get(0)?,
      version: row.get(1)?,
      description: row.get(2)?,
      filename: row.get(3)?,
      size: row.get::<_, i64>(4)?.max(0) as u64,
      checksum: row.get(5)?,
      essential: row.get::<_, i32>(6)? != 0,
    })
  }
}

// ---------------------------------------------------------------------------
// Providers — read view of the provides table
// ---------------------------------------------------------------------------

/// A borrow-scoped read view of the provides table, for finding the
/// packages that provide a virtual name.
pub struct Providers<'a>(&'a Connection);

impl Providers<'_> {
  /// Every package providing `name`, each with the version it
  /// declares — the full list, so the resolver can try alternatives
  /// when the preferred one does not satisfy a constraint.
  pub fn of(&self, name: &str) -> Result<Vec<ProviderRow>> {
    let mut stmt = self.0.prepare_cached(
      "SELECT p.name, prov.version
       FROM provides prov
       JOIN packages p ON p.id = prov.package_id
       WHERE prov.name = ?1
       ORDER BY (p.priority IS NULL), p.priority, p.name",
    )?;
    let rows = stmt.query_map(params![name], |row| {
      Ok(ProviderRow {
        package_name: row.get(0)?,
        provides_version: row.get(1)?,
      })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
  }

  /// The one preferred package providing `name`, chosen by priority
  /// with ties broken by name so the pick is reproducible; `None` when
  /// nothing provides it.
  pub fn first(&self, name: &str) -> Result<Option<String>> {
    let mut stmt = self.0.prepare_cached(
      "SELECT p.name
       FROM provides prov
       JOIN packages p ON p.id = prov.package_id
       WHERE prov.name = ?1
       ORDER BY (p.priority IS NULL), p.priority, p.name LIMIT 1",
    )?;
    let mut rows = stmt.query(params![name])?;
    match rows.next()? {
      Some(row) => Ok(Some(row.get(0)?)),
      None => Ok(None),
    }
  }

  /// The version `provider_name` declares for `virtual_name` — often
  /// not its own version — so a version constraint is checked against
  /// what the provides entry states.
  pub fn version(&self, virtual_name: &str, provider_name: &str) -> Result<Option<String>> {
    let mut stmt = self.0.prepare_cached(
      "SELECT prov.version
       FROM provides prov
       JOIN packages p ON p.id = prov.package_id
       WHERE prov.name = ?1 AND p.name = ?2",
    )?;
    let mut rows = stmt.query(params![virtual_name, provider_name])?;
    match rows.next()? {
      Some(row) => Ok(row.get(0)?),
      None => Ok(None),
    }
  }

  /// The provides entries matching `pattern`, attributed to each
  /// package's newest release, so an entry carried across releases is
  /// reported once.
  pub fn glob(&self, pattern: &str) -> Result<Vec<ProvidesGlobRow>> {
    let mut stmt = self.0.prepare_cached(
      "WITH latest_pkgs AS (
         SELECT id, name, version, description FROM (
           SELECT
             id, name, version, description,
             ROW_NUMBER() OVER (PARTITION BY name ORDER BY version COLLATE version_compare DESC) AS rn
           FROM packages
         ) WHERE rn = 1
       )
       SELECT p.name, prov.name, p.version, p.description
       FROM provides prov
       JOIN latest_pkgs p ON p.id = prov.package_id
       WHERE glob_bash(prov.name, ?1)
       ORDER BY p.name, prov.name",
    )?;
    let rows = stmt.query_map(params![pattern], |row| {
      Ok(ProvidesGlobRow {
        package_name: row.get(0)?,
        provides_name: row.get(1)?,
        package_version: row.get(2)?,
        package_description: row.get(3)?,
      })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
  }
}

// ---------------------------------------------------------------------------
// Dependencies — read view of the dependencies table
// ---------------------------------------------------------------------------

/// A borrow-scoped read view of the dependencies table, restoring the
/// group-and-alternative structure from the flat stored rows.
pub struct Dependencies<'a>(&'a Connection);

impl Dependencies<'_> {
  /// One kind's dependencies for `name_pkg`'s newest release, restoring
  /// the group-and-alternative structure — the resolver satisfies every
  /// group and picks one alternative per group.
  pub fn of(&self, name_pkg: &str, kind: DepKind) -> Result<Vec<Dependency>> {
    // Only the column-backed kinds (Depends/Recommends/Suggests/Conflicts/
    // Breaks) have `dependencies` rows. The kinds whose `as_dep_table_kind()`
    // is `None` (InstallIf/RichIf/RichUnless) live in other tables, so a read
    // for one of them yields the empty set rather than a spurious query.
    let kind = match kind.as_dep_table_kind() {
      Some(kind) => kind,
      None => return Ok(vec![]),
    };

    let id_pkg = match package_id_latest(self.0, name_pkg)? {
      Some(id) => id,
      None => return Ok(vec![]),
    };

    // The `dependencies` table stores one row per alternative. For a
    // source line `Depends: a | b, c, d | e` the parser writes:
    //
    //   group_id  position  name
    //   --------  --------  ----
    //          0         0  a
    //          0         1  b
    //          1         0  c
    //          2         0  d
    //          2         1  e
    //
    // - `group_id` = which comma-separated slot the row belongs to.
    //   A new group_id starts at every `,` in the source, so one
    //   package has many group_ids — not one shared across all rows.
    // - `position` = which alternative within that slot, 0..N-1 in the
    //   left-to-right order the parser saw.
    //
    // ORDER BY group_id, position is a two-key sort: groups come back
    // in declaration order (0, then 1, then 2), and within each group
    // alternatives come back in source order (position 0, then 1, ...).
    // The result reproduces the table row-for-row, and the loop below
    // walks it, starting a new Dependency every time group_id changes —
    // rebuilding [[a, b], [c], [d, e]].
    let mut stmt = self.0.prepare_cached(
      "SELECT group_id, name, version_constraint
       FROM dependencies WHERE package_id = ?1 AND kind = ?2
       ORDER BY group_id, position",
    )?;

    let rows = stmt.query_map(params![id_pkg, kind], |row| {
      Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
    })?;

    let mut groups: Vec<Dependency> = Vec::new();
    let mut current_group: i32 = -1;

    // Walk the sorted rows, opening a new Dependency each time group_id
    // changes and appending the alternative otherwise (current_group
    // starts at -1, an id no row can have). Correct only because the
    // ORDER BY makes same-group rows contiguous and in source order.
    for row in rows {
      let (group_idx, dep_name, version_constraint) = row?;
      let spec = DepSpec {
        name: dep_name,
        version_constraint,
      };
      match groups.last_mut() {
        Some(group) if group_idx == current_group => group.alternatives.push(spec),
        _ => {
          groups.push(Dependency {
            alternatives: vec![spec],
          });
          current_group = group_idx;
        }
      }
    }

    Ok(groups)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::package::fixtures::{dep, dep_alts, dep_ver, pkg};
  use crate::package::{DepKind, DepSpec, Package, RichDep};
  use rusqlite::Connection;

  /// Builds an in-memory database with the empty schema, the default
  /// version-ordering collation, and the glob matcher registered. The
  /// production read path opens a stored copy; tests bypass the on-disk
  /// TTL-and-rebuild lifecycle and assert directly against the views.
  fn fresh_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    Index::schema(&conn).unwrap();
    conn
  }

  /// Assert that a RichDep is a Pkg leaf with the given name.
  fn assert_pkg(rd: &RichDep, name_expected: &str) {
    match rd {
      RichDep::Pkg(spec) => assert_eq!(spec.name, name_expected),
      other => panic!("Expected Pkg({name_expected}), got {:?}", other),
    }
  }

  #[test]
  fn package_crud_round_trip() {
    let conn = fresh_db();

    let p = Package {
      name: "bash".to_string(),
      version: "5.2.15-2+b10".to_string(),
      description: "The GNU Bourne Again SHell".to_string(),
      filename: "pool/main/b/bash/bash_5.2.deb".to_string(),
      size: 1234567,
      checksum: "abc123def456".to_string(),
      essential: true,
      ..pkg("bash", "5.2")
    };
    IndexWriter::new(&conn).insert(&p).unwrap();

    assert!(Packages(&conn).exists("bash").unwrap());
    assert_eq!(Packages(&conn).count().unwrap(), 1);

    let row = Packages(&conn).get("bash").unwrap().expect("bash not found");
    assert_eq!(row.name, "bash");
    assert_eq!(row.version, "5.2.15-2+b10");
    assert_eq!(row.description, "The GNU Bourne Again SHell");
    assert_eq!(row.filename, "pool/main/b/bash/bash_5.2.deb");
    assert_eq!(row.size, 1234567);
    assert_eq!(row.checksum, "abc123def456");
    assert!(row.essential);
  }

  #[test]
  fn dependencies_grouping_and_ordering() {
    let conn = fresh_db();

    let mut p = pkg("app", "1.0");
    p.depends = vec![
      dep_ver("libc6", ">= 2.36"),
      dep_alts(&[("debconf", Some(">= 0.5")), ("cdebconf", None)]),
      dep("bash"),
    ];
    IndexWriter::new(&conn).insert(&p).unwrap();

    let groups = Dependencies(&conn).of("app", DepKind::Depends).unwrap();
    assert_eq!(groups.len(), 3);

    assert_eq!(groups[0].alternatives.len(), 1);
    assert_eq!(groups[0].alternatives[0].name, "libc6");
    assert_eq!(groups[0].alternatives[0].version_constraint.as_deref(), Some(">= 2.36"));

    assert_eq!(groups[1].alternatives.len(), 2);
    assert_eq!(groups[1].alternatives[0].name, "debconf");
    assert_eq!(groups[1].alternatives[0].version_constraint.as_deref(), Some(">= 0.5"));
    assert_eq!(groups[1].alternatives[1].name, "cdebconf");
    assert!(groups[1].alternatives[1].version_constraint.is_none());

    assert_eq!(groups[2].alternatives.len(), 1);
    assert_eq!(groups[2].alternatives[0].name, "bash");
    assert!(groups[2].alternatives[0].version_constraint.is_none());
  }

  #[test]
  fn provides_round_trip() {
    let conn = fresh_db();

    let mut p = pkg("mawk", "1.3.4");
    p.provides = vec![
      DepSpec {
        name: "awk".to_string(),
        version_constraint: None,
      },
      DepSpec {
        name: "libfoo.so".to_string(),
        version_constraint: Some("1.0".to_string()),
      },
    ];
    IndexWriter::new(&conn).insert(&p).unwrap();

    let awk_providers = Providers(&conn).of("awk").unwrap();
    assert_eq!(awk_providers.len(), 1);
    assert_eq!(awk_providers[0].package_name, "mawk");
    assert!(awk_providers[0].provides_version.is_none());

    let lib_providers = Providers(&conn).of("libfoo.so").unwrap();
    assert_eq!(lib_providers.len(), 1);
    assert_eq!(lib_providers[0].provides_version.as_deref(), Some("1.0"));

    assert_eq!(Providers(&conn).first("awk").unwrap(), Some("mawk".to_string()));

    assert!(Providers(&conn).version("awk", "mawk").unwrap().is_none());
    assert_eq!(Providers(&conn).version("libfoo.so", "mawk").unwrap(), Some("1.0".to_string()));
  }

  #[test]
  fn providers_alphabetical_ordering() {
    let conn = fresh_db();

    for name in &["original-awk", "gawk", "mawk"] {
      let mut p = pkg(name, "1.0");
      p.provides = vec![DepSpec {
        name: "awk".to_string(),
        version_constraint: None,
      }];
      IndexWriter::new(&conn).insert(&p).unwrap();
    }

    let providers = Providers(&conn).of("awk").unwrap();
    let names: Vec<&str> = providers.iter().map(|p| p.package_name.as_str()).collect();
    assert_eq!(names, vec!["gawk", "mawk", "original-awk"]);

    assert_eq!(Providers(&conn).first("awk").unwrap(), Some("gawk".to_string()));
  }

  #[test]
  fn providers_priority_beats_alphabetical() {
    let conn = fresh_db();

    // `gawk` sorts first alphabetically but carries Priority: optional;
    // `mawk` carries Priority: required. The required provider must win
    // both the ordered list and the first-provider lookup. `original-awk`
    // has no priority and falls to the alphabetical tail.
    for (name, priority) in &[("original-awk", None), ("gawk", Some(3u8)), ("mawk", Some(0u8))] {
      let mut p = pkg(name, "1.0");
      p.priority = *priority;
      p.provides = vec![DepSpec {
        name: "awk".to_string(),
        version_constraint: None,
      }];
      IndexWriter::new(&conn).insert(&p).unwrap();
    }

    let providers = Providers(&conn).of("awk").unwrap();
    let names: Vec<&str> = providers.iter().map(|p| p.package_name.as_str()).collect();
    assert_eq!(names, vec!["mawk", "gawk", "original-awk"]);

    assert_eq!(Providers(&conn).first("awk").unwrap(), Some("mawk".to_string()));
  }

  #[test]
  fn provides_glob_returns_matching_rows_with_package_metadata() {
    let conn = fresh_db();

    let mut libssl = Package {
      description: "Secure Sockets Layer toolkit".to_string(),
      ..pkg("libssl3", "3.0.11-1")
    };
    libssl.provides = vec![
      DepSpec {
        name: "libssl.so.3".to_string(),
        version_constraint: None,
      },
      DepSpec {
        name: "libcrypto.so.3".to_string(),
        version_constraint: None,
      },
    ];
    IndexWriter::new(&conn).insert(&libssl).unwrap();

    let mut bash = pkg("bash", "5.2.15-2");
    bash.provides = vec![DepSpec {
      name: "sh".to_string(),
      version_constraint: None,
    }];
    IndexWriter::new(&conn).insert(&bash).unwrap();

    // libssl* matches both `libssl.so.3` and `libcrypto.so.3`? No —
    // glob is `libssl*` so only libssl.so.3.
    let rows = Providers(&conn).glob("libssl*").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].package_name, "libssl3");
    assert_eq!(rows[0].provides_name, "libssl.so.3");
    assert_eq!(rows[0].package_version, "3.0.11-1");
    assert_eq!(rows[0].package_description, "Secure Sockets Layer toolkit");

    // lib*.so.3 matches both libssl.so.3 and libcrypto.so.3 — same package.
    let rows = Providers(&conn).glob("lib*.so.3").unwrap();
    assert_eq!(rows.len(), 2);
    let names: Vec<&str> = rows.iter().map(|r| r.provides_name.as_str()).collect();
    assert!(names.contains(&"libssl.so.3"));
    assert!(names.contains(&"libcrypto.so.3"));
    for row in &rows {
      assert_eq!(row.package_name, "libssl3");
    }

    // Non-matching glob returns empty.
    let rows = Providers(&conn).glob("nothere*").unwrap();
    assert!(rows.is_empty());

    // Glob is case-insensitive (matches lower-case behaviour of package_glob).
    let rows = Providers(&conn).glob("LIBSSL*").unwrap();
    assert_eq!(rows.len(), 1);
  }

  #[test]
  fn install_if_round_trip() {
    let conn = fresh_db();

    let mut p = pkg("gdk-pixbuf-loader-svg", "2.58");
    p.install_if = vec!["gdk-pixbuf".to_string(), "librsvg".to_string()];
    IndexWriter::new(&conn).insert(&p).unwrap();

    let entries = Packages(&conn).install_if().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "gdk-pixbuf-loader-svg");
    assert_eq!(entries[0].1, vec!["gdk-pixbuf", "librsvg"]);
  }

  #[test]
  fn rich_deps_json_round_trip() {
    let conn = fresh_db();

    let simple_rd = RichDep::If {
      payload: Box::new(RichDep::Pkg(DepSpec {
        name: "appstream-data".to_string(),
        version_constraint: None,
      })),
      condition: Box::new(RichDep::Pkg(DepSpec {
        name: "PackageKit".to_string(),
        version_constraint: None,
      })),
    };

    let nested_rd = RichDep::If {
      payload: Box::new(RichDep::Or(
        Box::new(RichDep::Pkg(DepSpec {
          name: "A".to_string(),
          version_constraint: None,
        })),
        Box::new(RichDep::Pkg(DepSpec {
          name: "B".to_string(),
          version_constraint: None,
        })),
      )),
      condition: Box::new(RichDep::And(
        Box::new(RichDep::Pkg(DepSpec {
          name: "C".to_string(),
          version_constraint: None,
        })),
        Box::new(RichDep::Pkg(DepSpec {
          name: "D".to_string(),
          version_constraint: None,
        })),
      )),
    };

    let mut p = pkg("test-pkg", "1.0");
    p.rich_deps = vec![simple_rd, nested_rd];
    IndexWriter::new(&conn).insert(&p).unwrap();

    let read_back = Packages(&conn).rich_deps("test-pkg").unwrap();
    assert_eq!(read_back.len(), 2);

    let RichDep::If { payload, condition } = &read_back[0] else {
      panic!("Expected If, got {:?}", read_back[0]);
    };
    assert_pkg(payload, "appstream-data");
    assert_pkg(condition, "PackageKit");

    let RichDep::If { payload, condition } = &read_back[1] else {
      panic!("Expected If, got {:?}", read_back[1]);
    };
    let RichDep::Or(l, r) = payload.as_ref() else {
      panic!("Expected Or, got {:?}", payload);
    };
    assert_pkg(l, "A");
    assert_pkg(r, "B");
    let RichDep::And(l, r) = condition.as_ref() else {
      panic!("Expected And, got {:?}", condition);
    };
    assert_pkg(l, "C");
    assert_pkg(r, "D");
  }

  #[test]
  fn package_multiple_versions_latest_deps() {
    let conn = fresh_db();

    let mut p1 = pkg("bash", "5.0");
    p1.depends = vec![dep("libc6")];
    IndexWriter::new(&conn).insert(&p1).unwrap();

    let mut p2 = pkg("bash", "5.2");
    p2.depends = vec![dep("libc6"), dep("libtinfo6")];
    IndexWriter::new(&conn).insert(&p2).unwrap();

    assert_eq!(Packages(&conn).count().unwrap(), 1);

    let row = Packages(&conn).get("bash").unwrap().unwrap();
    assert_eq!(row.version, "5.2");

    let deps = Dependencies(&conn).of("bash", DepKind::Depends).unwrap();
    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0].alternatives[0].name, "libc6");
    assert_eq!(deps[1].alternatives[0].name, "libtinfo6");
  }

  #[test]
  fn multiple_versions_returns_latest() {
    let conn = fresh_db();

    IndexWriter::new(&conn).insert(&pkg("firefox", "128.0-1")).unwrap();
    IndexWriter::new(&conn).insert(&pkg("firefox", "127.0-1")).unwrap();
    IndexWriter::new(&conn).insert(&pkg("firefox", "129.0-1")).unwrap();

    assert_eq!(Packages(&conn).get("firefox").unwrap().unwrap().version, "129.0-1");
  }

  #[test]
  fn essential_packages_query() {
    let conn = fresh_db();

    let mut p1 = pkg("dash", "0.5");
    p1.essential = true;
    let mut p2 = pkg("coreutils", "9.1");
    p2.essential = true;
    let p3 = pkg("vim", "9.0");

    IndexWriter::new(&conn).insert(&p1).unwrap();
    IndexWriter::new(&conn).insert(&p2).unwrap();
    IndexWriter::new(&conn).insert(&p3).unwrap();

    let mut essential = Packages(&conn).essential().unwrap();
    essential.sort();
    assert_eq!(essential, vec!["coreutils", "dash"]);
  }

  #[test]
  fn search_glob_patterns() {
    let conn = fresh_db();

    IndexWriter::new(&conn).insert(&pkg("firefox-esr", "128.0")).unwrap();
    IndexWriter::new(&conn)
      .insert(&pkg("firefox-esr-l10n-de", "128.0"))
      .unwrap();
    IndexWriter::new(&conn).insert(&pkg("thunderbird", "115.0")).unwrap();
    IndexWriter::new(&conn).insert(&pkg("python3.11", "3.11.2")).unwrap();
    IndexWriter::new(&conn).insert(&pkg("python3.12", "3.12.1")).unwrap();

    let results = Packages(&conn).glob("firefox-esr").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "firefox-esr");

    let results = Packages(&conn).glob("*firefox*").unwrap();
    assert_eq!(results.len(), 2);

    let results = Packages(&conn).glob("firefox-esr-*").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "firefox-esr-l10n-de");

    let results = Packages(&conn).glob("python3.??").unwrap();
    assert_eq!(results.len(), 2);

    let results = Packages(&conn).glob("zzz-nonexistent*").unwrap();
    assert!(results.is_empty());
  }

  #[test]
  fn tables_clear_wipes_all() {
    let conn = fresh_db();

    let mut p = pkg("test", "1.0");
    p.depends = vec![dep("libc6")];
    p.provides = vec![DepSpec {
      name: "virt".to_string(),
      version_constraint: None,
    }];
    p.install_if = vec!["trigger".to_string()];
    p.rich_deps = vec![RichDep::Pkg(DepSpec {
      name: "x".to_string(),
      version_constraint: None,
    })];
    IndexWriter::new(&conn).insert(&p).unwrap();

    assert_eq!(Packages(&conn).count().unwrap(), 1);

    IndexWriter::new(&conn).clear().unwrap();

    assert_eq!(Packages(&conn).count().unwrap(), 0);
    assert!(Packages(&conn).glob("*").unwrap().is_empty());
    assert!(Packages(&conn).essential().unwrap().is_empty());
    assert!(Packages(&conn).install_if().unwrap().is_empty());
    assert!(Packages(&conn).with_rich_deps().unwrap().is_empty());
  }

  #[test]
  fn latest_version_queries_ignore_older() {
    let conn = fresh_db();

    let mut p = pkg("test", "1.0");
    p.depends = vec![dep("libc6")];
    p.recommends = vec![dep("rec")];
    p.provides = vec![DepSpec {
      name: "virt".to_string(),
      version_constraint: None,
    }];
    p.install_if = vec!["trigger".to_string()];
    p.rich_deps = vec![RichDep::Pkg(DepSpec {
      name: "x".to_string(),
      version_constraint: None,
    })];
    IndexWriter::new(&conn).insert(&p).unwrap();

    IndexWriter::new(&conn).insert(&pkg("test", "2.0")).unwrap();

    assert_eq!(Packages(&conn).get("test").unwrap().unwrap().version, "2.0");

    assert!(Dependencies(&conn).of("test", DepKind::Depends).unwrap().is_empty());
    assert!(Dependencies(&conn).of("test", DepKind::Recommends).unwrap().is_empty());

    assert!(Packages(&conn).rich_deps("test").unwrap().is_empty());
  }

  #[test]
  fn empty_database_queries() {
    let conn = fresh_db();

    assert!(Packages(&conn).get("anything").unwrap().is_none());
    assert!(!Packages(&conn).exists("anything").unwrap());
    assert_eq!(Packages(&conn).count().unwrap(), 0);
    assert!(Packages(&conn).glob("*").unwrap().is_empty());
    assert!(Packages(&conn).glob("foo").unwrap().is_empty());
    assert!(Packages(&conn).essential().unwrap().is_empty());
    assert!(Packages(&conn).install_if().unwrap().is_empty());
    assert!(Packages(&conn).with_rich_deps().unwrap().is_empty());
    assert!(Providers(&conn).of("anything").unwrap().is_empty());
    assert!(Providers(&conn).first("anything").unwrap().is_none());
    assert!(Providers(&conn).version("a", "b").unwrap().is_none());
    assert!(Dependencies(&conn).of("anything", DepKind::Depends).unwrap().is_empty());
    assert!(Packages(&conn).rich_deps("anything").unwrap().is_empty());
  }

  #[test]
  fn all_dependency_kinds_independent() {
    let conn = fresh_db();

    let p = Package {
      name: "test".to_string(),
      version: "1.0".to_string(),
      depends: vec![dep("dep-a")],
      recommends: vec![dep("rec-a")],
      suggests: vec![dep("sug-a")],
      conflicts: vec![dep("con-a")],
      breaks: vec![dep("brk-a")],
      ..pkg("test", "1.0")
    };
    IndexWriter::new(&conn).insert(&p).unwrap();

    let depends = Dependencies(&conn).of("test", DepKind::Depends).unwrap();
    let recommends = Dependencies(&conn).of("test", DepKind::Recommends).unwrap();
    let suggests = Dependencies(&conn).of("test", DepKind::Suggests).unwrap();
    let conflicts = Dependencies(&conn).of("test", DepKind::Conflicts).unwrap();
    let breaks = Dependencies(&conn).of("test", DepKind::Breaks).unwrap();

    assert_eq!(depends.len(), 1);
    assert_eq!(depends[0].alternatives[0].name, "dep-a");

    assert_eq!(recommends.len(), 1);
    assert_eq!(recommends[0].alternatives[0].name, "rec-a");

    assert_eq!(suggests.len(), 1);
    assert_eq!(suggests[0].alternatives[0].name, "sug-a");

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].alternatives[0].name, "con-a");

    assert_eq!(breaks.len(), 1);
    assert_eq!(breaks[0].alternatives[0].name, "brk-a");
  }
}
