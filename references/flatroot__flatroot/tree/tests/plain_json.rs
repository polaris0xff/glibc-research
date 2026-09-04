//! The Shout encoder (`flatroot::internal::shout`) is the shared, domain-neutral
//! way every read verb flattens a structured value into one dotted `KEY=VALUE`
//! line per leaf, lex-sorted by object key, indexed by array position, and
//! refusing anything that would break parseability. These tests drive the public
//! encoder directly for the invariants the unit tests only touch piecemeal, then
//! prove the same contract holds end-to-end through `remote list`'s plain/json
//! bijection (the one read verb that needs neither network nor `--from`, so it
//! runs offline).
//!
//! Everything asserted is derived from `src/internal/shout.rs` and SHOUT.md: the
//! reserved `.`/`=`/newline/carriage-return characters, the empty-key and
//! canonical-integer-key rejection, the multi-component scope, the lex-sort, and
//! the all-or-nothing render.

use assert_cmd::Command;
use flatroot::internal::shout;
use serde_json::json;
use tempfile::TempDir;

/// Render a value under a scope exactly as `Report::emit` does for the plain
/// form: the flat projection of the same value the JSON form serialises.
fn render(value: &serde_json::Value, scope: &str) -> String {
  shout::Document::from_value(value, scope).render().unwrap()
}

// ---------------------------------------------------------------------------
// One line per leaf, lex-sorted dotted keys (CORE-044)
// ---------------------------------------------------------------------------

// covers: CORE-044
#[test]
fn plain_emits_one_line_per_leaf_with_lex_sorted_keys() {
  // A value with keys in non-lex order, a nested object, and an array — all
  // leaves. Object keys must appear lex-sorted regardless of source order.
  let v = json!({
    "zulu": "z",
    "alpha": "a",
    "mike": { "yankee": "y", "bravo": "b" },
    "deps": ["one", "two"],
  });
  let out = render(&v, "");

  assert_eq!(
    out, "alpha=a\ndeps.0=one\ndeps.1=two\nmike.bravo=b\nmike.yankee=y\nzulu=z\n",
    "leaves must be one-line-per-leaf, dotted, lex-sorted by key"
  );
  assert_eq!(out.lines().count(), 6, "one line per leaf");
  for line in out.lines() {
    assert_eq!(line.matches('=').count(), 1, "each line has exactly one '=' separator: {line}");
  }
}

// ---------------------------------------------------------------------------
// Arrays indexed by position, in order (CORE-045)
// ---------------------------------------------------------------------------

// covers: CORE-045
#[test]
fn plain_indexes_arrays_by_position_in_order() {
  // Scalar array: indices count from 0 and follow array order, NOT lex.
  assert_eq!(
    render(&json!({ "deps": ["zlib", "acl", "mango"] }), ""),
    "deps.0=zlib\ndeps.1=acl\ndeps.2=mango\n",
    "scalar-array elements keep their order under positional indices"
  );

  // Array of objects: `entries.<n>.<field>` composition, array order.
  assert_eq!(
    render(&json!({ "entries": [{ "name": "bash" }, { "name": "acl" }] }), ""),
    "entries.0.name=bash\nentries.1.name=acl\n",
    "array-of-objects composes entries.<n>.name and preserves order"
  );

  // Bare array root with a scope: `trace.<n>.name`.
  assert_eq!(
    render(&json!([{ "name": "x" }, { "name": "y" }, { "name": "z" }]), "trace"),
    "trace.0.name=x\ntrace.1.name=y\ntrace.2.name=z\n",
    "bare array root indexes each entry under the scope"
  );
}

// ---------------------------------------------------------------------------
// Scope prefixes every line; a dotted scope is the full command path (CORE-047)
// ---------------------------------------------------------------------------

// covers: CORE-047
#[test]
fn plain_scope_prefixes_every_line_and_dots_are_components() {
  // A single scope prefixes every produced line.
  assert_eq!(render(&json!({ "name": "bash" }), "trace"), "trace.name=bash\n", "single scope prefixes the leaf");

  // A dotted scope is the producer's full command path (SHOUT §6): its
  // components join with the path separator and prefix every line.
  assert_eq!(render(&json!({ "bar": "baz" }), "trace.foo"), "trace.foo.bar=baz\n", "dotted scope is multi-component");

  // The scope prefix applies to every leaf of a multi-leaf document.
  assert_eq!(
    render(&json!({ "a": "1", "b": "2" }), "section"),
    "section.a=1\nsection.b=2\n",
    "scope prefixes every line"
  );
}

// ---------------------------------------------------------------------------
// Render rejects keys/leaves that break parseability (CORE-048)
// ---------------------------------------------------------------------------

// covers: CORE-048
#[test]
fn plain_rejects_unparseable_keys_and_leaves_at_render() {
  let cases: &[(serde_json::Value, &str)] = &[
    // Key containing the path separator '.'.
    (json!({ "python3.11": "ok" }), "contains '.'"),
    // Key containing the leaf separator '='.
    (json!({ "foo=bar": "ok" }), "contains '='"),
    // Empty key.
    (json!({ "": "ok" }), "empty"),
    // Canonical-integer key (indistinguishable from an array index).
    (json!({ "0": "ok" }), "canonical integer"),
    // Leaf value containing a newline (the reserved line separator).
    (json!({ "k": "line1\nline2" }), "newline"),
    // Leaf value containing a carriage return.
    (json!({ "k": "line1\rline2" }), "carriage return"),
  ];
  for (value, needle) in cases {
    let err = shout::Document::from_value(value, "s")
      .render()
      .err()
      .expect("expected an error");
    assert!(err.to_string().contains(needle), "expected error containing {needle:?}, got: {err}");
  }
}

// covers: CORE-048
#[test]
fn plain_render_is_all_or_nothing() {
  // A value the format cannot represent unambiguously is refused and nothing is
  // written — a broken document never reaches the consumer half-written.
  let v = json!({ "a": "ok", "k": "line1\nline2" });
  let mut buf: Vec<u8> = Vec::new();
  assert!(shout::value_emit(&mut buf, &v, "s").is_err(), "a value with a newline leaf must be refused");
  assert!(buf.is_empty(), "no partial output is written on a validation failure");
}

// ---------------------------------------------------------------------------
// Empty containers emit zero bytes (CORE-049)
// ---------------------------------------------------------------------------

// covers: CORE-049
#[test]
fn plain_empty_containers_emit_zero_bytes() {
  // A top-level empty array, a top-level empty object, an empty nested array,
  // and an empty nested object each contribute no lines; a null leaf emits none.
  assert_eq!(render(&json!([]), "x"), "", "empty array root is zero bytes");
  assert_eq!(render(&json!({}), "x"), "", "empty object root is zero bytes");
  assert_eq!(
    render(&json!({ "name": "bash", "deps": [] }), ""),
    "name=bash\n",
    "empty nested array contributes no lines"
  );
  assert_eq!(
    render(&json!({ "name": "bash", "meta": {} }), ""),
    "name=bash\n",
    "empty nested object contributes no lines"
  );
  assert_eq!(render(&json!({ "kept": "v", "skipped": null }), ""), "kept=v\n", "a null leaf emits no line");
}

// ---------------------------------------------------------------------------
// CLI-observable bijection: remote list plain vs json (CORE-050, CORE-044/045)
// ---------------------------------------------------------------------------
//
// `remote list` is the one read verb that needs neither network nor --from, so
// it exercises the plain<->json bijection offline. The plain form is a lossless
// mirror of the JSON: every leaf in JSON appears as a dotted plain line and
// every plain line maps back to a JSON leaf.

fn flatroot_cmd() -> (Command, TempDir) {
  let cache = TempDir::new().expect("tempdir for flatroot cache");
  let mut cmd = Command::cargo_bin("flatroot").unwrap();
  cmd.env("FLATROOT_CACHE_HOME", cache.path());
  (cmd, cache)
}

/// Flatten a JSON value into the same dotted KEY=VALUE leaf set the plain
/// encoder produces: object keys join with '.', array indices inline, scalar
/// leaves render as their string form, nulls emit no line, empty containers
/// emit nothing. This mirrors the encoder's documented rules so the two
/// encodings can be compared leaf-for-leaf.
fn json_to_plain_leaves(value: &serde_json::Value, prefix: &str, out: &mut Vec<String>) {
  match value {
    serde_json::Value::Null => {} // null leaf emits no line
    serde_json::Value::Bool(b) => out.push(format!("{prefix}={b}")),
    serde_json::Value::Number(n) => out.push(format!("{prefix}={n}")),
    serde_json::Value::String(s) => out.push(format!("{prefix}={s}")),
    serde_json::Value::Array(items) => {
      for (i, item) in items.iter().enumerate() {
        let next = if prefix.is_empty() {
          i.to_string()
        } else {
          format!("{prefix}.{i}")
        };
        json_to_plain_leaves(item, &next, out);
      }
    }
    serde_json::Value::Object(map) => {
      for (k, v) in map {
        let next = if prefix.is_empty() {
          k.clone()
        } else {
          format!("{prefix}.{k}")
        };
        json_to_plain_leaves(v, &next, out);
      }
    }
  }
}

// covers: CORE-050, CORE-044, CORE-045
#[test]
fn remote_list_plain_is_a_lossless_mirror_of_json() {
  let plain = {
    let (mut cmd, _cache) = flatroot_cmd();
    let out = cmd.args(["remote", "list"]).output().unwrap();
    assert!(out.status.success(), "remote list (plain) failed:\n{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
  };
  let json = {
    let (mut cmd, _cache) = flatroot_cmd();
    let out = cmd.args(["remote", "list", "--format", "json"]).output().unwrap();
    assert!(out.status.success(), "remote list (json) failed:\n{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
  };

  let parsed: serde_json::Value = serde_json::from_str(&json).expect("remote list --format json must be valid JSON");
  assert!(parsed.as_array().map(|a| !a.is_empty()).unwrap_or(false), "remote list must list at least one distro");

  // Derive the plain leaf set the JSON implies, and compare it to the actual
  // plain output line-for-line — the full bijection, not just the name leaf.
  // `remote list` renders a bare top-level array under the `remote.list`
  // command-path scope, so the plain form prefixes every line with
  // `remote.list.`; the JSON array carries no such wrapper. Deriving under the
  // same scope makes the two comparable.
  let mut derived: Vec<String> = Vec::new();
  json_to_plain_leaves(&parsed, "remote.list", &mut derived);
  derived.sort();

  let mut actual: Vec<String> = plain.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect();
  actual.sort();

  assert_eq!(
    actual,
    derived,
    "plain output must be a lossless leaf-for-leaf mirror of JSON\nonly in plain: {:?}\nonly in json-derived: {:?}",
    actual.iter().filter(|l| !derived.contains(l)).collect::<Vec<_>>(),
    derived.iter().filter(|l| !actual.contains(l)).collect::<Vec<_>>(),
  );

  // The array is indexed by position (CORE-045) and keys are lex-sorted within
  // each entry (CORE-044): every plain line is `remote.list.<n>.<key>=<value>`.
  for line in plain.lines().filter(|l| !l.is_empty()) {
    let key = line.split_once('=').expect("plain line has '='").0;
    let parts: Vec<&str> = key.split('.').collect();
    assert_eq!(parts[0], "remote", "every leaf sits under the 'remote.list' command-path scope: {line}");
    assert_eq!(parts[1], "list", "every leaf sits under the 'remote.list' command-path scope: {line}");
    assert!(parts[2].parse::<usize>().is_ok(), "array index must be numeric: {line}");
  }
}
