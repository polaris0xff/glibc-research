//! Owned file-descriptor primitives for coordinating a parent with a
//! child it forks into new namespaces: a handshake pipe and draining the
//! child's captured output.

use std::os::unix::io::{FromRawFd, RawFd};

use anyhow::{Context, Result, bail};
use nix::libc;

/// One owned end of a pipe, so each end's lifetime is explicit rather
/// than tracked as a loose integer.
pub struct Fd(pub RawFd);

impl Fd {
  /// Creates a one-way pipe and yields both ends, for a parent and forked
  /// child to hand-shake — one side writes "done", the other reads it.
  pub fn pipe() -> Result<(Fd, Fd)> {
    let mut fds = [0 as RawFd; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
      bail!("pipe() failed: {}", std::io::Error::last_os_error());
    }
    Ok((Fd(fds[0]), Fd(fds[1])))
  }

  /// Releases this end. Fire-and-forget: nothing useful remains if the
  /// close itself fails, and dropping the last writer signals EOF to the
  /// reader on the far side.
  pub fn close(self) {
    unsafe { libc::close(self.0) };
  }

  /// Reads everything the far side sent, to EOF, into one buffer. `label`
  /// names the stream in the error so a failed read points at which one.
  pub fn drain(self, label: &str) -> Result<Vec<u8>> {
    let mut file = unsafe { std::fs::File::from_raw_fd(self.0) };
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buf).with_context(|| format!("read sandbox {label} pipe"))?;
    // Dropping `file` closes the fd.
    Ok(buf)
  }

  /// The underlying fd, ownership retained, for syscalls that take raw
  /// descriptors (redirecting a child's streams).
  pub fn raw(&self) -> RawFd {
    self.0
  }
}
