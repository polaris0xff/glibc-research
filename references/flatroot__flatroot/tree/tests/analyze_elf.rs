//! ELF-reader subsystem integration tests for the linker strategy of
//! `flatroot analyze trace`. The run-time-linkage analysis is grounded
//! entirely in [`flatroot::elf`] — `ElfDeps::read` extracts each
//! binary's `DT_NEEDED` sonames, its self-advertised `DT_SONAME`, its
//! ELF class, and its effective runtime search path (`DT_RUNPATH`
//! preferred over `DT_RPATH`); `ElfScan::scan` shortlists the files in
//! a package tree worth opening. These tests drive that public API
//! directly with hand-crafted ELF byte images so the section-based
//! dynamic-section parsing is exercised deterministically, without a
//! compiler or any host-specific binary.
//!
//! The byte images are built by [`ElfBuilder`], which emits a minimal
//! but real ELF: a 64-byte (or 52-byte, 32-bit) header, a `.dynamic`
//! section of `Elf{64,32}_Dyn` records, a `.dynstr` string table the
//! `.dynamic` section links to, and a `.shstrtab` for section names.
//! `flatroot::elf` resolves the dynamic section through the section
//! header table (`SHT_DYNAMIC`), exactly the path these images target.

#![allow(dead_code)]

use std::collections::BTreeMap;

use flatroot::elf::{ElfClass, ElfDeps, ElfScan};

// ELF / dynamic-section constants, mirrored from `object::elf`. These
// are the wire values the reader matches on; they are stable parts of
// the ELF ABI.
const SHT_STRTAB: u32 = 3;
const SHT_DYNAMIC: u32 = 6;

const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_SONAME: u64 = 14;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;

const EM_386: u16 = 3;
const EM_X86_64: u16 = 62;
const EM_AARCH64: u16 = 183;
const ET_DYN: u16 = 3;

/// Word size of the ELF image to emit. The reader reports it back as
/// [`ElfClass`]; the two layouts differ in header size, section-header
/// entry size, and dynamic-record width.
#[derive(Clone, Copy)]
enum Width {
  Elf32,
  Elf64,
}

/// Assembles a minimal, real ELF byte image with a controllable
/// dynamic section. Sonames, rpath, and runpath strings are interned
/// into a `.dynstr` table; dynamic records reference them by offset, so
/// the emitted image drives the same string-table lookup the reader
/// performs. A record may also be appended with a raw offset to force
/// an out-of-range string reference.
struct ElfBuilder {
  width: Width,
  machine: u16,
  with_dynamic: bool,
  strtab: Vec<u8>,
  offsets: BTreeMap<String, u32>,
  entries: Vec<(u64, u64)>,
}

impl ElfBuilder {
  fn new(width: Width, machine: u16) -> Self {
    // A string table always opens with a NUL byte so offset 0 is the
    // empty string.
    ElfBuilder {
      width,
      machine,
      with_dynamic: true,
      strtab: vec![0],
      offsets: BTreeMap::new(),
      entries: Vec::new(),
    }
  }

  /// Emit an image with no dynamic section at all — a statically linked
  /// binary as far as the reader is concerned.
  fn new_static(width: Width, machine: u16) -> Self {
    let mut b = Self::new(width, machine);
    b.with_dynamic = false;
    b
  }

  fn intern(&mut self, s: &str) -> u32 {
    if let Some(o) = self.offsets.get(s) {
      return *o;
    }
    let off = self.strtab.len() as u32;
    self.strtab.extend_from_slice(s.as_bytes());
    self.strtab.push(0);
    self.offsets.insert(s.to_string(), off);
    off
  }

  fn needed(&mut self, s: &str) -> &mut Self {
    let o = self.intern(s) as u64;
    self.entries.push((DT_NEEDED, o));
    self
  }

  fn soname(&mut self, s: &str) -> &mut Self {
    let o = self.intern(s) as u64;
    self.entries.push((DT_SONAME, o));
    self
  }

  fn rpath(&mut self, s: &str) -> &mut Self {
    let o = self.intern(s) as u64;
    self.entries.push((DT_RPATH, o));
    self
  }

  fn runpath(&mut self, s: &str) -> &mut Self {
    let o = self.intern(s) as u64;
    self.entries.push((DT_RUNPATH, o));
    self
  }

  /// Append a dynamic record with an explicit, possibly invalid, value
  /// — used to point a `DT_NEEDED` past the end of the string table.
  fn raw(&mut self, tag: u64, val: u64) -> &mut Self {
    self.entries.push((tag, val));
    self
  }

  fn is64(&self) -> bool {
    matches!(self.width, Width::Elf64)
  }

  fn build(&self) -> Vec<u8> {
    let is64 = self.is64();

    // Dynamic-section records, each followed by the mandatory DT_NULL
    // terminator.
    let mut dyn_bytes: Vec<u8> = Vec::new();
    for (tag, val) in &self.entries {
      if is64 {
        dyn_bytes.extend_from_slice(&tag.to_le_bytes());
        dyn_bytes.extend_from_slice(&val.to_le_bytes());
      } else {
        dyn_bytes.extend_from_slice(&(*tag as u32).to_le_bytes());
        dyn_bytes.extend_from_slice(&(*val as u32).to_le_bytes());
      }
    }
    if is64 {
      dyn_bytes.extend_from_slice(&DT_NULL.to_le_bytes());
      dyn_bytes.extend_from_slice(&0u64.to_le_bytes());
    } else {
      dyn_bytes.extend_from_slice(&(DT_NULL as u32).to_le_bytes());
      dyn_bytes.extend_from_slice(&0u32.to_le_bytes());
    }

    // Section-name string table.
    let mut shstr: Vec<u8> = vec![0];
    let mut shname = |s: &str, buf: &mut Vec<u8>| -> u32 {
      let o = buf.len() as u32;
      buf.extend_from_slice(s.as_bytes());
      buf.push(0);
      o
    };

    let ehsize: u64 = if is64 { 64 } else { 52 };
    let shentsize: u64 = if is64 { 64 } else { 40 };

    let mut cursor = ehsize;
    let (dyn_off, dynstr_off);
    if self.with_dynamic {
      dyn_off = cursor;
      cursor += dyn_bytes.len() as u64;
      dynstr_off = cursor;
      cursor += self.strtab.len() as u64;
    } else {
      dyn_off = 0;
      dynstr_off = 0;
    }
    let shstr_off = cursor;

    let name_dynamic = if self.with_dynamic {
      shname(".dynamic", &mut shstr)
    } else {
      0
    };
    let name_dynstr = if self.with_dynamic {
      shname(".dynstr", &mut shstr)
    } else {
      0
    };
    let name_shstrtab = shname(".shstrtab", &mut shstr);

    let shstr_size = shstr.len() as u64;
    let sh_off = shstr_off + shstr_size;

    let (shnum, shstrndx): (u16, u16) = if self.with_dynamic { (4, 3) } else { (2, 1) };

    let mut file: Vec<u8> = Vec::new();
    // e_ident
    file.extend_from_slice(b"\x7fELF");
    file.push(if is64 { 2 } else { 1 }); // EI_CLASS
    file.push(1); // EI_DATA = little-endian
    file.push(1); // EI_VERSION
    file.push(0); // EI_OSABI
    file.extend_from_slice(&[0u8; 8]); // EI_PAD up to 16 bytes

    file.extend_from_slice(&ET_DYN.to_le_bytes()); // e_type
    file.extend_from_slice(&self.machine.to_le_bytes()); // e_machine
    file.extend_from_slice(&1u32.to_le_bytes()); // e_version
    if is64 {
      file.extend_from_slice(&0u64.to_le_bytes()); // e_entry
      file.extend_from_slice(&0u64.to_le_bytes()); // e_phoff
      file.extend_from_slice(&sh_off.to_le_bytes()); // e_shoff
      file.extend_from_slice(&0u32.to_le_bytes()); // e_flags
      file.extend_from_slice(&(ehsize as u16).to_le_bytes()); // e_ehsize
      file.extend_from_slice(&0u16.to_le_bytes()); // e_phentsize
      file.extend_from_slice(&0u16.to_le_bytes()); // e_phnum
      file.extend_from_slice(&(shentsize as u16).to_le_bytes()); // e_shentsize
      file.extend_from_slice(&shnum.to_le_bytes()); // e_shnum
      file.extend_from_slice(&shstrndx.to_le_bytes()); // e_shstrndx
    } else {
      file.extend_from_slice(&0u32.to_le_bytes()); // e_entry
      file.extend_from_slice(&0u32.to_le_bytes()); // e_phoff
      file.extend_from_slice(&(sh_off as u32).to_le_bytes()); // e_shoff
      file.extend_from_slice(&0u32.to_le_bytes()); // e_flags
      file.extend_from_slice(&(ehsize as u16).to_le_bytes());
      file.extend_from_slice(&0u16.to_le_bytes());
      file.extend_from_slice(&0u16.to_le_bytes());
      file.extend_from_slice(&(shentsize as u16).to_le_bytes());
      file.extend_from_slice(&shnum.to_le_bytes());
      file.extend_from_slice(&shstrndx.to_le_bytes());
    }

    if self.with_dynamic {
      file.extend_from_slice(&dyn_bytes);
      file.extend_from_slice(&self.strtab);
    }
    file.extend_from_slice(&shstr);

    let push_sh = |file: &mut Vec<u8>, name: u32, typ: u32, off: u64, size: u64, link: u32, entsize: u64| {
      if is64 {
        file.extend_from_slice(&name.to_le_bytes());
        file.extend_from_slice(&typ.to_le_bytes());
        file.extend_from_slice(&0u64.to_le_bytes()); // sh_flags
        file.extend_from_slice(&0u64.to_le_bytes()); // sh_addr
        file.extend_from_slice(&off.to_le_bytes());
        file.extend_from_slice(&size.to_le_bytes());
        file.extend_from_slice(&link.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes()); // sh_info
        file.extend_from_slice(&1u64.to_le_bytes()); // sh_addralign
        file.extend_from_slice(&entsize.to_le_bytes());
      } else {
        file.extend_from_slice(&name.to_le_bytes());
        file.extend_from_slice(&typ.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&(off as u32).to_le_bytes());
        file.extend_from_slice(&(size as u32).to_le_bytes());
        file.extend_from_slice(&link.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&1u32.to_le_bytes());
        file.extend_from_slice(&(entsize as u32).to_le_bytes());
      }
    };

    // Section 0 is the mandatory null entry.
    push_sh(&mut file, 0, 0, 0, 0, 0, 0);
    if self.with_dynamic {
      let dyn_entsize: u64 = if is64 { 16 } else { 8 };
      // 1: .dynamic, linked to the .dynstr at section index 2.
      push_sh(&mut file, name_dynamic, SHT_DYNAMIC, dyn_off, dyn_bytes.len() as u64, 2, dyn_entsize);
      // 2: .dynstr
      push_sh(&mut file, name_dynstr, SHT_STRTAB, dynstr_off, self.strtab.len() as u64, 0, 0);
      // 3: .shstrtab
      push_sh(&mut file, name_shstrtab, SHT_STRTAB, shstr_off, shstr_size, 0, 0);
    } else {
      // 1: .shstrtab
      push_sh(&mut file, name_shstrtab, SHT_STRTAB, shstr_off, shstr_size, 0, 0);
    }

    file
  }

  /// Build the image and write it under `dir` at `name`, returning the
  /// path. The reader is path-based, so the bytes must land on disk.
  fn write(&self, dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, self.build()).unwrap();
    path
  }
}

// covers: ANL-041
#[test]
fn elf_reader_prefers_runpath_over_rpath_when_both_present() {
  // A binary declaring both DT_RUNPATH and DT_RPATH: the modern,
  // authoritative DT_RUNPATH must be the effective search path and the
  // legacy DT_RPATH must be ignored entirely. Asserted on both the
  // 64-bit and 32-bit layouts so the precedence holds across word size.
  let dir = tempfile::tempdir().unwrap();
  for (width, machine, name) in [(Width::Elf64, EM_X86_64, "x64.so"), (Width::Elf32, EM_386, "x32.so")] {
    let mut b = ElfBuilder::new(width, machine);
    b.needed("libc.so.6")
      .rpath("/legacy/rpath:/legacy/two")
      .runpath("/modern/runpath:/modern/two");
    let path = b.write(dir.path(), name);

    let deps = ElfDeps::read(&path).unwrap().expect("dynamic ELF must parse");
    let runpath: Vec<String> = deps.runpath.iter().map(|p| p.display().to_string()).collect();
    assert_eq!(
      runpath,
      vec!["/modern/runpath".to_string(), "/modern/two".to_string()],
      "DT_RUNPATH must win over DT_RPATH for {name}, got {runpath:?}"
    );
    assert!(
      !runpath.iter().any(|p| p.starts_with("/legacy")),
      "DT_RPATH entries must be ignored when DT_RUNPATH is present: {runpath:?}"
    );
  }
}

// covers: ANL-041
#[test]
fn elf_reader_falls_back_to_rpath_when_no_runpath() {
  // With only the legacy DT_RPATH present, it becomes the effective
  // search path — confirming the fallback leg of the same precedence
  // rule the previous test exercises from the other side.
  let dir = tempfile::tempdir().unwrap();
  let mut b = ElfBuilder::new(Width::Elf64, EM_X86_64);
  b.needed("libc.so.6").rpath("/only/rpath");
  let path = b.write(dir.path(), "rpath-only.so");

  let deps = ElfDeps::read(&path).unwrap().expect("dynamic ELF must parse");
  let runpath: Vec<String> = deps.runpath.iter().map(|p| p.display().to_string()).collect();
  assert_eq!(runpath, vec!["/only/rpath".to_string()], "legacy DT_RPATH must be used when DT_RUNPATH is absent");
}

// covers: ANL-042
#[test]
fn elf_reader_collects_needed_in_declaration_order() {
  // DT_NEEDED entries are emitted in a fixed order; the reader must
  // surface them in that same declaration order (it is a Vec, not a
  // set), because the linker walk follows them in sequence.
  let dir = tempfile::tempdir().unwrap();
  let mut b = ElfBuilder::new(Width::Elf64, EM_X86_64);
  b.needed("libc.so.6")
    .needed("libm.so.6")
    .needed("libpthread.so.0")
    .needed("libdl.so.2");
  let path = b.write(dir.path(), "ordered.so");

  let deps = ElfDeps::read(&path).unwrap().expect("dynamic ELF must parse");
  assert_eq!(
    deps.needed,
    vec![
      "libc.so.6".to_string(),
      "libm.so.6".to_string(),
      "libpthread.so.0".to_string(),
      "libdl.so.2".to_string(),
    ],
    "DT_NEEDED sonames must be collected in declaration order"
  );
}

// covers: ANL-043
#[test]
fn elf_reader_returns_soname_when_declared_and_none_when_absent() {
  let dir = tempfile::tempdir().unwrap();

  // A shared library advertises its own DT_SONAME.
  let mut lib = ElfBuilder::new(Width::Elf64, EM_X86_64);
  lib.soname("libfoo.so.1").needed("libc.so.6");
  let lib_path = lib.write(dir.path(), "libfoo.so.1.2.3");
  let lib_deps = ElfDeps::read(&lib_path).unwrap().expect("dynamic ELF must parse");
  assert_eq!(lib_deps.soname.as_deref(), Some("libfoo.so.1"), "DT_SONAME must be reported when declared");

  // An executable declares no DT_SONAME — only the libraries it needs.
  let mut exe = ElfBuilder::new(Width::Elf64, EM_X86_64);
  exe.needed("libc.so.6").needed("libtinfo.so.6");
  let exe_path = exe.write(dir.path(), "prog");
  let exe_deps = ElfDeps::read(&exe_path).unwrap().expect("dynamic ELF must parse");
  assert_eq!(exe_deps.soname, None, "an object without DT_SONAME must report soname=None");
  assert_eq!(exe_deps.needed.len(), 2, "the executable's two DT_NEEDED entries must still be read");
}

// covers: ANL-045
#[test]
fn elf_reader_errors_on_unknown_class_byte() {
  // EI_CLASS holds 1 (32-bit) or 2 (64-bit); any other value is a
  // corrupt header the reader must refuse rather than guess at.
  let dir = tempfile::tempdir().unwrap();
  let b = ElfBuilder::new_static(Width::Elf64, EM_X86_64);
  let mut bytes = b.build();
  bytes[4] = 7; // EI_CLASS = invalid
  let path = dir.path().join("badclass");
  std::fs::write(&path, &bytes).unwrap();

  let err = ElfDeps::read(&path).err().expect("expected an error");
  assert!(
    err.to_string().contains("Unknown ELF class 7"),
    "unknown EI_CLASS must be rejected with the class number, got: {err}"
  );
}

// covers: ANL-045
#[test]
fn elf_reader_errors_on_string_offset_out_of_range() {
  // A DT_NEEDED whose string offset points past the end of .dynstr is
  // a corrupt reference; resolving it is a hard error naming the kind
  // and offset, not a silently dropped dependency.
  let dir = tempfile::tempdir().unwrap();
  let mut b = ElfBuilder::new(Width::Elf64, EM_X86_64);
  // One valid entry, then a bogus offset well past the small .dynstr.
  b.needed("libc.so.6").raw(DT_NEEDED, 9999);
  let path = b.write(dir.path(), "badoffset.so");

  let err = ElfDeps::read(&path).err().expect("expected an error");
  let msg = err.to_string();
  assert!(
    msg.contains("string offset") && msg.contains("out of range"),
    "an out-of-range DT_NEEDED offset must be rejected, got: {msg}"
  );
}

// covers: ANL-046
#[test]
fn elf_reader_treats_no_dynamic_section_as_no_deps() {
  // A binary with no .dynamic section (statically linked) carries no
  // runtime-link facts: empty needed, empty runpath, no soname — but it
  // still parses (the class and machine are read from the header).
  let dir = tempfile::tempdir().unwrap();
  let b = ElfBuilder::new_static(Width::Elf64, EM_X86_64);
  let path = b.write(dir.path(), "static-bin");

  let deps = ElfDeps::read(&path)
    .unwrap()
    .expect("a static ELF still parses to ElfDeps");
  assert!(deps.needed.is_empty(), "static binary must report no DT_NEEDED: {:?}", deps.needed);
  assert!(deps.runpath.is_empty(), "static binary must report no runpath: {:?}", deps.runpath);
  assert_eq!(deps.soname, None, "static binary must report soname=None");
  assert!(matches!(deps.class, ElfClass::Elf64), "class must still be read from the header");
  assert_eq!(deps.e_machine, EM_X86_64, "e_machine must still be read from the header");
}

// covers: ANL-046
#[test]
fn elf_reader_returns_none_for_non_elf_bytes() {
  // The candidate shortlist admits files that may not be linkable
  // objects at all; a non-ELF candidate must be passed over (Ok(None))
  // rather than erroring the whole walk.
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("usr/bin/script");
  std::fs::create_dir_all(path.parent().unwrap()).unwrap();
  std::fs::write(&path, b"#!/bin/sh\necho hi\n").unwrap();
  assert!(ElfDeps::read(&path).unwrap().is_none(), "a shell script in a bin dir must read as a non-ELF (None)");
}

// covers: ANL-047
#[test]
fn elf_scan_selects_bin_and_so_prunes_metadata_and_skips_symlinks() {
  // ElfScan shortlists exactly the files worth opening: everything
  // under a bin dir, every `.so`/`.so.*` library anywhere, while
  // pruning whole metadata / pseudo-fs / docs subtrees, skipping
  // symlinks, and returning a sorted list. This drives the candidate
  // selection the linker walk depends on against a realistic package
  // tree.
  use std::os::unix::fs::symlink;

  let dir = tempfile::tempdir().unwrap();
  let root = dir.path();

  // Real linkable candidates.
  for d in [
    "bin",
    "sbin",
    "usr/bin",
    "usr/sbin",
    "usr/libexec",
    "usr/local/bin",
    "usr/local/sbin",
    "usr/lib",
  ] {
    std::fs::create_dir_all(root.join(d)).unwrap();
  }
  std::fs::write(root.join("usr/bin/cat"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("usr/sbin/init"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("usr/libexec/helper"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("usr/local/bin/tool"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("bin/sh"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("usr/lib/libfoo.so"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("usr/lib/libfoo.so.1.2.3"), b"\x7fELFstub").unwrap();

  // Pruned subtrees (metadata, host config, mutable state, pseudo-fs,
  // bundled docs/headers/sources) — none of their files may appear.
  for d in [
    ".flatroot/bin",
    "etc",
    "var/lib",
    "dev",
    "proc/1",
    "sys",
    "run",
    "tmp",
    "usr/share/doc",
    "usr/include",
    "usr/src",
  ] {
    std::fs::create_dir_all(root.join(d)).unwrap();
  }
  std::fs::write(root.join(".flatroot/bin/x"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("etc/ld.so"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("var/lib/v.so"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("proc/1/exe"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("usr/share/doc/lib.so"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("usr/include/foo.so"), b"\x7fELFstub").unwrap();
  std::fs::write(root.join("usr/src/bar.so"), b"\x7fELFstub").unwrap();

  // A data file in a non-pruned dir is not a candidate (not a bin dir,
  // not a .so).
  std::fs::write(root.join("usr/lib/data.conf"), b"text").unwrap();

  // A symlink to a real candidate must be skipped (counted once via its
  // true location only).
  symlink("libfoo.so.1.2.3", root.join("usr/lib/libfoo.so.1")).unwrap();

  let names: Vec<String> = ElfScan::scan(root)
    .unwrap()
    .iter()
    .map(|p| p.strip_prefix(root).unwrap().display().to_string())
    .collect();

  let expected = vec![
    "bin/sh".to_string(),
    "usr/bin/cat".to_string(),
    "usr/lib/libfoo.so".to_string(),
    "usr/lib/libfoo.so.1.2.3".to_string(),
    "usr/libexec/helper".to_string(),
    "usr/local/bin/tool".to_string(),
    "usr/sbin/init".to_string(),
  ];
  assert_eq!(names, expected, "ElfScan candidate set / pruning / symlink-skip / sort mismatch");
}
