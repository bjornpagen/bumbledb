//! Commit application (`docs/architecture/50-storage.md` § Write path):
//! phases 1-2 of the canonical commit order — all deletes, then all inserts —
//! maintaining `F`/`M`/`U` and enforcing every `Functionality` statement:
//! scalar keys by `U` put-conflict, pointwise keys by the ordered-neighbor
//! probe (`docs/architecture/30-dependencies.md` § pointwise lifting).
//!
//! Because every delete lands before any insert and the insert set is
//! deduplicated by construction, a `U` conflict during inserts is a genuine
//! functionality violation; user operation order inside the transaction is
//! semantically irrelevant. Phases 1-2 also maintain the `R` reverse edges
//! (one per containment statement whose source selection the fact
//! satisfies), and phase 3 — the judgment phase (`judgment`) — proves every
//! containment against the final state: the source side over inserted
//! facts, the target side over disestablished key tuples.

use std::collections::{BTreeMap, BTreeSet};

use crate::schema::{RelationId, StatementId};
use crate::storage::delta::WriteDelta;
use crate::storage::env::WriteTxn;
use crate::storage::keys::KeyBuf;

mod applier;
mod apply;
// The selection machinery (`judgment::Selections`, `judgment::satisfies`)
// is shared with `Db::verify_store` — the sweeper re-checks φ with the
// commit path's own helper, never a second implementation.
pub(crate) mod judgment;
mod write;

#[cfg(test)]
mod tests;

pub use apply::apply;
pub use write::commit;

/// The applied-but-uncommitted state after phases 1-2, carrying the open
/// LMDB write transaction plus the bookkeeping the later phases consume.
pub struct Applied<'env, 's> {
    /// The open, uncommitted LMDB write transaction.
    pub txn: WriteTxn<'env>,
    /// The source delta (its counters flush in the 50-storage doc's phase 4).
    pub delta: WriteDelta<'s>,
    /// Per-relation next row id after this apply (flushed to `S` by the
    /// 50-storage doc's phase 4).
    pub row_id_next: BTreeMap<RelationId, u64>,
    /// Guard keys deleted in phase 1, limited to key statements some
    /// containment targets — the target-side check set
    /// (`judgment::check_target`).
    pub deleted_guards: BTreeSet<(StatementId, Vec<u8>)>,
    /// Guard keys established in phase 2 for containment-targeted key
    /// statements — subtracted from `deleted_guards` before the
    /// target-side scan.
    pub inserted_guards: BTreeSet<(StatementId, Vec<u8>)>,
    /// Selection literals pre-encoded once for this commit (phases 1-2
    /// gate the `R` writes with them; phase 3 reuses them for its source
    /// and target checks).
    pub selections: judgment::Selections,
}

/// The commit outcome: whether logical state changed, and the resulting
/// storage generation (the 50-storage doc's cache-eviction subscriber; the
/// 70-api doc wires it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitReport {
    pub changed: bool,
    pub new_generation: u64,
}

/// Working state threaded through phases 1-2.
struct Applier<'env> {
    txn: WriteTxn<'env>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    row_id_next: BTreeMap<RelationId, u64>,
    deleted_guards: BTreeSet<(StatementId, Vec<u8>)>,
    inserted_guards: BTreeSet<(StatementId, Vec<u8>)>,
    selections: judgment::Selections,
    key: KeyBuf,
    guard: Vec<u8>,
}
