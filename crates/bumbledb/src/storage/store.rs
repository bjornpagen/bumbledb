//! The successor physical store (C04): one LMDB owner, owned coherent
//! snapshots, an elastic map, and the private candidate
//! prepare/admit/seal/commit capability.
//!
//! This module is the storage contract handed to P01 (candidate judgment),
//! P03 (cursor/snapshot access), P04/P05 (atomic host adjunct and snapshot
//! export) and P06 (native ownership/affinity). The transitional
//! `storage::env`/`storage::dict`/`storage::delta` machinery is deleted;
//! this store is the one storage engine.
//!
//! Selected representation, per `final-solution/10` §3–7 and `41`:
//!
//! - Rows are the canonical successor row bytes ([`crate::canonical`]),
//!   text inline in the LMDB value. There is **no dictionary database**: a
//!   deleted row leaves no independently live text entry (ENG-006).
//! - Membership is `(relation, 16-byte fingerprint, local row id) → ()`.
//!   The fingerprint is the first 16 bytes of a domain-separated BLAKE3
//!   digest; it selects a candidate bucket only. Full canonical bytes decide
//!   equality, also under forced collision. All colliding rows remain
//!   enumerable and individually deletable.
//! - Unique-key determinants are `(projection id, routing bytes, optional interval tail, local row id) → ()`
//!   where routing is either compact exact scalar bytes (≤16) or a 16-byte
//!   BLAKE3 fingerprint — a **multimap**, so competing proposals coexist
//!   proposals coexist physically while the final state is judged. Semantic
//!   uniqueness is a law enforced by judgment (C03), not an LMDB key
//!   constraint; installation-order accidents are unrepresentable. The
//!   entries are schema-derived ([`det_index`]): every sealed key
//!   statement's scalar determinant is projected, canonically encoded and
//!   fingerprinted inside the same transaction as its row mutation —
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
//!   module (ENG-008): every commit is an ordinary durable LMDB commit.
//! - [`OwnedSnapshot`] owns one real LMDB read transaction; rows, generation
//!   and opaque host attachment all derive from that one transaction
//!   (ENG-003), and export consumes only that view.
//!
//! Physical byte layout remains provisional until the F3 probes select the
//! final format (C12); the family/layout counters below exist so provisional
//! files are unambiguously refused after any change.
//!
//! # Ownership and thread constraints (C04 handoff)
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
//! on the owning worker (chapter 10 §7); there is no unsafe `Send` and no
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

pub use candidate::{
    AppliedChanges, CandidateJudge, CandidateState, Judgment, Prepared, PreparedWrite, RowIndexer,
    SealedWrite, StoreCommit, WriteOwner,
};
pub use error::{HostKeyFault, StoreError, StoreResult};
pub use fingerprint::{FP_LEN, Fingerprinter};
pub use crate::schema::{
    CompiledProjection, CompiledTheory, KeyEncoding, LMDB_KEY_LIMIT, MAX_EXACT_SCALAR_BYTES,
    ProjectionId, encode_scalar_group,
};
pub use format::{CoreStoreId, EnvironmentId, RelationVersion, RowId, StoreIdentity};
pub use host::{
    AttachmentChange, HostChanges, HostRecordChange, HostResume, HostSealError, HostWindow,
};
pub use judge_bridge::{SchemaJudge, UnindexedRows};
pub use map::{MapPolicy, MapReport};
pub use snapshot::{ExportReport, OwnedSnapshot, StorePageStats};
pub use store_env::{CloseReport, GrowReport, Store};
pub use verify::{VerifyCorruption, VerifyFinding};

#[cfg(test)]
mod tests;
