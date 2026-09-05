//! Complete and incremental judgment are different proofs (C4).
//!
//! [`judge_complete`] evaluates one **proposed final state** against every
//! sealed statement. Staging, restore, migrate and the offline verifier use
//! this entry. An empty delta is not a shortcut: the populated final state
//! is judged in full.
//!
//! [`judge_incremental`] requires a [`LawfulParent`] capability minted only
//! by checked create/open or a previous admitted commit. An unready store
//! cannot supply that premise. Incremental judgment enumerates affected
//! groups from the net delta and compiled adjacency, then visits those
//! groups through interned descriptors (`visit_compiled_group`) when the
//! state exposes them; it never treats an empty change set as complete
//! validation. Group identity is [`CompiledTheory::group_key`] (statement
//! order). [`CompiledTheory::index_key`] is used only at
//! `visit_compiled_group`.
//!
//! [`judge_final_state`] is the independent streaming reference. It shares
//! denotation with the production entries and does not share their
//! optimized access planning. Exact value equality decides identity — a
//! forced fingerprint collision cannot change a verdict here.
//!
//! Grouped state lives in charged scratch ([`grouped`]). Citations are
//! selected by canonical fact bytes **before** the labeled top-k budget
//! truncates (C4 / CORE-021). A completed rejection names every violated
//! statement; resource failure is not a rejection.
use crate::schema::compiled::{CompiledProjection, CompiledTheory, ProjectionBinding};
use crate::schema::{
    CapacityStatement, CompileError, ContainmentStatement, KeyStatement, RelationId, Schema,
    SealedBound, SealedWeight, Side, StatementId, StatementKind, StatementView,
};
use crate::work::ByteReservation;
use crate::{Value, WorkContext, WorkError};

mod citation;
mod grouped;

pub use grouped::{JudgeScratch, ScratchFault, store_fault};

use citation::CitationTopK;
use grouped::{FLAG_OVERFLOW, FLAG_RAY, GroupedMap, encode_value};

/// One proposed final-state view: for every ORDINARY relation, the judge
/// can visit each distinct proposed row (committed minus removed plus
/// added), decoded to sealed-field-order values. Closed relations are not
/// consulted — their ground axioms are sealed in the schema. The state
/// must present set semantics (no duplicate full rows); the change-set
/// normalization already guarantees that for the delta side.
///
/// `visit_rows` may be called several times per relation within one
/// judgment. Every call must yield the same rows in the same deterministic
/// order. Charged decoded rows are borrowed for the visit; the implementor
/// does not extract an owning `Box<[Value]>`.
pub trait CandidateFacts {
    /// The state's own iteration/decoding failure (I/O, corruption…),
    /// distinct from every semantic outcome.
    type Error;

    /// Visit `relation`'s proposed final rows. Return `false` from `visit`
    /// to stop early.
    ///
    /// # Errors
    /// The state's own failure channel.
    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error>;
}

/// The candidate delta's net shape over one relation — the affected-relation
/// information incremental judgment's relevance filters read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeltaShape {
    /// The delta adds at least one row to the relation.
    pub adds: bool,
    /// The delta removes at least one row from the relation.
    pub removes: bool,
}

impl DeltaShape {
    /// The delta touches the relation at all.
    #[must_use]
    pub const fn touched(self) -> bool {
        self.adds || self.removes
    }
}

/// A [`CandidateFacts`] state that additionally knows its own delta against
/// the committed parent and can enumerate one sealed key statement's
/// determinant group from an index instead of a relation stream.
pub trait DeltaFacts: CandidateFacts {
    /// The delta's net shape over `relation` (`DeltaShape::default()` for an
    /// untouched relation). Over-reporting a touch is sound (more judged);
    /// under-reporting is NOT — a touched relation reported untouched breaks
    /// the delta-locality argument.
    fn delta_shape(&self, relation: RelationId) -> DeltaShape;

    /// Visit the delta's ADDED rows of one relation. Rows already present
    /// in the parent may appear; that only widens the judged group set.
    ///
    /// # Errors
    /// The state's own failure channel.
    fn visit_added_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error>;

    /// Visit the delta's REMOVED rows of one relation (parent rows the
    /// delta deletes). Used to enumerate groups a removal can affect.
    ///
    /// # Errors
    /// The state's own failure channel.
    fn visit_removed_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error>;

    /// Bounded visitor over every row of one sealed key statement's
    /// determinant group in the proposed final state. `determinant` is the
    /// statement's scalar values in projection order. Return `false` from
    /// the visitor to stop early. `None` means the state maintains no group
    /// index for `statement`; the judge then falls back to the complete
    /// streaming path for that key.
    ///
    /// # Errors
    /// The state's own failure channel.
    fn visit_key_competitors(
        &self,
        statement: StatementId,
        determinant: &[Value],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<Option<()>, Self::Error>;

    /// Visit members of one compiled projection group. `determinant` is
    /// intern-order scalars from [`CompiledTheory::index_key`]. `None`
    /// means the state has no index.
    ///
    /// # Errors
    /// The state's own failure channel.
    fn visit_compiled_group(
        &self,
        _projection: &CompiledProjection,
        _determinant: &[Value],
        _visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<Option<()>, Self::Error> {
        Ok(None)
    }
}

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

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        if let Some(rows) = self.relations.get(&relation) {
            for row in rows {
                if !visit(row.as_ref())? {
                    break;
                }
            }
        }
        Ok(())
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
/// Examples are the bounded canonical top-k by portable fact bytes.
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
/// allowance; `State` is the candidate view's own failure channel (a
/// spilled grouped-state fault also travels through it, via the state's
/// [`JudgeScratch`] conversion); `UndefinedDuration` is the explicit
/// refusal of a ray in a duration-measured position; `MeasureOverflow`
/// reports a group total past the widened accumulator instead of wrapping
/// a witness; `Compile` is interned-projection exhaustion.
#[derive(Debug, PartialEq, Eq)]
pub enum JudgeError<E> {
    Work(WorkError),
    State(E),
    UndefinedDuration { statement: StatementId },
    MeasureOverflow { statement: StatementId },
    /// Compiled projection ids were exhausted. Not a semantic verdict.
    Compile(CompileError),
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

/// Capability that the committed parent is lawful (C4). Only complete
/// admission, trusted persisted open, or a prior admitted commit may mint it.
/// An [`UnreadyStore`](crate::storage::store::UnreadyStore) cannot. This
/// type has no public constructor; [`LawfulParent::established`] is
/// crate-private so an unready owner cannot forge the premise.
///
/// ```compile_fail
/// let _ = bumbledb::schema::LawfulParent {};
/// ```
/// ```compile_fail
/// let _ = bumbledb::schema::LawfulParent::established();
/// ```
/// ```compile_fail
/// fn unready_cannot_mint(unready: &bumbledb::store::UnreadyStore) {
///     let _ = unready.lawful_parent();
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LawfulParent {
    _priv: (),
}

impl LawfulParent {
    /// Established by complete final-state admission or trusted open.
    #[must_use]
    pub(crate) const fn established() -> Self {
        Self { _priv: () }
    }
}

/// Complete final-state judgment (C4). Staging, restore, migrate and the
/// offline verifier use this entry — never an empty-delta incremental skip.
///
/// # Errors
/// As [`judge_final_state`].
pub fn judge_complete<S: CandidateFacts>(
    schema: &Schema,
    state: &S,
    work: &WorkContext,
    budget: JudgeBudget,
) -> Result<Judgment, JudgeError<S::Error>> {
    judge_final_state(schema, state, work, budget)
}

/// Incremental judgment (C4). Requires a lawful-parent capability; an
/// unready populated store cannot supply that premise. An empty delta is
/// not complete validation: under the premise it admits without re-judging
/// standing facts.
///
/// # Errors
/// As [`judge_final_state`].
pub fn judge_incremental<S: DeltaFacts>(
    _parent: LawfulParent,
    schema: &Schema,
    state: &S,
    work: &WorkContext,
    budget: JudgeBudget,
    scratch: JudgeScratch<S::Error>,
) -> Result<Judgment, JudgeError<S::Error>> {
    judge_final_state_delta_local(schema, state, work, budget, scratch)
}

/// Judges the complete proposed final state against every sealed
/// statement. See the module doc for the exact semantics; the verdict for
/// a completed run always names EVERY violated statement.
///
/// States whose error type is the store's own (`storage::store::StoreError`
/// — the production candidate view and the offline sweeper) must pass
/// [`JudgeScratch::channel`] via [`judge_final_state_with_scratch`]; the
/// parameterless entry keeps grouped state in charged RAM only.
///
/// # Errors
/// [`JudgeError`] on exhausted work, a failing candidate view or its
/// spilled grouped state, a ray in a duration-measured position, or an
/// unwitnessable measure. No partial rejection is returned on any error
/// path.
pub fn judge_final_state<S: CandidateFacts>(
    schema: &Schema,
    state: &S,
    work: &WorkContext,
    budget: JudgeBudget,
) -> Result<Judgment, JudgeError<S::Error>> {
    judge_final_state_with_scratch(schema, state, work, budget, JudgeScratch::disabled())
}

/// [`judge_final_state`] with an explicit grouped-state spill policy: the
/// seam for states that want beyond-memory judgment under their own error
/// channel (e.g. the log's migration states).
///
/// # Errors
/// As [`judge_final_state`].
pub fn judge_final_state_with_scratch<S: CandidateFacts>(
    schema: &Schema,
    state: &S,
    work: &WorkContext,
    budget: JudgeBudget,
    scratch: JudgeScratch<S::Error>,
) -> Result<Judgment, JudgeError<S::Error>> {
    let mut judge = Judge {
        schema,
        work,
        budget,
        channel: scratch.channel,
        violations: Vec::new(),
        citation_charges: Vec::new(),
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

/// Incremental production judgment under a lawful parent. Callers must
/// hold [`LawfulParent`]; this function does not mint that premise and
/// does not treat an empty delta as complete validation.
///
/// Equivalence with [`judge_final_state`] holds exactly when the committed
/// parent satisfies every sealed statement. A parent made unlawful outside
/// the admission path can hide a standing violation whenever the delta
/// does not touch it — that is why the verifier re-runs complete judgment.
///
/// # Errors
/// As [`judge_final_state`].
pub fn judge_final_state_delta_local<S: DeltaFacts>(
    schema: &Schema,
    state: &S,
    work: &WorkContext,
    budget: JudgeBudget,
    scratch: JudgeScratch<S::Error>,
) -> Result<Judgment, JudgeError<S::Error>> {
    let theory = schema.compiled_theory().map_err(JudgeError::Compile)?;
    let mut judge = Judge {
        schema,
        work,
        budget,
        channel: scratch.channel,
        violations: Vec::new(),
        citation_charges: Vec::new(),
    };
    let mut delta = Vec::new();
    for (idx, _relation) in schema.relations().iter().enumerate() {
        let relation = RelationId(u32::try_from(idx).expect("relation count fits u32"));
        let shape = state.delta_shape(relation);
        if shape.touched() {
            delta.push((relation, shape));
        }
    }
    delta.sort_by_key(|&(id, _)| id);
    for view in theory.delta_local_statements(schema, &delta) {
        work.step(1)?;
        match view {
            StatementView::Key(_, statement) => {
                if !judge.key_delta_local(state, statement)? {
                    judge.key(state, statement)?;
                }
            }
            StatementView::Containment(_, statement) => {
                judge.containment_delta_local(state, statement)?;
            }
            StatementView::Capacity(_, statement) => {
                judge.capacity_delta_local(state, statement)?;
            }
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

struct Judge<'s, 'w, E> {
    schema: &'s Schema,
    work: &'w WorkContext,
    budget: JudgeBudget,
    channel: Option<fn(ScratchFault) -> E>,
    violations: Vec<JudgedViolation>,
    /// Working-byte reservations covering the cited example facts held by
    /// this judgment; released when the verdict's ownership passes to the
    /// caller at return.
    citation_charges: Vec<ByteReservation>,
}

impl<E> Judge<'_, '_, E> {
    fn grouped(&self) -> GroupedMap<E> {
        GroupedMap::new(self.work, self.channel)
    }

    fn pending(&self, statement: StatementId, kind: StatementKind) -> PendingViolation {
        PendingViolation::new(statement, kind, self.budget.examples_per_statement)
    }

    /// Stream one relation's proposed final rows in the state's
    /// deterministic order — from the sealed extension for closed
    /// relations, from the state otherwise — charging one work step per
    /// row. The visitor returns `false` to stop the walk. Charged decoded
    /// rows stay borrowed for the visit.
    fn for_each_row<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        relation: RelationId,
        mut visit: impl FnMut(&mut Self, u64, &[Value]) -> Result<bool, JudgeError<E>>,
    ) -> Result<(), JudgeError<E>> {
        let sealed = self.schema.relation(relation);
        if let Some(extension) = sealed.body().closed_rows() {
            for (seq, row) in extension.iter().enumerate() {
                self.work.step(1)?;
                let decoded =
                    crate::encoding::decode_values(sealed.layout().encoded(&row.fact), |_| {
                        unreachable!("closed relations refuse str columns")
                    })
                    .expect("sealed extension rows decode by construction");
                if !visit(self, seq as u64, &decoded)? {
                    return Ok(());
                }
            }
            return Ok(());
        }
        let mut seq = 0u64;
        let mut smuggled = None;
        let judge = self as *mut Self;
        let walked = state.visit_rows(relation, &mut |row| {
            // Safety: `visit_rows` only borrows `state` and invokes this
            // callback; it does not alias `Judge`.
            let judge = unsafe { &mut *judge };
            if let Err(error) = judge.work.step(1) {
                smuggled = Some(JudgeError::Work(error));
                return Ok(false);
            }
            match visit(judge, seq, row) {
                Ok(keep) => {
                    seq += 1;
                    Ok(keep)
                }
                Err(error) => {
                    smuggled = Some(error);
                    Ok(false)
                }
            }
        });
        if let Some(error) = smuggled {
            return Err(error);
        }
        walked.map_err(JudgeError::State)
    }

    /// Walk one compiled projection group. `None` means the state exposes
    /// no index for `compiled`.
    fn for_each_compiled_group<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        compiled: &CompiledProjection,
        determinant: &[Value],
        mut visit: impl FnMut(&mut Self, &[Value]) -> Result<bool, JudgeError<E>>,
    ) -> Result<Option<()>, JudgeError<E>> {
        let mut smuggled = None;
        let judge = self as *mut Self;
        let indexed = state.visit_compiled_group(compiled, determinant, &mut |row| {
            // Safety: `visit_compiled_group` only borrows `state` and
            // invokes this callback; it does not alias `Judge`.
            let judge = unsafe { &mut *judge };
            if let Err(error) = judge.work.step(1) {
                smuggled = Some(JudgeError::Work(error));
                return Ok(false);
            }
            match visit(judge, row) {
                Ok(keep) => Ok(keep),
                Err(error) => {
                    smuggled = Some(error);
                    Ok(false)
                }
            }
        });
        if let Some(error) = smuggled {
            return Err(error);
        }
        match indexed {
            Ok(some) => Ok(some),
            Err(error) => Err(JudgeError::State(error)),
        }
    }

    /// Index-boundary walk: [`CompiledTheory::index_key`] then
    /// [`DeltaFacts::visit_compiled_group`].
    fn visit_indexed_group<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        compiled: &CompiledProjection,
        binding: &ProjectionBinding,
        logical: &[Value],
        visit: impl FnMut(&mut Self, &[Value]) -> Result<bool, JudgeError<E>>,
    ) -> Result<Option<()>, JudgeError<E>> {
        let Some(physical) = CompiledTheory::index_key(binding, logical) else {
            return Ok(None);
        };
        self.for_each_compiled_group(state, compiled, &physical, visit)
    }

    fn offer(
        &mut self,
        pending: &mut PendingViolation,
        relation: RelationId,
        values: &[Value],
    ) -> Result<(), JudgeError<E>> {
        pending
            .citations
            .offer(self.schema, self.work, relation, values)
    }

    /// Delta-local key judgment over the state's group index: enumerate the
    /// full FINAL membership of every determinant group a delta-added row
    /// touches, and judge only those. Returns `false` when the state
    /// exposes no group index; the caller then runs the complete streaming
    /// pass. Competitors stay in ordered scratch; citations are selected
    /// by canonical bytes before the budget truncates.
    fn key_delta_local<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &KeyStatement,
    ) -> Result<bool, JudgeError<E>> {
        let relation = statement.relation;
        let fields = self.schema.relation(relation).fields();
        let mut scalar_fields = Vec::new();
        let mut interval_field = None;
        for field in &statement.projection {
            let idx = usize::from(field.0);
            if fields[idx].value_type.is_interval() {
                interval_field = Some(idx);
            } else {
                scalar_fields.push(idx);
            }
        }
        let mut seen = self.grouped();
        let mut groups = Vec::new();
        let mut det = Vec::new();
        let mut walk_error = None;
        state
            .visit_added_rows(relation, &mut |row| {
                if let Err(error) = self.work.step(1) {
                    walk_error = Some(JudgeError::Work(error));
                    return Ok(false);
                }
                det.clear();
                for &idx in &scalar_fields {
                    encode_value(&row[idx], &mut det);
                }
                match seen.insert_if_absent(&det) {
                    Ok(true) => {
                        groups.push(scalar_fields.iter().map(|&idx| row[idx].clone()).collect());
                        Ok(true)
                    }
                    Ok(false) => Ok(true),
                    Err(error) => {
                        walk_error = Some(error);
                        Ok(false)
                    }
                }
            })
            .map_err(JudgeError::State)?;
        if let Some(error) = walk_error {
            return Err(error);
        }
        let mut pending = self.pending(statement.id, StatementKind::Functionality);
        for determinant in &groups {
            let indexed = match interval_field {
                None => self.key_group_scalar(state, statement, relation, determinant, &mut pending)?,
                Some(tail) => self.key_group_pointwise(
                    state,
                    statement,
                    relation,
                    determinant,
                    tail,
                    &mut pending,
                )?,
            };
            if !indexed {
                return Ok(false);
            }
        }
        self.finish(pending);
        Ok(true)
    }

    fn key_group_scalar<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &KeyStatement,
        relation: RelationId,
        determinant: &[Value],
        pending: &mut PendingViolation,
    ) -> Result<bool, JudgeError<E>> {
        let mut count = 0u64;
        match state.visit_key_competitors(statement.id, determinant, &mut |_| {
            count += 1;
            Ok(true)
        }) {
            Ok(Some(())) => {}
            Ok(None) => return Ok(false),
            Err(error) => return Err(JudgeError::State(error)),
        }
        if count < 2 {
            return Ok(true);
        }
        pending.violated = true;
        self.offer_key_group(state, statement.id, relation, determinant, pending, None)
    }

    fn key_group_pointwise<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &KeyStatement,
        relation: RelationId,
        determinant: &[Value],
        tail: usize,
        pending: &mut PendingViolation,
    ) -> Result<bool, JudgeError<E>> {
        let mut spans = self.grouped();
        let mut seq = 0u64;
        let mut walk_error = None;
        match state.visit_key_competitors(statement.id, determinant, &mut |values| {
            let (start, end) = interval_order_words(&values[tail])
                .expect("a projected interval position holds an interval value");
            let key = span_key(0, start, end, seq);
            seq += 1;
            if let Err(error) = self.work.step(1) {
                walk_error = Some(JudgeError::Work(error));
                return Ok(false);
            }
            if let Err(error) = spans.put(&key, &[]) {
                walk_error = Some(error);
                return Ok(false);
            }
            Ok(true)
        }) {
            Ok(Some(())) => {}
            Ok(None) => return Ok(false),
            Err(error) => return Err(JudgeError::State(error)),
        }
        if let Some(error) = walk_error {
            return Err(error);
        }
        let mut offending = self.grouped();
        let mut violated = false;
        let mut previous: Option<u64> = None;
        let mut prev_seq = 0u64;
        spans.for_each(|key, _| {
            self.work.step(1)?;
            let (_, start, end, at) = parse_span_key(key);
            if let Some(prev_end) = previous
                && start < prev_end
            {
                violated = true;
                offending.put(&prev_seq.to_be_bytes(), &[])?;
                offending.put(&at.to_be_bytes(), &[])?;
            }
            previous = Some(end);
            prev_seq = at;
            Ok(true)
        })?;
        if !violated {
            return Ok(true);
        }
        pending.violated = true;
        self.offer_key_group(
            state,
            statement.id,
            relation,
            determinant,
            pending,
            Some(&mut offending),
        )
    }

    fn offer_key_group<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: StatementId,
        relation: RelationId,
        determinant: &[Value],
        pending: &mut PendingViolation,
        mut only: Option<&mut GroupedMap<E>>,
    ) -> Result<bool, JudgeError<E>> {
        let mut seq = 0u64;
        let mut walk_error = None;
        match state.visit_key_competitors(statement, determinant, &mut |values| {
            let at = seq;
            seq += 1;
            let take = match &mut only {
                None => true,
                Some(flagged) => match flagged.contains(&at.to_be_bytes()) {
                    Ok(hit) => hit,
                    Err(error) => {
                        walk_error = Some(error);
                        return Ok(false);
                    }
                },
            };
            if take {
                let judge = self as *mut Self;
                let judge = unsafe { &mut *judge };
                if let Err(error) = judge.offer(pending, relation, values) {
                    walk_error = Some(error);
                    return Ok(false);
                }
            }
            Ok(true)
        }) {
            Ok(Some(())) => {}
            Ok(None) => return Ok(false),
            Err(error) => return Err(JudgeError::State(error)),
        }
        if let Some(error) = walk_error {
            return Err(error);
        }
        Ok(true)
    }

    fn key<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &KeyStatement,
    ) -> Result<(), JudgeError<E>> {
        let relation = statement.relation;
        // Split the projection into the scalar determinant and an optional
        // trailing interval position (the pointwise form).
        let fields = self.schema.relation(relation).fields();
        let mut scalar_fields = Vec::new();
        let mut interval_field = None;
        for field in &statement.projection {
            let idx = usize::from(field.0);
            if fields[idx].value_type.is_interval() {
                interval_field = Some(idx);
            } else {
                scalar_fields.push(idx);
            }
        }
        let mut pending = self.pending(statement.id, StatementKind::Functionality);
        match interval_field {
            None => self.key_scalar(state, relation, &scalar_fields, &mut pending)?,
            Some(tail) => {
                self.key_pointwise(state, relation, &scalar_fields, tail, &mut pending)?;
            }
        }
        self.finish(pending);
        Ok(())
    }

    /// Scalar key: a determinant seen twice is a violation. Membership is
    /// exact encoded determinant bytes in the charged map — every competing
    /// row lands in its group before any uniqueness is enforced.
    fn key_scalar<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        relation: RelationId,
        scalar_fields: &[usize],
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut seen = self.grouped();
        let mut offending = self.grouped();
        let mut det = Vec::new();
        self.for_each_row(state, relation, |_judge, _seq, row| {
            det.clear();
            for &idx in scalar_fields {
                encode_value(&row[idx], &mut det);
            }
            if !seen.insert_if_absent(&det)? {
                offending.put(&det, &[])?;
            }
            Ok(true)
        })?;
        if offending.len() == 0 {
            return Ok(());
        }
        pending.violated = true;
        self.for_each_row(state, relation, |judge, _seq, row| {
            det.clear();
            for &idx in scalar_fields {
                encode_value(&row[idx], &mut det);
            }
            if offending.contains(&det)? {
                judge.offer(pending, relation, row)?;
            }
            Ok(true)
        })
    }

    /// Pointwise key: two rows with one determinant may coexist only with
    /// disjoint interval tails. Spans are staged in the charged map under
    /// fixed-width `(group token, start, end, seq)` keys, whose exact byte
    /// order IS the sweep order; adjacent overlap detects every violation.
    fn key_pointwise<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        relation: RelationId,
        scalar_fields: &[usize],
        tail: usize,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut tokens = self.grouped();
        let mut spans = self.grouped();
        let mut det = Vec::new();
        self.for_each_row(state, relation, |_judge, seq, row| {
            det.clear();
            for &idx in scalar_fields {
                encode_value(&row[idx], &mut det);
            }
            let token = tokens.token_of(&det)?;
            let (start, end) = interval_order_words(&row[tail])
                .expect("a projected interval position holds an interval value");
            let key = span_key(token, start, end, seq);
            spans.put(&key, &[])?;
            Ok(true)
        })?;
        // Sweep in exact (token, start, end, seq) order: sorted by start
        // within a group, any overlap is witnessed by an adjacent pair.
        let mut offending = self.grouped();
        let mut violated = false;
        let mut previous: Option<(u64, u64, u64)> = None; // token, end, seq
        spans.for_each(|key, _| {
            self.work.step(1)?;
            let (token, start, end, seq) = parse_span_key(key);
            if let Some((prev_token, prev_end, prev_seq)) = previous
                && prev_token == token
                && start < prev_end
            {
                violated = true;
                offending.put(&prev_seq.to_be_bytes(), &[])?;
                offending.put(&seq.to_be_bytes(), &[])?;
            }
            previous = Some((token, end, seq));
            Ok(true)
        })?;
        if !violated {
            return Ok(());
        }
        pending.violated = true;
        self.for_each_row(state, relation, |judge, seq, row| {
            if offending.contains(&seq.to_be_bytes())? {
                judge.offer(pending, relation, row)?;
            }
            Ok(true)
        })
    }

    fn containment<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
    ) -> Result<(), JudgeError<E>> {
        let target_fields = self.schema.relation(statement.target.relation).fields();
        // At most one trailing interval position (validation's rule); its
        // presence selects pointwise coverage instead of tuple existence.
        let coverage_position = statement
            .target
            .projection
            .iter()
            .position(|field| target_fields[usize::from(field.0)].value_type.is_interval());
        let mut pending = self
            .pending(statement.id, StatementKind::Containment)
            .with_direction(JudgedDirection::SourceUnsatisfied);
        match coverage_position {
            None => self.containment_scalar(state, statement, &mut pending)?,
            Some(position) => {
                self.containment_pointwise(state, statement, position, &mut pending)?;
            }
        }
        self.finish(pending);
        Ok(())
    }

    /// Tuple existence: stage every satisfying target projection in the
    /// charged set, then stream source rows and probe by exact bytes.
    fn containment_scalar<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut witnesses = self.grouped();
        let mut key = Vec::new();
        self.for_each_row(state, statement.target.relation, |_judge, _seq, row| {
            if satisfies(&statement.target, row) {
                key.clear();
                encode_projection(&statement.target, row, None, &mut key);
                witnesses.put(&key, &[])?;
            }
            Ok(true)
        })?;
        self.for_each_row(state, statement.source.relation, |judge, _seq, row| {
            if !satisfies(&statement.source, row) {
                return Ok(true);
            }
            key.clear();
            encode_projection(&statement.source, row, None, &mut key);
            if !witnesses.contains(&key)? {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(true)
        })
    }

    /// Pointwise coverage: scalar prefix equality plus coverage of the
    /// source span by the union of matching target spans. Target spans are
    /// merged into maximal coverage runs (adjacent spans connect, exactly
    /// like the reference frontier walk); each source span then needs one
    /// predecessor probe.
    fn containment_pointwise<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        position: usize,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut tokens = self.grouped();
        let mut spans = self.grouped();
        let mut prefix = Vec::new();
        self.for_each_row(state, statement.target.relation, |_judge, _seq, row| {
            if !satisfies(&statement.target, row) {
                return Ok(true);
            }
            prefix.clear();
            encode_projection(&statement.target, row, Some(position), &mut prefix);
            let token = tokens.token_of(&prefix)?;
            let span_value = &row[usize::from(statement.target.projection[position].0)];
            let (start, end) = interval_order_words(span_value)
                .expect("positional typing pairs interval positions");
            // Coverage is a set: identical spans collapse.
            spans.put(&span_key(token, start, end, 0), &[])?;
            Ok(true)
        })?;
        // Merge into maximal runs per token: `start ≤ current end` connects
        // (adjacency merges — the frontier walk's exact reachability).
        let mut runs = self.grouped();
        let mut current: Option<(u64, u64, u64)> = None; // token, run start, run end
        spans.for_each(|key, _| {
            self.work.step(1)?;
            let (token, start, end, _) = parse_span_key(key);
            match current {
                Some((run_token, run_start, run_end)) if run_token == token && start <= run_end => {
                    current = Some((run_token, run_start, run_end.max(end)));
                }
                _ => {
                    if let Some((run_token, run_start, run_end)) = current {
                        runs.put(&run_key(run_token, run_start), &run_end.to_be_bytes())?;
                    }
                    current = Some((token, start, end));
                }
            }
            Ok(true)
        })?;
        if let Some((run_token, run_start, run_end)) = current {
            runs.put(&run_key(run_token, run_start), &run_end.to_be_bytes())?;
        }
        let mut found_key = Vec::new();
        let mut found_value = Vec::new();
        self.for_each_row(state, statement.source.relation, |judge, _seq, row| {
            if !satisfies(&statement.source, row) {
                return Ok(true);
            }
            prefix.clear();
            encode_projection(&statement.source, row, Some(position), &mut prefix);
            let span_value = &row[usize::from(statement.source.projection[position].0)];
            let (span_start, span_end) = interval_order_words(span_value)
                .expect("positional typing pairs interval positions");
            let witnessed = match tokens.lookup_token(&prefix)? {
                None => false,
                Some(token) => {
                    runs.last_at_or_before(
                        &run_key(token, span_start),
                        &mut found_key,
                        &mut found_value,
                    )? && found_key.len() == 16
                        && found_key[..8] == token.to_be_bytes()
                        && found_value.len() == 8
                        && u64::from_be_bytes(found_value.as_slice().try_into().expect("checked"))
                            >= span_end
                }
            };
            if !witnessed {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(true)
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the two capacity passes over source and target are one \
                  linear judgment table"
    )]
    fn capacity<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &CapacityStatement,
    ) -> Result<(), JudgeError<E>> {
        let mut pending = self.pending(statement.id, StatementKind::Capacity);
        let mut totals = self.grouped();
        let mut group = Vec::new();
        let mut source_rows: u64 = 0;
        // Pass 1 — the exact nonnegative measure over DISTINCT matching
        // source facts per group, accumulated widened. A group's first
        // failure (ray duration, then overflow, in source order) is sticky
        // and only surfaces if a target row references the group — exactly
        // the reference semantics, where unreferenced groups are never
        // measured.
        self.for_each_row(state, statement.source.relation, |_judge, _seq, row| {
            source_rows += 1;
            if !satisfies(&statement.source, row) {
                return Ok(true);
            }
            group.clear();
            encode_projection(&statement.source, row, None, &mut group);
            let (mut total, mut flag) = totals.group_total(&group)?;
            if flag != 0 {
                return Ok(true);
            }
            let weight = match statement.weight {
                SealedWeight::Unit => Some(1u128),
                SealedWeight::Field(field) => match &row[usize::from(field.0)] {
                    Value::U64(weight) => Some(u128::from(*weight)),
                    _ => unreachable!("validation types a [field] weight as u64"),
                },
                SealedWeight::Duration { field, .. } => {
                    duration_words(&row[usize::from(field.0)]).map(u128::from)
                }
            };
            match weight {
                None => flag = FLAG_RAY,
                Some(weight) => match total.checked_add(weight) {
                    Some(next) => total = next,
                    None => flag = FLAG_OVERFLOW,
                },
            }
            totals.put_group_total(&group, total, flag)?;
            Ok(true)
        })?;
        // Pass 2 — every satisfying target row opens its group's window.
        // Violating group keys go into ordered scratch; citations are
        // selected by canonical bytes after both sides are offered.
        let mut violating = self.grouped();
        self.for_each_row(state, statement.target.relation, |judge, _seq, row| {
            if !satisfies(&statement.target, row) {
                return Ok(true);
            }
            group.clear();
            encode_projection(&statement.target, row, None, &mut group);
            let (total, flag) = totals.group_total(&group)?;
            if flag == FLAG_RAY {
                return Err(JudgeError::UndefinedDuration {
                    statement: statement.id,
                });
            }
            if flag == FLAG_OVERFLOW {
                return Err(JudgeError::MeasureOverflow {
                    statement: statement.id,
                });
            }
            let ceiling: Option<u128> = match statement.hi {
                SealedBound::Unbounded => None,
                SealedBound::Lit(hi) => Some(u128::from(hi)),
                SealedBound::TargetField(field) => match &row[usize::from(field.0)] {
                    Value::U64(hi) => Some(u128::from(*hi)),
                    _ => unreachable!("validation types a dependent bound as u64"),
                },
                SealedBound::Duration { field, .. } => Some(u128::from(duration_of(
                    &row[usize::from(field.0)],
                    statement.id,
                )?)),
            };
            let below = total < u128::from(statement.lo);
            let above = ceiling.is_some_and(|hi| total > hi);
            if below || above {
                pending.violated = true;
                // One violation per statement; the witnessed measure is the
                // last violating group's exact total in this deterministic
                // target order (P00-confirmed witness rule).
                pending.measure = Some(total);
                violating.put(&group, &[u8::from(above)])?;
                judge.offer(pending, statement.target.relation, row)?;
            }
            Ok(true)
        })?;
        if pending.violated {
            self.for_each_row(state, statement.source.relation, |judge, _seq, row| {
                if !satisfies(&statement.source, row) {
                    return Ok(true);
                }
                group.clear();
                encode_projection(&statement.source, row, None, &mut group);
                if violating.contains(&group)? {
                    judge.offer(pending, statement.source.relation, row)?;
                }
                Ok(true)
            })?;
        }
        let _ = source_rows;
        self.finish(pending);
        Ok(())
    }

    /// Incremental containment: judge only groups the delta can affect
    /// (added sources, removed targets) through compiled group visits.
    fn containment_delta_local<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
    ) -> Result<(), JudgeError<E>> {
        let theory = self.schema.compiled_theory().map_err(JudgeError::Compile)?;
        let target_fields = self.schema.relation(statement.target.relation).fields();
        let coverage_position = statement
            .target
            .projection
            .iter()
            .position(|field| target_fields[usize::from(field.0)].value_type.is_interval());
        let (Some(source_binding), Some(target_binding)) = (
            theory.source_binding(statement.id),
            theory.target_binding(statement.id),
        ) else {
            self.containment(state, statement)?;
            return Ok(());
        };
        let source_compiled = theory.source_projection(statement.id);
        let target_compiled = theory.target_projection(statement.id);
        let mut affected = self.grouped();
        let mut determinants = Vec::new();
        self.mark_delta_groups(
            state,
            statement.source.relation,
            &statement.source,
            source_binding,
            true,
            false,
            &mut affected,
            &mut determinants,
        )?;
        self.mark_delta_groups(
            state,
            statement.target.relation,
            &statement.target,
            target_binding,
            false,
            true,
            &mut affected,
            &mut determinants,
        )?;
        if affected.len() == 0 {
            return Ok(());
        }
        let mut pending = self
            .pending(statement.id, StatementKind::Containment)
            .with_direction(JudgedDirection::SourceUnsatisfied);
        let compiled = match coverage_position {
            None => self.containment_scalar_compiled(
                state,
                theory,
                statement,
                source_compiled,
                target_compiled,
                source_binding,
                target_binding,
                &determinants,
                &mut pending,
            )?,
            Some(position) => self.containment_pointwise_compiled(
                state,
                theory,
                statement,
                position,
                source_compiled,
                target_compiled,
                source_binding,
                target_binding,
                &determinants,
                &mut pending,
            )?,
        };
        if !compiled && !pending.violated {
            match coverage_position {
                None => {
                    self.containment_scalar_affected(
                        state,
                        statement,
                        source_binding,
                        target_binding,
                        &mut affected,
                        &mut pending,
                    )?;
                }
                Some(position) => {
                    self.containment_pointwise_affected(
                        state,
                        statement,
                        position,
                        source_binding,
                        target_binding,
                        &mut affected,
                        &mut pending,
                    )?;
                }
            }
        }
        self.finish(pending);
        Ok(())
    }

    fn containment_scalar_compiled<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        theory: &CompiledTheory,
        statement: &ContainmentStatement,
        source_compiled: Option<&CompiledProjection>,
        target_compiled: Option<&CompiledProjection>,
        source_binding: &ProjectionBinding,
        target_binding: &ProjectionBinding,
        determinants: &[Vec<Value>],
        pending: &mut PendingViolation,
    ) -> Result<bool, JudgeError<E>> {
        if let Some(sample) = determinants.first()
            && !self.compiled_indexes_live(
                state,
                theory,
                source_binding,
                target_binding,
                sample,
            )?
        {
            return Ok(false);
        }
        let mut witnesses = self.grouped();
        let mut key = Vec::new();
        for det in determinants {
            key.clear();
            encode_values(det, &mut key);
            if let Some(compiled) = target_compiled {
                match self.visit_indexed_group(state, compiled, target_binding, det, |_judge, row| {
                    if satisfies(&statement.target, row) {
                        witnesses.put(&key, &[])?;
                    }
                    Ok(true)
                })? {
                    Some(()) => {}
                    None => return Ok(false),
                }
            } else if self.unindexed_matches(
                state,
                statement.target.relation,
                &statement.target,
                target_binding,
                det,
            )? {
                witnesses.put(&key, &[])?;
            }
        }
        for det in determinants {
            key.clear();
            encode_values(det, &mut key);
            let missing = !witnesses.contains(&key)?;
            if let Some(compiled) = source_compiled {
                match self.visit_indexed_group(state, compiled, source_binding, det, |judge, row| {
                    if missing && satisfies(&statement.source, row) {
                        pending.violated = true;
                        judge.offer(pending, statement.source.relation, row)?;
                    }
                    Ok(true)
                })? {
                    Some(()) => {}
                    None => return Ok(false);
                }
            } else {
                self.offer_unindexed_unsatisfied(
                    state,
                    statement.source.relation,
                    &statement.source,
                    source_binding,
                    det,
                    missing,
                    pending,
                )?;
            }
        }
        Ok(true)
    }

    fn containment_pointwise_compiled<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        theory: &CompiledTheory,
        statement: &ContainmentStatement,
        position: usize,
        source_compiled: Option<&CompiledProjection>,
        target_compiled: Option<&CompiledProjection>,
        source_binding: &ProjectionBinding,
        target_binding: &ProjectionBinding,
        determinants: &[Vec<Value>],
        pending: &mut PendingViolation,
    ) -> Result<bool, JudgeError<E>> {
        if let Some(sample) = determinants.first()
            && !self.compiled_indexes_live(
                state,
                theory,
                source_binding,
                target_binding,
                sample,
            )?
        {
            return Ok(false);
        }
        for det in determinants {
            let mut spans = self.grouped();
            if let Some(compiled) = target_compiled {
                match self.visit_indexed_group(state, compiled, target_binding, det, |_judge, row| {
                    if satisfies(&statement.target, row) {
                        let span_value =
                            &row[usize::from(statement.target.projection[position].0)];
                        let (start, end) = interval_order_words(span_value)
                            .expect("positional typing pairs interval positions");
                        spans.put(&span_key(0, start, end, 0), &[])?;
                    }
                    Ok(true)
                })? {
                    Some(()) => {}
                    None => return Ok(false),
                }
            } else {
                self.for_each_row(state, statement.target.relation, |_judge, _seq, row| {
                    if satisfies(&statement.target, row)
                        && CompiledTheory::group_key(target_binding, row).as_slice() == det
                    {
                        let span_value =
                            &row[usize::from(statement.target.projection[position].0)];
                        let (start, end) = interval_order_words(span_value)
                            .expect("positional typing pairs interval positions");
                        spans.put(&span_key(0, start, end, 0), &[])?;
                    }
                    Ok(true)
                })?;
            }
            let mut runs = self.grouped();
            merge_coverage_runs(&mut spans, &mut runs, self.work)?;
            let mut found_key = Vec::new();
            let mut found_value = Vec::new();
            if let Some(compiled) = source_compiled {
                match self.visit_indexed_group(state, compiled, source_binding, det, |judge, row| {
                    if !satisfies(&statement.source, row) {
                        return Ok(true);
                    }
                    if !run_covers(
                        0,
                        &row[usize::from(statement.source.projection[position].0)],
                        &mut runs,
                        &mut found_key,
                        &mut found_value,
                    )? {
                        pending.violated = true;
                        judge.offer(pending, statement.source.relation, row)?;
                    }
                    Ok(true)
                })? {
                    Some(()) => {}
                    None => return Ok(false),
                }
            } else {
                self.for_each_row(state, statement.source.relation, |judge, _seq, row| {
                    if !satisfies(&statement.source, row)
                        || CompiledTheory::group_key(source_binding, row).as_slice() != det
                    {
                        return Ok(true);
                    }
                    if !run_covers(
                        0,
                        &row[usize::from(statement.source.projection[position].0)],
                        &mut runs,
                        &mut found_key,
                        &mut found_value,
                    )? {
                        pending.violated = true;
                        judge.offer(pending, statement.source.relation, row)?;
                    }
                    Ok(true)
                })?;
            }
        }
        Ok(true)
    }

    fn containment_scalar_affected<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        source_binding: &ProjectionBinding,
        target_binding: &ProjectionBinding,
        affected: &mut GroupedMap<E>,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut witnesses = self.grouped();
        let mut key = Vec::new();
        self.for_each_row(state, statement.target.relation, |_judge, _seq, row| {
            if satisfies(&statement.target, row) {
                key.clear();
                encode_values(&CompiledTheory::group_key(target_binding, row), &mut key);
                if affected.contains(&key)? {
                    witnesses.put(&key, &[])?;
                }
            }
            Ok(true)
        })?;
        self.for_each_row(state, statement.source.relation, |judge, _seq, row| {
            if !satisfies(&statement.source, row) {
                return Ok(true);
            }
            key.clear();
            encode_values(&CompiledTheory::group_key(source_binding, row), &mut key);
            if affected.contains(&key)? && !witnesses.contains(&key)? {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(true)
        })
    }

    fn containment_pointwise_affected<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        position: usize,
        source_binding: &ProjectionBinding,
        target_binding: &ProjectionBinding,
        affected: &mut GroupedMap<E>,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut tokens = self.grouped();
        let mut spans = self.grouped();
        let mut prefix = Vec::new();
        self.for_each_row(state, statement.target.relation, |_judge, _seq, row| {
            if !satisfies(&statement.target, row) {
                return Ok(true);
            }
            prefix.clear();
            encode_values(&CompiledTheory::group_key(target_binding, row), &mut prefix);
            if !affected.contains(&prefix)? {
                return Ok(true);
            }
            let token = tokens.token_of(&prefix)?;
            let span_value = &row[usize::from(statement.target.projection[position].0)];
            let (start, end) = interval_order_words(span_value)
                .expect("positional typing pairs interval positions");
            spans.put(&span_key(token, start, end, 0), &[])?;
            Ok(true)
        })?;
        let mut runs = self.grouped();
        merge_coverage_runs(&mut spans, &mut runs, self.work)?;
        let mut found_key = Vec::new();
        let mut found_value = Vec::new();
        self.for_each_row(state, statement.source.relation, |judge, _seq, row| {
            if !satisfies(&statement.source, row) {
                return Ok(true);
            }
            prefix.clear();
            encode_values(&CompiledTheory::group_key(source_binding, row), &mut prefix);
            if !affected.contains(&prefix)? {
                return Ok(true);
            }
            let span_value = &row[usize::from(statement.source.projection[position].0)];
            let witnessed = match tokens.lookup_token(&prefix)? {
                None => false,
                Some(token) => run_covers(
                    token,
                    span_value,
                    &mut runs,
                    &mut found_key,
                    &mut found_value,
                )?,
            };
            if !witnessed {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(true)
        })
    }

    /// Incremental capacity: recompute only groups the delta can change
    /// (touched source groups and added targets). Floor violations from
    /// removal and selected target replacement match the complete model.
    fn capacity_delta_local<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &CapacityStatement,
    ) -> Result<(), JudgeError<E>> {
        let theory = self.schema.compiled_theory().map_err(JudgeError::Compile)?;
        let (Some(source_binding), Some(target_binding)) = (
            theory.source_binding(statement.id),
            theory.target_binding(statement.id),
        ) else {
            self.capacity(state, statement)?;
            return Ok(());
        };
        let source_compiled = theory.source_projection(statement.id);
        let target_compiled = theory.target_projection(statement.id);
        let mut affected = self.grouped();
        let mut determinants = Vec::new();
        self.mark_delta_groups(
            state,
            statement.source.relation,
            &statement.source,
            source_binding,
            true,
            true,
            &mut affected,
            &mut determinants,
        )?;
        self.mark_delta_groups(
            state,
            statement.target.relation,
            &statement.target,
            target_binding,
            true,
            false,
            &mut affected,
            &mut determinants,
        )?;
        if affected.len() == 0 {
            return Ok(());
        }
        let mut pending = self.pending(statement.id, StatementKind::Capacity);
        if !self.capacity_compiled(
            state,
            theory,
            statement,
            source_compiled,
            target_compiled,
            source_binding,
            target_binding,
            &determinants,
            &mut pending,
        )? && !pending.violated
        {
            self.capacity_affected(
                state,
                statement,
                source_binding,
                target_binding,
                &mut affected,
                &mut pending,
            )?;
        }
        self.finish(pending);
        Ok(())
    }

    fn capacity_compiled<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        theory: &CompiledTheory,
        statement: &CapacityStatement,
        source_compiled: Option<&CompiledProjection>,
        target_compiled: Option<&CompiledProjection>,
        source_binding: &ProjectionBinding,
        target_binding: &ProjectionBinding,
        determinants: &[Vec<Value>],
        pending: &mut PendingViolation,
    ) -> Result<bool, JudgeError<E>> {
        if let Some(sample) = determinants.first()
            && !self.compiled_indexes_live(
                state,
                theory,
                source_binding,
                target_binding,
                sample,
            )?
        {
            return Ok(false);
        }
        let mut totals = self.grouped();
        let mut group = Vec::new();
        for det in determinants {
            group.clear();
            encode_values(det, &mut group);
            if let Some(compiled) = source_compiled {
                match self.visit_indexed_group(state, compiled, source_binding, det, |_judge, row| {
                    if satisfies(&statement.source, row) {
                        accumulate_capacity(&mut totals, statement, row, &group)?;
                    }
                    Ok(true)
                })? {
                    Some(()) => {}
                    None => return Ok(false),
                }
            } else {
                self.for_each_row(state, statement.source.relation, |_judge, _seq, row| {
                    if satisfies(&statement.source, row)
                        && CompiledTheory::group_key(source_binding, row).as_slice() == det
                    {
                        accumulate_capacity(&mut totals, statement, row, &group)?;
                    }
                    Ok(true)
                })?;
            }
        }
        let mut violating = self.grouped();
        for det in determinants {
            group.clear();
            encode_values(det, &mut group);
            if let Some(compiled) = target_compiled {
                match self.visit_indexed_group(state, compiled, target_binding, det, |judge, row| {
                    if satisfies(&statement.target, row) {
                        judge.note_capacity_target(
                            statement,
                            row,
                            &group,
                            &mut totals,
                            &mut violating,
                            pending,
                        )?;
                    }
                    Ok(true)
                })? {
                    Some(()) => {}
                    None => return Ok(false),
                }
            } else {
                self.for_each_row(state, statement.target.relation, |judge, _seq, row| {
                    if satisfies(&statement.target, row)
                        && CompiledTheory::group_key(target_binding, row).as_slice() == det
                    {
                        judge.note_capacity_target(
                            statement,
                            row,
                            &group,
                            &mut totals,
                            &mut violating,
                            pending,
                        )?;
                    }
                    Ok(true)
                })?;
            }
        }
        if pending.violated {
            for det in determinants {
                group.clear();
                encode_values(det, &mut group);
                if !violating.contains(&group)? {
                    continue;
                }
                if let Some(compiled) = source_compiled {
                    match self.visit_indexed_group(
                        state,
                        compiled,
                        source_binding,
                        det,
                        |judge, row| {
                            if satisfies(&statement.source, row) {
                                judge.offer(pending, statement.source.relation, row)?;
                            }
                            Ok(true)
                        },
                    )? {
                        Some(()) => {}
                        None => return Ok(false),
                    }
                } else {
                    self.for_each_row(state, statement.source.relation, |judge, _seq, row| {
                        if satisfies(&statement.source, row)
                            && CompiledTheory::group_key(source_binding, row).as_slice() == det
                        {
                            judge.offer(pending, statement.source.relation, row)?;
                        }
                        Ok(true)
                    })?;
                }
            }
        }
        Ok(true)
    }

    fn capacity_affected<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &CapacityStatement,
        source_binding: &ProjectionBinding,
        target_binding: &ProjectionBinding,
        affected: &mut GroupedMap<E>,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut totals = self.grouped();
        let mut group = Vec::new();
        self.for_each_row(state, statement.source.relation, |_judge, _seq, row| {
            if !satisfies(&statement.source, row) {
                return Ok(true);
            }
            group.clear();
            encode_values(&CompiledTheory::group_key(source_binding, row), &mut group);
            if !affected.contains(&group)? {
                return Ok(true);
            }
            accumulate_capacity(&mut totals, statement, row, &group)?;
            Ok(true)
        })?;
        let mut violating = self.grouped();
        self.for_each_row(state, statement.target.relation, |judge, _seq, row| {
            if !satisfies(&statement.target, row) {
                return Ok(true);
            }
            group.clear();
            encode_values(&CompiledTheory::group_key(target_binding, row), &mut group);
            if !affected.contains(&group)? {
                return Ok(true);
            }
            judge.note_capacity_target(
                statement,
                row,
                &group,
                &mut totals,
                &mut violating,
                pending,
            )?;
            Ok(true)
        })?;
        if pending.violated {
            self.for_each_row(state, statement.source.relation, |judge, _seq, row| {
                if !satisfies(&statement.source, row) {
                    return Ok(true);
                }
                group.clear();
                encode_values(&CompiledTheory::group_key(source_binding, row), &mut group);
                if violating.contains(&group)? {
                    judge.offer(pending, statement.source.relation, row)?;
                }
                Ok(true)
            })?;
        }
        Ok(())
    }

    fn note_capacity_target(
        &mut self,
        statement: &CapacityStatement,
        row: &[Value],
        group: &[u8],
        totals: &mut GroupedMap<E>,
        violating: &mut GroupedMap<E>,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let (total, flag) = totals.group_total(group)?;
        if flag == FLAG_RAY {
            return Err(JudgeError::UndefinedDuration {
                statement: statement.id,
            });
        }
        if flag == FLAG_OVERFLOW {
            return Err(JudgeError::MeasureOverflow {
                statement: statement.id,
            });
        }
        let ceiling: Option<u128> = match statement.hi {
            SealedBound::Unbounded => None,
            SealedBound::Lit(hi) => Some(u128::from(hi)),
            SealedBound::TargetField(field) => match &row[usize::from(field.0)] {
                Value::U64(hi) => Some(u128::from(*hi)),
                _ => unreachable!("validation types a dependent bound as u64"),
            },
            SealedBound::Duration { field, .. } => Some(u128::from(duration_of(
                &row[usize::from(field.0)],
                statement.id,
            )?)),
        };
        let below = total < u128::from(statement.lo);
        let above = ceiling.is_some_and(|hi| total > hi);
        if below || above {
            pending.violated = true;
            pending.measure = Some(total);
            violating.put(group, &[])?;
            self.offer(pending, statement.target.relation, row)?;
        }
        Ok(())
    }

    fn compiled_indexes_live<S: DeltaFacts<Error = E>>(
        &self,
        state: &S,
        theory: &CompiledTheory,
        source_binding: &ProjectionBinding,
        target_binding: &ProjectionBinding,
        sample: &[Value],
    ) -> Result<bool, JudgeError<E>> {
        for binding in [source_binding, target_binding] {
            let Some(id) = binding.projection else {
                continue;
            };
            let Some(compiled) = theory.projection(id) else {
                return Ok(false);
            };
            let Some(physical) = CompiledTheory::index_key(binding, sample) else {
                return Ok(false);
            };
            match state.visit_compiled_group(compiled, &physical, &mut |_| Ok(false)) {
                Ok(Some(())) => {}
                Ok(None) => return Ok(false),
                Err(error) => return Err(JudgeError::State(error)),
            }
        }
        Ok(true)
    }

    fn unindexed_matches<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        relation: RelationId,
        side: &Side,
        binding: &ProjectionBinding,
        det: &[Value],
    ) -> Result<bool, JudgeError<E>> {
        let mut found = false;
        self.for_each_row(state, relation, |_judge, _seq, row| {
            if satisfies(side, row) && CompiledTheory::group_key(binding, row).as_slice() == det {
                found = true;
            }
            Ok(true)
        })?;
        Ok(found)
    }

    fn offer_unindexed_unsatisfied<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        relation: RelationId,
        side: &Side,
        binding: &ProjectionBinding,
        det: &[Value],
        missing: bool,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        if !missing {
            return Ok(());
        }
        self.for_each_row(state, relation, |judge, _seq, row| {
            if satisfies(side, row) && CompiledTheory::group_key(binding, row).as_slice() == det {
                pending.violated = true;
                judge.offer(pending, relation, row)?;
            }
            Ok(true)
        })
    }

    fn mark_delta_groups<S: DeltaFacts<Error = E>>(
        &mut self,
        state: &S,
        relation: RelationId,
        side: &Side,
        binding: &ProjectionBinding,
        adds: bool,
        removes: bool,
        affected: &mut GroupedMap<E>,
        determinants: &mut Vec<Vec<Value>>,
    ) -> Result<(), JudgeError<E>> {
        let mut key = Vec::new();
        let mut walk_error = None;
        let mut mark = |row: &[Value]| -> Result<bool, S::Error> {
            if satisfies(side, row) {
                let det = CompiledTheory::group_key(binding, row);
                key.clear();
                encode_values(&det, &mut key);
                match affected.insert_if_absent(&key) {
                    Ok(true) => determinants.push(det),
                    Ok(false) => {}
                    Err(error) => {
                        walk_error = Some(error);
                        return Ok(false);
                    }
                }
            }
            Ok(true)
        };
        if adds {
            state
                .visit_added_rows(relation, &mut mark)
                .map_err(JudgeError::State)?;
            if let Some(error) = walk_error.take() {
                return Err(error);
            }
        }
        if removes {
            state
                .visit_removed_rows(relation, &mut mark)
                .map_err(JudgeError::State)?;
            if let Some(error) = walk_error.take() {
                return Err(error);
            }
        }
        Ok(())
    }

    fn finish(&mut self, pending: PendingViolation) {
        if pending.violated {
            let (examples, truncated, charges) = pending.citations.into_examples();
            self.citation_charges.extend(charges);
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
            citations: CitationTopK::new(budget),
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

/// Append the side's projected values as exact injective bytes, optionally
/// skipping one projection position (the pointwise interval slot).
fn encode_projection(side: &Side, row: &[Value], skip: Option<usize>, out: &mut Vec<u8>) {
    for (index, field) in side.projection.iter().enumerate() {
        if Some(index) == skip {
            continue;
        }
        encode_value(&row[usize::from(field.0)], out);
    }
}

fn encode_values(values: &[Value], out: &mut Vec<u8>) {
    for value in values {
        encode_value(value, out);
    }
}

fn merge_coverage_runs<E>(
    spans: &mut GroupedMap<E>,
    runs: &mut GroupedMap<E>,
    work: &WorkContext,
) -> Result<(), JudgeError<E>> {
    let mut current: Option<(u64, u64, u64)> = None;
    spans.for_each(|key, _| {
        work.step(1)?;
        let (token, start, end, _) = parse_span_key(key);
        match current {
            Some((run_token, run_start, run_end)) if run_token == token && start <= run_end => {
                current = Some((run_token, run_start, run_end.max(end)));
            }
            _ => {
                if let Some((run_token, run_start, run_end)) = current {
                    runs.put(&run_key(run_token, run_start), &run_end.to_be_bytes())?;
                }
                current = Some((token, start, end));
            }
        }
        Ok(true)
    })?;
    if let Some((run_token, run_start, run_end)) = current {
        runs.put(&run_key(run_token, run_start), &run_end.to_be_bytes())?;
    }
    Ok(())
}

fn run_covers<E>(
    token: u64,
    span: &Value,
    runs: &mut GroupedMap<E>,
    found_key: &mut Vec<u8>,
    found_value: &mut Vec<u8>,
) -> Result<bool, JudgeError<E>> {
    let (span_start, span_end) =
        interval_order_words(span).expect("positional typing pairs interval positions");
    Ok(runs.last_at_or_before(&run_key(token, span_start), found_key, found_value)?
        && found_key.len() == 16
        && found_key[..8] == token.to_be_bytes()
        && found_value.len() == 8
        && u64::from_be_bytes(found_value.as_slice().try_into().expect("checked")) >= span_end)
}

fn accumulate_capacity<E>(
    totals: &mut GroupedMap<E>,
    statement: &CapacityStatement,
    row: &[Value],
    group: &[u8],
) -> Result<(), JudgeError<E>> {
    let (mut total, mut flag) = totals.group_total(group)?;
    if flag != 0 {
        return Ok(());
    }
    let weight = match statement.weight {
        SealedWeight::Unit => Some(1u128),
        SealedWeight::Field(field) => match &row[usize::from(field.0)] {
            Value::U64(weight) => Some(u128::from(*weight)),
            _ => unreachable!("validation types a [field] weight as u64"),
        },
        SealedWeight::Duration { field, .. } => {
            duration_words(&row[usize::from(field.0)]).map(u128::from)
        }
    };
    match weight {
        None => flag = FLAG_RAY,
        Some(weight) => match total.checked_add(weight) {
            Some(next) => total = next,
            None => flag = FLAG_OVERFLOW,
        },
    }
    totals.put_group_total(group, total, flag)
}

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

fn run_key(token: u64, start: u64) -> [u8; 16] {
    let mut key = [0u8; 16];
    key[..8].copy_from_slice(&token.to_be_bytes());
    key[8..].copy_from_slice(&start.to_be_bytes());
    key
}

/// The exact integer duration of a discrete interval value; a ray refuses
/// (undefined measure), and a float interval is unreachable — validation
/// refuses float-duration weights and bounds.
fn duration_of<E>(value: &Value, statement: StatementId) -> Result<u64, JudgeError<E>> {
    duration_words(value).ok_or(JudgeError::UndefinedDuration { statement })
}

/// As [`duration_of`], with the ray refusal as `None` (the sticky per-group
/// flag's channel).
fn duration_words(value: &Value) -> Option<u64> {
    match value {
        Value::IntervalU64(interval) => interval.duration(),
        Value::IntervalI64(interval) => interval.duration(),
        Value::IntervalF64(_) => {
            unreachable!("validation refuses float intervals in duration positions")
        }
        _ => unreachable!("validation types duration positions as intervals"),
    }
}

/// Interval total-order words for one decoded interval value — shared by
/// the reference judge and compiled index consumers.
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
mod f3c_bounded;

#[cfg(test)]
mod discriminators;
