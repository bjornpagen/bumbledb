//! Commit application (docs/architecture/40-storage.md): phases 1-2 of the canonical commit order —
//! all deletes, then all inserts — maintaining F/M/U/R and detecting unique
//! violations (`docs/architecture/40-storage.md`).
//!
//! Because every delete lands before any insert and the insert set is
//! deduplicated by construction, a `U` conflict during inserts is a genuine
//! unique violation; user operation order inside the transaction is
//! semantically irrelevant. The 40-storage doc extends this into the full commit
//! (FK validation, counters, tx id, LMDB commit).

use std::collections::{BTreeMap, BTreeSet};

use crate::schema::{ConstraintId, RelationId};
use crate::storage::delta::WriteDelta;
use crate::storage::env::WriteTxn;
use crate::storage::keys::KeyBuf;

mod applier;
mod apply;
mod restrict;
mod write;

#[cfg(test)]
mod tests;

pub use apply::apply;
pub use write::commit;

/// One forward-FK probe the 40-storage doc must run: collected at insert time so
/// validation never re-derives key slices.
#[derive(Debug)]
pub struct FkProbe {
    /// The referencing side, for the violation error — one exemplar per
    /// distinct probed target key (a bulk load referencing one account
    /// probes it once, not once per posting).
    pub source_relation: RelationId,
    pub source_constraint: ConstraintId,
    /// The offending fact, for the violation error.
    pub fact_bytes: Vec<u8>,
}

/// Forward-FK probes deduplicated by target key: `(target relation,
/// target constraint, guard)` → the first referencing fact.
pub type FkProbes = BTreeMap<(RelationId, ConstraintId, Vec<u8>), FkProbe>;

/// The applied-but-uncommitted state after phases 1-2, carrying the open
/// LMDB write transaction plus the bookkeeping the 40-storage doc consumes.
pub struct Applied<'env, 's> {
    /// The open, uncommitted LMDB write transaction.
    pub txn: WriteTxn<'env>,
    /// The source delta (its counters flush in the 40-storage doc's phase 4).
    pub delta: WriteDelta<'s>,
    /// Whether any insert or delete actually applied (drives the 40-storage doc's
    /// tx-id-advances-iff-state-changed rule).
    pub changed: bool,
    /// Per-relation next row id after this apply (flushed to `S` by the 40-storage doc).
    pub row_id_next: BTreeMap<RelationId, u64>,
    /// Unique keys deleted in phase 1, restricted to FK-targeted
    /// constraints — the Restrict scan set.
    pub deleted_guards: BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    /// Unique keys established in phase 2 for FK-targeted constraints —
    /// subtracted from `deleted_guards` before Restrict scanning.
    pub inserted_guards: BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    /// Forward-FK probes for every inserted fact.
    pub fk_probes: FkProbes,
}

/// The commit outcome: whether logical state changed, and the resulting
/// storage generation (the 40-storage doc's cache-eviction subscriber; the 60-api doc wires it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitReport {
    pub changed: bool,
    pub new_generation: u64,
}

/// Working state threaded through phases 1-2.
struct Applier<'env> {
    txn: WriteTxn<'env>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    changed: bool,
    row_id_next: BTreeMap<RelationId, u64>,
    deleted_guards: BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    inserted_guards: BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    fk_probes: FkProbes,
    key: KeyBuf,
    guard: Vec<u8>,
}
