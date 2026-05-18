//! Core schema descriptors and sortable encodings.
//!
//! This crate has no LMDB dependency. It defines the typed logical schema and
//! the byte encodings that later storage/query layers consume.

pub mod datalog;
pub mod encoding;
pub mod schema;
