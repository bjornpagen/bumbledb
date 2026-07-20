//! The blessed Rust host-side query surface, quarantined
//! (docs/architecture/70-api.md § host-side sugar): hosts may depend on
//! this crate, the engine never depends back. This is the one name hosts
//! spell; the `query!` proc-macro mechanics — and the notation grammar's
//! normative module doc — live with the macro in `bumbledb-query-macros`,
//! re-exported here. The quarantine's second member is [`order`]:
//! host-side answer ordering, sort keys as data folded into one
//! comparator for the language's own sort.

pub mod order;

pub use bumbledb_query_macros::query;
