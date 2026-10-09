//! The engine-free half of bumbledb: the value vocabulary, the checked
//! [`Interval`] type, Allen's mask algebra, and the declared schema with
//! its checks ([`schema::SchemaDescriptor`], [`schema::check()`],
//! [`schema::spec::SchemaSpec`]). Plain data and pure judgment; the
//! `bumbledb` engine re-exports it.

#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only");

pub mod allen;
mod float;
pub mod interval;
pub mod schema;
pub mod value;

pub use allen::{AllenMask, Basic};
pub use float::{F64, F64CastError, F64ParseError};
pub use interval::{Discrete, Element, FloatMeasureError, Interval};
pub use schema::ValueType;
pub use uuid::Uuid;
pub use value::Value;
