//! What a `--from` choice concretely means: one module per supported
//! distribution, plus the shared registry, release listing, base
//! packages, and format/kind dispatch.

mod alma;
mod alpine;
mod arch;
mod base_packages;
mod cachyos;
mod centos;
mod debian;
mod fedora;
mod format;
mod kind;
mod listing;
mod opensuse;
mod registry;
mod rocky;
mod selector;
mod source;
mod table;
mod ubuntu;

pub use format::FormatPackage;
pub use kind::KindDistro;
pub use listing::{ReleaseScraper, ReleaseSource, WalkStrategy};
pub use registry::{Distro, DistroRegistry};
pub use selector::FromSelector;
pub use source::{LayoutRepository, Repository, SourceDistro};
pub use table::TableRelease;
