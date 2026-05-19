//! Optional allocation telemetry hooks.
//!
//! This module never installs a global allocator. Binaries that want allocation
//! telemetry can install their own allocator and call these record functions.

#[cfg(feature = "allocation-telemetry")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Number of allocation size-class buckets.
pub const ALLOCATION_SIZE_CLASS_COUNT: usize = 16;

/// Inclusive allocation size-class upper bounds in bytes.
pub const ALLOCATION_SIZE_CLASS_LIMITS: [u64; ALLOCATION_SIZE_CLASS_COUNT] = [
    8,
    16,
    32,
    64,
    128,
    256,
    512,
    1024,
    2048,
    4096,
    8192,
    16_384,
    32_768,
    65_536,
    262_144,
    u64::MAX,
];

#[cfg(feature = "allocation-telemetry")]
static ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "allocation-telemetry")]
static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "allocation-telemetry")]
static DEALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "allocation-telemetry")]
static REALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "allocation-telemetry")]
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "allocation-telemetry")]
static BYTES_DEALLOCATED: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "allocation-telemetry")]
static CURRENT_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "allocation-telemetry")]
static PEAK_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "allocation-telemetry")]
static SIZE_CLASS_ALLOCS: [AtomicU64; ALLOCATION_SIZE_CLASS_COUNT] =
    [const { AtomicU64::new(0) }; ALLOCATION_SIZE_CLASS_COUNT];

/// Allocation counter snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationSnapshot {
    /// True when allocation telemetry is active.
    pub enabled: bool,
    /// Allocation calls observed.
    pub alloc_calls: u64,
    /// Deallocation calls observed.
    pub dealloc_calls: u64,
    /// Reallocation calls observed.
    pub realloc_calls: u64,
    /// Bytes allocated.
    pub bytes_allocated: u64,
    /// Bytes deallocated.
    pub bytes_deallocated: u64,
    /// Currently live allocated bytes.
    pub current_live_bytes: u64,
    /// Peak live allocated bytes.
    pub peak_live_bytes: u64,
    /// Allocation calls by size class.
    pub size_class_allocs: [u64; ALLOCATION_SIZE_CLASS_COUNT],
}

/// Delta between two allocation snapshots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationDelta {
    /// True when allocation telemetry is active.
    pub enabled: bool,
    /// Allocation calls observed.
    pub alloc_calls: u64,
    /// Deallocation calls observed.
    pub dealloc_calls: u64,
    /// Reallocation calls observed.
    pub realloc_calls: u64,
    /// Bytes allocated.
    pub bytes_allocated: u64,
    /// Bytes deallocated.
    pub bytes_deallocated: u64,
    /// Net allocated bytes.
    pub net_bytes: i128,
    /// Current live byte delta after the interval.
    pub current_live_bytes: u64,
    /// Approximate peak live bytes over the snapshot interval.
    pub peak_live_bytes: u64,
    /// Allocation calls by size class.
    pub size_class_allocs: [u64; ALLOCATION_SIZE_CLASS_COUNT],
}

/// Records a successful allocation.
#[cfg(feature = "allocation-telemetry")]
#[inline]
pub fn record_alloc(size: usize) {
    let size = size as u64;
    ACTIVE.store(true, Ordering::Relaxed);
    ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    BYTES_ALLOCATED.fetch_add(size, Ordering::Relaxed);
    SIZE_CLASS_ALLOCS[size_class(size)].fetch_add(1, Ordering::Relaxed);
    let live = CURRENT_LIVE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
    update_peak(live);
}

/// Records a deallocation.
#[cfg(feature = "allocation-telemetry")]
#[inline]
pub fn record_dealloc(size: usize) {
    let size = size as u64;
    ACTIVE.store(true, Ordering::Relaxed);
    DEALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    BYTES_DEALLOCATED.fetch_add(size, Ordering::Relaxed);
    CURRENT_LIVE_BYTES.fetch_sub(size, Ordering::Relaxed);
}

/// Records a successful reallocation.
#[cfg(feature = "allocation-telemetry")]
#[inline]
pub fn record_realloc(old_size: usize, new_size: usize) {
    let old_size = old_size as u64;
    let new_size = new_size as u64;
    ACTIVE.store(true, Ordering::Relaxed);
    REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    BYTES_ALLOCATED.fetch_add(new_size, Ordering::Relaxed);
    BYTES_DEALLOCATED.fetch_add(old_size, Ordering::Relaxed);
    SIZE_CLASS_ALLOCS[size_class(new_size)].fetch_add(1, Ordering::Relaxed);
    if new_size >= old_size {
        let delta = new_size - old_size;
        let live = CURRENT_LIVE_BYTES.fetch_add(delta, Ordering::Relaxed) + delta;
        update_peak(live);
    } else {
        CURRENT_LIVE_BYTES.fetch_sub(old_size - new_size, Ordering::Relaxed);
    }
}

/// Records a successful allocation when telemetry is disabled.
#[cfg(not(feature = "allocation-telemetry"))]
#[inline]
pub fn record_alloc(_size: usize) {}

/// Records a deallocation when telemetry is disabled.
#[cfg(not(feature = "allocation-telemetry"))]
#[inline]
pub fn record_dealloc(_size: usize) {}

/// Records a successful reallocation when telemetry is disabled.
#[cfg(not(feature = "allocation-telemetry"))]
#[inline]
pub fn record_realloc(_old_size: usize, _new_size: usize) {}

/// Captures the current allocation counters.
#[cfg(feature = "allocation-telemetry")]
pub fn snapshot() -> AllocationSnapshot {
    let mut size_class_allocs = [0; ALLOCATION_SIZE_CLASS_COUNT];
    for (index, bucket) in SIZE_CLASS_ALLOCS.iter().enumerate() {
        size_class_allocs[index] = bucket.load(Ordering::Relaxed);
    }
    AllocationSnapshot {
        enabled: ACTIVE.load(Ordering::Relaxed),
        alloc_calls: ALLOC_CALLS.load(Ordering::Relaxed),
        dealloc_calls: DEALLOC_CALLS.load(Ordering::Relaxed),
        realloc_calls: REALLOC_CALLS.load(Ordering::Relaxed),
        bytes_allocated: BYTES_ALLOCATED.load(Ordering::Relaxed),
        bytes_deallocated: BYTES_DEALLOCATED.load(Ordering::Relaxed),
        current_live_bytes: CURRENT_LIVE_BYTES.load(Ordering::Relaxed),
        peak_live_bytes: PEAK_LIVE_BYTES.load(Ordering::Relaxed),
        size_class_allocs,
    }
}

/// Captures disabled allocation counters.
#[cfg(not(feature = "allocation-telemetry"))]
pub fn snapshot() -> AllocationSnapshot {
    AllocationSnapshot::default()
}

/// Returns the saturating delta between two snapshots.
pub fn delta(before: AllocationSnapshot, after: AllocationSnapshot) -> AllocationDelta {
    let mut size_class_allocs = [0; ALLOCATION_SIZE_CLASS_COUNT];
    for (index, value) in size_class_allocs.iter_mut().enumerate() {
        *value = after.size_class_allocs[index].saturating_sub(before.size_class_allocs[index]);
    }
    let bytes_allocated = after.bytes_allocated.saturating_sub(before.bytes_allocated);
    let bytes_deallocated = after
        .bytes_deallocated
        .saturating_sub(before.bytes_deallocated);
    let peak_live_bytes = if after.peak_live_bytes > before.peak_live_bytes {
        after
            .peak_live_bytes
            .saturating_sub(before.current_live_bytes)
    } else {
        after
            .current_live_bytes
            .saturating_sub(before.current_live_bytes)
    };
    AllocationDelta {
        enabled: before.enabled || after.enabled,
        alloc_calls: after.alloc_calls.saturating_sub(before.alloc_calls),
        dealloc_calls: after.dealloc_calls.saturating_sub(before.dealloc_calls),
        realloc_calls: after.realloc_calls.saturating_sub(before.realloc_calls),
        bytes_allocated,
        bytes_deallocated,
        net_bytes: i128::from(bytes_allocated) - i128::from(bytes_deallocated),
        current_live_bytes: after
            .current_live_bytes
            .saturating_sub(before.current_live_bytes),
        peak_live_bytes,
        size_class_allocs,
    }
}

#[cfg(feature = "allocation-telemetry")]
#[inline]
fn size_class(size: u64) -> usize {
    for (index, limit) in ALLOCATION_SIZE_CLASS_LIMITS.iter().enumerate() {
        if size <= *limit {
            return index;
        }
    }
    ALLOCATION_SIZE_CLASS_COUNT - 1
}

#[cfg(feature = "allocation-telemetry")]
#[inline]
fn update_peak(live: u64) {
    let mut observed = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
    while live > observed {
        match PEAK_LIVE_BYTES.compare_exchange_weak(
            observed,
            live,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(current) => observed = current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_delta_saturates_and_computes_net() {
        let before = AllocationSnapshot {
            enabled: true,
            alloc_calls: 10,
            dealloc_calls: 5,
            realloc_calls: 1,
            bytes_allocated: 100,
            bytes_deallocated: 40,
            current_live_bytes: 60,
            peak_live_bytes: 80,
            size_class_allocs: [1; ALLOCATION_SIZE_CLASS_COUNT],
        };
        let after = AllocationSnapshot {
            enabled: true,
            alloc_calls: 12,
            dealloc_calls: 6,
            realloc_calls: 3,
            bytes_allocated: 180,
            bytes_deallocated: 55,
            current_live_bytes: 125,
            peak_live_bytes: 150,
            size_class_allocs: [3; ALLOCATION_SIZE_CLASS_COUNT],
        };

        let delta = delta(before, after);

        assert!(delta.enabled);
        assert_eq!(delta.alloc_calls, 2);
        assert_eq!(delta.dealloc_calls, 1);
        assert_eq!(delta.realloc_calls, 2);
        assert_eq!(delta.bytes_allocated, 80);
        assert_eq!(delta.bytes_deallocated, 15);
        assert_eq!(delta.net_bytes, 65);
        assert_eq!(delta.current_live_bytes, 65);
        assert_eq!(delta.peak_live_bytes, 90);
        assert_eq!(delta.size_class_allocs, [2; ALLOCATION_SIZE_CLASS_COUNT]);
    }
}
