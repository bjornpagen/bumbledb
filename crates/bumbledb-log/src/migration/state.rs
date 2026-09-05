//! Ordered-step evaluation state: the executor's private, work-charged
//! in-memory relation sets between plan boundaries.
//!
//! Rows are keyed by their FULL canonical bytes (set semantics, canonical
//! iteration order) and carry their decoded values for expression
//! evaluation. Every byte held is reserved against the operation's
//! `WorkContext`; an exhausted budget refuses instead of growing — the
//! bounded RAM→temporary-LMDB scratch spill for >RAM intermediates is the
//! core scratch facility's seam (C05, P03; recorded dependency), not a
//! reason to hold unbounded memory here.
//!
//! Every `validate-schema` boundary judges the COMPLETE intermediate state
//! with the core judge (C03) — a later step cannot hide an earlier invalid
//! intermediate state, and fused suffix execution is literally ordered
//! execution with one final materialization.

use std::collections::BTreeMap;

use bumbledb::canonical::{CanonicalRow, RowError};
use bumbledb::scalar::{ScalarError, ScalarEvaluator};
use bumbledb::schema::judge::{
    CandidateFacts, JudgeBudget, JudgedViolation, Judgment, judge_final_state,
};
use bumbledb::schema::{RelationId, Schema};
use bumbledb::work::{ByteKind, ByteReservation};
use bumbledb::{ReadInstance, Value, WorkContext, WorkError};

use super::compile::{CompiledAction, CompiledPlan};
use super::frame::STATE_DIGEST_DOMAIN;

/// One mapped output row: canonical key bytes plus the decoded values.
type ProducedRow = (Box<[u8]>, Box<[Value]>);

/// Why state construction or a step refused.
#[derive(Debug)]
pub enum StateError {
    Work(WorkError),
    /// A core read/scan failure with its typed cause.
    Core(bumbledb::Error),
    /// A row refused canonical encoding (shape/type drift — corruption).
    Row(RowError),
    /// An expression refused on an actual row: the step's error boundary.
    Scalar {
        relation: RelationId,
        error: ScalarError,
    },
    /// The intermediate/final state violated its schema's laws: the exact
    /// judged violations, never a partial diagnosis.
    Rejected {
        schema: crate::history::SchemaId,
        violations: Box<[JudgedViolation]>,
    },
    /// The judge could not complete (work/overflow/ray) — a resource
    /// refusal, never a shorter rejection.
    Judge(String),
}

impl From<WorkError> for StateError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl From<bumbledb::Error> for StateError {
    fn from(error: bumbledb::Error) -> Self {
        Self::Core(error)
    }
}

impl From<RowError> for StateError {
    fn from(error: RowError) -> Self {
        Self::Row(error)
    }
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Work(error) => write!(f, "migration state: {error:?}"),
            Self::Core(error) => write!(f, "migration state: {error}"),
            Self::Row(error) => write!(f, "migration state row: {error:?}"),
            Self::Scalar { relation, error } => {
                write!(
                    f,
                    "migration expression on relation {}: {error}",
                    relation.0
                )
            }
            Self::Rejected { violations, .. } => {
                write!(
                    f,
                    "migration state rejected ({} statements)",
                    violations.len()
                )
            }
            Self::Judge(why) => write!(f, "migration judgment incomplete: {why}"),
        }
    }
}

impl std::error::Error for StateError {}

/// One relation's proposed rows: canonical bytes -> decoded values.
type Rows = BTreeMap<Box<[u8]>, Box<[Value]>>;

/// The complete private state between plan boundaries.
pub struct MigrationState {
    relations: BTreeMap<RelationId, Rows>,
    /// Linear reservations for every byte the maps hold.
    charges: Vec<ByteReservation>,
}

impl MigrationState {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            relations: BTreeMap::new(),
            charges: Vec::new(),
        }
    }

    /// Capture the complete frozen source: every ORDINARY relation's rows
    /// through one coherent read lease. Closed relations are schema axioms
    /// and are never captured or copied.
    /// # Errors
    /// Scan/read failures and exhausted work.
    pub fn from_source<S>(
        read: &ReadInstance<'_, S>,
        schema: &Schema,
        work: &WorkContext,
    ) -> Result<Self, StateError> {
        let mut state = Self::empty();
        for (index, relation) in schema.relations().iter().enumerate() {
            if relation.body().closed_rows().is_some() {
                continue;
            }
            let id = RelationId(u32::try_from(index).expect("validated relation count"));
            let fields = relation.fields();
            let mut rows = Rows::new();
            for row in read.scan(id)? {
                let values = row?;
                work.rows(1)?;
                let canonical = CanonicalRow::encode(fields, &values, work)?;
                state.charge(work, &canonical, &values)?;
                rows.insert(Box::from(canonical.as_bytes()), values.into_boxed_slice());
            }
            state.relations.insert(id, rows);
        }
        Ok(state)
    }

    fn charge(
        &mut self,
        work: &WorkContext,
        canonical: &CanonicalRow,
        values: &[Value],
    ) -> Result<(), WorkError> {
        // The canonical bytes plus a conservative per-value share for the
        // decoded copy. CanonicalRow's own reservation dies with it; the
        // state holds its own linear charge for what it retains.
        let bytes = (canonical.as_bytes().len() as u64)
            .saturating_mul(2)
            .saturating_add(48 * values.len() as u64)
            .saturating_add(64);
        self.charges.push(work.reserve(ByteKind::Working, bytes)?);
        Ok(())
    }

    /// Apply one compiled plan: ordered actions over this complete state,
    /// then the terminal complete-schema judgment. Consumes the input state
    /// (its reservations release as it drops).
    /// # Errors
    /// The step's exact error boundary: expression refusal on an actual
    /// row, exhausted work, or the complete judged rejection.
    pub fn apply(
        self,
        compiled: &CompiledPlan,
        evaluator: &ScalarEvaluator,
        work: &WorkContext,
    ) -> Result<Self, StateError> {
        let mut next = Self::empty();
        for action in &compiled.actions {
            match action {
                CompiledAction::Empty { target } => {
                    next.relations.entry(*target).or_default();
                }
                CompiledAction::Drop { .. } => {}
                CompiledAction::Seed { target, rows } => {
                    let fields = relation_fields(&compiled.to, *target);
                    let bucket = next.relations.entry(*target).or_default();
                    for row in rows {
                        work.rows(1)?;
                        let canonical = CanonicalRow::encode(fields, row, work)?;
                        let key: Box<[u8]> = Box::from(canonical.as_bytes());
                        let bytes = (key.len() as u64)
                            .saturating_mul(2)
                            .saturating_add(48 * row.len() as u64)
                            .saturating_add(64);
                        next.charges.push(work.reserve(ByteKind::Working, bytes)?);
                        bucket.insert(key, row.clone());
                    }
                }
                CompiledAction::Map {
                    source,
                    target,
                    expressions,
                } => {
                    let fields = relation_fields(&compiled.to, *target);
                    let empty = Rows::new();
                    let input = self.relations.get(source).unwrap_or(&empty);
                    let mut produced: Vec<ProducedRow> = Vec::new();
                    for values in input.values() {
                        work.rows(1)?;
                        work.step(expressions.len() as u64)?;
                        let mut out = Vec::with_capacity(expressions.len());
                        for expression in expressions {
                            let value = evaluator
                                .evaluate(expression, |var| {
                                    values
                                        .get(usize::from(var.0))
                                        .cloned()
                                        .ok_or(ScalarError::UnboundVariable(var))
                                })
                                .map_err(|error| StateError::Scalar {
                                    relation: *target,
                                    error,
                                })?;
                            out.push(value);
                        }
                        let canonical = CanonicalRow::encode(fields, &out, work)?;
                        produced.push((Box::from(canonical.as_bytes()), out.into_boxed_slice()));
                    }
                    let bucket = next.relations.entry(*target).or_default();
                    for (key, values) in produced {
                        let bytes = (key.len() as u64)
                            .saturating_mul(2)
                            .saturating_add(48 * values.len() as u64)
                            .saturating_add(64);
                        next.charges.push(work.reserve(ByteKind::Working, bytes)?);
                        bucket.insert(key, values);
                    }
                }
            }
        }
        drop(self);
        // The mandatory validate boundary: judge the COMPLETE state against
        // the step's target schema with the core judge.
        match judge_final_state(&compiled.to, &next, work, JudgeBudget::default()) {
            Ok(Judgment::Admitted) => Ok(next),
            Ok(Judgment::Rejected(violations)) => Err(StateError::Rejected {
                schema: compiled.to_id,
                violations,
            }),
            Err(error) => Err(match error {
                bumbledb::schema::judge::JudgeError::Work(work) => StateError::Work(work),
                other => StateError::Judge(format!("{other:?}")),
            }),
        }
    }

    /// Iterate one relation's rows in canonical order.
    pub fn rows_of(&self, relation: RelationId) -> impl Iterator<Item = &[Value]> {
        self.relations
            .get(&relation)
            .into_iter()
            .flat_map(|rows| rows.values().map(AsRef::as_ref))
    }

    /// The canonical application-state digest (C11 `targetDigest`): the
    /// ordered enumeration of every ordinary relation's canonical rows.
    /// Closed relations are sealed in the schema identity and excluded.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new_derive_key(STATE_DIGEST_DOMAIN);
        hasher.update(&(self.relations.len() as u64).to_be_bytes());
        for (relation, rows) in &self.relations {
            hasher.update(&relation.0.to_be_bytes());
            hasher.update(&(rows.len() as u64).to_be_bytes());
            for key in rows.keys() {
                hasher.update(&(key.len() as u64).to_be_bytes());
                hasher.update(key);
            }
        }
        *hasher.finalize().as_bytes()
    }
}

impl CandidateFacts for MigrationState {
    type Error = std::convert::Infallible;

    fn rows(
        &self,
        relation: RelationId,
    ) -> Box<dyn Iterator<Item = Result<Box<[Value]>, Self::Error>> + '_> {
        match self.relations.get(&relation) {
            Some(rows) => Box::new(rows.values().map(|values| Ok(values.clone()))),
            None => Box::new(std::iter::empty()),
        }
    }
}

fn relation_fields(schema: &Schema, relation: RelationId) -> &[bumbledb::schema::FieldDescriptor] {
    schema.relation(relation).fields()
}
