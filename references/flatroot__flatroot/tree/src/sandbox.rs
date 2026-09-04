//! Runs commands inside a rootfs mounted as `/` in unprivileged user
//! and mount namespaces, so install scripts written for a real root run
//! without real privilege.

use std::ffi::CString;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use nix::libc;

use nix::mount::{MntFlags, MsFlags, mount, umount2};
use nix::sched::{CloneFlags, unshare};
use nix::unistd::{ForkResult, Pid, chdir, fork, getgid, getuid, pivot_root};

use crate::internal::fd::Fd;
use crate::internal::fs::Fs;

/// The parent's ends of the four one-way pipes a sandboxed job uses:
/// the two handshakes (child-ready after unshare, mappings-done back)
/// and the two output captures. Paired with `ChildPipes` so each side
/// of the fork closes the ends it does not own in one call.
struct ParentPipes {
  ready_read: Fd,
  maps_write: Fd,
  stdout_read: Fd,
  stderr_read: Fd,
}

/// The child's ends of the same four pipes.
struct ChildPipes {
  ready_write: Fd,
  maps_read: Fd,
  stdout_write: Fd,
  stderr_write: Fd,
}

impl ParentPipes {
  fn close(self) {
    self.ready_read.close();
    self.maps_write.close();
    self.stdout_read.close();
    self.stderr_read.close();
  }
}

impl ChildPipes {
  fn close(self) {
    self.ready_write.close();
    self.maps_read.close();
    self.stdout_write.close();
    self.stderr_write.close();
  }
}

/// The four pipes, created together and split into their parent and
/// child halves so a fork hands each process exactly the ends it owns.
fn pipes_create() -> Result<(ParentPipes, ChildPipes)> {
  let (ready_read, ready_write) = Fd::pipe().context("child_ready pipe")?;
  let (maps_read, maps_write) = Fd::pipe().context("maps_done pipe")?;
  let (stdout_read, stdout_write) = Fd::pipe().context("stdout pipe")?;
  let (stderr_read, stderr_write) = Fd::pipe().context("stderr pipe")?;
  Ok((
    ParentPipes {
      ready_read,
      maps_write,
      stdout_read,
      stderr_read,
    },
    ChildPipes {
      ready_write,
      maps_read,
      stdout_write,
      stderr_write,
    },
  ))
}

/// One sandbox over a rootfs: each `run` mounts the rootfs as `/` in
/// fresh namespaces and executes one command inside it.
pub struct Sandbox {
  root: PathBuf,
}

impl Sandbox {
  /// Binds the sandbox to the rootfs at `root`.
  pub fn new(root: &Path) -> Sandbox {
    Sandbox {
      root: root.to_path_buf(),
    }
  }

  /// Checks up front that unprivileged user namespaces work, by actually
  /// entering one rather than trusting a setting, so a host that forbids
  /// them fails cleanly with guidance instead of dying mid-install.
  pub fn available() -> Result<()> {
    let (read_fd, write_fd) = Fd::pipe()?;

    match unsafe { fork() } {
      Ok(ForkResult::Child) => {
        read_fd.close();
        let result: u8 = match unshare(CloneFlags::CLONE_NEWUSER) {
          Ok(()) => 0,
          Err(_) => 1,
        };
        unsafe { libc::write(write_fd.raw(), &result as *const u8 as *const libc::c_void, 1) };
        write_fd.close();
        std::process::exit(0);
      }
      Ok(ForkResult::Parent { child }) => {
        write_fd.close();

        let mut buf = [0u8; 1];
        let n = unsafe { libc::read(read_fd.raw(), buf.as_mut_ptr() as *mut libc::c_void, 1) };
        read_fd.close();

        if let Err(e) = nix::sys::wait::waitpid(child, None) {
          eprintln!("warning: waitpid on namespace probe failed: {}", e);
        }

        if n == 1 && buf[0] == 0 {
          return Ok(());
        }
        bail!("unshare(CLONE_NEWUSER) failed — unprivileged user namespaces are not available")
      }
      Err(e) => bail!("fork() failed: {}", e),
    }
  }

  /// Runs one command inside the rootfs, unprivileged, capturing its
  /// output. A non-zero exit is returned in the `Output` rather than as
  /// an error; the caller decides how much a failure matters.
  pub fn run(&self, cmd: &[&str], env: &[(&str, &str)]) -> Result<std::process::Output> {
    if cmd.is_empty() {
      bail!("Sandbox::run called with empty command");
    }

    let (pipes_parent, pipes_child) = pipes_create()?;
    let cmd_owned: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
    let env_owned: Vec<(String, String)> = env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();

    match unsafe { fork() } {
      Ok(ForkResult::Child) => {
        pipes_parent.close();
        self.child_run(pipes_child, cmd_owned, env_owned)
      }
      Ok(ForkResult::Parent { child }) => {
        pipes_child.close();
        self.parent_collect(child, pipes_parent)
      }
      Err(e) => bail!("fork() failed: {}", e),
    }
  }

  /// The forked child's path from fork to exec: wire the capture pipes,
  /// enter fresh user and mount namespaces, wait for the parent to
  /// write the UID/GID mappings, set up the mounts, then exec the
  /// command. Never returns — each failure exits with the conventional
  /// cannot-execute code the parent reports onward.
  fn child_run(&self, pipes: ChildPipes, cmd: Vec<String>, env: Vec<(String, String)>) -> ! {
    unsafe {
      libc::dup2(pipes.stdout_write.raw(), 1);
      libc::dup2(pipes.stderr_write.raw(), 2);
    }
    pipes.stdout_write.close();
    pipes.stderr_write.close();
    Fd(0).close();

    if let Err(e) = unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS) {
      eprintln!("unshare failed: {}", e);
      std::process::exit(126);
    }

    // Signal the parent that unshare is done, then wait for it to write the
    // UID/GID mappings before touching anything identity-dependent.
    unsafe { libc::write(pipes.ready_write.raw(), &0u8 as *const u8 as *const libc::c_void, 1) };
    pipes.ready_write.close();
    let mut buf = [0u8; 1];
    unsafe { libc::read(pipes.maps_read.raw(), buf.as_mut_ptr() as *mut libc::c_void, 1) };
    pipes.maps_read.close();

    if let Err(e) = self.mounts_setup() {
      eprintln!("mount setup failed: {}", e);
      std::process::exit(126);
    }

    Self::environment_replace(&env);
    Self::command_exec(&cmd)
  }

  /// Replace the whole environment with the job's own variables.
  /// Safety: runs in the forked child — single-threaded, nothing else reads
  /// the environment concurrently.
  fn environment_replace(env: &[(String, String)]) {
    unsafe {
      for (key, _) in std::env::vars() {
        std::env::remove_var(&key);
      }
      for (key, value) in env {
        std::env::set_var(key, value);
      }
    }
  }

  /// exec the job's command; never returns. A command or argument containing
  /// an interior NUL byte can never be exec'd, and is reported as exactly
  /// that rather than silently substituted with a different program.
  fn command_exec(cmd: &[String]) -> ! {
    let c_prog = match CString::new(cmd[0].as_str()) {
      Ok(prog) => prog,
      Err(_) => {
        eprintln!("exec failed: command '{}' contains an interior NUL byte", cmd[0]);
        std::process::exit(127);
      }
    };
    let c_args: Vec<CString> = match cmd.iter().map(|a| CString::new(a.as_str())).collect() {
      Ok(args) => args,
      Err(_) => {
        eprintln!("exec failed: an argument of '{}' contains an interior NUL byte", cmd[0]);
        std::process::exit(127);
      }
    };
    let _ = nix::unistd::execv(&c_prog, &c_args);
    eprintln!("exec failed: {}", cmd[0]);
    std::process::exit(127);
  }

  /// The parent's half of one job: writes the child's UID/GID mappings
  /// to complete the handshake, drains the output pipes, and turns the
  /// wait status into a `std::process::Output`.
  fn parent_collect(&self, child: Pid, pipes: ParentPipes) -> Result<std::process::Output> {
    let mut buf = [0u8; 1];
    unsafe { libc::read(pipes.ready_read.raw(), buf.as_mut_ptr() as *mut libc::c_void, 1) };
    pipes.ready_read.close();

    self.id_mappings_write(child, getuid().as_raw(), getgid().as_raw())?;
    // Closing the maps pipe is the "mappings are ready" signal.
    pipes.maps_write.close();

    let stdout = pipes.stdout_read.drain("stdout")?;
    let stderr = pipes.stderr_read.drain("stderr")?;
    let status = nix::sys::wait::waitpid(child, None).context("waitpid")?;

    let exit_code = match status {
      nix::sys::wait::WaitStatus::Exited(_, code) => code,
      nix::sys::wait::WaitStatus::Signaled(_, sig, _) => 128 + sig as i32,
      _ => 1,
    };
    Ok(std::process::Output {
      status: std::process::ExitStatus::from_raw(exit_code << 8),
      stdout,
      stderr,
    })
  }

  /// Maps the ordinary user who launched the build to root inside the
  /// namespace only, so package setup believes it has the rights it
  /// expects while none of that authority reaches the real host.
  fn id_mappings_write(&self, child: Pid, uid: u32, gid: u32) -> Result<()> {
    let pid = child.as_raw();

    std::fs::write(format!("/proc/{}/setgroups", pid), "deny").context("write setgroups")?;
    std::fs::write(format!("/proc/{}/uid_map", pid), format!("0 {} 1\n", uid)).context("write uid_map")?;
    std::fs::write(format!("/proc/{}/gid_map", pid), format!("0 {} 1\n", gid)).context("write gid_map")?;

    Ok(())
  }

  /// Sets up the mounts: pivots the rootfs into `/` — private to this
  /// mount namespace, so the host's view is untouched — and mounts the
  /// `/tmp`, `/proc`, and minimal `/dev` the scripts expect.
  fn mounts_setup(&self) -> Result<()> {
    let root = self.root.as_path();
    let root_str = root
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("rootfs path is not valid UTF-8"))?;

    // Make all mounts private
    mount::<str, str, str, str>(None, "/", None, MsFlags::MS_REC | MsFlags::MS_PRIVATE, None)
      .context("make mounts private")?;

    // Bind-mount rootfs onto itself (pivot_root requires it to be a mount point)
    mount::<str, str, str, str>(Some(root_str), root_str, None, MsFlags::MS_BIND | MsFlags::MS_REC, None)
      .context("bind-mount rootfs")?;

    // Mount /dev as tmpfs BEFORE pivot_root, then bind-mount host device nodes
    self.dev_setup().context("setup /dev")?;

    // Bind-mount host /proc into the rootfs.
    // Mounting a fresh procfs requires CLONE_NEWPID (and a double fork),
    // but bind-mounting the host /proc is sufficient for our scripts —
    // they only need /proc/self/fd and basic proc access.
    let proc_dir = root.join("proc");
    std::fs::create_dir_all(&proc_dir).with_context(|| format!("create mount target {}", proc_dir.display()))?;
    mount::<str, Path, str, str>(Some("/proc"), &proc_dir, None, MsFlags::MS_BIND | MsFlags::MS_REC, None)
      .context("bind-mount /proc")?;

    // Mount /tmp inside the rootfs
    let tmp_dir = root.join("tmp");
    std::fs::create_dir_all(&tmp_dir).with_context(|| format!("create mount target {}", tmp_dir.display()))?;
    mount::<str, Path, str, str>(Some("tmpfs"), &tmp_dir, Some("tmpfs"), MsFlags::empty(), None)
      .context("mount /tmp")?;

    // Create old_root mount point for pivot_root
    let old_root = root.join(".pivot_old");
    std::fs::create_dir_all(&old_root).context("create .pivot_old")?;

    // pivot_root: switch filesystem root
    pivot_root(root, &old_root).context("pivot_root")?;

    // chdir to new root
    chdir("/").context("chdir /")?;

    // Unmount old root (detach — handles busy mounts)
    umount2("/.pivot_old", MntFlags::MNT_DETACH).context("unmount old root")?;
    // The mount is detached; the empty dir is cosmetic. Removal may race the
    // detach and is allowed to fail, but the named helper states that intent.
    if let Err(e) = std::fs::remove_dir("/.pivot_old") {
      eprintln!("warning: could not remove /.pivot_old: {}", e);
    }

    Ok(())
  }

  /// Provides the small set of device nodes install scripts assume —
  /// `null`, `zero`, `random`, `tty`, and the standard symlinks —
  /// inside the isolated `/dev`, without exposing the host's hardware.
  fn dev_setup(&self) -> Result<()> {
    let dev_dir = self.root.join("dev");
    std::fs::create_dir_all(&dev_dir).with_context(|| format!("create {}", dev_dir.display()))?;

    // Mount tmpfs on <rootfs>/dev
    mount::<str, Path, str, str>(
      Some("tmpfs"),
      &dev_dir,
      Some("tmpfs"),
      MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
      Some("mode=0755"),
    )
    .context("mount tmpfs on /dev")?;

    // Bind-mount essential device nodes from host /dev
    let devices = ["null", "zero", "full", "random", "urandom", "tty"];

    for name in &devices {
      let host_path = format!("/dev/{}", name);
      let target = dev_dir.join(name);

      // Create empty file as bind-mount target
      if std::fs::write(&target, "").is_err() {
        continue;
      }

      // Bind-mount the host device node
      if mount::<str, Path, str, str>(Some(&host_path), &target, None, MsFlags::MS_BIND | MsFlags::MS_REC, None)
        .is_err()
      {
        // Non-fatal — some devices may not exist on all systems
        Fs::remove_lenient(&target);
      }
    }

    // Create /dev/pts directory (some scripts expect it)
    let pts_dir = dev_dir.join("pts");
    std::fs::create_dir_all(&pts_dir).with_context(|| format!("create {}", pts_dir.display()))?;

    // Create symlinks that scripts expect
    let _ = std::os::unix::fs::symlink("/proc/self/fd", dev_dir.join("fd"));
    let _ = std::os::unix::fs::symlink("/proc/self/fd/0", dev_dir.join("stdin"));
    let _ = std::os::unix::fs::symlink("/proc/self/fd/1", dev_dir.join("stdout"));
    let _ = std::os::unix::fs::symlink("/proc/self/fd/2", dev_dir.join("stderr"));

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sandbox_check_available() {
    Sandbox::available().expect("namespaces should be available");
  }
}
