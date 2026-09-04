//! Black-box CLI tests for `flatroot ... query` — the raw-SQL escape hatch over
//! the cached package index. These cover the SQL-source routing (stdin, the `-`
//! sentinel, a file path), the SQLite value-type mapping, the NULL-drop-in-plain
//! vs null-preserving-json contract, the empty-result and unnamed-column edge
//! cases, the DML zero-row read, the env-var format fallback, and proof that the
//! `glob_bash` scalar function and `version_compare` collation are registered on
//! the query read connection.
//!
//! Each invocation isolates its cache via a fresh `FLATROOT_CACHE_HOME` TempDir.

#![allow(dead_code)]

use std::collections::BTreeMap;

use assert_cmd::Command;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run `flatroot --from debian:bookworm query [extra...]` with `sql` on stdin,
/// returning stdout (panicking with stderr on non-zero exit).
fn query_stdin_ok(sql: &str, extra: &[&str]) -> String {
  let cache = TempDir::new().unwrap();
  let mut args: Vec<&str> = vec!["--from", "debian:bookworm", "query"];
  args.extend_from_slice(extra);
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(&args)
    .write_stdin(sql)
    .output()
    .unwrap();
  assert!(
    out.status.success(),
    "query (sql={:?}, extra={:?}) failed:\nstderr:\n{}",
    sql,
    extra,
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Like `query_stdin_ok` but with one extra environment variable set.
fn query_stdin_ok_env(sql: &str, env_key: &str, env_val: &str, extra: &[&str]) -> String {
  let cache = TempDir::new().unwrap();
  let mut args: Vec<&str> = vec!["--from", "debian:bookworm", "query"];
  args.extend_from_slice(extra);
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .env(env_key, env_val)
    .args(&args)
    .write_stdin(sql)
    .output()
    .unwrap();
  assert!(
    out.status.success(),
    "query (sql={:?}, env {}={}) failed:\nstderr:\n{}",
    sql,
    env_key,
    env_val,
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Parse the plain dotted `query.<i>.cells.<j>.column=<name>` /
/// `query.<i>.cells.<j>.value=<value>` stream into one map per indexed row,
/// keyed by column name. A NULL cell emits only its `column` line, so its
/// column is simply absent from the map, preserving the plain "absence means
/// NULL" reading. Every structural dot precedes the `=`, so a column name or a
/// value that itself contains a dot is parsed correctly.
fn parse_plain_rows(stdout: &str) -> Vec<BTreeMap<String, String>> {
  // Gather each cell's column name and (optional) value by its (row, cell)
  // position, then fold the present values into one map per row.
  let mut columns: BTreeMap<(usize, usize), String> = BTreeMap::new();
  let mut values: BTreeMap<(usize, usize), String> = BTreeMap::new();
  for line in stdout.lines() {
    let rest = match line.strip_prefix("query.") {
      Some(r) => r,
      None => continue,
    };
    let (row_str, after_row) = match rest.split_once('.') {
      Some(p) => p,
      None => continue,
    };
    let after_cells = match after_row.strip_prefix("cells.") {
      Some(a) => a,
      None => continue,
    };
    let (cell_str, field_kv) = match after_cells.split_once('.') {
      Some(p) => p,
      None => continue,
    };
    let (field, value) = match field_kv.split_once('=') {
      Some(p) => p,
      None => continue,
    };
    let (row_idx, cell_idx): (usize, usize) = match (row_str.parse(), cell_str.parse()) {
      (Ok(r), Ok(c)) => (r, c),
      _ => continue,
    };
    match field {
      "column" => {
        columns.insert((row_idx, cell_idx), value.to_string());
      }
      "value" => {
        values.insert((row_idx, cell_idx), value.to_string());
      }
      _ => continue,
    }
  }
  let row_count = columns.keys().map(|(r, _)| *r + 1).max().unwrap_or(0);
  let mut rows: Vec<BTreeMap<String, String>> = vec![BTreeMap::new(); row_count];
  for ((row_idx, cell_idx), column) in &columns {
    if let Some(value) = values.get(&(*row_idx, *cell_idx)) {
      rows[*row_idx].insert(column.clone(), value.clone());
    }
  }
  rows
}

/// Collapse one json result row `{ "cells": [ {column, value}, … ] }` into a
/// column→value map so a test can look a column up by name. Duplicate columns
/// keep the last; the raw cell list is what the structured form actually
/// guarantees, but name lookup is enough for these assertions.
fn json_row_cells(row: &serde_json::Value) -> BTreeMap<String, serde_json::Value> {
  let mut map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
  for cell in row["cells"].as_array().expect("a query json row carries a cells array") {
    let column = cell["column"].as_str().expect("cell.column is a string").to_string();
    map.insert(column, cell["value"].clone());
  }
  map
}

// ---------------------------------------------------------------------------
// query from stdin (no FILE) — multi-column plain shape (IDX-026)
// ---------------------------------------------------------------------------

// covers: IDX-026
#[test]
fn query_stdin_multi_column_plain_rows() {
  // A two-column SELECT over stdin renders one record per row as an ordered list
  // of cells: query.<i>.cells.<j>.column / .value. Cells follow SELECT order, so
  // `name` is cell 0 and `version` is cell 1.
  let stdout = query_stdin_ok("SELECT name, version FROM packages ORDER BY name LIMIT 2", &[]);
  let rows = parse_plain_rows(&stdout);
  assert_eq!(rows.len(), 2, "two-row LIMIT must produce two records:\n{}", stdout);
  for (i, row) in rows.iter().enumerate() {
    assert!(row.contains_key("name"), "row {} must carry name: {:?}", i, row);
    assert!(row.contains_key("version"), "row {} must carry version: {:?}", i, row);
  }
  // Cell order follows the SELECT, not a key sort: name is cell 0, version cell 1.
  assert!(
    stdout.contains("query.0.cells.0.column=name"),
    "the first cell must be the first SELECT column (name):\n{}",
    stdout
  );
  assert!(
    stdout.contains("query.0.cells.1.column=version"),
    "the second cell must be the second SELECT column (version):\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// query from stdin via explicit '-' sentinel (IDX-027)
// ---------------------------------------------------------------------------

// covers: IDX-027
#[test]
fn query_dash_sentinel_reads_stdin() {
  // `query -` treats the positional as the stdin sentinel; the piped SQL runs.
  let stdout = query_stdin_ok("SELECT COUNT(*) AS n FROM packages", &["-"]);
  let rows = parse_plain_rows(&stdout);
  assert_eq!(rows.len(), 1, "count query must yield a single row:\n{}", stdout);
  let n: usize = rows[0]["n"].trim().parse().expect("n must be an integer");
  assert!(n > 60000, "`query -` must run the stdin SQL against the index, got {} packages", n);
}

// ---------------------------------------------------------------------------
// query from a SQL file path (IDX-028)
// ---------------------------------------------------------------------------

// covers: IDX-028
#[test]
fn query_reads_from_file_path() {
  // `query <FILE>` reads the SQL via read_to_string and runs it.
  let dir = TempDir::new().unwrap();
  let path_file_sql = dir.path().join("q.sql");
  std::fs::write(&path_file_sql, "SELECT name FROM packages WHERE name='bash' LIMIT 1").unwrap();
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "query", path_file_sql.to_str().unwrap()])
    .output()
    .unwrap();
  assert!(out.status.success(), "query from file failed:\n{}", String::from_utf8_lossy(&out.stderr));
  let stdout = String::from_utf8_lossy(&out.stdout);
  let rows = parse_plain_rows(&stdout);
  assert_eq!(rows.len(), 1, "file SQL must run and return the bash row:\n{}", stdout);
  assert_eq!(rows[0].get("name").map(|s| s.as_str()), Some("bash"), "file SQL result: {:?}", rows[0]);
}

// ---------------------------------------------------------------------------
// query with invalid SQL fails at prepare (IDX-030)
// ---------------------------------------------------------------------------

// covers: IDX-030
#[test]
fn query_invalid_sql_fails_at_prepare() {
  // A misspelled keyword fails `prepare` → "Failed to prepare SQL: <sql>", no rows.
  let bad = "SELEKT * FROM packages";
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "query"])
    .write_stdin(bad)
    .output()
    .unwrap();
  assert!(!out.status.success(), "invalid SQL must fail");
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains(&format!("Failed to prepare SQL: {}", bad)),
    "expected the prepare-failure context naming the SQL:\n{}",
    stderr
  );
  // No result rows emitted.
  assert!(
    out.stdout.is_empty(),
    "a prepare failure must emit no rows, got: {:?}",
    String::from_utf8_lossy(&out.stdout)
  );
}

// ---------------------------------------------------------------------------
// query NULL columns: dropped in plain, preserved as null in json (IDX-031)
// ---------------------------------------------------------------------------

// covers: IDX-031
#[test]
fn query_null_dropped_in_plain_preserved_in_json() {
  // A SELECT with a literal NULL column: plain omits the NULL line; json renders
  // a literal null. (A self-contained SELECT avoids depending on a distro's NULL
  // priority rows so the contract is exercised on every source identically.)
  let sql = "SELECT 'present' AS a, NULL AS b";

  let plain = query_stdin_ok(sql, &[]);
  let rows = parse_plain_rows(&plain);
  assert_eq!(rows.len(), 1, "one row expected:\n{}", plain);
  assert_eq!(rows[0].get("a").map(|s| s.as_str()), Some("present"), "plain must keep the present column");
  assert!(!rows[0].contains_key("b"), "plain must drop the NULL column entirely:\n{}", plain);
  // The NULL column still names itself (its column line) but emits no value line.
  assert!(plain.lines().any(|l| l == "query.0.cells.1.column=b"), "the NULL column still names itself:\n{}", plain);
  assert!(
    !plain.lines().any(|l| l.starts_with("query.0.cells.1.value=")),
    "the NULL column emits no value line in plain:\n{}",
    plain
  );

  let json = query_stdin_ok(sql, &["--format", "json"]);
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("query json must parse");
  let arr = parsed.as_array().expect("query json must be an array");
  let row = json_row_cells(&arr[0]);
  assert_eq!(row.get("a").and_then(|v| v.as_str()), Some("present"), "json must keep the present column: {}", json);
  assert!(
    row.get("b").map(|v| v.is_null()).unwrap_or(false),
    "json must preserve the NULL column as literal null: {}",
    json
  );
}

// ---------------------------------------------------------------------------
// query SQLite value-type mapping: Integer/Real/Text/Blob/Null (IDX-032)
// ---------------------------------------------------------------------------

// covers: IDX-032
#[test]
fn query_maps_all_sqlite_value_types() {
  // Integer, Real, Text rendered as-is; Blob → "<blob>"; Null omitted (plain) /
  // null (json).
  let sql = "SELECT 1 AS i, 2.5 AS r, 'x' AS t, x'00' AS b, NULL AS n";

  let plain = query_stdin_ok(sql, &[]);
  let rows = parse_plain_rows(&plain);
  assert_eq!(rows.len(), 1, "one row expected:\n{}", plain);
  let row = &rows[0];
  assert_eq!(row.get("i").map(|s| s.as_str()), Some("1"), "Integer must render as its decimal: {:?}", row);
  assert_eq!(row.get("r").map(|s| s.as_str()), Some("2.5"), "Real must render via to_string: {:?}", row);
  assert_eq!(row.get("t").map(|s| s.as_str()), Some("x"), "Text must render verbatim: {:?}", row);
  assert_eq!(row.get("b").map(|s| s.as_str()), Some("<blob>"), "Blob must render as the '<blob>' sentinel: {:?}", row);
  assert!(!row.contains_key("n"), "Null must be omitted from plain: {:?}", row);

  let json = query_stdin_ok(sql, &["--format", "json"]);
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("query json must parse");
  let obj = json_row_cells(&parsed.as_array().unwrap()[0]);
  // Plain renders every cell as a string, so json mirrors strings here too;
  // the Null cell is the literal null in json.
  assert_eq!(obj.get("i").and_then(|v| v.as_str()), Some("1"), "json i: {}", json);
  assert_eq!(obj.get("r").and_then(|v| v.as_str()), Some("2.5"), "json r: {}", json);
  assert_eq!(obj.get("t").and_then(|v| v.as_str()), Some("x"), "json t: {}", json);
  assert_eq!(obj.get("b").and_then(|v| v.as_str()), Some("<blob>"), "json b: {}", json);
  assert!(obj.get("n").map(|v| v.is_null()).unwrap_or(false), "json n must be literal null: {}", json);
}

// ---------------------------------------------------------------------------
// query empty result set: zero bytes plain / '[]' json (IDX-033)
// ---------------------------------------------------------------------------

// covers: IDX-033
#[test]
fn query_empty_result_formats() {
  // A WHERE that matches nothing: plain emits zero bytes; json emits "[]".
  let sql = "SELECT name FROM packages WHERE name='zzz-nope-xyz-no-such'";
  let plain = query_stdin_ok(sql, &[]);
  assert!(plain.is_empty(), "empty query result plain must be zero bytes, got {:?}", plain);

  let json = query_stdin_ok(sql, &["--format", "json"]);
  assert_eq!(json, "[]\n", "empty query result json must be exactly '[]\\n', got {:?}", json);
}

// ---------------------------------------------------------------------------
// query plain↔json bijection for multi-column rows, nulls kept in json (IDX-034)
// ---------------------------------------------------------------------------

// covers: IDX-034
#[test]
fn query_multi_column_plain_json_bijection() {
  // A multi-column SELECT (including a column that is sometimes NULL): every
  // non-null plain cell appears in json, and json additionally preserves nulls.
  let sql = "SELECT name, version, priority FROM packages ORDER BY name LIMIT 5";
  let plain = query_stdin_ok(sql, &[]);
  let json = query_stdin_ok(sql, &["--format", "json"]);

  let plain_rows = parse_plain_rows(&plain);
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("query json must parse");
  let arr = parsed.as_array().expect("query json must be an array");
  assert_eq!(plain_rows.len(), arr.len(), "row counts must agree:\nplain:\n{}\njson:\n{}", plain, json);

  for (i, (p_row, j_val)) in plain_rows.iter().zip(arr.iter()).enumerate() {
    let j_obj = json_row_cells(j_val);
    // Every json column whose value is non-null appears identically in plain.
    for (k, v) in &j_obj {
      if v.is_null() {
        // null in json ⇒ absent in plain
        assert!(!p_row.contains_key(k), "row {}: NULL column {} must be absent from plain: {:?}", i, k, p_row);
      } else {
        let jstr = v.as_str().expect("non-null query cells are strings");
        assert_eq!(
          p_row.get(k).map(|s| s.as_str()),
          Some(jstr),
          "row {}: column {} must match between plain and json",
          i,
          k
        );
      }
    }
    // Every plain column appears (non-null) in json.
    for (k, pv) in p_row {
      assert_eq!(
        j_obj.get(k).and_then(|v| v.as_str()),
        Some(pv.as_str()),
        "row {}: plain column {} must be in json",
        i,
        k
      );
    }
  }
}

// ---------------------------------------------------------------------------
// query format env fallback FLATROOT_ARG_QUERY_FORMAT (IDX-035)
// ---------------------------------------------------------------------------

// covers: IDX-035
#[test]
fn query_format_env_fallback_json() {
  // FLATROOT_ARG_QUERY_FORMAT=json makes query emit json with no --format flag.
  let json =
    query_stdin_ok_env("SELECT name FROM packages ORDER BY name LIMIT 1", "FLATROOT_ARG_QUERY_FORMAT", "json", &[]);
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("env-driven json must parse");
  let arr = parsed
    .as_array()
    .expect("FLATROOT_ARG_QUERY_FORMAT=json must produce a json array");
  assert_eq!(arr.len(), 1, "LIMIT 1 json must have one row:\n{}", json);
  assert!(json_row_cells(&arr[0]).contains_key("name"), "row must carry the name column: {}", json);
}

// ---------------------------------------------------------------------------
// query unaliased expression column: its expression text is the column name,
// carried as a cell value (IDX-036)
// ---------------------------------------------------------------------------

// covers: IDX-036
#[test]
fn query_unaliased_expression_column_is_named_by_its_expression_text() {
  // An unaliased expression column (`SELECT 1+1`) does not crash. SQLite names
  // such a column after its source text, `1+1`, and that name is data: it rides
  // in the cell's `column` value, never in an output key. The `?` fallback in
  // query.rs is a defensive guard for the InvalidColumnIndex / non-UTF-8 cases
  // that `column_name(0..count)` never hits on a normally-prepared statement.
  let stdout = query_stdin_ok("SELECT 1+1", &[]);
  assert!(stdout.contains("cells.0.column=1+1"), "the expression text must be the cell's column name:\n{}", stdout);
  let rows = parse_plain_rows(&stdout);
  assert_eq!(rows.len(), 1, "one row expected:\n{}", stdout);
  let row = &rows[0];
  assert_eq!(row.len(), 1, "the unaliased expression yields exactly one column:\n{}", stdout);
  let (key, value) = row.iter().next().unwrap();
  assert_eq!(value, "2", "the unaliased expression must compute 1+1 = 2:\n{}", stdout);
  assert_eq!(key, "1+1", "the column is named by its expression text:\n{}", stdout);
}

// ---------------------------------------------------------------------------
// query dotted column name renders as a cell value
// ---------------------------------------------------------------------------

#[test]
fn query_dotted_column_name_renders_as_value() {
  // An unaliased aggregate over a qualified column names the result column with a
  // dot (`max(p.size)`). The column name is data, carried in a cell value, so the
  // dot is just a character with no bearing on the line's structure.
  let stdout = query_stdin_ok("SELECT max(p.size) FROM packages p", &[]);
  assert!(
    stdout.contains("cells.0.column=max(p.size)"),
    "the dotted column name must appear verbatim as a cell value:\n{}",
    stdout
  );
  let rows = parse_plain_rows(&stdout);
  assert_eq!(rows.len(), 1, "the aggregate yields exactly one row:\n{}", stdout);
  let value = rows[0].get("max(p.size)").expect("the dotted column must be present");
  assert!(value.parse::<u64>().is_ok(), "max(size) must be an integer, got {:?}", value);
}

// ---------------------------------------------------------------------------
// query a non-SELECT (DML) behaves as a zero-row read (IDX-037)
// ---------------------------------------------------------------------------

// covers: IDX-037
#[test]
fn query_dml_statement_is_zero_row_read() {
  // A DELETE with a never-true predicate prepares to a zero-column statement;
  // stepping yields no rows. The command succeeds and emits nothing — the
  // read-only escape hatch never mutates the cached catalogue.
  let cache = TempDir::new().unwrap();
  let out = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args(["--from", "debian:bookworm", "query"])
    .write_stdin("DELETE FROM packages WHERE 0")
    .output()
    .unwrap();
  assert!(out.status.success(), "DML query must succeed as a zero-row read:\n{}", String::from_utf8_lossy(&out.stderr));
  assert!(
    out.stdout.is_empty(),
    "a DML statement must produce no result rows, got: {:?}",
    String::from_utf8_lossy(&out.stdout)
  );
}

// ---------------------------------------------------------------------------
// query exercises the registered glob_bash scalar function (IDX-038)
// ---------------------------------------------------------------------------

// covers: IDX-038
#[test]
fn query_glob_bash_function_is_registered_on_read_connection() {
  // Calling glob_bash() from user SQL proves it is registered on the query read
  // connection, and that it is case-insensitive (LIBSSL* matches libssl*).
  let stdout = query_stdin_ok("SELECT DISTINCT name FROM packages WHERE glob_bash(name, 'LIBSSL3') LIMIT 5", &[]);
  let rows = parse_plain_rows(&stdout);
  assert!(
    rows
      .iter()
      .any(|r| r.get("name").map(|n| n == "libssl3").unwrap_or(false)),
    "glob_bash must be callable from query SQL and case-insensitively match libssl3:\n{}",
    stdout
  );
}

// ---------------------------------------------------------------------------
// query exercises the version_compare collation on the read connection (IDX-039)
// ---------------------------------------------------------------------------

// covers: IDX-039
#[test]
fn query_version_compare_collation_is_registered_on_read_connection() {
  // ORDER BY version COLLATE version_compare must order versions natively (dpkg
  // here): the highest libc6 release sorts first. The collation must be present
  // on the query connection or this statement would fail to prepare/run.
  let stdout = query_stdin_ok(
    "SELECT version FROM packages WHERE name='libc6' ORDER BY version COLLATE version_compare DESC LIMIT 1",
    &[],
  );
  let rows = parse_plain_rows(&stdout);
  assert_eq!(rows.len(), 1, "expected the single newest libc6 version:\n{}", stdout);
  // Compare the collation-selected newest against the matcher's own newest pick.
  let newest_via_search = {
    let cache = TempDir::new().unwrap();
    let out = Command::cargo_bin("flatroot")
      .unwrap()
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args(["--from", "debian:bookworm", "search", "--format", "json", "libc6"])
      .output()
      .unwrap();
    assert!(out.status.success(), "search libc6 failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    parsed
      .as_array()
      .unwrap()
      .iter()
      .find(|e| e["name"] == "libc6")
      .and_then(|e| e["version"].as_str())
      .expect("search must report a libc6 version")
      .to_string()
  };
  assert_eq!(
    rows[0].get("version").map(|s| s.as_str()),
    Some(newest_via_search.as_str()),
    "COLLATE version_compare must select the same newest libc6 the matcher reports:\n{}",
    stdout
  );
}
