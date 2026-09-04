//! Resolver validation: CentOS 7 vs Docker centos:7.

mod resolver;

use serial_test::serial;

#[test]
#[serial]
fn centos7_curl() {
  resolver::rpm::validate_rpm("centos:7", "centos:7", "curl");
}

#[test]
#[serial]
fn centos7_firefox() {
  resolver::rpm::validate_rpm("centos:7", "centos:7", "firefox");
}

#[test]
#[serial]
fn centos7_gimp() {
  resolver::rpm::validate_rpm("centos:7", "centos:7", "gimp");
}

#[test]
#[serial]
fn centos8_curl() {
  resolver::rpm::validate_rpm("centos:8", "centos:8", "curl");
}
