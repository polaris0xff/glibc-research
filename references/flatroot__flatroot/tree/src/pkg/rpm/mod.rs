//! The RPM package format: parses the repodata indexes into the package
//! index and extracts `.rpm` files into the rootfs, so nothing else
//! needs to know what an RPM is.

mod container;
mod format;
mod isa;
mod repodata;
mod richdep;

pub use format::RpmFormat;
