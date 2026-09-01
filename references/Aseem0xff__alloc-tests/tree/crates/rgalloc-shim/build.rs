//! Emit the link directives for whichever allocator archive was built.
//!
//! Doing this here rather than through `RUSTFLAGS` matters: `RUSTFLAGS` applies
//! to every crate in the graph including build scripts and proc macros, which
//! on a musl host means the proc-macro `.so` also tries to link a static
//! allocator archive and fails in a way that reads as a Cargo bug. A build
//! script's directives apply to this crate's link only.

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=ALLOC_LIB_DIR");
    println!("cargo:rerun-if-env-changed=ALLOC_LIB_NAME");
    println!("cargo:rerun-if-env-changed=ALLOC_LINK_CXX");
    println!("cargo:rerun-if-env-changed=ALLOC_LINK_SEARCH");

    let Ok(dir) = env::var("ALLOC_LIB_DIR") else {
        // The `system` backend needs no archive. Any other feature without an
        // archive will fail at link with an undefined `mi_malloc` (or
        // equivalent), which is the correct outcome: a missing allocator must
        // break the build, never fall back to libc silently. That silent
        // fallback is upstream mimalloc-bench issue 245.
        return;
    };
    println!("cargo:rustc-link-search=native={}", dir);

    if let Ok(name) = env::var("ALLOC_LIB_NAME") {
        println!("cargo:rustc-link-lib=static={}", name);
    }

    // A C++ runtime archive lives under the compiler's own version directory,
    // which rustc does not search. The recipe that needs it says where.
    if let Ok(extra) = env::var("ALLOC_LINK_SEARCH") {
        for dir in extra.split(':').filter(|s| !s.is_empty()) {
            println!("cargo:rustc-link-search=native={}", dir);
        }
    }

    // snmalloc and Mesh are C++; their archives reference the C++ runtime.
    // Which runtime depends on the toolchain, so the caller names it.
    if let Ok(cxx) = env::var("ALLOC_LINK_CXX") {
        for lib in cxx.split(',').filter(|s| !s.is_empty()) {
            println!("cargo:rustc-link-lib={}", lib);
        }
    }
}
