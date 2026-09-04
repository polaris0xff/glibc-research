//! Resolver validation: Ubuntu noble vs Docker ubuntu:noble.

mod resolver;

use serial_test::serial;

#[test]
#[serial]
fn noble_curl() {
  resolver::deb::validate_deb("ubuntu:noble", "ubuntu:noble", "curl");
}

#[test]
#[serial]
fn noble_firefox() {
  resolver::deb::validate_deb("ubuntu:noble", "ubuntu:noble", "firefox");
}

#[test]
#[serial]
fn noble_gimp() {
  resolver::deb::validate_deb("ubuntu:noble", "ubuntu:noble", "gimp");
}

#[test]
#[serial]
fn jammy_curl() {
  resolver::deb::validate_deb("ubuntu:jammy", "ubuntu:jammy", "curl");
}

#[test]
#[serial]
fn bionic_curl() {
  resolver::deb::validate_deb("ubuntu:bionic", "ubuntu:bionic", "curl");
}

#[test]
#[serial]
fn xenial_curl() {
  resolver::deb::validate_deb("ubuntu:xenial", "ubuntu:xenial", "curl");
}
