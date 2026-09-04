//! `KindDistro`: the closed set of distributions the tool builds from.

use super::FormatPackage;
use crate::postinstall::PostInstall;

/// One variant per supported distribution, so a `match` covers every
/// case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindDistro {
  Debian,
  Ubuntu,
  ArchLinux,
  Cachyos,
  Alpine,
  Centos,
  Fedora,
  Alma,
  Rocky,
  Opensuse,
}

impl KindDistro {
  /// The package format this distribution uses; several distributions
  /// share one format.
  pub fn format(self) -> FormatPackage {
    match self {
      Self::Debian | Self::Ubuntu => FormatPackage::Deb,
      Self::ArchLinux | Self::Cachyos => FormatPackage::Pacman,
      Self::Alpine => FormatPackage::Apk,
      Self::Centos | Self::Fedora | Self::Alma | Self::Rocky | Self::Opensuse => FormatPackage::Rpm,
    }
  }

  /// The post-install behavior this distribution needs; distributions
  /// sharing a package format can still differ here.
  pub fn post_install(self) -> PostInstall {
    match self {
      Self::Debian | Self::Ubuntu | Self::Centos | Self::Fedora | Self::Alma | Self::Rocky | Self::Opensuse => {
        PostInstall::Debian
      }
      Self::ArchLinux | Self::Cachyos => PostInstall::Arch,
      Self::Alpine => PostInstall::Alpine,
    }
  }

  /// The `--from` prefix a user types to select this distribution.
  pub fn prefix(self) -> &'static str {
    match self {
      Self::Debian => "debian",
      Self::Ubuntu => "ubuntu",
      Self::ArchLinux => "arch",
      Self::Cachyos => "cachyos",
      Self::Alpine => "alpine",
      Self::Centos => "centos",
      Self::Fedora => "fedora",
      Self::Alma => "alma",
      Self::Rocky => "rocky",
      Self::Opensuse => "opensuse",
    }
  }
}
