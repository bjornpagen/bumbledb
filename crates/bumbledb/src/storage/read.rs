//! Storage read primitives (docs/architecture/40-storage.md): membership probe, unique-guard probe,
//! fact fetch, the sequential relation scan that feeds images and export,
//! and the planner's row count. All allocation-free with borrowed returns.
//!
//! Namespace readers per `docs/architecture/40-storage.md`: `M` serves
//! idempotence and point lookups, `U` constraint checks and guard-probe
//! lookups, `F` image builds / point-lookup fetch / export scan, `S` the
//! planner.

mod check_width;
mod fact_row;
mod fetch;
mod row_count;
mod row_id_value;
mod scan;
mod unique_row;

#[cfg(test)]
mod tests;

pub use fact_row::{fact_row, fact_row_by_hash};
pub use fetch::fetch;
pub use row_count::row_count;
pub use scan::scan;
pub use unique_row::unique_row;
