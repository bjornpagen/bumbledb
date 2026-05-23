//! Typed schema descriptors and current index layout generation.

#![allow(clippy::result_large_err)]

mod canonical;
mod descriptors;
mod error;
mod layout;
mod validation;

pub use descriptors::*;
pub use error::{Result, SchemaError};

const INDEX_KEY_OVERHEAD_BYTES: usize = 1 + 2 + 2;
const FACT_ID_BYTES: usize = 16;

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
