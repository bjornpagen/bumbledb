//! The physical store: one LMDB environment with three databases (`meta`,
//! `rows`, `det`), owned coherent snapshots, a fixed virtual map, and the
//! private candidate (prepare, judge, seal, commit).
//!
//! Rows live once, under `[relation][home][ordinal]`, as canonical bytes;
//! the home is the relation's narrowest exact scalar key or the row's
//! fingerprint, so membership and the home key share one bucket. Every
//! other key projection keeps a `det` multimap entry, so competing proposals
//! coexist until judgment; uniqueness is judged, never an LMDB constraint.
//!
//! | Capability | Send | Sync | Lifetime |
//! | --- | --- | --- | --- |
//! | [`Store`] | yes | yes | Owner; drop closes the env, then releases the lock |
//! | [`OwnedSnapshot`] | yes | no | Owns env clone + read txn; delays close |
//! | `WriteOwner` | no | no | Holds the writer slot; stays on its worker |
//! | `PreparedWrite` | no | no | Owns the uncommitted `RwTxn` |
//! | `SealedWrite` | no | no | Commit or abort only |

pub(crate) mod candidate;
pub(crate) mod det_index;
pub(crate) mod error;
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
pub(crate) use error::StoreError;
pub use format::DatabaseId;
pub(crate) use format::{RelationVersion, StoreIdentity};
pub use host::{Head, HostChanges, HostRecord};
pub(crate) use snapshot::OwnedSnapshot;
pub(crate) use store_env::Store;
pub use store_env::{CloseReport, Durability, Options};
pub use verify::VerifyCorruption;

#[cfg(test)]
mod tests;
