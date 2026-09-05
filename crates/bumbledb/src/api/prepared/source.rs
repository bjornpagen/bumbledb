//! The query lane's one storage seam (C05 consuming C04): every prepare,
//! statistic, image build, key probe, cursor fallback and result copy reads
//! committed rows through this enum — an owned coherent LMDB snapshot
//! ([`OwnedSnapshot`], one real read transaction) or an admitted heap
//! instance's sorted canonical rows. Closed relations never reach a source:
//! they synthesize from the schema's sealed extension.
//!
//! Identity discipline: a prepared query pins its source identity at
//! prepare. Store sources carry the store+environment identity
//! ([`StoreIdentity`]); executing against any other environment's snapshot
//! is `Error::ForeignPreparedQuery` before any work. Heap instances carry
//! no durable identity, so a heap-prepared query never memoizes images
//! across executions (the `ViewEpoch::Heap` tick) — correctness never
//! rides on an address comparison.

use crate::error::{Error, Result};
use crate::image::ViewEpoch;
use crate::schema::{
    CompiledProjection, CompiledTheory, DistinctnessWitness, Schema, VisitControl, VisitOutcome,
};
use crate::storage::store::{OwnedSnapshot, StoreError, StoreIdentity};
use crate::work::{ExecutionPolicy, WorkContext, WorkError};
use bumbledb_theory::schema::RelationId;
use std::cell::Cell;

/// Test-only unbounded policy — never used on production query paths.
#[cfg(test)]
pub(crate) const UNBOUNDED_POLICY: ExecutionPolicy = ExecutionPolicy {
    input_bytes: u64::MAX,
    working_bytes: u64::MAX,
    scratch_bytes: u64::MAX,
    result_bytes: u64::MAX,
    rows: u64::MAX,
    work_units: u64::MAX,
    timeout: std::time::Duration::from_hours(24 * 365),
};

/// # Errors
/// Only an invalid timeout, which the constant policy cannot produce in
/// practice; kept fallible so callers stay in the one ledger constructor.
#[cfg(test)]
pub(crate) fn unbounded_work() -> std::result::Result<WorkContext, WorkError> {
    UNBOUNDED_POLICY.start()
}

/// Finite default allowance for heap-instance prepare/execute paths that
/// carry no session lease (explicit bounded operation, never unlimited).
pub(crate) fn heap_default_work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 256 << 20,
        working_bytes: 256 << 20,
        scratch_bytes: 256 << 20,
        result_bytes: 256 << 20,
        rows: 1 << 24,
        work_units: 1 << 24,
        timeout: std::time::Duration::from_secs(3600),
    }
    .start()
    .expect("valid heap default policy")
}

pub(crate) fn work_error(error: WorkError) -> Error {
    Error::from_store(StoreError::Work(error))
}

pub(crate) fn store_error(error: StoreError) -> Error {
    Error::from_store(error)
}

/// Heap row access, type-erased over the instance's schema typestate.
/// Implemented for [`crate::api::db::OwnedInstance`] here (the query lane
/// owns its consumption; the instance's file is not edited).
pub(crate) trait HeapRows {
    /// Sorted canonical rows of one ordinary relation (empty for closed
    /// or unpopulated relations).
    fn rows(&self, relation: RelationId) -> &[Box<[u8]>];
}

impl<S> HeapRows for crate::api::db::OwnedInstance<S> {
    fn rows(&self, relation: RelationId) -> &[Box<[u8]>] {
        self.relation_rows(relation)
    }
}

/// What a prepared query pinned at prepare — checked at every execution
/// entry before any bind or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PinnedSource {
    Store(StoreIdentity),
    /// Heap-prepared: no durable identity exists; every execution rebuilds
    /// its images from the instance it was handed (never a memo hit).
    Heap,
}

/// Resident image positions are `u32`. Crossing this bound on the same
/// pinned snapshot selects the cursor fallback before COLT/image build
/// (CORE-010). One clean resident→disk restart is permitted afterward.
pub(crate) const RESIDENT_ROW_LIMIT: u64 = u32::MAX as u64;

/// One execution's row source.
pub(crate) enum QuerySource<'a> {
    Store {
        snapshot: &'a OwnedSnapshot,
        work: &'a WorkContext,
        visits: Cell<usize>,
    },
    Heap {
        rows: &'a dyn HeapRows,
        work: WorkContext,
        /// The prepared query's per-execution heap tick (epochs below).
        tick: u64,
        visits: Cell<usize>,
    },
}

impl<'a> QuerySource<'a> {
    pub(crate) fn store(snapshot: &'a OwnedSnapshot, work: &'a WorkContext) -> Self {
        Self::Store {
            snapshot,
            work,
            visits: Cell::new(0),
        }
    }

    /// One execution's row source on a heap instance under the caller's
    /// work policy (heap instances carry no durable identity).
    pub(crate) fn heap(rows: &'a dyn HeapRows, tick: u64, work: WorkContext) -> Self {
        Self::Heap {
            rows,
            work,
            tick,
            visits: Cell::new(0),
        }
    }

    pub(crate) fn work(&self) -> &WorkContext {
        match self {
            Self::Store { work, .. } => work,
            Self::Heap { work, .. } => work,
        }
    }

    /// Actual source-row visits this execution has charged (D10).
    #[must_use]
    pub(crate) fn visit_count(&self) -> usize {
        match self {
            Self::Store { visits, .. } | Self::Heap { visits, .. } => visits.get(),
        }
    }

    fn note_visits(&self, n: usize) {
        match self {
            Self::Store { visits, .. } | Self::Heap { visits, .. } => {
                visits.set(visits.get().saturating_add(n));
            }
        }
    }

    /// True when a resident image/COLT would overflow the `u32` position
    /// regime — fallback must be selected before that build.
    pub(crate) fn exceeds_resident_positions(&self, relation: RelationId) -> Result<bool> {
        Ok(self.row_count(relation)? >= RESIDENT_ROW_LIMIT)
    }

    pub(crate) fn pinned(&self) -> PinnedSource {
        match self {
            Self::Store { snapshot, .. } => PinnedSource::Store(snapshot.identity()),
            Self::Heap { .. } => PinnedSource::Heap,
        }
    }

    /// The view-validity epoch of one ordinary relation on this source: the
    /// relation's committed change version on a store snapshot (one small
    /// meta read from the snapshot's own transaction — an unrelated write
    /// leaves it equal, so untouched relations keep their images across
    /// generations; PERF-001), or the per-execution heap tick.
    /// # Errors
    /// Storage failure reading the version.
    pub(crate) fn relation_epoch(&self, relation: RelationId) -> Result<ViewEpoch> {
        match self {
            Self::Store { snapshot, .. } => Ok(ViewEpoch::Store(
                snapshot.relation_version(relation).map_err(store_error)?,
            )),
            Self::Heap { tick, .. } => Ok(ViewEpoch::Heap(*tick)),
        }
    }

    /// Committed row count of one ordinary relation.
    /// # Errors
    /// Storage failure.
    pub(crate) fn row_count(&self, relation: RelationId) -> Result<u64> {
        match self {
            Self::Store { snapshot, .. } => snapshot.row_count(relation).map_err(store_error),
            Self::Heap { rows, .. } => Ok(rows.rows(relation).len() as u64),
        }
    }

    /// Walk one ordinary relation's canonical row bytes in source order,
    /// charging one work step per row.
    /// # Errors
    /// Storage failure, stopped work, or the sink's failure.
    pub(crate) fn scan(
        &self,
        relation: RelationId,
        sink: &mut dyn FnMut(&[u8]) -> Result<()>,
    ) -> Result<()> {
        self.scan_early(relation, &mut |bytes| sink(bytes).map(|()| true))
    }

    /// As [`Self::scan`], but returning `Ok(false)` from the visitor stops
    /// early without visiting remaining rows.
    /// # Errors
    /// Storage failure, stopped work, or the sink's failure.
    pub(crate) fn scan_early(
        &self,
        relation: RelationId,
        sink: &mut dyn FnMut(&[u8]) -> Result<bool>,
    ) -> Result<()> {
        let work = self.work();
        match self {
            Self::Store { snapshot, .. } => {
                let iterator = snapshot.rows(relation).map_err(store_error)?;
                for entry in iterator {
                    work.step(1).map_err(work_error)?;
                    self.note_visits(1);
                    let (_, bytes) = entry.map_err(store_error)?;
                    if !sink(bytes)? {
                        return Ok(());
                    }
                }
                Ok(())
            }
            Self::Heap { rows, .. } => {
                for row in rows.rows(relation) {
                    work.step(1).map_err(work_error)?;
                    self.note_visits(1);
                    if !sink(row)? {
                        return Ok(());
                    }
                }
                Ok(())
            }
        }
    }

    /// Exact membership of one canonical row: fingerprint bucket plus full
    /// canonical bytes on a store; binary search on a heap instance.
    /// # Errors
    /// Storage failure or stopped work.
    pub(crate) fn contains(&self, relation: RelationId, row: &[u8]) -> Result<bool> {
        match self {
            Self::Store { snapshot, work, .. } => {
                snapshot.contains(relation, row, work).map_err(store_error)
            }
            Self::Heap { rows, work, .. } => {
                work.step(1).map_err(work_error)?;
                Ok(rows
                    .rows(relation)
                    .binary_search_by(|candidate| candidate.as_ref().cmp(row))
                    .is_ok())
            }
        }
    }

    /// Key-bound or existence-only walk through the compiled witness.
    /// Store sources seek [`OwnedSnapshot::visit_projection`] by
    /// [`crate::schema::ProjectionId`]; heap sources apply the same
    /// [`CompiledTheory::consume_visits`] control so visit counts and
    /// early-stop are honest (D10).
    /// # Errors
    /// Storage, work, compile, or visitor failure.
    pub(crate) fn consume_compiled_visits(
        &self,
        schema: &Schema,
        relation: RelationId,
        witness: DistinctnessWitness,
        key_fields: &[bumbledb_theory::schema::FieldId],
        key_words: &[u64],
        visit: &mut dyn FnMut(&[u8]) -> Result<VisitControl>,
    ) -> Result<Option<VisitOutcome>> {
        let theory = schema.compiled_theory().map_err(compile_error)?;
        let projection = compiled_key_projection(theory, relation, key_fields);
        match self {
            Self::Store { snapshot, work, .. } => {
                let Some(compiled) = projection else {
                    return Ok(None);
                };
                Ok(Some(self.visit_store_projection(
                    snapshot,
                    work,
                    compiled,
                    key_fields,
                    key_words,
                    witness,
                    visit,
                )?))
            }
            Self::Heap { .. } => {
                if projection.is_none() {
                    return Ok(None);
                }
                let _ = (key_fields, key_words);
                Ok(Some(self.visit_heap_rows(relation, witness, visit)?))
            }
        }
    }

    fn visit_store_projection(
        &self,
        snapshot: &OwnedSnapshot,
        work: &WorkContext,
        compiled: &CompiledProjection,
        key_fields: &[bumbledb_theory::schema::FieldId],
        key_words: &[u64],
        witness: DistinctnessWitness,
        visit: &mut dyn FnMut(&[u8]) -> Result<VisitControl>,
    ) -> Result<VisitOutcome> {
        let values = key_values_from_words(compiled, key_fields, key_words)?;
        let projected =
            crate::storage::store::det_index::determinant_bytes(compiled, &values, work)
                .map_err(store_error)?;
        let existence_only = matches!(witness, DistinctnessWitness::ExistenceOnly { .. });
        let mut visited = 0usize;
        let mut outcome = VisitOutcome::Exhausted { visited: 0 };
        let mut visit_err: Option<Error> = None;
        snapshot
            .visit_projection(compiled.id, &projected, work, &mut |_id, bytes| {
                if visit_err.is_some() {
                    return Ok(false);
                }
                work.step(1).map_err(StoreError::Work)?;
                visited = visited.saturating_add(1);
                match visit(bytes) {
                    Ok(VisitControl::Continue) => Ok(true),
                    Ok(VisitControl::Sufficient) if existence_only => {
                        outcome = VisitOutcome::Sufficient { visited };
                        Ok(false)
                    }
                    Ok(VisitControl::Sufficient) => Ok(true),
                    Ok(VisitControl::Stop) => {
                        outcome = VisitOutcome::Stopped { visited };
                        Ok(false)
                    }
                    Err(error) => {
                        visit_err = Some(error);
                        Ok(false)
                    }
                }
            })
            .map_err(store_error)?;
        if let Some(error) = visit_err {
            return Err(error);
        }
        if matches!(outcome, VisitOutcome::Exhausted { .. }) {
            outcome = VisitOutcome::Exhausted { visited };
        }
        self.note_visits(visited);
        Ok(outcome)
    }

    fn visit_heap_rows(
        &self,
        relation: RelationId,
        witness: DistinctnessWitness,
        visit: &mut dyn FnMut(&[u8]) -> Result<VisitControl>,
    ) -> Result<VisitOutcome> {
        let Self::Heap { rows, work, .. } = self else {
            return Ok(VisitOutcome::Exhausted { visited: 0 });
        };
        let outcome = CompiledTheory::consume_visits(
            witness,
            rows.rows(relation).iter().map(AsRef::as_ref),
            &mut |bytes| {
                work.step(1).map_err(work_error)?;
                visit(bytes)
            },
        )?;
        self.note_visits(match outcome {
            VisitOutcome::Exhausted { visited }
            | VisitOutcome::Sufficient { visited }
            | VisitOutcome::Stopped { visited } => visited,
        });
        Ok(outcome)
    }
}

fn compiled_key_projection<'a>(
    theory: &'a CompiledTheory,
    relation: RelationId,
    key_fields: &[bumbledb_theory::schema::FieldId],
) -> Option<&'a CompiledProjection> {
    theory.key_projections_of(relation).iter().find_map(|id| {
        let projection = theory.projection(*id)?;
        (projection.projection.as_ref() == key_fields).then_some(projection)
    })
}

fn key_values_from_words(
    compiled: &CompiledProjection,
    key_fields: &[bumbledb_theory::schema::FieldId],
    key_words: &[u64],
) -> Result<Vec<crate::ir::Value>> {
    if compiled.projection.as_ref() != key_fields || key_fields.len() != key_words.len() {
        return Err(Error::Corruption(
            crate::error::CorruptionError::MalformedValue("compiled key width"),
        ));
    }
    let mut values = Vec::with_capacity(compiled.scalar_positions.len());
    for &position in compiled.scalar_positions.iter() {
        let field = compiled.projection[position];
        let idx = key_fields
            .iter()
            .position(|f| *f == field)
            .ok_or_else(|| {
                Error::Corruption(crate::error::CorruptionError::MalformedValue(
                    "compiled key field",
                ))
            })?;
        let ty = compiled
            .scalar_fields
            .get(values.len())
            .map(|f| f.value_type)
            .unwrap_or(bumbledb_theory::schema::ValueType::U64);
        values.push(word_to_value(ty, key_words[idx])?);
    }
    Ok(values)
}

fn word_to_value(
    ty: bumbledb_theory::schema::ValueType,
    word: u64,
) -> Result<crate::ir::Value> {
    use bumbledb_theory::schema::ValueType;
    Ok(match ty {
        ValueType::Bool => crate::ir::Value::Bool(word != 0),
        ValueType::U64 => crate::ir::Value::U64(word),
        ValueType::I64 => crate::ir::Value::I64((word ^ (1 << 63)).cast_signed()),
        ValueType::F64 => crate::ir::Value::F64(
            bumbledb_theory::F64::from_order_key(word).map_err(|_| {
                Error::Corruption(crate::error::CorruptionError::MalformedValue(
                    "compiled key f64",
                ))
            })?,
        ),
        _ => crate::ir::Value::U64(word),
    })
}

pub(crate) fn compile_error(error: crate::schema::CompileError) -> Error {
    Error::Corruption(crate::error::CorruptionError::MalformedValue(
        match error {
            crate::schema::CompileError::ProjectionIdExhausted => "projection id exhausted",
        },
    ))
}

/// The one condition licensing the bounded resident→fallback restart: the
/// working-byte ledger refused a reservation (image slabs, decoded
/// batches). Other failures — semantic errors, cancellation, deadlines,
/// storage faults — are never retried into a different path.
pub(crate) fn is_working_exhaustion(error: &Error) -> bool {
    matches!(
        error,
        Error::Store(store) if matches!(
            **store,
            StoreError::Work(WorkError::Exhausted {
                resource: crate::work::Resource::WorkingBytes,
                ..
            })
        )
    )
}
