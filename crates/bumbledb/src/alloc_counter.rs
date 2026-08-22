//! The counting allocator: test-support machinery
//! for the zero-warm-allocation gate and the benchmark's memory
//! observability.
//! The counter type, its statics, and the reading functions always
//! compile so the ordinary-tier budget binary (`tests/alloc_budgets.rs`)
//! can register [`CountingAllocator`] without the feature. The lib
//! design — the protocol is single-threaded. The counter wraps the
//! Sanctioned allocation windows, documented per the protocol: the first
//! execution after prepare (COLT pools, sink maps, and view buffers grow
//! to their high-water), the first execution after a commit (image

#![allow(unsafe_code)] // GlobalAlloc is an unsafe trait; this module only

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_LIVE: AtomicU64 = AtomicU64::new(0);

fn bump_live(add: u64) {
    let prev = LIVE_BYTES.fetch_add(add, Ordering::Relaxed);
    PEAK_LIVE.fetch_max(prev.saturating_add(add), Ordering::Relaxed);
}

/// The wrapping allocator. The lib registers it under `alloc-counter`;
/// the ordinary-tier budget binary registers it when that feature is off.
pub struct CountingAllocator;

// SAFETY: every method delegates directly to `System`, which upholds the
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let bytes = layout.size() as u64;
        ALLOC_BYTES.fetch_add(bytes, Ordering::Relaxed);
        bump_live(bytes);
        // SAFETY: forwarded contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {

        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let bytes = layout.size() as u64;
        ALLOC_BYTES.fetch_add(bytes, Ordering::Relaxed);
        bump_live(bytes);
        // SAFETY: forwarded contract.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let bytes = layout.size() as u64;
        DEALLOC_BYTES.fetch_add(bytes, Ordering::Relaxed);
        LIVE_BYTES.fetch_sub(bytes, Ordering::Relaxed);
        // SAFETY: forwarded contract.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {

        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let old = layout.size() as u64;
        let new = new_size as u64;
        ALLOC_BYTES.fetch_add(new, Ordering::Relaxed);
        DEALLOC_BYTES.fetch_add(old, Ordering::Relaxed);

        if new >= old {
            bump_live(new - old);
        } else {
            LIVE_BYTES.fetch_sub(old - new, Ordering::Relaxed);
        }
        // SAFETY: forwarded contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[cfg(feature = "alloc-counter")]
#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Window-relative counters: events and bytes since the last [`reset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocWindow {

    pub allocs: u64,

    pub deallocs: u64,

    pub alloc_bytes: u64,

    pub dealloc_bytes: u64,
}

/// Process-lifetime counters: live heap and its high-water.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocAbsolute {

    pub live_bytes: u64,

    pub peak_live_bytes: u64,
}

/// One reading of every counter, split by time base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocSnapshot {
    pub window: AllocWindow,
    pub absolute: AllocAbsolute,
}

/// Reads every counter at once.
#[must_use]
pub fn snapshot() -> AllocSnapshot {
    AllocSnapshot {
        window: AllocWindow {
            allocs: ALLOCATIONS.load(Ordering::Relaxed),
            deallocs: DEALLOCATIONS.load(Ordering::Relaxed),
            alloc_bytes: ALLOC_BYTES.load(Ordering::Relaxed),
            dealloc_bytes: DEALLOC_BYTES.load(Ordering::Relaxed),
        },
        absolute: AllocAbsolute {
            live_bytes: LIVE_BYTES.load(Ordering::Relaxed),
            peak_live_bytes: PEAK_LIVE.load(Ordering::Relaxed),
        },
    }
}

/// Zeroes the window counters (events and bytes) — the start of a measured
/// window. Live bytes are absolute and unaffected.
pub fn reset() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
    DEALLOC_BYTES.store(0, Ordering::Relaxed);
}

/// Allocation events (including reallocations) since the last [`reset`].
#[must_use]
pub fn count() -> u64 {
    ALLOCATIONS.load(Ordering::Relaxed)
}

/// Deallocation events since the last [`reset`].
#[must_use]
pub fn dealloc_count() -> u64 {
    DEALLOCATIONS.load(Ordering::Relaxed)
}

#[cfg(all(test, feature = "alloc-counter"))]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static EXCLUSIVE: Mutex<()> = Mutex::new(());
    const MIB: u64 = 1 << 20;
    const MIB_USIZE: usize = 1 << 20;

    #[test]
    fn bytes_track_a_known_allocation_and_its_free() {
        let _exclusive_lock = EXCLUSIVE.lock().expect("exclusive");
        let before = snapshot();
        let v: Vec<u8> = Vec::with_capacity(8 * MIB_USIZE);
        let mid = snapshot();
        assert!(
            mid.window.alloc_bytes - before.window.alloc_bytes >= 8 * MIB,
            "allocating 8 MiB must move alloc_bytes by at least that"
        );
        assert!(
            mid.absolute.live_bytes >= before.absolute.live_bytes + 4 * MIB,
            "live rises by roughly the probe (background noise is KiBs)"
        );
        assert!(
            mid.absolute.peak_live_bytes >= mid.absolute.live_bytes,
            "peak-live is the high-water of live"
        );
        drop(v);
        let after = snapshot();
        assert!(
            after
                .window
                .dealloc_bytes
                .saturating_sub(mid.window.dealloc_bytes)
                >= 8 * MIB
        );
        assert!(
            after.absolute.live_bytes <= mid.absolute.live_bytes.saturating_sub(4 * MIB),
            "live falls back after the free"
        );
        assert!(
            after.absolute.peak_live_bytes >= mid.absolute.peak_live_bytes,
            "peak-live is monotone"
        );
    }

    #[test]
    fn reset_zeroes_windows_but_not_absolutes() {
        let _exclusive_lock = EXCLUSIVE.lock().expect("exclusive");
        let keep: Vec<u8> = Vec::with_capacity(8 * MIB_USIZE);
        reset();
        let snap = snapshot();

        assert!(snap.window.alloc_bytes < MIB, "windows rebased: {snap:?}");
        assert!(snap.window.dealloc_bytes < MIB, "windows rebased: {snap:?}");
        assert!(
            snap.absolute.live_bytes >= 8 * MIB,
            "live is absolute and survives reset"
        );
        assert!(
            snap.absolute.peak_live_bytes >= snap.absolute.live_bytes,
            "peak is absolute and survives reset"
        );
        drop(keep);
    }

    #[test]
    fn zeroed_allocations_count_once_like_plain_ones() {
        let _exclusive_lock = EXCLUSIVE.lock().expect("exclusive");
        let before = snapshot();
        let v = vec![0u8; 8 * MIB_USIZE];
        let mid = snapshot();
        assert!(
            mid.window.alloc_bytes - before.window.alloc_bytes >= 8 * MIB,
            "a zeroed 8 MiB allocation moves alloc_bytes by at least that"
        );
        assert!(
            mid.absolute.live_bytes >= before.absolute.live_bytes + 4 * MIB,
            "live rises by roughly the probe"
        );
        drop(v);
        let after = snapshot();
        assert!(
            after
                .window
                .dealloc_bytes
                .saturating_sub(mid.window.dealloc_bytes)
                >= 8 * MIB
        );
    }

    #[test]
    fn realloc_accounts_both_byte_sides() {
        let _exclusive_lock = EXCLUSIVE.lock().expect("exclusive");
        let mut v: Vec<u8> = Vec::with_capacity(2 * MIB_USIZE);
        v.extend(std::iter::repeat_n(0u8, 2 * MIB_USIZE));
        let before = snapshot();

        v.reserve_exact(14 * MIB_USIZE);
        let after = snapshot();
        assert!(
            after.window.alloc_bytes - before.window.alloc_bytes >= 16 * MIB,
            "growth allocates the new footprint"
        );
        assert!(
            after.window.dealloc_bytes - before.window.dealloc_bytes >= 2 * MIB,
            "growth accounts the old footprint as freed bytes"
        );
        let live_delta = after
            .absolute
            .live_bytes
            .saturating_sub(before.absolute.live_bytes);
        assert!(
            (13 * MIB..=17 * MIB).contains(&live_delta),
            "live moves by roughly the delta: {live_delta}"
        );
        drop(v);
    }
}
