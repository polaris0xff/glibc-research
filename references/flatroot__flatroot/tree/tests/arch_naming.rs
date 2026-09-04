//! Per-ecosystem architecture naming exercised through the public
//! `flatroot::arch::Arch` enum. The same chip wears a different word in every
//! packaging world — kernel `uname`, Debian, Alpine, the Go/OCI `goarch`, the
//! OCI variant qualifier, and the GNU multiarch triplet — and `Arch` is the
//! single place that hands out each spelling. These tests pin down every
//! rendering for all five supported targets, the kernel-vocabulary parse
//! (including the two 32-bit spellings and the unknown-token rejection), and
//! that `from_host` resolves to one of the supported targets.
//!
//! All spellings are copied verbatim from `src/arch.rs` (`as_uname`,
//! `as_goarch`, `oci_variant`, `as_debian`, `as_alpine`, `as_gnu_triplet`,
//! `from_uname`, `from_host`).

use flatroot::arch::Arch;

const ALL: [Arch; 5] = [Arch::X86_64, Arch::I686, Arch::Aarch64, Arch::Armv7l, Arch::Riscv64];

// ---------------------------------------------------------------------------
// uname spelling + round-trip (CORE-035 lib leg, CORE-038)
// ---------------------------------------------------------------------------

// covers: CORE-035, CORE-038
#[test]
fn as_uname_spells_every_target_and_round_trips() {
  assert_eq!(Arch::X86_64.as_uname(), "x86_64");
  assert_eq!(Arch::I686.as_uname(), "i686");
  assert_eq!(Arch::Aarch64.as_uname(), "aarch64");
  assert_eq!(Arch::Armv7l.as_uname(), "armv7l");
  assert_eq!(Arch::Riscv64.as_uname(), "riscv64");

  // Every target round-trips through its own uname spelling.
  for arch in ALL {
    assert_eq!(Arch::from_uname(arch.as_uname()).unwrap(), arch, "round-trip failed for {}", arch.as_uname());
  }
}

// covers: CORE-035
#[test]
fn from_uname_accepts_both_32bit_spellings_and_rejects_unknown() {
  // Both historical names for 32-bit Intel map to I686.
  assert_eq!(Arch::from_uname("x86").unwrap(), Arch::I686);
  assert_eq!(Arch::from_uname("i686").unwrap(), Arch::I686);

  // An unfamiliar token is refused outright with the source's exact wording.
  let err = Arch::from_uname("mips64").err().expect("expected an error");
  assert!(
    err.to_string().contains("Unsupported architecture 'mips64'"),
    "unknown arch must bail with the exact message, got: {err}"
  );
  assert!(Arch::from_uname("").is_err(), "empty token is unsupported");
  assert!(Arch::from_uname("amd64").is_err(), "Debian spelling is not kernel vocabulary");
}

// ---------------------------------------------------------------------------
// goarch / oci_variant (CORE-038)
// ---------------------------------------------------------------------------

// covers: CORE-038
#[test]
fn as_goarch_spells_the_oci_platform_vocabulary() {
  assert_eq!(Arch::X86_64.as_goarch(), "amd64");
  assert_eq!(Arch::I686.as_goarch(), "386");
  assert_eq!(Arch::Aarch64.as_goarch(), "arm64");
  assert_eq!(Arch::Armv7l.as_goarch(), "arm");
  assert_eq!(Arch::Riscv64.as_goarch(), "riscv64");
}

// covers: CORE-038
#[test]
fn oci_variant_is_present_only_for_armv7l() {
  assert_eq!(Arch::Armv7l.oci_variant(), Some("v7"), "armv7l needs the v7 qualifier");
  for arch in [Arch::X86_64, Arch::I686, Arch::Aarch64, Arch::Riscv64] {
    assert_eq!(arch.oci_variant(), None, "{} carries no OCI variant", arch.as_uname());
  }
}

// ---------------------------------------------------------------------------
// debian / alpine spellings (CORE-038)
// ---------------------------------------------------------------------------

// covers: CORE-038
#[test]
fn as_debian_spells_the_deb_family_vocabulary() {
  assert_eq!(Arch::X86_64.as_debian(), "amd64");
  assert_eq!(Arch::I686.as_debian(), "i386");
  assert_eq!(Arch::Aarch64.as_debian(), "arm64");
  assert_eq!(Arch::Armv7l.as_debian(), "armhf");
  assert_eq!(Arch::Riscv64.as_debian(), "riscv64");
}

// covers: CORE-038
#[test]
fn as_alpine_agrees_with_kernel_except_the_two_32bit_targets() {
  assert_eq!(Arch::X86_64.as_alpine(), "x86_64");
  assert_eq!(Arch::I686.as_alpine(), "x86");
  assert_eq!(Arch::Aarch64.as_alpine(), "aarch64");
  assert_eq!(Arch::Armv7l.as_alpine(), "armv7");
  assert_eq!(Arch::Riscv64.as_alpine(), "riscv64");
}

// ---------------------------------------------------------------------------
// gnu triplet (CORE-038)
// ---------------------------------------------------------------------------

// covers: CORE-038
#[test]
fn as_gnu_triplet_names_the_multiarch_library_dir() {
  assert_eq!(Arch::X86_64.as_gnu_triplet(), "x86_64-linux-gnu");
  assert_eq!(Arch::I686.as_gnu_triplet(), "i386-linux-gnu");
  assert_eq!(Arch::Aarch64.as_gnu_triplet(), "aarch64-linux-gnu");
  assert_eq!(Arch::Armv7l.as_gnu_triplet(), "arm-linux-gnueabihf");
  assert_eq!(Arch::Riscv64.as_gnu_triplet(), "riscv64-linux-gnu");
}

// ---------------------------------------------------------------------------
// every renderer distinguishes every target (no two targets collapse)
// ---------------------------------------------------------------------------

// covers: CORE-038
#[test]
fn each_renderer_is_injective_across_targets() {
  // For every naming scheme, the five targets must map to five distinct
  // spellings (with the one documented exception: only armv7l has an OCI
  // variant, so that field is not required to be injective).
  use std::collections::BTreeSet;
  let uname: BTreeSet<&str> = ALL.iter().map(|a| a.as_uname()).collect();
  let goarch: BTreeSet<&str> = ALL.iter().map(|a| a.as_goarch()).collect();
  let debian: BTreeSet<&str> = ALL.iter().map(|a| a.as_debian()).collect();
  let alpine: BTreeSet<&str> = ALL.iter().map(|a| a.as_alpine()).collect();
  let triplet: BTreeSet<&str> = ALL.iter().map(|a| a.as_gnu_triplet()).collect();
  assert_eq!(uname.len(), 5, "uname spellings must all differ");
  assert_eq!(goarch.len(), 5, "goarch spellings must all differ");
  assert_eq!(debian.len(), 5, "debian spellings must all differ");
  assert_eq!(alpine.len(), 5, "alpine spellings must all differ");
  assert_eq!(triplet.len(), 5, "gnu triplets must all differ");
}

// ---------------------------------------------------------------------------
// from_host resolves to a supported target whose uname round-trips (CORE-036)
// ---------------------------------------------------------------------------

// covers: CORE-036
#[test]
fn from_host_resolves_to_a_supported_target() {
  // The tool only ships for the supported targets, so the host is guaranteed
  // to be one of them. `from_host()` must therefore return a target whose
  // uname spelling round-trips and equals the process's own ARCH-derived
  // expectation.
  let host = Arch::from_host();
  assert!(ALL.contains(&host), "from_host must yield one of the supported targets, got {}", host.as_uname());
  // Its uname spelling parses back to the same target.
  assert_eq!(Arch::from_uname(host.as_uname()).unwrap(), host);

  // And it matches the mapping the source applies over std::env::consts::ARCH.
  let expected = match std::env::consts::ARCH {
    "x86_64" => Arch::X86_64,
    "aarch64" => Arch::Aarch64,
    "arm" => Arch::Armv7l,
    "x86" => Arch::I686,
    "riscv64" => Arch::Riscv64,
    other => panic!("test running on an unsupported host arch '{other}'"),
  };
  assert_eq!(host, expected, "from_host must map std::env::consts::ARCH the same way the source does");
}
