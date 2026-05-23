//! Typed logical schema descriptors.

#![allow(clippy::result_large_err)]

mod canonical;
mod descriptors;
mod error;
mod validation;

pub use descriptors::*;
pub use error::{Result, SchemaError};

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
