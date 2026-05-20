//! Core schema descriptors and sortable encodings.
//!
//! This crate has no LMDB dependency. It defines the typed logical schema and
//! the byte encodings that later storage/query layers consume.

pub mod encoding;
pub mod query_builder;
pub mod query_ir;
pub mod schema;
