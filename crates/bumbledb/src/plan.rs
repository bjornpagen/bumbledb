//! Statistics, the grounding, the DP planner, and Free Join plan lowering
//! (docs/architecture).

pub mod fj;
pub(crate) mod ground;
pub mod planner;
pub(crate) mod selectivity;
