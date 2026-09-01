//! Prepare a ripgrep checkout for one allocator configuration.
//!
//! -- ⛔ THE BASELINE TRAP ---------------------------------------------------
//!
//! ripgrep does not use the system allocator on musl. `crates/core/main.rs`
//! carries:
//!
//!     #[cfg(all(target_env = "musl", target_pointer_width = "64"))]
//!     #[global_allocator]
//!     static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
//!
//! (verified at tag 15.2.0, commit e89fff89ac9af12e8d4ce9d5fd07beb408ca730f).
//!
//! An unmodified `cargo build --target x86_64-unknown-linux-musl` therefore
//! produces a **jemalloc** binary. A benchmark that called that its "Alpine
//! system allocator baseline" would be comparing jemalloc against jemalloc and
//! reporting the difference as noise -- and every ratio in the report would be
//! wrong by whatever jemalloc beats musl by, which is the largest single effect
//! this project measures.
//!
//! So the first thing done to every checkout, for every cell including the
//! baseline, is to REMOVE that block. The baseline is then really musl's
//! allocator, and `identify` confirms it by finding musl's symbols and no
//! jemalloc.
//!
//! ⚠ This is a text transform over somebody else's source, so it asserts
//! afterwards rather than assuming: the count of `#[global_allocator]`
//! attributes left in the tree must be exactly 0 (baseline and replacement
//! modes) or exactly 1 (rust-global mode). A patch that silently matched
//! nothing is the failure this check exists to catch.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_sources(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name();
        let name = name.to_string_lossy();
        if p.is_dir() {
            // `target` is build output and `.git` is not source; walking them
            // would be slow and could find a vendored copy that is not built.
            if name == "target" || name == ".git" {
                continue;
            }
            rust_sources(&p, out);
        } else if name.ends_with(".rs") {
            out.push(p);
        }
    }
}

pub fn count_global_allocators(root: &Path) -> usize {
    let mut files = Vec::new();
    rust_sources(root, &mut files);
    let mut n = 0;
    for f in files {
        if let Ok(s) = fs::read_to_string(&f) {
            n += s
                .lines()
                .filter(|l| l.trim_start().starts_with("#[global_allocator]"))
                .count();
        }
    }
    n
}

/// Remove every `#[global_allocator]` item from one file.
///
/// The item removed is the attribute, any attributes contiguous with it above
/// (ripgrep's is preceded by a `#[cfg(...)]`), and the `static` declaration
/// below, up to and including the line that ends it.
fn strip_file(text: &str) -> (String, usize) {
    let lines: Vec<&str> = text.lines().collect();
    let mut drop = vec![false; lines.len()];
    let mut removed = 0;

    for i in 0..lines.len() {
        if lines[i].trim_start() != "#[global_allocator]" {
            continue;
        }
        removed += 1;
        drop[i] = true;
        // Attributes immediately above belong to the same item.
        let mut j = i;
        while j > 0 && lines[j - 1].trim_start().starts_with("#[") {
            j -= 1;
            drop[j] = true;
        }
        // The item itself: a `static NAME: T = V;` that may wrap lines.
        let mut k = i + 1;
        while k < lines.len() {
            drop[k] = true;
            if lines[k].trim_end().ends_with(';') {
                break;
            }
            k += 1;
        }
    }

    let kept: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop[*i])
        .map(|(_, l)| *l)
        .collect();
    let mut out = kept.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    (out, removed)
}

/// Delete a whole TOML table, from its header line to the next header.
fn strip_toml_table(text: &str, header_contains: &str) -> (String, bool) {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut hit = false;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with('[') && t.contains(header_contains) {
            hit = true;
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with('[') {
                i += 1;
            }
            continue;
        }
        out.push(lines[i]);
        i += 1;
    }
    let mut s = out.join("\n");
    if text.ends_with('\n') {
        s.push('\n');
    }
    (s, hit)
}

pub struct Report {
    pub stripped_items: usize,
    pub stripped_dep: bool,
    pub inserted: bool,
    pub final_count: usize,
}

/// `shim` is `Some((crate_path, feature))` for rust-global, `None` otherwise.
pub fn patch(root: &Path, shim: Option<(&str, &str)>) -> Result<Report, String> {
    // 1. Strip every existing #[global_allocator] from the whole tree.
    let mut files = Vec::new();
    rust_sources(root, &mut files);
    let mut stripped_items = 0;
    for f in &files {
        let Ok(s) = fs::read_to_string(f) else {
            continue;
        };
        if !s.contains("#[global_allocator]") {
            continue;
        }
        let (new, n) = strip_file(&s);
        fs::write(f, new).map_err(|e| format!("{}: {}", f.display(), e))?;
        stripped_items += n;
    }

    // 2. Drop the tikv-jemallocator dependency. Left in place it would still
    //    be compiled and linked, so the "baseline" binary would carry
    //    jemalloc's code even with the attribute gone, and `identify` would
    //    correctly refuse the cell.
    let cargo = root.join("Cargo.toml");
    let s = fs::read_to_string(&cargo).map_err(|e| format!("{}: {}", cargo.display(), e))?;
    let (s, stripped_dep) = strip_toml_table(&s, "tikv-jemallocator");

    // 3. For rust-global, add the shim and one attribute.
    let mut inserted = false;
    let s = if let Some((path, feature)) = shim {
        let dep = format!(
            "rgalloc-shim = {{ path = \"{}\", default-features = false, features = [\"{}\"] }}",
            path, feature
        );
        let mut out = Vec::new();
        let mut done = false;
        for line in s.lines() {
            out.push(line.to_string());
            if !done && line.trim() == "[dependencies]" {
                out.push(dep.clone());
                done = true;
            }
        }
        if !done {
            return Err("could not find a [dependencies] table in ripgrep's Cargo.toml".into());
        }
        out.join("\n") + "\n"
    } else {
        s
    };
    fs::write(&cargo, s).map_err(|e| format!("{}: {}", cargo.display(), e))?;

    if let Some((_, _)) = shim {
        let main = root.join("crates/core/main.rs");
        let text = fs::read_to_string(&main).map_err(|e| format!("{}: {}", main.display(), e))?;
        let lines: Vec<&str> = text.lines().collect();
        // Insert above `fn main`, skipping back over the doc comment and any
        // attributes attached to it: a `#[global_allocator]` wedged between a
        // doc comment and the item it documents does not compile.
        let Some(mut at) = lines
            .iter()
            .position(|l| l.trim_start().starts_with("fn main"))
        else {
            return Err("could not find `fn main` in crates/core/main.rs".into());
        };
        while at > 0 {
            let prev = lines[at - 1].trim_start();
            if prev.starts_with("///") || prev.starts_with("#[") || prev.starts_with("//!") {
                at -= 1;
            } else {
                break;
            }
        }
        let mut out: Vec<String> = lines[..at].iter().map(|s| s.to_string()).collect();
        out.push("#[global_allocator]".into());
        out.push("static ALLOC: rgalloc_shim::Alloc = rgalloc_shim::Alloc;".into());
        out.push(String::new());
        out.extend(lines[at..].iter().map(|s| s.to_string()));
        fs::write(&main, out.join("\n") + "\n")
            .map_err(|e| format!("{}: {}", main.display(), e))?;
        inserted = true;
    }

    // 4. Assert. A transform over third-party source that reports success
    //    without checking is how a "baseline" silently keeps its allocator.
    let final_count = count_global_allocators(root);
    let want = if shim.is_some() { 1 } else { 0 };
    if final_count != want {
        return Err(format!(
            "after patching, the tree has {} #[global_allocator] attribute(s), expected {}",
            final_count, want
        ));
    }

    Ok(Report {
        stripped_items,
        stripped_dep,
        inserted,
        final_count,
    })
}
