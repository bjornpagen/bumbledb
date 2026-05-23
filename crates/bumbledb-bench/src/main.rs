#![allow(clippy::result_large_err)]

#[cfg(feature = "alloc-profile")]
mod alloc_profile {
    use std::alloc::{GlobalAlloc, Layout, System};

    pub struct CountingAllocator;

    // SAFETY: this allocator forwards all operations to the standard system
    // allocator and only records successful operations with lock-free atomics.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // SAFETY: forwarding the exact layout to the system allocator.
            let ptr = unsafe { System.alloc(layout) };
            if !ptr.is_null() {
                bumbledb_lmdb::allocation::record_alloc(layout.size());
            }
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            bumbledb_lmdb::allocation::record_dealloc(layout.size());
            // SAFETY: forwarding the original pointer and layout to the system allocator.
            unsafe { System.dealloc(ptr, layout) };
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            // SAFETY: forwarding the original pointer, layout, and requested new size.
            let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
            if !new_ptr.is_null() {
                bumbledb_lmdb::allocation::record_realloc(layout.size(), new_size);
            }
            new_ptr
        }
    }
}

#[cfg(feature = "alloc-profile")]
#[global_allocator]
static GLOBAL_ALLOCATOR: alloc_profile::CountingAllocator = alloc_profile::CountingAllocator;

fn main() {
    println!("Bumbledb benchmark harness was purged pending PRD 20.");
}

#[cfg(test)]
mod tests {
    #[test]
    fn bench_harness_is_placeholder_until_prd_20() {
        assert_eq!(env!("CARGO_PKG_NAME"), "bumbledb-bench");
    }
}
