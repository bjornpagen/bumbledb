//! COLT, the executor, sinks, kernels, dispatch, and introspection.
pub mod colt;
pub mod dispatch;
pub mod kernel;
pub mod run;
pub(crate) mod scratch;
pub mod sink;
pub(crate) mod swar;
pub mod wordmap;

pub(crate) const SCAN_HOIST_THRESHOLD: usize = 8;
