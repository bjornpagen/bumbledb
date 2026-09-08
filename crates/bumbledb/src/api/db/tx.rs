//! One write transaction: an in-memory net delta over the committed
//! parent snapshot. Operations are set arithmetic — order is semantically
//! irrelevant, and `delete(old); insert(new)` in either order is the
//! blessed mutation idiom. Handed to [`super::Db::write`] closures; offers
//! no queries — point reads only ([`WriteTx::contains`] / [`WriteTx::get`]
//! / [`WriteTx::get_dyn`]), which observe the final-state view the
//! judgment phase will judge. Nothing touches LMDB until commit.
//!
//! The pending map is net-normalized against the parent: it holds exactly
//! the rows whose final presence differs from the committed parent, keyed
//! in canonical (relation, full canonical bytes) order — the exact
//! one-command normalized set effect a sealed [`crate::ChangeSet`] carries.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::canonical::{CanonicalRow, RowError};
use crate::changes::ChangeKind;
use crate::error::{DynIdError, Error, FactShapeError, Mismatch, Result};
use crate::ir::Value;
use crate::schema::Schema;
use crate::storage::store::{OwnedSnapshot, StoreError};
use crate::work::WorkContext;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

use super::closed::ClosedRows;
use super::collection::AcceptedCollection;
use super::get as get_path;
use super::row_reader::RowReader;
use super::{Fact, Key, MutationReport};

pub(super) enum TxPhase {
    Clean,
    Applied,
    Poisoned(Box<Error>),
}

struct PendingRelation {
    rows: BTreeMap<Box<[u8]>, ChangeKind>,
}

impl PendingRelation {
    fn apply(
        &mut self,
        row: CanonicalRow,
        want: ChangeKind,
        parent_contains: impl FnOnce(&[u8]) -> Result<bool>,
    ) -> Result<bool> {
        match self.rows.entry(row.into_bytes()) {
            Entry::Occupied(entry) => {
                if *entry.get() == want {
                    Ok(false)
                } else {
                    entry.remove();
                    Ok(true)
                }
            }
            Entry::Vacant(entry) => {
                if parent_contains(entry.key())? == (want == ChangeKind::Add) {
                    Ok(false)
                } else {
                    entry.insert(want);
                    Ok(true)
                }
            }
        }
    }
}

/// Rows and tree ownership travel together through transaction sealing.
#[derive(Default)]
pub(super) struct PendingDelta {
    relations: BTreeMap<RelationId, PendingRelation>,
}

impl PendingDelta {
    pub(super) fn seal(self, schema: &Schema, work: &WorkContext) -> Result<crate::ChangeSet> {
        crate::ChangeSet::from_ordered_records(
            schema,
            self.relations.iter().flat_map(|(relation, pending)| {
                pending
                    .rows
                    .iter()
                    .map(move |(row, change)| crate::changes::ChangeRef {
                        relation: *relation,
                        kind: *change,
                        row,
                    })
            }),
            work,
        )
        .map_err(|error| Error::from_store(StoreError::Changes(error)))
    }
}

#[derive(Default)]
struct RowBatch {
    rows: Vec<CanonicalRow>,
}

impl RowBatch {
    fn push(&mut self, row: CanonicalRow, work: &WorkContext) -> Result<()> {
        work.checkpoint().map_err(store_work)?;
        self.rows
            .try_reserve(1)
            .map_err(|_| row_error(crate::canonical::RowError::Allocation))?;
        self.rows.push(row);
        Ok(())
    }
}

/// One write transaction over the committed parent. `!Send`/`!Sync`
/// (borrows the parent snapshot, which is `!Sync`); carries the handle's
/// schema typestate `S`. No prepared-query or [`super::ReadFrame`] is
/// reachable from here.
pub struct WriteTx<'a, S> {
    schema: &'a Arc<Schema>,
    closed: &'a ClosedRows,
    parent: &'a OwnedSnapshot,
    work: &'a WorkContext,
    pending: PendingDelta,
    phase: TxPhase,
    marker: PhantomData<fn() -> S>,
}

pub(super) fn row_error(error: crate::canonical::RowError) -> Error {
    Error::from_store(StoreError::Changes(crate::changes::ChangeError::Row(error)))
}

/// Encode one dynamic value row to canonical row bytes (shape judged by
/// the canonical codec itself: arity, per-field type, interval validity).
pub(super) fn encode_values(
    schema: &Schema,
    relation: RelationId,
    values: &[Value],
    work: &WorkContext,
) -> Result<Vec<u8>> {
    Ok(encode_owned_values(schema, relation, values, work)?
        .into_bytes()
        .into_vec())
}

fn encode_owned_values(
    schema: &Schema,
    relation: RelationId,
    values: &[Value],
    work: &WorkContext,
) -> Result<CanonicalRow> {
    let Some(view) = schema.relation_checked(relation) else {
        return Err(DynIdError::UnknownRelation { relation }.into());
    };
    CanonicalRow::encode(view.fields(), values, work).map_err(row_error)
}

fn dynamic_row_error(
    relation: RelationId,
    witnessed: usize,
    required: usize,
    error: RowError,
) -> Error {
    match error {
        RowError::Arity => FactShapeError::ArityMismatch {
            relation,
            mismatch: Mismatch {
                witnessed,
                required,
            },
        }
        .into(),
        RowError::Type { field } => FactShapeError::TypeMismatch {
            relation,
            field: FieldId(u16::try_from(field).expect("sealed schema fields fit u16")),
        }
        .into(),
        error => row_error(error),
    }
}

impl<'a, S> WriteTx<'a, S> {
    pub(super) fn new(
        schema: &'a Arc<Schema>,
        closed: &'a ClosedRows,
        parent: &'a OwnedSnapshot,
        work: &'a WorkContext,
    ) -> Self {
        Self {
            schema,
            closed,
            parent,
            work,
            pending: PendingDelta::default(),
            phase: TxPhase::Clean,
            marker: PhantomData,
        }
    }

    pub(super) fn poisoned(&self) -> Option<&Error> {
        match &self.phase {
            TxPhase::Poisoned(source) => Some(source),
            TxPhase::Clean | TxPhase::Applied => None,
        }
    }

    /// The net normalized final-set effect this transaction proposes:
    /// exactly the rows whose presence differs from the parent, canonical
    /// order, at most one action per row.
    pub(super) fn into_pending(self) -> PendingDelta {
        self.pending
    }

    fn refuse_poisoned(&self) -> Result<()> {
        match &self.phase {
            TxPhase::Poisoned(source) => Err(Error::TransactionPoisoned {
                source: source.clone(),
            }),
            TxPhase::Clean | TxPhase::Applied => Ok(()),
        }
    }

    fn poison(&mut self, error: Error) -> Error {
        if let TxPhase::Applied = self.phase {
            self.phase = TxPhase::Poisoned(Box::new(error.clone()));
        }
        error
    }

    fn note_entered(&mut self) {
        if let TxPhase::Clean = self.phase {
            self.phase = TxPhase::Applied;
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

    fn parent_contains(&self, relation: RelationId, row: &[u8]) -> Result<bool> {
        self.parent
            .contains(relation, row, self.work)
            .map_err(Error::from_store)
    }

    /// Final-state presence of one canonical row.
    fn present(&self, relation: RelationId, row: &[u8]) -> Result<bool> {
        if let Some(change) = self
            .pending
            .relations
            .get(&relation)
            .and_then(|pending| pending.rows.get(row))
        {
            return Ok(*change == ChangeKind::Add);
        }
        self.parent_contains(relation, row)
    }

    /// Apply one net disposition. `true` exactly when the final-state view
    /// changed (recorded or cancelled a net disposition).
    fn apply(&mut self, relation: RelationId, row: CanonicalRow, want: ChangeKind) -> Result<bool> {
        self.work.checkpoint().map_err(store_work)?;
        let changed = match self.pending.relations.entry(relation) {
            Entry::Occupied(mut relation_entry) => {
                let pending = relation_entry.get_mut();
                let changed = pending.apply(row, want, |bytes| {
                    self.parent
                        .contains(relation, bytes, self.work)
                        .map_err(Error::from_store)
                })?;
                if pending.rows.is_empty() {
                    relation_entry.remove();
                }
                changed
            }
            Entry::Vacant(entry) => {
                let in_parent = self
                    .parent
                    .contains(relation, row.as_bytes(), self.work)
                    .map_err(Error::from_store)?;
                if in_parent == (want == ChangeKind::Add) {
                    false
                } else {
                    let mut rows = BTreeMap::new();
                    rows.insert(row.into_bytes(), want);
                    entry.insert(PendingRelation { rows });
                    true
                }
            }
        };
        if self.pending.relations.is_empty() {
            // BTreeMap may retain its empty root until the map is dropped.
            self.pending.relations = BTreeMap::new();
        }
        if changed {
            self.note_entered();
        }
        Ok(changed)
    }

    /// Parse-all-first collection application: every row is encoded before
    /// any member enters the pending delta.
    fn apply_rows(
        &mut self,
        relation: RelationId,
        rows: RowBatch,
        want: ChangeKind,
    ) -> Result<MutationReport> {
        let submitted = rows.rows.len() as u64;
        let mut changed = 0u64;
        for row in rows.rows {
            match self.apply(relation, row, want) {
                Ok(true) => changed += 1,
                Ok(false) => {}
                Err(error) => return Err(self.poison(error)),
            }
        }
        Ok(MutationReport::from_counts(submitted, changed))
    }

    fn encode_collection<T>(
        &mut self,
        relation: RelationId,
        facts: impl IntoIterator<Item = T>,
        mut encode: impl FnMut(&Self, T, &mut Vec<Value>) -> Result<CanonicalRow>,
    ) -> Result<RowBatch> {
        self.refuse_poisoned()?;
        self.refuse_closed(relation)?;
        let mut values = Vec::new();
        let mut rows = RowBatch::default();
        for fact in facts {
            match encode(self, fact, &mut values).and_then(|row| rows.push(row, self.work)) {
                Ok(()) => {}
                Err(error) => return Err(self.poison(error)),
            }
        }
        Ok(rows)
    }

    /// # Errors
    /// Shape refusals, closed-relation writes, `TransactionPoisoned` if a
    /// prior apply failed after a prefix entered the delta.
    pub fn insert<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        let rows = self.encode_collection(F::RELATION, facts, |tx, fact, values| {
            values.clear();
            fact.append_values(values)?;
            encode_owned_values(tx.schema.as_ref(), F::RELATION, values, tx.work)
        })?;
        self.apply_rows(F::RELATION, rows, ChangeKind::Add)
    }

    /// # Errors
    /// As [`WriteTx::insert`].
    pub fn delete<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        let rows = self.encode_collection(F::RELATION, facts, |tx, fact, values| {
            values.clear();
            fact.append_values(values)?;
            encode_owned_values(tx.schema.as_ref(), F::RELATION, values, tx.work)
        })?;
        self.apply_rows(F::RELATION, rows, ChangeKind::Remove)
    }

    /// The whole collection is parsed before any row enters the delta.
    /// Parsing checks cancellation as it proceeds, just like typed writes.
    /// A cancellation or allocation failure may precede a later shape
    /// refusal; either discards the whole collection without staging a prefix.
    /// # Errors
    /// As [`WriteTx::insert`], plus unknown-relation/arity/type refusals.
    pub fn insert_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        // A shape refusal after an applied prefix poisons the transaction:
        // the collection boundary is part of the apply, not a free pre-check.
        let Some(rows) = self
            .encode_dyn_collection(rel, facts)
            .map_err(|error| self.poison(error))?
        else {
            return Ok(MutationReport::EMPTY);
        };
        self.apply_rows(rel, rows, ChangeKind::Add)
    }

    /// # Errors
    /// As [`WriteTx::insert_dyn`].
    pub fn delete_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        let Some(rows) = self
            .encode_dyn_collection(rel, facts)
            .map_err(|error| self.poison(error))?
        else {
            return Ok(MutationReport::EMPTY);
        };
        self.apply_rows(rel, rows, ChangeKind::Remove)
    }

    /// # Errors
    /// As [`WriteTx::insert_dyn`]; the collection's shape proof already ran.
    #[doc(hidden)]
    pub fn insert_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.apply_accepted(collection, ChangeKind::Add)
    }

    /// # Errors
    /// As [`WriteTx::insert_dyn`]; the collection's shape proof already ran.
    #[doc(hidden)]
    pub fn delete_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.apply_accepted(collection, ChangeKind::Remove)
    }

    fn encode_dyn_collection(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<Option<RowBatch>> {
        let mut rows = facts.into_iter().peekable();
        if rows.peek().is_none() {
            return Ok(None);
        }
        self.refuse_poisoned()?;
        self.refuse_closed(rel)?;
        let schema = Arc::clone(self.schema);
        let Some(relation) = schema.relation_checked(rel) else {
            return Err(DynIdError::UnknownRelation { relation: rel }.into());
        };
        let fields = relation.fields();
        self.encode_collection(rel, rows, |tx, row, _values| {
            let values = row.as_ref();
            CanonicalRow::encode(fields, values, tx.work)
                .map_err(|error| dynamic_row_error(rel, values.len(), fields.len(), error))
        })
        .map(Some)
    }

    fn apply_accepted(
        &mut self,
        coll: &AcceptedCollection,
        want: ChangeKind,
    ) -> Result<MutationReport> {
        if coll.rows() == 0 {
            return Ok(MutationReport::EMPTY);
        }
        self.refuse_poisoned()?;
        let rel = coll.relation();
        // Shape refusals after an applied prefix poison the transaction —
        // the same hook `apply` failures take, so the accepted-collection
        // bridge cannot leave a half-applied transaction usable.
        if let Err(error) = self.refuse_closed(rel) {
            return Err(self.poison(error));
        }
        let schema = Arc::clone(self.schema);
        let Some(relation) = schema.relation_checked(rel) else {
            return Err(self.poison(DynIdError::UnknownRelation { relation: rel }.into()));
        };
        if usize::from(coll.arity()) != relation.fields().len() {
            return Err(self.poison(
                FactShapeError::ArityMismatch {
                    relation: rel,
                    mismatch: Mismatch {
                        witnessed: usize::from(coll.arity()),
                        required: relation.fields().len(),
                    },
                }
                .into(),
            ));
        }
        // The parsed roster must echo the sealed roster: an ETL-time schema
        // drift is the honest `TypeMismatch` naming the first foreign field.
        for (ordinal, (echoed, field)) in (0u16..).zip(coll.roster().iter().zip(relation.fields()))
        {
            if *echoed != field.value_type {
                return Err(self.poison(
                    FactShapeError::TypeMismatch {
                        relation: rel,
                        field: FieldId(ordinal),
                    }
                    .into(),
                ));
            }
        }
        let mut values = Vec::new();
        let mut rows = RowBatch::default();
        for row in 0..coll.rows() {
            coll.row_values_into(row, &mut values);
            match encode_owned_values(schema.as_ref(), rel, &values, self.work)
                .and_then(|row| rows.push(row, self.work))
            {
                Ok(()) => {}
                Err(error) => return Err(self.poison(error)),
            }
        }
        self.apply_rows(rel, rows, want)
    }

    /// Final-state membership — exactly what a post-commit read observes.
    /// # Errors
    /// Shape refusals or storage failure.
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        self.refuse_poisoned()?;
        let mut values = Vec::new();
        fact.append_values(&mut values)?;
        self.contains_values(F::RELATION, &values)
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.refuse_poisoned()?;
        self.contains_values(rel, values)
    }

    fn contains_values(&self, relation: RelationId, values: &[Value]) -> Result<bool> {
        if let Some(rows) = self.closed.get(relation) {
            return Ok(rows.iter().any(|row| row.values.as_ref() == values));
        }
        let bytes = encode_owned_values(self.schema.as_ref(), relation, values, self.work)?;
        self.present(relation, &bytes)
    }

    /// Keyed point read over the final-state view. The result borrows this
    /// transaction (pending rows) or the parent snapshot.
    /// # Errors
    /// Shape refusals or storage failure.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `tx.get(id)`: \
                  generated key structs are small — by-value keeps every \
                  call site free of `&` noise"
    )]
    pub fn get<'tx, K: Key<'tx, Schema = S>>(&'tx self, key: K) -> Result<Option<K::Fact>> {
        self.refuse_poisoned()?;
        let relation = <K::Fact as Fact<'tx>>::RELATION;
        let (_, statement) =
            get_path::key_statement_of(self.schema.as_ref(), relation, K::STATEMENT)?;
        let mut key_values = Vec::new();
        key.append_key_values(&mut key_values)?;
        get_path::check_key_shape(
            self.schema.as_ref(),
            relation,
            &statement.projection,
            &key_values,
        )?;
        if let Some(rows) = self.closed.get(relation) {
            return match get_path::closed_row_by_key(rows, statement, &key_values) {
                Some(row) => K::Fact::decode(RowReader::new(&row.canonical)?).map(Some),
                None => Ok(None),
            };
        }
        match self.find_by_key(relation, &statement.projection, &key_values, self.work)? {
            Some(bytes) => K::Fact::decode(RowReader::new(bytes)?).map(Some),
            None => Ok(None),
        }
    }

    /// Keyed point read over the final-state view with explicit cancellation
    /// (native/E seam).
    /// # Errors
    /// Shape refusals or storage failure.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Database operations accept owned call-scoped work and key values consistently"
    )]
    pub fn get_with_work<'tx, K: Key<'tx, Schema = S>>(
        &'tx self,
        key: K,
        work: WorkContext,
    ) -> Result<Option<K::Fact>> {
        self.refuse_poisoned()?;
        let relation = <K::Fact as Fact<'tx>>::RELATION;
        let (_, statement) =
            get_path::key_statement_of(self.schema.as_ref(), relation, K::STATEMENT)?;
        let mut key_values = Vec::new();
        key.append_key_values(&mut key_values)?;
        get_path::check_key_shape(
            self.schema.as_ref(),
            relation,
            &statement.projection,
            &key_values,
        )?;
        if let Some(rows) = self.closed.get(relation) {
            return match get_path::closed_row_by_key(rows, statement, &key_values) {
                Some(row) => K::Fact::decode(RowReader::new(&row.canonical)?).map(Some),
                None => Ok(None),
            };
        }
        match self.find_by_key(relation, &statement.projection, &key_values, &work)? {
            Some(bytes) => K::Fact::decode(RowReader::new(bytes)?).map(Some),
            None => Ok(None),
        }
    }

    /// # Errors
    /// Shape refusals or storage failure.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Database operations accept owned call-scoped work and key values consistently"
    )]
    pub fn get_dyn_with_work(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        work: WorkContext,
    ) -> Result<Option<crate::canonical::DecodedRow>> {
        self.refuse_poisoned()?;
        let (_, statement) = get_path::key_statement_of(self.schema.as_ref(), relation, key)?;
        get_path::check_key_shape(
            self.schema.as_ref(),
            relation,
            &statement.projection,
            key_values,
        )?;
        if let Some(rows) = self.closed.get(relation) {
            return get_path::closed_row_by_key(rows, statement, key_values)
                .map(|row| {
                    crate::canonical::decode(
                        self.schema.relation(relation).fields(),
                        &row.canonical,
                        &work,
                    )
                    .map_err(row_error)
                })
                .transpose();
        }
        match self.find_by_key(relation, &statement.projection, key_values, &work)? {
            Some(bytes) => {
                let fields = self.schema.relation(relation).fields();
                crate::canonical::decode(fields, bytes, &work)
                    .map(Some)
                    .map_err(row_error)
            }
            None => Ok(None),
        }
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn get_dyn(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
    ) -> Result<Option<crate::canonical::DecodedRow>> {
        self.get_dyn_with_work(relation, key, key_values, self.work.clone())
    }

    /// Keyed lookup over the final-state view: pending adds first (they are
    /// the freshest proposal, an in-memory walk of this transaction's own
    /// net delta), then the committed parent through the store's determinant
    /// index ([`get_path::find_snapshot_row`] — bucket-shaped work with
    /// exact decoded confirmation, never a relation scan), skipping a
    /// committed row this transaction pending-removes.
    fn find_by_key(
        &self,
        relation: RelationId,
        projection: &[FieldId],
        key_values: &[Value],
        work: &WorkContext,
    ) -> Result<Option<&[u8]>> {
        let fields = self.schema.relation(relation).fields();
        for (row, change) in self
            .pending
            .relations
            .get(&relation)
            .into_iter()
            .flat_map(|pending| pending.rows.iter())
        {
            if *change != ChangeKind::Add {
                continue;
            }
            work.checkpoint().map_err(store_work)?;
            let decoded = crate::canonical::decode(fields, row, work).map_err(row_error)?;
            if get_path::projection_matches(decoded.values(), projection, key_values) {
                return Ok(Some(row));
            }
        }
        // Committed fall-through: the parent snapshot answers through the
        // determinant bucket with exact confirmation. The committed state is
        // judged lawful against this key, so at most one committed row
        // matches the full projection; if this transaction removes it, the
        // final state holds no such row.
        let Some(row) = get_path::find_snapshot_row(
            self.parent,
            self.schema.as_ref(),
            relation,
            projection,
            key_values,
            work,
        )?
        else {
            return Ok(None);
        };
        if let Some(change) = self
            .pending
            .relations
            .get(&relation)
            .and_then(|pending| pending.rows.get(row))
            && *change == ChangeKind::Remove
        {
            return Ok(None);
        }
        Ok(Some(row))
    }
}

fn store_work(error: crate::work::WorkError) -> Error {
    Error::from_store(StoreError::Work(error))
}

#[cfg(test)]
mod pending_tests {
    use super::*;

    #[test]
    fn occupied_pending_rows_do_not_reprobe_the_committed_parent() {
        let work = WorkContext::new();
        let fields = [bumbledb_theory::schema::FieldDescriptor {
            name: "key".into(),
            value_type: bumbledb_theory::schema::ValueType::U64,
        }];
        let row = || CanonicalRow::encode(&fields, &[Value::U64(7)], &work).unwrap();
        let mut pending = PendingRelation {
            rows: BTreeMap::new(),
        };
        let mut probes = 0;
        assert!(
            pending
                .apply(row(), ChangeKind::Remove, |_| {
                    probes += 1;
                    Ok(true)
                })
                .unwrap()
        );
        assert_eq!(probes, 1);
        assert!(
            !pending
                .apply(row(), ChangeKind::Remove, |_| {
                    panic!("duplicate removal must not read its parent again")
                })
                .unwrap()
        );
        assert!(
            pending
                .apply(row(), ChangeKind::Add, |_| {
                    panic!("opposite mutation cancels known parent presence")
                })
                .unwrap()
        );
        assert!(pending.rows.is_empty());
    }
}
