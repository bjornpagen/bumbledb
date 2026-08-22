//! Shared collection-mutation algebra: [`MutationCore`] over a heap stage
//! or a store delta.
//!
//! `M` owns net dispositions, pending interns, fresh reservations, and
//! base lookup when a base exists. The core owns the schema, encode
//! scratch, construction phase, and parse-all-first collection protocol.
//! [`InstanceBuilder`] uses [`HeapMutation`]. [`WriteTx`] wraps
//! [`StoreMutation`].

use std::cell::Cell;
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::sync::Arc;

use super::MutationReport;
use super::apply::ApplyRow;
use super::collection::{AcceptedCollection, intern_accepted_row, intern_value_row};
use super::get as get_path;
use super::{CodecRead, CodecWrite, Fact, Fresh, FreshRange, Probe, codec_seal};
use crate::encoding::{FactLayout, InternId, ValueRef, encode_fact};
use crate::error::{DynIdError, Error, FactShapeError, Mismatch, Result};
use crate::ir::Value;
use crate::schema::FreshField;
use crate::schema::{KeyId, Schema};
use crate::storage::catalog::HeapStage;
use crate::storage::delta::{DeltaEffect, Disposition, WriteDelta};
use crate::storage::env::ReadTxn;
use crate::storage::read;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

/// Clean → Applied (a fact entered) → Poisoned (apply failed after).
pub(super) enum MutationPhase {
    Clean,
    Applied,
    Poisoned(Box<Error>),
}

/// Last-wins overlay for one determinant tuple.
pub(super) enum OverlayFact<'a> {
    Miss,
    Present(&'a [u8]),
    Absent,
}

/// Backend storage for [`MutationCore`].
pub(super) trait MutationBackend {
    fn apply(
        &mut self,
        schema: &Schema,
        relation: RelationId,
        fact: &[u8],
        want: Disposition,
    ) -> Result<DeltaEffect>;

    fn contains(&self, schema: &Schema, relation: RelationId, fact: &[u8]) -> Result<bool>;

    fn intern_str(&mut self, schema: &Schema, value: &str) -> Result<InternId>;

    fn resolve_str(&self, schema: &Schema, value: &str) -> Result<Option<InternId>>;

    fn resolve_raw(&self, id: InternId) -> Result<&[u8]>;

    fn reserve(
        &mut self,
        schema: &Schema,
        relation: RelationId,
        field: FieldId,
        count: NonZeroU64,
    ) -> Result<u64>;

    fn overlay_fact(
        &self,
        schema: &Schema,
        relation: RelationId,
        key: KeyId,
        determinant: &[u8],
    ) -> OverlayFact<'_>;

    fn committed_fact_at(
        &self,
        schema: &Schema,
        relation: RelationId,
        row_id: u64,
    ) -> Result<Option<&[u8]>>;

    fn committed_fact_for_key(
        &self,
        schema: &Schema,
        relation: RelationId,
        u_key: &[u8],
    ) -> Result<Option<&[u8]>>;
}

/// Heap construction backend: empty base, chunked [`HeapStage`].
pub(crate) struct HeapMutation {
    pub(super) stage: HeapStage,
}

/// Durable mutation backend: delta over an admitted store's read view.
pub(crate) struct StoreMutation<'db> {
    pub(super) view: ReadTxn<'db>,
    pub(super) delta: WriteDelta<'db>,
}

/// Private collection protocol shared by heap construction and durable
/// writes. `Send + !Sync`: the TypeScript binding hands a builder to an
/// async native task for admission.
pub(crate) struct MutationCore<M, S> {
    schema: Arc<Schema>,
    pub(super) scratch: Vec<u8>,
    refs: Vec<ValueRef>,
    parse_bytes: Vec<u8>,
    parse_ready: Vec<bool>,
    parse_spans: Vec<(usize, usize)>,
    phase: MutationPhase,
    pub(super) backend: M,
    not_sync: PhantomData<Cell<()>>,
    schema_ty: PhantomData<fn() -> S>,
}

impl<'db, S> MutationCore<StoreMutation<'db>, S> {
    pub(super) fn store(schema: Arc<Schema>, schema_ref: &'db Schema, view: ReadTxn<'db>) -> Self {
        Self {
            schema,
            scratch: Vec::new(),
            refs: Vec::new(),
            parse_bytes: Vec::new(),
            parse_ready: Vec::new(),
            parse_spans: Vec::new(),
            phase: MutationPhase::Clean,
            backend: StoreMutation {
                view,
                delta: WriteDelta::new(schema_ref),
            },
            not_sync: PhantomData,
            schema_ty: PhantomData,
        }
    }

    pub(super) fn into_store(self) -> (ReadTxn<'db>, WriteDelta<'db>) {
        let StoreMutation { view, delta } = self.backend;
        (view, delta)
    }
}

impl<S> MutationCore<HeapMutation, S> {
    pub(super) fn into_heap(self) -> (Arc<Schema>, HeapStage) {
        (self.schema, self.backend.stage)
    }

    pub(super) fn heap(schema: Arc<Schema>) -> Self {
        let stage = HeapStage::new(schema.as_ref());
        Self {
            schema,
            scratch: Vec::new(),
            refs: Vec::new(),
            parse_bytes: Vec::new(),
            parse_ready: Vec::new(),
            parse_spans: Vec::new(),
            phase: MutationPhase::Clean,
            backend: HeapMutation { stage },
            not_sync: PhantomData,
            schema_ty: PhantomData,
        }
    }
}

impl MutationBackend for HeapMutation {
    fn apply(
        &mut self,
        schema: &Schema,
        relation: RelationId,
        fact: &[u8],
        want: Disposition,
    ) -> Result<DeltaEffect> {
        Ok(self.stage.apply(schema, relation, fact, want))
    }

    fn contains(&self, _schema: &Schema, relation: RelationId, fact: &[u8]) -> Result<bool> {
        Ok(self.stage.contains(relation, fact))
    }

    fn intern_str(&mut self, _schema: &Schema, value: &str) -> Result<InternId> {
        Ok(self.stage.intern_str(value))
    }

    fn resolve_str(&self, _schema: &Schema, value: &str) -> Result<Option<InternId>> {
        Ok(self.stage.resolve_str(value))
    }

    fn resolve_raw(&self, id: InternId) -> Result<&[u8]> {
        self.stage.pending_raw(id).ok_or(Error::Corruption(
            crate::error::CorruptionError::DanglingInternId(id),
        ))
    }

    fn reserve(
        &mut self,
        schema: &Schema,
        relation: RelationId,
        field: FieldId,
        count: NonZeroU64,
    ) -> Result<u64> {
        self.stage.reserve(schema, relation, field, count)
    }

    fn overlay_fact(
        &self,
        schema: &Schema,
        relation: RelationId,
        key: KeyId,
        determinant: &[u8],
    ) -> OverlayFact<'_> {
        match self.stage.overlay_fact(schema, relation, key, determinant) {
            Some(bytes) => OverlayFact::Present(bytes),
            None => OverlayFact::Miss,
        }
    }

    fn committed_fact_at(
        &self,
        _schema: &Schema,
        _relation: RelationId,
        _row_id: u64,
    ) -> Result<Option<&[u8]>> {
        Ok(None)
    }

    fn committed_fact_for_key(
        &self,
        _schema: &Schema,
        _relation: RelationId,
        _u_key: &[u8],
    ) -> Result<Option<&[u8]>> {
        Ok(None)
    }
}

impl MutationBackend for StoreMutation<'_> {
    fn apply(
        &mut self,
        _schema: &Schema,
        relation: RelationId,
        fact: &[u8],
        want: Disposition,
    ) -> Result<DeltaEffect> {
        self.delta.apply(&self.view, relation, fact, want)
    }

    fn contains(&self, _schema: &Schema, relation: RelationId, fact: &[u8]) -> Result<bool> {
        self.delta.contains(&self.view, relation, fact)
    }

    fn intern_str(&mut self, _schema: &Schema, value: &str) -> Result<InternId> {
        self.delta.intern_str(&self.view, value)
    }

    fn resolve_str(&self, _schema: &Schema, value: &str) -> Result<Option<InternId>> {
        self.delta.resolve_str(&self.view, value)
    }

    fn resolve_raw(&self, id: InternId) -> Result<&[u8]> {
        match self.delta.pending_raw(id) {
            Some(raw) => Ok(raw),
            None => crate::storage::dict::resolve(&self.view, id),
        }
    }

    fn reserve(
        &mut self,
        _schema: &Schema,
        relation: RelationId,
        field: FieldId,
        count: NonZeroU64,
    ) -> Result<u64> {
        self.delta.reserve(&self.view, relation, field, count)
    }

    fn overlay_fact(
        &self,
        _schema: &Schema,
        _relation: RelationId,
        key: KeyId,
        determinant: &[u8],
    ) -> OverlayFact<'_> {
        match self.delta.determinant_overlay(key, determinant) {
            Some(crate::storage::delta::DeterminantOverlay::Present(bytes)) => {
                OverlayFact::Present(bytes)
            }
            Some(crate::storage::delta::DeterminantOverlay::Absent) => OverlayFact::Absent,
            None => OverlayFact::Miss,
        }
    }

    fn committed_fact_at(
        &self,
        schema: &Schema,
        relation: RelationId,
        row_id: u64,
    ) -> Result<Option<&[u8]>> {
        Ok(read::fact_at(&self.view, schema, relation, row_id)?
            .map(crate::encoding::FactView::bytes))
    }

    fn committed_fact_for_key(
        &self,
        schema: &Schema,
        relation: RelationId,
        u_key: &[u8],
    ) -> Result<Option<&[u8]>> {
        Ok(read::fact_for_key(&self.view, schema, relation, u_key)?
            .map(crate::encoding::FactView::bytes))
    }
}

impl<M, S> MutationCore<M, S> {
    pub(super) fn schema(&self) -> &Schema {
        self.schema.as_ref()
    }

    pub(super) fn refuse_poisoned(&self) -> Result<()> {
        match &self.phase {
            MutationPhase::Poisoned(source) => Err(Error::TransactionPoisoned {
                source: source.clone(),
            }),
            MutationPhase::Clean | MutationPhase::Applied => Ok(()),
        }
    }

    pub(super) fn poisoned(&self) -> Option<&Error> {
        match &self.phase {
            MutationPhase::Poisoned(source) => Some(source),
            MutationPhase::Clean | MutationPhase::Applied => None,
        }
    }

    pub(super) fn poison(&mut self, err: Error) -> Error {
        if let MutationPhase::Applied = self.phase {
            self.phase = MutationPhase::Poisoned(Box::new(err.clone()));
        }
        err
    }

    fn note_entered(&mut self) {
        if let MutationPhase::Clean = self.phase {
            self.phase = MutationPhase::Applied;
        }
    }

    fn refuse_closed(&self, relation: RelationId) -> Result<()> {
        match self.schema.relation_checked(relation) {
            Some(rel) if rel.body().closed_rows().is_some() => {
                Err(Error::ClosedRelationWrite { relation })
            }
            _ => Ok(()),
        }
    }

    pub(super) fn with_scratch<R>(
        &mut self,
        body: impl FnOnce(&mut Self, &mut Vec<u8>) -> Result<R>,
    ) -> Result<R> {
        let mut bytes = std::mem::take(&mut self.scratch);
        bytes.clear();
        let out = body(self, &mut bytes);
        self.scratch = bytes;
        out
    }
}

impl<M: MutationBackend, S> MutationCore<M, S> {
    /// Parse-all-first collection apply: every member is encoded before
    /// any disposition is recorded.
    pub(super) fn apply_collection<T>(
        &mut self,
        relation: RelationId,
        want: Disposition,
        facts: impl IntoIterator<Item = T>,
        mut encode: impl FnMut(&mut Self, T, &mut Vec<u8>) -> Result<ApplyRow>,
    ) -> Result<MutationReport> {
        let facts: Vec<T> = facts.into_iter().collect();
        if facts.is_empty() {
            return Ok(MutationReport::EMPTY);
        }
        self.refuse_poisoned()?;
        self.refuse_closed(relation)?;
        self.parse_bytes.clear();
        self.parse_ready.clear();
        self.parse_spans.clear();
        for fact in facts {
            let row = match self.with_scratch(|core, bytes| encode(core, fact, bytes)) {
                Ok(row) => row,
                Err(error) => return Err(self.poison(error)),
            };
            match row {
                ApplyRow::Skip => {
                    self.parse_ready.push(false);
                    self.parse_spans.push((0, 0));
                }
                ApplyRow::Ready => {
                    let start = self.parse_bytes.len();
                    // `with_scratch` restored the encoded bytes into `scratch`.
                    let len = self.scratch.len();
                    self.parse_bytes.extend_from_slice(&self.scratch);
                    self.parse_ready.push(true);
                    self.parse_spans.push((start, len));
                }
            }
        }
        self.apply_prepared(relation, want)
    }

    fn apply_prepared(
        &mut self,
        relation: RelationId,
        want: Disposition,
    ) -> Result<MutationReport> {
        let submitted = u64::try_from(self.parse_ready.len()).expect("collection fits u64");
        let mut changed = 0u64;
        let ready = std::mem::take(&mut self.parse_ready);
        let spans = std::mem::take(&mut self.parse_spans);
        let bytes = std::mem::take(&mut self.parse_bytes);
        for (is_ready, (start, len)) in ready.iter().zip(spans.iter()) {
            if !is_ready {
                continue;
            }
            let fact = &bytes[*start..*start + *len];
            match self
                .backend
                .apply(self.schema.as_ref(), relation, fact, want)
            {
                Ok(effect) => {
                    if effect.changed() {
                        self.note_entered();
                        changed += 1;
                    }
                }
                Err(error) => {
                    self.parse_bytes = bytes;
                    self.parse_ready = ready;
                    self.parse_spans = spans;
                    self.parse_bytes.clear();
                    self.parse_ready.clear();
                    self.parse_spans.clear();
                    return Err(self.poison(error));
                }
            }
        }
        self.parse_bytes = bytes;
        self.parse_ready = ready;
        self.parse_spans = spans;
        self.parse_bytes.clear();
        self.parse_ready.clear();
        self.parse_spans.clear();
        Ok(MutationReport::from_counts(submitted, changed))
    }

    pub(super) fn reserve_count(
        &mut self,
        relation: RelationId,
        field: FieldId,
        count: u64,
    ) -> Result<u64> {
        self.refuse_poisoned()?;
        let Some(count) = NonZeroU64::new(count) else {
            return Ok(0);
        };
        self.refuse_closed(relation)?;
        match self
            .backend
            .reserve(self.schema.as_ref(), relation, field, count)
        {
            Ok(start) => Ok(start),
            Err(error) => Err(self.poison(error)),
        }
    }

    pub(super) fn reserve_at_count(&mut self, field: FreshField<S>, count: u64) -> Result<u64> {
        self.refuse_poisoned()?;
        let Some(count) = NonZeroU64::new(count) else {
            return Ok(0);
        };
        match self
            .backend
            .reserve(self.schema.as_ref(), field.relation(), field.field(), count)
        {
            Ok(start) => Ok(start),
            Err(error) => Err(self.poison(error)),
        }
    }

    /// Parses the dyn lane's rows into ONE [`AcceptedCollection`], under
    /// the standing law order: an empty collection is `None` — no engine
    /// request, judged before any refusal — then poison, closed, unknown
    /// relation, and the shape parse (arity per row, type-kind per cell).
    fn accept_dyn(
        &self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<Option<AcceptedCollection>> {
        let mut rows = facts.into_iter().peekable();
        if rows.peek().is_none() {
            return Ok(None);
        }
        self.refuse_poisoned()?;
        self.refuse_closed(rel)?;
        let Some(relation) = self.schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        let coll = AcceptedCollection::from_value_rows(rel, relation.fields(), rows)?;
        Ok(Some(coll))
    }

    /// Applies one shape-proved collection under `want` — the one
    /// consumption of [`AcceptedCollection`], built ON the parse-all-first
    /// [`Self::apply_collection`] machinery: row indices in, borrowed cell
    /// views interned and encoded through the reused `refs`/`scratch`
    /// path, no per-row container anywhere.
    ///
    /// Law order preserved exactly: empty is `MutationReport::EMPTY`
    /// before any refusal; then poison, closed, unknown relation, and the
    /// roster re-verification — the authoritative second wall, BOTH
    /// halves. The collection's own relation IS the apply target (the
    /// transport surfaces take only the collection), so relation equality
    /// holds by construction; the roster re-anchor is the arity check
    /// against the relation this core sealed PLUS the roster-echo proof
    /// (the collection carries the value-type row its cells were judged
    /// against, and apply proves that echo IS the target roster — arity
    /// alone would admit a same-arity, type-different forgery straight
    /// into the encoder's positional arms).
    pub(super) fn apply_accepted(
        &mut self,
        coll: &AcceptedCollection,
        want: Disposition,
    ) -> Result<MutationReport> {
        if coll.rows() == 0 {
            return Ok(MutationReport::EMPTY);
        }
        self.refuse_poisoned()?;
        let rel = coll.relation();
        self.refuse_closed(rel)?;
        let schema = Arc::clone(&self.schema);
        let Some(relation) = schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        if usize::from(coll.arity()) != relation.fields().len() {
            return Err(FactShapeError::ArityMismatch {
                relation: rel,
                mismatch: Mismatch {
                    witnessed: usize::from(coll.arity()),
                    required: relation.fields().len(),
                },
            }
            .into());
        }
        // The roster ECHO (the second wall's type half): O(arity) per
        // collection, zero per-cell cost — sealed cells were proved
        // against the echo, this loop proves the echo is the target
        // roster, and the refusal is the honest `TypeMismatch` naming the
        // first differing field.
        for (ordinal, (echoed, field)) in (0u16..).zip(coll.roster().iter().zip(relation.fields()))
        {
            if *echoed != field.value_type {
                return Err(FactShapeError::TypeMismatch {
                    relation: rel,
                    field: FieldId(ordinal),
                }
                .into());
            }
        }
        let layout = relation.layout();
        // The arity-0 collapse law (set semantics; `proposals/
        // one-representation/20`): every row of a fieldless relation IS
        // the one empty tuple, so a collection of N rows is ONE judged
        // apply — the empty tuple, applied once (rows > 0 here; empty
        // short-circuited above) — with `submitted = rows` echoed exactly
        // and `changed` the one effect (0 or 1), insert and delete twins
        // symmetric (one op each, `changed <= 1`). The count is DATA on
        // the bridge crossing (the cells wall `0 == rows × 0` is vacuous,
        // so any stated count is shape-lawful), and the cost of an apply
        // must be bounded by the payload the caller marshaled, never by a
        // stated count: the collapse is O(1) where the general loop would
        // be O(rows). The one apply rides the SAME parse-all-first
        // machinery (spans, poison-on-failure discipline included), so a
        // failure of the one judged apply poisons exactly as the general
        // loop's first row would.
        if relation.fields().is_empty() {
            let one = match want {
                Disposition::Insert => {
                    self.apply_collection(rel, want, std::iter::once(0u64), |core, row, bytes| {
                        core.encode_accepted_mint(coll, row, layout, bytes)
                    })?
                }
                Disposition::Delete => {
                    self.apply_collection(rel, want, std::iter::once(0u64), |core, row, bytes| {
                        core.encode_accepted_resolve(coll, row, layout, bytes)
                    })?
                }
            };
            return Ok(MutationReport::from_counts(coll.rows(), one.changed()));
        }
        match want {
            Disposition::Insert => {
                self.apply_collection(rel, want, 0..coll.rows(), |core, row, bytes| {
                    core.encode_accepted_mint(coll, row, layout, bytes)
                })
            }
            Disposition::Delete => {
                self.apply_collection(rel, want, 0..coll.rows(), |core, row, bytes| {
                    core.encode_accepted_resolve(coll, row, layout, bytes)
                })
            }
        }
    }

    /// Encodes accepted row `row`, minting novel strings — the insert
    /// disposition's arm.
    fn encode_accepted_mint(
        &mut self,
        coll: &AcceptedCollection,
        row: u64,
        layout: &FactLayout,
        bytes: &mut Vec<u8>,
    ) -> Result<ApplyRow> {
        let mut refs = std::mem::take(&mut self.refs);
        let encoded = intern_accepted_row(coll, row, &mut refs, |text| {
            self.backend
                .intern_str(self.schema.as_ref(), text)
                .map(Some)
        });
        finish_encode(encoded, refs, self, layout, bytes)
    }

    /// Encodes accepted row `row`, resolve-only — the delete
    /// disposition's arm: a never-interned string proves the row absent
    /// ([`ApplyRow::Skip`]) without growing the dictionary.
    fn encode_accepted_resolve(
        &mut self,
        coll: &AcceptedCollection,
        row: u64,
        layout: &FactLayout,
        bytes: &mut Vec<u8>,
    ) -> Result<ApplyRow> {
        let mut refs = std::mem::take(&mut self.refs);
        let encoded = intern_accepted_row(coll, row, &mut refs, |text| {
            self.backend.resolve_str(self.schema.as_ref(), text)
        });
        finish_encode(encoded, refs, self, layout, bytes)
    }

    pub(super) fn encode_dyn(&mut self, rel: RelationId, values: &[Value]) -> Result<bool> {
        let schema = Arc::clone(&self.schema);
        let Some(relation) = schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        let layout = relation.layout();
        self.with_scratch(|core, bytes| {
            let mut refs = std::mem::take(&mut core.refs);
            let encoded = intern_value_row(rel, relation.fields(), values, &mut refs, |text| {
                core.backend.resolve_str(core.schema.as_ref(), text)
            });
            match finish_encode(encoded, refs, core, layout, bytes)? {
                ApplyRow::Ready => Ok(true),
                ApplyRow::Skip => Ok(false),
            }
        })
    }

    pub(super) fn overlay_contains(&self, relation: RelationId, fact_bytes: &[u8]) -> Result<bool> {
        if let Some(extension) = self.schema.relation(relation).body().closed_rows() {
            return Ok(extension.iter().any(|row| row.fact.as_ref() == fact_bytes));
        }
        self.backend
            .contains(self.schema.as_ref(), relation, fact_bytes)
    }

    pub(super) fn fact_by_key(
        &self,
        relation: RelationId,
        key: KeyId,
        u_key: &[u8],
    ) -> Result<Option<&[u8]>> {
        let rel = self.schema.relation(relation);
        let statement = self.schema.key(key);
        let determinant = &u_key[read::DETERMINANT_KEY_HEADER..];
        match get_path::point_read(rel, statement, determinant) {
            get_path::PointRead::Closed => Ok(get_path::closed_fact_by_determinant(
                rel,
                statement,
                determinant,
            )),
            path => {
                match self
                    .backend
                    .overlay_fact(self.schema.as_ref(), relation, key, determinant)
                {
                    OverlayFact::Present(bytes) => Ok(Some(bytes)),
                    OverlayFact::Absent => Ok(None),
                    OverlayFact::Miss => {
                        match path {
                            get_path::PointRead::FreshRow { row_id } => self
                                .backend
                                .committed_fact_at(self.schema.as_ref(), relation, row_id),
                            get_path::PointRead::Determinant => self
                                .backend
                                .committed_fact_for_key(self.schema.as_ref(), relation, u_key),
                            get_path::PointRead::Closed => {
                                unreachable!("closed relations have no overlay")
                            }
                        }
                    }
                }
            }
        }
    }

    fn decode_values_keyed(
        &self,
        relation: RelationId,
        projection: &[FieldId],
        key_values: &[Value],
        fact: &[u8],
        out: &mut Vec<Value>,
    ) -> Result<()> {
        crate::encoding::decode_values_keyed_into(
            self.schema.relation(relation).layout().encoded(fact),
            projection,
            key_values,
            |id| {
                Ok(Box::from(
                    self.resolve_intern(crate::encoding::InternId::from_raw(id))?,
                ))
            },
            out,
        )
    }

    fn resolve_intern(&self, id: InternId) -> Result<&str> {
        let raw = self.backend.resolve_raw(id)?;
        std::str::from_utf8(raw)
            .map_err(|_| Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(id.raw())))
    }
}

fn finish_encode<M, S>(
    encoded: Result<bool>,
    refs: Vec<ValueRef>,
    core: &mut MutationCore<M, S>,
    layout: &FactLayout,
    bytes: &mut Vec<u8>,
) -> Result<ApplyRow> {
    let encoded = match encoded {
        Ok(encoded) => encoded,
        Err(error) => {
            core.refs = refs;
            return Err(error);
        }
    };
    if encoded {
        bytes.clear();
        encode_fact(&refs, layout, bytes);
    }
    core.refs = refs;
    Ok(if encoded {
        ApplyRow::Ready
    } else {
        ApplyRow::Skip
    })
}

impl<M: MutationBackend, S> codec_seal::Sealed for MutationCore<M, S> {}

impl<M: MutationBackend, S> CodecRead<S> for MutationCore<M, S> {
    fn schema(&self) -> &Schema {
        self.schema.as_ref()
    }

    fn lookup_str(&self, value: &str) -> Result<Option<InternId>> {
        self.backend.resolve_str(self.schema.as_ref(), value)
    }

    fn resolve_str(&self, id: InternId) -> Result<&str> {
        self.resolve_intern(id)
    }
}

impl<M: MutationBackend, S> CodecWrite<S> for MutationCore<M, S> {
    fn intern_str(&mut self, value: &str) -> Result<InternId> {
        self.backend.intern_str(self.schema.as_ref(), value)
    }
}

impl<M: MutationBackend, S> MutationCore<M, S> {
    pub(super) fn load<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        self.apply_collection(
            F::RELATION,
            Disposition::Insert,
            facts,
            |core, fact, bytes| {
                fact.encode_insert(core, bytes)?;
                Ok(ApplyRow::Ready)
            },
        )
    }

    pub(super) fn delete<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        self.apply_collection(
            F::RELATION,
            Disposition::Delete,
            facts,
            |core, fact, bytes| {
                if matches!(fact.encode_probe(core, bytes)?, Probe::Encoded) {
                    Ok(ApplyRow::Ready)
                } else {
                    Ok(ApplyRow::Skip)
                }
            },
        )
    }

    pub(super) fn load_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        let Some(coll) = self.accept_dyn(rel, facts)? else {
            return Ok(MutationReport::EMPTY);
        };
        self.apply_accepted(&coll, Disposition::Insert)
    }

    pub(super) fn delete_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        let Some(coll) = self.accept_dyn(rel, facts)? else {
            return Ok(MutationReport::EMPTY);
        };
        self.apply_accepted(&coll, Disposition::Delete)
    }

    pub(super) fn reserve<T: Fresh<Schema = S>>(&mut self, count: u64) -> Result<FreshRange<T>> {
        if count == 0 {
            self.refuse_poisoned()?;
            return Ok(FreshRange::Empty);
        }
        let start = self.reserve_count(T::RELATION, T::FIELD, count)?;
        Ok(FreshRange::minted(
            start,
            NonZeroU64::new(count).expect("count checked nonzero"),
        ))
    }

    pub(super) fn reserve_at(
        &mut self,
        field: FreshField<S>,
        count: u64,
    ) -> Result<FreshRange<u64>> {
        if count == 0 {
            self.refuse_poisoned()?;
            return Ok(FreshRange::Empty);
        }
        let start = self.reserve_at_count(field, count)?;
        Ok(FreshRange::minted(
            start,
            NonZeroU64::new(count).expect("count checked nonzero"),
        ))
    }

    pub(super) fn contains<'f, F: Fact<'f, Schema = S>>(&mut self, fact: &F) -> Result<bool> {
        self.with_scratch(|core, bytes| {
            if matches!(fact.encode_probe(core, bytes)?, Probe::ProvablyAbsent) {
                return Ok(false);
            }
            core.overlay_contains(F::RELATION, bytes)
        })
    }

    pub(super) fn contains_dyn(&mut self, rel: RelationId, values: &[Value]) -> Result<bool> {
        if !self.encode_dyn(rel, values)? {
            return Ok(false);
        }
        self.overlay_contains(rel, &self.scratch)
    }

    pub(super) fn get_dyn_into(
        &mut self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        out.clear();
        let schema = Arc::clone(&self.schema);
        self.with_scratch(|core, key_bytes| {
            let (key_id, statement) = get_path::key_statement_of(schema.as_ref(), relation, key)?;
            let projection = &statement.projection;
            read::begin_determinant_key(key_bytes, relation, statement.id);
            if !get_path::encode_determinant_with(
                schema.as_ref(),
                relation,
                projection,
                key_values,
                key_bytes,
                |text| core.backend.resolve_str(schema.as_ref(), text),
            )? {
                return Ok(false);
            }
            let Some(bytes) = core.fact_by_key(relation, key_id, key_bytes)? else {
                return Ok(false);
            };
            core.decode_values_keyed(relation, projection, key_values, bytes, out)?;
            Ok(true)
        })
    }
}
