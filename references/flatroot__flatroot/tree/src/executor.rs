//! Dispatches one parsed invocation to its command, resolving the
//! defaults the request left implicit and refusing self-contradictory
//! combinations before any work begins.

use anyhow::Result;

use flatroot::arch::Arch;
use flatroot::ui::RunVoice;

use crate::commands;
use crate::commands::session::Session;
use crate::parser;

/// Sets the run's verbosity, opens the `Session`, and routes the
/// request to its command with defaults resolved and
/// self-contradictory combinations refused.
pub async fn execute(args: parser::Args) -> Result<()> {
  RunVoice::verbose_set(args.verbose);
  let session = Session::open(args.http_retries)?;

  match args.command {
    parser::Command::Install {
      output,
      patterns,
      with,
      postinstall,
      no_deps,
      parallel,
      exclude,
      match_type,
      combine,
    } => {
      match_combine_admit(match_type, combine)?;
      let from_str = parser::Args::source_required(&args.from)?;
      let output = output.ok_or_else(|| {
        anyhow::anyhow!("Install output directory is required. Use -o <PATH> or set FLATROOT_ARG_INSTALL_OUTPUT")
      })?;
      commands::install::run(
        &session,
        commands::install::InstallArgs {
          remote_str: from_str.to_string(),
          root: output,
          archs: archs_parse(&args.arch)?,
          patterns,
          jobs: parallel,
          recommends: with.contains(&parser::SoftDep::Recommends),
          suggests: with.contains(&parser::SoftDep::Suggests),
          postinstall: postinstall_phases_from_cli(postinstall)?,
          no_deps,
          exclude: exclude_parse(&exclude),
          match_type,
          combine,
        },
      )
      .await
    }
    parser::Command::Remote { action } => match action {
      parser::RemoteCommand::List { format } => commands::remote::list(format),
    },
    parser::Command::Release { action } => match action {
      parser::ReleaseCommand::List { format } => {
        let from_str = parser::Args::source_required(&args.from)?.to_string();
        tokio::task::spawn_blocking(move || commands::release::list(&session, &from_str, format)).await?
      }
    },
    parser::Command::Search {
      patterns,
      match_type,
      combine,
      format,
    } => {
      match_combine_admit(match_type, combine)?;
      let from_str = parser::Args::source_required(&args.from)?.to_string();
      let arch = Arch::from_uname(&args.arch)?;
      tokio::task::spawn_blocking(move || {
        commands::search::run(&session, &from_str, arch, &patterns, match_type, combine, format)
      })
      .await?
    }
    parser::Command::Query { file, format } => {
      let from_str = parser::Args::source_required(&args.from)?.to_string();
      let arch = Arch::from_uname(&args.arch)?;
      let sql = commands::query::sql_read(file)?;
      tokio::task::spawn_blocking(move || commands::query::run(&session, &from_str, arch, &sql, format)).await?
    }
    parser::Command::Export {
      src_dir,
      output,
      format,
      tag,
    } => commands::export::ExportPlan::new(format, tag.as_deref()).run(&src_dir, &output),
    parser::Command::Analyze { action } => match action {
      parser::AnalyzeCommand::Trace {
        patterns,
        match_type,
        combine,
        format,
        strategy,
        with,
      } => {
        match_combine_admit(match_type, combine)?;
        trace_execute(
          &session,
          TraceCli {
            from: args.from,
            arch: args.arch,
            patterns,
            match_type,
            combine,
            format,
            strategy,
            with,
          },
        )
        .await
      }
    },
  }
}

/// One trace request: the `Analyze::Trace` fields plus the global source
/// and architecture, bundled so the dispatcher passes one value instead
/// of seven.
struct TraceCli {
  from: Option<String>,
  arch: String,
  patterns: Vec<String>,
  match_type: crate::parser::MatchType,
  combine: crate::parser::MatchCombine,
  format: parser::AnalyzeFormat,
  strategy: Vec<parser::TraceStrategy>,
  with: Vec<parser::SoftDep>,
}

/// Runs one trace and presents it: human framing on stderr, data on
/// stdout, the unresolved-soname summary last. The empty strategy set is
/// refused; the analyze architecture is the first `--arch` entry.
async fn trace_execute(session: &Session, cli: TraceCli) -> Result<()> {
  if cli.strategy.is_empty() {
    anyhow::bail!("--strategy requires at least one of: declared, linker");
  }
  let from_str = parser::Args::source_required(&cli.from)?;
  let arch_token = cli.arch.split(',').next().unwrap_or(&cli.arch);
  let arch = Arch::from_uname(arch_token)?;

  let outcome = (commands::analyze::AnalyzeArgs {
    remote_str: from_str,
    arch,
    patterns: &cli.patterns,
    match_type: cli.match_type,
    combine: cli.combine,
    run_declared: cli.strategy.contains(&parser::TraceStrategy::Declared),
    run_linker: cli.strategy.contains(&parser::TraceStrategy::Linker),
    include_recommends: cli.with.contains(&parser::SoftDep::Recommends),
    include_suggests: cli.with.contains(&parser::SoftDep::Suggests),
  })
  .run(session)
  .await?;

  // Framing goes to stderr — human context about what is being traced,
  // kept off stdout so a pipeline consumer sees only the trace data.
  analyze_framing_print(&outcome);
  let format = match cli.format {
    parser::AnalyzeFormat::Plain => parser::OutputFormat::Plain,
    parser::AnalyzeFormat::Json => parser::OutputFormat::Json,
  };
  outcome.render(format)?;
  unresolved_warn(&outcome);
  Ok(())
}

/// Trailing stderr summary: unresolved sonames are an actionable
/// diagnostic about index completeness.
fn unresolved_warn(outcome: &commands::analyze::AnalysisOutcome) {
  let unresolved_count = outcome.unresolved_count();
  if unresolved_count == 0 {
    return;
  }
  eprintln!();
  eprintln!("warning: {} soname(s) could not be resolved against the index", unresolved_count);
}

/// The comma-separated `--exclude` list, trimmed, with empty entries dropped.
fn exclude_parse(exclude: &str) -> Vec<String> {
  exclude
    .split(',')
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .collect()
}

/// The comma-separated `--arch` list parsed into the supported set; an
/// unknown token fails the whole invocation.
fn archs_parse(arch: &str) -> Result<Vec<Arch>> {
  arch.split(',').map(Arch::from_uname).collect()
}

/// Prints the trace's human framing to stderr — which packages, source,
/// architecture — kept apart from the stdout data a program consumes.
fn analyze_framing_print(outcome: &commands::analyze::AnalysisOutcome) {
  match outcome.seeds.as_slice() {
    [] => {
      eprintln!("Analyzing 0 packages from {} ({})", outcome.source, outcome.arch);
    }
    [single] => {
      eprintln!("Analyzing {} {} from {} ({})", single.name, single.version, outcome.source, outcome.arch);
    }
    many => {
      let total = many.len();
      const PREVIEW: usize = 5;
      let summary: String = if total > PREVIEW {
        let head: Vec<String> = many[..PREVIEW]
          .iter()
          .map(|s| format!("{} {}", s.name, s.version))
          .collect();
        format!("{}, ... ({} total)", head.join(", "), total)
      } else {
        many
          .iter()
          .map(|s| format!("{} {}", s.name, s.version))
          .collect::<Vec<_>>()
          .join(", ")
      };
      eprintln!("Analyzing {} packages from {} ({}): {}", total, outcome.source, outcome.arch, summary);
    }
  }
}

/// Turns the requested post-install phases into a phase list. An
/// explicit `none` and an empty choice both mean "run none"; `none`
/// mixed with named phases is self-contradictory and refused.
fn postinstall_phases_from_cli(cli_phases: Vec<parser::PostinstallPhase>) -> Result<Vec<flatroot::postinstall::Phase>> {
  let has_none = cli_phases.iter().any(|p| matches!(p, parser::PostinstallPhase::None));
  let has_phase = cli_phases.iter().any(|p| !matches!(p, parser::PostinstallPhase::None));
  if has_none && has_phase {
    anyhow::bail!(
      "--postinstall=none cannot be combined with phase values. \
       Use --postinstall=none alone to skip every phase, or list only phase values \
       (ldconfig, scripts, hooks)."
    );
  }
  if has_none || cli_phases.is_empty() {
    return Ok(Vec::new());
  }
  Ok(
    cli_phases
      .into_iter()
      .filter_map(|p| match p {
        parser::PostinstallPhase::Ldconfig => Some(flatroot::postinstall::Phase::Ldconfig),
        parser::PostinstallPhase::Scripts => Some(flatroot::postinstall::Phase::Scripts),
        parser::PostinstallPhase::Hooks => Some(flatroot::postinstall::Phase::Hooks),
        // `none` mixed with phases was refused above and a lone `none`
        // already returned empty; mapping it away keeps this arm total.
        parser::PostinstallPhase::None => None,
      })
      .collect(),
  )
}

/// `--match all` intersects the owning-package sets that path and library
/// patterns resolve to, which is only meaningful when a pattern maps
/// *indirectly* to candidate owners. A `--type package` pattern already names
/// its package outright, so there is no owner set to intersect — the request
/// contradicts itself. Rather than silently downgrade it to a union (doing
/// something other than what was asked), the combination is refused up front,
/// before any index is fetched.
fn match_combine_admit(match_type: parser::MatchType, combine: parser::MatchCombine) -> Result<()> {
  if matches!(combine, parser::MatchCombine::All) && matches!(match_type, parser::MatchType::Package) {
    anyhow::bail!(
      "--match all is meaningful only with --type path or --type library, where a pattern \
       resolves to a set of owning packages to intersect; a --type package pattern already \
       names the package, so there is nothing to intersect"
    );
  }
  Ok(())
}
