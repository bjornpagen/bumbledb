//! Judgment of a proposed final state. [`judge_complete`] judges every
//! statement over the whole state; [`judge_incremental`] assumes a lawful
//! parent and judges only the groups the delta can affect, through the state's
//! group indexes when it has them. Both name every violated statement, citing
//! examples chosen by canonical fact bytes before the budget truncates.
use std::ops::ControlFlow::{self, Break, Continue};

use crate::canonical::append_value;
use crate::changes::DeltaShape;
use crate::error::Result;
use crate::schema::compiled::{CompiledProjection, CompiledTheory, ProjectionBinding};
use crate::schema::{RelationId, Schema, Side, StatementId, StatementKind, StatementView};
use crate::{Value, WorkContext};

mod capacity;
mod citation;
mod containment;
mod grouped;
mod key;

use citation::CitationTopK;
use grouped::{GroupedMap, ScalarKeyScratch};

/// Borrowed row visitation; `Break` stops the walk.
pub(crate) type RowVisitor<'a> = &'a mut (dyn FnMut(&[Value]) -> Result<ControlFlow<()>> + 'a);

/// A borrowed row with its rank: its position in the state's logical row
/// order. Ranks are unique within a relation, may be sparse, and agree
/// between full and indexed visits; encounter order need not be rank order.
pub(crate) type RankedRowVisitor<'a> =
    &'a mut (dyn FnMut(u64, &[Value]) -> Result<ControlFlow<()>> + 'a);

/// A proposed final state: every distinct row of each ordinary relation,
/// decoded in sealed field order. Closed relations are read from the schema.
/// Each visit of a relation yields the same rows in the same order.
pub(crate) trait Facts {
    fn visit_rows(&self, relation: RelationId, visit: RowVisitor<'_>) -> Result<()>;

    /// Rows with their ranks; the default ranks the [`Facts::visit_rows`]
    /// stream.
    fn visit_ranked_rows(&self, relation: RelationId, visit: RankedRowVisitor<'_>) -> Result<()> {
        let mut rank = 0u64;
        self.visit_rows(relation, &mut |row| {
            let flow = visit(rank, row)?;
            rank += 1;
            Ok(flow)
        })
    }
}

/// An indexed group visit: the state keeps no index for the group, or it
/// walked the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Indexed {
    Unindexed,
    Walked,
}

/// A [`Facts`] state that knows its delta against a lawful parent and can
/// visit determinant groups through indexes.
pub(crate) trait DeltaFacts: Facts {
    /// The candidate preserves a scalar key the parent satisfies, for every
    /// mutation in it. `false` only asks for judgment.
    fn scalar_key_preserved(&self, _statement: StatementId) -> bool {
        false
    }

    /// The delta's shape over `relation`. Over-reporting a touch is sound;
    /// under-reporting one is not.
    fn delta_shape(&self, relation: RelationId) -> DeltaShape;

    /// The delta's added rows of `relation` (parent rows may repeat).
    fn visit_added_rows(&self, relation: RelationId, visit: RowVisitor<'_>) -> Result<()>;

    /// The parent rows the delta removes from `relation`.
    fn visit_removed_rows(&self, relation: RelationId, visit: RowVisitor<'_>) -> Result<()>;

    /// Every final row of one key statement's determinant group;
    /// `determinant` is the key's scalar values in projection order.
    fn visit_key_competitors(
        &self,
        statement: StatementId,
        determinant: &[Value],
        visit: RowVisitor<'_>,
    ) -> Result<Indexed>;

    /// Members of one compiled projection group; `determinant` is in
    /// [`CompiledTheory::index_key`] order.
    fn visit_compiled_group(
        &self,
        _projection: &CompiledProjection,
        _determinant: &[Value],
        _visit: RowVisitor<'_>,
    ) -> Result<Indexed> {
        Ok(Indexed::Unindexed)
    }

    /// [`DeltaFacts::visit_compiled_group`] with the relation-wide ranks of
    /// [`Facts::visit_ranked_rows`].
    fn visit_ranked_compiled_group(
        &self,
        _projection: &CompiledProjection,
        _determinant: &[Value],
        _visit: RankedRowVisitor<'_>,
    ) -> Result<Indexed> {
        Ok(Indexed::Unindexed)
    }
}

/// An owned map state: the reference [`Facts`] for tests and models.
#[derive(Debug, Default)]
pub(crate) struct MapState {
    relations: std::collections::BTreeMap<RelationId, Vec<Box<[Value]>>>,
}

impl MapState {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Inserts one proposed final row; exact duplicates collapse.
    #[cfg(test)]
    pub(crate) fn insert(&mut self, relation: RelationId, values: Vec<Value>) {
        let rows = self.relations.entry(relation).or_default();
        if !rows.iter().any(|row| row.as_ref() == values.as_slice()) {
            rows.push(values.into_boxed_slice());
        }
    }
}

impl Facts for MapState {
    fn visit_rows(&self, relation: RelationId, visit: RowVisitor<'_>) -> Result<()> {
        for row in self.relations.get(&relation).into_iter().flatten() {
            if visit(row)?.is_break() {
                break;
            }
        }
        Ok(())
    }
}

/// One cited example fact: a relation and its decoded sealed-order values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateFact {
    pub(crate) relation: RelationId,
    pub(crate) values: Box<[Value]>,
}

/// The side of a containment that failed: the judge evaluates each one-way
/// statement, so it is always the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JudgedDirection {
    SourceUnsatisfied,
}

/// One violated statement with its bounded examples; `examples_truncated`
/// says more offending facts exist beyond the budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JudgedViolation {
    pub(crate) statement: StatementId,
    pub(crate) kind: StatementKind,
    pub(crate) direction: Option<JudgedDirection>,
    /// A capacity violation's exact widened measure: that of the last
    /// violating group in logical target order.
    pub(crate) measure: Option<u128>,
    pub(crate) examples: Box<[CandidateFact]>,
    pub(crate) examples_truncated: bool,
}

/// Admitted, or every violated statement in statement order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Judgment {
    Admitted,
    Rejected(Box<[JudgedViolation]>),
}

/// The number of examples cited per violated statement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct JudgeBudget {
    pub(crate) examples_per_statement: usize,
}

impl Default for JudgeBudget {
    fn default() -> Self {
        Self {
            examples_per_statement: 4,
        }
    }
}

/// Judges the whole proposed final state against every statement.
///
/// # Errors
/// Cancellation, the state's own failure, a ray in a duration-measured
/// position, or a measure past the widened accumulator. No partial
/// rejection is returned.
pub(crate) fn judge_complete<S: Facts>(
    schema: &Schema,
    state: &S,
    work: &WorkContext,
    budget: JudgeBudget,
) -> Result<Judgment> {
    let mut judge = Judge::new(schema, work, budget);
    for view in schema.statements() {
        work.checkpoint()?;
        if schema.closed_constant(view) {
            continue;
        }
        match view {
            StatementView::Key(_, statement) => judge.key(state, statement)?,
            StatementView::Containment(_, statement) => judge.containment(state, statement)?,
            StatementView::Capacity(_, statement) => judge.capacity(state, statement)?,
        }
    }
    Ok(judge.judgment())
}

/// Judges the statements the delta can affect, under a lawful parent. It
/// equals [`judge_complete`] exactly when the parent satisfies every
/// statement; an empty delta admits without re-judging standing facts.
///
/// # Errors
/// As [`judge_complete`], and schema compilation.
pub(crate) fn judge_incremental<S: DeltaFacts>(
    schema: &Schema,
    state: &S,
    work: &WorkContext,
    budget: JudgeBudget,
) -> Result<Judgment> {
    let theory = schema.compiled_theory()?;
    let mut judge = Judge::new(schema, work, budget);
    for view in schema.complete_obligations() {
        if !delta_can_violate(view, |relation| state.delta_shape(relation)) {
            continue;
        }
        work.checkpoint()?;
        match view {
            StatementView::Key(_, statement) => {
                if judge.key_delta_local(state, statement)? == Indexed::Unindexed {
                    judge.key(state, statement)?;
                }
            }
            StatementView::Containment(_, statement) => {
                judge.containment_delta_local(state, theory, statement)?;
            }
            StatementView::Capacity(_, statement) => {
                judge.capacity_delta_local(state, theory, statement)?;
            }
        }
    }
    Ok(judge.judgment())
}

/// Whether a delta of this shape can violate a statement its lawful parent
/// satisfies: a key only through additions, a containment through source
/// additions or target removals, a capacity through any source change or a
/// target addition.
fn delta_can_violate(view: StatementView<'_>, shape: impl Fn(RelationId) -> DeltaShape) -> bool {
    match view {
        StatementView::Key(_, statement) => shape(statement.relation).adds,
        StatementView::Containment(_, statement) => {
            shape(statement.source.relation).adds || shape(statement.target.relation).removes
        }
        StatementView::Capacity(_, statement) => {
            shape(statement.source.relation).touched() || shape(statement.target.relation).adds
        }
    }
}

struct Judge<'s, 'w> {
    schema: &'s Schema,
    work: &'w WorkContext,
    budget: JudgeBudget,
    violations: Vec<JudgedViolation>,
}

impl<'s, 'w> Judge<'s, 'w> {
    const fn new(schema: &'s Schema, work: &'w WorkContext, budget: JudgeBudget) -> Self {
        Self {
            schema,
            work,
            budget,
            violations: Vec::new(),
        }
    }

    fn judgment(self) -> Judgment {
        let mut violations = self.violations;
        violations.sort_by_key(|violation| violation.statement);
        if violations.is_empty() {
            Judgment::Admitted
        } else {
            Judgment::Rejected(violations.into_boxed_slice())
        }
    }

    fn pending(&self, statement: StatementId, kind: StatementKind) -> PendingViolation {
        PendingViolation::new(statement, kind, self.budget.examples_per_statement)
    }

    /// Ranked proposed rows: a closed relation's sealed rows, an ordinary
    /// relation's state rows. One cancellation poll per row.
    fn for_each_row<S: Facts>(
        &mut self,
        state: &S,
        relation: RelationId,
        mut visit: impl FnMut(&mut Self, u64, &[Value]) -> Result<ControlFlow<()>>,
    ) -> Result<()> {
        if let Some(rows) = self.schema.closed_rows(relation) {
            for (rank, row) in (0u64..).zip(rows) {
                self.work.checkpoint()?;
                if visit(self, rank, &row.values)?.is_break() {
                    break;
                }
            }
            return Ok(());
        }
        state.visit_ranked_rows(relation, &mut |rank, row| {
            self.work.checkpoint()?;
            visit(self, rank, row)
        })
    }

    /// One compiled projection group, addressed through
    /// [`CompiledTheory::index_key`].
    fn visit_indexed_group<S: DeltaFacts>(
        &mut self,
        state: &S,
        compiled: &CompiledProjection,
        binding: &ProjectionBinding,
        logical: &[Value],
        mut visit: impl FnMut(&mut Self, &[Value]) -> Result<ControlFlow<()>>,
    ) -> Result<Indexed> {
        let Some(physical) = CompiledTheory::index_key(binding, logical) else {
            return Ok(Indexed::Unindexed);
        };
        state.visit_compiled_group(compiled, &physical, &mut |row| {
            self.work.checkpoint()?;
            visit(self, row)
        })
    }

    /// [`Judge::visit_indexed_group`] with relation-wide ranks.
    fn visit_ranked_indexed_group<S: DeltaFacts>(
        &mut self,
        state: &S,
        compiled: &CompiledProjection,
        binding: &ProjectionBinding,
        logical: &[Value],
        mut visit: impl FnMut(&mut Self, u64, &[Value]) -> Result<ControlFlow<()>>,
    ) -> Result<Indexed> {
        let Some(physical) = CompiledTheory::index_key(binding, logical) else {
            return Ok(Indexed::Unindexed);
        };
        state.visit_ranked_compiled_group(compiled, &physical, &mut |rank, row| {
            self.work.checkpoint()?;
            visit(self, rank, row)
        })
    }

    /// Whether the state indexes both sides of a statement, probed with one
    /// sample determinant.
    fn compiled_indexes_live<S: DeltaFacts>(
        state: &S,
        theory: &CompiledTheory,
        source_binding: &ProjectionBinding,
        target_binding: &ProjectionBinding,
        sample: &[Value],
    ) -> Result<Indexed> {
        for binding in [source_binding, target_binding] {
            let Some(id) = binding.projection else {
                continue;
            };
            let Some(compiled) = theory.projection(id) else {
                return Ok(Indexed::Unindexed);
            };
            let Some(physical) = CompiledTheory::index_key(binding, sample) else {
                return Ok(Indexed::Unindexed);
            };
            if state.visit_compiled_group(compiled, &physical, &mut |_| Ok(Break(())))?
                == Indexed::Unindexed
            {
                return Ok(Indexed::Unindexed);
            }
        }
        Ok(Indexed::Walked)
    }

    fn offer(
        &mut self,
        pending: &mut PendingViolation,
        relation: RelationId,
        values: &[Value],
    ) -> Result<()> {
        pending
            .citations
            .offer(self.schema, self.work, relation, values)
    }

    fn finish(&mut self, pending: PendingViolation) {
        if pending.violated {
            let (examples, truncated) = pending.citations.into_examples();
            self.violations.push(JudgedViolation {
                statement: pending.statement,
                kind: pending.kind,
                direction: pending.direction,
                measure: pending.measure,
                examples,
                examples_truncated: truncated,
            });
        }
    }
}

struct PendingViolation {
    statement: StatementId,
    kind: StatementKind,
    direction: Option<JudgedDirection>,
    measure: Option<u128>,
    measure_rank: Option<u64>,
    citations: CitationTopK,
    violated: bool,
}

impl PendingViolation {
    fn new(statement: StatementId, kind: StatementKind, budget: usize) -> Self {
        Self {
            statement,
            kind,
            direction: None,
            measure: None,
            measure_rank: None,
            citations: CitationTopK::new(budget),
            violated: false,
        }
    }

    fn with_direction(mut self, direction: JudgedDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    fn record_measure_at(&mut self, rank: u64, measure: u128) {
        if self.measure_rank.is_none_or(|previous| rank > previous) {
            self.measure_rank = Some(rank);
            self.measure = Some(measure);
        }
    }
}

/// Whether the row satisfies the side's selection: each selected field's
/// value is one of its literals.
fn satisfies(side: &Side, row: &[Value]) -> bool {
    side.selection.iter().all(|(field, literals)| {
        let actual = &row[usize::from(field.0)];
        literals.literals().iter().any(|literal| literal == actual)
    })
}

/// Appends the side's projected values as exact injective bytes, optionally
/// skipping one projection position (the pointwise interval).
fn encode_projection(side: &Side, row: &[Value], skip: Option<usize>, out: &mut Vec<u8>) {
    for (index, field) in side.projection.iter().enumerate() {
        if Some(index) != skip {
            append_value(out, &row[usize::from(field.0)]);
        }
    }
}

fn encode_values(values: &[Value], out: &mut Vec<u8>) {
    for value in values {
        append_value(out, value);
    }
}

/// Marks every determinant group that the delta's rows of one side, of the
/// selected kinds, can affect.
fn mark_delta_groups<S: DeltaFacts>(
    state: &S,
    side: &Side,
    binding: &ProjectionBinding,
    kinds: DeltaShape,
    affected: &mut GroupedMap,
) -> Result<()> {
    let mut key = ScalarKeyScratch::new(0);
    let mut mark = |row: &[Value]| {
        if satisfies(side, row) {
            affected.insert_if_absent(key.encode_projection(row, &binding.logical_scalars));
        }
        Ok(Continue(()))
    };
    if kinds.adds {
        state.visit_added_rows(side.relation, &mut mark)?;
    }
    if kinds.removes {
        state.visit_removed_rows(side.relation, &mut mark)?;
    }
    Ok(())
}

/// A span map key: `(group token, start, end, seq)` as big-endian words,
/// so byte order is sweep order.
fn span_key(token: u64, start: u64, end: u64, seq: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[..8].copy_from_slice(&token.to_be_bytes());
    key[8..16].copy_from_slice(&start.to_be_bytes());
    key[16..24].copy_from_slice(&end.to_be_bytes());
    key[24..].copy_from_slice(&seq.to_be_bytes());
    key
}

fn parse_span_key(key: &[u8]) -> (u64, u64, u64, u64) {
    let word = |at: usize| u64::from_be_bytes(key[at..at + 8].try_into().expect("span key width"));
    (word(0), word(8), word(16), word(24))
}

/// An interval's endpoints as order-preserving words; `None` for a scalar.
pub(crate) fn interval_order_words(value: &Value) -> Option<(u64, u64)> {
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod delta_tests;

#[cfg(test)]
mod grouped_tests;

#[cfg(test)]
mod discriminators;
