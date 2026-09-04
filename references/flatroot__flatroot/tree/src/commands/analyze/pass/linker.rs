//! The linker pass: downloads each package, reads which sonames its ELF
//! binaries need, and resolves each soname to the package that provides
//! it — walking newly found providers until no new package appears.

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tempfile::TempDir;

use flatroot::downloader::Downloader;
use flatroot::elf;
use flatroot::package::PackageIdentity;
use flatroot::remote::Soname;
use flatroot::ui::RunVoice;

use super::super::ctx::AnalysisContext;
use super::super::outcome::{ConsumedSoname, UnresolvedSoname};

/// One package that provides sonames binaries in the walk need.
pub(crate) struct LinkerProvider {
  /// Version recorded when the walk first found this package as a
  /// provider.
  pub version: String,
  /// Sonames of this package that some binary in the walk needs,
  /// deduped: several binaries can need the same soname.
  pub sonames: BTreeSet<String>,
}

/// Everything the linker walk found, grouped by package name.
pub(crate) struct LinkerWalkOutcome {
  /// Packages that provide needed sonames, keyed by package name.
  pub providers: BTreeMap<String, LinkerProvider>,
  /// Resolved sonames, keyed by the package whose binaries need them.
  pub consumed: BTreeMap<String, Vec<ConsumedSoname>>,
  /// Sonames matched to no providing package, keyed by the package
  /// whose binaries need them.
  pub unresolved: BTreeMap<String, Vec<UnresolvedSoname>>,
}

/// One linker walk in progress: packages are visited breadth-first,
/// starting from the seed packages.
struct LinkerWalk<'a> {
  ctx: &'a AnalysisContext<'a>,
  downloader: Downloader<'a>,
  resolver: Soname<'a>,
  pb: indicatif::ProgressBar,
  path_dir_work: PathBuf,
  visited: HashSet<String>,
  frontier: VecDeque<String>,
  outcome: LinkerWalkOutcome,
}

impl LinkerWalk<'_> {
  /// Downloads, extracts, and reads one package: records every soname
  /// its ELF binaries need and adds newly found providers to `frontier`.
  async fn package_walk(&mut self, pkg_name: &str) -> Result<()> {
    self.pb.set_message(format!("Walking {}", pkg_name));
    let extract_root = self.package_materialize(pkg_name).await?;

    for elf_path in elf::ElfScan::scan(&extract_root)? {
      let Some(deps) = elf::ElfDeps::read(&elf_path)? else {
        continue;
      };
      let rel_binary: PathBuf = elf_path
        .strip_prefix(&extract_root)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| elf_path.clone());
      for soname in &deps.needed {
        self.soname_record(pkg_name, &rel_binary, soname, &deps)?;
      }
    }
    Ok(())
  }

  /// Downloads and extracts one package, returning the directory it was
  /// extracted into.
  async fn package_materialize(&mut self, pkg_name: &str) -> Result<PathBuf> {
    let downloaded = self.downloader.fetch(&[pkg_name.to_string()], &self.pb).await?;
    let archive = downloaded
      .get(pkg_name)
      .ok_or_else(|| anyhow::anyhow!("Downloader::fetch did not return a path for '{}'", pkg_name))?;
    let extract_root = self.path_dir_work.join(pkg_name);
    std::fs::create_dir_all(&extract_root)?;
    let identity = PackageIdentity {
      name: pkg_name.to_string(),
      arch: self.ctx.arch.to_string(),
    };
    self.ctx.remote.extract(archive, &extract_root, &identity)?;
    Ok(extract_root)
  }

  /// Records one `DT_NEEDED` soname. When a providing package is found,
  /// the soname lands in `consumed` and on the provider's entry, and a
  /// provider seen for the first time joins `frontier`; otherwise the
  /// soname lands in `unresolved`.
  fn soname_record(&mut self, consumer: &str, rel_binary: &Path, soname: &str, deps: &elf::ElfDeps) -> Result<()> {
    let provider = self.resolver.resolve(soname, deps)?;
    let Some(provider_pkg) = provider else {
      self
        .outcome
        .unresolved
        .entry(consumer.to_string())
        .or_default()
        .push(UnresolvedSoname {
          binary: rel_binary.to_path_buf(),
          soname: soname.to_string(),
        });
      return Ok(());
    };

    // Keyed by the consumer's name so the merge can attach the list to
    // the consumer's `TraceEntry` by direct lookup.
    self
      .outcome
      .consumed
      .entry(consumer.to_string())
      .or_default()
      .push(ConsumedSoname {
        binary: rel_binary.to_path_buf(),
        provider: provider_pkg.clone(),
        soname: soname.to_string(),
      });

    self.provider_credit(&provider_pkg, soname)?;
    if self.visited.insert(provider_pkg.clone()) {
      self.frontier.push_back(provider_pkg);
    }
    Ok(())
  }

  /// Adds one soname to its provider's `LinkerProvider` entry. The
  /// provider's package row is looked up only the first time the
  /// provider appears, so the DB query is not repeated per soname.
  fn provider_credit(&mut self, provider_pkg: &str, soname: &str) -> Result<()> {
    let entry = match self.outcome.providers.entry(provider_pkg.to_string()) {
      std::collections::btree_map::Entry::Occupied(occupied) => occupied.into_mut(),
      std::collections::btree_map::Entry::Vacant(vacant) => {
        let row = self
          .ctx
          .index
          .packages()
          .get(provider_pkg)?
          .ok_or_else(|| anyhow::anyhow!("provider '{}' missing from index after lookup", provider_pkg))?;
        vacant.insert(LinkerProvider {
          version: row.version,
          sonames: BTreeSet::new(),
        })
      }
    };
    entry.sonames.insert(soname.to_string());
    Ok(())
  }
}

/// The pass that finds which packages the seeds' ELF binaries actually
/// load libraries from at run time, including packages the metadata
/// never declares.
pub struct LinkerPass;

impl LinkerPass {
  /// Walks the run-time library closure of the seed packages: each
  /// package is downloaded and its ELF binaries read, and every provider
  /// of a needed soname is walked in turn, until no new package appears.
  /// Sonames with no providing package are recorded in `unresolved`.
  pub async fn run(ctx: &AnalysisContext<'_>) -> Result<LinkerWalkOutcome> {
    let workdir = TempDir::new().context("failed to create analyze tempdir")?;

    // Download concurrency 1: packages are fetched one at a time, since
    // the next frontier is known only after each package's binaries are
    // read.
    let mut walk = LinkerWalk {
      ctx,
      downloader: Downloader::new(ctx.index, ctx.remote, ctx.path_dir_cache, ctx.http_retries, 1)?,
      resolver: ctx.remote.soname_resolver(ctx.index),
      pb: RunVoice::counter("Linker walk"),
      path_dir_work: workdir.path().to_path_buf(),
      visited: ctx.targets.iter().map(|t| t.name.clone()).collect(),
      frontier: ctx.targets.iter().map(|t| t.name.clone()).collect(),
      outcome: LinkerWalkOutcome {
        providers: BTreeMap::new(),
        consumed: BTreeMap::new(),
        unresolved: BTreeMap::new(),
      },
    };

    // Start the counter at the seed count — `visited` already holds
    // every seed, and the display would otherwise read 0 while the
    // first download runs.
    walk.pb.set_position(walk.visited.len() as u64);

    while let Some(pkg_name) = walk.frontier.pop_front() {
      walk.package_walk(&pkg_name).await?;
      // Set the counter to the number of packages discovered so far;
      // this also overrides the `inc(1)` the fetch performed, so the
      // display counts discovered packages, not downloads.
      walk.pb.set_position(walk.visited.len() as u64);
    }

    RunVoice::finish_ok(
      &walk.pb,
      format!(
        "Linker walk: {} packages, {} sonames provided",
        walk.outcome.providers.len(),
        walk.outcome.providers.values().map(|p| p.sonames.len()).sum::<usize>()
      ),
    );

    Ok(walk.outcome)
  }
}
