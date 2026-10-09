//! COLT, the executor, sinks, kernels, dispatch, and introspection.
pub(crate) mod colt;
pub(crate) mod dispatch;
pub(crate) mod kernel;
pub(crate) mod run;
pub(crate) mod sink;
pub(crate) mod swar;
pub(crate) mod wordmap;

pub(crate) const SCAN_HOIST_THRESHOLD: usize = 8;

/// A seeded sweep's case count: `n`, or sixteen times `n` under
/// `BUMBLEDB_DEEP=1`. The extra cases extend the same seed sequence.
#[cfg(test)]
pub(crate) fn sweep(n: usize) -> usize {
    if std::env::var_os("BUMBLEDB_DEEP").is_some_and(|deep| deep == "1") {
        n * 16
    } else {
        n
    }
}
