//! The counting allocator (docs/architecture/30-execution.md allocation
//! contract, 00-product.md success criterion 3): test-support machinery
//! for the zero-warm-allocation gate and the benchmark's memory
//! observability.
//!
//! Feature-gated (`alloc-counter`) and thread-naive by design — the gate
//! protocol is single-threaded. The counter wraps the system allocator and
//! tracks **events** (allocations including reallocations; deallocations)
//! and **bytes** (window-relative alloc/dealloc totals; absolute live and
//! peak-live). A measured window must see **zero** of either event kind —
//! arena growth is a failure, not amortization.
//!
//! Event/byte asymmetry for `realloc`, deliberate: a realloc counts as one
//! allocation event and zero deallocation events (the gate's historical
//! contract — events answer "did the warm path touch the allocator"),
//! while bytes account both sides (`alloc_bytes += new_size`,
//! `dealloc_bytes += old_size` — bytes answer "how much").
//!
//! Window vs absolute: [`reset`] zeroes the four window counters
//! (events + bytes); `live_bytes` and `peak_live_bytes` are absolute
//! process-lifetime values ([`reset_peak`] rebases the peak to the current
//! live).
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
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);

fn add_live(bytes: u64) {
    let live = LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    // Publish a new peak; a lost race means another thread published a
    // higher (or equally fresh) value — retry until ours is not higher.
    loop {
        let peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
        if live <= peak
            || PEAK_LIVE_BYTES
                .compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            break;
        }
    }
}

/// The wrapping allocator, registered as the global allocator whenever the
/// `alloc-counter` feature is on.
pub struct CountingAllocator;

// SAFETY: every method delegates directly to `System`, which upholds the
// GlobalAlloc contract; the counters are side effects with no aliasing.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let bytes = layout.size() as u64;
        ALLOC_BYTES.fetch_add(bytes, Ordering::Relaxed);
        add_live(bytes);
        // SAFETY: forwarded contract.
        unsafe { System.alloc(layout) }
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
        // One allocation event, zero dealloc events (the gate contract);
        // both byte sides accounted (module docs).
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let old = layout.size() as u64;
        let new = new_size as u64;
        ALLOC_BYTES.fetch_add(new, Ordering::Relaxed);
        DEALLOC_BYTES.fetch_add(old, Ordering::Relaxed);
        // Live moves by the delta: add the new footprint, drop the old.
        add_live(new);
        LIVE_BYTES.fetch_sub(old, Ordering::Relaxed);
        // SAFETY: forwarded contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// One reading of every counter (windows and absolutes together).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocSnapshot {
    /// Allocation events (including reallocations) since the last [`reset`].
    pub allocs: u64,
    /// Deallocation events since the last [`reset`].
    pub deallocs: u64,
    /// Bytes allocated since the last [`reset`].
    pub alloc_bytes: u64,
    /// Bytes deallocated since the last [`reset`].
    pub dealloc_bytes: u64,
    /// Absolute live heap bytes right now.
    pub live_bytes: u64,
    /// Absolute peak of `live_bytes` since process start (or the last
    /// [`reset_peak`]).
    pub peak_live_bytes: u64,
}

/// Reads every counter at once.
#[must_use]
pub fn snapshot() -> AllocSnapshot {
    AllocSnapshot {
        allocs: ALLOCATIONS.load(Ordering::Relaxed),
        deallocs: DEALLOCATIONS.load(Ordering::Relaxed),
        alloc_bytes: ALLOC_BYTES.load(Ordering::Relaxed),
        dealloc_bytes: DEALLOC_BYTES.load(Ordering::Relaxed),
        live_bytes: LIVE_BYTES.load(Ordering::Relaxed),
        peak_live_bytes: PEAK_LIVE_BYTES.load(Ordering::Relaxed),
    }
}

/// Zeroes the window counters (events and bytes) — the start of a measured
/// window. Live and peak are absolute and unaffected.
pub fn reset() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
    DEALLOC_BYTES.store(0, Ordering::Relaxed);
}

/// Rebases the peak to the current live footprint (the start of a
/// peak-observation window).
pub fn reset_peak() {
    PEAK_LIVE_BYTES.store(LIVE_BYTES.load(Ordering::Relaxed), Ordering::Relaxed);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// These tests read/reset shared global counters and therefore
    /// serialize among themselves; the *rest* of the lib test binary still
    /// runs concurrently, so every assertion uses a probe allocation large
    /// enough (MiBs) to dominate that background noise, with one-sided
    /// bounds or generous slack.
    static SERIAL: Mutex<()> = Mutex::new(());
    const MIB: u64 = 1 << 20;

    #[test]
    fn bytes_track_a_known_allocation_and_its_free() {
        let _guard = SERIAL.lock().expect("serial");
        let before = snapshot();
        let v: Vec<u8> = Vec::with_capacity(8 * MIB as usize);
        let mid = snapshot();
        assert!(
            mid.alloc_bytes - before.alloc_bytes >= 8 * MIB,
            "allocating 8 MiB must move alloc_bytes by at least that"
        );
        assert!(
            mid.live_bytes >= before.live_bytes + 4 * MIB,
            "live rises by roughly the probe (background noise is KiBs)"
        );
        drop(v);
        let after = snapshot();
        assert!(after.dealloc_bytes.saturating_sub(mid.dealloc_bytes) >= 8 * MIB);
        assert!(
            after.live_bytes <= mid.live_bytes.saturating_sub(4 * MIB),
            "live falls back after the free"
        );
    }

    #[test]
    fn peak_observes_a_transient_spike() {
        let _guard = SERIAL.lock().expect("serial");
        reset_peak();
        let baseline = snapshot().peak_live_bytes;
        let big: Vec<u8> = Vec::with_capacity(16 * MIB as usize);
        drop(big);
        let peak = snapshot().peak_live_bytes;
        assert!(
            peak >= baseline + 8 * MIB,
            "the 16 MiB spike must be visible in the peak: {baseline} -> {peak}"
        );
    }

    #[test]
    fn reset_zeroes_windows_but_not_absolutes() {
        let _guard = SERIAL.lock().expect("serial");
        let keep: Vec<u8> = Vec::with_capacity(8 * MIB as usize);
        reset();
        let snap = snapshot();
        // Window counters were just zeroed; background threads may have
        // ticked them since — assert they are tiny relative to the probe,
        // not exactly zero (the gate binary asserts exact zero in its
        // single-threaded world).
        assert!(snap.alloc_bytes < MIB, "windows rebased: {snap:?}");
        assert!(snap.dealloc_bytes < MIB, "windows rebased: {snap:?}");
        assert!(
            snap.live_bytes >= 8 * MIB,
            "live is absolute and survives reset"
        );
        drop(keep);
    }

    #[test]
    fn realloc_accounts_both_byte_sides() {
        let _guard = SERIAL.lock().expect("serial");
        let mut v: Vec<u8> = Vec::with_capacity(2 * MIB as usize);
        v.extend(std::iter::repeat_n(0u8, 2 * MIB as usize));
        let before = snapshot();
        // Force genuine growth of the same vec.
        v.reserve_exact(14 * MIB as usize);
        let after = snapshot();
        assert!(
            after.alloc_bytes - before.alloc_bytes >= 16 * MIB,
            "growth allocates the new footprint"
        );
        assert!(
            after.dealloc_bytes - before.dealloc_bytes >= 2 * MIB,
            "growth accounts the old footprint as freed bytes"
        );
        let live_delta = after.live_bytes.saturating_sub(before.live_bytes);
        assert!(
            (13 * MIB..=17 * MIB).contains(&live_delta),
            "live moves by roughly the delta: {live_delta}"
        );
        drop(v);
    }
}
