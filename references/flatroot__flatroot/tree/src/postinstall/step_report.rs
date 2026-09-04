//! Reports how an isolated post-install step turned out without halting
//! the pass: silent on success, terse with the step's own complaint on
//! failure.

use std::process::Output;

use anyhow::Result;
use indicatif::ProgressBar;

/// One place for the quiet-on-success, terse-on-failure reporting rule, so
/// every pass presents failures the same way and none ends the run.
pub struct StepReport;

impl StepReport {
  /// Reports one finished step above the live progress bar, naming its pass
  /// and the work — silent on success, terse on failure.
  pub fn print(pb: &ProgressBar, label: &str, name: &str, outcome: &Result<Output>) {
    for line in Self::lines(label, name, outcome) {
      pb.println(line);
    }
  }

  /// The lines `print` emits: none on success, a failure header plus up to
  /// five stderr lines when the step ran but rejected the work, one reason
  /// line when it could not be attempted.
  fn lines(label: &str, name: &str, outcome: &Result<Output>) -> Vec<String> {
    match outcome {
      Ok(o) if o.status.success() => Vec::new(),
      Ok(o) => {
        let mut out = vec![format!(
          "{}: {} ... failed (exit {})",
          label,
          name,
          o.status.code().unwrap_or(-1)
        )];
        let stderr = String::from_utf8_lossy(&o.stderr);
        if !stderr.is_empty() {
          for line in stderr.lines().take(5) {
            out.push(format!("    {}", line));
          }
        }
        out
      }
      Err(e) => vec![format!("{}: {} ... failed ({})", label, name, e)],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::process::ExitStatusExt;
  use std::process::ExitStatus;

  /// An `Output` with a chosen exit code and stderr, without spawning anything.
  /// On Unix the wait-status raw value is `code << 8` for a normal exit, so a
  /// zero code yields `status.success() == true` and a non-zero code surfaces
  /// via `status.code()`.
  fn output(code: i32, stderr: &str) -> Output {
    Output {
      status: ExitStatus::from_raw(code << 8),
      stdout: Vec::new(),
      stderr: stderr.as_bytes().to_vec(),
    }
  }

  // covers: a successful step is silent (quiet-on-success convention)
  #[test]
  fn success_emits_nothing() {
    assert!(StepReport::lines("ldconfig", "bash", &Ok(output(0, "ignored on success"))).is_empty());
  }

  // covers: a step that ran but rejected the work shows the failure header and
  // at most five lines of its own stderr.
  #[test]
  fn ran_but_failed_shows_header_and_capped_stderr() {
    let lines = StepReport::lines("scripts", "pkgfoo", &Ok(output(1, "one\ntwo\nthree\nfour\nfive\nsix")));
    assert_eq!(lines[0], "scripts: pkgfoo ... failed (exit 1)");
    assert_eq!(&lines[1..], &["    one", "    two", "    three", "    four", "    five"]);
    assert_eq!(lines.len(), 6, "header plus at most five stderr lines");
  }

  // covers: POST-047
  #[test]
  fn could_not_be_attempted_reports_the_error_and_keeps_going() {
    // A step whose run could not even be attempted surfaces as
    // "<label>: <name> ... failed (<error>)"; producing the line never panics
    // or halts, so the surrounding pass keeps moving.
    let lines = StepReport::lines("hooks", "pkgbar", &Err(anyhow::anyhow!("interpreter missing")));
    assert_eq!(lines, vec!["hooks: pkgbar ... failed (interpreter missing)".to_string()]);
  }
}
