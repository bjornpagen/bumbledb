//! The reference final-state judge (contract C03).
//!
//! Admission judges one **proposed final state** — the committed snapshot
//! plus the normalized delta — against every sealed statement, BEFORE any
//! physical unique index installs a row. The judge therefore sees every
//! competing candidate row: two proposed rows fighting over one key both
//! appear in its universe and both are cited, which a first-wins physical
//! index cannot do (ENG-005, E-ADMIT).
//!
//! This module is the semantic reference: a complete, independent
//! evaluation over decoded canonical values. Exact value equality decides
//! identity — no fingerprint or hash participates, so a forced fingerprint
//! collision cannot alter a verdict here by construction. The physical
//! commit path (P02) implements the same judgment sum over LMDB with
//! accelerating indexes and must agree with this evaluator; P11's models
//! treat this as the executable denotation, not as an oracle for itself.
//!
//! A completed rejection returns the COMPLETE set of violated statement
//! ids, each with a bounded, explicitly labeled number of example facts.
//! If the work allowance expires before all statements are judged, the
//! judge returns a resource error — never a falsely complete rejection.
use crate::schema::{
    CapacityStatement, ContainmentStatement, KeyStatement, RelationId, Schema, SealedBound,
    SealedWeight, Side, StatementId, StatementKind, StatementView,
};
use crate::{Value, WorkContext, WorkError};

/// One proposed final-state view: for every ORDINARY relation, the judge
/// can visit each distinct proposed row (committed minus removed plus
/// added), decoded to sealed-field-order values. Closed relations are not
/// consulted — their ground axioms are sealed in the schema. The state
/// must present set semantics (no duplicate full rows); the change-set
/// normalization already guarantees that for the delta side.
pub trait CandidateFacts {
    /// The state's own iteration/decoding failure (I/O, corruption…),
    /// distinct from every semantic outcome.
    type Error;

    /// Visit `relation`'s proposed final rows in any order.
    fn rows(&self, relation: RelationId) -> CandidateRows<'_, Self::Error>;
}

/// One relation's proposed final rows, decoded — [`CandidateFacts::rows`]'s
/// item stream.
pub type CandidateRows<'a, E> = Box<dyn Iterator<Item = Result<Box<[Value]>, E>> + 'a>;

/// A tiny owned map state — the reference implementation of
/// [`CandidateFacts`] used by oracles, models and the judge's own tests.
#[derive(Debug, Default)]
pub struct MapState {
    relations: std::collections::BTreeMap<RelationId, Vec<Box<[Value]>>>,
}

impl MapState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts one proposed final row; exact duplicates collapse (a set).
    pub fn insert(&mut self, relation: RelationId, values: Vec<Value>) {
        let rows = self.relations.entry(relation).or_default();
        if !rows.iter().any(|row| row.as_ref() == values.as_slice()) {
            rows.push(values.into_boxed_slice());
        }
    }
}

impl CandidateFacts for MapState {
    type Error = std::convert::Infallible;

    fn rows(
        &self,
        relation: RelationId,
    ) -> Box<dyn Iterator<Item = Result<Box<[Value]>, Self::Error>> + '_> {
        match self.relations.get(&relation) {
            Some(rows) => Box::new(rows.iter().map(|row| Ok(row.clone()))),
            None => Box::new(std::iter::empty()),
        }
    }
}

/// One cited example fact: a relation and its decoded sealed-order values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFact {
    pub relation: RelationId,
    pub values: Box<[Value]>,
}

/// Which side of a containment failed. Mirrors the engine's diagnostic
/// direction: the reference judge evaluates each one-way statement, so the
/// source side is always the unsatisfied one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudgedDirection {
    SourceUnsatisfied,
}

/// One violated statement with bounded, explicitly labeled examples.
/// `statement` is the stable materialized-order identity every surface
/// cites; `examples_truncated` says whether more offending facts exist
/// beyond the budget — the judge never silently narrows a diagnosis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgedViolation {
    pub statement: StatementId,
    pub kind: StatementKind,
    pub direction: Option<JudgedDirection>,
    /// The witnessed grouped measure for a capacity violation: the exact
    /// widened total, never narrowed or wrapped. One violation row exists
    /// per statement; when several groups violate the same statement, the
    /// recorded witness is the LAST violating group in the deterministic
    /// target iteration order (a labeled example witness, not a claim of
    /// uniqueness) — a confirmed P00 decision, load-bearing for byte-exact
    /// evidence replay.
    pub measure: Option<u128>,
    pub examples: Box<[CandidateFact]>,
    pub examples_truncated: bool,
}

/// The complete judgment sum: admitted, or the complete ordered set of
/// violated statements. Resource exhaustion and state failure are the
/// `Err` channel of [`judge_final_state`], never a third verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgment {
    Admitted,
    /// Nonempty, ordered by statement id — canonical diagnostic order.
    Rejected(Box<[JudgedViolation]>),
}

/// Judgment failure: not a semantic verdict. `Work` is the operation
/// allowance; `State` is the candidate view's own failure;
/// `UndefinedDuration` is the explicit refusal of a ray in a
/// duration-measured position; `MeasureOverflow` reports a group total
/// past the widened accumulator instead of wrapping a witness.
#[derive(Debug, PartialEq, Eq)]
pub enum JudgeError<E> {
    Work(WorkError),
    State(E),
    UndefinedDuration { statement: StatementId },
    MeasureOverflow { statement: StatementId },
}

impl<E> From<WorkError> for JudgeError<E> {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

/// Diagnostic bounds. `examples_per_statement` is the explicitly labeled
/// example budget; zero keeps verdicts complete with no cited facts.
#[derive(Debug, Clone, Copy)]
pub struct JudgeBudget {
    pub examples_per_statement: usize,
}

impl Default for JudgeBudget {
    fn default() -> Self {
        Self {
            examples_per_statement: 4,
        }
    }
}

/// Judges the complete proposed final state against every sealed
/// statement. See the module doc for the exact semantics; the verdict for
/// a completed run always names EVERY violated statement.
///
/// # Errors
/// [`JudgeError`] on exhausted work, a failing candidate view, a ray in a
/// duration-measured position, or an unwitnessable measure. No partial
/// rejection is returned on any error path.
pub fn judge_final_state<S: CandidateFacts>(
    schema: &Schema,
    state: &S,
    work: &WorkContext,
    budget: JudgeBudget,
) -> Result<Judgment, JudgeError<S::Error>> {
    let mut judge = Judge {
        schema,
        work,
        budget,
        loaded: std::collections::BTreeMap::new(),
        violations: Vec::new(),
    };
    for view in schema.statements() {
        work.step(1)?;
        if schema.closed_constant(view) {
            // Sealed at validation over frozen ground axioms; nothing in a
            // candidate state can change it.
            continue;
        }
        match view {
            StatementView::Key(_, statement) => judge.key(state, statement)?,
            StatementView::Containment(_, statement) => judge.containment(state, statement)?,
            StatementView::Capacity(_, statement) => judge.capacity(state, statement)?,
        }
    }
    let mut violations = judge.violations;
    violations.sort_by_key(|violation| violation.statement);
    if violations.is_empty() {
        Ok(Judgment::Admitted)
    } else {
        Ok(Judgment::Rejected(violations.into_boxed_slice()))
    }
}

struct Judge<'s, 'w> {
    schema: &'s Schema,
    work: &'w WorkContext,
    budget: JudgeBudget,
    /// Rows loaded per relation for this judgment, decoded once. Closed
    /// relations load from the sealed extension, never the state.
    loaded: std::collections::BTreeMap<RelationId, std::rc::Rc<Vec<Box<[Value]>>>>,
    violations: Vec<JudgedViolation>,
}

/// One relation's rows, decoded and cached for the judgment walk.
type LoadedRows = std::rc::Rc<Vec<Box<[Value]>>>;

impl Judge<'_, '_> {
    fn rows<S: CandidateFacts>(
        &mut self,
        state: &S,
        relation: RelationId,
    ) -> Result<LoadedRows, JudgeError<S::Error>> {
        if let Some(rows) = self.loaded.get(&relation) {
            return Ok(std::rc::Rc::clone(rows));
        }
        let sealed = self.schema.relation(relation);
        let mut rows = Vec::new();
        if let Some(extension) = sealed.body().closed_rows() {
            for row in extension {
                self.work.step(1)?;
                let decoded =
                    crate::encoding::decode_values(sealed.layout().encoded(&row.fact), |_| {
                        unreachable!("closed relations refuse str columns")
                    })
                    .expect("sealed extension rows decode by construction");
                rows.push(decoded.into_boxed_slice());
            }
        } else {
            for row in state.rows(relation) {
                self.work.step(1)?;
                rows.push(row.map_err(JudgeError::State)?);
            }
        }
        let rows = std::rc::Rc::new(rows);
        self.loaded.insert(relation, std::rc::Rc::clone(&rows));
        Ok(rows)
    }

    fn cite(&mut self, pending: &mut PendingViolation, relation: RelationId, values: &[Value]) {
        if pending.examples.len() >= self.budget.examples_per_statement {
            pending.truncated = true;
        } else {
            let already = pending
                .examples
                .iter()
                .any(|fact| fact.relation == relation && fact.values.as_ref() == values);
            if !already {
                pending.examples.push(CandidateFact {
                    relation,
                    values: values.to_vec().into_boxed_slice(),
                });
            }
        }
    }

    fn key<S: CandidateFacts>(
        &mut self,
        state: &S,
        statement: &KeyStatement,
    ) -> Result<(), JudgeError<S::Error>> {
        let relation = statement.relation;
        let rows = self.rows(state, relation)?;
        // Split the projection into the scalar determinant and an optional
        // trailing interval position (the pointwise form).
        let fields = self.schema.relation(relation).fields();
        let (scalar_fields, interval_field): (Vec<usize>, Option<usize>) = {
            let mut scalars = Vec::new();
            let mut interval = None;
            for field in &statement.projection {
                let idx = usize::from(field.0);
                if fields[idx].value_type.is_interval() {
                    interval = Some(idx);
                } else {
                    scalars.push(idx);
                }
            }
            (scalars, interval)
        };
        let mut pending = PendingViolation::new(statement.id, StatementKind::Functionality);
        // A determinant multimap over the WHOLE candidate: every competing
        // row lands in its bucket before any uniqueness is enforced.
        let mut buckets: std::collections::BTreeMap<OrdValues, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (row_idx, row) in rows.iter().enumerate() {
            self.work.step(1)?;
            let determinant = OrdValues(
                scalar_fields
                    .iter()
                    .map(|&idx| row[idx].clone())
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            );
            buckets.entry(determinant).or_default().push(row_idx);
        }
        for bucket in buckets.values() {
            self.work.step(bucket.len() as u64)?;
            match interval_field {
                None => {
                    if bucket.len() > 1 {
                        for &row_idx in bucket {
                            self.cite(&mut pending, relation, &rows[row_idx]);
                        }
                        if bucket.len() > self.budget.examples_per_statement {
                            pending.truncated = true;
                        }
                        pending.violated = true;
                    }
                }
                Some(tail) => {
                    // Pointwise: two rows with one determinant may coexist
                    // only with disjoint interval tails.
                    let mut spans: Vec<(u64, u64, usize)> = Vec::new();
                    for &row_idx in bucket {
                        let (start, end) = interval_order_words(&rows[row_idx][tail])
                            .expect("a projected interval position holds an interval value");
                        spans.push((start, end, row_idx));
                    }
                    spans.sort_unstable();
                    for pair in spans.windows(2) {
                        self.work.step(1)?;
                        let (a_start, a_end, a_idx) = pair[0];
                        let (b_start, _, b_idx) = pair[1];
                        debug_assert!(a_start <= b_start);
                        if b_start < a_end {
                            pending.violated = true;
                            self.cite(&mut pending, relation, &rows[a_idx]);
                            self.cite(&mut pending, relation, &rows[b_idx]);
                        }
                    }
                }
            }
        }
        self.finish(pending);
        Ok(())
    }

    fn containment<S: CandidateFacts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
    ) -> Result<(), JudgeError<S::Error>> {
        let source_rows = self.rows(state, statement.source.relation)?;
        let target_rows = self.rows(state, statement.target.relation)?;
        let target_fields = self.schema.relation(statement.target.relation).fields();
        // At most one trailing interval position (validation's rule); its
        // presence selects pointwise coverage instead of tuple existence.
        let coverage_position = statement
            .target
            .projection
            .iter()
            .position(|field| target_fields[usize::from(field.0)].value_type.is_interval());
        let mut pending = PendingViolation::new(statement.id, StatementKind::Containment)
            .with_direction(JudgedDirection::SourceUnsatisfied);
        for source_row in source_rows.iter() {
            self.work.step(1)?;
            if !satisfies(&statement.source, source_row) {
                continue;
            }
            let projected = project(&statement.source, source_row);
            let mut witnessed = false;
            match coverage_position {
                None => {
                    for target_row in target_rows.iter() {
                        self.work.step(1)?;
                        if satisfies(&statement.target, target_row)
                            && project(&statement.target, target_row) == projected
                        {
                            witnessed = true;
                            break;
                        }
                    }
                }
                Some(position) => {
                    // Scalar prefix equality plus pointwise coverage of the
                    // source span by the union of matching target spans.
                    let (span_start, span_end) = interval_order_words(&projected[position])
                        .expect("positional typing pairs interval positions");
                    let mut covers: Vec<(u64, u64)> = Vec::new();
                    for target_row in target_rows.iter() {
                        self.work.step(1)?;
                        if !satisfies(&statement.target, target_row) {
                            continue;
                        }
                        let candidate = project(&statement.target, target_row);
                        let scalars_agree = candidate
                            .iter()
                            .zip(projected.iter())
                            .enumerate()
                            .all(|(idx, (a, b))| idx == position || a == b);
                        if !scalars_agree {
                            continue;
                        }
                        let (start, end) = interval_order_words(&candidate[position])
                            .expect("positional typing pairs interval positions");
                        covers.push((start, end));
                    }
                    covers.sort_unstable();
                    let mut frontier = span_start;
                    for (start, end) in covers {
                        self.work.step(1)?;
                        if start > frontier {
                            break;
                        }
                        frontier = frontier.max(end);
                        if frontier >= span_end {
                            break;
                        }
                    }
                    witnessed = frontier >= span_end;
                }
            }
            if !witnessed {
                pending.violated = true;
                self.cite(&mut pending, statement.source.relation, source_row);
            }
        }
        self.finish(pending);
        Ok(())
    }

    fn capacity<S: CandidateFacts>(
        &mut self,
        state: &S,
        statement: &CapacityStatement,
    ) -> Result<(), JudgeError<S::Error>> {
        let source_rows = self.rows(state, statement.source.relation)?;
        let target_rows = self.rows(state, statement.target.relation)?;
        let mut pending = PendingViolation::new(statement.id, StatementKind::Capacity);
        for target_row in target_rows.iter() {
            self.work.step(1)?;
            if !satisfies(&statement.target, target_row) {
                continue;
            }
            let group = project(&statement.target, target_row);
            // The exact nonnegative measure over DISTINCT matching source
            // facts, accumulated widened; an existing selected parent with
            // no children has total zero.
            let mut total: u128 = 0;
            for source_row in source_rows.iter() {
                self.work.step(1)?;
                if !satisfies(&statement.source, source_row) {
                    continue;
                }
                if project(&statement.source, source_row) != group {
                    continue;
                }
                let weight = match statement.weight {
                    SealedWeight::Unit => 1,
                    SealedWeight::Field(field) => match &source_row[usize::from(field.0)] {
                        Value::U64(weight) => u128::from(*weight),
                        _ => unreachable!("validation types a [field] weight as u64"),
                    },
                    SealedWeight::Duration { field, .. } => u128::from(duration_of(
                        &source_row[usize::from(field.0)],
                        statement.id,
                    )?),
                };
                total = total
                    .checked_add(weight)
                    .ok_or(JudgeError::MeasureOverflow {
                        statement: statement.id,
                    })?;
            }
            let ceiling: Option<u128> = match statement.hi {
                SealedBound::Unbounded => None,
                SealedBound::Lit(hi) => Some(u128::from(hi)),
                SealedBound::TargetField(field) => match &target_row[usize::from(field.0)] {
                    Value::U64(hi) => Some(u128::from(*hi)),
                    _ => unreachable!("validation types a dependent bound as u64"),
                },
                SealedBound::Duration { field, .. } => Some(u128::from(duration_of(
                    &target_row[usize::from(field.0)],
                    statement.id,
                )?)),
            };
            let below = total < u128::from(statement.lo);
            let above = ceiling.is_some_and(|hi| total > hi);
            if below || above {
                pending.violated = true;
                // One violation per statement; the witnessed measure is the
                // last violating group's exact total in this deterministic
                // target order (P00-confirmed witness rule; the cited target
                // rows label every violating group up to the budget).
                pending.measure = Some(total);
                self.cite(&mut pending, statement.target.relation, target_row);
                if above {
                    // Cite offending children up to the budget so the group
                    // evidence is inspectable; truncation is labeled.
                    for source_row in source_rows.iter() {
                        self.work.step(1)?;
                        if pending.examples.len() >= self.budget.examples_per_statement {
                            pending.truncated = true;
                            break;
                        }
                        if satisfies(&statement.source, source_row)
                            && project(&statement.source, source_row) == group
                        {
                            self.cite(&mut pending, statement.source.relation, source_row);
                        }
                    }
                }
            }
        }
        self.finish(pending);
        Ok(())
    }

    fn finish(&mut self, pending: PendingViolation) {
        if pending.violated {
            self.violations.push(JudgedViolation {
                statement: pending.statement,
                kind: pending.kind,
                direction: pending.direction,
                measure: pending.measure,
                examples: pending.examples.into_boxed_slice(),
                examples_truncated: pending.truncated,
            });
        }
    }
}

struct PendingViolation {
    statement: StatementId,
    kind: StatementKind,
    direction: Option<JudgedDirection>,
    measure: Option<u128>,
    examples: Vec<CandidateFact>,
    truncated: bool,
    violated: bool,
}

impl PendingViolation {
    fn new(statement: StatementId, kind: StatementKind) -> Self {
        Self {
            statement,
            kind,
            direction: None,
            measure: None,
            examples: Vec::new(),
            truncated: false,
            violated: false,
        }
    }

    fn with_direction(mut self, direction: JudgedDirection) -> Self {
        self.direction = Some(direction);
        self
    }
}

/// Does the row satisfy the side's selection (each bound field's value is
/// a member of its literal set)? Exact canonical value equality, only.
fn satisfies(side: &Side, row: &[Value]) -> bool {
    side.selection.iter().all(|(field, literals)| {
        let actual = &row[usize::from(field.0)];
        literals.literals().iter().any(|literal| literal == actual)
    })
}

fn project(side: &Side, row: &[Value]) -> Vec<Value> {
    side.projection
        .iter()
        .map(|field| row[usize::from(field.0)].clone())
        .collect()
}

/// The exact integer duration of a discrete interval value; a ray refuses
/// (undefined measure), and a float interval is unreachable — validation
/// refuses float-duration weights and bounds.
fn duration_of<E>(value: &Value, statement: StatementId) -> Result<u64, JudgeError<E>> {
    let duration = match value {
        Value::IntervalU64(interval) => interval.duration(),
        Value::IntervalI64(interval) => interval.duration(),
        Value::IntervalF64(_) => {
            unreachable!("validation refuses float intervals in duration positions")
        }
        _ => unreachable!("validation types duration positions as intervals"),
    };
    duration.ok_or(JudgeError::UndefinedDuration { statement })
}

/// One order-preserving u64 word per endpoint, shared across the three
/// element domains: the same total endpoint order the physical index words
/// use, so the reference sweep and the engine kernels agree by definition.
fn interval_order_words(value: &Value) -> Option<(u64, u64)> {
    match value {
        Value::IntervalU64(interval) => Some((interval.start(), interval.end())),
        Value::IntervalI64(interval) => Some((
            u64::from_be_bytes(crate::encoding::encode_i64(interval.start())),
            u64::from_be_bytes(crate::encoding::encode_i64(interval.end())),
        )),
        Value::IntervalF64(interval) => Some((
            interval.start().to_order_key(),
            interval.end().to_order_key(),
        )),
        _ => None,
    }
}

/// A total order over decoded value tuples so grouping is deterministic;
/// used only as a map key, never exposed as value semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OrdValues(Box<[Value]>);

impl Ord for OrdValues {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let by_len = self.0.len().cmp(&other.0.len());
        if by_len != std::cmp::Ordering::Equal {
            return by_len;
        }
        for (a, b) in self.0.iter().zip(other.0.iter()) {
            let order = value_cmp(a, b);
            if order != std::cmp::Ordering::Equal {
                return order;
            }
        }
        std::cmp::Ordering::Equal
    }
}

impl PartialOrd for OrdValues {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn value_cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    fn rank(value: &Value) -> u8 {
        match value {
            Value::Bool(_) => 0,
            Value::U64(_) => 1,
            Value::I64(_) => 2,
            Value::String(_) => 3,
            Value::FixedBytes(_) => 4,
            Value::IntervalU64(_) => 5,
            Value::IntervalI64(_) => 6,
            Value::F64(_) => 7,
            Value::Id128(_) => 8,
            Value::IntervalF64(_) => 9,
        }
    }
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::U64(x), Value::U64(y)) => x.cmp(y),
        (Value::I64(x), Value::I64(y)) => x.cmp(y),
        (Value::F64(x), Value::F64(y)) => x.cmp(y),
        (Value::Id128(x), Value::Id128(y)) => x.cmp(y),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::FixedBytes(x), Value::FixedBytes(y)) => x.cmp(y),
        (Value::IntervalU64(x), Value::IntervalU64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        (Value::IntervalI64(x), Value::IntervalI64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        (Value::IntervalF64(x), Value::IntervalF64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        _ => rank(a).cmp(&rank(b)),
    }
}

#[cfg(test)]
mod tests;
