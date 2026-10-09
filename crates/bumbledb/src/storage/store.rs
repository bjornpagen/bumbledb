//! The physical store: one LMDB environment with `meta`, `rows` and `det`.
//! Rows live once under `[relation][home][ordinal]` as canonical rows; the home
//! is the relation's narrowest exact scalar key or the row's fingerprint. Every
//! other key projection keeps a `det` multimap entry, so competing rows coexist
//! until judgment: uniqueness is judged, never an LMDB constraint.

pub(crate) mod candidate;
pub(crate) mod det_index;
pub(crate) mod fingerprint;
pub(crate) mod format;
pub(crate) mod gate;
pub(crate) mod host;
pub(crate) mod judge_bridge;
pub(crate) mod keys;
pub(crate) mod rows;
pub(crate) mod snapshot;
pub(crate) mod staging;
pub(crate) mod store_env;
pub(crate) mod verify;

pub use candidate::{Applied, Commit};
pub use format::DatabaseId;
pub(crate) use format::{RelationVersion, StoreIdentity};
pub use host::{Head, HostChanges, HostRecord};
pub(crate) use snapshot::OwnedSnapshot;
pub(crate) use store_env::Store;
pub use store_env::{CloseReport, Durability, Options};
pub use verify::VerifyCorruption;

#[cfg(test)]
mod tests;
