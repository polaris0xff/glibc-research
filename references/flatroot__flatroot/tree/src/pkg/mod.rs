//! The package format parsers and extractors, one module per format,
//! so everything downstream works with one uniform notion of a package.

mod apk;
mod deb;
mod dispatch;
mod format;
mod pacman;
mod rpm;

pub use apk::ApkFormat;
pub use deb::DebFormat;
pub use format::PackageFormat;
pub use pacman::PacmanFormat;
pub use rpm::RpmFormat;
