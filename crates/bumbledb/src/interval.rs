//! The checked [`Interval`] (from `bumbledb-theory`) and the engine's interval
//! machinery: the segment sweep and the overlap index.
pub(crate) mod overlap;
pub(crate) mod sweep;

pub use bumbledb_theory::{Discrete, Element, FloatMeasureError, Interval};
