//! The `query` command: runs a free-form SQL query against a source's
//! cached package index, for auditing or debugging beyond the built-in
//! lookups.

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use flatroot::arch::Arch;

use crate::commands::report::Report;
use crate::commands::session::Session;

/// Reads the SQL text from `file`, or from stdin when `file` is absent
/// or `-`.
pub fn sql_read(file: Option<PathBuf>) -> Result<String> {
  match file {
    None => stdin_read_all(),
    Some(path) if path == Path::new("-") => stdin_read_all(),
    Some(path) => {
      std::fs::read_to_string(&path).with_context(|| format!("Failed to read SQL file: {}", path.display()))
    }
  }
}

fn stdin_read_all() -> Result<String> {
  let mut buf = String::new();
  std::io::stdin()
    .read_to_string(&mut buf)
    .context("Failed to read SQL from stdin")?;
  Ok(buf)
}

/// One result cell: its column name and value. The column name is whatever
/// the user typed — carried as data, never a key, since it may hold dots,
/// `=`, be empty, or repeat. `None` is a SQL NULL, kept distinct from an
/// empty value.
#[derive(serde::Serialize)]
struct QueryCell {
  /// The column's name exactly as the query named it — data, not a key.
  column: String,
  /// The cell's value, or `None` for a SQL NULL.
  value: Option<String>,
}

/// One result row: its cells in SELECT order. Each column is its own
/// cell, so two columns sharing a name stay distinct.
#[derive(serde::Serialize)]
struct QueryRow {
  cells: Vec<QueryCell>,
}

/// Opens the package index, runs `sql` against it, and prints every
/// result row in the chosen format; NULL cells are preserved.
pub fn run(
  session: &Session,
  remote_str: &str,
  arch: Arch,
  sql: &str,
  format: crate::parser::OutputFormat,
) -> Result<()> {
  let ctx = session.context_open_blocking(remote_str, arch)?;

  let mut stmt = ctx
    .index
    .connection()
    .prepare(sql)
    .with_context(|| format!("Failed to prepare SQL: {}", sql))?;
  let column_count = stmt.column_count();
  let column_names: Vec<String> = (0..column_count)
    .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
    .collect();

  let mut rows_data: Vec<QueryRow> = Vec::new();
  let mut rows = stmt.query([]).context("Failed to execute query")?;
  while let Some(row) = rows.next()? {
    let mut cells: Vec<QueryCell> = Vec::new();
    for i in 0..column_count {
      let value: rusqlite::types::Value = row.get(i)?;
      let cell_value = match value {
        rusqlite::types::Value::Null => None,
        rusqlite::types::Value::Integer(n) => Some(n.to_string()),
        rusqlite::types::Value::Real(f) => Some(f.to_string()),
        rusqlite::types::Value::Text(s) => Some(s),
        rusqlite::types::Value::Blob(_) => Some("<blob>".to_string()),
      };
      cells.push(QueryCell {
        column: column_names[i].clone(),
        value: cell_value,
      });
    }
    rows_data.push(QueryRow { cells });
  }

  Report::emit(&["query"], &rows_data, format)
}

#[cfg(test)]
mod tests {
  use super::{QueryCell, QueryRow};
  use flatroot::internal::shout;

  /// Build one cell from a column name and an optional value.
  fn cell(column: &str, value: Option<&str>) -> QueryCell {
    QueryCell {
      column: column.to_string(),
      value: value.map(str::to_string),
    }
  }

  /// Renders rows exactly as `Report::emit` does for the plain format,
  /// scoped under `query`.
  fn render_plain(rows: &[QueryRow]) -> String {
    let value = serde_json::to_value(rows).unwrap();
    shout::Document::from_value(&value, "query").render().unwrap()
  }

  #[test]
  fn plain_row_emits_cells_in_order() {
    let rows = vec![QueryRow {
      cells: vec![cell("name", Some("bash")), cell("version", Some("5.2"))],
    }];
    assert_eq!(
      render_plain(&rows),
      "query.0.cells.0.column=name\nquery.0.cells.0.value=bash\nquery.0.cells.1.column=version\nquery.0.cells.1.value=5.2\n"
    );
  }

  #[test]
  fn dotted_column_name_is_a_value_not_a_key() {
    // An un-aliased aggregate names a column with a dot; the dotted name is a
    // cell value, so it renders verbatim with no bearing on the line's structure.
    let rows = vec![QueryRow {
      cells: vec![cell("group_concat(i.trigger)", Some("docs,7zip"))],
    }];
    assert_eq!(
      render_plain(&rows),
      "query.0.cells.0.column=group_concat(i.trigger)\nquery.0.cells.0.value=docs,7zip\n"
    );
  }

  #[test]
  fn duplicate_column_names_are_preserved() {
    // `SELECT 1 AS x, 2 AS x` — two columns share the name `x`; each is its own
    // cell, so both are kept.
    let rows = vec![QueryRow {
      cells: vec![cell("x", Some("1")), cell("x", Some("2"))],
    }];
    assert_eq!(
      render_plain(&rows),
      "query.0.cells.0.column=x\nquery.0.cells.0.value=1\nquery.0.cells.1.column=x\nquery.0.cells.1.value=2\n"
    );
  }

  #[test]
  fn column_order_follows_select_not_lexical() {
    let rows = vec![QueryRow {
      cells: vec![cell("version", Some("5.2")), cell("name", Some("bash"))],
    }];
    let out = render_plain(&rows);
    let pos_version = out.find("column=version").unwrap();
    let pos_name = out.find("column=name").unwrap();
    assert!(pos_version < pos_name, "version must precede name (SELECT order):\n{out}");
  }

  #[test]
  fn null_cell_omits_value_line_in_plain() {
    let rows = vec![QueryRow {
      cells: vec![cell("priority", None)],
    }];
    assert_eq!(render_plain(&rows), "query.0.cells.0.column=priority\n");
  }

  #[test]
  fn empty_alias_column_is_renderable() {
    // `SELECT 1 AS ""` — an empty column name is data in a cell value, which
    // renders fine.
    let rows = vec![QueryRow {
      cells: vec![cell("", Some("1"))],
    }];
    assert_eq!(render_plain(&rows), "query.0.cells.0.column=\nquery.0.cells.0.value=1\n");
  }

  #[test]
  fn null_cell_is_explicit_in_json() {
    let rows = vec![QueryRow {
      cells: vec![cell("priority", None)],
    }];
    let v: serde_json::Value = serde_json::to_value(&rows).unwrap();
    assert_eq!(v[0]["cells"][0]["column"], "priority");
    assert!(v[0]["cells"][0]["value"].is_null());
  }

  #[test]
  fn json_mirrors_plain_shape() {
    let rows = vec![QueryRow {
      cells: vec![cell("name", Some("bash")), cell("version", Some("5.2"))],
    }];
    let v: serde_json::Value = serde_json::to_value(&rows).unwrap();
    assert_eq!(v[0]["cells"][0]["column"], "name");
    assert_eq!(v[0]["cells"][0]["value"], "bash");
    assert_eq!(v[0]["cells"][1]["column"], "version");
    assert_eq!(v[0]["cells"][1]["value"], "5.2");
  }
}
