//! Final corrections applied after extraction and post-install, fixing
//! what an unprivileged build could not reproduce.

mod apply;
pub mod permissions;

pub use apply::Postfix;
