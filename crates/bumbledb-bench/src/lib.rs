//! The benchmark and oracle suite for bumbledb
//! (design authority: `docs/architecture/60-validation.md`).
//!
//! Library-first: every capability is a `pub` function here; the binary is
//! argument parsing plus dispatch. The dependency quarantine
//! (`docs/architecture/00-product.md`) allows exactly `rusqlite` — JSON,
//! statistics, argument parsing, and randomness are hand-rolled.

pub mod calendar;
pub mod cli;
pub mod clockproxy;
pub mod closure;
pub mod compare;
pub mod conformance;
pub mod corpus;
pub mod corpus_gen;
pub mod devhonesty;
pub mod differential;
pub mod driver;
pub mod families;
pub(crate) mod fixture;
pub mod harness;
pub mod json;
pub mod naive;
pub mod querygen;
pub mod report;
pub mod scenarios;
pub mod schema;
pub mod sqlite_run;
pub mod sqlmap;
pub mod storemode;
#[cfg(test)]
mod stress;
pub mod trace_out;
pub mod translate;
pub mod tripwires;
pub mod verify;
pub mod windowed;
pub mod writebench;
