//! Internal Rust implementation of the bumbledb log product. The public
//! log surface is TypeScript-only (`@bjornpagen/bumbledb-log`); this
//! crate is `publish = false` and every module is `#[doc(hidden)]`
//! internal implementation — not a supported public Rust log SDK.
#[doc(hidden)]
pub mod admin;
#[doc(hidden)]
pub mod apply;
#[doc(hidden)]
pub mod backup;
#[doc(hidden)]
pub mod certainty;
#[doc(hidden)]
pub mod checkpointer;
#[doc(hidden)]
pub mod codec;
#[doc(hidden)]
pub mod erase;
#[doc(hidden)]
pub mod gc;
#[doc(hidden)]
pub mod history;
#[doc(hidden)]
pub mod identities;
#[doc(hidden)]
pub mod inspect;
#[doc(hidden)]
pub mod local_roots;
#[doc(hidden)]
pub mod manifest;
#[doc(hidden)]
pub mod migration;
#[doc(hidden)]
pub mod recovery;
#[doc(hidden)]
pub mod replica;
#[doc(hidden)]
pub mod restore;
#[doc(hidden)]
pub mod schema_file;
#[doc(hidden)]
pub mod store;
#[doc(hidden)]
pub mod tenants;
#[doc(hidden)]
pub mod writer;
