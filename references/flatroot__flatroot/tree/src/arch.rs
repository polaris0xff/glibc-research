//! `Arch`: which architecture a rootfs targets, with the spelling each
//! consumer — kernel, packaging, container tooling, loader — expects
//! for it.

use std::path::Path;

use anyhow::{Result, bail};

/// The architecture a rootfs targets, one of the supported set; each
/// consumer's name for it is derived on demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
  /// 64-bit Intel/AMD.
  X86_64,
  /// 32-bit Intel.
  I686,
  /// 64-bit ARM.
  Aarch64,
  /// 32-bit hard-float ARM.
  Armv7l,
  /// 64-bit RISC-V.
  Riscv64,
}

impl Arch {
  /// The kernel's `uname -m` spelling — the project's canonical form,
  /// what a user types and a machine reports.
  pub fn as_uname(&self) -> &'static str {
    match self {
      Arch::X86_64 => "x86_64",
      Arch::I686 => "i686",
      Arch::Aarch64 => "aarch64",
      Arch::Armv7l => "armv7l",
      Arch::Riscv64 => "riscv64",
    }
  }

  /// The spelling container tooling records as an image's platform, so a
  /// runtime only tries to run it where it fits.
  pub fn as_goarch(&self) -> &'static str {
    match self {
      Arch::X86_64 => "amd64",
      Arch::I686 => "386",
      Arch::Aarch64 => "arm64",
      Arch::Armv7l => "arm",
      Arch::Riscv64 => "riscv64",
    }
  }

  /// The extra qualifier container tooling needs where the platform name
  /// alone is ambiguous (the ARM revision); `None` where none is needed.
  pub fn oci_variant(&self) -> Option<&'static str> {
    match self {
      Arch::Armv7l => Some("v7"),
      _ => None,
    }
  }

  /// The Debian/Ubuntu spelling, which their archives and package
  /// metadata key off rather than the kernel's.
  pub fn as_debian(&self) -> &'static str {
    match self {
      Arch::X86_64 => "amd64",
      Arch::I686 => "i386",
      Arch::Aarch64 => "arm64",
      Arch::Armv7l => "armhf",
      Arch::Riscv64 => "riscv64",
    }
  }

  /// The Alpine spelling, which matches the kernel except on the two
  /// 32-bit targets.
  pub fn as_alpine(&self) -> &'static str {
    match self {
      Arch::X86_64 => "x86_64",
      Arch::I686 => "x86",
      Arch::Aarch64 => "aarch64",
      Arch::Armv7l => "armv7",
      Arch::Riscv64 => "riscv64",
    }
  }

  /// The Debian multiarch subdirectory name the dynamic loader searches
  /// for this target's shared libraries.
  pub fn as_gnu_triplet(&self) -> &'static str {
    match self {
      Arch::X86_64 => "x86_64-linux-gnu",
      Arch::I686 => "i386-linux-gnu",
      Arch::Aarch64 => "aarch64-linux-gnu",
      Arch::Armv7l => "arm-linux-gnueabihf",
      Arch::Riscv64 => "riscv64-linux-gnu",
    }
  }

  /// Parses the uname-spelling target a user typed, accepting both
  /// `x86` and `i686` for 32-bit Intel. An unknown name is an error,
  /// since building for the wrong architecture is worse than a refusal.
  pub fn from_uname(s: &str) -> Result<Arch> {
    match s {
      "x86_64" => Ok(Arch::X86_64),
      "x86" | "i686" => Ok(Arch::I686),
      "aarch64" => Ok(Arch::Aarch64),
      "armv7l" => Ok(Arch::Armv7l),
      "riscv64" => Ok(Arch::Riscv64),
      other => bail!("Unsupported architecture '{}'", other),
    }
  }

  /// The host's own architecture, the default when no target is named.
  /// Resolved at compile time, so an unsupported host fails to compile
  /// rather than hitting an unanswerable case at runtime.
  pub fn from_host() -> Arch {
    #[cfg(target_arch = "x86_64")]
    return Arch::X86_64;
    #[cfg(target_arch = "x86")]
    return Arch::I686;
    #[cfg(target_arch = "aarch64")]
    return Arch::Aarch64;
    #[cfg(target_arch = "arm")]
    return Arch::Armv7l;
    #[cfg(target_arch = "riscv64")]
    return Arch::Riscv64;
  }

  /// Detects the architecture a finished rootfs targets by reading its
  /// own programs' ELF headers, since the build host may be a different
  /// architecture and a mislabeled image would refuse to run.
  pub fn detect(rootfs: &Path) -> Result<Arch> {
    // busybox-based rootfses (e.g. alpine) ship one real ELF — `bin/busybox` —
    // and expose `bin/sh` and friends only as applet symlinks to it, which an
    // absolute symlink leaves unreadable from outside the rootfs. Listing
    // busybox itself lets such a rootfs still report its architecture.
    let candidates = [
      "bin/sh",
      "usr/bin/sh",
      "usr/bin/env",
      "usr/bin/coreutils",
      "bin/busybox",
      "usr/bin/busybox",
    ];

    for candidate in &candidates {
      let path = rootfs.join(candidate);
      // symlink_metadata, not exists: the file may be a symlink that does
      // not resolve outside the rootfs yet is still a real ELF.
      if path.symlink_metadata().is_ok() {
        if let Ok(arch) = Self::detect_elf(&path) {
          return Ok(arch);
        }
      }
    }

    bail!(
      "Could not detect rootfs architecture. None of the expected binaries \
       ({}) were found or readable in {}",
      candidates.join(", "),
      rootfs.display()
    )
  }

  /// Recovers the architecture from an ELF `e_machine` code — the
  /// standardized processor id, constant across kernels. An unsupported
  /// code is an error, never a guess.
  pub fn from_e_machine(e_machine: u16) -> Result<Arch> {
    use object::elf;
    match e_machine {
      elf::EM_X86_64 => Ok(Arch::X86_64),
      elf::EM_386 => Ok(Arch::I686),
      elf::EM_AARCH64 => Ok(Arch::Aarch64),
      elf::EM_ARM => Ok(Arch::Armv7l),
      elf::EM_RISCV => Ok(Arch::Riscv64),
      other => bail!("Unknown ELF e_machine value: {}", other),
    }
  }

  /// Detects the architecture a single program targets from the
  /// `e_machine` field in its ELF header. A non-ELF file or unsupported
  /// architecture is an error, never a guess.
  fn detect_elf(path: &Path) -> Result<Arch> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 20];
    file.read_exact(&mut header)?;

    if &header[0..4] != b"\x7fELF" {
      bail!("Not an ELF file");
    }

    let little_endian = header[5] == 1;
    let e_machine = if little_endian {
      u16::from_le_bytes([header[18], header[19]])
    } else {
      u16::from_be_bytes([header[18], header[19]])
    };

    Self::from_e_machine(e_machine)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn from_uname_round_trips_through_as_uname() {
    for arch in [Arch::X86_64, Arch::I686, Arch::Aarch64, Arch::Armv7l, Arch::Riscv64] {
      assert_eq!(Arch::from_uname(arch.as_uname()).unwrap(), arch);
    }
  }

  #[test]
  fn from_uname_accepts_both_x86_and_i686() {
    assert_eq!(Arch::from_uname("x86").unwrap(), Arch::I686);
    assert_eq!(Arch::from_uname("i686").unwrap(), Arch::I686);
  }

  #[test]
  fn from_uname_rejects_unknown_token() {
    assert!(Arch::from_uname("mips64").is_err());
  }

  #[test]
  fn as_alpine_spells_x86_and_armv7() {
    assert_eq!(Arch::X86_64.as_alpine(), "x86_64");
    assert_eq!(Arch::I686.as_alpine(), "x86");
    assert_eq!(Arch::Aarch64.as_alpine(), "aarch64");
    assert_eq!(Arch::Armv7l.as_alpine(), "armv7");
    assert_eq!(Arch::Riscv64.as_alpine(), "riscv64");
  }

  #[test]
  fn as_gnu_triplet_matches_multiarch_dirs() {
    assert_eq!(Arch::X86_64.as_gnu_triplet(), "x86_64-linux-gnu");
    assert_eq!(Arch::I686.as_gnu_triplet(), "i386-linux-gnu");
    assert_eq!(Arch::Aarch64.as_gnu_triplet(), "aarch64-linux-gnu");
    assert_eq!(Arch::Armv7l.as_gnu_triplet(), "arm-linux-gnueabihf");
    assert_eq!(Arch::Riscv64.as_gnu_triplet(), "riscv64-linux-gnu");
  }
}
