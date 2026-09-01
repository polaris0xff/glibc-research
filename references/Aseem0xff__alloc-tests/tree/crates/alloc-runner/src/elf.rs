//! A minimal ELF64 reader, used as an ORACLE.
//!
//! Why this exists rather than shelling out to `nm` and `readelf`: the answer
//! to "which allocator is actually in this binary, and is it really a static
//! PIE" has to come from reading the artefact, not from the build log that
//! claims to describe it, and not from the program's own report. binutils is
//! not present in every image this runs in (Alpine's base has neither), and
//! parsing `readelf` prose across versions is its own defect surface.
//!
//! Both supported architectures are 64-bit little-endian, so that is all this
//! parses. A file that is not ELF64-LE is reported as such rather than guessed
//! at.

use std::fs;
use std::path::Path;

const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;

const PT_INTERP: u32 = 3;

const SHT_SYMTAB: u32 = 2;
const SHT_DYNSYM: u32 = 11;

const SHN_UNDEF: u16 = 0;

pub const STB_GLOBAL: u8 = 1;
pub const STB_WEAK: u8 = 2;

pub const STT_FUNC: u8 = 2;
pub const STT_OBJECT: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    /// ET_EXEC, no PT_INTERP: a classic non-relocatable static binary.
    Static,
    /// ET_DYN, no PT_INTERP: a static PIE. This is the combination that gets
    /// misreported most often, because `file` calls it "shared object".
    StaticPie,
    /// Has PT_INTERP: needs an interpreter at run time.
    Dynamic,
    /// ET_REL and anything else.
    Other,
}

impl LinkKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkKind::Static => "static",
            LinkKind::StaticPie => "static-pie",
            LinkKind::Dynamic => "dynamic",
            LinkKind::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sym {
    pub name: String,
    /// `STB_LOCAL`/`GLOBAL`/`WEAK`. Read by `ar::definers`, which only counts a
    /// member as a provider when the definition is externally visible.
    pub bind: u8,
    pub kind: u8,
    pub defined: bool,
}

#[derive(Debug)]
pub struct Elf {
    pub machine: u16,
    pub kind: LinkKind,
    pub interp: Option<String>,
    /// Every symbol from `.symtab` if present, else `.dynsym`.
    pub syms: Vec<Sym>,
    pub had_symtab: bool,
}

pub fn machine_name(m: u16) -> &'static str {
    match m {
        0x3e => "x86_64",
        0xb7 => "aarch64",
        _ => "unknown",
    }
}

fn u16le(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(off..off + 2)?.try_into().ok()?))
}
fn u32le(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(off..off + 4)?.try_into().ok()?))
}
fn u64le(b: &[u8], off: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(off..off + 8)?.try_into().ok()?))
}

fn cstr(b: &[u8], off: usize) -> String {
    let end = b[off.min(b.len())..]
        .iter()
        .position(|&c| c == 0)
        .map(|p| off + p)
        .unwrap_or(b.len());
    String::from_utf8_lossy(&b[off.min(b.len())..end]).into_owned()
}

pub fn parse_bytes(b: &[u8]) -> Result<Elf, String> {
    if b.len() < 64 || &b[0..4] != b"\x7fELF" {
        return Err("not an ELF file".into());
    }
    if b[4] != 2 {
        return Err("not ELF64".into());
    }
    if b[5] != 1 {
        return Err("not little-endian".into());
    }

    let e_type = u16le(b, 16).ok_or("truncated header")?;
    let machine = u16le(b, 18).ok_or("truncated header")?;
    let e_phoff = u64le(b, 32).ok_or("truncated header")? as usize;
    let e_shoff = u64le(b, 40).ok_or("truncated header")? as usize;
    let e_phentsize = u16le(b, 54).ok_or("truncated header")? as usize;
    let e_phnum = u16le(b, 56).ok_or("truncated header")? as usize;
    let e_shentsize = u16le(b, 58).ok_or("truncated header")? as usize;
    let e_shnum = u16le(b, 60).ok_or("truncated header")? as usize;

    // PT_INTERP is what separates a static PIE from a dynamic executable.
    // Both are ET_DYN, so e_type alone cannot answer it.
    let mut interp = None;
    for i in 0..e_phnum {
        let off = e_phoff + i * e_phentsize;
        let Some(p_type) = u32le(b, off) else { break };
        if p_type == PT_INTERP {
            let p_offset = u64le(b, off + 8).unwrap_or(0) as usize;
            interp = Some(cstr(b, p_offset));
        }
    }

    let kind = match (e_type, interp.is_some()) {
        (_, true) => LinkKind::Dynamic,
        (ET_EXEC, false) => LinkKind::Static,
        (ET_DYN, false) => LinkKind::StaticPie,
        _ => LinkKind::Other,
    };

    // Prefer .symtab: it carries local symbols, which is what tells us whether
    // an allocator's internal objects were really linked in, as opposed to
    // just its exported names being referenced.
    let mut best: Option<(usize, usize, usize, usize)> = None; // (off, size, entsize, strtab_idx)
    let mut fallback: Option<(usize, usize, usize, usize)> = None;
    let mut sections = Vec::new();
    for i in 0..e_shnum {
        let off = e_shoff + i * e_shentsize;
        let Some(sh_type) = u32le(b, off + 4) else {
            break;
        };
        let sh_offset = u64le(b, off + 24).unwrap_or(0) as usize;
        let sh_size = u64le(b, off + 32).unwrap_or(0) as usize;
        let sh_link = u32le(b, off + 40).unwrap_or(0) as usize;
        let sh_entsize = u64le(b, off + 56).unwrap_or(0) as usize;
        sections.push((sh_offset, sh_size));
        if sh_type == SHT_SYMTAB && sh_entsize >= 24 {
            best = Some((sh_offset, sh_size, sh_entsize, sh_link));
        } else if sh_type == SHT_DYNSYM && sh_entsize >= 24 && fallback.is_none() {
            fallback = Some((sh_offset, sh_size, sh_entsize, sh_link));
        }
    }

    let had_symtab = best.is_some();
    let mut syms = Vec::new();
    if let Some((off, size, entsize, strtab_idx)) = best.or(fallback) {
        let (str_off, str_size) = sections.get(strtab_idx).copied().unwrap_or((0, 0));
        let strtab = b.get(str_off..str_off + str_size).unwrap_or(&[]);
        let n = size / entsize;
        for i in 0..n {
            let so = off + i * entsize;
            let Some(st_name) = u32le(b, so) else { break };
            let st_info = *b.get(so + 4).unwrap_or(&0);
            let st_shndx = u16le(b, so + 6).unwrap_or(0);
            let name = if st_name == 0 {
                String::new()
            } else {
                cstr(strtab, st_name as usize)
            };
            if name.is_empty() {
                continue;
            }
            syms.push(Sym {
                name,
                bind: st_info >> 4,
                kind: st_info & 0xf,
                defined: st_shndx != SHN_UNDEF,
            });
        }
    }

    Ok(Elf {
        machine,
        kind,
        interp,
        syms,
        had_symtab,
    })
}

pub fn parse(path: &Path) -> Result<Elf, String> {
    let b = fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    parse_bytes(&b)
}

impl Elf {
    /// Any symbol at all, including local ones. Local symbols are the strong
    /// evidence that an allocator's *implementation* is present, because an
    /// allocator's internal helpers are usually static.
    pub fn has_symbol(&self, name: &str) -> bool {
        self.syms.iter().any(|s| s.name == name)
    }
}
