//! COLT, the executor, sinks, kernels, dispatch, and introspection.
pub(crate) mod colt;
pub(crate) mod dispatch;
pub(crate) mod kernel;
pub(crate) mod run;
pub(crate) mod sink;
pub(crate) mod swar;
pub(crate) mod wordmap;

pub(crate) const SCAN_HOIST_THRESHOLD: usize = 8;
