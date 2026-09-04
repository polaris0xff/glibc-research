//! Resolver validation: Debian bookworm vs Docker debian:bookworm.

mod resolver;

use serial_test::serial;

#[test]
#[serial]
fn bookworm_curl() {
  resolver::deb::validate_deb("debian:bookworm", "debian:bookworm", "curl");
}

#[test]
#[serial]
fn bookworm_firefox() {
  resolver::deb::validate_deb("debian:bookworm", "debian:bookworm", "firefox-esr");
}

#[test]
#[serial]
fn bookworm_gimp() {
  resolver::deb::validate_deb("debian:bookworm", "debian:bookworm", "gimp");
}

#[test]
#[serial]
fn trixie_curl() {
  resolver::deb::validate_deb("debian:trixie", "debian:trixie", "curl");
}

#[test]
#[serial]
fn forky_curl() {
  resolver::deb::validate_deb("debian:forky", "debian:forky", "curl");
}

#[test]
#[serial]
fn bullseye_curl() {
  resolver::deb::validate_deb("debian:bullseye", "debian:bullseye", "curl");
}
