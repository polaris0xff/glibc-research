//! The base packages a distribution family seeds into every resolve
//! regardless of the request, so a bare install still produces a working
//! base. Families that share a baseline (the RHEL descendants, the
//! pacman descendants) draw it from one place here, so a change is made
//! once; a distribution with its own baseline (openSUSE, Debian, Alpine)
//! keeps it in its own builder.

/// The RHEL-family baseline (CentOS, Fedora, AlmaLinux, Rocky).
pub(crate) const RHEL: &[&str] = &["filesystem", "basesystem", "bash", "coreutils", "glibc", "glibc-common"];

/// The pacman-family baseline (Arch Linux, CachyOS).
pub(crate) const PACMAN: &[&str] = &["filesystem", "glibc", "bash", "coreutils"];

/// One family's base list in the owned form `SourceDistro` carries.
pub(crate) fn from_table(table: &[&str]) -> Vec<String> {
  table.iter().map(|s| s.to_string()).collect()
}
