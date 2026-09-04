//! Version comparison, one implementation per distribution family
//! behind the shared `VersionCompare` trait.

mod apk;
mod compare;
mod dpkg;
mod pacman;
mod rpm;

pub use compare::{
  ApkVersionCompare, ComparisonOp, DpkgVersionCompare, PacmanVersionCompare, RpmVersionCompare, VersionCompare,
};
