//! The counting allocator (PRD 26): test-support machinery for the
//! zero-warm-allocation gate (`docs/architecture/30-execution.md`,
//! success criterion 3 of `00-product.md`).
//!
//! Feature-gated (`alloc-counter`) and thread-naive by design — the gate
//! protocol is single-threaded. The counter wraps the system allocator and
//! counts every allocation and reallocation; deallocations are counted
//! separately (a measured window must see **zero** of either — arena
//! growth is a failure, not amortization).
//!
//! Sanctioned allocation windows, documented per the protocol: the first
//! execution after prepare (COLT pools, sink maps, and view buffers grow
//! to their high-water), the first execution after a commit (image
//! rebuild), and caller result-buffer growth. Everything else on a warm
//! execution is a bug.

#![allow(unsafe_code)] // GlobalAlloc is an unsafe trait; this module only
                       // delegates to the system allocator and counts.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);

/// The wrapping allocator, registered as the global allocator whenever the
/// `alloc-counter` feature is on.
pub struct CountingAllocator;

// SAFETY: every method delegates directly to `System`, which upholds the
// GlobalAlloc contract; the counters are side effects with no aliasing.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: forwarded contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: forwarded contract.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: forwarded contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Zeroes both counters (the start of a measured window).
pub fn reset() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
}

/// Allocations (including reallocations) since the last [`reset`].
#[must_use]
pub fn count() -> u64 {
    ALLOCATIONS.load(Ordering::Relaxed)
}

/// Deallocations since the last [`reset`].
#[must_use]
pub fn dealloc_count() -> u64 {
    DEALLOCATIONS.load(Ordering::Relaxed)
}
