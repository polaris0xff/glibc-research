//! The `install` command: turns a distribution's upstream packages into
//! a ready-to-use root filesystem, without root privileges and without
//! the host's package manager.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Result, bail};

use flatroot::arch::Arch;
use flatroot::db::Index;
use flatroot::distro::FormatPackage;
use flatroot::downloader::Downloader;
use flatroot::library::LibraryMatch;
use flatroot::manifest::{Checksum, ManifestState, PackageRecord};
use flatroot::package::{DepKind, PackageIdentity};
use flatroot::path::PathMatch;
use flatroot::postfixes::Postfix;
use flatroot::postinstall::PostInstallPlan;
use flatroot::remote::RemoteDistro;
use flatroot::resolver;
use flatroot::ui::RunVoice;

use crate::commands::arch_context::ArchContext;
use crate::commands::session::Session;
use crate::parser::{MatchCombine, MatchType};

/// One parsed install request; every field comes straight from the
/// command line.
pub struct InstallArgs {
  /// The `--from` source naming distribution and release (e.g.
  /// `debian:bookworm`, `arch:rolling@2026-04-01`).
  pub remote_str: String,
  /// Target directory for the rootfs. Created if missing; an existing
  /// tree is reused, its prior install record carrying the state forward.
  pub root: PathBuf,
  /// The requested architectures; the pipeline runs once per entry into
  /// the same destination.
  pub archs: Vec<Arch>,
  /// The user's patterns, resolved through `match_type` into package names
  /// that seed resolution (joined with base and essential packages unless
  /// `no_deps`).
  pub patterns: Vec<String>,
  /// Maximum number of archive downloads to run in parallel.
  pub jobs: usize,
  /// Include recommended packages in the closure where the distribution
  /// publishes them (Debian / Ubuntu).
  pub recommends: bool,
  /// Include suggested packages in the closure where the distribution
  /// publishes them (Debian / Ubuntu).
  pub suggests: bool,
  /// Post-install phases to run; empty means every phase the distribution
  /// exposes.
  pub postinstall: Vec<flatroot::postinstall::Phase>,
  /// Skip resolution and install only the requested packages, after
  /// verifying each exists.
  pub no_deps: bool,
  /// Package names to drop from the closure.
  pub exclude: Vec<String>,
  /// Whether patterns match package names, library files, or installed
  /// paths.
  pub match_type: MatchType,
  /// How patterns combine when each resolves to a set of owners: `Any`
  /// unions, `All` intersects. No effect under `--type package`, where
  /// the executor rejects `All`.
  pub combine: MatchCombine,
}

/// Runs the whole install: one pass per requested architecture into the
/// shared `root`, the manifest written once after every pass succeeds,
/// and post-install run only when something changed on disk. An
/// incompatible prior install in `root` is refused before any work
/// begins.
pub async fn run(session: &Session, args: InstallArgs) -> Result<()> {
  std::fs::create_dir_all(&args.root)?;

  // Read the existing manifest (if any) BEFORE any network or disk work.
  // A malformed manifest surfaces here as a hard error; cross-distro
  // incompatibility is rejected immediately after.
  let existing = ManifestState::read(&args.root)?;
  ManifestState::source_admit(existing.as_ref(), &args.remote_str, &args.root)?;

  // Run the full pipeline for each architecture.
  // For `--arch x86_64,i686`, this runs twice — once per architecture.
  // Both extract into the same root. Libraries go into arch-specific paths
  // (e.g., /usr/lib/x86_64-linux-gnu/ vs /usr/lib/i386-linux-gnu/).
  //
  // The existing manifest lets each per-arch pass short-circuit packages
  // whose (name, arch, version, checksum) already match — we track the
  // total extraction count across archs so post-install can be skipped
  // entirely when nothing changed on disk.
  let mut all_records: Vec<PackageRecord> = Vec::new();
  let mut total_extracted: usize = 0;
  for arch in &args.archs {
    if args.archs.len() > 1 {
      eprintln!("=== {} ===", arch.as_uname());
    }

    let ctx = session.context_open(&args.remote_str, *arch).await?;
    let (records, extracted) = install_for_arch(session, &args, &ctx, existing.as_ref()).await?;
    all_records.extend(records);
    total_extracted += extracted;
  }

  // Write the manifest after all extractions succeed. On any crash
  // before this point the rootfs keeps its prior manifest, so the
  // manifest never describes a partial extraction.
  let state = ManifestState::merge(existing, all_records, env!("CARGO_PKG_VERSION"));
  state.write(&args.root)?;
  let total_installed = state.packages.len();

  // Post-install runs once after all architectures are extracted — but only
  // if at least one package was actually (re)extracted. When every resolved
  // package was already current, the rootfs is unchanged, so ldconfig /
  // font cache / MIME regeneration would all be pure waste.
  if total_extracted > 0 {
    let primary_arch = args.archs.first().copied().unwrap_or(Arch::X86_64);
    let remote = RemoteDistro::from_str(&args.remote_str, primary_arch, session.http().clone())?;
    let pi = remote.post_install();
    PostInstallPlan::new(&args.root, &args.postinstall, primary_arch.as_uname()).run(&pi)?;
    Postfix::apply(&args.root)?;
    eprintln!("Done. {} packages installed to {}", total_installed, args.root.display());
  } else {
    eprintln!("Nothing to do. All {} packages already current in {}", total_installed, args.root.display());
  }
  Ok(())
}

/// Installs one architecture into the shared tree: resolves the full
/// package set (or verifies the named ones under `--no-deps`), fetches
/// and extracts only the packages that are not already current, in
/// dependency order, and returns the records plus the count of packages
/// extracted.
async fn install_for_arch(
  session: &Session,
  args: &InstallArgs,
  ctx: &ArchContext,
  existing: Option<&ManifestState>,
) -> Result<(Vec<PackageRecord>, usize)> {
  let requested = requested_resolve(&ctx.index, ctx.remote.format(), args.match_type, args.combine, &args.patterns)?;
  let resolved = resolved_set(args, ctx, &requested)?;

  let (mut records, rows_extract) = records_partition(ctx, existing, &resolved)?;
  if rows_extract.is_empty() {
    eprintln!("All {} resolved packages already current for {}.", resolved.len(), ctx.arch.as_uname());
    return Ok((records, 0));
  }

  let downloaded = archives_download(session, args, ctx, &rows_extract, records.len()).await?;
  let extracted_count = archives_extract(args, ctx, rows_extract, &downloaded, &mut records)?;
  Ok((records, extracted_count))
}

/// Resolves the user's patterns to package names under `match_type` —
/// package names directly, or the owners of matched libraries and paths
/// — with the same matcher `search` uses. A pattern matching nothing is
/// an error naming that pattern; `combine` then unions or intersects
/// the per-pattern owner sets, and an empty intersection is an error
/// naming every pattern. `--type path` is refused for apk, which
/// publishes no file lists.
fn requested_resolve(
  index: &Index,
  format: FormatPackage,
  match_type: MatchType,
  combine: MatchCombine,
  patterns: &[String],
) -> Result<Vec<String>> {
  if matches!(match_type, MatchType::Path) && matches!(format, FormatPackage::Apk) {
    bail!(
      "--type path is unsupported for apk sources: Alpine publishes no file lists (APKINDEX carries no file-to-package data)"
    );
  }

  let mut groups: Vec<Vec<String>> = Vec::new();
  for pattern in patterns {
    let matched: Vec<String> = match match_type {
      MatchType::Package => index.packages().glob(pattern)?.into_iter().map(|r| r.name).collect(),
      MatchType::Library => LibraryMatch::glob(index, pattern)?
        .into_iter()
        .map(|m| m.package)
        .collect(),
      MatchType::Path => PathMatch::glob(index, pattern)?
        .into_iter()
        .map(|m| m.package)
        .collect(),
    };
    // A package owning several matched libraries or paths appears once per
    // hit; the request cares about the owner, so collapse to unique names
    // preserving first-seen order before the patterns are combined.
    let mut names: Vec<String> = Vec::new();
    let mut seen_pattern: HashSet<&str> = HashSet::new();
    for name in &matched {
      if seen_pattern.insert(name.as_str()) {
        names.push(name.clone());
      }
    }
    if names.is_empty() {
      match match_type {
        MatchType::Package => bail!("Package '{}' not found in the index", pattern),
        MatchType::Library => bail!("Library pattern '{}' matched no package in the index", pattern),
        MatchType::Path => bail!("Path pattern '{}' matched no package in the index", pattern),
      }
    }
    if !(names.len() == 1 && names[0] == *pattern) {
      eprintln!("Resolved '{}' -> {}", pattern, names.join(" "));
    }
    groups.push(names);
  }

  let requested = combine.owners_fold(&groups);
  if requested.is_empty() {
    // Reachable only under `--match all`: every pattern matched at least one
    // package (an empty group already failed above), yet their owner sets
    // share none in common, so the intersection the user asked to install is
    // empty. That is a request for a package that does not exist, not a
    // smaller install, so it fails rather than installing nothing.
    bail!(
      "No package owns a match for every pattern under --match all: {}. Each pattern matched at least one package, but no single package matched them all.",
      patterns.join(" ")
    );
  }
  Ok(requested)
}

/// The package set to install for one architecture: the verified
/// requested list under `--no-deps`, or the full dependency closure of
/// the base, essential, and requested packages.
fn resolved_set(args: &InstallArgs, ctx: &ArchContext, requested: &[String]) -> Result<Vec<String>> {
  if args.no_deps {
    return packages_verify(requested, ctx);
  }

  let seeds = seed_set(ctx, requested)?;
  let pb = RunVoice::spinner("Resolving dependencies");
  let cfg = resolver::ResolutionEnv {
    index: &ctx.index,
    include_recommends: args.recommends,
    include_suggests: args.suggests,
    exclude: &args.exclude,
  };
  let resolved = resolver::DepWalker::resolve(&cfg, &seeds)?;
  RunVoice::finish_ok(&pb, format!("Resolved {} packages", resolved.len()));
  Ok(resolved)
}

/// `--no-deps`: skips the resolver and the base and essential packages;
/// verifies each requested package exists in the index — refusing
/// unknown names before any download — and returns the list unchanged.
fn packages_verify(requested: &[String], ctx: &ArchContext) -> Result<Vec<String>> {
  let mut direct = Vec::new();
  for p in requested {
    if !ctx.index.packages().exists(p.as_str())? {
      bail!("Package '{}' not found in the index", p);
    }
    direct.push(p.clone());
  }
  eprintln!("Installing {} packages (--no-deps)", direct.len());
  Ok(direct)
}

/// The resolver's seeds in priority order — distro base packages that exist
/// in this index (Arch: filesystem, glibc, …), the distribution's
/// `Essential: yes` set (Debian/Ubuntu only), then the user's requests,
/// already resolved from their patterns — each added once.
fn seed_set(ctx: &ArchContext, requested: &[String]) -> Result<Vec<String>> {
  let mut seeds: Vec<String> = Vec::new();
  for base in ctx.remote.base_packages() {
    if ctx.index.packages().exists(base)? && !seeds.contains(&base.to_string()) {
      seeds.push(base.to_string());
    }
  }
  for name in ctx.index.packages().essential()? {
    if !seeds.contains(&name) {
      seeds.push(name);
    }
  }
  for p in requested {
    if !seeds.contains(p) {
      seeds.push(p.clone());
    }
  }
  Ok(seeds)
}

/// Partitions the resolved set into records carried forward verbatim
/// and rows that need fetching. A package is already current when the
/// manifest holds the same (name, arch) at matching version and
/// checksum — a matching checksum means a bit-identical archive, so
/// re-extraction would write the exact same bytes.
fn records_partition(
  ctx: &ArchContext,
  existing: Option<&ManifestState>,
  resolved: &[String],
) -> Result<(Vec<PackageRecord>, Vec<flatroot::db::PackageRow>)> {
  let arch_uname = ctx.arch.as_uname();
  let checksum_algo = ctx.remote.checksum_type();
  let mut records: Vec<PackageRecord> = Vec::with_capacity(resolved.len());
  let mut rows_extract: Vec<flatroot::db::PackageRow> = Vec::new();

  for name in resolved {
    let pkg = pkg_row_required(&ctx.index, name)?;
    let key = PackageIdentity {
      name: pkg.name.clone(),
      arch: arch_uname.to_string(),
    };
    let Some(prior) = existing.and_then(|e| e.packages.get(&key)) else {
      rows_extract.push(pkg);
      continue;
    };
    if prior.version != pkg.version || prior.checksum.hex != pkg.checksum {
      rows_extract.push(pkg);
      continue;
    }

    // Reuse the prior record's source, url, and files verbatim.
    // `ManifestState::source_admit` already guarantees the new install's
    // source equals the prior one; copying the prior fields keeps the
    // manifest correct even if that check ever changes.
    let depends = ctx.index.dependencies().of(name, DepKind::Depends)?;
    records.push(PackageRecord {
      name: pkg.name.clone(),
      version: pkg.version,
      architecture: arch_uname.to_string(),
      source: prior.source.clone(),
      url: prior.url.clone(),
      checksum: Checksum {
        algorithm: checksum_algo,
        hex: pkg.checksum,
      },
      size: pkg.size,
      depends,
      files: prior.files.clone(),
    });
  }
  Ok((records, rows_extract))
}

/// Downloads the archives for the rows that need extraction. The
/// `Downloader` caches archives on disk, so already-fetched archives
/// cost nothing either way.
async fn archives_download(
  session: &Session,
  args: &InstallArgs,
  ctx: &ArchContext,
  rows_extract: &[flatroot::db::PackageRow],
  count_current: usize,
) -> Result<std::collections::HashMap<String, PathBuf>> {
  let names: Vec<String> = rows_extract.iter().map(|row| row.name.clone()).collect();
  let pb = RunVoice::bar(names.len() as u64, "preparing");
  let downloader =
    Downloader::new(&ctx.index, ctx.remote.as_ref(), &ctx.cache_dir, session.http().retries(), args.jobs)?;
  let downloaded = downloader.fetch(&names, &pb).await?;
  RunVoice::finish_ok(&pb, format!("Downloaded {} packages ({} already current)", names.len(), count_current));
  Ok(downloaded)
}

/// Unpacks the archives in resolver order — dependency-first. Packages
/// like `filesystem` (Arch) or `usrmerge` (Debian) create directory
/// symlinks (/bin → usr/bin); a later package extracting into /bin/
/// before the symlink exists would create a real directory instead,
/// breaking the merged-usr layout.
fn archives_extract(
  args: &InstallArgs,
  ctx: &ArchContext,
  rows_extract: Vec<flatroot::db::PackageRow>,
  downloaded: &std::collections::HashMap<String, PathBuf>,
  records: &mut Vec<PackageRecord>,
) -> Result<usize> {
  let arch_uname = ctx.arch.as_uname();
  let checksum_algo = ctx.remote.checksum_type();
  let pb = RunVoice::bar(rows_extract.len() as u64, "preparing");
  let mut extracted_count = 0;

  for pkg in rows_extract {
    let Some(path_file_archive) = downloaded_archive(downloaded, &pkg.name) else {
      continue;
    };
    pb.set_message(format!("extracting {}", pkg.name));
    let identity = PackageIdentity {
      name: pkg.name.clone(),
      arch: arch_uname.to_string(),
    };
    let files = ctx.remote.extract(path_file_archive, &args.root, &identity)?;
    extracted_count += 1;
    pb.inc(1);

    let depends = ctx.index.dependencies().of(&pkg.name, DepKind::Depends)?;
    records.push(PackageRecord {
      name: pkg.name.clone(),
      version: pkg.version,
      architecture: arch_uname.to_string(),
      source: args.remote_str.clone(),
      url: ctx.remote.download_url(&pkg.filename)?,
      checksum: Checksum {
        algorithm: checksum_algo,
        hex: pkg.checksum,
      },
      size: pkg.size,
      depends,
      files,
    });
  }
  RunVoice::finish_ok(&pb, format!("Extracted {} packages to {}", extracted_count, args.root.display()));
  Ok(extracted_count)
}

/// The package row for a resolved name; a missing row is a hard error,
/// never a silently dropped package. The resolver only yields names the
/// index holds, so a miss means the resolved set and the index disagree
/// and the build must stop.
fn pkg_row_required(index: &flatroot::db::Index, name: &str) -> Result<flatroot::db::PackageRow> {
  index
    .packages()
    .get(name)?
    .ok_or_else(|| anyhow::anyhow!("Resolved package '{}' not found in database", name))
}

/// The downloaded archive for a package that needs extracting. `None` means the
/// downloader returned no path for this name, in which case the extraction loop
/// skips the package rather than treating it as fatal. The downloader yields a
/// path for every requested name or fails outright, so this is a defensive
/// guard.
fn downloaded_archive<'m>(
  downloaded: &'m std::collections::HashMap<String, PathBuf>,
  name: &str,
) -> Option<&'m PathBuf> {
  downloaded.get(name)
}

#[cfg(test)]
mod tests {
  use super::{downloaded_archive, pkg_row_required};
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::Arc;

  use flatroot::db::{Index, IndexWriter};
  use flatroot::package::Package;
  use flatroot::version::DpkgVersionCompare;
  use serial_test::serial;
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

  /// Builds a populated package index under a fresh cache home holding
  /// only the named packages, entirely offline. Mirrors the
  /// `open_or_populate` idiom the library tests use; it mutates the
  /// process environment, so every caller is `#[serial]`.
  fn index_with(names: &[&str]) -> (TempDir, Index) {
    let tmp = TempDir::new().unwrap();
    // Safety: callers are #[serial]; no other test mutates the env concurrently.
    unsafe { std::env::set_var("FLATROOT_CACHE_HOME", tmp.path()) };
    let owned: Vec<String> = names.iter().map(|s| s.to_string()).collect();
    let index = Index::open_or_populate(
      "install-guard-test",
      Arc::new(DpkgVersionCompare),
      move |writer: &mut IndexWriter| -> anyhow::Result<()> {
        for n in &owned {
          writer.insert(&pkg(n))?;
        }
        Ok(())
      },
    )
    .unwrap();
    (tmp, index)
  }

  // covers: INST-044
  #[test]
  #[serial]
  fn pkg_row_required_bails_when_resolved_name_is_absent_from_db() {
    let (_tmp, index) = index_with(&["realpkg"]);
    // A name the index holds resolves to its row.
    assert!(pkg_row_required(&index, "realpkg").is_ok());
    // A name not in the index is the hard error the post-resolution
    // loop raises rather than continuing with a hole in the install set.
    let err = pkg_row_required(&index, "ghostpkg")
      .err()
      .expect("absent resolved package must error");
    assert_eq!(err.to_string(), "Resolved package 'ghostpkg' not found in database");
  }

  // covers: INST-060
  #[test]
  fn downloaded_archive_is_none_for_an_unmapped_package() {
    let mut downloaded: HashMap<String, PathBuf> = HashMap::new();
    downloaded.insert("present".to_string(), PathBuf::from("/cache/present.deb"));
    // A package the downloader returned a path for is found and extracted...
    assert_eq!(downloaded_archive(&downloaded, "present"), Some(&PathBuf::from("/cache/present.deb")));
    // ...one with no entry in the download map yields None, which the extraction
    // loop turns into a skip (`continue`) instead of a failure.
    assert!(downloaded_archive(&downloaded, "missing").is_none());
  }

  // ── requested_resolve ──────────────────────────────────────────────────

  use super::requested_resolve;
  use crate::parser::{MatchCombine, MatchType};
  use flatroot::distro::FormatPackage;
  use flatroot::package::DepSpec;

  /// Builds a populated package index with optional provides and
  /// file-ownership rows under a fresh cache home. Callers are
  /// `#[serial]` (env mutation).
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

  fn patterns(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
  }

  #[test]
  #[serial]
  fn requested_resolve_package_exact_name_is_identity() {
    let (_tmp, index) = index_with_facts("req-pkg-exact", vec![pkg("bash"), pkg("bash-doc")], &[]);
    let names =
      requested_resolve(&index, FormatPackage::Deb, MatchType::Package, MatchCombine::Any, &patterns(&["bash"]))
        .unwrap();
    assert_eq!(names, vec!["bash"], "a wildcard-free pattern matches exactly, never partially");
  }

  #[test]
  #[serial]
  fn requested_resolve_package_wildcard_expands_to_every_match() {
    let (_tmp, index) = index_with_facts("req-pkg-glob", vec![pkg("bash"), pkg("bash-doc"), pkg("zsh")], &[]);
    let names =
      requested_resolve(&index, FormatPackage::Deb, MatchType::Package, MatchCombine::Any, &patterns(&["bash*"]))
        .unwrap();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"bash".to_string()) && names.contains(&"bash-doc".to_string()));
  }

  #[test]
  #[serial]
  fn requested_resolve_package_unknown_keeps_established_wording() {
    let (_tmp, index) = index_with_facts("req-pkg-unknown", vec![pkg("bash")], &[]);
    let err =
      requested_resolve(&index, FormatPackage::Deb, MatchType::Package, MatchCombine::Any, &patterns(&["ghost"]))
        .unwrap_err();
    assert!(err.to_string().contains("not found in the index"), "got: {err}");
  }

  #[test]
  #[serial]
  fn requested_resolve_library_pattern_resolves_to_owner() {
    let (_tmp, index) =
      index_with_facts("req-lib-owner", vec![pkg_provides("libssl3", &["libssl.so.3"]), pkg("bash")], &[]);
    let names =
      requested_resolve(&index, FormatPackage::Deb, MatchType::Library, MatchCombine::Any, &patterns(&["libssl.so.3"]))
        .unwrap();
    assert_eq!(names, vec!["libssl3"]);
  }

  #[test]
  #[serial]
  fn requested_resolve_path_pattern_resolves_to_owner() {
    let (_tmp, index) = index_with_facts("req-path-owner", vec![pkg("bash")], &[("/usr/bin", "bash", "bash")]);
    let names =
      requested_resolve(&index, FormatPackage::Deb, MatchType::Path, MatchCombine::Any, &patterns(&["usr/bin/bash"]))
        .unwrap();
    assert_eq!(names, vec!["bash"]);
  }

  #[test]
  #[serial]
  fn requested_resolve_path_refused_on_apk_before_any_pattern() {
    let (_tmp, index) = index_with_facts("req-path-apk", vec![pkg("bash")], &[("/usr/bin", "bash", "bash")]);
    let err =
      requested_resolve(&index, FormatPackage::Apk, MatchType::Path, MatchCombine::Any, &patterns(&["usr/bin/bash"]))
        .unwrap_err();
    assert!(err.to_string().contains("unsupported for apk sources"), "got: {err}");
  }

  #[test]
  #[serial]
  fn requested_resolve_empty_match_fails_naming_the_pattern() {
    let (_tmp, index) = index_with_facts("req-path-empty", vec![pkg("bash")], &[("/usr/bin", "bash", "bash")]);
    let err =
      requested_resolve(&index, FormatPackage::Deb, MatchType::Path, MatchCombine::Any, &patterns(&["zzz/none"]))
        .unwrap_err();
    assert!(err.to_string().contains("zzz/none"), "the failing pattern must be named: {err}");
  }

  #[test]
  #[serial]
  fn requested_resolve_dedups_owners_across_patterns() {
    // Two path patterns owned by the same package collapse to one request;
    // a package shipping several matched paths appears once.
    let (_tmp, index) = index_with_facts(
      "req-dedup",
      vec![pkg("bash")],
      &[("/usr/bin", "bash", "bash"), ("/usr/bin", "bashbug", "bash")],
    );
    let names = requested_resolve(
      &index,
      FormatPackage::Deb,
      MatchType::Path,
      MatchCombine::Any,
      &patterns(&["usr/bin/bash*", "usr/bin/bashbug"]),
    )
    .unwrap();
    assert_eq!(names, vec!["bash"]);
  }

  // ── requested_resolve: --match all (intersection) ──────────────────────

  // covers: INST-061
  #[test]
  #[serial]
  fn requested_resolve_path_intersect_picks_the_common_owner() {
    // usr/sbin/sendmail is shipped by both MTAs; etc/postfix/main.cf only by
    // postfix. `--match all` narrows to the package owning both.
    let (_tmp, index) = index_with_facts(
      "req-path-intersect",
      vec![pkg("postfix"), pkg("exim4")],
      &[
        ("/usr/sbin", "sendmail", "postfix"),
        ("/usr/sbin", "sendmail", "exim4"),
        ("/etc/postfix", "main.cf", "postfix"),
      ],
    );
    let names = requested_resolve(
      &index,
      FormatPackage::Deb,
      MatchType::Path,
      MatchCombine::All,
      &patterns(&["usr/sbin/sendmail", "etc/postfix/main.cf"]),
    )
    .unwrap();
    assert_eq!(names, vec!["postfix"]);
  }

  // covers: INST-062
  #[test]
  #[serial]
  fn requested_resolve_path_intersect_disjoint_owners_errors() {
    // Two paths owned by different packages: under `--match all` their
    // intersection is empty, which is a request for a package that does not
    // exist and must fail naming both patterns.
    let (_tmp, index) = index_with_facts(
      "req-path-intersect-empty",
      vec![pkg("foo"), pkg("bar")],
      &[("/usr/bin", "foo", "foo"), ("/usr/bin", "bar", "bar")],
    );
    let err = requested_resolve(
      &index,
      FormatPackage::Deb,
      MatchType::Path,
      MatchCombine::All,
      &patterns(&["usr/bin/foo", "usr/bin/bar"]),
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("every pattern under --match all"), "got: {msg}");
    assert!(msg.contains("usr/bin/foo") && msg.contains("usr/bin/bar"), "names both patterns: {msg}");
  }

  // covers: INST-063
  #[test]
  #[serial]
  fn requested_resolve_path_match_any_unions_owners() {
    // Two disjoint paths under `any`: the union keeps both owners.
    let (_tmp, index) = index_with_facts(
      "req-path-union",
      vec![pkg("foo"), pkg("bar")],
      &[("/usr/bin", "foo", "foo"), ("/usr/bin", "bar", "bar")],
    );
    let names = requested_resolve(
      &index,
      FormatPackage::Deb,
      MatchType::Path,
      MatchCombine::Any,
      &patterns(&["usr/bin/foo", "usr/bin/bar"]),
    )
    .unwrap();
    assert_eq!(names, vec!["foo", "bar"]);
  }

  // covers: INST-064
  #[test]
  #[serial]
  fn requested_resolve_library_intersect_picks_common_owner() {
    // libGL.so.1 is provided by both stacks; libEGL.so.1 only by mesa.
    // Intersecting the two sonames narrows to mesa.
    let (_tmp, index) = index_with_facts(
      "req-lib-intersect",
      vec![
        pkg_provides("mesa", &["libGL.so.1", "libEGL.so.1"]),
        pkg_provides("nvidia", &["libGL.so.1"]),
      ],
      &[],
    );
    let names = requested_resolve(
      &index,
      FormatPackage::Deb,
      MatchType::Library,
      MatchCombine::All,
      &patterns(&["libGL.so.1", "libEGL.so.1"]),
    )
    .unwrap();
    assert_eq!(names, vec!["mesa"]);
  }

  // covers: INST-065
  #[test]
  #[serial]
  fn requested_resolve_library_intersect_disjoint_errors() {
    let (_tmp, index) = index_with_facts(
      "req-lib-intersect-disjoint",
      vec![
        pkg_provides("liba", &["libfoo.so.1"]),
        pkg_provides("libb", &["libbar.so.1"]),
      ],
      &[],
    );
    let err = requested_resolve(
      &index,
      FormatPackage::Deb,
      MatchType::Library,
      MatchCombine::All,
      &patterns(&["libfoo.so.1", "libbar.so.1"]),
    )
    .unwrap_err();
    assert!(err.to_string().contains("every pattern under --match all"), "got: {}", err);
  }

  // covers: INST-066
  #[test]
  #[serial]
  fn requested_resolve_intersect_single_pattern_identity() {
    // One pattern under `all` is a degenerate no-op: the intersection of a
    // single owner set is itself.
    let (_tmp, index) =
      index_with_facts("req-path-intersect-single", vec![pkg("bash")], &[("/usr/bin", "bash", "bash")]);
    let names =
      requested_resolve(&index, FormatPackage::Deb, MatchType::Path, MatchCombine::All, &patterns(&["usr/bin/bash"]))
        .unwrap();
    assert_eq!(names, vec!["bash"]);
  }

  // covers: INST-067
  #[test]
  #[serial]
  fn requested_resolve_intersect_empty_pattern_errors_per_pattern() {
    // A pattern matching nothing fails with the per-pattern error, NOT the
    // empty-intersection error — the two failure paths stay distinct even
    // under `--match all`.
    let (_tmp, index) =
      index_with_facts("req-path-intersect-emptypat", vec![pkg("bash")], &[("/usr/bin", "bash", "bash")]);
    let err = requested_resolve(
      &index,
      FormatPackage::Deb,
      MatchType::Path,
      MatchCombine::All,
      &patterns(&["usr/bin/bash", "zzz/none"]),
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Path pattern 'zzz/none' matched no package"), "per-pattern empty fires first: {msg}");
    assert!(!msg.contains("every pattern under --match all"), "not the intersection error: {msg}");
  }

  // covers: INST-068
  #[test]
  #[serial]
  fn requested_resolve_intersect_with_glob_expansion() {
    // A wildcard pattern expands to several owners; intersecting it with a
    // second pattern only one of them satisfies narrows to that one.
    let (_tmp, index) = index_with_facts(
      "req-path-intersect-glob",
      vec![pkg("foo"), pkg("bar")],
      &[
        ("/usr/bin", "foo", "foo"),
        ("/usr/bin", "foobar", "bar"),
        ("/etc", "foo.conf", "foo"),
      ],
    );
    let names = requested_resolve(
      &index,
      FormatPackage::Deb,
      MatchType::Path,
      MatchCombine::All,
      &patterns(&["usr/bin/foo*", "etc/foo.conf"]),
    )
    .unwrap();
    assert_eq!(names, vec!["foo"]);
  }
}
