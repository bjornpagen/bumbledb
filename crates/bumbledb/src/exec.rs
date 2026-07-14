//! COLT, the executor, sinks, kernels, dispatch, and introspection (docs/architecture).

pub mod colt;
pub mod dispatch;
pub mod introspection;
pub mod kernel;
pub mod run;
pub mod sink;
mod swar;
pub mod wordmap;

/// Run length at which hoisting operand/column tables pays for itself —
/// shared by the leaf-scan residual tables (run.rs) and the projection
/// scan's column hoist (sink.rs), which encode the same measured
/// crossover: L* = `build_cost` ÷ `per-item saving`.
/// The old value of 32 was forced by a `from_fn`-of-Options table
/// costing ~34 ns/run (rust-lang/rust#108765); the Option-free prefix
/// table builds in ~3.4 ns straight-line, putting the measured
/// crossover at 4–8. The in-tree derivation test
/// (`scan_hoist_crossover_derivation`) records the curve.
pub(crate) const SCAN_HOIST_THRESHOLD: usize = 8;
