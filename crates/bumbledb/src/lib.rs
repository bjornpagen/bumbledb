//! bumbledb: an embedded, typed, set-semantic relational database over LMDB.
//!
//! The normative design lives in `docs/architecture/`; the build plan in
//! `docs/implementation/`.

// 64-bit only (docs/architecture/00-product.md): `usize` is 8 bytes everywhere
// and no design decision accommodates narrower platforms. Building for a
// 32-bit target (e.g. `--target i686-unknown-linux-gnu`) fails with this
// explicit error instead of miscompiling pointer-width assumptions.
#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only (docs/architecture/00-product.md)");

pub mod api;
pub mod encoding;
pub mod error;
pub mod exec;
pub mod image;
pub mod ir;
pub mod plan;
pub mod schema;
pub mod storage;
