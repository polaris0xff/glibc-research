//! Resolves a binary's needed library name (soname) to the package that
//! ships it, with one `SonameDialect` per package format.

use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::arch::Arch;
use crate::db::Index;
use crate::distro::{FormatPackage, SourceDistro};
use crate::elf::{ElfClass, ElfDeps};

/// How one package format records which package ships a library.
enum SonameDialect {
  /// Debian-family resolution. The Debian architecture name
  /// (`amd64`/`arm64`/…) is carried from the source; the
  /// multiarch triplet it implies is derived from the
  /// `(deb_arch, arch_class)` pair at resolve time.
  Deb { deb_arch: String },
  /// RPM-family resolution — arch-suffixed then bare provides,
  /// then the class-appropriate lib dirs.
  Rpm,
  /// Pacman-family resolution — loader-form then major-stripped
  /// provides, then `/usr/lib`+`/lib`.
  Pacman,
  /// Alpine resolution — the `so:`-namespaced provides surface
  /// only; no path index is consulted.
  Apk,
}

/// Resolves sonames to providing packages: the active `SonameDialect`
/// plus the package index, chosen once per walk.
pub struct Soname<'a> {
  dialect: SonameDialect,
  index: &'a Index,
}

impl<'a> Soname<'a> {
  /// Picks the dialect matching the source's format, keeping the
  /// Debian architecture name (its lib dirs are per-arch); the other
  /// formats need nothing extra.
  pub fn for_source(format: FormatPackage, source: &SourceDistro, index: &'a Index) -> Soname<'a> {
    let dialect = match format {
      FormatPackage::Deb => SonameDialect::Deb {
        deb_arch: source.native_arch.clone(),
      },
      FormatPackage::Rpm => SonameDialect::Rpm,
      FormatPackage::Pacman => SonameDialect::Pacman,
      FormatPackage::Apk => SonameDialect::Apk,
    };
    Soname { dialect, index }
  }

  /// Resolves one soname to its providing package by the format's own
  /// procedure: the most precise provides entry first, then looser
  /// matches, then the file-to-package index. The
  /// architecture-suffixed provides entry is checked before the bare
  /// one, so a library built for both ELF classes is not claimed by the
  /// wrong build. `None` means genuinely unresolved, never a guess.
  pub fn resolve(&self, soname: &str, deps: &ElfDeps) -> Result<Option<String>> {
    let providers = self.index.providers();
    match &self.dialect {
      SonameDialect::Deb { .. } => {
        let dirs = self.candidate_dirs(deps.class)?;
        self.probe(&dirs, soname, &deps.runpath)
      }
      SonameDialect::Rpm => {
        let suffix = match deps.class {
          ElfClass::Elf64 => "()(64bit)",
          ElfClass::Elf32 => "()(32bit)",
        };
        let mangled = format!("{}{}", soname, suffix);
        if let Some(pkg) = providers.first(&mangled)? {
          return Ok(Some(pkg));
        }
        if let Some(pkg) = providers.first(soname)? {
          return Ok(Some(pkg));
        }
        let dirs = self.candidate_dirs(deps.class)?;
        self.probe(&dirs, soname, &deps.runpath)
      }
      SonameDialect::Pacman => {
        if let Some(pkg) = providers.first(soname)? {
          return Ok(Some(pkg));
        }
        if let Some(idx) = soname.rfind(".so")
          && let bare = &soname[..idx + 3]
          && bare != soname
          && let Some(pkg) = providers.first(bare)?
        {
          return Ok(Some(pkg));
        }
        let dirs = self.candidate_dirs(deps.class)?;
        self.probe(&dirs, soname, &deps.runpath)
      }
      SonameDialect::Apk => {
        let prefixed = format!("so:{}", soname);
        providers.first(&prefixed)
      }
    }
  }

  /// The directories to search for a library of this ELF class, in
  /// order, for the file-to-package fallback. A `(deb_arch, class)`
  /// pair with no known layout is an error, not a default.
  fn candidate_dirs(&self, arch_class: ElfClass) -> Result<Vec<String>> {
    Ok(match &self.dialect {
      SonameDialect::Deb { deb_arch } => {
        let arch = match (deb_arch.as_str(), arch_class) {
          ("amd64", ElfClass::Elf64) => Arch::X86_64,
          ("i386", ElfClass::Elf32) => Arch::I686,
          ("arm64", ElfClass::Elf64) => Arch::Aarch64,
          ("armhf", ElfClass::Elf32) => Arch::Armv7l,
          ("riscv64", ElfClass::Elf64) => Arch::Riscv64,
          _ => bail!("unsupported (deb_arch={}, class={:?}) pair for soname lookup", deb_arch, arch_class),
        };
        let triplet = arch.as_gnu_triplet();
        vec![
          format!("/lib/{}", triplet),
          format!("/usr/lib/{}", triplet),
          "/lib".to_string(),
          "/usr/lib".to_string(),
        ]
      }
      SonameDialect::Rpm => match arch_class {
        ElfClass::Elf64 => vec!["/usr/lib64".to_string(), "/lib64".to_string(), "/usr/lib".to_string()],
        ElfClass::Elf32 => vec!["/usr/lib".to_string(), "/lib".to_string()],
      },
      SonameDialect::Pacman => vec!["/usr/lib".to_string(), "/lib".to_string()],
      SonameDialect::Apk => Vec::new(),
    })
  }

  /// Looks up the owning package by path, trying the binary's own
  /// `DT_RUNPATH` directories before the format's defaults, stopping at
  /// the first owner.
  fn probe(&self, dirs: &[String], soname: &str, runpath_hint: &[PathBuf]) -> Result<Option<String>> {
    let Some(index) = self.index.paths()? else {
      return Ok(None);
    };
    for rp in runpath_hint {
      let rp_str = rp.display().to_string();
      if Self::placeholder_has(&rp_str) {
        continue;
      }
      let path = format!("{}/{}", rp_str.trim_end_matches('/'), soname);
      if let Some(pkg) = index.query(&path)? {
        return Ok(Some(pkg));
      }
    }
    for d in dirs {
      let path = format!("{}/{}", d, soname);
      if let Some(pkg) = index.query(&path)? {
        return Ok(Some(pkg));
      }
    }
    Ok(None)
  }

  /// Whether a search location holds a loader token (`$ORIGIN`, `$LIB`,
  /// `$PLATFORM`) expanded only at run time, so it cannot match the
  /// path index and is skipped.
  fn placeholder_has(rp: &str) -> bool {
    rp.contains("$ORIGIN") || rp.contains("$LIB") || rp.contains("$PLATFORM")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::Index;
  use crate::package::{DepSpec, Package};
  use crate::path_index::PathIndexBuilder;

  fn pkg_with_provides(name: &str, provides: &[&str]) -> Package {
    Package {
      name: name.to_string(),
      version: "1.0".to_string(),
      depends: vec![],
      provides: provides
        .iter()
        .map(|n| DepSpec {
          name: n.to_string(),
          version_constraint: None,
        })
        .collect(),
      recommends: vec![],
      suggests: vec![],
      install_if: vec![],
      conflicts: vec![],
      breaks: vec![],
      essential: false,
      priority: None,
      description: String::new(),
      filename: format!("{}.pkg", name),
      size: 0,
      checksum: String::new(),
      rich_deps: vec![],
    }
  }

  fn index_with(provides_pairs: &[(&str, &[&str])]) -> Index {
    let packages = provides_pairs
      .iter()
      .map(|(pkg_name, provides)| pkg_with_provides(pkg_name, provides))
      .collect();
    Index::in_memory(packages)
  }

  /// An on-disk index whose companion path index holds the given
  /// file-ownership rows, so the path-index fallback has real data to
  /// consult.
  fn index_with_paths(
    provides_pairs: &[(&str, &[&str])],
    entries: &[(&str, &str, &str)],
  ) -> (tempfile::TempDir, Index) {
    let tmp = tempfile::tempdir().unwrap();
    let packages = provides_pairs
      .iter()
      .map(|(pkg_name, provides)| pkg_with_provides(pkg_name, provides))
      .collect();
    let index = Index::on_disk(&tmp.path().join("test.db"), packages);
    let mut builder = PathIndexBuilder::new();
    for (dir, fname, pkg) in entries {
      builder.entry_push(dir, fname, pkg);
    }
    builder.finalize(&tmp.path().join("test.pathidx")).unwrap();
    (tmp, index)
  }

  /// An `ElfDeps` with the given class and runpath — the two fields a
  /// resolve reads.
  fn deps(class: ElfClass, runpath: &[&str]) -> ElfDeps {
    ElfDeps {
      class,
      e_machine: 0,
      soname: None,
      needed: Vec::new(),
      runpath: runpath.iter().map(PathBuf::from).collect(),
    }
  }

  fn soname_deb<'a>(deb_arch: &str, index: &'a Index) -> Soname<'a> {
    Soname {
      dialect: SonameDialect::Deb {
        deb_arch: deb_arch.to_string(),
      },
      index,
    }
  }

  fn soname_of(dialect: SonameDialect, index: &Index) -> Soname<'_> {
    Soname { dialect, index }
  }

  #[test]
  fn soname_provider_deb_resolves_via_multiarch_lib_dir() {
    let (_tmp, index) = index_with_paths(&[], &[("/usr/lib/x86_64-linux-gnu", "libc.so.6", "libc6")]);
    let pkg = soname_deb("amd64", &index)
      .resolve("libc.so.6", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("libc6".to_string()));
  }

  #[test]
  fn soname_provider_deb_prefers_runpath_over_standard_dirs() {
    let (_tmp, index) = index_with_paths(
      &[],
      &[
        ("/opt/myapp/lib", "libfoo.so.1", "myapp"),
        ("/usr/lib/x86_64-linux-gnu", "libfoo.so.1", "system-foo"),
      ],
    );
    let pkg = soname_deb("amd64", &index)
      .resolve("libfoo.so.1", &deps(ElfClass::Elf64, &["/opt/myapp/lib"]))
      .unwrap();
    assert_eq!(pkg.as_deref(), Some("myapp"), "DT_RUNPATH should win over standard dirs");
  }

  #[test]
  fn soname_provider_deb_skips_origin_placeholder() {
    let (_tmp, index) = index_with_paths(&[], &[("/usr/lib/x86_64-linux-gnu", "libc.so.6", "libc6")]);
    let pkg = soname_deb("amd64", &index)
      .resolve("libc.so.6", &deps(ElfClass::Elf64, &["$ORIGIN/lib"]))
      .unwrap();
    assert_eq!(pkg, Some("libc6".to_string()));
  }

  #[test]
  fn soname_provider_deb_returns_none_when_unindexed() {
    let (_tmp, index) = index_with_paths(&[], &[("/usr/lib/x86_64-linux-gnu", "libc.so.6", "libc6")]);
    let pkg = soname_deb("amd64", &index)
      .resolve("libnonexistent.so.99", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert!(pkg.is_none());
  }

  #[test]
  fn soname_provider_deb_unsupported_arch_is_error() {
    let (_tmp, index) = index_with_paths(&[], &[]);
    let result = soname_deb("mips64", &index).resolve("libc.so.6", &deps(ElfClass::Elf64, &[]));
    assert!(result.is_err());
  }

  #[test]
  fn soname_provider_rpm_finds_plain_soname() {
    let index = index_with(&[("libc6", &["libc.so.6"])]);
    let pkg = soname_of(SonameDialect::Rpm, &index)
      .resolve("libc.so.6", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("libc6".to_string()));
  }

  #[test]
  fn soname_provider_rpm_finds_suffixed_64bit_soname() {
    let index = index_with(&[("libfoo", &["libfoo.so.6()(64bit)"])]);
    let pkg = soname_of(SonameDialect::Rpm, &index)
      .resolve("libfoo.so.6", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("libfoo".to_string()));
  }

  #[test]
  fn soname_provider_rpm_prefers_arch_suffixed_over_bare() {
    let index = index_with(&[("glibc-32bit", &["libc.so.6"]), ("glibc", &["libc.so.6()(64bit)"])]);
    let pkg = soname_of(SonameDialect::Rpm, &index)
      .resolve("libc.so.6", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("glibc".to_string()));
  }

  #[test]
  fn soname_provider_rpm_falls_back_to_path_index() {
    let (_tmp, index) = index_with_paths(&[], &[("/usr/lib64", "libfoo.so.6", "libfoo")]);
    let pkg = soname_of(SonameDialect::Rpm, &index)
      .resolve("libfoo.so.6", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("libfoo".to_string()));
  }

  #[test]
  fn soname_provider_pacman_finds_plain_soname() {
    let index = index_with(&[("libfoo", &["libfoo.so"])]);
    let pkg = soname_of(SonameDialect::Pacman, &index)
      .resolve("libfoo.so", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("libfoo".to_string()));
  }

  #[test]
  fn soname_provider_pacman_strips_major_suffix() {
    let index = index_with(&[("glibc", &["libc.so"])]);
    let pkg = soname_of(SonameDialect::Pacman, &index)
      .resolve("libc.so.6", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("glibc".to_string()));
  }

  #[test]
  fn soname_provider_pacman_falls_back_to_path_index() {
    let (_tmp, index) = index_with_paths(&[], &[("/usr/lib", "libc.so.6", "glibc")]);
    let pkg = soname_of(SonameDialect::Pacman, &index)
      .resolve("libc.so.6", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("glibc".to_string()));
  }

  #[test]
  fn soname_provider_pacman_returns_none_when_unprovided() {
    let index = index_with(&[("libfoo", &["libfoo.so"])]);
    let pkg = soname_of(SonameDialect::Pacman, &index)
      .resolve("libnonexistent.so", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert!(pkg.is_none());
  }

  #[test]
  fn soname_provider_apk_resolves_so_prefix() {
    let index = index_with(&[("musl", &["so:libc.musl-x86_64.so.1"])]);
    let pkg = soname_of(SonameDialect::Apk, &index)
      .resolve("libc.musl-x86_64.so.1", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert_eq!(pkg, Some("musl".to_string()));
  }

  #[test]
  fn soname_provider_apk_returns_none_for_plain_soname() {
    let index = index_with(&[("musl", &["libc.musl-x86_64.so.1"])]);
    let pkg = soname_of(SonameDialect::Apk, &index)
      .resolve("libc.musl-x86_64.so.1", &deps(ElfClass::Elf64, &[]))
      .unwrap();
    assert!(pkg.is_none());
  }
}
