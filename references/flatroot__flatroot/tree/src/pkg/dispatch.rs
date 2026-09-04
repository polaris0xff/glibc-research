//! Maps a `FormatPackage` to the `PackageFormat` backend that parses
//! its indexes and extracts its archives.

use crate::distro::FormatPackage;
use crate::pkg::{ApkFormat, DebFormat, PackageFormat, PacmanFormat, RpmFormat};

impl FormatPackage {
  /// The backend that parses this format's indexes and extracts its
  /// archives.
  pub fn backend(self) -> &'static dyn PackageFormat {
    match self {
      FormatPackage::Deb => &DebFormat,
      FormatPackage::Rpm => &RpmFormat,
      FormatPackage::Pacman => &PacmanFormat,
      FormatPackage::Apk => &ApkFormat,
    }
  }
}
