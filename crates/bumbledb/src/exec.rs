//! COLT, the executor, sinks, kernels, dispatch, and introspection.
pub mod colt;
pub mod dispatch;
pub mod kernel;
pub mod run;
#[allow(
    dead_code,
    unused_imports,
    clippy::unused_self,
    reason = "the scratch tier is deleted once aggregate group spill is gone"
)]
pub(crate) mod scratch;
pub mod sink;
pub(crate) mod swar;
pub mod wordmap;

pub(crate) const SCAN_HOIST_THRESHOLD: usize = 8;
