//! Human-readable byte sizes — bytes, KiB, or MiB with a sensible
//! precision — so build progress reads at a glance rather than as a wall
//! of digits.

/// A raw byte count that renders itself (via `Display`) as a short
/// human-readable size.
pub struct ByteSize(pub u64);

impl std::fmt::Display for ByteSize {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let size = self.0;
    if size >= 1_048_576 {
      write!(f, "{:.1} MiB", size as f64 / 1_048_576.0)
    } else if size >= 1024 {
      write!(f, "{:.0} KiB", size as f64 / 1024.0)
    } else {
      write!(f, "{} B", size)
    }
  }
}
