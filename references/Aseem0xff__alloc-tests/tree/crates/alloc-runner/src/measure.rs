//! External measurement: fork, exec, wait, read the kernel's accounting.
//!
//! ⭐ The subject is never asked how it did. Wall time comes from
//! `CLOCK_MONOTONIC` in the parent around `fork`/`waitpid`, and memory and
//! fault counts come from `wait4`'s `rusage`, which the kernel fills in. The
//! measured program contributes nothing to its own numbers.
//!
//! `wait4` is used rather than `getrusage(RUSAGE_CHILDREN)` deliberately:
//! `RUSAGE_CHILDREN` reports the maximum over *all* reaped children, so after
//! the first sample every later `ru_maxrss` would be that running maximum
//! rather than this run's. That produces a flat, plausible, wrong series.
//!
//! ⚠ `ru_maxrss` is in kilobytes on Linux. It is peak resident set for the
//! child, which is the number that matters for a container memory limit, and
//! it is not the same as the allocator's own idea of heap size.

use crate::json::J;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Sample {
    pub wall_ns: u64,
    pub user_us: u64,
    pub sys_us: u64,
    pub maxrss_kb: u64,
    pub minflt: u64,
    pub majflt: u64,
    pub exit_code: i32,
    /// `Some(signal)` when the child was killed. A segfaulting configuration
    /// must never be reported as a fast one.
    pub signal: Option<i32>,
}

impl Sample {
    pub fn ok(&self, accept: &[i32]) -> bool {
        self.signal.is_none() && accept.contains(&self.exit_code)
    }
    pub fn to_json(&self) -> J {
        J::obj(vec![
            ("wall_ns", J::U(self.wall_ns)),
            ("wall_s", J::F(self.wall_ns as f64 / 1e9)),
            ("user_us", J::U(self.user_us)),
            ("sys_us", J::U(self.sys_us)),
            ("maxrss_kb", J::U(self.maxrss_kb)),
            ("minflt", J::U(self.minflt)),
            ("majflt", J::U(self.majflt)),
            ("exit_code", J::I(self.exit_code as i64)),
            (
                "signal",
                match self.signal {
                    Some(s) => J::I(s as i64),
                    None => J::Null,
                },
            ),
        ])
    }
}

fn now_ns() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // CLOCK_MONOTONIC, not CLOCK_REALTIME: an NTP step mid-run must not become
    // a measurement.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

/// Run `argv` once with `envp` additions, discarding output, and measure it.
///
/// `cwd` is applied in the child. `timeout_s` bounds a hung run: a
/// configuration that deadlocks is a failure, not an infinite measurement.
pub fn run_once(
    argv: &[String],
    extra_env: &[(String, String)],
    cwd: Option<&Path>,
    timeout_s: u64,
) -> Result<Sample, String> {
    if argv.is_empty() {
        return Err("empty argv".into());
    }

    // Everything that can allocate or fail is done BEFORE the fork. Between
    // fork and exec only async-signal-safe calls are legal, and a failed
    // allocation there would be undiagnosable.
    let c_argv: Vec<CString> = argv
        .iter()
        .map(|a| CString::new(a.as_bytes()).map_err(|_| "argv contains NUL".to_string()))
        .collect::<Result<_, _>>()?;
    let mut argv_ptrs: Vec<*const libc::c_char> = c_argv.iter().map(|s| s.as_ptr()).collect();
    argv_ptrs.push(std::ptr::null());

    let mut env_pairs: Vec<CString> = Vec::new();
    for (k, v) in std::env::vars() {
        if extra_env.iter().any(|(ek, _)| *ek == k) {
            continue;
        }
        env_pairs.push(CString::new(format!("{}={}", k, v)).map_err(|_| "env NUL".to_string())?);
    }
    for (k, v) in extra_env {
        env_pairs.push(CString::new(format!("{}={}", k, v)).map_err(|_| "env NUL".to_string())?);
    }
    let mut env_ptrs: Vec<*const libc::c_char> = env_pairs.iter().map(|s| s.as_ptr()).collect();
    env_ptrs.push(std::ptr::null());

    let c_cwd = match cwd {
        Some(p) => Some(CString::new(p.as_os_str().as_bytes()).map_err(|_| "cwd NUL".to_string())?),
        None => None,
    };
    let devnull = CString::new("/dev/null").unwrap();

    let t0 = now_ns();
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return Err("fork failed".into());
    }

    if pid == 0 {
        // ---- child ----
        unsafe {
            if let Some(d) = &c_cwd {
                if libc::chdir(d.as_ptr()) != 0 {
                    libc::_exit(126);
                }
            }
            // Output is discarded rather than piped: a pipe makes the parent a
            // participant in the measurement, and a full pipe buffer would
            // stall the child and be recorded as slowness.
            let fd = libc::open(devnull.as_ptr(), libc::O_WRONLY);
            if fd >= 0 {
                libc::dup2(fd, 1);
                libc::dup2(fd, 2);
                if fd > 2 {
                    libc::close(fd);
                }
            }
            libc::execve(argv_ptrs[0], argv_ptrs.as_ptr(), env_ptrs.as_ptr());
            // Only reachable when exec failed.
            libc::_exit(127);
        }
    }

    // ---- parent ----
    let mut status: libc::c_int = 0;
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    let deadline = t0 + timeout_s.saturating_mul(1_000_000_000);
    let mut killed = false;

    loop {
        let r = unsafe { libc::wait4(pid, &mut status, libc::WNOHANG, &mut ru) };
        if r == pid {
            break;
        }
        if r < 0 {
            return Err("wait4 failed".into());
        }
        if now_ns() > deadline && !killed {
            unsafe { libc::kill(pid, libc::SIGKILL) };
            killed = true;
        }
        // 200 µs: fine enough that the poll adds well under a millisecond to a
        // run measured in hundreds, coarse enough not to spin a core.
        let ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 200_000,
        };
        unsafe { libc::nanosleep(&ts, std::ptr::null_mut()) };
    }
    let t1 = now_ns();

    let exited = libc::WIFEXITED(status);
    let signaled = libc::WIFSIGNALED(status);

    Ok(Sample {
        wall_ns: t1.saturating_sub(t0),
        user_us: ru.ru_utime.tv_sec as u64 * 1_000_000 + ru.ru_utime.tv_usec as u64,
        sys_us: ru.ru_stime.tv_sec as u64 * 1_000_000 + ru.ru_stime.tv_usec as u64,
        maxrss_kb: ru.ru_maxrss.max(0) as u64,
        minflt: ru.ru_minflt.max(0) as u64,
        majflt: ru.ru_majflt.max(0) as u64,
        exit_code: if exited {
            libc::WEXITSTATUS(status)
        } else {
            -1
        },
        signal: if signaled {
            Some(libc::WTERMSIG(status))
        } else {
            None
        },
    })
}

// ---------------------------------------------------------------------------
// Summary statistics.
//
// Deliberately modest. docs/methodology.md states why there is no confidence
// interval here: the runs are not independent draws from a stationary process
// (a shared CI runner is not that), so an interval computed as though they were
// would be a precise-looking claim about nothing. Median and MAD say what they
// say and no more.

pub struct Stats {
    pub n: usize,
    pub min: f64,
    pub median: f64,
    pub mean: f64,
    pub max: f64,
    pub stddev: f64,
    /// Median absolute deviation, scaled to be comparable to a standard
    /// deviation for normal data. Robust to the one slow run a noisy neighbour
    /// causes, which a stddev is not.
    pub mad: f64,
    /// MAD as a fraction of the median. The run is flagged above a threshold.
    pub rel_mad: f64,
}

pub fn stats(values: &[f64]) -> Option<Stats> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    let median = if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    };
    let mean = v.iter().sum::<f64>() / n as f64;
    let var = if n > 1 {
        v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64
    } else {
        0.0
    };
    let mut dev: Vec<f64> = v.iter().map(|x| (x - median).abs()).collect();
    dev.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mad_raw = if n % 2 == 1 {
        dev[n / 2]
    } else {
        (dev[n / 2 - 1] + dev[n / 2]) / 2.0
    };
    let mad = mad_raw * 1.4826;
    Some(Stats {
        n,
        min: v[0],
        median,
        mean,
        max: v[n - 1],
        stddev: var.sqrt(),
        mad,
        rel_mad: if median > 0.0 { mad / median } else { 0.0 },
    })
}

impl Stats {
    pub fn to_json(&self) -> J {
        J::obj(vec![
            ("n", J::U(self.n as u64)),
            ("min", J::F(self.min)),
            ("median", J::F(self.median)),
            ("mean", J::F(self.mean)),
            ("max", J::F(self.max)),
            ("stddev", J::F(self.stddev)),
            ("mad", J::F(self.mad)),
            ("rel_mad", J::F(self.rel_mad)),
        ])
    }
}
