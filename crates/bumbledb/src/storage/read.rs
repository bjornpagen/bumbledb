//! Storage read primitives (docs/architecture/50-storage.md): membership probe, key-guard probe,
//! fact fetch, the sequential relation scan that feeds images and export,
//! and the planner's row count. All allocation-free with borrowed returns.
//!
//! Namespace readers per `docs/architecture/50-storage.md`: `M` serves
//! idempotence and point lookups, `U` functionality judgments and
//! guard-probe lookups, `F` image builds / point-lookup fetch / export
//! scan, `S` the planner.

mod check_width;
mod data_entries;
mod fact_row;
mod fetch;
mod guard_row;
mod row_count;
mod row_id_value;
mod scan;

#[cfg(test)]
mod tests;

pub use data_entries::data_entries;
pub use fact_row::{fact_row, fact_row_by_hash};
pub use fetch::fetch;
pub use guard_row::guard_row;
pub use row_count::row_count;
pub use scan::scan;
