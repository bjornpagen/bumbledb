pub(crate) mod binary2fj;
pub(crate) mod free_join;
pub(crate) mod model;
pub(crate) mod normalize;
pub(crate) mod planner;

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod normalize_tests;
