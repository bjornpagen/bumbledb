//! The private candidate: one uncommitted write transaction on its owning
//! worker that committed readers never observe. Rows apply first; judgment
//! reads the proposed final state through the same transaction; seal adds
//! only host records and the head, so it cannot invalidate a verdict; commit
//! is the one durability point. A failed seal drops the whole transaction.

use bumbledb_theory::schema::{RelationId, StatementId};
use heed::{RoTxn, RwTxn};

use super::format::{K_GENERATION, RowId};
use super::host::HostChanges;
use super::rows::{self, RowWriter};
use super::store_env::{GatedRwTxn, Store, StoreInner, WriterGuard, read_generation};
use crate::changes::{ChangeKind, ChangeSet};
use crate::error::{Error, Result};
use crate::schema::judge::{JudgedViolation, Judgment};
use crate::schema::{CompiledProjection, Schema};
use crate::storage::GenerationId;
use crate::work::WorkContext;

/// Net row changes of one applied change set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Applied {
    pub added: u64,
    pub removed: u64,
}

impl Applied {
    fn changed(self) -> bool {
        self.added + self.removed > 0
    }
}

/// One change set of a batch decision: its own net changes and its
/// judgment.
#[derive(Debug)]
pub(crate) struct Decided {
    pub(crate) applied: Applied,
    pub(crate) judgment: Judgment,
}

/// A judged candidate: admitted with its open transaction, or rejected with
/// the transaction already dropped.
pub(crate) enum Candidate<'owner, 'store> {
    Admitted(PreparedWrite<'owner, 'store>),
    Rejected(Box<[JudgedViolation]>),
}

/// The exclusive writer session: it holds the writer mutex, so it stays on
/// its owning worker, and a rejected or aborted candidate leaves it owned.
pub(crate) struct WriteOwner<'store> {
    store: &'store Store,
    work: WorkContext,
    _guard: WriterGuard<'store>,
}

struct AppliedRows {
    applied: Applied,
    home_keys_preserved: bool,
}

fn apply_rows(
    inner: &StoreInner,
    txn: &mut RwTxn<'_>,
    changes: &ChangeSet,
    work: &WorkContext,
) -> Result<AppliedRows> {
    if changes.schema() != inner.schema_fp {
        return Err(Error::ForeignSchema);
    }
    let mut writer = RowWriter::new(inner, txn, work);
    let mut applied = Applied::default();
    // Removes before adds: a sealed change set already normalized its
    // one-command tie, so physical order cannot change the final state.
    for kind in [ChangeKind::Remove, ChangeKind::Add] {
        for record in changes.records().filter(|record| record.kind == kind) {
            work.checkpoint()?;
            match kind {
                ChangeKind::Remove => {
                    if writer.remove(record.relation, record.row)? {
                        applied.removed += 1;
                    }
                }
                ChangeKind::Add => {
                    if writer.insert(record.relation, record.row)?.is_some() {
                        applied.added += 1;
                    }
                }
            }
        }
    }
    let home_keys_preserved = writer.home_keys_preserved();
    writer.finish()?;
    Ok(AppliedRows {
        applied,
        home_keys_preserved,
    })
}

impl<'store> WriteOwner<'store> {
    pub(crate) fn new(store: &'store Store, guard: WriterGuard<'store>, work: WorkContext) -> Self {
        Self {
            store,
            work,
            _guard: guard,
        }
    }

    /// The committed parent generation, read while exclusivity is held.
    pub(crate) fn parent_generation(&self) -> Result<GenerationId> {
        self.store.committed_generation(&self.work)
    }

    /// Apply `changes` and judge the proposed final state incrementally: the
    /// committed parent is lawful because every commit was judged.
    pub(crate) fn prepare_judged<'owner>(
        &'owner mut self,
        schema: &Schema,
        changes: &ChangeSet,
    ) -> Result<Candidate<'owner, 'store>> {
        self.work.checkpoint()?;
        let inner = &self.store.inner;
        let mut txn = self.store.gated_write_txn(&self.work)?;
        let parent = read_generation(inner, &txn.txn)?;
        let rows = apply_rows(inner, &mut txn.txn, changes, &self.work)?;
        let state = CandidateState {
            inner,
            txn: &txn.txn,
            changes,
            home_keys_preserved: rows.home_keys_preserved,
        };
        match super::judge_bridge::judge_candidate(schema, &state, &self.work)? {
            Judgment::Rejected(rejection) => Ok(Candidate::Rejected(rejection)),
            Judgment::Admitted => Ok(Candidate::Admitted(PreparedWrite {
                owner: self,
                txn,
                parent,
                applied: vec![rows.applied],
            })),
        }
    }

    /// Apply already-decided change sets in order, unjudged.
    pub(crate) fn prepare_decided<'owner>(
        &'owner mut self,
        changes: &[ChangeSet],
    ) -> Result<PreparedWrite<'owner, 'store>> {
        self.work.checkpoint()?;
        let inner = &self.store.inner;
        let mut txn = self.store.gated_write_txn(&self.work)?;
        let parent = read_generation(inner, &txn.txn)?;
        let mut applied = Vec::with_capacity(changes.len());
        for changes in changes {
            applied.push(apply_rows(inner, &mut txn.txn, changes, &self.work)?.applied);
        }
        Ok(PreparedWrite {
            owner: self,
            txn,
            parent,
            applied,
        })
    }

    /// Insert canonical rows of one relation, unjudged: a migration
    /// population copying rows from another database.
    pub(crate) fn prepare_rows<'owner, 'row>(
        &'owner mut self,
        relation: RelationId,
        rows: &mut dyn Iterator<Item = Result<&'row [u8]>>,
    ) -> Result<PreparedWrite<'owner, 'store>> {
        let inner = &self.store.inner;
        let mut txn = self.store.gated_write_txn(&self.work)?;
        let parent = read_generation(inner, &txn.txn)?;
        let mut writer = RowWriter::new(inner, &mut txn.txn, &self.work);
        let mut applied = Applied::default();
        for row in rows {
            self.work.checkpoint()?;
            if writer.insert(relation, row?)?.is_some() {
                applied.added += 1;
            }
        }
        writer.finish()?;
        Ok(PreparedWrite {
            owner: self,
            txn,
            parent,
            applied: vec![applied],
        })
    }

    /// Judge each change set in order against the parent plus the earlier
    /// admitted sets; a rejected set rolls back alone. Nothing commits.
    pub(crate) fn decide_all(
        &mut self,
        schema: &Schema,
        changes: &[ChangeSet],
    ) -> Result<Vec<Decided>> {
        self.work.checkpoint()?;
        let inner = &self.store.inner;
        let mut txn = self.store.gated_write_txn(&self.work)?;
        let mut decided = Vec::with_capacity(changes.len());
        for changes in changes {
            let mut nested = inner
                .env
                .nested_write_txn(&mut txn.txn)
                .map_err(|error| inner.txn_error(error))?;
            let rows = apply_rows(inner, &mut nested, changes, &self.work)?;
            let state = CandidateState {
                inner,
                txn: &nested,
                changes,
                home_keys_preserved: rows.home_keys_preserved,
            };
            let judgment = super::judge_bridge::judge_candidate(schema, &state, &self.work)?;
            if judgment == Judgment::Admitted {
                nested.commit().map_err(|error| inner.txn_error(error))?;
            }
            decided.push(Decided {
                applied: rows.applied,
                judgment,
            });
        }
        Ok(decided)
    }

    /// A transaction against the unchanged parent for host records only.
    pub(crate) fn prepare_unchanged<'owner>(
        &'owner mut self,
    ) -> Result<PreparedWrite<'owner, 'store>> {
        let txn = self.store.gated_write_txn(&self.work)?;
        let parent = read_generation(&self.store.inner, &txn.txn)?;
        Ok(PreparedWrite {
            owner: self,
            txn,
            parent,
            applied: Vec::new(),
        })
    }
}

/// The proposed final state, readable by judgment only: every read comes
/// from the candidate transaction itself.
pub(crate) struct CandidateState<'a> {
    inner: &'a StoreInner,
    txn: &'a RoTxn<'a, heed::AnyTls>,
    changes: &'a ChangeSet,
    home_keys_preserved: bool,
}

impl<'a> CandidateState<'a> {
    pub(crate) fn changes(&self) -> &'a ChangeSet {
        self.changes
    }

    /// No insertion met a different row in its home bucket, so a scalar home
    /// key the lawful parent satisfied still holds.
    pub(crate) fn preserves_home_key(&self, statement: StatementId) -> bool {
        self.home_keys_preserved
            && self
                .inner
                .det
                .projection_of(statement)
                .is_some_and(|projection| {
                    projection.interval_field().is_none() && self.inner.det.is_home(projection)
                })
    }

    /// Proposed rows of one relation in key order, ranked by ordinal.
    pub(crate) fn rows(
        &self,
        relation: RelationId,
    ) -> Result<impl Iterator<Item = Result<(RowId, &'a [u8])>> + use<'a>> {
        Ok(rows::scan(self.inner, self.txn, relation)?
            .map(|entry| entry.map(|(key, bytes)| (key.row, bytes))))
    }

    /// Candidates of one determinant group in the proposed state.
    pub(crate) fn visit_determinant_bucket(
        &self,
        compiled: &CompiledProjection,
        projected: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(RowId, &'a [u8]) -> Result<bool>,
    ) -> Result<()> {
        let routing = rows::routing(self.inner, compiled, projected)?;
        rows::visit_bucket(self.inner, self.txn, compiled, &routing, work, visit)
    }
}

/// An admitted or decided candidate: owns the uncommitted transaction.
pub(crate) struct PreparedWrite<'owner, 'store> {
    owner: &'owner mut WriteOwner<'store>,
    txn: GatedRwTxn<'store>,
    parent: GenerationId,
    applied: Vec<Applied>,
}

impl<'owner, 'store> PreparedWrite<'owner, 'store> {
    pub(crate) fn applied_each(&self) -> &[Applied] {
        &self.applied
    }

    pub(crate) fn applied(&self) -> Applied {
        self.applied
            .iter()
            .fold(Applied::default(), |sum, each| Applied {
                added: sum.added + each.added,
                removed: sum.removed + each.removed,
            })
    }

    /// Seal host records and the head into the same transaction; the
    /// generation advances once when facts or host bytes changed.
    pub(crate) fn seal(mut self, host: HostChanges<'_>) -> Result<SealedWrite<'owner, 'store>> {
        let inner = &self.owner.store.inner;
        let host_mutated = super::host::apply(inner, &mut self.txn.txn, host, &self.owner.work)?;
        let changed = host_mutated || self.applied.iter().any(|applied| applied.changed());
        let generation = if changed {
            let next = self
                .parent
                .value()
                .checked_add(1)
                .map(GenerationId::from_storage)
                .ok_or(Error::Exhausted(crate::error::Counter::Generations))?;
            inner
                .meta
                .put(
                    &mut self.txn.txn,
                    K_GENERATION,
                    &next.storage_word().to_be_bytes(),
                )
                .map_err(|error| inner.txn_error(error))?;
            next
        } else {
            self.parent
        };
        Ok(SealedWrite {
            _owner: self.owner,
            txn: self.txn,
            generation,
            changed,
        })
    }

    pub(crate) fn abort(self) {
        drop(self.txn);
    }
}

/// The committed outcome. `changed` covers facts or host bytes; the
/// generation moved exactly when it is true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Commit {
    pub generation: GenerationId,
    pub changed: bool,
}

/// Commit or abort only; facts and host bytes are frozen.
pub(crate) struct SealedWrite<'owner, 'store> {
    _owner: &'owner mut WriteOwner<'store>,
    txn: GatedRwTxn<'store>,
    generation: GenerationId,
    changed: bool,
}

impl SealedWrite<'_, '_> {
    pub(crate) fn commit(self) -> Result<Commit> {
        let commit = Commit {
            generation: self.generation,
            changed: self.changed,
        };
        self.txn.commit()?;
        Ok(commit)
    }

    pub(crate) fn abort(self) {
        drop(self.txn);
    }
}

#[cfg(test)]
impl WriteOwner<'_> {
    /// Apply `changes` in a transaction that is then dropped; returns the
    /// incremental verdict and the complete verdict over the same state.
    pub(crate) fn judge_both(
        &mut self,
        schema: &Schema,
        changes: &ChangeSet,
    ) -> Result<(Judgment, Judgment)> {
        let inner = &self.store.inner;
        let mut txn = self.store.gated_write_txn(&self.work)?;
        let rows = apply_rows(inner, &mut txn.txn, changes, &self.work)?;
        let state = CandidateState {
            inner,
            txn: &txn.txn,
            changes,
            home_keys_preserved: rows.home_keys_preserved,
        };
        Ok((
            super::judge_bridge::judge_candidate(schema, &state, &self.work)?,
            super::judge_bridge::judge_candidate_complete(schema, &state, &self.work)?,
        ))
    }
}
