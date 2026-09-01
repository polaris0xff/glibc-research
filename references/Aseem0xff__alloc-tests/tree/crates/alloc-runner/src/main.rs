//! alloc-runner -- the instrument.
//!
//! This binary runs INSIDE the benchmark container. It generates the corpus,
//! reads the ELF to establish which allocator is really present, runs the
//! correctness gate, and takes the measurements. The host orchestrator
//! (`alloc-bench`) never times anything itself: putting `docker run` inside the
//! timed region would measure the container runtime.
//!
//! ⛔ EXIT CODES, uniform across every subcommand:
//!   0  the measurement ran and the thing passed
//!   1  the measurement ran and the thing FAILED
//!   2  the measurement could not run (missing input, bad arguments)
//!
//! ⚠ 2 is never reported as a pass. A configuration that could not be measured
//! and a configuration that was measured and lost are different results, and
//! the report prints them differently.

mod ar;
mod aslr;
mod corpus;
mod elf;
mod ident;
mod json;
mod measure;
mod patchrg;
mod verify;

use elf::LinkKind;
use json::J;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_FAILED: u8 = 1;
const EXIT_CANNOT_RUN: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        return ExitCode::from(EXIT_CANNOT_RUN);
    }
    let flags = parse_flags(&args[2..]);

    // ⛔ Same defect as alloc-bench, and it bites harder here: help was matched
    // only at args[1], so `alloc-runner patch-rg --help` would PATCH A SOURCE
    // TREE and `gen-corpus --help` would write a corpus. A request to read the
    // usage must never be a request to act. Answered before dispatch; `-h` is
    // scanned in the raw argv because `parse_flags` only recognises `--`.
    if flags.contains_key("help") || args[2..].iter().any(|a| a == "-h") {
        usage();
        return ExitCode::from(EXIT_OK);
    }

    let code = match args[1].as_str() {
        "gen-corpus" => cmd_gen_corpus(&flags),
        "identify" => cmd_identify(&flags),
        "verify" => cmd_verify(&flags),
        "measure" => cmd_measure(&flags),
        "archive-check" => cmd_archive_check(&flags),
        "ar-members" => cmd_ar_members(&flags),
        "patch-rg" => cmd_patch_rg(&flags),
        "aslr-probe" => cmd_aslr_probe(&flags),
        "workloads" => cmd_workloads(),
        "selftest" => cmd_selftest(),
        "-h" | "--help" | "help" => {
            usage();
            EXIT_OK
        }
        other => {
            eprintln!("alloc-runner: unknown subcommand: {}", other);
            usage();
            EXIT_CANNOT_RUN
        }
    };
    ExitCode::from(code)
}

fn usage() {
    eprintln!(
        r#"alloc-runner -- the in-container instrument for alloc-tests

  gen-corpus    --out DIR --seed N --profile smoke|standard|large
  identify      --bin PATH --expect-allocator ID [--expect-kind static|static-pie|dynamic]
                [--replacement]
  verify        --bin PATH --corpus DIR
  measure       --bin PATH --corpus DIR --workload NAME --repeat N [--warmup N]
                [--timeout S] [--env K=V ...]
  archive-check --archive PATH --symbol NAME [--expect-providers N]
  patch-rg      --src DIR [--shim-path PATH --shim-feature NAME]
  aslr-probe    --bin PATH --corpus DIR [--runs N] [--expect randomised|fixed]
  workloads     list the workload definitions
  selftest      offline checks of the instrument itself

Exit: 0 ran and passed, 1 ran and failed, 2 could not run."#
    );
}

// ---------------------------------------------------------------------------
// Argument handling. Deliberately tiny: a CLI parser is a dependency the
// container image would have to build, and this surface is fixed.

type Flags = BTreeMap<String, Vec<String>>;

fn parse_flags(args: &[String]) -> Flags {
    let mut out: Flags = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(name) = a.strip_prefix("--") {
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                out.entry(name.to_string())
                    .or_default()
                    .push(args[i + 1].clone());
                i += 2;
            } else {
                out.entry(name.to_string()).or_default().push("true".into());
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

fn one<'a>(f: &'a Flags, k: &str) -> Option<&'a str> {
    f.get(k).and_then(|v| v.first()).map(|s| s.as_str())
}
fn has(f: &Flags, k: &str) -> bool {
    f.contains_key(k)
}
fn num(f: &Flags, k: &str, default: u64) -> u64 {
    one(f, k).and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn emit(v: &J) {
    println!("{}", v.to_string());
}

// ---------------------------------------------------------------------------

fn resolve_profile(f: &Flags) -> Result<&'static corpus::Profile, u8> {
    let name = one(f, "profile").unwrap_or("standard");
    corpus::profile(name).ok_or_else(|| {
        eprintln!(
            "alloc-runner: unknown profile {:?}; known: {}",
            name,
            corpus::PROFILES
                .iter()
                .map(|p| p.name)
                .collect::<Vec<_>>()
                .join(", ")
        );
        EXIT_CANNOT_RUN
    })
}

fn cmd_gen_corpus(f: &Flags) -> u8 {
    let Some(out) = one(f, "out") else {
        eprintln!("alloc-runner gen-corpus: --out is required");
        return EXIT_CANNOT_RUN;
    };
    let prof = match resolve_profile(f) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let seed = num(f, "seed", 20260901);
    let out = PathBuf::from(out);

    match corpus::generate(Some(&out), seed, prof) {
        Ok(t) => {
            // Two files, on purpose. manifest.json is provenance the
            // orchestrator records with the results; truth.kv is what `verify`
            // reads back, in a format that needs no parser in this binary.
            let manifest = t.to_json(seed, prof);
            if let Err(e) = std::fs::write(out.join("manifest.json"), manifest.to_string()) {
                eprintln!("alloc-runner: writing manifest: {}", e);
                return EXIT_CANNOT_RUN;
            }
            let kv = format!(
                "files={}\nbytes={}\nlines={}\nliteral_lines={}\nliteral_files={}\nlower_lines={}\nunicode_lines={}\nregex_lines={}\ndigest={:016x}\nseed={}\nprofile={}\n",
                t.files, t.bytes, t.lines, t.literal_lines, t.literal_files, t.lower_lines,
                t.unicode_lines, t.regex_lines, t.digest, seed, prof.name
            );
            if let Err(e) = std::fs::write(out.join("truth.kv"), kv) {
                eprintln!("alloc-runner: writing truth.kv: {}", e);
                return EXIT_CANNOT_RUN;
            }
            emit(&manifest);
            EXIT_OK
        }
        Err(e) => {
            eprintln!("alloc-runner gen-corpus: {}", e);
            EXIT_CANNOT_RUN
        }
    }
}

fn read_truth(corpus_dir: &Path) -> Result<corpus::Truth, String> {
    let p = corpus_dir.join("truth.kv");
    let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {}", p.display(), e))?;
    let mut m: BTreeMap<&str, &str> = BTreeMap::new();
    for line in s.lines() {
        if let Some((k, v)) = line.split_once('=') {
            m.insert(k.trim(), v.trim());
        }
    }
    let g = |k: &str| -> u64 { m.get(k).and_then(|v| v.parse().ok()).unwrap_or(0) };
    Ok(corpus::Truth {
        files: g("files"),
        bytes: g("bytes"),
        lines: g("lines"),
        literal_lines: g("literal_lines"),
        literal_files: g("literal_files"),
        lower_lines: g("lower_lines"),
        unicode_lines: g("unicode_lines"),
        regex_lines: g("regex_lines"),
        digest: u64::from_str_radix(m.get("digest").unwrap_or(&"0"), 16).unwrap_or(0),
    })
}

fn cmd_identify(f: &Flags) -> u8 {
    let Some(bin) = one(f, "bin") else {
        eprintln!("alloc-runner identify: --bin is required");
        return EXIT_CANNOT_RUN;
    };
    let e = match elf::parse(Path::new(bin)) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("alloc-runner identify: {}", err);
            return EXIT_CANNOT_RUN;
        }
    };
    let expect_alloc = one(f, "expect-allocator").unwrap_or("system");
    let expect_kind = one(f, "expect-kind").and_then(|k| match k {
        "static" => Some(LinkKind::Static),
        "static-pie" => Some(LinkKind::StaticPie),
        "dynamic" => Some(LinkKind::Dynamic),
        _ => None,
    });
    let verdict = ident::judge(
        &e,
        expect_alloc,
        expect_kind.as_ref(),
        has(f, "replacement"),
    );
    let size = std::fs::metadata(bin).map(|m| m.len()).unwrap_or(0);
    let mut report = ident::report_json(&e, &verdict);
    if let J::O(ref mut pairs) = report {
        pairs.push(("binary_bytes".into(), J::U(size)));
        pairs.push(("expect_allocator".into(), J::s(expect_alloc)));
    }
    emit(&report);
    if verdict.ok {
        EXIT_OK
    } else {
        for r in &verdict.reasons {
            eprintln!("alloc-runner identify: {}", r);
        }
        EXIT_FAILED
    }
}

fn cmd_verify(f: &Flags) -> u8 {
    let (Some(bin), Some(corpus_dir)) = (one(f, "bin"), one(f, "corpus")) else {
        eprintln!("alloc-runner verify: --bin and --corpus are required");
        return EXIT_CANNOT_RUN;
    };
    let truth = match read_truth(Path::new(corpus_dir)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("alloc-runner verify: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    if truth.files == 0 {
        eprintln!("alloc-runner verify: corpus truth says 0 files; the corpus was not generated");
        return EXIT_CANNOT_RUN;
    }
    let data = Path::new(corpus_dir).join(corpus::DATA_SUBDIR);
    if !data.is_dir() {
        eprintln!(
            "alloc-runner verify: {} has no data/ subdirectory; regenerate the corpus",
            corpus_dir
        );
        return EXIT_CANNOT_RUN;
    }
    let outcome = verify::run_gate(Path::new(bin), &data, &truth);
    emit(&J::obj(vec![
        ("ok", J::Bool(outcome.ok())),
        (
            "output_digest",
            J::s(format!("{:016x}", outcome.output_digest)),
        ),
        (
            "checks",
            J::arr(outcome.checks.iter().map(|c| c.to_json()).collect()),
        ),
    ]));
    for c in &outcome.checks {
        if !c.ok {
            eprintln!("alloc-runner verify: FAIL {}: {}", c.name, c.detail);
        }
    }
    if outcome.ok() {
        EXIT_OK
    } else {
        EXIT_FAILED
    }
}

// ---------------------------------------------------------------------------
// Workloads.
//
// Each exists for a stated reason. A workload with no reason to exist burns CI
// time and adds a column nobody can interpret.

pub struct Workload {
    pub name: &'static str,
    pub why: &'static str,
    /// `{corpus}` and `{onefile}` are substituted.
    pub args: &'static [&'static str],
    pub accept: &'static [i32],
}

pub const WORKLOADS: &[Workload] = &[
    Workload {
        name: "literal",
        why: "Literal search that prints every hit. The printer allocates per match, so this is the allocation-heaviest realistic path.",
        args: &["--no-ignore", "--hidden", "--no-messages", "ZORKMID", "{corpus}"],
        accept: &[0],
    },
    Workload {
        name: "literal-j1",
        why: "The same search pinned to one thread. Separates single-thread allocator cost from the thread-caching behaviour the parallel run exercises.",
        args: &["--no-ignore", "--hidden", "--no-messages", "-j", "1", "ZORKMID", "{corpus}"],
        accept: &[0],
    },
    Workload {
        name: "regex",
        why: "A pattern with a bounded repeat and an alternation, so the regex engine runs instead of a literal fast path.",
        args: &["--no-ignore", "--hidden", "--no-messages", r"TRACE-[0-9]{4}-(alpha|omega)", "{corpus}"],
        accept: &[0],
    },
    Workload {
        name: "nomatch",
        why: "Scans every byte and prints nothing. Isolates traversal and buffering from output allocation; exits 1 by design.",
        args: &["--no-ignore", "--hidden", "--no-messages", "QQQQ-ZZZZ-NO-SUCH-TOKEN", "{corpus}"],
        accept: &[1],
    },
    Workload {
        name: "files",
        why: "-l stops at the first hit per file, so directory walking and per-file setup dominate. This is where many small allocations live.",
        args: &["--no-ignore", "--hidden", "--no-messages", "-l", "ZORKMID", "{corpus}"],
        accept: &[0],
    },
    Workload {
        name: "json",
        why: "Structured output goes through a different buffering path from the plain printer.",
        args: &["--no-ignore", "--hidden", "--no-messages", "--json", "ZORKMID", "{corpus}"],
        accept: &[0],
    },
    Workload {
        name: "startup",
        why: "One small file. Dominated by process start and allocator initialisation, which is the cost a short-lived container command actually pays.",
        args: &["--no-ignore", "--hidden", "--no-messages", "ZORKMID", "{onefile}"],
        accept: &[0, 1],
    },
];

/// Which members of an archive define any of `--symbols`.
///
/// ⭐ This is what makes libc surgery survive a libc upgrade. The prior art
/// (references/haskell-wasm__rust-alpine-mimalloc, tree/build.sh) deletes a
/// HARD-CODED list of musl object names -- `malloc.lo`, `free.lo`,
/// `lite_malloc.lo` and so on. Those names are a property of the musl release
/// that built the archive. When a future musl renames or splits one, the `ar`
/// DELETE matches nothing, both allocators end up in libc.a, and which one
/// serves `malloc` is decided by link order. Nothing fails; the numbers are
/// just wrong.
///
/// Deriving the list from the archive itself cannot go stale, and the surgery
/// script asserts on the count afterwards either way.
fn cmd_ar_members(f: &Flags) -> u8 {
    let Some(archive) = one(f, "archive") else {
        eprintln!("alloc-runner ar-members: --archive is required");
        return EXIT_CANNOT_RUN;
    };
    let symbols: Vec<&str> = one(f, "symbols")
        .unwrap_or("malloc")
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let mut names: Vec<String> = Vec::new();
    for sym in &symbols {
        match ar::definers(Path::new(archive), sym) {
            Ok(ps) => {
                for p in ps {
                    if !names.contains(&p.member) {
                        names.push(p.member);
                    }
                }
            }
            Err(e) => {
                eprintln!("alloc-runner ar-members: {}", e);
                return EXIT_CANNOT_RUN;
            }
        }
    }
    names.sort();
    for n in &names {
        println!("{}", n);
    }
    if names.is_empty() {
        // Not an error: an archive that defines none of these symbols is a
        // fact the caller needs, and it is reported as exit 1 so a script
        // cannot mistake "nothing to delete" for "deleted successfully".
        eprintln!(
            "alloc-runner ar-members: no member of {} defines any of {:?}",
            archive, symbols
        );
        return EXIT_FAILED;
    }
    EXIT_OK
}

fn cmd_patch_rg(f: &Flags) -> u8 {
    let Some(src) = one(f, "src") else {
        eprintln!("alloc-runner patch-rg: --src is required");
        return EXIT_CANNOT_RUN;
    };
    let root = Path::new(src);
    if !root.join("crates/core/main.rs").exists() {
        eprintln!(
            "alloc-runner patch-rg: {} does not look like a ripgrep checkout (no crates/core/main.rs)",
            src
        );
        return EXIT_CANNOT_RUN;
    }
    let before = patchrg::count_global_allocators(root);
    let shim = match (one(f, "shim-path"), one(f, "shim-feature")) {
        (Some(p), Some(feat)) => Some((p, feat)),
        (None, None) => None,
        _ => {
            eprintln!("alloc-runner patch-rg: --shim-path and --shim-feature go together");
            return EXIT_CANNOT_RUN;
        }
    };
    match patchrg::patch(root, shim) {
        Ok(r) => {
            emit(&J::obj(vec![
                ("global_allocators_before", J::U(before as u64)),
                ("stripped_items", J::U(r.stripped_items as u64)),
                ("stripped_jemalloc_dep", J::Bool(r.stripped_dep)),
                ("inserted_shim", J::Bool(r.inserted)),
                ("global_allocators_after", J::U(r.final_count as u64)),
            ]));
            EXIT_OK
        }
        Err(e) => {
            eprintln!("alloc-runner patch-rg: {}", e);
            EXIT_FAILED
        }
    }
}

fn cmd_aslr_probe(f: &Flags) -> u8 {
    let (Some(bin), Some(corpus_dir)) = (one(f, "bin"), one(f, "corpus")) else {
        eprintln!("alloc-runner aslr-probe: --bin and --corpus are required");
        return EXIT_CANNOT_RUN;
    };
    // The `nomatch` workload is used because it runs long enough to be sampled
    // and produces no output to compete with.
    let data = Path::new(corpus_dir)
        .join(corpus::DATA_SUBDIR)
        .to_string_lossy()
        .into_owned();
    let args: Vec<String> = [
        "--no-ignore",
        "--hidden",
        "--no-messages",
        "QQQQ-ZZZZ-NO-SUCH-TOKEN",
        &data,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let runs = num(f, "runs", 6) as usize;
    match aslr::probe(Path::new(bin), &args, runs) {
        Ok(obs) => {
            emit(&obs.to_json());
            if obs.sampled == 0 {
                eprintln!(
                    "alloc-runner aslr-probe: never caught the child mapped; nothing observed"
                );
                return EXIT_CANNOT_RUN;
            }
            match one(f, "expect") {
                Some("randomised") if !obs.randomised() => {
                    eprintln!(
                        "alloc-runner aslr-probe: expected a moving base, saw {} distinct in {} samples",
                        obs.distinct(),
                        obs.sampled
                    );
                    EXIT_FAILED
                }
                Some("fixed") if obs.randomised() => {
                    eprintln!(
                        "alloc-runner aslr-probe: expected a fixed base, saw {} distinct",
                        obs.distinct()
                    );
                    EXIT_FAILED
                }
                _ => EXIT_OK,
            }
        }
        Err(e) => {
            eprintln!("alloc-runner aslr-probe: {}", e);
            EXIT_CANNOT_RUN
        }
    }
}

fn cmd_workloads() -> u8 {
    emit(&J::arr(
        WORKLOADS
            .iter()
            .map(|w| {
                J::obj(vec![
                    ("name", J::s(w.name)),
                    ("why", J::s(w.why)),
                    ("args", J::arr(w.args.iter().map(|a| J::s(*a)).collect())),
                    (
                        "accept_exit",
                        J::arr(w.accept.iter().map(|c| J::I(*c as i64)).collect()),
                    ),
                ])
            })
            .collect(),
    ));
    EXIT_OK
}

fn cmd_measure(f: &Flags) -> u8 {
    let (Some(bin), Some(corpus_dir), Some(wname)) =
        (one(f, "bin"), one(f, "corpus"), one(f, "workload"))
    else {
        eprintln!("alloc-runner measure: --bin, --corpus and --workload are required");
        return EXIT_CANNOT_RUN;
    };
    let Some(w) = WORKLOADS.iter().find(|w| w.name == wname) else {
        eprintln!(
            "alloc-runner measure: unknown workload {:?}; known: {}",
            wname,
            WORKLOADS
                .iter()
                .map(|w| w.name)
                .collect::<Vec<_>>()
                .join(", ")
        );
        return EXIT_CANNOT_RUN;
    };
    if !Path::new(bin).exists() {
        eprintln!("alloc-runner measure: no such binary: {}", bin);
        return EXIT_CANNOT_RUN;
    }
    let data = Path::new(corpus_dir).join(corpus::DATA_SUBDIR);
    let onefile = data.join("d000/f0000.txt");
    if !onefile.exists() {
        eprintln!(
            "alloc-runner measure: corpus incomplete: {} missing",
            onefile.display()
        );
        return EXIT_CANNOT_RUN;
    }

    let repeat = num(f, "repeat", 10).max(1) as usize;
    // ⚠ Warm-up runs are DISCARDED, not averaged in. The first run pays for a
    // cold page cache over a 65 MB corpus, which is a filesystem measurement,
    // not an allocator one.
    let warmup = num(f, "warmup", 2) as usize;
    let timeout = num(f, "timeout", 300);

    let argv: Vec<String> = std::iter::once(bin.to_string())
        .chain(w.args.iter().map(|a| {
            a.replace("{corpus}", &data.to_string_lossy())
                .replace("{onefile}", &onefile.to_string_lossy())
        }))
        .collect();

    let extra_env: Vec<(String, String)> = f
        .get("env")
        .map(|vs| {
            vs.iter()
                .filter_map(|kv| {
                    kv.split_once('=')
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    let mut samples = Vec::new();
    let mut failures = 0usize;
    for i in 0..(warmup + repeat) {
        match measure::run_once(&argv, &extra_env, None, timeout) {
            Ok(s) => {
                if i >= warmup {
                    if !s.ok(w.accept) {
                        failures += 1;
                    }
                    samples.push(s);
                } else if !s.ok(w.accept) {
                    // A warm-up that fails is still a failure; it just is not a
                    // timing sample. Reporting it keeps a configuration that
                    // only ever fails from looking like it produced no data.
                    failures += 1;
                }
            }
            Err(e) => {
                eprintln!("alloc-runner measure: {}", e);
                return EXIT_CANNOT_RUN;
            }
        }
    }

    let walls: Vec<f64> = samples
        .iter()
        .filter(|s| s.ok(w.accept))
        .map(|s| s.wall_ns as f64 / 1e9)
        .collect();
    let rss: Vec<f64> = samples
        .iter()
        .filter(|s| s.ok(w.accept))
        .map(|s| s.maxrss_kb as f64)
        .collect();

    let wall_stats = measure::stats(&walls);
    let rss_stats = measure::stats(&rss);

    emit(&J::obj(vec![
        ("workload", J::s(w.name)),
        ("binary", J::s(bin)),
        (
            "argv",
            J::arr(argv.iter().map(|a| J::s(a.clone())).collect()),
        ),
        ("repeat", J::U(repeat as u64)),
        ("warmup", J::U(warmup as u64)),
        ("failures", J::U(failures as u64)),
        ("samples_ok", J::U(walls.len() as u64)),
        (
            "wall_s",
            wall_stats.as_ref().map(|s| s.to_json()).unwrap_or(J::Null),
        ),
        (
            "maxrss_kb",
            rss_stats.as_ref().map(|s| s.to_json()).unwrap_or(J::Null),
        ),
        (
            "samples",
            J::arr(samples.iter().map(|s| s.to_json()).collect()),
        ),
    ]));

    if failures > 0 || walls.is_empty() {
        EXIT_FAILED
    } else {
        EXIT_OK
    }
}

fn cmd_archive_check(f: &Flags) -> u8 {
    let Some(archive) = one(f, "archive") else {
        eprintln!("alloc-runner archive-check: --archive is required");
        return EXIT_CANNOT_RUN;
    };
    let symbol = one(f, "symbol").unwrap_or("malloc");
    let providers = match ar::definers(Path::new(archive), symbol) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("alloc-runner archive-check: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    let expect = num(f, "expect-providers", 1) as usize;
    let ok = providers.len() == expect;
    emit(&J::obj(vec![
        ("archive", J::s(archive)),
        ("symbol", J::s(symbol)),
        ("expect_providers", J::U(expect as u64)),
        ("found_providers", J::U(providers.len() as u64)),
        (
            "members",
            J::arr(providers.iter().map(|p| J::s(p.member.clone())).collect()),
        ),
        ("ok", J::Bool(ok)),
    ]));
    if !ok {
        eprintln!(
            "alloc-runner archive-check: {} definition(s) of {} in {}, expected {}: {:?}",
            providers.len(),
            symbol,
            archive,
            expect,
            providers.iter().map(|p| &p.member).collect::<Vec<_>>()
        );
        return EXIT_FAILED;
    }
    EXIT_OK
}

// ---------------------------------------------------------------------------

fn cmd_selftest() -> u8 {
    let mut fails = 0;
    let mut check = |name: &str, ok: bool, detail: String| {
        println!(
            "  {}  {} {}",
            if ok { "ok  " } else { "FAIL" },
            name,
            detail
        );
        if !ok {
            fails += 1;
        }
    };

    // The generator must be a pure function of (seed, profile). If it is not,
    // two hosts cannot be compared and nothing downstream means anything.
    let a = corpus::generate(None, 42, corpus::profile("smoke").unwrap()).unwrap();
    let b = corpus::generate(None, 42, corpus::profile("smoke").unwrap()).unwrap();
    check(
        "corpus-deterministic",
        a.digest == b.digest && a.literal_lines == b.literal_lines,
        format!("digest {:016x}", a.digest),
    );

    let c = corpus::generate(None, 43, corpus::profile("smoke").unwrap()).unwrap();
    check(
        "corpus-seed-sensitive",
        a.digest != c.digest,
        format!("{:016x} vs {:016x}", a.digest, c.digest),
    );

    // A positive control for the truth itself: the smoke corpus must actually
    // contain planted needles. A generator that planted none would make every
    // "expected 0, got 0" check pass for the wrong reason.
    check(
        "corpus-plants-needles",
        a.literal_lines > 0 && a.regex_lines > 0 && a.unicode_lines > 0,
        format!(
            "literal={} regex={} unicode={}",
            a.literal_lines, a.regex_lines, a.unicode_lines
        ),
    );

    // The ELF reader must refuse a non-ELF rather than report an empty one:
    // "no allocator symbols found" and "this is not a binary" must not read
    // the same.
    check(
        "elf-rejects-non-elf",
        elf::parse_bytes(b"not an elf at all, not even close").is_err(),
        String::new(),
    );

    // And it must read the one ELF that is certainly present: itself.
    match std::env::current_exe()
        .map_err(|e| e.to_string())
        .and_then(|p| elf::parse(&p))
    {
        Ok(e) => check(
            "elf-reads-self",
            !e.syms.is_empty() || !e.had_symtab,
            format!(
                "{} {} symbols={}",
                elf::machine_name(e.machine),
                e.kind.as_str(),
                e.syms.len()
            ),
        ),
        Err(e) => check("elf-reads-self", false, e),
    }

    // The statistics must be robust where they claim to be. One wild outlier
    // moves the mean and must not move the median.
    let s = measure::stats(&[1.0, 1.0, 1.0, 1.0, 100.0]).unwrap();
    check(
        "stats-median-robust",
        (s.median - 1.0).abs() < 1e-9 && s.mean > 10.0,
        format!("median={} mean={:.1}", s.median, s.mean),
    );

    // Measurement must observe a real failure as a failure.
    match measure::run_once(
        &["/bin/sh".into(), "-c".into(), "exit 3".into()],
        &[],
        None,
        30,
    ) {
        Ok(smp) => check(
            "measure-sees-exit-code",
            smp.exit_code == 3 && smp.signal.is_none() && smp.wall_ns > 0,
            format!("exit={} wall_ns={}", smp.exit_code, smp.wall_ns),
        ),
        Err(e) => check("measure-sees-exit-code", false, e),
    }

    // ...and a signal as a signal, not as a fast run.
    match measure::run_once(
        &["/bin/sh".into(), "-c".into(), "kill -SEGV $$".into()],
        &[],
        None,
        30,
    ) {
        Ok(smp) => check(
            "measure-sees-signal",
            smp.signal == Some(libc::SIGSEGV) && !smp.ok(&[0]),
            format!("signal={:?}", smp.signal),
        ),
        Err(e) => check("measure-sees-signal", false, e),
    }

    // The timeout must fire, or a deadlocked allocator would hang the run.
    match measure::run_once(
        &["/bin/sh".into(), "-c".into(), "sleep 30".into()],
        &[],
        None,
        1,
    ) {
        Ok(smp) => check(
            "measure-timeout-kills",
            smp.signal == Some(libc::SIGKILL) && smp.wall_ns < 5_000_000_000,
            format!(
                "signal={:?} wall_s={:.2}",
                smp.signal,
                smp.wall_ns as f64 / 1e9
            ),
        ),
        Err(e) => check("measure-timeout-kills", false, e),
    }

    println!(
        "alloc-runner selftest: {} check(s), {} failure(s).",
        9, fails
    );
    if fails == 0 {
        EXIT_OK
    } else {
        EXIT_FAILED
    }
}
