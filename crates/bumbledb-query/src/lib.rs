//! The blessed Rust host-side query surface, quarantined:
//! hosts may depend on
//! this crate, the engine never depends back. This is the one name hosts
//! spell; the `query!` proc-macro mechanics — and the notation grammar's
//! normative module doc — live with the macro in `bumbledb-query-macros`,
//! re-exported here.

pub use bumbledb_query_macros::query;
