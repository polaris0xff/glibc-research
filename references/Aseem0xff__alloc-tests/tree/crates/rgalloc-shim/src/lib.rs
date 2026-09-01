//! A `GlobalAlloc` backed by one allocator's prefixed C API.
//!
//! This is the `rust-global` integration mechanism: the allocator is *added to
//! the application*, and libc's `malloc` is left exactly where it was. It is
//! not the same experiment as replacing the distribution's allocator, and the
//! project reports the two separately. See docs/allocator-integration.md.
//!
//! ⭐ Why prefixed symbols. Binding to `mi_malloc` rather than to `malloc`
//! means the link fails loudly when the archive is missing, instead of quietly
//! resolving to libc and producing a full set of numbers for the wrong
//! allocator.
//!
//! -- ALIGNMENT, WHICH IS WHERE THIS KIND OF SHIM GOES WRONG -----------------
//!
//! Rust's `GlobalAlloc` passes an alignment with every call, including on
//! `dealloc`, and permits alignments larger than `malloc`'s guarantee. Three
//! rules are followed here, and each one is a real bug in shims that skip it:
//!
//!   1. Over-aligned requests never go to plain `malloc`.
//!   2. `dealloc` uses the *same family* of function the pointer came from.
//!      An `aligned_alloc` pointer freed with a mismatched free is undefined
//!      behaviour on some of these allocators.
//!   3. `realloc` on an over-aligned block is done by hand — allocate, copy
//!      `min(old, new)`, free — because the C `realloc` family does not
//!      preserve an alignment stronger than `max_align_t`.

#![deny(unsafe_op_in_unsafe_fn)]

use core::alloc::{GlobalAlloc, Layout};

/// The alignment `malloc` already guarantees on every 64-bit target this
/// project builds for. At or below it, the plain entry point is correct and
/// avoids the aligned path's extra work.
const MIN_ALIGN: usize = 16;

/// Copy the overlap and free the old block. Used for every over-aligned
/// `realloc`, where the C `realloc` cannot be trusted to keep the alignment.
///
/// # Safety
/// `ptr` must be a live allocation described by `layout`.
unsafe fn realloc_by_hand<A: RawAlloc>(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    let new_ptr = unsafe { A::alloc(new_size, layout.align()) };
    if new_ptr.is_null() {
        // On failure the original block must survive: `GlobalAlloc::realloc`
        // requires that the old pointer stays valid when it returns null.
        return core::ptr::null_mut();
    }
    let n = core::cmp::min(layout.size(), new_size);
    unsafe {
        core::ptr::copy_nonoverlapping(ptr, new_ptr, n);
        A::dealloc(ptr, layout.size(), layout.align());
    }
    new_ptr
}

/// The one thing each backend has to provide.
trait RawAlloc {
    /// # Safety
    /// `align` must be a power of two.
    unsafe fn alloc(size: usize, align: usize) -> *mut u8;
    /// # Safety
    /// `size`/`align` must describe the live allocation at `ptr`.
    unsafe fn dealloc(ptr: *mut u8, size: usize, align: usize);
    /// # Safety
    /// As `dealloc`, and the result follows `GlobalAlloc::realloc`'s contract.
    unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8;
    /// # Safety
    /// As `alloc`. Must return zeroed memory.
    unsafe fn zalloc(size: usize, align: usize) -> *mut u8;
    /// Called once per process before any allocation. Only rpmalloc needs it.
    fn init_process() {}
    /// Called on each thread before its first allocation.
    ///
    /// ⚠ Only rpmalloc needs this, and rpmalloc calls it from inside its own
    /// `alloc`/`dealloc` rather than through the trait, because there is no
    /// hook Rust runs on every thread start. It stays on the trait as the
    /// documented place that requirement lives; `allow(dead_code)` records that
    /// the default is deliberately unreachable rather than forgotten.
    #[allow(dead_code)]
    #[inline(always)]
    fn init_thread() {}
    const NAME: &'static str;
}

// ===========================================================================
// mimalloc
// ===========================================================================
#[cfg(feature = "mimalloc")]
mod backend {
    use super::*;
    extern "C" {
        fn mi_malloc(size: usize) -> *mut u8;
        fn mi_zalloc(size: usize) -> *mut u8;
        fn mi_realloc(p: *mut u8, newsize: usize) -> *mut u8;
        fn mi_free(p: *mut u8);
        fn mi_malloc_aligned(size: usize, alignment: usize) -> *mut u8;
        fn mi_zalloc_aligned(size: usize, alignment: usize) -> *mut u8;
        fn mi_realloc_aligned(p: *mut u8, newsize: usize, alignment: usize) -> *mut u8;
    }
    pub struct B;
    impl RawAlloc for B {
        const NAME: &'static str = "mimalloc";
        #[inline]
        unsafe fn alloc(size: usize, align: usize) -> *mut u8 {
            if align <= MIN_ALIGN {
                unsafe { mi_malloc(size) }
            } else {
                unsafe { mi_malloc_aligned(size, align) }
            }
        }
        #[inline]
        unsafe fn dealloc(ptr: *mut u8, _size: usize, _align: usize) {
            // mimalloc's free accepts pointers from every mi_* entry point.
            unsafe { mi_free(ptr) }
        }
        #[inline]
        unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if layout.align() <= MIN_ALIGN {
                unsafe { mi_realloc(ptr, new_size) }
            } else {
                // mimalloc has a real aligned realloc, so no hand copy needed.
                unsafe { mi_realloc_aligned(ptr, new_size, layout.align()) }
            }
        }
        #[inline]
        unsafe fn zalloc(size: usize, align: usize) -> *mut u8 {
            if align <= MIN_ALIGN {
                unsafe { mi_zalloc(size) }
            } else {
                unsafe { mi_zalloc_aligned(size, align) }
            }
        }
    }
}

// ===========================================================================
// jemalloc, built with --with-jemalloc-prefix=je_
// ===========================================================================
#[cfg(feature = "jemalloc")]
mod backend {
    use super::*;
    // The extended API is used rather than je_malloc/je_free because it takes
    // the alignment and the size on every call, which is exactly what
    // GlobalAlloc has and what lets jemalloc skip its size lookup on free.
    const MALLOCX_ZERO: i32 = 0x40;
    #[inline]
    fn align_flag(align: usize) -> i32 {
        // MALLOCX_ALIGN(a) is log2(a) in the low bits.
        align.trailing_zeros() as i32
    }
    extern "C" {
        fn je_mallocx(size: usize, flags: i32) -> *mut u8;
        fn je_rallocx(ptr: *mut u8, size: usize, flags: i32) -> *mut u8;
        fn je_sdallocx(ptr: *mut u8, size: usize, flags: i32);
    }
    pub struct B;
    impl RawAlloc for B {
        const NAME: &'static str = "jemalloc";
        #[inline]
        unsafe fn alloc(size: usize, align: usize) -> *mut u8 {
            // ⚠ mallocx(0, ..) is undefined in jemalloc. Rust never asks for a
            // zero-size allocation through GlobalAlloc, but a one-byte request
            // costs nothing and removes the trap entirely.
            let size = if size == 0 { 1 } else { size };
            unsafe { je_mallocx(size, align_flag(align)) }
        }
        #[inline]
        unsafe fn dealloc(ptr: *mut u8, size: usize, align: usize) {
            let size = if size == 0 { 1 } else { size };
            unsafe { je_sdallocx(ptr, size, align_flag(align)) }
        }
        #[inline]
        unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            let n = if new_size == 0 { 1 } else { new_size };
            // rallocx preserves the alignment given in its flags, so this is
            // correct for over-aligned blocks too.
            unsafe { je_rallocx(ptr, n, align_flag(layout.align())) }
        }
        #[inline]
        unsafe fn zalloc(size: usize, align: usize) -> *mut u8 {
            let size = if size == 0 { 1 } else { size };
            unsafe { je_mallocx(size, align_flag(align) | MALLOCX_ZERO) }
        }
    }
}

// ===========================================================================
// snmalloc, built with SNMALLOC_STATIC_LIBRARY_PREFIX=sn_
// ===========================================================================
#[cfg(feature = "snmalloc")]
mod backend {
    use super::*;
    extern "C" {
        fn sn_malloc(size: usize) -> *mut u8;
        fn sn_calloc(n: usize, size: usize) -> *mut u8;
        fn sn_realloc(p: *mut u8, size: usize) -> *mut u8;
        fn sn_free(p: *mut u8);
        fn sn_aligned_alloc(alignment: usize, size: usize) -> *mut u8;
    }
    pub struct B;
    impl RawAlloc for B {
        const NAME: &'static str = "snmalloc";
        #[inline]
        unsafe fn alloc(size: usize, align: usize) -> *mut u8 {
            if align <= MIN_ALIGN {
                unsafe { sn_malloc(size) }
            } else {
                // C requires size to be a multiple of alignment for
                // aligned_alloc; snmalloc is lenient but the rounding costs
                // nothing and keeps this correct against a stricter build.
                let size = (size + align - 1) & !(align - 1);
                unsafe { sn_aligned_alloc(align, size) }
            }
        }
        #[inline]
        unsafe fn dealloc(ptr: *mut u8, _size: usize, _align: usize) {
            unsafe { sn_free(ptr) }
        }
        #[inline]
        unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if layout.align() <= MIN_ALIGN {
                unsafe { sn_realloc(ptr, new_size) }
            } else {
                // No aligned realloc in the C surface: do it by hand.
                unsafe { realloc_by_hand::<B>(ptr, layout, new_size) }
            }
        }
        #[inline]
        unsafe fn zalloc(size: usize, align: usize) -> *mut u8 {
            if align <= MIN_ALIGN {
                unsafe { sn_calloc(1, size) }
            } else {
                let p = unsafe { Self::alloc(size, align) };
                if !p.is_null() {
                    unsafe { core::ptr::write_bytes(p, 0, size) };
                }
                p
            }
        }
    }
}

// ===========================================================================
// rpmalloc
// ===========================================================================
#[cfg(feature = "rpmalloc")]
mod backend {
    use super::*;
    use std::sync::Once;

    extern "C" {
        fn rpmalloc_initialize(memory_interface: *mut core::ffi::c_void) -> i32;
        fn rpmalloc_thread_initialize();
        fn rpmalloc_is_thread_initialized() -> i32;
        fn rpaligned_alloc(alignment: usize, size: usize) -> *mut u8;
        fn rpaligned_calloc(alignment: usize, num: usize, size: usize) -> *mut u8;
        fn rpaligned_realloc(
            ptr: *mut u8,
            alignment: usize,
            size: usize,
            oldsize: usize,
            flags: u32,
        ) -> *mut u8;
        fn rpfree(ptr: *mut u8);
    }

    static INIT: Once = Once::new();

    // ⚠ rpmalloc is the only allocator here that must be initialised
    // explicitly, per process AND per thread, before its first use. Skipping
    // the thread step does not fail loudly: it corrupts. `rpmalloc_thread_
    // initialize` is therefore called on every allocation path, guarded by
    // rpmalloc's own cheap `is_thread_initialized` check rather than by a Rust
    // thread-local, because a thread_local with a destructor would itself
    // allocate and recurse into this allocator during teardown.
    #[inline(always)]
    fn ensure() {
        INIT.call_once(|| unsafe {
            rpmalloc_initialize(core::ptr::null_mut());
        });
        unsafe {
            if rpmalloc_is_thread_initialized() == 0 {
                rpmalloc_thread_initialize();
            }
        }
    }

    pub struct B;
    impl RawAlloc for B {
        const NAME: &'static str = "rpmalloc";
        fn init_process() {
            ensure();
        }
        #[inline(always)]
        fn init_thread() {
            ensure();
        }
        #[inline]
        unsafe fn alloc(size: usize, align: usize) -> *mut u8 {
            ensure();
            unsafe { rpaligned_alloc(align, size) }
        }
        #[inline]
        unsafe fn dealloc(ptr: *mut u8, _size: usize, _align: usize) {
            ensure();
            unsafe { rpfree(ptr) }
        }
        #[inline]
        unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            ensure();
            unsafe { rpaligned_realloc(ptr, layout.align(), new_size, layout.size(), 0) }
        }
        #[inline]
        unsafe fn zalloc(size: usize, align: usize) -> *mut u8 {
            ensure();
            unsafe { rpaligned_calloc(align, 1, size) }
        }
    }
}

// ===========================================================================
// hardened_malloc
// ===========================================================================
#[cfg(feature = "hardened_malloc")]
mod backend {
    use super::*;
    extern "C" {
        fn h_malloc(size: usize) -> *mut u8;
        fn h_calloc(nmemb: usize, size: usize) -> *mut u8;
        fn h_realloc(ptr: *mut u8, size: usize) -> *mut u8;
        fn h_free(ptr: *mut u8);
        fn h_aligned_alloc(alignment: usize, size: usize) -> *mut u8;
    }
    pub struct B;
    impl RawAlloc for B {
        const NAME: &'static str = "hardened_malloc";
        #[inline]
        unsafe fn alloc(size: usize, align: usize) -> *mut u8 {
            if align <= MIN_ALIGN {
                unsafe { h_malloc(size) }
            } else {
                let size = (size + align - 1) & !(align - 1);
                unsafe { h_aligned_alloc(align, size) }
            }
        }
        #[inline]
        unsafe fn dealloc(ptr: *mut u8, _size: usize, _align: usize) {
            unsafe { h_free(ptr) }
        }
        #[inline]
        unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if layout.align() <= MIN_ALIGN {
                unsafe { h_realloc(ptr, new_size) }
            } else {
                unsafe { realloc_by_hand::<B>(ptr, layout, new_size) }
            }
        }
        #[inline]
        unsafe fn zalloc(size: usize, align: usize) -> *mut u8 {
            if align <= MIN_ALIGN {
                unsafe { h_calloc(1, size) }
            } else {
                let p = unsafe { Self::alloc(size, align) };
                if !p.is_null() {
                    unsafe { core::ptr::write_bytes(p, 0, size) };
                }
                p
            }
        }
    }
}

// ===========================================================================
// system: plain libc. This is what the binary gets when no allocator feature
// is selected, and it is also the shape the `libc-surgery` and `link-override`
// mechanisms use -- there, `malloc` itself has been replaced underneath, so the
// shim must NOT reach past it to a prefixed symbol.
// ===========================================================================
#[cfg(not(any(
    feature = "mimalloc",
    feature = "jemalloc",
    feature = "snmalloc",
    feature = "rpmalloc",
    feature = "hardened_malloc"
)))]
mod backend {
    use super::*;
    extern "C" {
        fn malloc(size: usize) -> *mut u8;
        fn calloc(n: usize, size: usize) -> *mut u8;
        fn realloc(p: *mut u8, size: usize) -> *mut u8;
        fn free(p: *mut u8);
        fn posix_memalign(memptr: *mut *mut u8, alignment: usize, size: usize) -> i32;
    }
    pub struct B;
    impl RawAlloc for B {
        const NAME: &'static str = "system";
        #[inline]
        unsafe fn alloc(size: usize, align: usize) -> *mut u8 {
            if align <= MIN_ALIGN {
                unsafe { malloc(size) }
            } else {
                let mut p: *mut u8 = core::ptr::null_mut();
                // posix_memalign rather than aligned_alloc: it is present in
                // every libc this project targets, including older musl, and
                // it does not require size to be a multiple of alignment.
                let rc = unsafe { posix_memalign(&mut p, align, size) };
                if rc != 0 {
                    core::ptr::null_mut()
                } else {
                    p
                }
            }
        }
        #[inline]
        unsafe fn dealloc(ptr: *mut u8, _size: usize, _align: usize) {
            unsafe { free(ptr) }
        }
        #[inline]
        unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if layout.align() <= MIN_ALIGN {
                unsafe { realloc(ptr, new_size) }
            } else {
                unsafe { realloc_by_hand::<B>(ptr, layout, new_size) }
            }
        }
        #[inline]
        unsafe fn zalloc(size: usize, align: usize) -> *mut u8 {
            if align <= MIN_ALIGN {
                unsafe { calloc(1, size) }
            } else {
                let p = unsafe { Self::alloc(size, align) };
                if !p.is_null() {
                    unsafe { core::ptr::write_bytes(p, 0, size) };
                }
                p
            }
        }
    }
}

/// The type ripgrep's `main.rs` is given as its `#[global_allocator]`.
pub struct Alloc;

/// The backend actually compiled in. Printed by the build metadata and
/// asserted against the ELF evidence, so a mis-specified feature is caught
/// rather than measured.
pub const BACKEND: &str = <backend::B as RawAlloc>::NAME;

/// Force process-level initialisation. Safe to call more than once.
pub fn init() {
    <backend::B as RawAlloc>::init_process();
}

unsafe impl GlobalAlloc for Alloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { <backend::B as RawAlloc>::alloc(layout.size(), layout.align()) }
    }
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { <backend::B as RawAlloc>::dealloc(ptr, layout.size(), layout.align()) }
    }
    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe { <backend::B as RawAlloc>::realloc(ptr, layout, new_size) }
    }
    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { <backend::B as RawAlloc>::zalloc(layout.size(), layout.align()) }
    }
}

// ---------------------------------------------------------------------------
// These run under the host's own allocator when the crate is tested without a
// backend feature. They are here because an alignment bug in this file would
// show up as an allocator "crashing" and be written down as that allocator's
// fault.
#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::GlobalAlloc;

    #[test]
    fn over_aligned_roundtrip() {
        for &align in &[16usize, 32, 64, 128, 4096] {
            for &size in &[1usize, 15, 64, 1000, 100_000] {
                let layout = Layout::from_size_align(size, align).unwrap();
                unsafe {
                    let p = Alloc.alloc(layout);
                    assert!(!p.is_null(), "alloc failed size={} align={}", size, align);
                    assert_eq!(
                        p as usize % align,
                        0,
                        "misaligned size={} align={}",
                        size,
                        align
                    );
                    core::ptr::write_bytes(p, 0xAB, size);
                    Alloc.dealloc(p, layout);
                }
            }
        }
    }

    #[test]
    fn zeroed_is_zero() {
        for &align in &[16usize, 64, 4096] {
            let layout = Layout::from_size_align(4096, align).unwrap();
            unsafe {
                let p = Alloc.alloc_zeroed(layout);
                assert!(!p.is_null());
                assert_eq!(p as usize % align, 0);
                assert!(core::slice::from_raw_parts(p, 4096).iter().all(|b| *b == 0));
                Alloc.dealloc(p, layout);
            }
        }
    }

    #[test]
    fn realloc_preserves_bytes_and_alignment() {
        for &align in &[16usize, 64, 256] {
            let layout = Layout::from_size_align(256, align).unwrap();
            unsafe {
                let p = Alloc.alloc(layout);
                assert!(!p.is_null());
                for i in 0..256 {
                    *p.add(i) = (i % 251) as u8;
                }
                let q = Alloc.realloc(p, layout, 8192);
                assert!(!q.is_null());
                assert_eq!(q as usize % align, 0, "realloc lost alignment {}", align);
                for i in 0..256 {
                    assert_eq!(*q.add(i), (i % 251) as u8, "realloc lost byte {}", i);
                }
                Alloc.dealloc(q, Layout::from_size_align(8192, align).unwrap());
            }
        }
    }

    #[test]
    fn shrink_keeps_prefix() {
        let layout = Layout::from_size_align(8192, 64).unwrap();
        unsafe {
            let p = Alloc.alloc(layout);
            for i in 0..8192 {
                *p.add(i) = (i % 253) as u8;
            }
            let q = Alloc.realloc(p, layout, 128);
            assert!(!q.is_null());
            for i in 0..128 {
                assert_eq!(*q.add(i), (i % 253) as u8);
            }
            Alloc.dealloc(q, Layout::from_size_align(128, 64).unwrap());
        }
    }
}
