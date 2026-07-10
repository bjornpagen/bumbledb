//! Commit application (`docs/architecture/50-storage.md` § Write path):
//! the commit's bookkeeping is computed first as a value — the
//! [`plan::CommitPlan`], a pure function of (delta, schema) — then phases
//! 1-2 execute it in canonical order (all deletes, then all inserts),
//! maintaining `F`/`M`/`U`/`R` and enforcing every `Functionality`
//! statement: scalar keys by `U` put-conflict, pointwise keys by the
//! ordered-neighbor probe (`docs/architecture/30-dependencies.md`
//! § pointwise lifting).
//!
//! Because every delete lands before any insert and the insert set is
//! deduplicated by construction, a `U` conflict during inserts is a genuine
//! functionality violation; user operation order inside the transaction is
//! semantically irrelevant. Phase 3 — the judgment phase (`judgment`) —
//! proves every containment against the final state, consuming the plan's
//! source-probe list and disestablished-guard check sets.

use std::collections::BTreeMap;

use crate::schema::RelationId;
use crate::storage::env::WriteTxn;
use crate::storage::keys::KeyBuf;

mod applier;
mod apply;
// The selection machinery (`judgment::Selections`, `judgment::satisfies`)
// is shared with `Db::verify_store` — the sweeper re-checks φ with the
// commit path's own helper, never a second implementation.
pub(crate) mod judgment;
mod plan;
mod write;

#[cfg(test)]
mod tests;

pub use apply::apply;
pub use write::commit;

/// The applied-but-uncommitted state after phases 1-2: the open LMDB
/// write transaction plus the one thing the executor alone can know —
/// the row ids it minted. Everything else the later phases consume lives
/// in the [`plan::CommitPlan`].
pub struct Applied<'env> {
    /// The open, uncommitted LMDB write transaction.
    pub txn: WriteTxn<'env>,
    /// Per-relation next row id after this apply (flushed to `S` by the
    /// 50-storage doc's phase 4).
    pub row_id_next: BTreeMap<RelationId, u64>,
}

/// The commit outcome: whether logical state changed, and the resulting
/// storage generation (the 50-storage doc's cache-eviction subscriber; the
/// 70-api doc wires it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitReport {
    pub changed: bool,
    pub new_generation: u64,
}

/// Working state threaded through phases 1-2: the transaction, the row-id
/// plumbing, and one key scratch — no derivation state; the plan owns it.
struct Applier<'env> {
    txn: WriteTxn<'env>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    row_id_next: BTreeMap<RelationId, u64>,
    key: KeyBuf,
}
