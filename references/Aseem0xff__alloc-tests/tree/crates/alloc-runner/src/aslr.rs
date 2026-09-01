//! Does the executable image actually move between runs?
//!
//! ⚠ The usual way this gets reported is by inference: the ELF is `ET_DYN`, so
//! the loader must be randomising it, so ASLR is on. That is a claim about the
//! kernel derived from a property of the file, and it is wrong whenever the
//! container was started with `--security-opt seccomp` variations, under
//! `setarch -R`, or on a kernel with `randomize_va_space=0`.
//!
//! ⭐ So it is observed instead. The child is started on a workload long enough
//! to still be alive when the parent looks, and the parent reads the load
//! address out of `/proc/<pid>/maps` -- an independent view, from outside the
//! process, of where the kernel actually put it.
//!
//! ⛔ A single run cannot answer this. Two runs at the same address are
//! evidence of no randomisation only if the sampler is known to work, so the
//! result reports the number of distinct bases seen out of N and the caller
//! decides. A `--expect` flag turns that into an assertion.

use crate::json::J;
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs;
use std::path::Path;

/// The start address of the first executable mapping backed by `bin`.
fn base_of(pid: i32, bin: &str) -> Option<u64> {
    let maps = fs::read_to_string(format!("/proc/{}/maps", pid)).ok()?;
    for line in maps.lines() {
        // 55a4c0e00000-55a4c0e21000 r-xp 00000000 08:01 12345 /path/to/rg
        let mut parts = line.split_whitespace();
        let range = parts.next()?;
        let perms = parts.next()?;
        let path = line.rsplit_once(' ').map(|(_, p)| p.trim()).unwrap_or("");
        if !perms.contains('x') {
            continue;
        }
        if !path.ends_with(bin) && path != bin {
            continue;
        }
        let start = range.split('-').next()?;
        return u64::from_str_radix(start, 16).ok();
    }
    None
}

pub struct Observation {
    pub bases: Vec<u64>,
    pub sampled: usize,
    pub attempts: usize,
}

impl Observation {
    pub fn distinct(&self) -> usize {
        self.bases.iter().copied().collect::<BTreeSet<u64>>().len()
    }
    /// Randomised if more than one distinct base was seen. With a working
    /// sampler and >= 4 samples, one distinct base means it is not moving.
    pub fn randomised(&self) -> bool {
        self.distinct() > 1
    }
    pub fn to_json(&self) -> J {
        J::obj(vec![
            ("attempts", J::U(self.attempts as u64)),
            ("sampled", J::U(self.sampled as u64)),
            ("distinct_bases", J::U(self.distinct() as u64)),
            ("randomised", J::Bool(self.randomised())),
            (
                "bases",
                J::arr(
                    self.bases
                        .iter()
                        .map(|b| J::s(format!("0x{:x}", b)))
                        .collect(),
                ),
            ),
        ])
    }
}

pub fn probe(bin: &Path, args: &[String], runs: usize) -> Result<Observation, String> {
    let bin_s = bin.to_string_lossy().to_string();
    let mut bases = Vec::new();

    let argv: Vec<String> = std::iter::once(bin_s.clone())
        .chain(args.iter().cloned())
        .collect();
    let c_argv: Vec<CString> = argv
        .iter()
        .map(|a| CString::new(a.as_bytes()).map_err(|_| "argv NUL".to_string()))
        .collect::<Result<_, _>>()?;
    let mut ptrs: Vec<*const libc::c_char> = c_argv.iter().map(|s| s.as_ptr()).collect();
    ptrs.push(std::ptr::null());
    let devnull = CString::new("/dev/null").unwrap();

    for _ in 0..runs {
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err("fork failed".into());
        }
        if pid == 0 {
            unsafe {
                let fd = libc::open(devnull.as_ptr(), libc::O_WRONLY);
                if fd >= 0 {
                    libc::dup2(fd, 1);
                    libc::dup2(fd, 2);
                    if fd > 2 {
                        libc::close(fd);
                    }
                }
                libc::execv(ptrs[0], ptrs.as_ptr());
                libc::_exit(127);
            }
        }
        // Poll until the mapping appears. The child has to have exec'd and not
        // yet exited; a workload of a few hundred milliseconds makes that easy,
        // and a miss is recorded as a miss rather than as "not randomised".
        let mut got = None;
        for _ in 0..2000 {
            if let Some(b) = base_of(pid, &bin_s) {
                got = Some(b);
                break;
            }
            let ts = libc::timespec {
                tv_sec: 0,
                tv_nsec: 100_000,
            };
            unsafe { libc::nanosleep(&ts, std::ptr::null_mut()) };
        }
        if let Some(b) = got {
            bases.push(b);
        }
        let mut status = 0;
        unsafe { libc::waitpid(pid, &mut status, 0) };
    }

    Ok(Observation {
        sampled: bases.len(),
        attempts: runs,
        bases,
    })
}
