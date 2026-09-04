//! The `search` command: matches glob patterns against a distribution's
//! package names, library files, or installed paths.

use std::collections::HashSet;

use anyhow::{Result, bail};

use flatroot::arch::Arch;
use flatroot::db::Index;
use flatroot::distro::FormatPackage;
use flatroot::library::LibraryMatch;
use flatroot::path::PathMatch;

use crate::commands::arch_context::ArchContext;
use crate::commands::report::Report;
use crate::commands::session::Session;

/// One package-name search hit.
#[derive(serde::Serialize)]
struct SearchEntry {
  /// Short description from the source's index.
  description: String,
  /// Package name as the index records it.
  name: String,
  /// Version the index records, in the source's native syntax.
  version: String,
}

/// One library search hit: the matched library file and the package
/// that provides it.
#[derive(serde::Serialize)]
struct LibraryEntry {
  /// Short description of the owning package.
  description: String,
  /// The shared object or static archive the glob matched, as the index
  /// records it (e.g. `usr/lib/x86_64-linux-gnu/libssl.so.3`).
  library: String,
  /// Owning package name — what to install to acquire `library`.
  name: String,
  /// Version of the owning package.
  version: String,
}

/// One path search hit: the matched installed path and the package that
/// ships it.
#[derive(serde::Serialize)]
struct PathEntry {
  /// Short description of the owning package.
  description: String,
  /// Owning package name — what to install to acquire `path`.
  name: String,
  /// Full installed path the glob matched, leading slash stripped (e.g.
  /// `usr/bin/bash`).
  path: String,
  /// Version of the owning package.
  version: String,
}

/// Opens the package index, then dispatches on `match_type`.
pub fn run(
  session: &Session,
  remote_str: &str,
  arch: Arch,
  patterns: &[String],
  match_type: crate::parser::MatchType,
  combine: crate::parser::MatchCombine,
  format: crate::parser::OutputFormat,
) -> Result<()> {
  let ctx = session.context_open_blocking(remote_str, arch)?;

  match match_type {
    crate::parser::MatchType::Package => run_package(&ctx.index, patterns, format),
    crate::parser::MatchType::Library => run_library(&ctx.index, patterns, combine, format),
    crate::parser::MatchType::Path => run_path(&ctx, patterns, combine, format),
  }
}

/// Matches every pattern against package names; hits are sorted by name
/// and deduplicated.
fn run_package(index: &Index, patterns: &[String], format: crate::parser::OutputFormat) -> Result<()> {
  let mut entries: Vec<SearchEntry> = Vec::new();
  for pattern in patterns {
    for p in index.packages().glob(pattern)? {
      entries.push(SearchEntry {
        description: p.description,
        name: p.name,
        version: p.version,
      });
    }
  }
  entries.sort_by(|a, b| a.name.cmp(&b.name));
  entries.dedup_by(|a, b| a.name == b.name);

  Report::emit(&["search"], &entries, format)
}

/// Matches every pattern against library files. `combine` unions or
/// intersects the per-pattern owner sets; hits are sorted and
/// deduplicated by library and package.
fn run_library(
  index: &Index,
  patterns: &[String],
  combine: crate::parser::MatchCombine,
  format: crate::parser::OutputFormat,
) -> Result<()> {
  let mut entries: Vec<LibraryEntry> = Vec::new();
  let mut groups: Vec<Vec<String>> = Vec::new();
  for pattern in patterns {
    let mut owners: Vec<String> = Vec::new();
    for m in LibraryMatch::glob(index, pattern)? {
      owners.push(m.package.clone());
      entries.push(LibraryEntry {
        description: m.description,
        library: m.library,
        name: m.package,
        version: m.version,
      });
    }
    groups.push(owners);
  }
  // Under `Any` the fold yields every owner, so the retain keeps every
  // entry; under `All` only entries whose package matched every pattern
  // survive.
  let keep: HashSet<String> = combine.owners_fold(&groups).into_iter().collect();
  entries.retain(|e| keep.contains(&e.name));
  entries.sort_by(|a, b| (&a.library, &a.name).cmp(&(&b.library, &b.name)));
  entries.dedup_by(|a, b| a.library == b.library && a.name == b.name);

  Report::emit(&["search"], &entries, format)
}

/// Matches every pattern against installed paths. `combine` unions or
/// intersects the per-pattern owner sets. An apk source is refused,
/// since Alpine publishes no file lists.
fn run_path(
  ctx: &ArchContext,
  patterns: &[String],
  combine: crate::parser::MatchCombine,
  format: crate::parser::OutputFormat,
) -> Result<()> {
  if matches!(ctx.remote.format(), FormatPackage::Apk) {
    bail!(
      "--type path is unsupported for apk sources: Alpine publishes no file lists (APKINDEX carries no file-to-package data)"
    );
  }

  let mut entries: Vec<PathEntry> = Vec::new();
  let mut groups: Vec<Vec<String>> = Vec::new();
  for pattern in patterns {
    let mut owners: Vec<String> = Vec::new();
    for m in PathMatch::glob(&ctx.index, pattern)? {
      owners.push(m.package.clone());
      entries.push(PathEntry {
        description: m.description,
        name: m.package,
        path: m.path,
        version: m.version,
      });
    }
    groups.push(owners);
  }
  // Under `Any` the fold yields every owner, so the retain keeps every
  // entry; under `All` only entries whose package matched every pattern
  // survive.
  let keep: HashSet<String> = combine.owners_fold(&groups).into_iter().collect();
  entries.retain(|e| keep.contains(&e.name));
  entries.sort_by(|a, b| (&a.path, &a.name).cmp(&(&b.path, &b.name)));
  entries.dedup_by(|a, b| a.path == b.path && a.name == b.name);

  Report::emit(&["search"], &entries, format)
}
