//! Unproved heap candidate: collection mutation and overlay point reads
//! from an empty base. No query preparation or execution — an unproved
//! candidate cannot be queried. [`InstanceBuilder::admit`] runs the
//! complete production final-state judgment and consumes the builder into
//! an [`super::OwnedInstance`].
//!
//! `Send + !Sync`: a host may move the builder onto another thread for
//! admission.
//! ```compile_fail
//! fn require_sync<T: Sync>() {}
//! require_sync::<bumbledb::InstanceBuilder<()>>();
//! ```
//! ```compile_fail
//! fn require_prepare(builder: &bumbledb::InstanceBuilder<()>) {
//!     let _ = builder.prepare;
//! }
//! ```
//! ```compile_fail
//! fn require_execute(builder: &bumbledb::InstanceBuilder<()>) {
//!     let _ = builder.execute;
//! }
//! ```

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use std::sync::Arc;

use crate::error::{Admission, DynIdError, Error, Result};
use crate::ir::Value;
use crate::schema::judge::{JudgeBudget, Judgment, MapState, judge_final_state};
use crate::schema::{Schema, Theory, ValidateDescriptor as _};
use crate::work::WorkContext;
use bumbledb_theory::schema::{RelationId, StatementId};

use super::closed::ClosedRows;
use super::collection::AcceptedCollection;
use super::get as get_path;
use super::row_reader::RowReader;
use super::tx::{encode_values, row_error};
use super::{Fact, Key, MutationReport, OwnedInstance, embedded_work};

enum BuilderPhase {
    Clean,
    Poisoned(Box<Error>),
}

pub struct InstanceBuilder<S> {
    schema: Arc<Schema>,
    closed: Arc<ClosedRows>,
    work: WorkContext,
    /// The staged final set: canonical rows per relation. Set semantics by
    /// construction — the builder starts empty, so a net delta and the
    /// final state coincide.
    staged: BTreeMap<RelationId, BTreeSet<Box<[u8]>>>,
    phase: BuilderPhase,
    not_sync: PhantomData<Cell<()>>,
    marker: PhantomData<fn() -> S>,
}

impl<S: Theory> InstanceBuilder<S> {
    /// # Errors
    /// Schema validation.
    pub fn new(theory: S) -> Result<Self> {
        let schema = Arc::new(theory.descriptor().validate()?);
        let work = embedded_work()?;
        let closed = Arc::new(ClosedRows::build(schema.as_ref(), &work)?);
        Ok(Self {
            schema,
            closed,
            work,
            staged: BTreeMap::new(),
            phase: BuilderPhase::Clean,
            not_sync: PhantomData,
            marker: PhantomData,
        })
    }
}

impl<S> InstanceBuilder<S> {
    fn refuse_poisoned(&self) -> Result<()> {
        match &self.phase {
            BuilderPhase::Poisoned(source) => Err(Error::TransactionPoisoned {
                source: source.clone(),
            }),
            BuilderPhase::Clean => Ok(()),
        }
    }

    fn poison(&mut self, error: Error) -> Error {
        if self.staged.values().any(|rows| !rows.is_empty())
            && matches!(self.phase, BuilderPhase::Clean)
        {
            self.phase = BuilderPhase::Poisoned(Box::new(error.clone()));
        }
        error
    }

    fn refuse_closed(&self, relation: RelationId) -> Result<()> {
        match self.schema.relation_checked(relation) {
            Some(rel) if rel.body().closed_rows().is_some() => {
                Err(Error::ClosedRelationWrite { relation })
            }
            _ => Ok(()),
        }
    }

    /// Parse-all-first: encode the whole collection before staging any row.
    fn apply_rows(
        &mut self,
        relation: RelationId,
        rows: Vec<Vec<u8>>,
        insert: bool,
    ) -> MutationReport {
        let submitted = rows.len() as u64;
        let mut changed = 0u64;
        let staged = self.staged.entry(relation).or_default();
        for row in rows {
            let moved = if insert {
                staged.insert(row.into_boxed_slice())
            } else {
                staged.remove(row.as_slice())
            };
            if moved {
                changed += 1;
            }
        }
        MutationReport::from_counts(submitted, changed)
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

    /// The whole collection is encoded before any member is staged.
    /// # Errors
    /// Shape refusals; `TransactionPoisoned` if a prior apply failed after
    /// a prefix was staged.
    pub fn load<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        let rows = self.encode_collection(F::RELATION, facts, |builder, fact, values| {
            values.clear();
            fact.append_values(values)?;
            encode_values(builder.schema.as_ref(), F::RELATION, values, &builder.work)
        })?;
        Ok(self.apply_rows(F::RELATION, rows, true))
    }

    /// # Errors
    /// As [`InstanceBuilder::load`].
    pub fn delete<'f, F: Fact<'f, Schema = S> + 'f>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport> {
        let rows = self.encode_collection(F::RELATION, facts, |builder, fact, values| {
            values.clear();
            fact.append_values(values)?;
            encode_values(builder.schema.as_ref(), F::RELATION, values, &builder.work)
        })?;
        Ok(self.apply_rows(F::RELATION, rows, false))
    }

    /// # Errors
    /// As [`InstanceBuilder::load`], plus unknown-relation/arity/type
    /// refusals.
    pub fn load_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        let Some(coll) = self.accept_dyn(rel, facts)? else {
            return Ok(MutationReport::EMPTY);
        };
        self.apply_accepted(&coll, true)
    }

    /// # Errors
    /// As [`InstanceBuilder::load_dyn`].
    pub fn delete_dyn(
        &mut self,
        rel: RelationId,
        facts: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<MutationReport> {
        let Some(coll) = self.accept_dyn(rel, facts)? else {
            return Ok(MutationReport::EMPTY);
        };
        self.apply_accepted(&coll, false)
    }

    /// # Errors
    /// As [`InstanceBuilder::load_dyn`]; the shape proof already ran.
    #[doc(hidden)]
    pub fn load_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.apply_accepted(collection, true)
    }

    /// # Errors
    /// As [`InstanceBuilder::load_dyn`]; the shape proof already ran.
    #[doc(hidden)]
    pub fn delete_accepted(&mut self, collection: &AcceptedCollection) -> Result<MutationReport> {
        self.apply_accepted(collection, false)
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
        insert: bool,
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
            return Err(crate::error::FactShapeError::ArityMismatch {
                relation: rel,
                mismatch: crate::error::Mismatch {
                    witnessed: usize::from(coll.arity()),
                    required: relation.fields().len(),
                },
            }
            .into());
        }
        for (ordinal, (echoed, field)) in (0u16..).zip(coll.roster().iter().zip(relation.fields()))
        {
            if *echoed != field.value_type {
                return Err(crate::error::FactShapeError::TypeMismatch {
                    relation: rel,
                    field: bumbledb_theory::schema::FieldId(ordinal),
                }
                .into());
            }
        }
        let mut values = Vec::new();
        let mut rows = Vec::with_capacity(usize::try_from(coll.rows()).unwrap_or(0));
        for row in 0..coll.rows() {
            coll.row_values_into(row, &mut values);
            match encode_values(schema.as_ref(), rel, &values, &self.work) {
                Ok(bytes) => rows.push(bytes),
                Err(error) => return Err(self.poison(error)),
            }
        }
        Ok(self.apply_rows(rel, rows, insert))
    }

    /// # Errors
    /// Shape refusals.
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool> {
        self.refuse_poisoned()?;
        let mut values = Vec::new();
        fact.append_values(&mut values)?;
        self.contains_values(F::RELATION, &values)
    }

    /// # Errors
    /// Shape refusals.
    pub fn contains_dyn(&self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.refuse_poisoned()?;
        self.contains_values(rel, values)
    }

    fn contains_values(&self, relation: RelationId, values: &[Value]) -> Result<bool> {
        if let Some(rows) = self.closed.get(relation) {
            return Ok(rows.iter().any(|row| row.values.as_ref() == values));
        }
        let bytes = encode_values(self.schema.as_ref(), relation, values, &self.work)?;
        Ok(self
            .staged
            .get(&relation)
            .is_some_and(|rows| rows.contains(bytes.as_slice())))
    }

    /// # Errors
    /// Shape refusals.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `builder.get(id)`"
    )]
    pub fn get<'a, K: Key<'a, Schema = S>>(&'a self, key: K) -> Result<Option<K::Fact>> {
        self.refuse_poisoned()?;
        let relation = <K::Fact as Fact<'a>>::RELATION;
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
    /// Shape refusals.
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
    /// Shape refusals.
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
                    crate::canonical::decode(fields, bytes, &self.work).map_err(row_error)?;
                out.extend(decoded.values);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn find_by_key(
        &self,
        relation: RelationId,
        projection: &[bumbledb_theory::schema::FieldId],
        key_values: &[Value],
    ) -> Result<Option<&[u8]>> {
        let fields = self.schema.relation(relation).fields();
        let Some(rows) = self.staged.get(&relation) else {
            return Ok(None);
        };
        for row in rows {
            let decoded = crate::canonical::decode(fields, row, &self.work).map_err(row_error)?;
            if get_path::projection_matches(&decoded.values, projection, key_values) {
                return Ok(Some(row));
            }
        }
        Ok(None)
    }

    /// Run the complete production final-state judgment over the staged
    /// state and seal it into an immutable [`OwnedInstance`].
    /// # Errors
    /// `TransactionPoisoned` after a partial apply; judge refusals.
    pub fn admit(self) -> Result<Admission<OwnedInstance<S>>> {
        self.refuse_poisoned()?;
        let mut state = MapState::new();
        for (relation, rows) in &self.staged {
            let fields = self.schema.relation(*relation).fields();
            for row in rows {
                let decoded =
                    crate::canonical::decode(fields, row, &self.work).map_err(row_error)?;
                state.insert(*relation, decoded.values);
            }
        }
        match judge_final_state(
            self.schema.as_ref(),
            &state,
            &self.work,
            JudgeBudget::default(),
        )
        .map_err(super::violations::judge_refusal)?
        {
            Judgment::Rejected(violations) => Ok(Admission::Rejected(
                super::violations::violations_from_judged(
                    self.schema.as_ref(),
                    violations,
                    &self.work,
                )?,
            )),
            Judgment::Admitted => Ok(Admission::Accepted(OwnedInstance::seal(
                self.schema,
                self.closed,
                self.staged
                    .into_iter()
                    .map(|(relation, rows)| (relation, rows.into_iter().collect()))
                    .collect(),
            ))),
        }
    }
}
