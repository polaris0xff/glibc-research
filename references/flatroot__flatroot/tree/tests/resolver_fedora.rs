//! Resolver validation: Fedora 42 vs Docker fedora:42.

mod resolver;

use serial_test::serial;

#[test]
#[serial]
fn fedora42_bash() {
  resolver::rpm::validate_rpm("fedora:42", "fedora:42", "bash");
}

#[test]
#[serial]
fn fedora42_firefox() {
  resolver::rpm::validate_rpm("fedora:42", "fedora:42", "firefox");
}

#[test]
#[serial]
fn fedora42_gimp() {
  resolver::rpm::validate_rpm("fedora:42", "fedora:42", "gimp");
}

#[test]
#[serial]
fn fedora42_inkscape() {
  resolver::rpm::validate_rpm("fedora:42", "fedora:42", "inkscape");
}

#[test]
#[serial]
fn fedora44_curl() {
  resolver::rpm::validate_rpm("fedora:44", "fedora:44", "curl");
}

#[test]
#[serial]
fn fedora43_curl() {
  resolver::rpm::validate_rpm("fedora:43", "fedora:43", "curl");
}

#[test]
#[serial]
fn fedora41_curl() {
  resolver::rpm::validate_rpm("fedora:41", "fedora:41", "curl");
}

#[test]
#[serial]
fn fedora40_curl() {
  resolver::rpm::validate_rpm("fedora:40", "fedora:40", "curl");
}

#[test]
#[serial]
fn fedora_rawhide_curl() {
  resolver::rpm::validate_rpm("fedora:rawhide", "fedora:rawhide", "curl");
}
