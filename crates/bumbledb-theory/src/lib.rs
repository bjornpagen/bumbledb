//! bumbledb-theory: the engine-free half of bumbledb — the value
//! vocabulary, the checked [`Interval`] type, Allen's mask algebra, and
//! the schema-as-declared surface ([`schema::SchemaDescriptor`],
//! [`schema::spec::SchemaSpec`], and the one name→id lowering).
//! Everything here is plain data and pure judgment: zero dependencies,
//! zero LMDB/exec reach. The engine crate (`bumbledb`) re-exports this
#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only");

pub mod allen;
mod float;
mod id128;
pub mod interval;
pub mod schema;
pub mod value;

pub use allen::{AllenMask, Basic};
pub use float::{F64, F64CastError, F64ParseError};
pub use id128::{Id128, Id128ParseError};
pub use interval::{Discrete, Element, FloatMeasureError, Interval};
pub use schema::ValueType;
pub use value::Value;
