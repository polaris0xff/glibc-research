//! Shout — the workspace's line-oriented "Shell Output" format: every scalar in
//! a result tree on its own line, named by its full dotted path and set equal to
//! its value, so downstream shell tooling needs only `cut`/`grep`/`while read`
//! and never a structural parser. The encoder knows nothing of any one tool's
//! domain — it is handed an already-serialised `serde_json::Value` and flattens
//! it — so both flatroot and flatcapsule render their `--format plain` output
//! through this one path, the flat projection of the very same data their
//! `--format json` emits. The full contract is SHOUT.md at the workspace root.
//!
//! VERBATIM SHARED FILE — keep byte-identical with the sibling copy. Both live at
//!   flatroot/flatroot/src/internal/shout.rs
//!   flatcapsule/flatcapsule/src/internal/shout.rs
//! A change to one must be mirrored to the other (the same cross-project sync
//! invariant as flatcapsule `manifest.rs` <-> flatimage `db/capsule.hpp`).

use std::io::Write;

use anyhow::{Result, bail};
use serde_json::Value;

/// A finished Shout document: a serialised value tree together with the command
/// path its lines hang under. Built from a `serde_json::Value` so the same value
/// that feeds the JSON form feeds this one, keeping the two renderings faithful
/// mirrors by construction rather than by hand. Rendering validates and is
/// all-or-nothing: a value the format cannot represent unambiguously is refused
/// and nothing is written, never a half-formed document (SHOUT §9).
pub struct Document<'v> {
  /// Command/sub-command tokens prepended to every line (SHOUT §6): the scope
  /// `analyze.gpu` becomes the components `["analyze", "gpu"]`. Each is a key
  /// like any other and is validated when the document is rendered.
  scope: Vec<String>,
  /// The value tree to flatten. Borrowed, never owned: the caller already holds
  /// it — it is the same value their JSON form serialises.
  value: &'v Value,
}

impl<'v> Document<'v> {
  /// Builds a document from a value and a dotted scope string. The scope is the
  /// producer's full command path (SHOUT §6); it is split on the path separator
  /// into its component tokens here, with an empty scope meaning no prefix.
  /// Validation is deferred to rendering, so this itself cannot fail.
  pub fn from_value(value: &'v Value, scope: &str) -> Self {
    let scope = if scope.is_empty() {
      Vec::new()
    } else {
      scope.split('.').map(str::to_string).collect()
    };
    Self { scope, value }
  }

  /// Renders the whole document to a string, or reports the first value that
  /// could not be represented unambiguously. Materialises in memory, so the
  /// result is always a complete, valid document or an error — never a partial
  /// one.
  pub fn render(&self) -> Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    self.emit(&mut buf)?;
    Ok(String::from_utf8(buf)?)
  }

  /// Renders the document to an output stream. Because a broken document must
  /// never reach a consumer half-written, the lines are built and validated in
  /// full before a single byte is handed to `out`: on any validation failure
  /// `out` is left untouched and the first error is returned (SHOUT §9).
  pub fn render_to<W: Write>(&self, out: &mut W) -> Result<()> {
    let mut buf: Vec<u8> = Vec::new();
    self.emit(&mut buf)?;
    out.write_all(&buf)?;
    Ok(())
  }

  /// Validates the scope components and descends the value tree, accumulating
  /// each line into `buf`. Kept separate from the public entry points so both
  /// the string and the streaming form share one all-or-nothing build.
  fn emit(&self, buf: &mut Vec<u8>) -> Result<()> {
    let mut path: Vec<String> = Vec::with_capacity(self.scope.len() + 4);
    for component in &self.scope {
      key_validate(component)?;
      path.push(component.clone());
    }
    value_walk(self.value, &mut path, buf)
  }
}

/// Renders a value to a stream under a dotted scope in one call — the shorthand
/// every reporting path uses, equivalent to
/// `Document::from_value(value, scope).render_to(out)`. All-or-nothing, exactly
/// as `Document::render_to`.
pub fn value_emit<W: Write>(out: &mut W, value: &Value, scope: &str) -> Result<()> {
  Document::from_value(value, scope).render_to(out)
}

/// Depth-first descent producing one line per scalar leaf. An object contributes
/// each member under `<path>.<key>`, its keys emitted in lexicographic order so
/// the output is stable whatever order the value was built in (SHOUT §8) and
/// regardless of how `serde_json` happens to store the map. An array contributes
/// each element under `<path>.<index>` in positional order, so a null element
/// still consumes its index while emitting no line. A null leaf emits nothing; a
/// string, number, or boolean emits its single line.
fn value_walk(value: &Value, path: &mut Vec<String>, buf: &mut Vec<u8>) -> Result<()> {
  match value {
    Value::Object(map) => {
      let mut entries: Vec<(&String, &Value)> = map.iter().collect();
      entries.sort_by(|a, b| a.0.cmp(b.0));
      for (key, child) in entries {
        key_validate(key)?;
        path.push(key.clone());
        value_walk(child, path, buf)?;
        path.pop();
      }
      Ok(())
    }
    Value::Array(items) => {
      for (index, child) in items.iter().enumerate() {
        path.push(index.to_string());
        value_walk(child, path, buf)?;
        path.pop();
      }
      Ok(())
    }
    Value::String(s) => leaf_emit(path, s, buf),
    Value::Number(n) => leaf_emit(path, &n.to_string(), buf),
    Value::Bool(b) => leaf_emit(path, if *b { "true" } else { "false" }, buf),
    Value::Null => Ok(()),
  }
}

/// Writes one `<path>=<value>` line after checking the value can occupy a single
/// line. The path is already validated component by component as it was built.
fn leaf_emit(path: &[String], value: &str, buf: &mut Vec<u8>) -> Result<()> {
  leaf_validate(value)?;
  let joined = path.join(".");
  writeln!(buf, "{joined}={value}")?;
  Ok(())
}

/// Refuses a key (object member name or scope component) that would make a line
/// ambiguous: empty, carrying the path or leaf separator, or a canonical
/// non-negative integer a reader could not tell from an array index (SHOUT §9).
/// A leading-zero form such as `01` is unambiguous and allowed. Keys are fixed
/// structural labels by contract (SHOUT §5), so this only ever fires on a
/// programming error, never on runtime data.
fn key_validate(key: &str) -> Result<()> {
  if key.is_empty() {
    bail!("shout key cannot be empty");
  }
  if key.contains('.') {
    bail!("shout key '{key}' contains '.', reserved as the path separator");
  }
  if key.contains('=') {
    bail!("shout key '{key}' contains '=', reserved as the leaf separator");
  }
  let canonical_integer = key.bytes().all(|b| b.is_ascii_digit()) && (key == "0" || !key.starts_with('0'));
  if canonical_integer {
    bail!("shout key '{key}' is a canonical integer, indistinguishable from an array index");
  }
  Ok(())
}

/// Refuses a leaf value that would not stay on one line. A line feed would split
/// one value into several, and a stray carriage return would corrupt the
/// one-value-one-line invariant for line-based consumers, so both are barred
/// (SHOUT §4, §9). This is the only check that guards genuinely variable data.
fn leaf_validate(value: &str) -> Result<()> {
  if value.contains('\n') {
    bail!("shout leaf value contains a newline, reserved as the line separator");
  }
  if value.contains('\r') {
    bail!("shout leaf value contains a carriage return, barred from leaf values");
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn render(value: &Value, scope: &str) -> String {
    Document::from_value(value, scope).render().unwrap()
  }

  #[test]
  fn object_keys_emit_in_lexicographic_order() {
    let v = json!({ "zebra": "z", "apple": "a", "mango": "m" });
    assert_eq!(render(&v, "x"), "x.apple=a\nx.mango=m\nx.zebra=z\n");
  }

  #[test]
  fn array_elements_emit_in_positional_order() {
    assert_eq!(render(&json!(["a", "b", "c"]), "x"), "x.0=a\nx.1=b\nx.2=c\n");
  }

  #[test]
  fn nested_object_and_array_compose() {
    assert_eq!(render(&json!({ "outer": { "inner": [1, 2] } }), "p"), "p.outer.inner.0=1\np.outer.inner.1=2\n");
  }

  #[test]
  fn null_leaf_emits_no_line() {
    assert_eq!(render(&json!({ "kept": "v", "skipped": null }), ""), "kept=v\n");
  }

  #[test]
  fn null_array_element_still_consumes_its_index() {
    // The null at index 1 emits nothing, but "c" stays at index 2 (positional).
    assert_eq!(render(&json!(["a", null, "c"]), "x"), "x.0=a\nx.2=c\n");
  }

  #[test]
  fn empty_array_emits_nothing() {
    assert_eq!(render(&json!([]), "x"), "");
  }

  #[test]
  fn empty_object_emits_nothing() {
    assert_eq!(render(&json!({}), "x"), "");
  }

  #[test]
  fn empty_string_leaf_emits_a_bare_line() {
    assert_eq!(render(&json!({ "k": "" }), "s"), "s.k=\n");
  }

  #[test]
  fn boolean_and_integer_leaves_render_canonically() {
    assert_eq!(render(&json!({ "b": true, "n": 42 }), "x"), "x.b=true\nx.n=42\n");
  }

  #[test]
  fn multi_component_scope_prefixes_every_line() {
    assert_eq!(render(&json!({ "k": "v" }), "analyze.gpu"), "analyze.gpu.k=v\n");
  }

  #[test]
  fn top_level_null_is_an_empty_document() {
    assert_eq!(render(&json!(null), "s"), "");
  }

  #[test]
  fn key_with_dot_is_refused() {
    let err = Document::from_value(&json!({ "a.b": "v" }), "s").render().unwrap_err();
    assert!(err.to_string().contains("'.'"), "{err}");
  }

  #[test]
  fn key_with_equals_is_refused() {
    let err = Document::from_value(&json!({ "a=b": "v" }), "s").render().unwrap_err();
    assert!(err.to_string().contains("'='"), "{err}");
  }

  #[test]
  fn canonical_integer_key_is_refused() {
    let err = Document::from_value(&json!({ "0": "v" }), "s").render().unwrap_err();
    assert!(err.to_string().contains("canonical integer"), "{err}");
  }

  #[test]
  fn leading_zero_key_is_allowed() {
    assert_eq!(render(&json!({ "01": "v" }), "s"), "s.01=v\n");
  }

  #[test]
  fn carriage_return_value_is_refused() {
    let err = Document::from_value(&json!({ "k": "a\rb" }), "s").render().unwrap_err();
    assert!(err.to_string().contains("carriage return"), "{err}");
  }

  #[test]
  fn newline_value_is_refused_and_writes_nothing() {
    // All-or-nothing: a poisoned document leaves the output untouched (SHOUT §9).
    let v = json!({ "a": "ok", "k": "line1\nline2" });
    let mut buf: Vec<u8> = Vec::new();
    assert!(value_emit(&mut buf, &v, "s").is_err());
    assert!(buf.is_empty(), "no partial output");
  }

  #[test]
  fn scope_with_dot_splits_into_validated_components() {
    assert_eq!(render(&json!({ "k": "v" }), "a.b"), "a.b.k=v\n");
  }
}
