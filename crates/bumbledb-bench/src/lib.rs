//! The benchmark and oracle suite for bumbledb
//! (`docs/benchmarks/README.md` is the build plan; `docs/architecture/`
//! stays the design authority).
//!
//! Library-first: every capability is a `pub` function here; the binary is
//! argument parsing plus dispatch. The dependency quarantine
//! (`docs/architecture/00-product.md`) allows exactly `rusqlite` — JSON,
//! statistics, argument parsing, and randomness are hand-rolled.

pub mod cli;
pub mod compare;
pub mod corpus;
pub mod families;
pub mod gen;
pub mod harness;
pub mod querygen;
pub mod schema;
pub mod sqlmap;
pub mod translate;
pub mod verify;
pub mod writebench;
