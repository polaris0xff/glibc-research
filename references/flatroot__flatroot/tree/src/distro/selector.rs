//! `FromSelector`: the user's `<distro>:<release>` request, split once
//! into its two halves so every caller reads the same split.

use anyhow::Result;

/// The user's `<distro>:<release>` request split into its two halves.
pub struct FromSelector {
  /// The distro token before `:` (e.g. `debian`).
  prefix: String,
  /// The release tail after `:`, including any `@<date>`
  /// snapshot suffix.
  release: String,
}

impl FromSelector {
  /// Splits `spec` at the first `:`; a spec without one is an error
  /// showing the expected form.
  pub fn parse(spec: &str) -> Result<FromSelector> {
    let (prefix, release) = spec
      .split_once(':')
      .ok_or_else(|| anyhow::anyhow!("expected <distro>:<release>, got '{}'", spec))?;
    Ok(FromSelector {
      prefix: prefix.to_string(),
      release: release.to_string(),
    })
  }

  /// The distribution half of the request.
  pub fn prefix(&self) -> &str {
    &self.prefix
  }

  /// The release half, including any `@<date>` snapshot qualifier.
  pub fn release(&self) -> &str {
    &self.release
  }

  /// The distribution half alone, tolerating a missing `:` — `release
  /// list` takes a bare distribution name, which `parse` would reject.
  pub fn prefix_of(spec: &str) -> &str {
    spec.split_once(':').map(|(prefix, _)| prefix).unwrap_or(spec)
  }
}
