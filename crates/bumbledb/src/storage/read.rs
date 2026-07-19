//! Storage read primitives (docs/architecture/50-storage.md): membership probe, key-determinant probe,
//! fact fetch, the sequential relation scan that feeds images and export,
//! and the planner's row count. All allocation-free with borrowed returns.
//!
//! Namespace readers per `docs/architecture/50-storage.md`: `M` serves
//! idempotence and point lookups, `U` functionality judgments and
//! determinant-probe lookups, `F` image builds / point-lookup fetch / export
//! scan, `S` the planner.

mod check_width;
mod data_entries;
mod determinant_row;
mod fact_row;
mod fetch;
mod row_count;
mod row_id_high_water;
mod row_id_value;
mod scan;

#[cfg(test)]
mod tests;

pub use data_entries::data_entries;
pub use determinant_row::determinant_row;
pub use fact_row::{fact_row, fact_row_by_hash};
pub use fetch::fetch;
pub use row_count::row_count;
pub use row_id_high_water::row_id_high_water;
pub use scan::{scan, scan_from};
