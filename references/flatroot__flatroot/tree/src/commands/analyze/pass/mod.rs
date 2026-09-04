//! The two analysis passes: `declared` resolves the dependencies the
//! package metadata lists, `linker` resolves the shared libraries the
//! packages' ELF binaries load at run time.

pub mod declared;
pub mod linker;
