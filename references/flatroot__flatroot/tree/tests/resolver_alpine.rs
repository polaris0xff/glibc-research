//! Resolver validation: Alpine edge vs Docker alpine:edge.

mod resolver;

use serial_test::serial;

#[test]
#[serial]
fn edge_curl() {
  resolver::apk::validate_apk("alpine:edge", "alpine:edge", "curl");
}

#[test]
#[serial]
fn edge_firefox() {
  resolver::apk::validate_apk("alpine:edge", "alpine:edge", "firefox");
}

#[test]
#[serial]
fn edge_gimp() {
  resolver::apk::validate_apk("alpine:edge", "alpine:edge", "gimp");
}

#[test]
#[serial]
fn v3_23_curl() {
  resolver::apk::validate_apk("alpine:v3.23", "alpine:3.23", "curl");
}

#[test]
#[serial]
fn v3_22_curl() {
  resolver::apk::validate_apk("alpine:v3.22", "alpine:3.22", "curl");
}

#[test]
#[serial]
fn v3_21_curl() {
  resolver::apk::validate_apk("alpine:v3.21", "alpine:3.21", "curl");
}

#[test]
#[serial]
fn v3_20_curl() {
  resolver::apk::validate_apk("alpine:v3.20", "alpine:3.20", "curl");
}
