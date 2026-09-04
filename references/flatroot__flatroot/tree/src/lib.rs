//! Builds a complete Linux root filesystem from a distribution's own
//! published packages — resolving dependencies, fetching, extracting,
//! and running post-install — without elevated privileges and without
//! the host's package manager.

pub mod arch;
pub mod db;
pub mod dep_tree;
pub mod distro;
pub mod downloader;
pub mod elf;
pub mod internal;
pub mod library;
pub mod manifest;
pub mod mirror;
pub mod package;
pub mod path;
pub mod path_index;
pub mod pkg;
pub mod postfixes;
pub mod postinstall;
pub mod remote;
pub mod resolver;
pub mod sandbox;
pub mod ui;
pub mod version;
