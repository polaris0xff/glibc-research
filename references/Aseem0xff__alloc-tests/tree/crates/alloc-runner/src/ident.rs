//! The allocator-identity oracle.
//!
//! -- THE DEFECT THIS EXISTS TO CATCH ----------------------------------------
//!
//! ⛔ A benchmark that cannot tell whether the allocator it names is actually
//! in the binary will publish the system allocator's numbers under seven
//! different names and rank them against each other. That is not hypothetical:
//! upstream mimalloc-bench issues 245 and 247 report exactly it — a missing
//! allocator library produced a full, plausible, green results table
//! (references/daanx__mimalloc-bench, commit
//! 3ad2732048312b0cc472b60302ff120f02ee9558, api/issues.json).
//!
//! ⭐ So identity is established by reading the ELF, before any timing happens.
//! Not from the build log, not from a flag the build was *asked* to use, and
//! not from the program's own report.
//!
//! ⚠ What this can and cannot prove. Finding an allocator's internal symbols
//! proves its code was linked in. It does not by itself prove every allocation
//! flows through it. The second half of that is the negative control below:
//! for a replacement build, the *displaced* allocator's implementation must be
//! ABSENT. Both together are strong; either alone is not.

use crate::elf::{Elf, LinkKind};
use crate::json::J;

pub struct Signature {
    pub id: &'static str,
    /// Symbols that must all be present. Chosen to be implementation
    /// internals rather than the public API where possible, because a public
    /// name can be an undefined reference in a binary that never linked the
    /// implementation.
    pub required: &'static [&'static str],
    /// At least one of these must be present.
    pub any_of: &'static [&'static str],
}

pub const SIGNATURES: &[Signature] = &[
    Signature {
        id: "mimalloc",
        required: &[],
        any_of: &[
            "mi_malloc",
            "mi_heap_malloc",
            "mi_option_get",
            "_mi_page_malloc",
            "mi_free",
        ],
    },
    Signature {
        id: "jemalloc",
        required: &[],
        any_of: &[
            "je_mallocx",
            "mallocx",
            "je_malloc_conf",
            "malloc_conf",
            "je_arena_boot",
            "je_sdallocx",
            "sdallocx",
        ],
    },
    Signature {
        id: "snmalloc",
        required: &[],
        any_of: &["sn_malloc", "sn_free", "snmalloc_alloc", "sn_realloc"],
    },
    Signature {
        id: "rpmalloc",
        required: &[],
        any_of: &[
            "rpmalloc",
            "rpmalloc_initialize",
            "rpfree",
            "rpaligned_alloc",
        ],
    },
    Signature {
        id: "hardened_malloc",
        required: &[],
        any_of: &["h_malloc", "h_free", "h_malloc_object_size", "h_realloc"],
    },
    Signature {
        id: "mesh",
        // Mesh is C++ and its symbols arrive mangled; `mesh` appears in the
        // mangled namespace component as `4mesh`.
        required: &[],
        any_of: &["_ZN4mesh11runtimeInstEv", "mesh_malloc"],
    },
    Signature {
        id: "tcmalloc",
        required: &[],
        any_of: &[
            "TCMallocInternalMalloc",
            "TCMallocInternalFree",
            "MallocExtension_Internal_GetNumericProperty",
        ],
    },
];

/// Symbols that identify the *libc's own* allocator implementation. Used as
/// the negative control for a replacement build: after libc surgery, none of
/// these may survive.
pub const LIBC_ALLOCATOR_SIGNATURES: &[Signature] = &[
    Signature {
        id: "musl",
        required: &[],
        // musl's mallocng internals. `__libc_malloc_impl` is musl's oldmalloc;
        // both are checked because Alpine has shipped each.
        any_of: &[
            "__libc_malloc_impl",
            "__malloc_donate",
            "alloc_meta",
            "get_meta",
            "nontrivial_free",
        ],
    },
    Signature {
        id: "glibc",
        required: &[],
        // ⛔ `__libc_malloc` IS NOT IN THIS LIST, AND THAT IS THE POINT.
        //
        // It looks like the obvious glibc marker and it is not glibc-exclusive:
        // several allocators define `__libc_malloc`, `__libc_free` and
        // `__libc_realloc` as compatibility aliases in their override layer.
        // mimalloc does. So a musl binary whose libc allocator had been
        // correctly and completely displaced BY mimalloc reported "still
        // contains the glibc allocator" -- a false positive that failed a cell
        // which had in fact worked. Observed here on 2026-09-01 on the first
        // libc-surgery run.
        //
        // ⭐ A negative control has to key on symbols only the displaced
        // implementation can have. These are glibc's private internals: no
        // allocator defines `_int_malloc` or `tcache_init` as a compatibility
        // alias, because nothing outside glibc calls them.
        any_of: &[
            "_int_malloc",
            "_int_free",
            "ptmalloc_init",
            "tcache_init",
            "sysmalloc",
            "arena_get2",
        ],
    },
];

pub fn detect(e: &Elf) -> Vec<&'static str> {
    let mut found = Vec::new();
    for s in SIGNATURES {
        let req = s.required.iter().all(|n| e.has_symbol(n));
        let any = s.any_of.is_empty() || s.any_of.iter().any(|n| e.has_symbol(n));
        if req && any {
            found.push(s.id);
        }
    }
    found
}

pub fn detect_libc_allocator(e: &Elf) -> Vec<&'static str> {
    let mut found = Vec::new();
    for s in LIBC_ALLOCATOR_SIGNATURES {
        if s.any_of.iter().any(|n| e.has_symbol(n)) {
            found.push(s.id);
        }
    }
    found
}

#[derive(Debug)]
pub struct Verdict {
    pub ok: bool,
    pub reasons: Vec<String>,
}

/// Decide whether `e` really is the configuration it claims to be.
///
/// `expect_allocator` is the allocator id (`system` for the baseline),
/// `expect_kind` the link kind the build profile asked for, and
/// `replacement` whether the integration mechanism claims to have *displaced*
/// the libc allocator (libc-surgery / link-override) rather than merely added
/// one alongside it (rust-global).
pub fn judge(
    e: &Elf,
    expect_allocator: &str,
    expect_kind: Option<&LinkKind>,
    replacement: bool,
) -> Verdict {
    let mut reasons = Vec::new();
    let found = detect(e);
    let libc_alloc = detect_libc_allocator(e);

    if let Some(k) = expect_kind {
        if &e.kind != k {
            reasons.push(format!(
                "link kind is {} but the profile asked for {}",
                e.kind.as_str(),
                k.as_str()
            ));
        }
    }

    // ⚠ A stripped binary has no .symtab, so an absence proves nothing. Saying
    // so is the difference between "the allocator is not there" and "this
    // instrument could not look". They are not the same and must not read the
    // same.
    if !e.had_symtab {
        reasons.push(
            "binary has no .symtab (stripped): symbol evidence is unavailable, so identity is UNPROVEN"
                .to_string(),
        );
        return Verdict { ok: false, reasons };
    }

    if expect_allocator == "system" {
        // The control. It must contain the libc allocator and none of the
        // candidates. A candidate here means the image leaked one in — which
        // is precisely what happens if libc surgery from another cell was not
        // isolated, and it would silently make the baseline fast.
        if !found.is_empty() {
            reasons.push(format!(
                "baseline binary contains candidate allocator(s): {}. The control is contaminated.",
                found.join(", ")
            ));
        }
        if libc_alloc.is_empty() {
            reasons.push(
                "baseline binary shows no libc allocator implementation; expected musl or glibc"
                    .to_string(),
            );
        }
    } else {
        if !found.contains(&expect_allocator) {
            reasons.push(format!(
                "no symbol evidence of {} in the binary (found: {})",
                expect_allocator,
                if found.is_empty() {
                    "none".to_string()
                } else {
                    found.join(", ")
                }
            ));
        }
        let extra: Vec<_> = found.iter().filter(|f| **f != expect_allocator).collect();
        if !extra.is_empty() {
            reasons.push(format!(
                "binary also contains other candidate allocator(s): {:?}. Two allocators in one binary is not the configuration under test.",
                extra
            ));
        }
        if replacement && !libc_alloc.is_empty() {
            // The negative control for a replacement build.
            reasons.push(format!(
                "replacement build still contains the {} allocator implementation: the displaced allocator was not removed, so which one serves malloc is decided by link order rather than by this configuration",
                libc_alloc.join(", ")
            ));
        }
    }

    Verdict {
        ok: reasons.is_empty(),
        reasons,
    }
}

pub fn report_json(e: &Elf, verdict: &Verdict) -> J {
    J::obj(vec![
        ("machine", J::s(crate::elf::machine_name(e.machine))),
        ("link_kind", J::s(e.kind.as_str())),
        (
            "interp",
            match &e.interp {
                Some(i) => J::s(i.clone()),
                None => J::Null,
            },
        ),
        ("has_symtab", J::Bool(e.had_symtab)),
        ("symbols_total", J::U(e.syms.len() as u64)),
        (
            "allocators_detected",
            J::arr(detect(e).into_iter().map(J::s).collect()),
        ),
        (
            "libc_allocator_detected",
            J::arr(detect_libc_allocator(e).into_iter().map(J::s).collect()),
        ),
        ("ok", J::Bool(verdict.ok)),
        (
            "reasons",
            J::arr(verdict.reasons.iter().cloned().map(J::S).collect()),
        ),
    ])
}
