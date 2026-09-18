//! Owned regions of explicitly admissible worlds.
//!
//! The same completed Boolean function supports values, dependencies and
//! relational operators. Probability is a separate observation. This crate has
//! no dependency on storage, the query engine, or a model provider.
#![forbid(unsafe_code)]
#[cfg(not(target_pointer_width = "64"))]
compile_error!("bumbledb-event currently requires a 64-bit target");

mod arena;
mod boolean;
mod codec;
mod error;
mod map;
mod registry;
mod space;

pub use boolean::{BoolOp4, Signature};
pub use error::{Capacity, Control, Error, Limits, Result};
pub use map::{CoordinateMap, SurjectiveMap};
pub use registry::Registry;
pub use space::{Event, EventKey, Space, SpaceId, Statistics};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod map_tests;
