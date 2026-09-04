//! `FormatPackage`: the package format a distribution uses — what index
//! parsing and archive extraction dispatch on.

use crate::manifest::ChecksumAlgorithm;
use crate::version::VersionCompare;

/// The package format a distribution uses; distributions sharing a
/// format share the same index parser and extractor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatPackage {
  Deb,
  Rpm,
  Pacman,
  Apk,
}

impl FormatPackage {
  /// The checksum algorithm this format publishes for its packages,
  /// recorded explicitly in the manifest rather than guessed from a
  /// digest's length.
  pub fn checksum_type(self) -> ChecksumAlgorithm {
    match self {
      Self::Deb | Self::Rpm | Self::Pacman => ChecksumAlgorithm::Sha256,
      Self::Apk => ChecksumAlgorithm::Q1Sha1,
    }
  }

  /// The version comparison rules this format defines. Version strings
  /// only look alike across formats, so each format gets its own
  /// comparator.
  pub fn version_compare(self) -> Box<dyn VersionCompare> {
    match self {
      Self::Deb => Box::new(crate::version::DpkgVersionCompare),
      Self::Rpm => Box::new(crate::version::RpmVersionCompare),
      Self::Pacman => Box::new(crate::version::PacmanVersionCompare),
      Self::Apk => Box::new(crate::version::ApkVersionCompare),
    }
  }
}
