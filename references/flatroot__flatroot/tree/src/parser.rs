//! The command-line grammar (clap): parses the process arguments into a
//! typed `Args` the executor acts on without re-interpreting.

use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::path::PathBuf;

use flatroot::arch::Arch;

/// Everything one invocation needs, so a run is fully determined at
/// start. Each setting comes from the command line or the environment, so
/// a person and an automated pipeline drive the same request.
#[derive(Parser)]
#[command(
  name = "FlatRoot",
  about = "Build Linux root filesystem directories from official distribution packages",
  version
)]
pub struct Args {
  /// Package source to fetch from, as `<distro>:<release>[@<date>]`
  /// (e.g. `debian:bookworm`, `ubuntu:noble@2024-06-15`,
  /// `alpine:edge`).
  #[arg(long, env = "FLATROOT_ARG_FROM")]
  pub from: Option<String>,

  /// Target architecture(s) for the rootfs contents,
  /// comma-separated for multiarch. Uses `uname -m` names:
  /// `x86_64`, `aarch64`, `armv7l`, `i686`, `riscv64`. Defaults
  /// to the host arch.
  #[arg(long, env = "FLATROOT_ARG_ARCH", default_value_t = Arch::from_host().as_uname().to_string())]
  pub arch: String,

  /// Retry attempts after a failed HTTP request (index fetches and
  /// downloads). `0` disables retries.
  #[arg(long, env = "FLATROOT_ARG_HTTP_RETRIES", default_value_t = 3)]
  pub http_retries: u32,

  /// Emit diagnostic warnings to stderr (quiet by default).
  #[arg(long, short = 'v', env = "FLATROOT_ARG_VERBOSE", default_value_t = false)]
  pub verbose: bool,

  /// The subcommand the executor will run.
  #[command(subcommand)]
  pub command: Command,
}

impl Args {
  /// Requires `--from` only at the point an action turns out to need it,
  /// so actions that need no source (listing backends, repackaging a tree)
  /// run without one. A missing source is rejected with an example.
  pub fn source_required(from: &Option<String>) -> anyhow::Result<&str> {
    from.as_deref().ok_or_else(|| {
      anyhow::anyhow!(
        "--from is required. Example: --from debian:bookworm, --from ubuntu:noble, --from arch:rolling, --from alpine:v3.21"
      )
    })
  }
}

/// Reads the process arguments under the declared grammar into a typed
/// `Args`.
pub fn parse() -> Args {
  Args::parse()
}

/// The subcommand this invocation runs, each with its own settings.
#[derive(Subcommand)]
pub enum Command {
  /// Build a rootfs from the named packages plus their transitive
  /// dependency closure.
  Install {
    /// Target directory for the rootfs. Created if absent; existing
    /// files are overwritten in place.
    #[arg(long, short = 'o', env = "FLATROOT_ARG_INSTALL_OUTPUT")]
    output: Option<PathBuf>,

    /// Soft-dependency kinds to also install (comma-separated).
    /// Defaults to none.
    #[arg(
      long,
      value_enum,
      value_delimiter = ',',
      env = "FLATROOT_ARG_INSTALL_WITH",
      default_values_t = Vec::<SoftDep>::new()
    )]
    with: Vec<SoftDep>,

    /// Post-install phases to run after extraction
    /// (comma-separated). `none` skips all; defaults to all phases.
    #[arg(
      long,
      value_enum,
      value_delimiter = ',',
      env = "FLATROOT_ARG_INSTALL_POSTINSTALL",
      default_values_t = vec![PostinstallPhase::Ldconfig, PostinstallPhase::Scripts, PostinstallPhase::Hooks]
    )]
    postinstall: Vec<PostinstallPhase>,

    /// Install only the listed packages, skipping dependency
    /// resolution and base packages.
    #[arg(long, env = "FLATROOT_ARG_INSTALL_NO_DEPS", default_value_t = false)]
    no_deps: bool,

    /// Number of parallel downloads. `1` forces sequential.
    #[arg(long, short = 'p', env = "FLATROOT_ARG_INSTALL_PARALLEL", default_value_t = 4)]
    parallel: usize,

    /// Comma-separated package names to drop from the install set.
    #[arg(long, env = "FLATROOT_ARG_INSTALL_EXCLUDE", default_value_t = String::new())]
    exclude: String,

    /// Match space for the patterns: `package` (default),
    /// `library`, or `path`.
    #[arg(
      long = "type",
      value_enum,
      env = "FLATROOT_ARG_INSTALL_TYPE",
      default_value = "package"
    )]
    match_type: MatchType,

    /// How patterns combine when each resolves to a set of owning
    /// packages (`--type path`/`library`): `any` (default) unions,
    /// `all` intersects. Rejected with `--type package`.
    #[arg(
      long = "match",
      value_enum,
      env = "FLATROOT_ARG_INSTALL_MATCH",
      default_value = "any"
    )]
    combine: MatchCombine,

    /// Patterns naming what to install. At least one required; a
    /// pattern matching nothing fails the run.
    #[arg(required = true)]
    patterns: Vec<String>,
  },

  /// Search the source's index for packages, libraries, or
  /// installed paths matching one or more glob patterns.
  Search {
    /// Match space: `package` (default), `library`, or `path`.
    #[arg(
      long = "type",
      value_enum,
      env = "FLATROOT_ARG_SEARCH_TYPE",
      default_value = "package"
    )]
    match_type: MatchType,

    /// How patterns combine for `--type path`/`library`: `any`
    /// (default) unions, `all` intersects. Rejected with `--type
    /// package`.
    #[arg(long = "match", value_enum, env = "FLATROOT_ARG_SEARCH_MATCH", default_value = "any")]
    combine: MatchCombine,

    /// Output format: `plain` (default) or `json`.
    #[arg(long, value_enum, env = "FLATROOT_ARG_SEARCH_FORMAT", default_value = "plain")]
    format: OutputFormat,

    /// Glob patterns to match (case-insensitive; `*`, `?`,
    /// `[abc]`). Without wildcards, matches exactly.
    #[arg(required = true, num_args = 1..)]
    patterns: Vec<String>,
  },

  /// Run a SQL query against the package index database.
  Query {
    /// Path to a `.sql` file. Omit or use `-` to read from stdin.
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Output format: `plain` (default) or `json`.
    #[arg(long, value_enum, env = "FLATROOT_ARG_QUERY_FORMAT", default_value = "plain")]
    format: OutputFormat,
  },

  /// Operations on remote backends.
  Remote {
    #[command(subcommand)]
    action: RemoteCommand,
  },

  /// Operations on distribution releases.
  Release {
    #[command(subcommand)]
    action: ReleaseCommand,
  },

  /// Export an existing rootfs directory to a portable container
  /// or archive format (OCI, tar, DwarFS, SquashFS).
  Export {
    /// Source rootfs directory to export.
    src_dir: PathBuf,

    /// Output file path.
    output: PathBuf,

    /// Output format. Inferred from the output extension when
    /// unambiguous; required for `.tar`.
    #[arg(long, env = "FLATROOT_ARG_EXPORT_FORMAT")]
    format: Option<ExportFormat>,

    /// Image tag for OCI format (e.g. `myapp:v1.0`). Required with
    /// `--format=oci`.
    #[arg(long, short = 't', env = "FLATROOT_ARG_EXPORT_TAG")]
    tag: Option<String>,
  },

  /// Analyse packages against the source's index.
  Analyze {
    #[command(subcommand)]
    action: AnalyzeCommand,
  },
}

/// Diagnostic questions about a package and the closure it pulls in,
/// answered without committing anything to disk.
#[derive(Subcommand)]
pub enum AnalyzeCommand {
  /// Walk the full dependency closure of packages — declared edges
  /// plus linker edges — flagging under-declarations.
  Trace {
    /// Match space: `package` (default), `library`, or `path`.
    #[arg(
      long = "type",
      value_enum,
      env = "FLATROOT_ARG_ANALYZE_TRACE_TYPE",
      default_value = "package"
    )]
    match_type: MatchType,

    /// How seed patterns combine for `--type path`/`library`: `any`
    /// (default) unions, `all` intersects. Rejected with `--type
    /// package`.
    #[arg(
      long = "match",
      value_enum,
      env = "FLATROOT_ARG_ANALYZE_TRACE_MATCH",
      default_value = "any"
    )]
    combine: MatchCombine,

    /// Output format: `plain` (default) or `json`.
    #[arg(long, value_enum, env = "FLATROOT_ARG_ANALYZE_TRACE_FORMAT", default_value = "plain")]
    format: AnalyzeFormat,

    /// Trace strategies to run (comma-separated): `declared`,
    /// `linker`, or both (default).
    #[arg(
      long,
      value_enum,
      value_delimiter = ',',
      env = "FLATROOT_ARG_ANALYZE_TRACE_STRATEGY",
      default_values_t = vec![TraceStrategy::Declared, TraceStrategy::Linker]
    )]
    strategy: Vec<TraceStrategy>,

    /// Soft-dependency kinds to widen the declared closure
    /// (comma-separated). No effect on `linker`. Defaults to none.
    #[arg(
      long,
      value_enum,
      value_delimiter = ',',
      env = "FLATROOT_ARG_ANALYZE_TRACE_WITH",
      default_values_t = Vec::<SoftDep>::new()
    )]
    with: Vec<SoftDep>,

    /// Glob patterns to match (case-insensitive; `*`, `?`,
    /// `[abc]`). Without wildcards, matches exactly.
    #[arg(required = true, num_args = 1..)]
    patterns: Vec<String>,
  },
}

/// Which rendering the caller wants: a flat line-oriented one to glance
/// at or filter, or a structured one another tool consumes losslessly.
#[derive(Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
  /// Dotted `KEY=VALUE` lines.
  Plain,
  /// Pretty-printed JSON.
  Json,
}

/// What patterns match against: package names, the libraries packages
/// ship, or the files they install — so a library or path search can
/// name the owning package.
#[derive(Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum MatchType {
  /// Match patterns against package names.
  Package,
  /// Match against libraries (`.so`, `.so.*`, sonames, `.a`);
  /// resolves to the owning package.
  Library,
  /// Match against installed paths without leading slash
  /// (`usr/bin/bash`); resolves to the owning package. Unsupported
  /// on apk.
  Path,
}

/// How multiple patterns combine when each resolves to a set of owning
/// packages: gather everything any names, or keep only what they all
/// share.
#[derive(Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum MatchCombine {
  /// Union the owner sets (default).
  Any,
  /// Intersect the owner sets.
  All,
}

impl MatchCombine {
  /// Folds the per-pattern owner sets into one: everything together for
  /// `Any`, only the common owners for `All`, in first-seen order with
  /// repeats collapsed so callers can pass raw owner lists.
  ///
  /// Worked example — two `--type path` patterns, where `usr/sbin/sendmail`
  /// is shipped by three packages and `usr/sbin/postfix` by one, so the
  /// caller passes `[["postfix", "exim4", "nullmailer"], ["postfix"]]`:
  /// `Any` yields `["postfix", "exim4", "nullmailer"]`; `All` yields
  /// `["postfix"]`.
  pub fn owners_fold(self, groups: &[Vec<String>]) -> Vec<String> {
    match self {
      // Union, dropping repeats while preserving first-seen order: `insert`
      // reports `true` only the first time a name is seen, so the stateful
      // filter keeps each owner exactly once.
      // e.g. [["postfix", "exim4"], ["postfix"]] -> ["postfix", "exim4"].
      MatchCombine::Any => {
        let mut seen: HashSet<String> = HashSet::new();
        groups
          .iter()
          .flatten()
          .filter(|&name| seen.insert(name.clone()))
          .cloned()
          .collect()
      }
      // Intersection: keep the first group's names that appear in every other
      // group, each emitted once (the second filter drops repeats the first
      // group may carry). An empty `groups` has no first group, so it folds to
      // nothing. e.g. [["postfix", "exim4"], ["postfix"]] -> ["postfix"].
      MatchCombine::All => match groups.split_first() {
        None => Vec::new(),
        Some((first, rest)) => {
          let mut seen: HashSet<String> = HashSet::new();
          first
            .iter()
            .filter(|&name| rest.iter().all(|group| group.contains(name)))
            .filter(|&name| seen.insert(name.clone()))
            .cloned()
            .collect()
        }
      },
    }
  }
}

/// Which dependency view to follow: what the metadata declares, what the
/// binaries link against, or both — so gaps between them become visible.
#[derive(Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum TraceStrategy {
  /// Walk the dependencies stated in package metadata.
  Declared,
  /// Follow `DT_NEEDED` sonames from each package's ELF binaries
  /// to their providers.
  Linker,
}

/// Which optional dependency kinds to also follow; none by default so a
/// rootfs does not grow past what is needed.
#[derive(Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum SoftDep {
  /// Recommended packages (Debian `Recommends:`).
  Recommends,
  /// Suggested/optional packages (Debian `Suggests:`, Arch
  /// `%OPTDEPENDS%`).
  Suggests,
}

/// Which post-install phases to run, down to `none` for a bare
/// extraction.
#[derive(Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum PostinstallPhase {
  /// Run `sbin/ldconfig` if present (skipped on musl distros).
  Ldconfig,
  /// Replay each package's post-install script in the sandbox.
  Scripts,
  /// Regenerate runtime caches (font, icon, MIME, GSettings, GTK,
  /// dconf, CA bundles).
  Hooks,
  /// Skip every post-install phase.
  None,
}

/// The formats a rootfs directory can be exported to.
#[derive(Clone, clap::ValueEnum)]
pub enum ExportFormat {
  /// OCI container image (loadable by `docker load`/`podman
  /// load`). Requires `--tag`.
  Oci,
  /// Gzip-compressed tar archive (`.tar.gz`).
  Tar,
  /// DwarFS compressed filesystem image. Requires `mkdwarfs` on
  /// `PATH`.
  Dwarfs,
  /// SquashFS compressed filesystem image. Requires `mksquashfs`
  /// on `PATH`.
  Sqfs,
}

/// Which encoding of the trace result the caller wants: flat lines or
/// structured output.
#[derive(Clone, clap::ValueEnum)]
pub enum AnalyzeFormat {
  /// Dotted `KEY=VALUE` lines.
  Plain,
  /// Pretty-printed JSON.
  Json,
}

/// Questions about which distribution backends the tool supports,
/// answerable without a source or the network.
#[derive(Subcommand)]
pub enum RemoteCommand {
  /// Show every supported distribution backend and its package
  /// format.
  List {
    /// Output format: `plain` (default) or `json`. Encodings are
    /// bijective.
    #[arg(long, value_enum, env = "FLATROOT_ARG_REMOTE_LIST_FORMAT", default_value = "plain")]
    format: OutputFormat,
  },
}

/// Questions about which releases a chosen distribution publishes and
/// what each offers.
#[derive(Subcommand)]
pub enum ReleaseCommand {
  /// Show the releases the source distro publishes, with version
  /// and libc metadata. Requires `--from`.
  List {
    /// Output format: `plain` (default) or `json`. Fields vary by
    /// distro.
    #[arg(long, value_enum, env = "FLATROOT_ARG_RELEASE_LIST_FORMAT", default_value = "plain")]
    format: OutputFormat,
  },
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Build per-pattern owner groups from a compact literal of name slices, so
  /// each case reads as the owner sets the resolver would hand the fold.
  fn groups(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows
      .iter()
      .map(|row| row.iter().map(|s| s.to_string()).collect())
      .collect()
  }

  // covers: CLI-075
  #[test]
  fn owners_fold_any_unions_preserving_first_seen_order() {
    let g = groups(&[&["postfix", "exim4"], &["nullmailer"]]);
    assert_eq!(MatchCombine::Any.owners_fold(&g), vec!["postfix", "exim4", "nullmailer"]);
  }

  // covers: CLI-076
  #[test]
  fn owners_fold_any_dedups_across_groups() {
    let g = groups(&[&["postfix", "exim4"], &["postfix"]]);
    assert_eq!(MatchCombine::Any.owners_fold(&g), vec!["postfix", "exim4"]);
  }

  // covers: CLI-077
  #[test]
  fn owners_fold_all_keeps_only_the_common_owner() {
    let g = groups(&[&["postfix", "exim4", "nullmailer"], &["postfix"]]);
    assert_eq!(MatchCombine::All.owners_fold(&g), vec!["postfix"]);
  }

  // covers: CLI-078
  #[test]
  fn owners_fold_all_is_empty_when_no_owner_is_common() {
    let g = groups(&[&["pkg-a"], &["pkg-b"]]);
    assert!(MatchCombine::All.owners_fold(&g).is_empty());
  }

  // covers: CLI-079
  #[test]
  fn owners_fold_all_single_group_is_identity() {
    let g = groups(&[&["postfix", "exim4"]]);
    assert_eq!(MatchCombine::All.owners_fold(&g), vec!["postfix", "exim4"]);
  }

  // covers: CLI-080
  #[test]
  fn owners_fold_all_preserves_first_group_order() {
    let g = groups(&[&["c", "a", "b"], &["a", "b", "c"]]);
    assert_eq!(MatchCombine::All.owners_fold(&g), vec!["c", "a", "b"]);
  }

  // covers: CLI-081
  #[test]
  fn owners_fold_all_drops_partial_membership() {
    // A name present in two of three groups is not common to all, so it is
    // dropped; only the name in every group survives.
    let g = groups(&[&["a", "b"], &["a", "b"], &["b"]]);
    assert_eq!(MatchCombine::All.owners_fold(&g), vec!["b"]);
  }

  // covers: CLI-082
  #[test]
  fn owners_fold_all_empty_group_yields_empty() {
    // One pattern resolving to no owner makes the intersection empty, even
    // when the other patterns matched — callers surface this as their own
    // empty-intersection outcome.
    let g = groups(&[&["postfix"], &[]]);
    assert!(MatchCombine::All.owners_fold(&g).is_empty());
  }

  // covers: CLI-083
  #[test]
  fn owners_fold_empty_groups_is_empty() {
    // Totality: no patterns at all folds to nothing under either mode rather
    // than panicking on an empty slice.
    let g: Vec<Vec<String>> = Vec::new();
    assert!(MatchCombine::Any.owners_fold(&g).is_empty());
    assert!(MatchCombine::All.owners_fold(&g).is_empty());
  }

  // covers: CLI-084
  #[test]
  fn owners_fold_any_tolerates_intragroup_dup() {
    // A wildcard pattern can match several files owned by the same package, so
    // a group may carry a repeat; the union fold still emits each name once.
    let g = groups(&[&["postfix", "postfix"], &["exim4"]]);
    assert_eq!(MatchCombine::Any.owners_fold(&g), vec!["postfix", "exim4"]);
  }

  // covers: CLI-098
  #[test]
  fn owners_fold_all_dedups_a_repeat_in_the_first_group() {
    // Same wildcard case for the intersection: a name repeated in the first
    // group, and common to every group, is emitted once.
    let g = groups(&[&["postfix", "postfix"], &["postfix"]]);
    assert_eq!(MatchCombine::All.owners_fold(&g), vec!["postfix"]);
  }
}
