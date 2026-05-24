pub(crate) mod binary2fj;
pub(crate) mod cover;
pub(crate) mod executor;
pub(crate) mod explain;
pub(crate) mod free_join;
pub(crate) mod model;
pub(crate) mod normalize;
pub(crate) mod planner;
pub(crate) mod predicate;
pub(crate) mod projection_dedup;
pub(crate) mod run;
pub(crate) mod runtime;
pub(crate) mod runtime_frame;
pub(crate) mod runtime_keys;
pub(crate) mod runtime_vectorized;
pub(crate) mod sink;
pub(crate) mod source_build;
pub(crate) mod trace;

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod normalize_tests;
