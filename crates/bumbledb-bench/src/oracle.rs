//! Independent oracles: the naive evaluator and admission model, the SQLite
//! translation, the bit-exact float models, and the differential and
//! conformance drivers that hold the engine to them.
pub mod binary64;
pub mod binary64_interval;
pub mod compare;
pub mod conformance;
pub mod differential;
pub(crate) mod float;
pub mod naive;
pub mod poststate;
pub mod querygen;
pub mod sqlite;
pub(crate) mod walk;
