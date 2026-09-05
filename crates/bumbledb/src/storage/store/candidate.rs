//! Private candidate: prepare/admit → opaque seal → commit/abort.
//!
//! The candidate is one uncommitted LMDB write transaction on its owning
//! worker. Existing committed readers never observe it; a losing candidate
//! is dropped, never readable. Judgment (C03, produced by P01) sees the
//! **proposed final state** — the transaction's own view, where the
//! determinant namespace is a multimap holding every competing proposal —
//! before any decision, so unique-index installation order cannot hide a
//! second conflicting row (ENG-005's physical precondition).
//!
//! ```compile_fail
//! fn require_competing_rows() {
//!     let _: bumbledb::store::CompetingRows = Vec::new();
//! }
//! ```
//!
//! `seal` writes only opaque host records and the attachment (C04): it can
//! never amend judged application facts, so sealing cannot invalidate the
//! admission evidence. A failed seal drops the entire transaction —
//! including any host-record prefix — and dispatches nothing. After seal,
//! the only capabilities are commit and abort.
//!
//! `MDB_MAP_FULL` while preparing aborts the transaction, grows the map
//! under the exclusive gate, and **reapplies the same owned canonical
//! delta** — immutable native work, never an application callback. The loop
//! is bounded by actual growth progress: `grow` either strictly grows or
//! returns a typed refusal. Map-full during seal or commit surfaces as a
//! typed error with nothing dispatched/committed; the caller (the log)
//! owns replay of its immutable attempt after `Store::grow`.

use std::sync::MutexGuard;

use bumbledb_theory::schema::{RelationId, StatementId};
use heed::RoTxn;

use crate::schema::ProjectionId;

use super::error::{HostKeyFault, StoreCorruption, StoreError, StoreResult};
use super::format::{K_ATTACHMENT, K_GENERATION, K_HOST_RECORD_TAG, RowId};
use super::host::{AttachmentChange, HostChanges, HostRecordChange};
use super::keys::HOST_KEY_MAX;
use super::rows;
use super::store_env::{GatedRwTxn, Store, map_txn_error, read_generation};
use crate::Value;
use crate::changes::{ChangeKind, ChangeSet};
use crate::storage::GenerationId;
use crate::work::WorkContext;

/// Net application-fact changes of one candidate, independent of
/// metadata-only generation movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppliedChanges {
    pub added: u64,
    pub removed: u64,
}

/// The committed outcome. `changed` covers facts **or** host records; the
/// generation moved exactly when it is true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreCommit {
    pub generation: GenerationId,
    pub application: AppliedChanges,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitKind {
    Changed { new_generation: GenerationId },
    Noop { generation: GenerationId },
}

impl CommitKind {
    fn generation(self) -> GenerationId {
        match self {
            Self::Changed { new_generation } => new_generation,
            Self::Noop { generation } => generation,
        }
    }
}

/// C01/C03 seam: interned projections a stored row participates in, as
/// (`ProjectionId`, routing bytes, optional interval tail). Shared physical
/// indexes emit once. The store fingerprints and maintains the entries;
/// projection semantics stay with the schema owner.
pub trait RowIndexer {
    /// # Errors
    /// Propagates work exhaustion or the emit sink's storage failure.
    fn index_row(
        &self,
        relation: RelationId,
        row: &[u8],
        work: &WorkContext,
        emit: &mut dyn FnMut(ProjectionId, &[u8], Option<&[u8]>) -> StoreResult<()>,
    ) -> StoreResult<()>;
}

/// C03 seam, produced by P01: judge the proposed final state before any
/// commit capability exists. A completed semantic rejection carries the
/// producer's evidence type; a resource failure is a `StoreError`, never a
/// fabricated rejection.
pub trait CandidateJudge {
    type Rejection;

    /// # Errors
    /// Resource/storage failure only; a domain rejection is a `Judgment`.
    fn judge(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>>;
}

#[derive(Debug)]
pub enum Judgment<R> {
    Admitted,
    Rejected(R),
}

/// Outcome of `prepare`: an owned prepared capability, or the judge's
/// completed rejection with the writer session retained.
pub enum Prepared<'owner, 'store, R> {
    Admitted(PreparedWrite<'owner, 'store>),
    Rejected(R),
}

impl<R: std::fmt::Debug> std::fmt::Debug for Prepared<'_, '_, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admitted(prepared) => write!(
                f,
                "Prepared::Admitted(generation {})",
                prepared.proposed_generation()
            ),
            Self::Rejected(rejection) => write!(f, "Prepared::Rejected({rejection:?})"),
        }
    }
}

/// The exclusive writer session. `!Send`/`!Sync` (mutex guard); it stays on
/// its owning worker across a hosted publication attempt. A domain
/// rejection or aborted candidate leaves the session owned, so the log can
/// prepare its receipt-only transaction against the unchanged parent with
/// no gap for another local writer.
pub struct WriteOwner<'store> {
    store: &'store Store,
    work: WorkContext,
    _guard: MutexGuard<'store, ()>,
}

impl std::fmt::Debug for WriteOwner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WriteOwner({})", self.store.store_id())
    }
}

impl Drop for WriteOwner<'_> {
    fn drop(&mut self) {
        self.store.release_writer_thread();
    }
}

impl<'store> WriteOwner<'store> {
    pub(crate) fn new(
        store: &'store Store,
        guard: MutexGuard<'store, ()>,
        work: WorkContext,
    ) -> Self {
        Self {
            store,
            work,
            _guard: guard,
        }
    }

    /// The committed parent generation, read while exclusivity is held.
    /// Gated: the read transaction must not race an exclusive resize.
    /// # Errors
    /// Storage failure or stopped work.
    pub fn parent_generation(&self) -> StoreResult<GenerationId> {
        let _pass = self.store.inner.gate.enter(&self.work)?;
        let txn = self
            .store
            .inner
            .env
            .read_txn()
            .map_err(StoreError::from_heed)?;
        read_generation(&self.store.inner, &txn)
    }

    /// Prepare and judge one sealed canonical delta as a private candidate
    /// through a custom [`CandidateJudge`]. Production admitted writes use
    /// [`Self::prepare_incremental`] instead.
    /// # Errors
    /// `ForeignSchema`, malformed change bytes, growth refusals, storage
    /// failure or stopped work. A judge rejection is a `Prepared::Rejected`,
    /// not an error, and retains this session.
    pub fn prepare<'owner, I, J>(
        &'owner mut self,
        changes: &ChangeSet,
        indexer: &I,
        judge: &J,
    ) -> StoreResult<Prepared<'owner, 'store, J::Rejection>>
    where
        I: RowIndexer + ?Sized,
        J: CandidateJudge + ?Sized,
    {
        self.prepare_judged(changes, indexer, |state, work| judge.judge(state, work))
    }

    /// Ordinary write on an already-admitted store: apply the delta, then
    /// incremental judgment under a real [`crate::schema::judge::LawfulParent`].
    /// Never minted from [`super::staging::UnreadyStore`]. An empty
    /// `ChangeSet` is a no-op under that parent, not complete validation.
    /// # Errors
    /// As [`Self::prepare`].
    pub fn prepare_incremental<'owner, I>(
        &'owner mut self,
        parent: crate::schema::judge::LawfulParent,
        changes: &ChangeSet,
        indexer: &I,
        judge: &super::judge_bridge::SchemaJudge<'_>,
    ) -> StoreResult<Prepared<'owner, 'store, Box<[crate::schema::judge::JudgedViolation]>>>
    where
        I: RowIndexer + ?Sized,
    {
        self.prepare_judged(changes, indexer, |state, work| {
            judge.judge_incremental(parent, state, work)
        })
    }

    fn prepare_judged<'owner, I, R>(
        &'owner mut self,
        changes: &ChangeSet,
        indexer: &I,
        mut decide: impl FnMut(&CandidateState<'_, '_>, &WorkContext) -> StoreResult<Judgment<R>>,
    ) -> StoreResult<Prepared<'owner, 'store, R>>
    where
        I: RowIndexer + ?Sized,
    {
        if changes.schema() != self.store.inner.schema_fp {
            return Err(StoreError::ForeignSchema);
        }
        loop {
            self.work.checkpoint()?;
            match self.attempt(changes, indexer) {
                Err(StoreError::MapFull { .. }) => {
                    // The failed transaction is already dropped. Grow under
                    // the exclusive gate and reapply the same owned delta;
                    // `grow` strictly grows or returns the typed refusal
                    // that bounds this loop.
                    self.store.grow(&self.work, None)?;
                }
                Err(error) => return Err(error),
                Ok((txn, report, application)) => {
                    let state = CandidateState {
                        store: self.store,
                        txn: &txn,
                        changes: Some(changes),
                    };
                    match decide(&state, &self.work)? {
                        Judgment::Rejected(rejection) => {
                            drop(txn); // the losing candidate is never readable
                            return Ok(Prepared::Rejected(rejection));
                        }
                        Judgment::Admitted => {
                            return Ok(Prepared::Admitted(PreparedWrite {
                                owner: self,
                                txn,
                                report,
                                application,
                            }));
                        }
                    }
                }
            }
        }
    }

    /// Apply a sealed delta without judgment. Only the unready population
    /// path may call this; readiness is [`super::staging::UnreadyStore::admit`].
    pub(crate) fn ingest<I: RowIndexer + ?Sized>(
        &mut self,
        changes: &ChangeSet,
        indexer: &I,
    ) -> StoreResult<StoreCommit> {
        if changes.schema() != self.store.inner.schema_fp {
            return Err(StoreError::ForeignSchema);
        }
        loop {
            self.work.checkpoint()?;
            match self.attempt(changes, indexer) {
                Err(StoreError::MapFull { .. }) => {
                    self.store.grow(&self.work, None)?;
                }
                Err(error) => return Err(error),
                Ok((txn, report, application)) => {
                    return PreparedWrite {
                        owner: self,
                        txn,
                        report,
                        application,
                    }
                    .seal(HostChanges {
                        records: &[],
                        attachment: AttachmentChange::Keep,
                    })?
                    .commit();
                }
            }
        }
    }

    /// A metadata-only transaction against the unchanged committed parent:
    /// the rejection-receipt / no-op decision path. No application fact can
    /// change through it; sealing host records may still advance the
    /// generation.
    /// # Errors
    /// Storage failure or stopped work.
    pub fn prepare_unchanged<'owner>(
        &'owner mut self,
    ) -> StoreResult<PreparedWrite<'owner, 'store>> {
        loop {
            self.work.checkpoint()?;
            let txn = match self.store.gated_write_txn(&self.work) {
                Err(StoreError::MapFull { .. }) => {
                    self.store.grow(&self.work, None)?;
                    continue;
                }
                other => other?,
            };
            let generation = read_generation(&self.store.inner, &txn.txn)?;
            return Ok(PreparedWrite {
                owner: self,
                txn,
                report: CommitKind::Noop { generation },
                application: AppliedChanges::default(),
            });
        }
    }

    fn attempt<I: RowIndexer + ?Sized>(
        &self,
        changes: &ChangeSet,
        indexer: &I,
    ) -> StoreResult<(GatedRwTxn<'store>, CommitKind, AppliedChanges)> {
        let inner = &self.store.inner;
        let mut gated = self.store.gated_write_txn(&self.work)?;
        let mut application = AppliedChanges::default();
        // The relations this candidate actually changed (a staged add of an
        // already-present row or a remove of an absent one changes nothing):
        // exactly these advance their per-relation change version below, so
        // an untouched relation's image memos stay provably reusable.
        let mut changed_relations: std::collections::BTreeSet<RelationId> =
            std::collections::BTreeSet::new();
        // Deletes before inserts: the one-command tie rule is already
        // normalized inside the sealed ChangeSet (add wins); physical order
        // here cannot change the final state.
        for kind in [ChangeKind::Remove, ChangeKind::Add] {
            for record in changes.records().filter(|record| record.kind == kind) {
                self.work.step(1)?;
                let relation = record.relation;
                match kind {
                    ChangeKind::Remove => {
                        if rows::remove_row(
                            inner,
                            &mut gated.txn,
                            relation,
                            record.row,
                            indexer,
                            &self.work,
                        )? {
                            application.removed += 1;
                            changed_relations.insert(relation);
                        }
                    }
                    ChangeKind::Add => {
                        if rows::insert_row(
                            inner,
                            &mut gated.txn,
                            relation,
                            record.row,
                            indexer,
                            &self.work,
                        )?
                        .is_some()
                        {
                            application.added += 1;
                            changed_relations.insert(relation);
                        }
                    }
                }
            }
        }
        let parent = read_generation(inner, &gated.txn)?;
        let report = if application.added + application.removed > 0 {
            let new_generation = next_generation(parent)?;
            inner
                .meta
                .put(
                    &mut gated.txn,
                    K_GENERATION,
                    &new_generation.storage_word().to_be_bytes(),
                )
                .map_err(map_txn_error)?;
            // Advance exactly the touched relations' change versions, in the
            // same transaction as the rows they cover. Host-record-only
            // seals never reach this arm (they change no relation's rows).
            for relation in &changed_relations {
                self.work.step(1)?;
                let next = super::format::read_relation_version(&inner.meta, &gated.txn, *relation)?
                    .next()?;
                inner
                    .meta
                    .put(
                        &mut gated.txn,
                        super::format::relation_version_key(*relation).as_slice(),
                        &next.storage_word().to_be_bytes(),
                    )
                    .map_err(map_txn_error)?;
            }
            CommitKind::Changed { new_generation }
        } else {
            CommitKind::Noop { generation: parent }
        };
        Ok((gated, report, application))
    }
}

fn next_generation(parent: GenerationId) -> StoreResult<GenerationId> {
    parent
        .value()
        .checked_add(1)
        .map(GenerationId::from_storage)
        .ok_or(StoreError::GenerationExhausted)
}

/// The proposed final state, exposed to judgment only. Every read comes
/// from the candidate transaction itself; committed readers elsewhere still
/// see the parent snapshot.
pub struct CandidateState<'a, 'store> {
    store: &'store Store,
    txn: &'a GatedRwTxn<'store>,
    changes: Option<&'a ChangeSet>,
}

impl CandidateState<'_, '_> {
    /// Committed populated state with no delta — complete judgment only.
    pub(crate) fn of_committed<'a, 'store>(
        store: &'store Store,
        txn: &'a GatedRwTxn<'store>,
    ) -> CandidateState<'a, 'store> {
        CandidateState {
            store,
            txn,
            changes: None,
        }
    }

    fn read_txn(&self) -> &RoTxn<'_, heed::AnyTls> {
        &self.txn.txn
    }

    /// The sealed delta under judgment (absent for metadata-only
    /// transactions, which propose no fact changes).
    #[must_use]
    pub fn changes(&self) -> Option<&ChangeSet> {
        self.changes
    }

    /// Proposed final rows of one relation, in local row-id order.
    /// # Errors
    /// Storage failure.
    pub fn rows(
        &self,
        relation: RelationId,
    ) -> StoreResult<impl Iterator<Item = StoreResult<(RowId, &[u8])>>> {
        rows::scan_rows(&self.store.inner, self.read_txn(), relation)
    }

    /// Exact membership in the proposed final state.
    /// # Errors
    /// Storage failure or stopped work.
    pub fn contains(
        &self,
        relation: RelationId,
        row: &[u8],
        work: &WorkContext,
    ) -> StoreResult<bool> {
        Ok(rows::exact_lookup(&self.store.inner, self.read_txn(), relation, row, work)?.is_some())
    }

    /// All candidate row ids sharing one determinant bucket (ids only).
    /// # Errors
    /// Storage failure or stopped work.
    pub fn determinant_candidates(
        &self,
        projection: ProjectionId,
        projected: &[u8],
        work: &WorkContext,
    ) -> StoreResult<Vec<RowId>> {
        rows::determinant_bucket_ids(
            &self.store.inner,
            self.read_txn(),
            projection,
            projected,
            work,
        )
    }

    /// Bounded visitor over one determinant bucket in the proposed final
    /// state. The visitor receives each `(row id, canonical row bytes)`;
    /// return `false` to stop early.
    /// # Errors
    /// Storage failure, stopped work, or visitor failure.
    pub fn visit_determinant_bucket(
        &self,
        projection: ProjectionId,
        projected: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(RowId, &[u8]) -> StoreResult<bool>,
    ) -> StoreResult<()> {
        let inner = &self.store.inner;
        let Some(key) = inner.det.projection(projection) else {
            return Ok(());
        };
        let routing = rows::routing_for_projected(inner, projection, projected)?;
        rows::visit_determinant_bucket(
            inner,
            self.read_txn(),
            projection,
            &routing,
            work,
            &mut |id| {
                let bytes = rows::fetch_row(inner, self.read_txn(), key.relation, id)?
                    .ok_or(StoreError::Corruption(StoreCorruption::DanglingIndexEntry))?;
                visit(id, bytes)
            },
        )
    }

    /// Bounded visitor over confirmed determinant-group competitors.
    /// Charged decoded rows are borrowed for the visit; there is no owning
    /// `CompetingRows` / `into_values` extraction. Returns `None` when the
    /// statement is not a sealed key.
    /// # Errors
    /// Storage failure or stopped work.
    pub fn visit_determinant_competitors(
        &self,
        statement: StatementId,
        determinant: &[Value],
        work: &WorkContext,
        visit: &mut dyn FnMut(u64, &[Value]) -> StoreResult<bool>,
    ) -> StoreResult<Option<()>> {
        let inner = &self.store.inner;
        let Some(key) = inner.det.projection_of(statement) else {
            return Ok(None);
        };
        let projected = super::det_index::determinant_bytes(key, determinant, work)?;
        let fields = inner
            .det
            .fields_of(key.relation)
            .ok_or(StoreError::ForeignSchema)?;
        self.visit_determinant_bucket(key.id, &projected, work, &mut |id, bytes| {
            work.step(1)?;
            let decoded = crate::canonical::decode(fields, bytes, work)?;
            if key.scalar_values(decoded.values()).as_slice() == determinant {
                visit(id.0, decoded.values())
            } else {
                Ok(true)
            }
        })?;
        Ok(Some(()))
    }

    /// Fetch one proposed row's canonical bytes by local id.
    /// # Errors
    /// Storage failure.
    pub fn fetch(&self, relation: RelationId, row: RowId) -> StoreResult<Option<&[u8]>> {
        rows::fetch_row(&self.store.inner, self.read_txn(), relation, row)
    }

    /// Live row count of one relation in the proposed final state.
    /// # Errors
    /// Storage failure.
    pub fn row_count(&self, relation: RelationId) -> StoreResult<u64> {
        rows::row_count(&self.store.inner, self.read_txn(), relation)
    }
}

/// An admitted private candidate: owns the uncommitted transaction and its
/// admission evidence. Not clonable; exposes no committed read capability;
/// `!Send`/`!Sync` through the transaction and owner borrow.
pub struct PreparedWrite<'owner, 'store> {
    owner: &'owner mut WriteOwner<'store>,
    txn: GatedRwTxn<'store>,
    report: CommitKind,
    application: AppliedChanges,
}

impl<'owner, 'store> PreparedWrite<'owner, 'store> {
    #[must_use]
    pub fn application_changes(&self) -> AppliedChanges {
        self.application
    }

    #[must_use]
    pub fn proposed_generation(&self) -> GenerationId {
        self.report.generation()
    }

    /// Seal opaque host records and the attachment into the same
    /// transaction. Only host bytes can change — never application facts or
    /// indexes — so admission evidence stays valid. Any failure (including
    /// map-full) consumes the capability and drops the entire private
    /// transaction: nothing was dispatched, nothing committed.
    /// # Errors
    /// Host-key grammar violations, growth exhaustion, storage failure or
    /// stopped work.
    pub fn seal(mut self, host: HostChanges<'_>) -> StoreResult<SealedWrite<'owner, 'store>> {
        let work = self.owner.work.clone();
        let mutated = apply_host_changes(self.owner.store, &mut self.txn, host, &work)?;
        if mutated && let CommitKind::Noop { generation } = self.report {
            let new_generation = next_generation(generation)?;
            self.owner
                .store
                .inner
                .meta
                .put(
                    &mut self.txn.txn,
                    K_GENERATION,
                    &new_generation.storage_word().to_be_bytes(),
                )
                .map_err(map_txn_error)?;
            self.report = CommitKind::Changed { new_generation };
        }
        Ok(SealedWrite {
            _owner: self.owner,
            txn: self.txn,
            report: self.report,
            application: self.application,
        })
    }

    /// Drop the candidate. Committed state is untouched; the writer session
    /// is retained by the owner.
    pub fn abort(self) {
        drop(self.txn);
    }
}

/// Sealed: one LMDB durability point for facts, generation, host records
/// and attachment together. Commit or abort only.
pub struct SealedWrite<'owner, 'store> {
    _owner: &'owner mut WriteOwner<'store>,
    txn: GatedRwTxn<'store>,
    report: CommitKind,
    application: AppliedChanges,
}

impl SealedWrite<'_, '_> {
    /// # Errors
    /// Local durability failure, reported without any claim about a remote
    /// publication; a hosted caller preserves its already-known receipt.
    pub fn commit(self) -> StoreResult<StoreCommit> {
        let report = self.report;
        let application = self.application;
        self.txn.commit()?;
        Ok(StoreCommit {
            generation: report.generation(),
            application,
            changed: matches!(report, CommitKind::Changed { .. }),
        })
    }

    pub fn abort(self) {
        drop(self.txn);
    }
}

const BYTE_QUANTUM: usize = rows::BYTE_QUANTUM;

fn host_key<'k>(key: &[u8], buffer: &'k mut [u8; 1 + HOST_KEY_MAX]) -> StoreResult<&'k [u8]> {
    if key.len() > HOST_KEY_MAX {
        return Err(StoreError::HostKey(HostKeyFault::TooLong {
            actual: key.len(),
        }));
    }
    buffer[0] = K_HOST_RECORD_TAG;
    buffer[1..=key.len()].copy_from_slice(key);
    Ok(&buffer[..=key.len()])
}

fn same_value(existing: Option<&[u8]>, proposed: &[u8], work: &WorkContext) -> StoreResult<bool> {
    match existing {
        Some(existing) => rows::chunked_eq(existing, proposed, work),
        None => Ok(false),
    }
}

fn validate_host(host: &HostChanges<'_>, work: &WorkContext) -> StoreResult<()> {
    work.checkpoint()?;
    let mut previous: Option<&[u8]> = None;
    for record in host.records {
        work.step(1)?;
        let key = match *record {
            HostRecordChange::Put { key, value } => {
                work.input(value.len() as u64)?;
                key
            }
            HostRecordChange::Delete { key } => key,
        };
        if key.len() > HOST_KEY_MAX {
            return Err(StoreError::HostKey(HostKeyFault::TooLong {
                actual: key.len(),
            }));
        }
        work.input(key.len() as u64)?;
        if previous.is_some_and(|previous| previous >= key) {
            return Err(StoreError::HostKey(HostKeyFault::NotStrictlyOrdered));
        }
        previous = Some(key);
    }
    if let AttachmentChange::Put(bytes) = host.attachment {
        work.input(bytes.len() as u64)?;
    }
    Ok(())
}

fn put_chunked(
    store: &Store,
    txn: &mut GatedRwTxn<'_>,
    key: &[u8],
    value: &[u8],
    work: &WorkContext,
) -> StoreResult<()> {
    use std::io::Write as _;
    work.checkpoint()?;
    let mut stopped = None;
    let result = store
        .inner
        .meta
        .put_reserved(&mut txn.txn, key, value.len(), |space| {
            for chunk in value.chunks(BYTE_QUANTUM) {
                work.step(chunk.len() as u64).map_err(|error| {
                    stopped = Some(error);
                    std::io::Error::from(std::io::ErrorKind::Interrupted)
                })?;
                space.write_all(chunk)?;
            }
            Ok(())
        });
    if let Some(error) = stopped {
        return Err(StoreError::Work(error));
    }
    result.map_err(map_txn_error)
}

fn apply_host_changes(
    store: &Store,
    txn: &mut GatedRwTxn<'_>,
    host: HostChanges<'_>,
    work: &WorkContext,
) -> StoreResult<bool> {
    validate_host(&host, work)?;
    let mut buffer = [0u8; 1 + HOST_KEY_MAX];
    let mut mutated = false;
    for (index, record) in host.records.iter().enumerate() {
        work.step(1)?;
        #[cfg(not(test))]
        let _ = index;
        #[cfg(test)]
        if *store
            .inner
            .fail_host_after
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            == Some(index)
        {
            return Err(StoreError::MapFull {
                map_bytes: store.current_map_bytes(),
            });
        }
        match *record {
            HostRecordChange::Put { key, value } => {
                let key = host_key(key, &mut buffer)?;
                let existing = store
                    .inner
                    .meta
                    .get(&txn.txn, key)
                    .map_err(StoreError::from_heed)?;
                if !same_value(existing, value, work)? {
                    put_chunked(store, txn, key, value, work)?;
                    mutated = true;
                }
            }
            HostRecordChange::Delete { key } => {
                let key = host_key(key, &mut buffer)?;
                mutated |= store
                    .inner
                    .meta
                    .delete(&mut txn.txn, key)
                    .map_err(map_txn_error)?;
            }
        }
    }
    work.step(1)?;
    match host.attachment {
        AttachmentChange::Keep => {}
        AttachmentChange::Put(bytes) => {
            let existing = store
                .inner
                .meta
                .get(&txn.txn, K_ATTACHMENT)
                .map_err(StoreError::from_heed)?;
            if !same_value(existing, bytes, work)? {
                put_chunked(store, txn, K_ATTACHMENT, bytes, work)?;
                mutated = true;
            }
        }
        AttachmentChange::Clear => {
            mutated |= store
                .inner
                .meta
                .delete(&mut txn.txn, K_ATTACHMENT)
                .map_err(map_txn_error)?;
        }
    }
    work.checkpoint()?;
    Ok(mutated)
}
