//! `RunVoice`: the one place progress and warnings are printed, so
//! output looks the same everywhere and one verbosity setting governs
//! the whole run.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

/// The run's user-facing output: the shared verbosity setting and the
/// progress displays every stage uses.
pub struct RunVoice;

static VERBOSE: AtomicBool = AtomicBool::new(false);

impl RunVoice {
  /// Sets the run's verbosity once at startup, shared by every stage.
  pub fn verbose_set(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
  }

  /// Reads the shared verbosity setting.
  pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
  }

  /// Prints a warning to stderr, but only when verbose mode is on.
  pub fn warn(msg: impl AsRef<str>) {
    if Self::is_verbose() {
      eprintln!("  warning: {}", msg.as_ref());
    }
  }

  /// An animated display for work with no measurable total (fetching
  /// an index), so a long pause is not mistaken for a hang.
  pub fn spinner(msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
      ProgressStyle::default_spinner()
        .template("{spinner} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(msg.into());
    pb
  }

  /// A bounded progress bar for work whose total is known, so the user
  /// can gauge how far along the build is.
  pub fn bar(total: u64, msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new(total);
    // Width is the digit count of `total` so `{pos}` and `{len}` both pad to
    // the same fixed prefix width — the bar's left edge stops moving as
    // `pos` ticks across digit boundaries (1 → 10 → 100). The variable-width
    // `{msg}` (package name) goes to the end so it can grow without nudging
    // the bar.
    let width = total.to_string().len();
    let template = format!("[{{pos:>{w}}}/{{len:>{w}}}] [{{bar:30}}] {{msg}}", w = width);
    pb.set_style(
      ProgressStyle::default_bar()
        .template(&template)
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("=> "),
    );
    pb.set_message(msg.into());
    pb
  }

  /// A running count for open-ended discovery (the dependency
  /// closure), where no total exists to show a fraction of.
  pub fn counter(msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
      ProgressStyle::default_spinner()
        .template("{spinner} [{pos:>4} discovered] {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(msg.into());
    pb
  }

  /// Clears a stage's live display and prints one final line recording
  /// it as done, so the next stage's display never overlaps it.
  pub fn finish_ok(bar: &ProgressBar, msg: impl Into<String>) {
    let msg = msg.into();
    bar.finish_and_clear();
    eprintln!("{}", msg);
  }
}
