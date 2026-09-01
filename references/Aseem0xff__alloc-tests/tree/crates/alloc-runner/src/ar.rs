//! A minimal `ar` archive reader.
//!
//! This exists for exactly one question, and it is the question the
//! libc-surgery technique lives or dies on:
//!
//!   after splicing an allocator's archive into `libc.a`, how many objects in
//!   that archive still define `malloc`?
//!
//! The surgery deletes musl's allocator objects by NAME (`malloc.lo`,
//! `free.lo`, ...). Those names are a property of the musl version that built
//! the archive. If a future musl renames or splits one, the `ar -M` `DELETE`
//! silently matches nothing, both definitions end up in the archive, and the
//! link picks whichever it reaches first. Nothing in the build fails, and the
//! run that follows measures an allocator nobody chose.
//!
//! So the surgery is not trusted; it is checked, by reading the archive.

use crate::elf;
use std::fs;
use std::path::Path;

pub struct Member {
    pub name: String,
    pub data_off: usize,
    pub size: usize,
}

/// Split a `!<arch>` archive into members, resolving GNU long names.
pub fn members(b: &[u8]) -> Result<Vec<Member>, String> {
    if b.len() < 8 || &b[0..8] != b"!<arch>\n" {
        return Err("not an ar archive".into());
    }
    // The GNU long-name table lives in a member literally called `//`.
    let mut longnames: Vec<u8> = Vec::new();
    let mut out = Vec::new();
    let mut pos = 8usize;

    while pos + 60 <= b.len() {
        let raw_name = String::from_utf8_lossy(&b[pos..pos + 16])
            .trim_end()
            .to_string();
        let size_s = String::from_utf8_lossy(&b[pos + 48..pos + 58])
            .trim()
            .to_string();
        if &b[pos + 58..pos + 60] != b"`\n" {
            return Err(format!("bad ar member header at offset {}", pos));
        }
        let size: usize = size_s
            .parse()
            .map_err(|_| format!("bad ar size {:?}", size_s))?;
        let data_off = pos + 60;
        if data_off + size > b.len() {
            return Err("truncated ar member".into());
        }

        if raw_name == "//" {
            longnames = b[data_off..data_off + size].to_vec();
        } else if raw_name == "/" || raw_name == "/SYM64/" {
            // symbol index, not an object
        } else {
            let name = if let Some(rest) = raw_name.strip_prefix('/') {
                // `/N` -> offset into the long-name table.
                match rest.parse::<usize>() {
                    Ok(off) => {
                        let end = longnames[off.min(longnames.len())..]
                            .iter()
                            .position(|&c| c == b'/' || c == b'\n')
                            .map(|p| off + p)
                            .unwrap_or(longnames.len());
                        String::from_utf8_lossy(&longnames[off.min(longnames.len())..end])
                            .into_owned()
                    }
                    Err(_) => raw_name.clone(),
                }
            } else {
                raw_name.trim_end_matches('/').to_string()
            };
            out.push(Member {
                name,
                data_off,
                size,
            });
        }

        pos = data_off + size + (size & 1); // members are 2-byte aligned
    }
    Ok(out)
}

pub struct Provider {
    pub member: String,
}

/// Every archive member that carries a global or weak *definition* of `symbol`.
///
/// More than one is the defect this module exists to catch. Zero, for a symbol
/// the target needs, is the other one.
pub fn definers(path: &Path, symbol: &str) -> Result<Vec<Provider>, String> {
    let b = fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let mut out = Vec::new();
    for m in members(&b)? {
        let obj = &b[m.data_off..m.data_off + m.size];
        // Members that are not ELF (a nested archive, a note) are skipped
        // rather than treated as empty: `parse_bytes` reports which.
        let Ok(e) = elf::parse_bytes(obj) else {
            continue;
        };
        for s in &e.syms {
            if s.name == symbol
                && s.defined
                && (s.bind == elf::STB_GLOBAL || s.bind == elf::STB_WEAK)
                && (s.kind == elf::STT_FUNC || s.kind == elf::STT_OBJECT)
            {
                out.push(Provider {
                    member: m.name.clone(),
                });
                break;
            }
        }
    }
    Ok(out)
}
