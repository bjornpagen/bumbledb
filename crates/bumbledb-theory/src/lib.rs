//! bumbledb-theory: the engine-free half of bumbledb — the value
//! vocabulary, the checked [`Interval`] type, Allen's mask algebra, and
//! the schema-as-declared surface ([`schema::SchemaDescriptor`],
//! [`schema::spec::SchemaSpec`], and the one name→id lowering).
//!
//! Everything here is plain data and pure judgment: zero dependencies,
//! zero LMDB/exec reach. The engine crate (`bumbledb`) re-exports this
//! entire surface as its own — hosts depend on one crate and never name
//! this one — and the `schema!` macro shares the same lowering, so the
//! macro and the runtime spec path cannot drift
//! (`docs/architecture/70-api.md` § the `SchemaSpec` bindings contract).
//! Encoding of facts, validation into the sealed witness, the
//! fingerprint, and storage stay engine-side.

// 64-bit only (docs/architecture/00-product.md): the product law holds
// for every workspace crate — no design decision accommodates narrower
// platforms.
#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only (docs/architecture/00-product.md)");

pub mod allen;
pub mod interval;
pub mod schema;
pub mod type_desc;
pub mod value;

pub use allen::{AllenMask, Basic};
pub use interval::Interval;
pub use type_desc::TypeDesc;
pub use value::Value;
