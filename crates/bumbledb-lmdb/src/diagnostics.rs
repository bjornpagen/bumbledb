use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(test)]
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static ALLOCATION_TRACKING_ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static DEALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static REALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
static ALLOCATION_TEST_LOCK: Mutex<()> = Mutex::new(());

pub struct TrackingAllocator<A> {
    inner: A,
}

impl TrackingAllocator<System> {
    pub const fn system() -> Self {
        Self { inner: System }
    }
}

// SAFETY: This allocator delegates allocation behavior to `inner` and only
// updates atomics, which does not allocate or change allocator contracts.
unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller upholds `GlobalAlloc::alloc`; this forwards the same layout.
        let ptr = unsafe { self.inner.alloc(layout) };
        if allocation_tracking_enabled() && !ptr.is_null() {
            ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if allocation_tracking_enabled() {
            DEALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        // SAFETY: The caller upholds `GlobalAlloc::dealloc`; this forwards the same pointer and layout.
        unsafe { self.inner.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: The caller upholds `GlobalAlloc::realloc`; this forwards the same arguments.
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if allocation_tracking_enabled() && !new_ptr.is_null() {
            REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationSnapshot {
    pub alloc_calls: u64,
    pub dealloc_calls: u64,
    pub realloc_calls: u64,
    pub allocated_bytes: u64,
    pub deallocated_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationDelta {
    pub alloc_calls: u64,
    pub dealloc_calls: u64,
    pub realloc_calls: u64,
    pub allocated_bytes: u64,
    pub deallocated_bytes: u64,
    pub net_bytes: i128,
}

pub fn set_allocation_tracking_enabled(enabled: bool) {
    ALLOCATION_TRACKING_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn allocation_tracking_enabled() -> bool {
    ALLOCATION_TRACKING_ENABLED.load(Ordering::Relaxed)
}

pub fn allocation_snapshot() -> AllocationSnapshot {
    AllocationSnapshot {
        alloc_calls: ALLOC_CALLS.load(Ordering::Relaxed),
        dealloc_calls: DEALLOC_CALLS.load(Ordering::Relaxed),
        realloc_calls: REALLOC_CALLS.load(Ordering::Relaxed),
        allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
        deallocated_bytes: DEALLOCATED_BYTES.load(Ordering::Relaxed),
    }
}

pub fn allocation_delta(start: AllocationSnapshot, end: AllocationSnapshot) -> AllocationDelta {
    let allocated_bytes = end.allocated_bytes.saturating_sub(start.allocated_bytes);
    let deallocated_bytes = end
        .deallocated_bytes
        .saturating_sub(start.deallocated_bytes);
    AllocationDelta {
        alloc_calls: end.alloc_calls.saturating_sub(start.alloc_calls),
        dealloc_calls: end.dealloc_calls.saturating_sub(start.dealloc_calls),
        realloc_calls: end.realloc_calls.saturating_sub(start.realloc_calls),
        allocated_bytes,
        deallocated_bytes,
        net_bytes: allocated_bytes as i128 - deallocated_bytes as i128,
    }
}

#[cfg(test)]
fn reset_allocation_counters_for_test() {
    ALLOC_CALLS.store(0, Ordering::Relaxed);
    DEALLOC_CALLS.store(0, Ordering::Relaxed);
    REALLOC_CALLS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn with_allocation_tracking_for_test<T>(f: impl FnOnce() -> T) -> T {
    let _guard = match ALLOCATION_TEST_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    reset_allocation_counters_for_test();
    set_allocation_tracking_enabled(true);
    let output = f();
    set_allocation_tracking_enabled(false);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_delta_increases_after_vec_allocation() {
        with_allocation_tracking_for_test(|| {
            let start = allocation_snapshot();

            let values = Vec::<u64>::with_capacity(128);

            let end = allocation_snapshot();
            assert!(values.capacity() >= 128);
            let delta = allocation_delta(start, end);
            assert!(delta.alloc_calls > 0);
            assert!(delta.allocated_bytes >= 128 * 8);
        });
    }
}
