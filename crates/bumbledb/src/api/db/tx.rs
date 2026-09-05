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

use crate::canonical::CanonicalRow;
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

/// One write transaction over the committed parent. `!Send`/`!Sync`
/// (borrows the parent snapshot, which is `!Sync`); carries the handle's
/// schema typestate `S`. No prepared-query or [`super::ReadInstance`] is
/// reachable from here.
pub struct WriteTx<'a, S> {
    schema: &'a Arc<Schema>,
    closed: &'a ClosedRows,
    parent: &'a OwnedSnapshot,
    work: &'a WorkContext,
    pending: BTreeMap<(RelationId, Box<[u8]>), ChangeKind>,
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
    let Some(view) = schema.relation_checked(relation) else {
        return Err(DynIdError::UnknownRelation { relation }.into());
    };
    let row = CanonicalRow::encode(view.fields(), values, work).map_err(row_error)?;
    Ok(row.as_bytes().to_vec())
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
            pending: BTreeMap::new(),
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
    pub(super) fn into_pending(self) -> BTreeMap<(RelationId, Box<[u8]>), ChangeKind> {
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
        // A borrowed (RelationId, Box<[u8]>) lookup needs an owned key with
        // BTreeMap's default borrowing; range over the exact key instead.
        if let Some((_, kind)) = self
            .pending
            .range((relation, row_key(row))..=(relation, row_key(row)))
            .next()
        {
            return Ok(*kind == ChangeKind::Add);
        }
        self.parent_contains(relation, row)
    }

    /// Apply one net disposition. `true` exactly when the final-state view
    /// changed (recorded or cancelled a net disposition).
    fn apply(&mut self, relation: RelationId, row: Vec<u8>, want: ChangeKind) -> Result<bool> {
        self.work.step(1).map_err(store_work)?;
        let in_parent = self.parent_contains(relation, &row)?;
        let key = (relation, row.into_boxed_slice());
        let changed = match self.pending.entry(key) {
            Entry::Occupied(entry) => {
                if *entry.get() == want {
                    false
                } else {
                    // The opposite disposition exists; the requested action
                    // returns the row to its parent state.
                    entry.remove();
                    true
                }
            }
            Entry::Vacant(entry) => match want {
                ChangeKind::Add => {
                    if in_parent {
                        false
                    } else {
                        entry.insert(ChangeKind::Add);
                        true
                    }
                }
                ChangeKind::Remove => {
                    if in_parent {
                        entry.insert(ChangeKind::Remove);
                        true
                    } else {
                        false
                    }
                }
            },
        };
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
        rows: Vec<Vec<u8>>,
        want: ChangeKind,
    ) -> Result<MutationReport> {
        let submitted = rows.len() as u64;
        let mut changed = 0u64;
        for row in rows {
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
        mut encode: impl FnMut(&Self, T, &mut Vec<Value>) -> Result<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>> {
        self.refuse_poisoned()?;
        self.refuse_closed(relation)?;
        let mut values = Vec::new();
        let mut rows = Vec::new();
        for fact in facts {
            match encode(self, fact, &mut values) {
                Ok(row) => rows.push(row),
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
            encode_values(tx.schema.as_ref(), F::RELATION, values, tx.work)
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
            encode_values(tx.schema.as_ref(), F::RELATION, values, tx.work)
        })?;
        self.apply_rows(F::RELATION, rows, ChangeKind::Remove)
    }

    /// The whole collection is parsed before any row enters the delta.
    /// # Errors
    /// As [`WriteTx::insert`], plus unknown-relation/arity/type refusals.
    pub fn insert_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        let Some(coll) = self.accept_dyn(rel, facts)? else {
            return Ok(MutationReport::EMPTY);
        };
        self.apply_accepted(&coll, ChangeKind::Add)
    }

    /// # Errors
    /// As [`WriteTx::insert_dyn`].
    pub fn delete_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        let Some(coll) = self.accept_dyn(rel, facts)? else {
            return Ok(MutationReport::EMPTY);
        };
        self.apply_accepted(&coll, ChangeKind::Remove)
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
        Ok(Some(AcceptedCollection::from_value_rows(
            rel,
            relation.fields(),
            rows,
        )?))
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
        self.refuse_closed(rel)?;
        let schema = Arc::clone(self.schema);
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
        // The parsed roster must echo the sealed roster: an ETL-time schema
        // drift is the honest `TypeMismatch` naming the first foreign field.
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
        let mut values = Vec::new();
        let mut rows = Vec::with_capacity(usize::try_from(coll.rows()).unwrap_or(0));
        for row in 0..coll.rows() {
            coll.row_values_into(row, &mut values);
            match encode_values(schema.as_ref(), rel, &values, self.work) {
                Ok(bytes) => rows.push(bytes),
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
        let bytes = encode_values(self.schema.as_ref(), relation, values, self.work)?;
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
        match self.find_by_key(relation, &statement.projection, &key_values)? {
            Some(bytes) => K::Fact::decode(RowReader::new(bytes)?).map(Some),
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
    ) -> Result<Option<Vec<Value>>> {
        let mut out = Vec::new();
        Ok(self
            .get_dyn_into(relation, key, key_values, &mut out)?
            .then_some(out))
    }

    /// # Errors
    /// Shape refusals or storage failure.
    pub fn get_dyn_into(
        &self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        out.clear();
        self.refuse_poisoned()?;
        let (_, statement) = get_path::key_statement_of(self.schema.as_ref(), relation, key)?;
        get_path::check_key_shape(
            self.schema.as_ref(),
            relation,
            &statement.projection,
            key_values,
        )?;
        if let Some(rows) = self.closed.get(relation) {
            return Ok(
                match get_path::closed_row_by_key(rows, statement, key_values) {
                    Some(row) => {
                        out.extend(row.values.iter().cloned());
                        true
                    }
                    None => false,
                },
            );
        }
        match self.find_by_key(relation, &statement.projection, key_values)? {
            Some(bytes) => {
                let fields = self.schema.relation(relation).fields();
                let decoded =
                    crate::canonical::decode(fields, bytes, self.work).map_err(row_error)?;
                out.extend(decoded.values);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Reference keyed lookup over the final-state view: pending adds
    /// first (they are the freshest proposal), then parent rows that are
    /// not pending removes. Exact decoded-value comparison; acceleration
    /// is the recorded C04/C05 follow-up.
    fn find_by_key(
        &self,
        relation: RelationId,
        projection: &[FieldId],
        key_values: &[Value],
    ) -> Result<Option<&[u8]>> {
        let fields = self.schema.relation(relation).fields();
        let lower = (relation, row_key(&[][..]));
        for ((rel, row), kind) in self.pending.range(lower..) {
            if *rel != relation {
                break;
            }
            if *kind != ChangeKind::Add {
                continue;
            }
            self.work.step(1).map_err(store_work)?;
            let decoded = crate::canonical::decode(fields, row, self.work).map_err(row_error)?;
            if get_path::projection_matches(&decoded.values, projection, key_values) {
                return Ok(Some(row));
            }
        }
        let iterator = self.parent.rows(relation).map_err(Error::from_store)?;
        for entry in iterator {
            self.work.step(1).map_err(store_work)?;
            let (_, row) = entry.map_err(Error::from_store)?;
            if let Some((_, kind)) = self
                .pending
                .range((relation, row_key(row))..=(relation, row_key(row)))
                .next()
                && *kind == ChangeKind::Remove
            {
                continue;
            }
            let decoded = crate::canonical::decode(fields, row, self.work).map_err(row_error)?;
            if get_path::projection_matches(&decoded.values, projection, key_values) {
                return Ok(Some(row));
            }
        }
        Ok(None)
    }
}

fn store_work(error: crate::work::WorkError) -> Error {
    Error::from_store(StoreError::Work(error))
}

fn row_key(row: &[u8]) -> Box<[u8]> {
    Box::from(row)
}

/// Serialize one net pending delta as the sealed `ChangeSet` wire and parse
/// it back through the strict boundary — one normalization implementation,
/// no second writer of the format.
pub(super) fn change_set_of_pending(
    schema: &Schema,
    pending: &BTreeMap<(RelationId, Box<[u8]>), ChangeKind>,
    work: &WorkContext,
) -> Result<crate::ChangeSet> {
    const MAGIC: &[u8; 8] = b"BDBCSET\0";
    const VERSION: u16 = 1;
    let identity = crate::schema::fingerprint::fingerprint(schema);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_be_bytes());
    bytes.extend_from_slice(&identity.0);
    bytes.extend_from_slice(&(pending.len() as u64).to_be_bytes());
    for ((relation, row), kind) in pending {
        bytes.push(u8::from(*kind == ChangeKind::Add));
        bytes.extend_from_slice(&relation.0.to_be_bytes());
        bytes.extend_from_slice(&(row.len() as u64).to_be_bytes());
        bytes.extend_from_slice(row);
    }
    crate::ChangeSet::parse(schema, &bytes, work)
        .map_err(|error| Error::from_store(StoreError::Changes(error)))
}
