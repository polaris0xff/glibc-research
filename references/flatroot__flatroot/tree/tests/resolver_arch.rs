//! Resolver validation: Arch rolling vs Docker archlinux:latest.
//!
//! `arch:multilib` has no resolver test: Docker Hub does not publish
//! an `archlinux:multilib` image, and `archlinux:latest` ships with
//! the multilib repository disabled, so there is no upstream pacman
//! state to validate flatroot's multilib resolution against.

mod resolver;

use serial_test::serial;

#[test]
#[serial]
fn rolling_curl() {
  resolver::pacman::validate_pacman("arch:rolling", "archlinux:latest", "curl");
}

#[test]
#[serial]
fn rolling_firefox() {
  resolver::pacman::validate_pacman("arch:rolling", "archlinux:latest", "firefox");
}

#[test]
#[serial]
fn rolling_gimp() {
  resolver::pacman::validate_pacman("arch:rolling", "archlinux:latest", "gimp");
}
