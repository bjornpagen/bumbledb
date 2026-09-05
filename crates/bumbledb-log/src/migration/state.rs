//! Ordered-step evaluation state: the executor's private, work-charged
//! relation sets between plan boundaries.
//!
//! Each relation is a spill-backed exact set (canonical bytes as keys).
//! Map transforms stream compiled expressions into that set and deduplicate
//! there. [`MapSpill::finish`] does not reconstruct `Rows` / `BTreeMap`.
//! Every byte held is reserved by the core scratch owner; an exhausted
//! budget refuses instead of growing a database-sized shadow map.
//!
//! Every `validate-schema` boundary judges the COMPLETE intermediate state
//! with the core judge (C03) — a later step cannot hide an earlier invalid
//! intermediate state, and fused suffix execution is literally ordered
//! execution with one final materialization.

use std::cell::RefCell;
use std::collections::BTreeMap;

use bumbledb::canonical::{CanonicalRow, RowError};
use bumbledb::scalar::{ScalarError, ScalarEvaluator};
use bumbledb::schema::judge::{
    CandidateFacts, JudgeBudget, JudgedViolation, Judgment, judge_final_state,
};
use bumbledb::schema::{FieldDescriptor, RelationId, Schema};
use bumbledb::work::{DEFAULT_RAM_BYTES, ScratchRelation};
use bumbledb::{ReadInstance, Value, WorkContext, WorkError};

use super::compile::{CompiledAction, CompiledPlan};
use super::frame::STATE_DIGEST_DOMAIN;

/// Scratch-backed exact set for one relation. Convergent Map output
/// deduplicates here; consumers visit the spill-backed relation.
struct StagedRelation {
    scratch: RefCell<ScratchRelation>,
}

impl StagedRelation {
    fn new(work: &WorkContext) -> Self {
        Self {
            scratch: RefCell::new(ScratchRelation::new(work, DEFAULT_RAM_BYTES)),
        }
    }

    fn insert_canonical(&self, key: &[u8]) -> Result<(), StateError> {
        self.scratch
            .borrow_mut()
            .put(key, &[])
            .map_err(StateError::Core)
    }

    fn spilled(&self) -> bool {
        self.scratch.borrow().spilled()
    }

    fn visit_keys(
        &self,
        visit: &mut dyn FnMut(&[u8]) -> Result<bool, StateError>,
    ) -> Result<(), StateError> {
        let mut err = None;
        self.scratch
            .borrow_mut()
            .for_each(&mut |key, _value| match visit(key) {
                Ok(cont) => Ok(cont),
                Err(error) => {
                    err = Some(error);
                    Ok(false)
                }
            })
            .map_err(StateError::Core)?;
        if let Some(error) = err {
            return Err(error);
        }
        Ok(())
    }

    fn visit_values(
        &self,
        fields: &[FieldDescriptor],
        work: &WorkContext,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StateError>,
    ) -> Result<(), StateError> {
        let mut err = None;
        self.scratch
            .borrow_mut()
            .for_each(&mut |key, _value| {
                match bumbledb::canonical::decode(fields, key, work) {
                    Ok(decoded) => match visit(decoded.values()) {
                        Ok(cont) => Ok(cont),
                        Err(error) => {
                            err = Some(error);
                            Ok(false)
                        }
                    },
                    Err(error) => {
                        err = Some(StateError::Row(error));
                        Ok(false)
                    }
                }
            })
            .map_err(StateError::Core)?;
        if let Some(error) = err {
            return Err(error);
        }
        Ok(())
    }
}

/// Bounded Map producer: streams compiled output into a staged set.
/// `finish` is the staged set itself — never a full Rows reconstruction.
struct MapSpill {
    staged: StagedRelation,
}

impl MapSpill {
    fn new(work: &WorkContext) -> Self {
        Self {
            staged: StagedRelation::new(work),
        }
    }

    fn insert(
        &mut self,
        work: &WorkContext,
        fields: &[FieldDescriptor],
        out: &[Value],
    ) -> Result<(), StateError> {
        work.rows(1)?;
        let canonical = CanonicalRow::encode(fields, out, work)?;
        self.staged.insert_canonical(canonical.as_bytes())
    }

    fn finish(self) -> StagedRelation {
        self.staged
    }
}

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

/// The complete private state between plan boundaries.
pub struct MigrationState {
    relations: BTreeMap<RelationId, StagedRelation>,
}

impl MigrationState {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            relations: BTreeMap::new(),
        }
    }

    /// True when any relation has crossed into the scratch tier.
    #[must_use]
    pub fn spilled(&self) -> bool {
        self.relations.values().any(StagedRelation::spilled)
    }

    /// Capture the complete frozen source: every ORDINARY relation's rows
    /// through one coherent read lease, streamed into spill-backed sets.
    /// Closed relations are schema axioms and are never captured or copied.
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
            let staged = StagedRelation::new(work);
            for row in read.scan(id)? {
                let values = row?;
                work.rows(1)?;
                let canonical = CanonicalRow::encode(fields, &values, work)?;
                staged.insert_canonical(canonical.as_bytes())?;
            }
            state.relations.insert(id, staged);
        }
        Ok(state)
    }

    /// Apply one compiled plan: ordered actions over this complete state,
    /// then the terminal complete-schema judgment. Expressions were compiled
    /// under verified source/target schemas before this iteration, including
    /// the zero-row case. Consumes the input state.
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
                    next.relations
                        .entry(*target)
                        .or_insert_with(|| StagedRelation::new(work));
                }
                CompiledAction::Drop { .. } => {}
                CompiledAction::Seed { target, rows } => {
                    let fields = relation_fields(&compiled.to, *target);
                    let staged = next
                        .relations
                        .entry(*target)
                        .or_insert_with(|| StagedRelation::new(work));
                    for row in rows {
                        work.rows(1)?;
                        let canonical = CanonicalRow::encode(fields, row, work)?;
                        staged.insert_canonical(canonical.as_bytes())?;
                    }
                }
                CompiledAction::Map {
                    source,
                    target,
                    expressions,
                } => {
                    let source_fields = relation_fields(&compiled.from, *source);
                    let target_fields = relation_fields(&compiled.to, *target);
                    let mut spill = MapSpill::new(work);
                    if let Some(input) = self.relations.get(source) {
                        input.visit_values(source_fields, work, &mut |values| {
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
                            spill.insert(work, target_fields, &out)?;
                            Ok(true)
                        })?;
                    }
                    next.relations.insert(*target, spill.finish());
                }
            }
        }
        drop(self);
        let judged = {
            let bound = JudgeBound {
                state: &next,
                schema: &compiled.to,
                work,
            };
            judge_final_state(&compiled.to, &bound, work, JudgeBudget::default())
        };
        match judged {
            Ok(Judgment::Admitted) => Ok(next),
            Ok(Judgment::Rejected(violations)) => Err(StateError::Rejected {
                schema: compiled.to_id,
                violations,
            }),
            Err(error) => Err(match error {
                bumbledb::schema::judge::JudgeError::Work(work) => StateError::Work(work),
                bumbledb::schema::judge::JudgeError::State(state) => state,
                other => StateError::Judge(format!("{other:?}")),
            }),
        }
    }

    /// Visit one relation's canonical keys in set order (spill-backed).
    /// # Errors
    /// Scratch visit failures.
    pub fn visit_canonical(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[u8]) -> Result<bool, StateError>,
    ) -> Result<(), StateError> {
        if let Some(staged) = self.relations.get(&relation) {
            staged.visit_keys(visit)?;
        }
        Ok(())
    }

    /// Visit one relation's decoded rows in canonical order.
    /// # Errors
    /// Decode or scratch visit failures.
    pub fn visit_rows(
        &self,
        relation: RelationId,
        fields: &[FieldDescriptor],
        work: &WorkContext,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StateError>,
    ) -> Result<(), StateError> {
        if let Some(staged) = self.relations.get(&relation) {
            staged.visit_values(fields, work, visit)?;
        }
        Ok(())
    }

    /// The canonical application-state digest (C11 `targetDigest`): the
    /// ordered enumeration of every ordinary relation's canonical keys.
    /// Closed relations are sealed in the schema identity and excluded.
    /// # Errors
    /// Scratch visit failures.
    pub fn digest(&self) -> Result<[u8; 32], StateError> {
        let mut hasher = blake3::Hasher::new_derive_key(STATE_DIGEST_DOMAIN);
        hasher.update(&(self.relations.len() as u64).to_be_bytes());
        for (relation, rows) in &self.relations {
            hasher.update(&relation.0.to_be_bytes());
            let mut count = 0u64;
            rows.visit_keys(&mut |_| {
                count += 1;
                Ok(true)
            })?;
            hasher.update(&count.to_be_bytes());
            rows.visit_keys(&mut |key| {
                hasher.update(&(key.len() as u64).to_be_bytes());
                hasher.update(key);
                Ok(true)
            })?;
        }
        Ok(*hasher.finalize().as_bytes())
    }
}

/// Schema-bound candidate view so complete judgment can decode spill keys.
struct JudgeBound<'a> {
    state: &'a MigrationState,
    schema: &'a Schema,
    work: &'a WorkContext,
}

impl CandidateFacts for JudgeBound<'_> {
    type Error = StateError;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        let fields = relation_fields(self.schema, relation);
        self.state.visit_rows(relation, fields, self.work, visit)
    }
}

fn relation_fields(schema: &Schema, relation: RelationId) -> &[FieldDescriptor] {
    schema.relation(relation).fields()
}
