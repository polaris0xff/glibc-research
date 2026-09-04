//! The `Remote` interface the build talks to once a source is chosen —
//! index fetching, download URLs, extraction, post-install — with no
//! caller knowing which distribution family it is.

mod soname;
mod source;

pub use soname::Soname;
pub use source::{Remote, RemoteDistro};
