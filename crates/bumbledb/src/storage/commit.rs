//! Commit application:
//! the commit's bookkeeping is computed first as a value — the
//! [`plan::CommitPlan`], a pure function of (delta, schema) — then phases
//! 1-2 execute it in canonical order (all deletes, then all inserts),
//! statement: scalar keys by `U` put-conflict, pointwise keys by the
//! ordered-neighbor probe.
use std::collections::BTreeMap;

use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn};

use crate::encoding::FactView;
use crate::error::{Admission, CorruptionError, Error, Result};
use crate::schema::Schema;
use crate::storage::catalog::CatalogWrite;
use crate::storage::env::{GenerationId, WriteTxn};
use crate::storage::keys::{self, KeyBuf};
use bumbledb_theory::schema::RelationId;

mod applier;
mod apply;

pub(crate) mod judgment;
mod plan;
mod write;

#[cfg(test)]
mod tests;

pub use apply::apply;
pub use write::commit;
pub(crate) use write::{flush_escaped_fresh_ids, flush_pending_escaped_fresh_ids};

/// the row ids it minted. Everything else the later phases consume lives
/// in the [`plan::CommitPlan`]. Phase 3 is [`Applied::judge`]; the
/// committable transaction is [`Judged`].
/// The applied-but-uncommitted state after phases 1-2: the open LMDB
pub struct Applied<'env> {
    /// The open, uncommitted LMDB write transaction.
    pub txn: WriteTxn<'env>,
    /// Per-relation next row id after this apply (flushed to `S` by the
    pub row_id_next: BTreeMap<RelationId, u64>,
}

/// Phase 3 has run: the write transaction may flush counters and commit.
pub struct Judged<'env> {
    txn: WriteTxn<'env>,
    row_id_next: BTreeMap<RelationId, u64>,
}

impl<'env> Applied<'env> {
    /// transaction to drop.
    pub(crate) fn judge(self, plan: &plan::CommitPlan<'_>) -> Result<Admission<Judged<'env>>> {
        let schema = plan.selections.schema();
        let final_state = judgment::FinalStateView::new(&self.txn, schema, plan);
        Ok(match judgment::judge(&final_state)? {
            Admission::Rejected(violations) => Admission::Rejected(violations),
            Admission::Accepted(()) => Admission::Accepted(Judged {
                txn: self.txn,
                row_id_next: self.row_id_next,
            }),
        })
    }
}

impl Judged<'_> {
    /// Phases 4–5: counter/dictionary flush, generation advance, LMDB commit.
    pub(crate) fn finish(
        mut self,
        delta: &crate::storage::delta::WriteDelta<'_>,
        env: &crate::storage::env::Environment,
    ) -> Result<CommitReport> {
        {
            let mut span = crate::obs::span(crate::obs::names::COUNTERS_FLUSH);
            let intern_count = delta
                .interns()
                .map_or(0, |interns| interns.entries().count() as u64);
            write::flush_counters(&mut self.txn, delta, &self.row_id_next, env)?;
            span.set_count(intern_count);
        }
        let new_generation = self.txn.generation()?.next();
        self.txn.put_generation(new_generation)?;
        {
            let _s = crate::obs::span(crate::obs::names::LMDB_COMMIT);
            self.txn.commit()?;
        }
        Ok(CommitReport::Changed { new_generation })
    }
}

/// The commit outcome: whether logical state changed, and the resulting
/// storage generation (the 50-storage doc's cache-advance subscriber; the
/// 70-api doc wires it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitReport {
    Noop { generation: GenerationId },

    Changed { new_generation: GenerationId },
}

#[cfg_attr(not(test), allow(dead_code))]
impl CommitReport {
    #[must_use]
    pub const fn changed(self) -> bool {
        matches!(self, Self::Changed { .. })
    }

    /// Generation after this commit — unchanged on [`Self::Noop`].
    #[must_use]
    pub const fn generation(self) -> GenerationId {
        match self {
            Self::Noop { generation } => generation,
            Self::Changed { new_generation } => new_generation,
        }
    }
}

/// Working state threaded through phases 1-2: the catalog, the row-id plumbing,
/// one key scratch — no derivation state; the plan owns it — and the
/// key-violation collector (recorded conflicts, sealed into the complete
/// rejection set after phase 2).
struct Applier<'c, 's, C: CatalogWrite> {
    catalog: &'c mut C,
    schema: &'s Schema,
    row_id_next: BTreeMap<RelationId, u64>,
    key: KeyBuf,
    violations: Vec<crate::error::Violation>,
}

fn decode_row_id(bytes: &[u8]) -> Result<u64> {
    crate::storage::stored_u64(bytes, "M row id")
}

fn fact_by_row<'t, 's>(
    data: Database<Bytes, Bytes>,
    txn: &'t RoTxn<'_, AnyTls>,
    schema: &'s Schema,
    relation: RelationId,
    row_id: u64,
) -> Result<FactView<'t, 's>> {
    let key = keys::fact_key(relation, row_id);
    let bytes = data
        .get(txn, &key)?
        .ok_or(Error::Corruption(CorruptionError::MissingFact {
            relation,
            row_id,
        }))?;
    crate::storage::read::check_width(schema, relation, row_id, bytes)
}
