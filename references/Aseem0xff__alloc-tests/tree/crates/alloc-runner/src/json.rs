//! A tiny JSON writer.
//!
//! The runner is copied into every benchmark image, so every dependency it
//! carries is a dependency the image build can fail on. It only ever *writes*
//! JSON, and the shapes are fixed and known here, so a 90-line emitter buys
//! back a dependency. Reading is the host orchestrator's job, where `serde` is
//! used properly.

use std::fmt::Write as _;

/// A JSON value built for output. Deliberately not a general-purpose parser.
#[derive(Debug, Clone)]
pub enum J {
    Null,
    Bool(bool),
    I(i64),
    U(u64),
    F(f64),
    S(String),
    A(Vec<J>),
    O(Vec<(String, J)>),
}

impl J {
    pub fn s(v: impl Into<String>) -> J {
        J::S(v.into())
    }
    pub fn obj(pairs: Vec<(&str, J)>) -> J {
        J::O(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
    pub fn arr(items: Vec<J>) -> J {
        J::A(items)
    }

    /// `f64` has no JSON representation for NaN or infinity. Emitting `null`
    /// keeps the document parseable and makes the absence explicit, which is
    /// the behaviour the validator downstream expects: a missing number is
    /// never read as a zero.
    fn write_f(out: &mut String, v: f64) {
        if v.is_finite() {
            // Enough digits to round-trip a nanosecond in a 1000-second run.
            let _ = write!(out, "{:.9}", v);
        } else {
            out.push_str("null");
        }
    }

    pub fn write(&self, out: &mut String) {
        match self {
            J::Null => out.push_str("null"),
            J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            J::I(v) => {
                let _ = write!(out, "{}", v);
            }
            J::U(v) => {
                let _ = write!(out, "{}", v);
            }
            J::F(v) => Self::write_f(out, *v),
            J::S(v) => escape(out, v),
            J::A(items) => {
                out.push('[');
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    it.write(out);
                }
                out.push(']');
            }
            J::O(pairs) => {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    escape(out, k);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    /// Render to a JSON string.
    ///
    /// Deliberately not `Display`/`ToString`: this is an encoder, and a `J`
    /// that accidentally got formatted with `{}` somewhere should not silently
    /// produce JSON.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }
}

fn escape(out: &mut String, v: &str) {
    out.push('"');
    for c in v.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
