//! Resolver validation: openSUSE Leap vs Docker opensuse/leap.

mod resolver;

use serial_test::serial;

#[test]
#[serial]
fn leap_15_5_curl() {
  resolver::rpm::validate_rpm("opensuse:15.5", "opensuse/leap:15.5", "curl");
}
