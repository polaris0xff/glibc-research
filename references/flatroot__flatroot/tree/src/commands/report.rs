//! `Report`: the one place a reading command's rows are printed, as
//! either plain Shout lines or JSON.

use anyhow::Result;

use flatroot::internal::shout;

use crate::parser::OutputFormat;

/// The shared printer every reading command emits its rows through.
pub struct Report;

impl Report {
  /// Prints `rows` in `format`, scoped under the command path. An empty
  /// `rows` prints nothing under `Plain` but still prints `[]` under
  /// `Json`, since a parser expects the empty collection spelled out.
  pub fn emit<T: serde::Serialize>(scope: &[&str], rows: &[T], format: OutputFormat) -> Result<()> {
    match format {
      OutputFormat::Plain => {
        let value = serde_json::to_value(rows)?;
        let stdout = std::io::stdout();
        shout::value_emit(&mut stdout.lock(), &value, &scope.join("."))?;
        Ok(())
      }
      OutputFormat::Json => {
        let s = serde_json::to_string_pretty(rows)?;
        println!("{s}");
        Ok(())
      }
    }
  }
}
