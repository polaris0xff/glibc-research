//! `SourceDistro`: the complete description of one chosen distribution
//! and release, from which the rest of the build reads everything
//! distribution-specific.

use super::KindDistro;
use crate::mirror::ListMirror;

/// The complete description of one chosen distribution and release.
#[derive(Clone)]
pub struct SourceDistro {
  /// Which distribution this is, deciding the package format and the
  /// post-install behavior.
  pub kind: KindDistro,
  /// Release identifier as the user typed it, including any `@<date>`
  /// suffix; carried verbatim for diagnostics and cache keys.
  pub release: String,
  /// Architecture in the distro's native naming (`amd64`, `x86_64`, …),
  /// translated by the per-distro builder.
  pub native_arch: String,
  /// Cache namespace (e.g. `"debian/bookworm/amd64"`) the index and
  /// download directory key off.
  pub cache_id: String,
  /// Repositories in dependency-precedence order, each with its own mirror
  /// list and URL templates.
  pub repos: Vec<Repository>,
  /// Packages the resolver seeds regardless of user input. Empty for
  /// Debian and Ubuntu — the `Essential:` flag is their equivalent signal.
  pub base_packages: Vec<String>,
}

/// One repository packages come from: its preference-ordered mirror
/// list and the layout of contents inside each mirror.
#[derive(Clone, Debug)]
pub struct Repository {
  /// Human-readable identifier surfaced in error messages and
  /// progress lines — e.g. `"ubuntu:noble-security/main"`.
  pub label: String,
  /// Mirrors serving this repository, ordered by preference.
  /// The format parser tries each mirror in turn until one
  /// succeeds.
  pub mirrors: ListMirror,
  /// Format-specific URL templates inside each mirror — the
  /// paths the format parser fetches for indexes and the
  /// directory each archive lives under.
  pub layout: LayoutRepository,
}

/// Where a repository's indexes and archives live inside a mirror, one
/// variant per package format; each variant carries only the paths its
/// format has.
#[derive(Clone, Debug)]
pub enum LayoutRepository {
  /// Debian-family layout — Debian and Ubuntu repositories.
  Deb {
    /// Suite directory under the mirror root, e.g.
    /// `"dists/bookworm"`. The format parser appends the
    /// component path under this directory.
    path_dir_suite: String,
    /// Component name within the suite — `"main"`,
    /// `"universe"`, etc.
    component: String,
    /// Architecture in deb-native naming — `"amd64"`,
    /// `"arm64"`, `"i386"`, etc.
    arch: String,
    /// Contents listings (`Contents-<arch>.gz`, `Contents-all.gz`)
    /// that contribute to the binary path-index, in fetch order —
    /// zero or more per repo, populated on the repo that publishes
    /// contents for the suite (so the index ingestion does not
    /// duplicate entries across components). A listing absent on
    /// every mirror is skipped at fetch time — the long-term
    /// archive folds `Contents-all` into the per-arch listing and
    /// drops the separate file — but a repo whose every declared
    /// listing is absent fails population.
    paths_file_contents: Vec<String>,
  },
  /// RPM-family layout — CentOS, Fedora, AlmaLinux, Rocky,
  /// openSUSE.
  Rpm {
    /// Directory containing `repodata/repomd.xml`. The format
    /// parser fetches repomd, follows the `data` hrefs to
    /// `primary.xml` and `filelists.xml`, and records, alongside
    /// each package, the base address of the mirror that just
    /// served the index. Pairing every package with the mirror
    /// it came from lets each archive later be fetched from the
    /// very mirror that delivered its index entry, rather than
    /// from a possibly different one.
    path_dir_repo: String,
  },
  /// Pacman-family layout — Arch Linux and CachyOS.
  Pacman {
    /// Path to the repository's compressed package database
    /// (`<repo>.db.tar.gz` or `<repo>.db`).
    path_file_db: String,
    /// Path to the repository's compressed file-list database
    /// (`<repo>.files.tar.gz` or `<repo>.files`).
    path_file_files: String,
    /// Directory under the mirror root where
    /// `.pkg.tar.zst` archives live. The format parser pairs
    /// each package with the base address it must be fetched
    /// from, so that downloads still route to the right host
    /// even when one distribution draws its repositories from
    /// several different hosts.
    path_dir_packages: String,
  },
  /// APK layout — Alpine Linux.
  Apk {
    /// Path to the repository's `APKINDEX.tar.gz`.
    path_file_apkindex: String,
    /// Directory under the mirror root where
    /// `<name>-<version>.apk` archives live. The format parser
    /// pairs each parsed package name with the address of the
    /// mirror that served its index, so every archive is later
    /// fetched from that same mirror rather than a possibly
    /// different one.
    path_dir_packages: String,
  },
}
