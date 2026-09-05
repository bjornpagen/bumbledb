//! The blessed Rust host-side query surface, quarantined:
//! hosts may depend on
//! this crate, the engine never depends back. This is the one name hosts
//! spell; the `query!`/`params!` proc-macro mechanics — and the notation
//! grammar's normative module doc, including the chapter 34 typed-template
//! contract (`template.bind(params! { name: value })` → `Vec<ParamArg>`)
//! — live with the macros in `bumbledb-query-macros`, re-exported here
//! (and from `bumbledb` itself — the same macros, one expansion).
pub use bumbledb_query_macros::{params, query};
