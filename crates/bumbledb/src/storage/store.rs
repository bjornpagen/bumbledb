//! The physical store: one LMDB owner, owned coherent
//! snapshots, an elastic map, and the private candidate
//! prepare/admit/seal/commit capability.
//!
//! Judgment, snapshot access, log metadata and native ownership all use
//! this storage engine.
//!
//! Physical representation:
//!
//! - Rows are the canonical row bytes ([`crate::canonical`]),
//!   text inline in the LMDB value. There is **no dictionary database**: a
//!   deleted row leaves no independently live text entry.
//! - Row bodies are `(relation, selected exact scalar home, local ordinal)`.
//!   Relations without an eligible home use an empty home and retain ordinal
//!   placement. The body exists once: no row directory or duplicate cache.
//! - Exact membership reuses the selected scalar-home row bucket when
//!   available. Other relations keep
//!   `(relation, 16-byte fingerprint, local row id) → ()`.
//!   The fingerprint is the first 16 bytes of a domain-separated BLAKE3
//!   digest; it selects a candidate bucket only. Full canonical bytes decide
//!   equality, also under forced collision. All colliding rows remain
//!   enumerable and individually deletable.
//! - Secondary determinants are `(projection id, routing bytes, local row id) → home`
//!   where routing is either compact exact scalar bytes (≤16) or a 16-byte
//!   BLAKE3 fingerprint — a **multimap**, so competing proposals coexist
//!   physically while the final state is judged. Semantic
//!   uniqueness is a law enforced by judgment, not an LMDB key
//!   constraint; installation-order accidents are unrepresentable. The
//!   entries are schema-derived ([`det_index`]); the selected home projection
//!   has no separate entry because its row bucket already serves it. Other
//!   determinants are projected and encoded inside the
//!   same transaction as its row mutation —
//!   insert, replace, delete and snapshot adoption all maintain the index
//!   atomically, and keyed reads (point gets, key probes, judgment
//!   enumeration) resolve through the bucket plus exact decoded-value
//!   confirmation instead of a relation scan.
//! - Every physical key is fixed-width and far below LMDB's key bound; no
//!   variable-width determinant or text ever enters a key (long-key safe by
//!   construction).
//! - The map is elastic: sized from the populated file plus headroom, grown
//!   geometrically under an exclusive transaction gate ([`gate`]). There is
//!   no 32 GiB policy constant and no `NO_SYNC` open lane anywhere in this
//!   module: every commit is an ordinary durable LMDB commit.
//! - [`OwnedSnapshot`] owns one real LMDB read transaction; rows, generation
//!   and opaque host attachment all derive from that one transaction
//!   and export consumes only that view.
//!
//! [`format::FAMILY`] and [`format::LAYOUT`] identify the persisted format.
//! Incompatible layouts are refused before any mutation.
//!
//! # Ownership and thread constraints
//!
//! | Capability | Send | Sync | Lifetime |
//! | --- | --- | --- | --- |
//! | [`Store`] | yes | yes | Owner; drop closes env then releases the lock |
//! | [`OwnedSnapshot`] | yes | no | Owns env clone + read txn; blocks resize |
//! | [`WriteOwner`] | no | no | Holds the writer mutex; stays on its worker |
//! | [`PreparedWrite`] | no | no | Owns the uncommitted `RwTxn` + evidence |
//! | [`SealedWrite`] | no | no | Commit/abort only; facts frozen at seal |
//!
//! A prepared/sealed write never crosses threads: LMDB write transactions
//! are thread-affine and the types are `!Send` through the owner borrow and
//! the `RwTxn`. A hosted publication attempt therefore keeps its candidate
//! on the owning worker; there is no unsafe `Send` and no
//! lifetime erasure here, and none may be added.

pub mod candidate;
pub mod copy;
pub use copy::FreshDestination;
pub(crate) mod det_index;
pub mod error;
pub mod fingerprint;
pub mod format;
pub mod gate;
pub mod host;
pub mod judge_bridge;
pub mod keys;
pub mod map;
pub(crate) mod rows;
pub mod snapshot;
pub mod staging;
pub mod store_env;
pub mod verify;

pub use staging::{
    AdmittedStore, InstallOutcome, StageReader, StageWriter, StagingCleanup, UnreadyStore,
};

pub use crate::schema::{
    CompiledProjection, CompiledTheory, KeyEncoding, LMDB_KEY_LIMIT, MAX_EXACT_SCALAR_BYTES,
    ProjectionId, encode_scalar_group,
};
pub use candidate::{
    AppliedChanges, CandidateJudge, CandidateState, Judgment, Prepared, PreparedWrite,
    ProjectionEmitter, RowIndexer, SealedWrite, StoreCommit, WriteOwner,
};
pub use error::{HostKeyFault, StoreError, StoreResult};
pub use fingerprint::{FP_LEN, Fingerprinter};
pub use format::{CoreStoreId, EnvironmentId, RelationVersion, RowId, RowLocator, StoreIdentity};
pub use host::{
    AttachmentChange, HostChanges, HostRecordChange, HostResume, HostSealError, HostWindow,
};
pub use judge_bridge::{SchemaJudge, UnindexedRows};
pub use keys::{PhysicalKeyKind, PhysicalKeyWidths};
pub use map::{MapPolicy, MapReport};
pub use snapshot::{ExportReport, OwnedSnapshot, StorePageStats};
pub use store_env::{CloseReport, GrowReport, Store};
pub use verify::{VerifyCorruption, VerifyFinding};

#[cfg(test)]
mod tests;
