//! Reads an ELF binary's dynamic-linking data: the shared libraries it
//! needs, its ELF class, and where it searches for those libraries.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use object::Endianness;
use object::elf;
use object::read::elf::{Dyn, FileHeader};

/// A binary's ELF class (word size), which decides the header layout
/// and which library directories apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
  /// 32-bit ELF.
  Elf32,
  /// 64-bit ELF.
  Elf64,
}

/// One binary's dynamic-linking data, read once.
#[derive(Debug, Clone)]
pub struct ElfDeps {
  /// Word size from the binary's header.
  pub class: ElfClass,
  /// Numeric architecture id from the binary's header.
  pub e_machine: u16,
  /// The soname a shared library announces itself by; `None` for
  /// executables and objects that declare none.
  pub soname: Option<String>,
  /// Short names of the shared libraries the binary expects at runtime,
  /// in declaration order, e.g. `libc.so.6`.
  pub needed: Vec<String>,
  /// Runtime search directories: `DT_RUNPATH` when the binary declares
  /// it, else `DT_RPATH`, split per directory with empties dropped.
  pub runpath: Vec<PathBuf>,
}

impl ElfDeps {
  /// Reads a file's dynamic-linking data; `None` for a file that is not
  /// an ELF object.
  pub fn read(path_file_elf: &Path) -> Result<Option<ElfDeps>> {
    let data = std::fs::read(path_file_elf)
      .with_context(|| format!("Failed to read ELF candidate {}", path_file_elf.display()))?;

    if data.len() < 5 || &data[0..4] != b"\x7fELF" {
      return Ok(None);
    }

    match data[4] {
      1 => Ok(Some(Self::decode::<elf::FileHeader32<Endianness>>(&data, ElfClass::Elf32, path_file_elf)?)),
      2 => Ok(Some(Self::decode::<elf::FileHeader64<Endianness>>(&data, ElfClass::Elf64, path_file_elf)?)),
      other => bail!("Unknown ELF class {} in {}", other, path_file_elf.display()),
    }
  }

  /// Extracts the dynamic-linking data, shared across both ELF classes
  /// with `class` passed in. Out-of-range string offsets are skipped
  /// rather than truncated, and `DT_RUNPATH` is used instead of the
  /// legacy `DT_RPATH` when both are present.
  fn decode<H: FileHeader<Endian = Endianness>>(data: &[u8], class: ElfClass, path_file_elf: &Path) -> Result<ElfDeps> {
    let header =
      H::parse(data).with_context(|| format!("Failed to parse ELF header in {}", path_file_elf.display()))?;
    let endian = header
      .endian()
      .with_context(|| format!("Unrecognised endianness in {}", path_file_elf.display()))?;
    let e_machine = header.e_machine(endian);

    let sections = header
      .sections(endian, data)
      .with_context(|| format!("Failed to read sections in {}", path_file_elf.display()))?;

    let (dynamic, str_idx) = match sections
      .dynamic(endian, data)
      .with_context(|| format!("Failed to read dynamic section in {}", path_file_elf.display()))?
    {
      Some(pair) => pair,
      None => {
        return Ok(ElfDeps {
          class,
          e_machine,
          soname: None,
          needed: Vec::new(),
          runpath: Vec::new(),
        });
      }
    };

    let strings = sections
      .strings(endian, data, str_idx)
      .with_context(|| format!("Failed to read dynamic string table in {}", path_file_elf.display()))?;

    let mut needed: Vec<String> = Vec::new();
    let mut soname: Option<String> = None;
    let mut runpath_entries: Vec<String> = Vec::new();
    let mut rpath_entries: Vec<String> = Vec::new();

    for entry in dynamic {
      let tag = entry.d_tag(endian).into() as u32;
      let val: u64 = entry.d_val(endian).into();
      if val > u32::MAX as u64 {
        // Dynamic-string-table offsets are u32; a value beyond that
        // cannot be a valid offset, so skip it rather than truncate.
        continue;
      }
      let off = val as u32;

      match tag {
        elf::DT_NEEDED => needed.push(Self::name_at(strings, off, "DT_NEEDED", path_file_elf)?),
        elf::DT_SONAME => soname = Some(Self::name_at(strings, off, "DT_SONAME", path_file_elf)?),
        elf::DT_RPATH => {
          let raw = Self::name_at(strings, off, "DT_RPATH", path_file_elf)?;
          rpath_entries.extend(raw.split(':').map(|s| s.to_string()));
        }
        elf::DT_RUNPATH => {
          let raw = Self::name_at(strings, off, "DT_RUNPATH", path_file_elf)?;
          runpath_entries.extend(raw.split(':').map(|s| s.to_string()));
        }
        _ => {}
      }
    }

    let chosen = if !runpath_entries.is_empty() {
      runpath_entries
    } else {
      rpath_entries
    };

    Ok(ElfDeps {
      class,
      e_machine,
      soname,
      needed,
      runpath: chosen
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect(),
    })
  }

  /// Resolves one name from the dynamic string table. An offset pointing
  /// out of range is treated as a corrupt binary, since a wrong name here
  /// would misdirect every dependency decision that follows.
  fn name_at(strings: object::read::StringTable<'_>, offset: u32, kind: &str, path_file_elf: &Path) -> Result<String> {
    let bytes = strings
      .get(offset)
      .map_err(|()| anyhow::anyhow!("{} string offset {} out of range in {}", kind, offset, path_file_elf.display()))?;
    Ok(String::from_utf8_lossy(bytes).into_owned())
  }
}

/// Picks which files in a package tree are worth opening as ELF
/// candidates: where programs live, what names mark a shared library,
/// and which directories to skip.
pub struct ElfScan;

impl ElfScan {
  /// Narrows a package tree to just the programs and shared libraries
  /// worth reading for link dependencies, sorted.
  pub fn scan(path_dir_root: &Path) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    Self::collect(path_dir_root, path_dir_root, &mut out)
      .with_context(|| format!("Failed to walk {}", path_dir_root.display()))?;
    out.sort();
    Ok(out)
  }

  /// Descends the tree collecting candidate binaries: pruned areas are
  /// skipped, and symlinks are not followed, since they would escape the
  /// tree or double-count a file already reached by its real location.
  fn collect(path_dir: &Path, path_dir_root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match std::fs::read_dir(path_dir) {
      Ok(it) => it,
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
      Err(e) => return Err(e).with_context(|| format!("read_dir({})", path_dir.display())),
    };

    for entry in entries {
      let entry = entry.with_context(|| format!("read_dir entry under {}", path_dir.display()))?;
      let path = entry.path();
      let file_type = entry
        .file_type()
        .with_context(|| format!("file_type({})", path.display()))?;

      if file_type.is_symlink() {
        continue;
      }

      let rel = path.strip_prefix(path_dir_root).unwrap_or(&path);
      if Self::dir_pruned(rel) {
        continue;
      }

      if file_type.is_dir() {
        Self::collect(&path, path_dir_root, out)?;
      } else if file_type.is_file() && Self::is_candidate(rel) {
        out.push(path);
      }
    }

    Ok(())
  }

  /// Recognises tree regions that hold nothing link-relevant — build
  /// metadata, config, mutable state, pseudo-filesystems, docs, headers —
  /// so the walk steps past them.
  fn dir_pruned(rel: &Path) -> bool {
    let s = rel.to_string_lossy();
    let prefixes = [
      ".flatroot",
      "etc",
      "var",
      "dev",
      "proc",
      "sys",
      "run",
      "tmp",
      "usr/share",
      "usr/include",
      "usr/src",
    ];
    for p in &prefixes {
      if s == *p || s.starts_with(&format!("{}/", p)) {
        return true;
      }
    }
    false
  }

  /// Whether one file is worth inspecting: it lives in an executable
  /// directory or is named like a shared library (`.so`, `.so.`).
  fn is_candidate(rel: &Path) -> bool {
    let s = rel.to_string_lossy();

    let bin_dirs = [
      "bin/",
      "sbin/",
      "usr/bin/",
      "usr/sbin/",
      "usr/libexec/",
      "usr/local/bin/",
      "usr/local/sbin/",
    ];
    if bin_dirs.iter().any(|d| s.starts_with(d)) {
      return true;
    }

    let basename = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if basename.ends_with(".so") {
      return true;
    }
    if basename.contains(".so.") {
      return true;
    }

    false
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn elf_inspect_returns_none_on_non_elf() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notelf");
    std::fs::write(&path, b"this is not an ELF file").unwrap();
    assert!(ElfDeps::read(&path).unwrap().is_none());
  }

  #[test]
  fn elf_inspect_returns_none_on_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty");
    std::fs::write(&path, b"").unwrap();
    assert!(ElfDeps::read(&path).unwrap().is_none());
  }

  #[test]
  fn elf_inspect_returns_none_on_short_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("short");
    std::fs::write(&path, b"\x7fEL").unwrap();
    assert!(ElfDeps::read(&path).unwrap().is_none());
  }

  #[test]
  fn elf_inspect_handles_real_host_binary() {
    // /bin/sh is mandatory on every supported host (Linux FHS).
    // We accept either a 32-bit or 64-bit ELF and assert the parse
    // produces a valid ElfDeps shape.
    let path = Path::new("/bin/sh");
    if !path.exists() {
      return;
    }
    // /bin/sh can be a symlink to dash or bash; resolve through the
    // OS so we read the underlying ELF.
    let resolved = std::fs::canonicalize(path).unwrap();
    let deps = ElfDeps::read(&resolved).unwrap().expect("/bin/sh must be ELF");
    assert!(matches!(deps.class, ElfClass::Elf32 | ElfClass::Elf64));
    assert_ne!(deps.e_machine, 0, "e_machine must be set");
  }

  #[test]
  fn elf_walk_includes_bin_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("usr/bin")).unwrap();
    std::fs::create_dir_all(dir.path().join("usr/lib")).unwrap();
    std::fs::create_dir_all(dir.path().join("usr/share/doc")).unwrap();
    std::fs::write(dir.path().join("usr/bin/cat"), b"\x7fELFstub").unwrap();
    std::fs::write(dir.path().join("usr/lib/libfoo.so.1.2.3"), b"\x7fELFstub").unwrap();
    std::fs::write(dir.path().join("usr/lib/libfoo.so"), b"\x7fELFstub").unwrap();
    std::fs::write(dir.path().join("usr/share/doc/README"), b"text").unwrap();

    let out = ElfScan::scan(dir.path()).unwrap();
    let names: Vec<String> = out
      .iter()
      .map(|p| p.strip_prefix(dir.path()).unwrap().display().to_string())
      .collect();

    assert!(names.contains(&"usr/bin/cat".to_string()), "missing cat: {names:?}");
    assert!(names.contains(&"usr/lib/libfoo.so".to_string()), "missing libfoo.so: {names:?}");
    assert!(names.contains(&"usr/lib/libfoo.so.1.2.3".to_string()), "missing libfoo.so.1.2.3: {names:?}");
    assert!(!names.iter().any(|n| n == "usr/share/doc/README"), "leaked text file: {names:?}");
  }

  #[test]
  fn elf_walk_prunes_metadata_and_pseudofs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".flatroot/bin")).unwrap();
    std::fs::create_dir_all(dir.path().join("proc/1")).unwrap();
    std::fs::create_dir_all(dir.path().join("sys")).unwrap();
    std::fs::create_dir_all(dir.path().join("etc")).unwrap();
    std::fs::create_dir_all(dir.path().join("usr/bin")).unwrap();
    std::fs::write(dir.path().join(".flatroot/bin/stub"), b"\x7fELFstub").unwrap();
    std::fs::write(dir.path().join("proc/1/exe"), b"\x7fELFstub").unwrap();
    std::fs::write(dir.path().join("sys/kernel"), b"\x7fELFstub").unwrap();
    std::fs::write(dir.path().join("etc/passwd"), b"text").unwrap();
    std::fs::write(dir.path().join("usr/bin/cat"), b"\x7fELFstub").unwrap();

    let out = ElfScan::scan(dir.path()).unwrap();
    let names: Vec<String> = out
      .iter()
      .map(|p| p.strip_prefix(dir.path()).unwrap().display().to_string())
      .collect();

    assert_eq!(names, vec!["usr/bin/cat".to_string()], "pruned dirs leaked into walk: {names:?}");
  }

  #[test]
  fn elf_walk_skips_symlinks() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("usr/lib")).unwrap();
    std::fs::write(dir.path().join("usr/lib/libfoo.so.1"), b"\x7fELFstub").unwrap();
    symlink("libfoo.so.1", dir.path().join("usr/lib/libfoo.so")).unwrap();

    let out = ElfScan::scan(dir.path()).unwrap();
    let names: Vec<String> = out
      .iter()
      .map(|p| p.strip_prefix(dir.path()).unwrap().display().to_string())
      .collect();

    assert_eq!(names, vec!["usr/lib/libfoo.so.1".to_string()], "symlink leaked: {names:?}");
  }
}
