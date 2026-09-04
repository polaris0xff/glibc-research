//! Where upstream bytes come from: the `Mirror` trait hides whether a
//! fetch reads a live mirror, a date-pinned snapshot, or an ordered
//! list of fallbacks.

pub mod http;
pub mod list;
mod source;

pub use http::HttpMirror;
pub use list::ListMirror;
pub use source::Mirror;
