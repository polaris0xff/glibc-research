//! Runs the `analyze trace` command: `AnalyzeArgs` holds the parsed
//! request, and its `run` turns it into a finished `AnalysisOutcome`.

use anyhow::{Result, bail};

use flatroot::arch::Arch;
use flatroot::db::PackageRow;
use flatroot::distro::FormatPackage;
use flatroot::library::LibraryMatch;
use flatroot::path::PathMatch;

use crate::commands::session::Session;

use super::ctx::AnalysisContext;
use super::outcome::{AnalysisOutcome, TraceSeed};
use super::pass::declared::DeclaredPass;
use super::pass::linker::LinkerPass;

/// One parsed `analyze trace` request; every field comes straight from
/// the command line.
pub struct AnalyzeArgs<'a> {
  /// The `--from` source (e.g. `debian:bookworm`) naming distribution and
  /// release.
  pub remote_str: &'a str,
  /// Target architecture; analyze runs against one, so only the first of
  /// several `--arch` entries is taken.
  pub arch: Arch,
  /// Positional glob patterns, resolved to seed packages under
  /// `match_type` and combined per `combine`.
  pub patterns: &'a [String],
  /// Whether patterns match package names, library files, or installed
  /// paths.
  pub match_type: crate::parser::MatchType,
  /// How patterns combine when each resolves to a set of owners: `Any`
  /// unions, `All` intersects. No effect under `--type package`.
  pub combine: crate::parser::MatchCombine,
  /// Run the declared pass. Disabling both passes is refused by `run`.
  pub run_declared: bool,
  /// Run the linker pass.
  pub run_linker: bool,
  /// Follow `Recommends:` dependencies in the declared pass, for formats
  /// that publish them.
  pub include_recommends: bool,
  /// Follow `Suggests:` dependencies in the declared pass, for formats
  /// that publish them.
  pub include_suggests: bool,
}

impl AnalyzeArgs<'_> {
  /// Runs the trace: opens the package index, resolves the patterns to
  /// seed packages, runs each enabled pass, and merges the results into
  /// one `AnalysisOutcome`. Disabling both passes is an error; patterns
  /// that match no package produce an empty outcome, not an error.
  pub async fn run(self, session: &Session) -> Result<AnalysisOutcome> {
    if !self.run_declared && !self.run_linker {
      bail!("Neither the declared nor the linker analysis was requested — analyze has nothing to do");
    }

    // Analyze runs against exactly one architecture: the linker pass
    // reads individual ELF binaries, and each binary belongs to one
    // architecture. `main.rs` already picked the first `--arch` entry.
    let arch = self.arch;
    let arch_uname = arch.as_uname().to_string();

    // Opens the per-arch session: the distribution backend, the cached
    // package index, and the archive cache directory.
    let arch_ctx = session.context_open(self.remote_str, arch).await?;
    let index = &arch_ctx.index;
    let remote = &*arch_ctx.remote;

    let targets = self.targets_resolve(index, remote.format())?;
    if targets.is_empty() {
      // Every pattern matched zero packages: exit 0 with no stdout;
      // `main.rs` prints the empty-result note on stderr.
      return Ok(AnalysisOutcome::empty(self.remote_str.to_string(), arch_uname));
    }

    let ctx = AnalysisContext {
      index,
      remote,
      path_dir_cache: &arch_ctx.cache_dir,
      targets: &targets,
      arch: &arch_uname,
      http_retries: session.http().retries(),
    };

    let dep_tree = if self.run_declared {
      Some(DeclaredPass::run(
        &ctx,
        super::pass::declared::Options {
          include_recommends: self.include_recommends,
          include_suggests: self.include_suggests,
        },
      )?)
    } else {
      None
    };

    let linker = if self.run_linker {
      Some(LinkerPass::run(&ctx).await?)
    } else {
      None
    };

    let seeds: Vec<TraceSeed> = targets
      .iter()
      .map(|t| TraceSeed {
        name: t.name.clone(),
        version: t.version.clone(),
      })
      .collect();

    Ok(AnalysisOutcome::merge(self.remote_str.to_string(), arch_uname, seeds, dep_tree, linker))
  }

  /// Resolves the patterns to seed package rows: each pattern matches
  /// under `match_type`, and `combine` unions or intersects the
  /// per-pattern owner sets. An empty result is not an error. `--type
  /// path` is refused for apk, which publishes no file lists.
  fn targets_resolve(&self, index: &flatroot::db::Index, format: FormatPackage) -> Result<Vec<PackageRow>> {
    if matches!(self.match_type, crate::parser::MatchType::Path) && matches!(format, FormatPackage::Apk) {
      bail!(
        "--type path is unsupported for apk sources: Alpine publishes no file lists (APKINDEX carries no file-to-package data)"
      );
    }

    let mut groups: Vec<Vec<String>> = Vec::new();
    for pattern in self.patterns {
      let owners: Vec<String> = match self.match_type {
        crate::parser::MatchType::Package => index.packages().glob(pattern)?.into_iter().map(|r| r.name).collect(),
        crate::parser::MatchType::Library => LibraryMatch::glob(index, pattern)?
          .into_iter()
          .map(|m| m.package)
          .collect(),
        crate::parser::MatchType::Path => PathMatch::glob(index, pattern)?
          .into_iter()
          .map(|m| m.package)
          .collect(),
      };
      groups.push(owners);
    }
    let seed_names = self.combine.owners_fold(&groups);

    let mut targets: Vec<PackageRow> = Vec::with_capacity(seed_names.len());
    for name in &seed_names {
      let row = index
        .packages()
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("seed package '{}' missing from index after glob match", name))?;
      targets.push(row);
    }
    Ok(targets)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::commands::analyze::pass::linker::{LinkerProvider, LinkerWalkOutcome};
  use crate::commands::analyze::{AnalysisReason, ConsumedSoname, TraceEntry, UnresolvedSoname};
  use flatroot::dep_tree::{DepNode, DepTree};
  use flatroot::package::DepKind;
  use flatroot::resolver::DepEdge;
  use std::collections::BTreeMap;
  use std::path::PathBuf;

  fn bash_seed() -> Vec<TraceSeed> {
    vec![TraceSeed {
      name: "bash".into(),
      version: "5.2".into(),
    }]
  }

  fn outcome_build_default(
    seeds: Vec<TraceSeed>,
    dep_tree: Option<DepTree>,
    linker: Option<LinkerWalkOutcome>,
  ) -> AnalysisOutcome {
    AnalysisOutcome::merge("debian:bookworm".into(), "x86_64".into(), seeds, dep_tree, linker)
  }

  fn make_node(name: &str, version: &str, edges: &[(&str, DepKind)]) -> DepNode {
    DepNode {
      name: name.into(),
      version: version.into(),
      edges: edges
        .iter()
        .map(|(child, kind)| DepEdge {
          child: child.to_string(),
          kind: kind.clone(),
          picked_alternative: None,
          original_constraint: None,
        })
        .collect(),
    }
  }

  fn make_tree(root: &str, nodes: Vec<DepNode>) -> DepTree {
    let mut map = BTreeMap::new();
    for n in nodes {
      map.insert(n.name.clone(), n);
    }
    DepTree {
      roots: vec![root.into()],
      nodes: map,
    }
  }

  fn make_linker(entries: Vec<(&str, &str, &[&str])>) -> LinkerWalkOutcome {
    let mut providers = BTreeMap::new();
    for (name, version, sonames) in entries {
      providers.insert(
        name.into(),
        LinkerProvider {
          version: version.into(),
          sonames: sonames.iter().map(|s| s.to_string()).collect(),
        },
      );
    }
    LinkerWalkOutcome {
      providers,
      consumed: BTreeMap::new(),
      unresolved: BTreeMap::new(),
    }
  }

  fn entry<'a>(outcome: &'a AnalysisOutcome, name: &str) -> &'a TraceEntry {
    outcome
      .entries
      .iter()
      .find(|e| e.name == name)
      .unwrap_or_else(|| panic!("entry '{}' missing from outcome", name))
  }

  #[test]
  fn outcome_build_target_marked_when_in_both_passes() {
    let tree = make_tree(
      "bash",
      vec![
        make_node("bash", "5.2", &[("libc6", DepKind::Depends)]),
        make_node("libc6", "2.36", &[]),
      ],
    );
    let linker = make_linker(vec![("bash", "5.2", &[]), ("libc6", "2.36", &["libc.so.6"])]);

    let outcome = outcome_build_default(bash_seed(), Some(tree), Some(linker));
    assert_eq!(entry(&outcome, "bash").reason, AnalysisReason::Target);
    assert_eq!(entry(&outcome, "libc6").reason, AnalysisReason::DeclaredLinker);
    assert_eq!(entry(&outcome, "libc6").sonames_provided, vec!["libc.so.6".to_string()]);
    assert_eq!(entry(&outcome, "libc6").depends_on.len(), 0);
  }

  #[test]
  fn outcome_build_linker_only_package_marked_undeclared() {
    let tree = make_tree("bash", vec![make_node("bash", "5.2", &[])]);
    let linker = make_linker(vec![("bash", "5.2", &[]), ("undeclared-lib", "1.0", &["libfoo.so.1"])]);

    let outcome = outcome_build_default(bash_seed(), Some(tree), Some(linker));
    let undeclared = entry(&outcome, "undeclared-lib");
    assert_eq!(undeclared.reason, AnalysisReason::Linker);
    assert_eq!(undeclared.sonames_provided, vec!["libfoo.so.1".to_string()]);
  }

  #[test]
  fn outcome_build_declared_only_package_keeps_declared_reason() {
    let tree = make_tree(
      "bash",
      vec![
        make_node("bash", "5.2", &[("config-only", DepKind::Depends)]),
        make_node("config-only", "1.0", &[]),
      ],
    );
    let linker = make_linker(vec![("bash", "5.2", &[])]);

    let outcome = outcome_build_default(bash_seed(), Some(tree), Some(linker));
    assert_eq!(entry(&outcome, "config-only").reason, AnalysisReason::Declared);
    assert!(entry(&outcome, "config-only").sonames_provided.is_empty());
    assert_eq!(entry(&outcome, "bash").reason, AnalysisReason::Target);
  }

  #[test]
  fn outcome_build_unresolved_sonames_propagate_per_consumer() {
    let tree = make_tree("bash", vec![make_node("bash", "5.2", &[])]);
    let mut linker = make_linker(vec![("bash", "5.2", &[])]);
    linker.unresolved.insert(
      "bash".into(),
      vec![UnresolvedSoname {
        binary: PathBuf::from("bin/bash"),
        soname: "libmystery.so.1".into(),
      }],
    );

    let outcome = outcome_build_default(bash_seed(), Some(tree), Some(linker));
    assert_eq!(outcome.unresolved_count(), 1);
    let bash = entry(&outcome, "bash");
    assert_eq!(bash.unresolved_sonames.len(), 1);
    assert_eq!(bash.unresolved_sonames[0].soname, "libmystery.so.1");
  }

  #[test]
  fn outcome_build_consumed_sonames_attach_to_consumer() {
    let tree = make_tree(
      "bash",
      vec![
        make_node("bash", "5.2", &[("libc6", DepKind::Depends)]),
        make_node("libc6", "2.36", &[]),
      ],
    );
    let mut linker = make_linker(vec![("bash", "5.2", &[]), ("libc6", "2.36", &["libc.so.6"])]);
    linker.consumed.insert(
      "bash".into(),
      vec![ConsumedSoname {
        binary: PathBuf::from("usr/bin/bash"),
        provider: "libc6".into(),
        soname: "libc.so.6".into(),
      }],
    );

    let outcome = outcome_build_default(bash_seed(), Some(tree), Some(linker));
    let bash = entry(&outcome, "bash");
    assert_eq!(bash.sonames_consumed.len(), 1);
    assert_eq!(bash.sonames_consumed[0].provider, "libc6");
    assert_eq!(bash.sonames_consumed[0].soname, "libc.so.6");
  }

  #[test]
  fn outcome_build_no_deps_keeps_target_present() {
    let linker = make_linker(vec![("bash", "5.2", &[])]);
    let outcome = outcome_build_default(bash_seed(), None, Some(linker));
    assert_eq!(entry(&outcome, "bash").reason, AnalysisReason::Target);
  }

  #[test]
  fn outcome_build_consumer_only_seed_keeps_version() {
    // --strategy=linker with an executable seed: no declared pass, and
    // `bash` provides no consumed soname, so it enters only through the
    // `consumed` map (which has no version to give). The resolved seed
    // version must still land on the entry.
    let mut linker = make_linker(vec![]);
    linker.consumed.insert(
      "bash".into(),
      vec![ConsumedSoname {
        binary: PathBuf::from("bin/bash"),
        provider: "libc6".into(),
        soname: "libc.so.6".into(),
      }],
    );
    let outcome = outcome_build_default(bash_seed(), None, Some(linker));
    let bash = entry(&outcome, "bash");
    assert_eq!(bash.reason, AnalysisReason::Target);
    assert_eq!(bash.version, "5.2", "consumer-only seed must keep its resolved version");
  }

  #[test]
  fn outcome_build_no_linker_keeps_target_present() {
    let tree = make_tree("bash", vec![make_node("bash", "5.2", &[])]);
    let outcome = outcome_build_default(bash_seed(), Some(tree), None);
    assert_eq!(entry(&outcome, "bash").reason, AnalysisReason::Target);
  }

  #[test]
  fn outcome_build_entries_sorted_alphabetically_by_name() {
    // Build a tree rooted at `bash` so the target (also `bash`)
    // and the tree's root agree; the assertion focuses on whether
    // the merge produces alphabetically-ordered entries.
    let tree = make_tree(
      "bash",
      vec![
        make_node("bash", "5.2", &[("libc6", DepKind::Depends), ("acl", DepKind::Depends)]),
        make_node("libc6", "2.36", &[]),
        make_node("acl", "2.3", &[]),
      ],
    );
    let outcome = outcome_build_default(bash_seed(), Some(tree), None);
    let names: Vec<&str> = outcome.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["acl", "bash", "libc6"]);
  }

  // ── targets_resolve: --match combine ───────────────────────────────────

  use crate::parser::{MatchCombine, MatchType};
  use flatroot::arch::Arch;
  use flatroot::db::{Index, IndexWriter};
  use flatroot::distro::FormatPackage;
  use flatroot::package::{DepSpec, Package};
  use flatroot::version::DpkgVersionCompare;
  use serial_test::serial;
  use std::sync::Arc;
  use tempfile::TempDir;

  fn pkg(name: &str) -> Package {
    Package {
      name: name.into(),
      version: "1.0".into(),
      depends: vec![],
      recommends: vec![],
      suggests: vec![],
      install_if: vec![],
      provides: vec![],
      conflicts: vec![],
      breaks: vec![],
      essential: false,
      priority: None,
      description: String::new(),
      filename: format!("pool/{}.deb", name),
      size: 0,
      checksum: String::new(),
      rich_deps: vec![],
    }
  }

  fn pkg_provides(name: &str, provides: &[&str]) -> Package {
    let mut p = pkg(name);
    p.provides = provides
      .iter()
      .map(|n| DepSpec {
        name: (*n).to_string(),
        version_constraint: None,
      })
      .collect();
    p
  }

  /// Builds a populated package index, with optional file-ownership rows,
  /// under a fresh cache home. The bin crate cannot reach the lib's
  /// test-only `Index::on_disk`, so this mirrors `install.rs`'s
  /// `open_or_populate` idiom; it mutates the process environment, so
  /// every caller is `#[serial]`.
  fn index_with_facts(cache_key: &str, packages: Vec<Package>, path_facts: &[(&str, &str, &str)]) -> (TempDir, Index) {
    let tmp = TempDir::new().unwrap();
    // Safety: callers are #[serial]; no other test mutates the env concurrently.
    unsafe { std::env::set_var("FLATROOT_CACHE_HOME", tmp.path()) };
    let path_facts: Vec<(String, String, String)> = path_facts
      .iter()
      .map(|(d, f, p)| (d.to_string(), f.to_string(), p.to_string()))
      .collect();
    let index = Index::open_or_populate(
      cache_key,
      Arc::new(DpkgVersionCompare),
      move |writer: &mut IndexWriter| -> anyhow::Result<()> {
        for p in &packages {
          writer.insert(p)?;
        }
        for (d, f, p) in &path_facts {
          writer.path_push(d, f, p);
        }
        Ok(())
      },
    )
    .unwrap();
    (tmp, index)
  }

  /// An `AnalyzeArgs` with only the fields `targets_resolve` reads filled
  /// meaningfully; the rest carry placeholders it never reads.
  fn analyze_args<'a>(patterns: &'a [String], match_type: MatchType, combine: MatchCombine) -> AnalyzeArgs<'a> {
    AnalyzeArgs {
      remote_str: "debian:bookworm",
      arch: Arch::X86_64,
      patterns,
      match_type,
      combine,
      run_declared: true,
      run_linker: false,
      include_recommends: false,
      include_suggests: false,
    }
  }

  // covers: ANL-071
  #[test]
  #[serial]
  fn targets_resolve_path_intersect_picks_the_common_owner() {
    let (_tmp, index) = index_with_facts(
      "trc-path-intersect",
      vec![pkg("postfix"), pkg("exim4")],
      &[
        ("/usr/sbin", "sendmail", "postfix"),
        ("/usr/sbin", "sendmail", "exim4"),
        ("/etc/postfix", "main.cf", "postfix"),
      ],
    );
    let patterns = vec!["usr/sbin/sendmail".to_string(), "etc/postfix/main.cf".to_string()];
    let rows = analyze_args(&patterns, MatchType::Path, MatchCombine::All)
      .targets_resolve(&index, FormatPackage::Deb)
      .unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["postfix"]);
  }

  // covers: ANL-072
  #[test]
  #[serial]
  fn targets_resolve_path_intersect_disjoint_is_empty_not_error() {
    let (_tmp, index) = index_with_facts(
      "trc-path-disjoint",
      vec![pkg("foo"), pkg("bar")],
      &[("/usr/bin", "foo", "foo"), ("/usr/bin", "bar", "bar")],
    );
    let patterns = vec!["usr/bin/foo".to_string(), "usr/bin/bar".to_string()];
    let rows = analyze_args(&patterns, MatchType::Path, MatchCombine::All)
      .targets_resolve(&index, FormatPackage::Deb)
      .unwrap();
    assert!(rows.is_empty(), "an empty intersection is analyze's ordinary nothing answer, not an error");
  }

  // covers: ANL-073
  #[test]
  #[serial]
  fn targets_resolve_library_intersect_picks_common_owner() {
    let (_tmp, index) = index_with_facts(
      "trc-lib-intersect",
      vec![
        pkg_provides("mesa", &["libGL.so.1", "libEGL.so.1"]),
        pkg_provides("nvidia", &["libGL.so.1"]),
      ],
      &[],
    );
    let patterns = vec!["libGL.so.1".to_string(), "libEGL.so.1".to_string()];
    let rows = analyze_args(&patterns, MatchType::Library, MatchCombine::All)
      .targets_resolve(&index, FormatPackage::Deb)
      .unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["mesa"]);
  }

  // covers: ANL-065
  #[test]
  #[serial]
  fn targets_resolve_path_union_explicit() {
    let (_tmp, index) = index_with_facts(
      "trc-path-union",
      vec![pkg("foo"), pkg("bar")],
      &[("/usr/bin", "foo", "foo"), ("/usr/bin", "bar", "bar")],
    );
    let patterns = vec!["usr/bin/foo".to_string(), "usr/bin/bar".to_string()];
    let rows = analyze_args(&patterns, MatchType::Path, MatchCombine::Any)
      .targets_resolve(&index, FormatPackage::Deb)
      .unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["foo", "bar"]);
  }

  // covers: ANL-066
  #[test]
  #[serial]
  fn targets_resolve_intersect_empty_pattern_is_silent_empty() {
    // Contrast with install (INST-067): analyze never errors on a pattern that
    // matches nothing; the empty group makes the intersection empty, and the
    // empty seed set is the ordinary empty outcome.
    let (_tmp, index) = index_with_facts("trc-path-emptypat", vec![pkg("bash")], &[("/usr/bin", "bash", "bash")]);
    let patterns = vec!["usr/bin/bash".to_string(), "zzz/none".to_string()];
    let rows = analyze_args(&patterns, MatchType::Path, MatchCombine::All)
      .targets_resolve(&index, FormatPackage::Deb)
      .unwrap();
    assert!(rows.is_empty());
  }

  // covers: ANL-067
  #[test]
  #[serial]
  fn targets_resolve_intersect_single_pattern_identity() {
    let (_tmp, index) = index_with_facts("trc-path-single", vec![pkg("bash")], &[("/usr/bin", "bash", "bash")]);
    let patterns = vec!["usr/bin/bash".to_string()];
    let rows = analyze_args(&patterns, MatchType::Path, MatchCombine::All)
      .targets_resolve(&index, FormatPackage::Deb)
      .unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["bash"]);
  }
}
