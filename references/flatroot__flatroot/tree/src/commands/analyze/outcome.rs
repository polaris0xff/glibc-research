//! The result types of a finished trace: one `TraceEntry` per package
//! the analysis reached, each carrying why it was reached and its
//! library links, collected under one `AnalysisOutcome`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use flatroot::dep_tree::DepTree;

use super::pass::linker::LinkerWalkOutcome;

/// Why a package is in the trace: which pass reached it, or both, or the
/// user asked about it directly. A package only the linker pass reached
/// is a dependency the metadata fails to declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisReason {
  /// A seed package the user asked about. Assigned last, so it
  /// overrides whatever reason a pass assigned.
  Target,
  /// Reached only by the declared pass: the metadata depends on
  /// it, but no binary in the trace links against a library it
  /// provides.
  Declared,
  /// Reached only by the linker pass: a binary in the trace
  /// links against a library this package provides, but the
  /// metadata never declares the dependency.
  Linker,
  /// Reached by both passes: the metadata declares it and a
  /// binary links against it — the normal case for library
  /// dependencies.
  DeclaredLinker,
}

/// One seed package the user's patterns matched, with the version the
/// package index recorded when the seed was resolved.
#[derive(Debug, Clone)]
pub struct TraceSeed {
  /// Package name the user asked about (one per deduped glob hit).
  pub name: String,
  /// Version the package index recorded when the seed was resolved.
  pub version: String,
}

/// Everything the analysis found about one package.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceEntry {
  /// Packages this one declares it depends on; empty when only the
  /// linker pass reached it.
  pub depends_on: Vec<String>,
  /// Package name as the package index records it.
  pub name: String,
  /// Why this package is in the trace.
  pub reason: AnalysisReason,
  /// Sonames this package's binaries need, each with the package that
  /// provides it.
  pub sonames_consumed: Vec<ConsumedSoname>,
  /// Sonames this package provides that some binary in the trace needs.
  pub sonames_provided: Vec<String>,
  /// Sonames this package's binaries need that no package could be
  /// found to provide.
  pub unresolved_sonames: Vec<UnresolvedSoname>,
  /// Version the package index records for this package.
  pub version: String,
}

impl TraceEntry {
  /// An entry with only name, reason, and version set; every list
  /// starts empty and is filled during the merge.
  fn new(name: String, reason: AnalysisReason, version: String) -> TraceEntry {
    TraceEntry {
      depends_on: Vec::new(),
      name,
      reason,
      sonames_consumed: Vec::new(),
      sonames_provided: Vec::new(),
      unresolved_sonames: Vec::new(),
      version,
    }
  }
}

/// One resolved library link: a binary, the soname it needs, and the
/// package that provides it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConsumedSoname {
  /// Path to the binary, relative to the package root.
  pub binary: PathBuf,
  /// Package that provides the soname.
  pub provider: String,
  /// Soname the binary needs.
  pub soname: String,
}

/// One library link no package could be found to provide — a library
/// that would be missing at run time.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UnresolvedSoname {
  /// Path to the binary, relative to the package root.
  pub binary: PathBuf,
  /// Soname the binary needs, matched to no providing package.
  pub soname: String,
}

/// The finished result of one `analyze trace` run.
#[derive(Debug, Clone)]
pub struct AnalysisOutcome {
  /// The `--from` source the trace ran against.
  pub source: String,
  /// Target architecture the trace ran against, as a uname string.
  pub arch: String,
  /// The seed packages (one per deduped glob hit).
  pub seeds: Vec<TraceSeed>,
  /// Every package the analysis reached, alphabetical by name.
  pub entries: Vec<TraceEntry>,
}

impl AnalysisOutcome {
  /// Merges the two pass results into one entry per package: a package
  /// both passes reached is `DeclaredLinker`, one only the linker pass
  /// reached is `Linker`, and every seed becomes `Target` regardless of
  /// how it was reached.
  pub fn merge(
    source: String,
    arch: String,
    seeds: Vec<TraceSeed>,
    dep_tree: Option<DepTree>,
    linker: Option<LinkerWalkOutcome>,
  ) -> Self {
    let mut by_name: BTreeMap<String, TraceEntry> = BTreeMap::new();

    if let Some(tree) = dep_tree {
      for (name, node) in tree.nodes {
        let mut entry = TraceEntry::new(node.name, AnalysisReason::Declared, node.version);
        entry.depends_on = node.edges.into_iter().map(|e| e.child).collect();
        by_name.insert(name, entry);
      }
    }

    if let Some(linker) = linker {
      Self::linker_fold(&mut by_name, linker);
    }

    Self::seeds_stamp(&mut by_name, &seeds);

    AnalysisOutcome {
      source,
      arch,
      seeds,
      entries: by_name.into_values().collect(),
    }
  }

  /// Folds the linker pass result in. A provider the declared pass
  /// already added becomes `DeclaredLinker`; a provider new to the map
  /// enters as `Linker`. A package can appear in `consumed` or
  /// `unresolved` without providing anything itself — a seed executable
  /// is the typical case — so those enter as `Linker` entries too.
  fn linker_fold(by_name: &mut BTreeMap<String, TraceEntry>, linker: LinkerWalkOutcome) {
    for (pkg, provider) in linker.providers {
      let sonames_vec: Vec<String> = provider.sonames.into_iter().collect();
      match by_name.get_mut(&pkg) {
        Some(entry) => {
          entry.reason = AnalysisReason::DeclaredLinker;
          entry.sonames_provided = sonames_vec;
        }
        None => {
          let mut entry = TraceEntry::new(pkg.clone(), AnalysisReason::Linker, provider.version);
          entry.sonames_provided = sonames_vec;
          by_name.insert(pkg, entry);
        }
      }
    }
    for (consumer, edges) in linker.consumed {
      Self::consumer_entry(by_name, consumer).sonames_consumed = edges;
    }
    for (consumer, list) in linker.unresolved {
      Self::consumer_entry(by_name, consumer).unresolved_sonames = list;
    }
  }

  /// The entry for a package named in `consumed` or `unresolved`,
  /// created as a `Linker` entry with an empty version when neither pass
  /// added it.
  fn consumer_entry(by_name: &mut BTreeMap<String, TraceEntry>, consumer: String) -> &mut TraceEntry {
    by_name
      .entry(consumer.clone())
      .or_insert_with(|| TraceEntry::new(consumer, AnalysisReason::Linker, String::new()))
  }

  /// Sets every seed's reason to `Target`, overriding whatever reason
  /// the passes assigned. A seed absent from both pass results still
  /// gets an entry. A seed that entered only through `consumed` carries
  /// an empty version; the seed's resolved version fills it.
  fn seeds_stamp(by_name: &mut BTreeMap<String, TraceEntry>, seeds: &[TraceSeed]) {
    for seed in seeds {
      match by_name.get_mut(&seed.name) {
        Some(entry) => {
          entry.reason = AnalysisReason::Target;
          if entry.version.is_empty() {
            entry.version = seed.version.clone();
          }
        }
        None => {
          by_name.insert(
            seed.name.clone(),
            TraceEntry::new(seed.name.clone(), AnalysisReason::Target, seed.version.clone()),
          );
        }
      }
    }
  }

  /// The outcome for patterns that matched no package: no seeds and no
  /// entries, with the source and architecture still recorded.
  pub fn empty(source: String, arch: String) -> Self {
    AnalysisOutcome {
      source,
      arch,
      seeds: Vec::new(),
      entries: Vec::new(),
    }
  }

  /// Total number of unresolved sonames across all entries.
  pub fn unresolved_count(&self) -> usize {
    self.entries.iter().map(|e| e.unresolved_sonames.len()).sum()
  }
}
