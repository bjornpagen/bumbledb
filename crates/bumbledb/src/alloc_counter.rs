//! Allocation accounting for allocation gates and benchmark diagnostics.
//! This crate's unit tests register [`CountingAllocator`] globally; another
//! test binary registers it with its own `#[global_allocator]`. Every counter
//! belongs to the calling thread, so a measured window sees only the work that
//! thread did, never the test harness's other threads.
#![allow(unsafe_code)] // GlobalAlloc delegates to the system allocator below.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Where a thread's live level starts. A thread that frees memory another
/// thread allocated stays far above zero, so differences between two readings
/// are always exact; the absolute value means nothing.
const LIVE_ORIGIN: u64 = 1 << 62;

#[derive(Clone, Copy)]
struct Counters {
    allocs: u64,
    deallocs: u64,
    alloc_bytes: u64,
    dealloc_bytes: u64,
    live: u64,
    peak: u64,
}

thread_local! {
    // Const-initialized and without a destructor, so the allocator can touch it
    // at any point in a thread's life without allocating.
    static COUNTERS: Cell<Counters> = const {
        Cell::new(Counters {
            allocs: 0,
            deallocs: 0,
            alloc_bytes: 0,
            dealloc_bytes: 0,
            live: LIVE_ORIGIN,
            peak: LIVE_ORIGIN,
        })
    };
}

/// Wrapping arithmetic: a panic inside the allocator would abort the process.
fn record(allocs: u64, deallocs: u64, alloc_bytes: u64, dealloc_bytes: u64) {
    let _ = COUNTERS.try_with(|cell| {
        let mut c = cell.get();
        c.allocs = c.allocs.wrapping_add(allocs);
        c.deallocs = c.deallocs.wrapping_add(deallocs);
        c.alloc_bytes = c.alloc_bytes.wrapping_add(alloc_bytes);
        c.dealloc_bytes = c.dealloc_bytes.wrapping_add(dealloc_bytes);
        c.live = c.live.wrapping_add(alloc_bytes).wrapping_sub(dealloc_bytes);
        c.peak = c.peak.max(c.live);
        cell.set(c);
    });
}

fn read() -> Counters {
    COUNTERS.with(Cell::get)
}

/// The counting wrapper around the system allocator.
pub struct CountingAllocator;

// SAFETY: every method forwards the caller's GlobalAlloc contract to System.
// Accounting touches only a thread-local `Cell` and does not allocate or alter
// pointers/layouts.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(1, 0, layout.size() as u64, 0);
        // SAFETY: forwarded contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(1, 0, layout.size() as u64, 0);
        // SAFETY: forwarded contract.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(0, 1, 0, layout.size() as u64);
        // SAFETY: forwarded contract.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(1, 0, new_size as u64, layout.size() as u64);
        // SAFETY: forwarded contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[cfg(test)]
#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Window-relative counters: this thread's events and bytes since its last
/// [`reset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocWindow {
    pub allocs: u64,

    pub deallocs: u64,

    pub alloc_bytes: u64,

    pub dealloc_bytes: u64,
}

/// This thread's live heap level and its high-water. Compare two readings;
/// the absolute values carry an arbitrary origin.
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

/// Reads every counter of the calling thread at once.
#[must_use]
pub fn snapshot() -> AllocSnapshot {
    let c = read();
    AllocSnapshot {
        window: AllocWindow {
            allocs: c.allocs,
            deallocs: c.deallocs,
            alloc_bytes: c.alloc_bytes,
            dealloc_bytes: c.dealloc_bytes,
        },
        absolute: AllocAbsolute {
            live_bytes: c.live,
            peak_live_bytes: c.peak,
        },
    }
}

/// Zeroes the calling thread's window counters (events and bytes) — the start
/// of a measured window. The live level is unaffected.
pub fn reset() {
    COUNTERS.with(|cell| {
        let c = cell.get();
        cell.set(Counters {
            allocs: 0,
            deallocs: 0,
            alloc_bytes: 0,
            dealloc_bytes: 0,
            ..c
        });
    });
}

/// This thread's allocation events (including reallocations) since its last
/// [`reset`].
#[must_use]
pub fn count() -> u64 {
    read().allocs
}

/// This thread's deallocation events since its last [`reset`].
#[must_use]
pub fn dealloc_count() -> u64 {
    read().deallocs
}

#[cfg(test)]
mod tests {
    use super::*;

    // Miri interprets every byte a probe writes; 64 KiB blocks keep the same
    // accounting checks well above background noise there.
    const BLOCK_USIZE: usize = if cfg!(miri) { 1 << 16 } else { 1 << 20 };
    const BLOCK: u64 = BLOCK_USIZE as u64;

    #[test]
    fn bytes_track_a_known_allocation_and_its_free() {
        let before = snapshot();
        let v: Vec<u8> = Vec::with_capacity(8 * BLOCK_USIZE);
        let mid = snapshot();
        assert!(
            mid.window.alloc_bytes - before.window.alloc_bytes >= 8 * BLOCK,
            "allocating 8 blocks must move alloc_bytes by at least that"
        );
        assert!(
            mid.absolute.live_bytes >= before.absolute.live_bytes + 4 * BLOCK,
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
                >= 8 * BLOCK
        );
        assert!(
            after.absolute.live_bytes <= mid.absolute.live_bytes.saturating_sub(4 * BLOCK),
            "live falls back after the free"
        );
        assert!(
            after.absolute.peak_live_bytes >= mid.absolute.peak_live_bytes,
            "peak-live is monotone"
        );
    }

    #[test]
    fn reset_zeroes_windows_but_not_absolutes() {
        let before = snapshot();
        let keep: Vec<u8> = Vec::with_capacity(8 * BLOCK_USIZE);
        reset();
        let snap = snapshot();

        assert!(snap.window.alloc_bytes < BLOCK, "windows rebased: {snap:?}");
        assert!(
            snap.window.dealloc_bytes < BLOCK,
            "windows rebased: {snap:?}"
        );
        assert!(
            snap.absolute.live_bytes >= before.absolute.live_bytes + 8 * BLOCK,
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
        let before = snapshot();
        let v = vec![0u8; 8 * BLOCK_USIZE];
        let mid = snapshot();
        assert!(
            mid.window.alloc_bytes - before.window.alloc_bytes >= 8 * BLOCK,
            "a zeroed 8-block allocation moves alloc_bytes by at least that"
        );
        assert!(
            mid.absolute.live_bytes >= before.absolute.live_bytes + 4 * BLOCK,
            "live rises by roughly the probe"
        );
        drop(v);
        let after = snapshot();
        assert!(
            after
                .window
                .dealloc_bytes
                .saturating_sub(mid.window.dealloc_bytes)
                >= 8 * BLOCK
        );
    }

    #[test]
    fn realloc_accounts_both_byte_sides() {
        let mut v: Vec<u8> = Vec::with_capacity(2 * BLOCK_USIZE);
        v.extend(std::iter::repeat_n(0u8, 2 * BLOCK_USIZE));
        let before = snapshot();

        v.reserve_exact(14 * BLOCK_USIZE);
        let after = snapshot();
        assert!(
            after.window.alloc_bytes - before.window.alloc_bytes >= 16 * BLOCK,
            "growth allocates the new footprint"
        );
        assert!(
            after.window.dealloc_bytes - before.window.dealloc_bytes >= 2 * BLOCK,
            "growth accounts the old footprint as freed bytes"
        );
        let live_delta = after
            .absolute
            .live_bytes
            .saturating_sub(before.absolute.live_bytes);
        assert!(
            (13 * BLOCK..=17 * BLOCK).contains(&live_delta),
            "live moves by roughly the delta: {live_delta}"
        );
        drop(v);
    }
}
