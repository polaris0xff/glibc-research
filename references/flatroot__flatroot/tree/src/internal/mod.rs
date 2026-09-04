//! Domain-neutral plumbing that owes nothing to the rootfs domain and
//! could be lifted into an unrelated program untouched.

pub mod byte_size;
pub mod cache;
pub mod codec;
pub mod fd;
pub mod fs;
pub mod http;
pub mod shout;
pub mod tar;
pub mod tool;
