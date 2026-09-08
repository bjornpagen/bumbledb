//! Fresh cancellation contexts for independent benchmark operations.
//! Allocation measurements belong to the separate allocator pass, not to
//! counters on the product's ordinary execution path.

use bumbledb::WorkContext;

/// Each operation starts uncancelled, independent of earlier operations.
#[must_use]
pub fn bench_work() -> WorkContext {
    WorkContext::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn cancelled_operation_does_not_poison_the_next_operation() {
        let work = super::bench_work();
        work.checkpoint().unwrap();
        work.cancel();
        assert_eq!(work.checkpoint(), Err(bumbledb::WorkError::Cancelled));
        super::bench_work().checkpoint().unwrap();
    }
}
