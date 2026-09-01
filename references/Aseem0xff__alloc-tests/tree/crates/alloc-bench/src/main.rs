//! alloc-bench -- the host orchestrator.
//!
//! ⛔ EXIT CODES, the same convention the experiment scripts use:
//!   0  it ran and the thing passed
//!   1  it ran and the thing FAILED (a validation error, a failed cell under
//!      --strict)
//!   2  it could not run (no container runtime, missing input, bad arguments)
//!
//! ⚠ A slow allocator is never a non-zero exit. Only a broken *experiment* is.

mod envinfo;
mod exec;
mod model;
mod plan;
mod rank;
mod report;
mod run;
mod svg;
mod update;
mod validate;

use model::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_FAILED: u8 = 1;
const EXIT_CANNOT_RUN: u8 = 2;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() < 2 {
        usage();
        return ExitCode::from(EXIT_CANNOT_RUN);
    }
    let f = Flags::parse(&argv[2..]);

    // ⛔ `alloc-bench run --help` used to EXECUTE THE DEFAULT SUITE. Help was
    // matched only at argv[1], so with a subcommand present `--help` fell
    // through to `Flags::parse`, became an ordinary flag that `cmd_run` never
    // reads, and the tool built an image and measured a suite. Asking what a
    // command does and asking it to do it are different requests -- the same
    // distinction §7's exit codes draw between "could not look" and "looked and
    // found nothing" -- and the failure is silent and expensive: the caller
    // wanted one screen of text and got a container build.
    //
    // The help flag is therefore answered wherever it appears, before any
    // subcommand dispatches. `-h` is scanned in the raw argv because
    // `Flags::parse` only recognises `--` prefixes and would drop it.
    if f.has("help") || argv[2..].iter().any(|a| a == "-h") {
        usage();
        return ExitCode::from(EXIT_OK);
    }

    let code = match argv[1].as_str() {
        "doctor" => cmd_doctor(&f),
        "update" => cmd_update(&f),
        "plan" => cmd_plan(&f),
        "run" => cmd_run(&f),
        "validate" => cmd_validate(&f),
        "report" => cmd_report(&f),
        "-h" | "--help" | "help" => {
            usage();
            EXIT_OK
        }
        other => {
            eprintln!("alloc-bench: unknown subcommand: {}", other);
            usage();
            EXIT_CANNOT_RUN
        }
    };
    ExitCode::from(code)
}

fn usage() {
    eprintln!(
        r#"alloc-bench -- orchestrator for the allocator benchmark

  doctor                          check this host can run the benchmark
  update   [--write]              resolve upstream "latest" into allocators.lock.json
  plan     --suite ID[,ID|all]    expand the matrix and print the cells
  run      --suite ID[,ID|all]    build, verify and measure every cell
  validate --run DIR              re-check a dataset
  report   --run DIR              regenerate tables and graphs from a dataset

Common flags
  --root DIR        repository root (default: the current directory)
  --out DIR         where results go (default: results/local/<run-id>)
  --runtime NAME    docker | podman (default: whichever answers `info`)
  --arch A[,B]      restrict to architectures
  --distro D[,E]    restrict to distributions
  --allocator A[,B] restrict to allocators
  --seed N          corpus seed (default 20260901)
  --repeat N        override the suite's repetition count
  --keep-going      do not stop at the first failing cell (default: on)
  --strict          exit non-zero if any planned cell failed
  --allow-emulation run cells whose architecture is not the host's

Exit: 0 ran and passed, 1 ran and failed, 2 could not run."#
    );
}

// ---------------------------------------------------------------------------

struct Flags(BTreeMap<String, Vec<String>>);

impl Flags {
    fn parse(args: &[String]) -> Flags {
        let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut i = 0;
        while i < args.len() {
            if let Some(name) = args[i].strip_prefix("--") {
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    m.entry(name.into()).or_default().push(args[i + 1].clone());
                    i += 2;
                } else {
                    m.entry(name.into()).or_default().push("true".into());
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        Flags(m)
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.0.get(k).and_then(|v| v.first()).map(|s| s.as_str())
    }
    fn has(&self, k: &str) -> bool {
        self.0.contains_key(k)
    }
    fn list(&self, k: &str) -> Vec<String> {
        self.get(k)
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }
    fn root(&self) -> PathBuf {
        PathBuf::from(self.get("root").unwrap_or("."))
    }
    fn num(&self, k: &str, d: u64) -> u64 {
        self.get(k).and_then(|s| s.parse().ok()).unwrap_or(d)
    }
}

/// The schema versions this build understands.
///
/// ⚠ Checked rather than ignored. A configuration written for a later schema
/// would otherwise be parsed with today's rules, silently dropping whatever the
/// new version added -- and a dropped dimension is a matrix that quietly
/// measures something else.
const SCHEMA_VERSION: u32 = 1;

fn check_schema(file: &Path, got: u32) -> Result<(), String> {
    if got != SCHEMA_VERSION {
        return Err(format!(
            "{}: schema_version is {}, but this build understands {}. \
             Update alloc-bench, or the file.",
            file.display(),
            got,
            SCHEMA_VERSION
        ));
    }
    Ok(())
}

fn load_manifest(root: &Path) -> Result<Manifest, String> {
    let p = root.join("allocators/allocators.toml");
    let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {}", p.display(), e))?;
    let m: Manifest = toml::from_str(&s).map_err(|e| format!("{}: {}", p.display(), e))?;
    check_schema(&p, m.schema_version)?;
    Ok(m)
}

fn load_matrix(root: &Path) -> Result<MatrixFile, String> {
    let p = root.join("benchmarks/matrix.toml");
    let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {}", p.display(), e))?;
    let m: MatrixFile = toml::from_str(&s).map_err(|e| format!("{}: {}", p.display(), e))?;
    check_schema(&p, m.schema_version)?;
    Ok(m)
}

fn load_lock(root: &Path) -> Result<Lock, String> {
    let p = root.join("allocators/allocators.lock.json");
    let s = std::fs::read_to_string(&p).map_err(|e| {
        format!(
            "{}: {}. ⛔ There is no lock file, so no revision is pinned. Run `alloc-bench update --write` first.",
            p.display(),
            e
        )
    })?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {}", p.display(), e))
}

// ---------------------------------------------------------------------------

fn cmd_doctor(f: &Flags) -> u8 {
    let root = f.root();
    let mut bad = 0;
    let mut say = |ok: bool, what: &str, detail: String| {
        println!(
            "  {}  {:<28} {}",
            if ok { "ok  " } else { "FAIL" },
            what,
            detail
        );
        if !ok {
            bad += 1;
        }
    };

    match exec::Runtime::detect(f.get("runtime")) {
        Ok(rt) => say(
            true,
            "container runtime",
            format!("{} server {}", rt.bin, rt.version),
        ),
        Err(e) => say(false, "container runtime", e.replace('\n', " ")),
    }
    for tool in ["git", "curl"] {
        match exec::run(tool, &["--version".into()], None) {
            Ok(o) if o.ok() => say(true, tool, o.stdout.lines().next().unwrap_or("").into()),
            _ => say(false, tool, "not found on PATH".into()),
        }
    }
    match load_manifest(&root) {
        Ok(mf) => say(
            true,
            "allocators.toml",
            format!("{} allocator(s)", mf.allocators.len()),
        ),
        Err(e) => say(false, "allocators.toml", e),
    }
    match load_matrix(&root) {
        Ok(mx) => say(true, "matrix.toml", format!("{} suite(s)", mx.suites.len())),
        Err(e) => say(false, "matrix.toml", e),
    }
    match load_lock(&root) {
        Ok(l) => say(
            true,
            "allocators.lock.json",
            format!("{} pin(s), resolved {}", l.entries.len(), l.resolved_at),
        ),
        Err(e) => say(false, "allocators.lock.json", e),
    }

    // ⚠ Free space, because a run that dies half way through leaves a dataset
    // that looks partial rather than failed. Several images plus source trees
    // plus a 65 MB corpus is tens of gigabytes.
    match exec::run("df", &["-Pk".into(), root.display().to_string()], None) {
        Ok(o) if o.ok() => {
            let avail_kb: u64 = o
                .stdout
                .lines()
                .nth(1)
                .and_then(|l| l.split_whitespace().nth(3))
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let gb = avail_kb / 1024 / 1024;
            say(
                gb >= 20,
                "free disk",
                format!("{} GiB available (20 GiB recommended)", gb),
            );
        }
        _ => say(true, "free disk", "could not determine".into()),
    }

    println!("\nalloc-bench doctor: {} problem(s).", bad);
    if bad == 0 {
        EXIT_OK
    } else {
        EXIT_CANNOT_RUN
    }
}

fn cmd_update(f: &Flags) -> u8 {
    let root = f.root();
    let manifest = match load_manifest(&root) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("alloc-bench update: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    let lock = match update::resolve(&manifest) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("alloc-bench update: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    let json = serde_json::to_string_pretty(&lock).unwrap_or_default();
    if f.has("write") {
        let p = root.join("allocators/allocators.lock.json");
        if let Err(e) = std::fs::write(&p, format!("{}\n", json)) {
            eprintln!("alloc-bench update: {}: {}", p.display(), e);
            return EXIT_CANNOT_RUN;
        }
        eprintln!(
            "alloc-bench update: wrote {} ({} pins)",
            p.display(),
            lock.entries.len()
        );
        // The diff is the reviewable artefact: a changed commit here changes
        // what every future result means.
        for (id, e) in &lock.entries {
            eprintln!(
                "  {:<18} {} {} -> {}",
                id,
                e.kind,
                e.reference,
                &e.commit[..12.min(e.commit.len())]
            );
        }
    } else {
        println!("{}", json);
    }
    EXIT_OK
}

/// Reject a filter value that names nothing the project knows about.
///
/// ⚠ Being absent from the *selected* suites is fine and is not checked here;
/// only being absent from the whole matrix is an error. See the call site.
fn check_filter(flag: &str, given: &[String], mut known: Vec<String>) -> Result<(), String> {
    known.sort();
    known.dedup();
    let bad: Vec<&str> = given
        .iter()
        .filter(|g| !known.iter().any(|k| k == *g))
        .map(|g| g.as_str())
        .collect();
    if bad.is_empty() {
        return Ok(());
    }
    Err(format!(
        "unknown {} {:?}; known: {}",
        flag,
        bad.join(", "),
        known.join(", ")
    ))
}

fn build_plan(f: &Flags) -> Result<(Vec<Cell>, Manifest), String> {
    let root = f.root();
    let manifest = load_manifest(&root)?;
    let matrix = load_matrix(&root)?;
    let suites = {
        let s = f.list("suite");
        if s.is_empty() {
            vec!["smoke".to_string()]
        } else {
            s
        }
    };
    let planner = plan::Planner {
        manifest: &manifest,
        matrix: &matrix,
    };
    let mut cells = planner.plan(&suites)?;

    let arches = f.list("arch");
    let distros = f.list("distro");
    let allocs = f.list("allocator");

    // ⛔ A MISSPELLED FILTER MUST NOT LOOK LIKE AN EMPTY ANSWER.
    //
    // `--suite nosuch` already errors and lists the known suites, but
    // `--arch riscv64` used to `retain` its way to zero cells and exit 0 with
    // `[]`. A caller then sees "0 cells" and cannot tell a typo from a suite
    // that genuinely has nothing for that architecture -- the same "a
    // configuration vanished and looks like one nobody thought of" failure the
    // matrix CI check exists to catch. Observed for real: bench.yml run 1
    // dispatched `smoke` for aarch64 and the job failed with "the filters
    // selected no cells", which was correct but unreadable.
    //
    // ⭐ The universe is the whole matrix, NOT the selected suites. That is the
    // distinction that makes this useful: `--arch aarch64 --suite smoke` is a
    // legitimate empty result (smoke is x86_64-only) and stays empty, while
    // `--arch riscv64` is a mistake and says so.
    check_filter("arch", &arches, matrix.all_arches())?;
    check_filter("distro", &distros, matrix.all_distros())?;
    check_filter(
        "allocator",
        &allocs,
        manifest.allocators.iter().map(|a| a.id.clone()).collect(),
    )?;
    if !arches.is_empty() {
        cells.retain(|c| arches.contains(&c.arch));
    }
    if !distros.is_empty() {
        cells.retain(|c| {
            distros.contains(&c.distro)
                || distros
                    .iter()
                    .any(|d| plan::effective_distro(d, &c.arch) == c.distro)
        });
    }
    if !allocs.is_empty() {
        // The control is always kept: without it no ratio can be computed, and
        // a table of absolute times from one machine is not the deliverable.
        cells.retain(|c| allocs.contains(&c.allocator) || c.is_baseline());
    }
    if let Some(r) = f.get("repeat").and_then(|s| s.parse::<u32>().ok()) {
        for c in &mut cells {
            c.repeat = r;
        }
    }
    Ok((cells, manifest))
}

fn cmd_plan(f: &Flags) -> u8 {
    let (cells, _) = match build_plan(f) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("alloc-bench plan: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    // ⭐ Print the question each suite exists to answer. A matrix whose reason
    // for existing lives only in a config file is one nobody re-reads.
    if let Ok(mx) = load_matrix(&f.root()) {
        let used: std::collections::BTreeSet<&str> =
            cells.iter().map(|c| c.suite.as_str()).collect();
        for s in mx.suites.iter().filter(|s| used.contains(s.id.as_str())) {
            eprintln!("\n== suite {} ==\n{}", s.id, s.why.trim());
        }
    }
    let planned = cells.iter().filter(|c| c.status == "planned").count();
    println!(
        "{}",
        serde_json::to_string_pretty(&cells).unwrap_or_default()
    );
    eprintln!(
        "alloc-bench plan: {} cell(s): {} planned, {} unsupported",
        cells.len(),
        planned,
        cells.len() - planned
    );
    EXIT_OK
}

/// One built image, and what it says about itself. An empty `tag` means the
/// build failed; the cells for it are recorded as failed rather than skipped.
#[derive(Clone, Default)]
struct Image {
    tag: String,
    env: BTreeMap<String, String>,
    digest: Option<String>,
}

fn run_id() -> String {
    envinfo::now_iso8601()
        .replace([':', '-'], "")
        .replace('T', "-")
        .replace('Z', "")
}

fn cmd_run(f: &Flags) -> u8 {
    let root = f.root();
    let (cells, _manifest) = match build_plan(f) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("alloc-bench run: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    if cells.is_empty() {
        eprintln!("alloc-bench run: the filters selected no cells");
        return EXIT_CANNOT_RUN;
    }
    let lock = match load_lock(&root) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("alloc-bench run: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    let rt = match exec::Runtime::detect(f.get("runtime")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("alloc-bench run: {}", e);
            eprintln!("alloc-bench run: install Docker or Podman; see docs/reproducing.md");
            return EXIT_CANNOT_RUN;
        }
    };

    let id = run_id();
    let out_root = f
        .get("out")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("results/local").join(&id));
    let cache_root = root.join(".cache");
    let _ = std::fs::create_dir_all(&out_root);
    let _ = std::fs::create_dir_all(&cache_root);

    // ⚠ Bind mounts must be absolute. A relative path is read by the container
    // runtime as a NAMED VOLUME, so the cell writes into a volume nobody reads
    // and every result comes back empty -- which looks exactly like a build
    // that produced nothing.
    let out_root = std::fs::canonicalize(&out_root).unwrap_or(out_root);
    let cache_root = std::fs::canonicalize(&cache_root).unwrap_or(cache_root);

    let host_arch = exec::run("uname", &["-m".into()], None)
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();
    let allow_emulation = f.has("allow-emulation");

    // ⚠ Refuse an emulated architecture unless asked. Running it silently would
    // put emulated timings in the same table as native ones.
    let mut cells = cells;
    if !allow_emulation {
        let before = cells.len();
        cells.retain(|c| c.arch == host_arch);
        if cells.len() < before {
            eprintln!(
                "alloc-bench run: skipping {} cell(s) whose architecture is not this host's ({}). \
                 Pass --allow-emulation to run them under binfmt; their timings will be recorded and \
                 excluded from ranking.",
                before - cells.len(),
                host_arch
            );
        }
    }
    if cells.is_empty() {
        eprintln!("alloc-bench run: nothing left to run on this host");
        return EXIT_CANNOT_RUN;
    }

    let target_arch = cells.first().map(|c| c.arch.clone());
    let meta = RunMeta {
        run_id: id.clone(),
        started_at: envinfo::now_iso8601(),
        suites: f.list("suite"),
        host: envinfo::host(&rt, target_arch.as_deref()),
        tool_versions: envinfo::tool_versions(),
        lock: lock.clone(),
        ci: envinfo::ci_info(),
        corpus_seed: f.num("seed", 20260901),
        git_commit: envinfo::git_commit(),
    };
    let _ = std::fs::write(
        out_root.join("run.json"),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    );
    let _ = std::fs::write(
        out_root.join("plan.json"),
        serde_json::to_string_pretty(&cells).unwrap_or_default(),
    );

    let runner = run::Runner {
        rt,
        repo_root: root.clone(),
        out_root: out_root.clone(),
        cache_root,
        corpus_seed: meta.corpus_seed,
        keep_going: !f.has("stop-on-failure"),
    };

    // Images first: one per (distribution, architecture), reused by every cell.
    let mut images: BTreeMap<(String, String), Image> = BTreeMap::new();
    for c in cells.iter().filter(|c| c.status == "planned") {
        let key = (c.distro.clone(), c.arch.clone());
        if images.contains_key(&key) {
            continue;
        }
        eprintln!("alloc-bench: building image for {}/{}", c.distro, c.arch);
        match runner.ensure_image(&c.distro, &c.arch) {
            Ok((tag, env)) => {
                let digest = runner.rt.digest(&tag);
                images.insert(key, Image { tag, env, digest });
            }
            Err(e) => {
                eprintln!(
                    "alloc-bench: image build FAILED for {}/{}: {}",
                    c.distro, c.arch, e
                );
                images.insert(key, Image::default());
            }
        }
    }

    let total = cells.len();
    let mut failures = 0usize;
    for (i, cell) in cells.iter().enumerate() {
        let key = (cell.distro.clone(), cell.arch.clone());
        let img = images.get(&key).cloned().unwrap_or_default();

        let result = if cell.status == "planned" && img.tag.is_empty() {
            let mut r = runner.run_cell(cell, "", None, BTreeMap::new(), &lock);
            r.outcome = "build_failed".into();
            r.detail = Some(format!(
                "the image for {}/{} did not build; see logs/image-{}-{}.log",
                cell.distro, cell.arch, cell.distro, cell.arch
            ));
            r
        } else {
            runner.run_cell(cell, &img.tag, img.digest, img.env, &lock)
        };

        eprintln!(
            "[{:>3}/{}] {:<58} {}",
            i + 1,
            total,
            cell.id,
            match result.outcome.as_str() {
                "ok" => "ok".to_string(),
                "unsupported" => format!("unsupported: {}", short(result.detail.as_deref())),
                o => format!("{}: {}", o, short(result.detail.as_deref())),
            }
        );
        if result.outcome != "ok" && result.outcome != "unsupported" {
            failures += 1;
        }
        if let Err(e) = runner.write_result(&result) {
            eprintln!("alloc-bench: could not record {}: {}", cell.id, e);
            return EXIT_CANNOT_RUN;
        }
        if failures > 0 && !runner.keep_going {
            eprintln!("alloc-bench: stopping at the first failure (--stop-on-failure)");
            break;
        }
    }

    let code = finish(&out_root, &cells, &meta, f);
    eprintln!("alloc-bench: results in {}", out_root.display());
    if f.has("strict") && failures > 0 {
        return EXIT_FAILED;
    }
    code
}

fn short(s: Option<&str>) -> String {
    s.unwrap_or("")
        .replace('\n', " ")
        .chars()
        .take(96)
        .collect()
}

/// Validate then report. Shared by `run`, `validate` and `report` so a dataset
/// is judged the same way whoever asks.
fn finish(out_root: &Path, plan: &[Cell], meta: &RunMeta, f: &Flags) -> u8 {
    let results = match run::load_results(&out_root.join("results")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("alloc-bench: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    let findings = validate::validate(plan, &results, meta);
    let (errors, warns) = report::severity_summary(&findings);

    let workload = f.get("workload").unwrap_or(rank::primary_workload());
    let rep = report::build(meta, plan, &results, &findings, workload);
    if let Err(e) = report::write(out_root, &rep) {
        eprintln!("alloc-bench: writing the report: {}", e);
        return EXIT_CANNOT_RUN;
    }

    eprintln!(
        "alloc-bench: validation -> {} error(s), {} warning(s); report at {}",
        errors,
        warns,
        out_root.join("report.md").display()
    );
    for fd in findings
        .iter()
        .filter(|f| f.severity == validate::Severity::Error)
    {
        eprintln!(
            "  ERROR {} {}: {}",
            fd.check,
            fd.cell.as_deref().unwrap_or("-"),
            short(Some(&fd.message))
        );
    }
    if errors > 0 {
        // ⛔ The report is still written -- the evidence of a broken run is the
        // point -- but the exit status says the ranking must not be trusted.
        return EXIT_FAILED;
    }
    EXIT_OK
}

fn load_run(dir: &Path) -> Result<(RunMeta, Vec<Cell>), String> {
    let meta: RunMeta = serde_json::from_str(
        &std::fs::read_to_string(dir.join("run.json"))
            .map_err(|e| format!("{}/run.json: {}", dir.display(), e))?,
    )
    .map_err(|e| format!("run.json: {}", e))?;
    let plan: Vec<Cell> = serde_json::from_str(
        &std::fs::read_to_string(dir.join("plan.json"))
            .map_err(|e| format!("{}/plan.json: {}", dir.display(), e))?,
    )
    .map_err(|e| format!("plan.json: {}", e))?;
    Ok((meta, plan))
}

fn cmd_validate(f: &Flags) -> u8 {
    let Some(dir) = f.get("run") else {
        eprintln!("alloc-bench validate: --run DIR is required");
        return EXIT_CANNOT_RUN;
    };
    let dir = PathBuf::from(dir);
    let (meta, plan) = match load_run(&dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("alloc-bench validate: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    let results = match run::load_results(&dir.join("results")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("alloc-bench validate: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    let findings = validate::validate(&plan, &results, &meta);
    for fd in &findings {
        println!(
            "{:<5} {:<26} {:<52} {}",
            fd.severity.as_str(),
            fd.check,
            fd.cell.as_deref().unwrap_or("-"),
            fd.message.replace('\n', " ")
        );
    }
    let (errors, warns) = report::severity_summary(&findings);
    println!(
        "\n{} error(s), {} warning(s) over {} result(s).",
        errors,
        warns,
        results.len()
    );
    if validate::has_errors(&findings) {
        EXIT_FAILED
    } else {
        EXIT_OK
    }
}

fn cmd_report(f: &Flags) -> u8 {
    let Some(dir) = f.get("run") else {
        eprintln!("alloc-bench report: --run DIR is required");
        return EXIT_CANNOT_RUN;
    };
    let dir = PathBuf::from(dir);
    let (meta, plan) = match load_run(&dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("alloc-bench report: {}", e);
            return EXIT_CANNOT_RUN;
        }
    };
    finish(&dir, &plan, &meta, f)
}
