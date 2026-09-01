//! Deterministic corpus generation, with ground truth computed by construction.
//!
//! Two things make a benchmark corpus useful here, and a downloaded tarball
//! gives neither:
//!
//!   1. **Byte-identical on every host.** Two machines can then be compared on
//!      the same work, and a corpus difference can never be mistaken for an
//!      allocator difference.
//!   2. **Known answers.** The generator plants each needle, so it knows
//!      exactly how many matching lines exist before ripgrep is ever run. That
//!      is an oracle independent of the thing being tested, which is what lets
//!      the correctness gate assert an exact count rather than "it printed
//!      something".
//!
//! ⭐ The negative control matters as much as the positive one. `nomatch` must
//! return zero matches AND exit 1; a corpus bug that produced an empty tree
//! would satisfy "zero matches" while satisfying nothing else, so the positive
//! queries are asserted in the same run.

use crate::json::J;
use std::fs;
use std::io::Write;
use std::path::Path;

/// xorshift64*. Chosen because it is four lines, has no dependency, and is
/// identical on every platform: the corpus must not vary with the host's
/// libc or word size.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // A zero state is absorbing for xorshift, so it is never allowed.
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    #[inline]
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

const WORDS: &[&str] = &[
    "alpha", "beacon", "cinder", "delta", "ember", "fathom", "gossamer", "harbor", "ingot",
    "jasper", "kernel", "lattice", "marrow", "nimbus", "onyx", "pillar", "quartz", "rampart",
    "sable", "tundra", "umber", "vellum", "willow", "xenon", "yarrow", "zephyr", "anvil",
    "bramble", "cobalt", "dovetail", "elmwood", "furrow", "granite", "hollow", "iris", "juniper",
    "kestrel", "lichen", "mantle", "nettle", "orchid", "plinth", "quiver", "runnel", "sorrel",
    "thistle", "upland", "verdant", "wicket", "yeoman",
];

/// The planted literal. Chosen so it cannot occur by accident from `WORDS`,
/// and so it is not a substring of any other planted token.
pub const NEEDLE: &str = "ZORKMID";
/// A lowercase variant, planted separately, so `-i` has a different expected
/// count from the case-sensitive search. If both counts were equal, a binary
/// that ignored `-i` entirely would pass the gate.
pub const NEEDLE_LOWER: &str = "zorkmid";
/// Non-ASCII, to catch a build whose UTF-8 handling differs.
pub const NEEDLE_UNICODE: &str = "λ-Ωmega-Ж";
/// Matched by the regex query. The digits vary so the pattern must really be
/// applied rather than a literal fast path taken.
pub const REGEX_PATTERN: &str = r"TRACE-[0-9]{4}-(alpha|omega)";

/// Where the searchable files live under the corpus directory.
pub const DATA_SUBDIR: &str = "data";

pub struct Profile {
    pub name: &'static str,
    pub dirs: usize,
    pub files_per_dir: usize,
    pub lines_per_file: usize,
}

pub const PROFILES: &[Profile] = &[
    // Fast enough to run inside a pull-request check.
    Profile {
        name: "smoke",
        dirs: 6,
        files_per_dir: 20,
        lines_per_file: 120,
    },
    // The default. ~7 000 files, ~65 MB: large enough that a single ripgrep run
    // is hundreds of milliseconds (so timer resolution is irrelevant) and file
    // handling dominates, which is where a container binary's allocator shows.
    Profile {
        name: "standard",
        dirs: 60,
        files_per_dir: 120,
        lines_per_file: 150,
    },
    Profile {
        name: "large",
        dirs: 120,
        files_per_dir: 200,
        lines_per_file: 200,
    },
];

pub fn profile(name: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.name == name)
}

#[derive(Default, Debug)]
pub struct Truth {
    pub files: u64,
    pub bytes: u64,
    pub lines: u64,
    pub literal_lines: u64,
    pub literal_files: u64,
    pub lower_lines: u64,
    pub unicode_lines: u64,
    pub regex_lines: u64,
    pub digest: u64,
}

#[inline]
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Write the corpus and return the ground truth.
///
/// Files are visited in a fixed order, so the running FNV-1a digest covers path
/// and content in a defined sequence. Two hosts that disagree on it did not
/// generate the same corpus, and the measurement that follows would not have
/// been comparable.
///
/// `out = None` computes the ground truth without touching the filesystem. That
/// is what makes the truth re-derivable from `(seed, profile)` alone: a later
/// session can check a published expectation without the corpus, and the
/// selftest can prove the generator still produces the same numbers.
///
/// ⛔ THE DATA GOES IN A SUBDIRECTORY, and this is not tidiness.
///
/// `manifest.json` records the patterns so a reader can check the
/// expectations. It therefore CONTAINS the needle strings. Written beside the
/// data, ripgrep searches it too: every count comes back one too high and the
/// negative control finds one match instead of none. That was observed here on
/// 2026-09-01, on the first end-to-end run, as `expected 211 got 212` across
/// every check at once -- a uniform off-by-one that looks like a corpus bug and
/// is really the instrument measuring its own notes.
pub fn generate(out: Option<&Path>, seed: u64, prof: &Profile) -> Result<Truth, String> {
    let out = out.map(|o| o.join(DATA_SUBDIR));
    let out = out.as_deref();
    if let Some(out) = out {
        fs::create_dir_all(out).map_err(|e| format!("{}: {}", out.display(), e))?;
    }
    let mut rng = Rng::new(seed);
    let mut t = Truth::default();
    let mut digest: u64 = 0xcbf29ce484222325;
    let mut line = String::with_capacity(256);

    for d in 0..prof.dirs {
        if let Some(out) = out {
            let dir = out.join(format!("d{:03}", d));
            fs::create_dir_all(&dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
        }
        for f in 0..prof.files_per_dir {
            let rel = format!("d{:03}/f{:04}.txt", d, f);
            let mut buf = String::with_capacity(prof.lines_per_file * 72);
            let mut file_has_literal = false;

            for _ in 0..prof.lines_per_file {
                line.clear();
                let nwords = 6 + rng.below(7) as usize;
                for w in 0..nwords {
                    if w > 0 {
                        line.push(' ');
                    }
                    line.push_str(WORDS[rng.below(WORDS.len() as u64) as usize]);
                }

                // At most ONE planted token per line. That is what makes
                // "matching lines" equal to "tokens planted": ripgrep counts a
                // line once however many times the pattern hits it, so a line
                // with two needles would break the identity and the gate would
                // be asserting the wrong number.
                match rng.below(1000) {
                    0..=14 => {
                        line.push(' ');
                        line.push_str(NEEDLE);
                        t.literal_lines += 1;
                        file_has_literal = true;
                    }
                    15..=24 => {
                        line.push(' ');
                        line.push_str(NEEDLE_LOWER);
                        t.lower_lines += 1;
                    }
                    25..=31 => {
                        line.push(' ');
                        line.push_str(NEEDLE_UNICODE);
                        t.unicode_lines += 1;
                    }
                    32..=49 => {
                        let n = rng.below(10000);
                        let tail = if rng.below(2) == 0 { "alpha" } else { "omega" };
                        line.push_str(&format!(" TRACE-{:04}-{}", n, tail));
                        t.regex_lines += 1;
                    }
                    _ => {}
                }
                line.push('\n');
                buf.push_str(&line);
                t.lines += 1;
            }

            digest = fnv1a(digest, rel.as_bytes());
            digest = fnv1a(digest, buf.as_bytes());
            t.bytes += buf.len() as u64;
            t.files += 1;
            if file_has_literal {
                t.literal_files += 1;
            }

            if let Some(out) = out {
                let path = out.join(&rel);
                let mut fh =
                    fs::File::create(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
                fh.write_all(buf.as_bytes())
                    .map_err(|e| format!("{}: {}", path.display(), e))?;
            }
        }
    }

    t.digest = digest;
    Ok(t)
}

impl Truth {
    pub fn to_json(&self, seed: u64, prof: &Profile) -> J {
        J::obj(vec![
            ("seed", J::U(seed)),
            ("profile", J::s(prof.name)),
            ("files", J::U(self.files)),
            ("bytes", J::U(self.bytes)),
            ("lines", J::U(self.lines)),
            ("digest_fnv1a64", J::s(format!("{:016x}", self.digest))),
            (
                "expect",
                J::obj(vec![
                    ("literal_lines", J::U(self.literal_lines)),
                    ("literal_files", J::U(self.literal_files)),
                    // `-i ZORKMID` matches both the uppercase and lowercase
                    // plantings, and nothing else: no WORDS entry contains the
                    // substring in any case.
                    ("icase_lines", J::U(self.literal_lines + self.lower_lines)),
                    ("unicode_lines", J::U(self.unicode_lines)),
                    ("regex_lines", J::U(self.regex_lines)),
                    ("nomatch_lines", J::U(0)),
                ]),
            ),
            (
                "patterns",
                J::obj(vec![
                    ("literal", J::s(NEEDLE)),
                    ("icase", J::s(NEEDLE)),
                    ("unicode", J::s(NEEDLE_UNICODE)),
                    ("regex", J::s(REGEX_PATTERN)),
                    ("nomatch", J::s("QQQQ-ZZZZ-NO-SUCH-TOKEN")),
                ]),
            ),
        ])
    }
}
