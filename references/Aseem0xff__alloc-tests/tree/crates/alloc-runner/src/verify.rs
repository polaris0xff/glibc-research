//! The correctness gate.
//!
//! ⛔ Nothing is timed until it passes here. A binary that segfaults, or that
//! finds the wrong number of lines, has no interesting performance: publishing
//! its speed would rank a broken configuration against working ones.
//!
//! Every expectation is a number the corpus generator computed while planting
//! the data, so the oracle is independent of ripgrep. A check that only asked
//! "did it print something" would pass for a binary that printed the wrong
//! thing, which is the failure mode that matters when an allocator is
//! corrupting memory rather than crashing on it.

use crate::corpus::{Truth, NEEDLE, NEEDLE_UNICODE, REGEX_PATTERN};
use crate::json::J;
use std::path::Path;
use std::process::Command;

pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

impl Check {
    fn pass(name: &str, detail: impl Into<String>) -> Check {
        Check {
            name: name.into(),
            ok: true,
            detail: detail.into(),
        }
    }
    fn fail(name: &str, detail: impl Into<String>) -> Check {
        Check {
            name: name.into(),
            ok: false,
            detail: detail.into(),
        }
    }
    pub fn to_json(&self) -> J {
        J::obj(vec![
            ("name", J::s(self.name.clone())),
            ("ok", J::Bool(self.ok)),
            ("detail", J::s(self.detail.clone())),
        ])
    }
}

struct Run {
    code: i32,
    signal: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run(bin: &Path, args: &[&str]) -> Result<Run, String> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| format!("spawn {}: {}", bin.display(), e))?;
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    Ok(Run {
        code: out.status.code().unwrap_or(-1),
        signal,
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// `rg -c` prints `path:count` per file. Summing the counts gives total
/// matching lines; counting the printed lines gives matching files. Both are
/// asserted, because a binary that found the right total in the wrong number of
/// files has still gone wrong.
fn parse_counts(stdout: &str) -> (u64, u64) {
    let mut lines = 0u64;
    let mut files = 0u64;
    for l in stdout.lines() {
        if l.is_empty() {
            continue;
        }
        if let Some(idx) = l.rfind(':') {
            if let Ok(n) = l[idx + 1..].trim().parse::<u64>() {
                lines += n;
                files += 1;
            }
        }
    }
    (lines, files)
}

fn fnv1a_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Flags applied to every functional and benchmark invocation.
///
/// `--no-ignore` and `--hidden` remove ripgrep's gitignore and hidden-file
/// behaviour from the measurement. They are not there to make ripgrep faster:
/// they are there so the set of files searched is a property of the corpus and
/// not of what happens to be on the filesystem around it.
pub const BASE_FLAGS: &[&str] = &["--no-ignore", "--hidden", "--no-messages"];

pub struct Outcome {
    pub checks: Vec<Check>,
    /// Digest of the sorted matching-file list. Every correct binary must
    /// agree on it, which is a differential check across allocators that needs
    /// no extra expectations.
    pub output_digest: u64,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }
}

pub fn run_gate(bin: &Path, corpus: &Path, truth: &Truth) -> Outcome {
    let mut checks = Vec::new();
    let c = corpus.to_string_lossy().to_string();
    let mut digest = 0u64;

    // 1. It starts at all.
    match run(bin, &["--version"]) {
        Ok(r) if r.signal.is_some() => {
            checks.push(Check::fail(
                "starts",
                format!("killed by signal {:?}", r.signal),
            ));
            // Nothing after this can mean anything.
            return Outcome {
                checks,
                output_digest: 0,
            };
        }
        Ok(r) if r.code != 0 => {
            checks.push(Check::fail(
                "starts",
                format!("--version exited {}", r.code),
            ));
            return Outcome {
                checks,
                output_digest: 0,
            };
        }
        Ok(r) if !r.stdout.contains("ripgrep") => {
            checks.push(Check::fail(
                "starts",
                format!("--version said {:?}", r.stdout.lines().next()),
            ));
        }
        Ok(r) => checks.push(Check::pass(
            "starts",
            r.stdout.lines().next().unwrap_or("").to_string(),
        )),
        Err(e) => {
            checks.push(Check::fail("starts", e));
            return Outcome {
                checks,
                output_digest: 0,
            };
        }
    }

    // 2. Literal search: exact matching lines AND exact matching files.
    let mut args: Vec<&str> = BASE_FLAGS.to_vec();
    args.extend_from_slice(&["-c", NEEDLE, &c]);
    match run(bin, &args) {
        Ok(r) if r.signal.is_some() => checks.push(Check::fail(
            "literal",
            format!("killed by signal {:?}", r.signal),
        )),
        Ok(r) => {
            let (lines, files) = parse_counts(&r.stdout);
            if lines == truth.literal_lines && files == truth.literal_files {
                checks.push(Check::pass(
                    "literal",
                    format!("{} lines in {} files", lines, files),
                ));
            } else {
                checks.push(Check::fail(
                    "literal",
                    format!(
                        "got {} lines in {} files, expected {} lines in {} files (exit {}, stderr {:?})",
                        lines, files, truth.literal_lines, truth.literal_files, r.code,
                        r.stderr.lines().next()
                    ),
                ));
            }
        }
        Err(e) => checks.push(Check::fail("literal", e)),
    }

    // 3. Case-insensitive must find MORE than case-sensitive. If a build
    //    silently ignored -i the two counts would be equal, and a check that
    //    only asserted "greater than zero" would pass.
    let mut args: Vec<&str> = BASE_FLAGS.to_vec();
    args.extend_from_slice(&["-c", "-i", NEEDLE, &c]);
    match run(bin, &args) {
        Ok(r) => {
            let (lines, _) = parse_counts(&r.stdout);
            let expect = truth.literal_lines + truth.lower_lines;
            if lines == expect {
                checks.push(Check::pass("icase", format!("{} lines", lines)));
            } else {
                checks.push(Check::fail(
                    "icase",
                    format!("got {}, expected {}", lines, expect),
                ));
            }
        }
        Err(e) => checks.push(Check::fail("icase", e)),
    }

    // 4. Non-ASCII.
    let mut args: Vec<&str> = BASE_FLAGS.to_vec();
    args.extend_from_slice(&["-c", NEEDLE_UNICODE, &c]);
    match run(bin, &args) {
        Ok(r) => {
            let (lines, _) = parse_counts(&r.stdout);
            if lines == truth.unicode_lines {
                checks.push(Check::pass("unicode", format!("{} lines", lines)));
            } else {
                checks.push(Check::fail(
                    "unicode",
                    format!("got {}, expected {}", lines, truth.unicode_lines),
                ));
            }
        }
        Err(e) => checks.push(Check::fail("unicode", e)),
    }

    // 5. Regex, not a literal fast path.
    let mut args: Vec<&str> = BASE_FLAGS.to_vec();
    args.extend_from_slice(&["-c", REGEX_PATTERN, &c]);
    match run(bin, &args) {
        Ok(r) => {
            let (lines, _) = parse_counts(&r.stdout);
            if lines == truth.regex_lines {
                checks.push(Check::pass("regex", format!("{} lines", lines)));
            } else {
                checks.push(Check::fail(
                    "regex",
                    format!("got {}, expected {}", lines, truth.regex_lines),
                ));
            }
        }
        Err(e) => checks.push(Check::fail("regex", e)),
    }

    // 6. The negative control. Zero matches AND exit status 1. A binary that
    //    exits 0 here is not reporting "no match", and a corpus that vanished
    //    would also produce zero matches — which is why the positive checks
    //    above run in the same gate.
    let mut args: Vec<&str> = BASE_FLAGS.to_vec();
    args.extend_from_slice(&["-c", "QQQQ-ZZZZ-NO-SUCH-TOKEN", &c]);
    match run(bin, &args) {
        Ok(r) => {
            let (lines, _) = parse_counts(&r.stdout);
            if lines == 0 && r.code == 1 {
                checks.push(Check::pass("nomatch", "0 lines, exit 1"));
            } else {
                checks.push(Check::fail(
                    "nomatch",
                    format!("{} lines, exit {}", lines, r.code),
                ));
            }
        }
        Err(e) => checks.push(Check::fail("nomatch", e)),
    }

    // 7. Thread count must not change the answer. This is the check that finds
    //    an allocator that is unsound under concurrency: a data race in a
    //    thread-caching allocator shows up as a wrong or unstable count long
    //    before it shows up as a crash.
    let mut one = String::new();
    let mut agree = true;
    for j in ["1", "4"] {
        let mut args: Vec<&str> = BASE_FLAGS.to_vec();
        args.extend_from_slice(&["-l", "-j", j, NEEDLE, &c]);
        match run(bin, &args) {
            Ok(r) if r.signal.is_some() => {
                checks.push(Check::fail(
                    "threads",
                    format!("-j {} killed by signal {:?}", j, r.signal),
                ));
                agree = false;
            }
            Ok(r) => {
                let mut v: Vec<&str> = r.stdout.lines().collect();
                v.sort_unstable();
                let joined = v.join("\n");
                if j == "1" {
                    one = joined;
                } else if joined != one {
                    checks.push(Check::fail(
                        "threads",
                        format!(
                            "-j1 listed {} files, -j4 listed {}",
                            one.lines().count(),
                            joined.lines().count()
                        ),
                    ));
                    agree = false;
                } else {
                    digest = fnv1a_str(&joined);
                }
            }
            Err(e) => {
                checks.push(Check::fail("threads", e));
                agree = false;
            }
        }
    }
    if agree {
        checks.push(Check::pass(
            "threads",
            format!("-j1 and -j4 agree on {} files", one.lines().count()),
        ));
    }

    // 8. Repeat the whole literal search. ⭐ A control run once is a
    //    coincidence nobody has noticed yet; an allocator fault that shows up
    //    one run in ten is exactly the kind this gate exists to catch.
    let mut args: Vec<&str> = BASE_FLAGS.to_vec();
    args.extend_from_slice(&["-c", NEEDLE, &c]);
    match run(bin, &args) {
        Ok(r) => {
            let (lines, _) = parse_counts(&r.stdout);
            if lines == truth.literal_lines {
                checks.push(Check::pass("repeat", "second run agrees"));
            } else {
                checks.push(Check::fail(
                    "repeat",
                    format!(
                        "second run got {}, first expected {}",
                        lines, truth.literal_lines
                    ),
                ));
            }
        }
        Err(e) => checks.push(Check::fail("repeat", e)),
    }

    // 9. Structured output, which exercises a different allocation path
    //    (serde_json buffers) from the plain printer.
    let mut args: Vec<&str> = BASE_FLAGS.to_vec();
    args.extend_from_slice(&["--json", NEEDLE, &c]);
    match run(bin, &args) {
        Ok(r) if r.signal.is_some() => checks.push(Check::fail(
            "json",
            format!("killed by signal {:?}", r.signal),
        )),
        Ok(r) => {
            let n = r
                .stdout
                .lines()
                .filter(|l| l.contains("\"type\":\"match\""))
                .count() as u64;
            if n == truth.literal_lines {
                checks.push(Check::pass("json", format!("{} match events", n)));
            } else {
                checks.push(Check::fail(
                    "json",
                    format!("got {} match events, expected {}", n, truth.literal_lines),
                ));
            }
        }
        Err(e) => checks.push(Check::fail("json", e)),
    }

    Outcome {
        checks,
        output_digest: digest,
    }
}
