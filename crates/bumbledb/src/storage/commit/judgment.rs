//! Phase 3, the containment judgment. Source side:
//! every inserted fact satisfying a statement's source selection proves
//! determinant probe, interval positions by the coverage walk. Target side:
//! statements' `R` prefixes for surviving sources — a scalar survivor is
//! the violation outright; an interval survivor re-runs the coverage walk
//! against the final `U` state. LMDB write transactions read their own

use std::collections::BTreeSet;
use std::ops::Bound;

use super::plan::{CommitPlan, IncrementalObligations, MarkWeight, Owed, RKeyOp};
use super::{decode_row_id, fact_by_row};
use crate::encoding::{
    FactLayout, InternId, encode_u64, field_bytes, field_word_bytes, interval_words,
};
use crate::error::{
    Admission, Check, CorruptionError, Direction, Error, Result, Violation, Violations,
};
use crate::interval::sweep::{Continuation, sweep};
use crate::obs;
use crate::schema::{
    AxiomIndex, BoundCeiling, CapacityEnforcement, CapacityId, CapacityStatement, CompiledCheck,
    CompiledSide, ContainmentId, DisjointDeterminantProof, EncodableCheck, Enforcement, KeyForm,
    KeyId, Schema, SealedBound, SealedWeight, Survivors, ValueType,
};
use crate::storage::catalog::{
    Bounds, CatalogMap, CatalogRead, LmdbPeekCatalog, LmdbSortedGets, ReadCursor, SortedGets,
};
use crate::storage::delta::WriteDelta;
use crate::storage::env::{ReadTxn, WriteTxn};
use crate::storage::keys::{self, DeterminantImage, KeyBuf, MAX_KEY};
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

pub(super) struct FinalStateView<'state, 'env, 'delta> {
    txn: &'state WriteTxn<'env>,
    schema: &'state Schema,
    plan: &'state CommitPlan<'delta>,
}

impl<'state, 'env, 'delta> FinalStateView<'state, 'env, 'delta> {
    pub(super) fn new(
        txn: &'state WriteTxn<'env>,
        schema: &'state Schema,
        plan: &'state CommitPlan<'delta>,
    ) -> Self {
        Self { txn, schema, plan }
    }
}

/// Judges the whole statement phase against one named final state —
/// containments (both directions) and capacity statements (per touched parent)
/// — and seals the complete violation set of the phase
/// (`lean/Bumbledb/Txn.lean: rejection_is_complete`, the statement arm).
pub(super) fn judge(view: &FinalStateView<'_, '_, '_>) -> Result<Admission<()>> {
    let obligations = view.plan.incremental_obligations();
    let mut violations = Vec::new();
    check_source(view, &obligations, &mut violations)?;
    check_target(view, &obligations, &mut violations)?;
    check_capacities(view, &obligations, &mut violations)?;
    Ok(Violations::seal(view.plan.selections.schema(), violations))
}

/// One source fact's weight under a capacity statement's measure
/// (`lean/Bumbledb/Capacity.lean: Weight.apply`): `Unit` is 1 — the count
/// instance; `Field` reads the u64-encoded SOURCE position; `DurationOf` reads
/// the SOURCE interval position's measure in encoded word space (`end − start`
/// — both element encodings preserve differences, the R5 machinery). A
/// ray-valued Duration weight is the typed C10 refusal naming the row — a ray
/// has no finite measure.
pub(crate) fn child_weight(
    statement: &CapacityStatement,
    layout: &FactLayout,
    fact: &[u8],
) -> Result<u64> {
    measure_weight(statement.weight, layout, fact, statement.id)
}

fn exceeds_ceiling(measure: u128, hi: BoundCeiling) -> bool {
    match hi {
        BoundCeiling::Unbounded => false,
        BoundCeiling::Finite(n) => measure > u128::from(n),
    }
}

/// The ONE weight arithmetic behind [`child_weight`] — spelled over the raw
/// `(weight, sealed tail)` pair so validate's closed-constant arm (which
/// measures extension rows before any [`CapacityStatement`] exists) reads THIS
/// definition too: the measure law has exactly one engine definition, never an
/// inline re-implementation.
pub(crate) fn measure_weight(
    weight: SealedWeight,
    layout: &FactLayout,
    fact: &[u8],
    statement: bumbledb_theory::schema::StatementId,
) -> Result<u64> {
    match weight {
        SealedWeight::Unit => Ok(1),
        SealedWeight::Field(field) => Ok(u64::from_be_bytes(field_word_bytes(
            layout.encoded(fact),
            usize::from(field.0),
        ))),
        SealedWeight::Duration { field, tail } => interval_measure(
            tail,
            field_bytes(layout.encoded(fact), usize::from(field.0)),
            statement,
            fact,
        ),
    }
}

/// The ONE ceiling resolution behind [`Checker::resolve_hi`]
/// (`lean/Bumbledb/Capacity.lean: CapWindow.resolve`) — spelled over the raw
/// `(bound, sealed tail)` pair so validate's closed-constant arm reads THIS
/// definition too, exactly as [`measure_weight`]: a literal passes through; a
/// dependent bound reads the named TARGET-row field — u64 word or interval
/// measure — off the holder fact in hand.
pub(crate) fn resolve_bound(
    bound: SealedBound,
    layout: &FactLayout,
    parent_fact: &[u8],
    statement: bumbledb_theory::schema::StatementId,
) -> Result<BoundCeiling> {
    match bound {
        SealedBound::Unbounded => Ok(BoundCeiling::Unbounded),
        SealedBound::Lit(n) => Ok(BoundCeiling::Finite(n)),
        SealedBound::TargetField(field) => Ok(BoundCeiling::Finite(u64::from_be_bytes(
            field_word_bytes(layout.encoded(parent_fact), usize::from(field.0)),
        ))),
        SealedBound::Duration { field, tail } => interval_measure(
            tail,
            field_bytes(layout.encoded(parent_fact), usize::from(field.0)),
            statement,
            parent_fact,
        )
        .map(BoundCeiling::Finite),
    }
}

pub(crate) fn expected_slot_weight(
    statement: &CapacityStatement,
    layout: &FactLayout,
    fact: &[u8],
) -> Result<Option<u64>> {
    match SlotShape::of(statement.weight) {
        SlotShape::Empty => Ok(None),
        SlotShape::Word => Ok(Some(child_weight(statement, layout, fact)?)),
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SlotShape {
    Empty,
    Word,
}

impl SlotShape {
    pub(crate) fn of(weight: SealedWeight) -> Self {
        match weight {
            SealedWeight::Unit => Self::Empty,
            SealedWeight::Field(_) | SealedWeight::Duration { .. } => Self::Word,
        }
    }

    fn decode(self, value: &[u8]) -> Result<u64> {
        match self {
            Self::Empty => {
                if !value.is_empty() {
                    return Err(Error::Corruption(CorruptionError::MalformedValue(
                        "R capacity value width",
                    )));
                }
                Ok(1)
            }
            Self::Word => {
                let word: [u8; 8] = value.try_into().map_err(|_| {
                    Error::Corruption(CorruptionError::MalformedValue("R capacity value width"))
                })?;
                Ok(u64::from_le_bytes(word))
            }
        }
    }
}

/// A ray (`end == u64::MAX` in both element encodings) has no finite measure —
/// the typed commit refusal naming the row (ruled 2026-07-24, C10;
/// [`crate::Error::CapacityRayMeasure`]).
fn interval_measure(
    tail: ValueType,
    bytes: &[u8],
    statement: bumbledb_theory::schema::StatementId,
    fact: &[u8],
) -> Result<u64> {
    let (start, end) = interval_words(tail, bytes).ok_or(Error::Corruption(
        CorruptionError::MalformedValue("capacity interval field"),
    ))?;
    if end == u64::MAX {
        return Err(Error::CapacityRayMeasure {
            statement,
            fact: fact.into(),
        });
    }
    end.checked_sub(start)
        .ok_or(Error::Corruption(CorruptionError::MalformedValue(
            "capacity interval inverted",
        )))
}

/// One binding's pre-encoded comparison: the singleton compare (today's
/// equality, one slice compare) or the disjunctive set's alternatives
/// (membership among the sealed encodings — `lean/Bumbledb/Schema.lean:
/// Selection.satisfies`, the field's value a MEMBER of the spelled set).
pub(crate) enum FieldCheck {
    One(Box<[u8]>),

    AnyOf(Box<[Box<[u8]>]>),
}

impl FieldCheck {
    fn matches(&self, actual: &[u8]) -> bool {
        match self {
            Self::One(literal) => actual == &literal[..],
            Self::AnyOf(alternatives) => alternatives.iter().any(|bytes| actual == &bytes[..]),
        }
    }
}

pub(crate) enum SelectionCheck {
    Empty,

    Compare(Box<[(FieldId, FieldCheck)]>),

    Never,
}

pub(crate) struct SideChecks {
    pub(crate) source: SelectionCheck,
    pub(crate) target: SelectionCheck,
}

pub(crate) struct Selections<'s> {
    schema: &'s Schema,

    checks: Box<[SideChecks]>,

    capacities: Box<[SideChecks]>,
}

impl<'s> Selections<'s> {
    pub(crate) fn encode(delta: &'s WriteDelta<'_>, view: &ReadTxn<'_>) -> Result<Self> {
        Self::encode_with(delta.schema(), &mut |raw| delta.resolve(view, raw))
    }

    pub(crate) fn encode_committed(schema: &'s Schema, view: &ReadTxn<'_>) -> Result<Self> {
        Self::encode_with(schema, &mut |raw| crate::storage::dict::lookup(view, raw))
    }

    pub(crate) fn encode_lookup(
        schema: &'s Schema,
        mut lookup: impl FnMut(&[u8]) -> Result<Option<InternId>>,
    ) -> Result<Self> {
        Self::encode_with(schema, &mut lookup)
    }

    fn encode_with<F>(schema: &'s Schema, resolve: &mut F) -> Result<Self>
    where
        F: FnMut(&[u8]) -> Result<Option<InternId>>,
    {
        let checks = schema
            .containments()
            .iter()
            .map(|statement| {
                Ok(SideChecks {
                    source: resolve_side(&statement.checks.source, resolve)?,
                    target: resolve_side(&statement.checks.target, resolve)?,
                })
            })
            .collect::<Result<Box<[_]>>>()?;
        let capacities = schema
            .capacities()
            .iter()
            .map(|statement| {
                Ok(SideChecks {
                    source: resolve_side(&statement.checks.source, resolve)?,
                    target: resolve_side(&statement.checks.target, resolve)?,
                })
            })
            .collect::<Result<Box<[_]>>>()?;
        Ok(Self {
            schema,
            checks,
            capacities,
        })
    }

    pub(crate) fn schema(&self) -> &'s Schema {
        self.schema
    }

    pub(crate) fn bind(self, schema: &Schema) -> Selections<'_> {
        debug_assert!(std::ptr::eq(self.schema, schema));
        Selections {
            schema,
            checks: self.checks,
            capacities: self.capacities,
        }
    }

    pub(crate) fn containment(&self, id: ContainmentId) -> &SideChecks {
        &self.checks[usize::from(id.0)]
    }

    pub(crate) fn capacity(&self, id: CapacityId) -> &SideChecks {
        &self.capacities[usize::from(id.0)]
    }
}

fn resolve_side<F>(side: &CompiledSide, resolve: &mut F) -> Result<SelectionCheck>
where
    F: FnMut(&[u8]) -> Result<Option<InternId>>,
{
    if side.is_empty() {
        return Ok(SelectionCheck::Empty);
    }
    if let Some(checks) = side.ordinary() {
        resolve_checks(checks, resolve)
    } else if let Some(checks) = side.closed() {
        Ok(resolve_encodable(checks))
    } else {
        unreachable!("CompiledSide is Ordinary or Closed")
    }
}

fn resolve_encodable(checks: &[EncodableCheck]) -> SelectionCheck {
    if checks.is_empty() {
        return SelectionCheck::Empty;
    }
    SelectionCheck::Compare(
        checks
            .iter()
            .map(|check| match check {
                EncodableCheck::Encoded { field, bytes } => {
                    (*field, FieldCheck::One(bytes.clone()))
                }
                EncodableCheck::EncodedSet {
                    field,
                    alternatives,
                } => (*field, FieldCheck::AnyOf(alternatives.clone())),
            })
            .collect(),
    )
}

/// A set binding's never-interned `str` alternatives drop out of the
/// disjunction (each is individually unsatisfiable); a binding with nothing
/// left is `Never`.
fn resolve_checks<F>(compiled: &[CompiledCheck], resolve: &mut F) -> Result<SelectionCheck>
where
    F: FnMut(&[u8]) -> Result<Option<InternId>>,
{
    if compiled.is_empty() {
        return Ok(SelectionCheck::Empty);
    }
    let mut fields = Vec::with_capacity(compiled.len());
    for check in compiled {
        let (field, encoded): (FieldId, FieldCheck) = match check {
            CompiledCheck::Encoded { field, bytes } => (*field, FieldCheck::One(bytes.clone())),
            CompiledCheck::EncodedSet {
                field,
                alternatives,
            } => (*field, FieldCheck::AnyOf(alternatives.clone())),
            CompiledCheck::Interned { field, text } => match resolve(text.as_bytes())? {
                Some(id) => (*field, FieldCheck::One(Box::new(encode_u64(id.raw())))),
                None => return Ok(SelectionCheck::Never),
            },
            CompiledCheck::InternedSet { field, texts } => {
                let mut alternatives = Vec::with_capacity(texts.len());
                for text in texts {
                    if let Some(id) = resolve(text.as_bytes())? {
                        alternatives.push(Box::new(encode_u64(id.raw())) as Box<[u8]>);
                    }
                }
                if alternatives.is_empty() {
                    return Ok(SelectionCheck::Never);
                }
                (*field, FieldCheck::AnyOf(alternatives.into()))
            }
        };
        fields.push((field, encoded));
    }
    Ok(SelectionCheck::Compare(fields.into()))
}

pub(crate) fn satisfies(check: &SelectionCheck, layout: &FactLayout, fact_bytes: &[u8]) -> bool {
    match check {
        SelectionCheck::Empty => true,
        SelectionCheck::Never => false,
        SelectionCheck::Compare(fields) => fields.iter().all(|(field, literal)| {
            literal.matches(field_bytes(
                layout.encoded(fact_bytes),
                usize::from(field.0),
            ))
        }),
    }
}

pub(crate) fn collect(outcome: Result<Check>, violations: &mut Vec<Violation>) -> Result<()> {
    match outcome? {
        Check::Holds => Ok(()),
        Check::Violated(violation) => {
            violations.push(violation);
            Ok(())
        }
    }
}

pub(super) fn check_source(
    view: &FinalStateView<'_, '_, '_>,
    obligations: &IncrementalObligations<'_, '_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let FinalStateView { txn, schema, plan } = view;
    let catalog = LmdbPeekCatalog::new(txn);
    let mut checker = Checker::new(&catalog, schema);
    let mut probes = 0u64;
    let mut span = obs::span(obs::names::JUDGMENT_SOURCE);

    let mut worklist: Vec<(ContainmentId, &RKeyOp<MarkWeight>, &[u8])> =
        obligations.source_edges().collect();
    worklist.sort_unstable_by(|(a, a_edge, a_fact), (b, b_edge, b_fact)| {
        (*a, &a_edge.key_bytes, *a_fact).cmp(&(*b, &b_edge.key_bytes, *b_fact))
    });

    let mut sorted_gets = LmdbSortedGets::new(txn.raw(), txn.env().data());
    let mut group: Option<crate::schema::ContainmentId> = None;
    for (containment, edge, fact_bytes) in worklist {
        probes += 1;
        if group != Some(containment) {
            sorted_gets.reset();
            group = Some(containment);
        }
        let statement = schema.containment(containment);
        let probe = Probe::of(
            statement,
            &plan.selections.containment(containment).target,
            &edge.key_bytes,
            fact_bytes,
            Direction::SourceUnsatisfied,
        );
        let outcome = match &statement.enforcement {
            Enforcement::ScalarProbe { .. } => {
                checker.check_scalar_sorted(&probe, &mut sorted_gets)
            }
            Enforcement::IntervalCoverage {
                disjoint,
                source_tail,
                target_tail,
                ..
            } => checker.check_coverage(*disjoint, *source_tail, *target_tail, &probe),
            Enforcement::Closed { .. } => {
                unreachable!("closed-target containments produce memberships, not edges")
            }
        };
        collect(outcome, violations)?;
    }
    for membership in obligations.memberships() {
        collect(Ok(membership.check.clone()), violations)?;
    }
    span.set_count(probes);
    span.end();
    Ok(())
}

#[derive(Clone, Copy)]
struct AffectedSource<'a> {
    containment: ContainmentId,
    source_rel: RelationId,
    source_row: u64,
    key_bytes: &'a [u8],
    disjoint: DisjointDeterminantProof,
    source_tail: ValueType,
    target_tail: ValueType,
}

impl AffectedSource<'_> {
    fn identity(&self) -> (ContainmentId, RelationId, u64, &[u8]) {
        (
            self.containment,
            self.source_rel,
            self.source_row,
            self.key_bytes,
        )
    }
}

impl PartialEq for AffectedSource<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}

impl Eq for AffectedSource<'_> {}

impl PartialOrd for AffectedSource<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AffectedSource<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identity().cmp(&other.identity())
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
pub(super) fn check_target(
    view: &FinalStateView<'_, '_, '_>,
    obligations: &IncrementalObligations<'_, '_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let FinalStateView { txn, schema, plan } = view;
    let data = txn.env().data();
    let mut span = obs::span(obs::names::JUDGMENT_TARGET);
    let mut scanned = 0u64;
    let mut key: KeyBuf = [0; MAX_KEY];

    // Affected sources of interval statements, deduped before any walk:

    let mut affected: BTreeSet<AffectedSource<'_>> = BTreeSet::new();
    for check in obligations.target_checks() {
        let determinant = check.determinant.as_bytes();
        let key_statement = schema.key(check.key);

        let mut establisher: Option<&[u8]> = None;
        let mut counted = false;
        for dependent in &check.dependents {
            let statement = schema.containment(dependent.containment);
            let sid = statement.id;
            if let Owed::IfEstablisherFails = dependent.owed {
                let fact = if let Some(fact) = establisher {
                    fact
                } else {
                    let fact = establishing_fact(data, txn, schema, check.key, determinant)?;
                    establisher = Some(fact);
                    fact
                };
                let target_check = &plan.selections.containment(dependent.containment).target;
                if satisfies(
                    target_check,
                    schema.relation(key_statement.relation).layout(),
                    fact,
                ) {
                    continue;
                }
            }
            if !counted {
                scanned += 1;
                counted = true;
            }
            match (statement.survivors, &statement.enforcement) {
                (
                    Survivors::ReverseEdges,
                    Enforcement::IntervalCoverage {
                        disjoint,
                        source_tail,
                        target_tail,
                        ..
                    },
                ) => {
                    let source_tail = *source_tail;
                    let target_tail = *target_tail;
                    let (ts, te) = interval_words(
                        target_tail,
                        &determinant[determinant.len() - target_tail.width()..],
                    )
                    .ok_or(Error::Corruption(
                        CorruptionError::MalformedValue("U determinant tail"),
                    ))?;
                    let group = &determinant[..determinant.len() - target_tail.width()];
                    let prefix = keys::reverse_prefix(&mut key, sid, group);
                    let bounds: (Bound<&[u8]>, Bound<&[u8]>) =
                        (Bound::Included(prefix), Bound::Unbounded);
                    for group_entry in data.range(txn.raw(), &bounds)? {
                        let (k, _) = group_entry?;
                        if !k.starts_with(prefix) {
                            break;
                        }
                        let Some((_, key_bytes, source_rel, source_row)) =
                            keys::parse_reverse_key(k)
                        else {
                            return Err(Error::Corruption(CorruptionError::MalformedValue(
                                "R key shape",
                            )));
                        };
                        if key_bytes.len() != group.len() + source_tail.width() {
                            return Err(Error::Corruption(CorruptionError::MalformedValue(
                                "R key width",
                            )));
                        }
                        let (ss, se) = interval_words(
                            source_tail,
                            &key_bytes[key_bytes.len() - source_tail.width()..],
                        )
                        .ok_or(Error::Corruption(
                            CorruptionError::MalformedValue("R key interval tail"),
                        ))?;
                        if ss < te && ts < se {
                            affected.insert(AffectedSource {
                                containment: dependent.containment,
                                key_bytes,
                                source_rel,
                                source_row,
                                disjoint: *disjoint,
                                source_tail,
                                target_tail,
                            });
                        }
                    }
                }
                (Survivors::SealedRows, Enforcement::ScalarProbe { .. }) => {
                    if let Some(row) =
                        closed_source_survivor(schema, plan, dependent.containment, determinant)
                    {
                        violations.push(Violation::containment(
                            schema.cite(sid),
                            Direction::TargetRequired,
                            row,
                        ));
                    }
                }
                (
                    Survivors::ReverseEdges,
                    Enforcement::ScalarProbe { .. } | Enforcement::Closed { .. },
                ) => {
                    let prefix = keys::reverse_prefix(&mut key, sid, determinant);
                    let bounds: (Bound<&[u8]>, Bound<&[u8]>) =
                        (Bound::Included(prefix), Bound::Unbounded);
                    for entry in data.range(txn.raw(), &bounds)? {
                        let (r_key, _) = entry?;
                        if !r_key.starts_with(prefix) {
                            break;
                        }
                        let (_, _, source_rel, source_row) = keys::parse_reverse_key(r_key).ok_or(
                            Error::Corruption(CorruptionError::MalformedValue("R key shape")),
                        )?;
                        let fact = fact_by_row(data, txn.raw(), schema, source_rel, source_row)?;
                        if plan.inserts_fact(source_rel, fact.bytes()) {
                            continue;
                        }
                        violations.push(Violation::containment(
                            schema.cite(sid),
                            Direction::TargetRequired,
                            fact.bytes().into(),
                        ));
                        break;
                    }
                }
                (Survivors::SealedRows, Enforcement::Closed { .. }) => {}
                (Survivors::SealedRows, Enforcement::IntervalCoverage { .. }) => {
                    unreachable!("closed sources refuse interval containments at validate")
                }
            }
        }
    }

    let catalog = LmdbPeekCatalog::new(txn);
    let mut checker = Checker::new(&catalog, schema);
    for source in &affected {
        let statement = schema.containment(source.containment);
        let fact_bytes = fact_by_row(
            data,
            txn.raw(),
            schema,
            source.source_rel,
            source.source_row,
        )?;
        if plan.inserts_fact(source.source_rel, fact_bytes.bytes()) {
            continue;
        }
        let probe = Probe::of(
            statement,
            &plan.selections.containment(source.containment).target,
            source.key_bytes,
            fact_bytes.bytes(),
            Direction::TargetRequired,
        );
        collect(
            checker.check_coverage(
                source.disjoint,
                source.source_tail,
                source.target_tail,
                &probe,
            ),
            violations,
        )?;
    }
    span.set_count(scanned);
    span.end();
    Ok(())
}

/// The capacity judgment: every TOUCHED parent key tuple — every tuple any
/// delta child fact projects to, plus the delta's ψ-selected parents themselves
/// (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`) — resolves its
/// ψ-selected holder in the final state, resolves any dependent bound from the
/// holder's own row, and measures its child group against the window
/// (`lean/Bumbledb/Oracle.lean: capacity_plan_decides` — the walk's measure
/// verdict IS the delta-restricted check). The floored-Count/containment
/// sharing (`lean/Bumbledb/Subsumption.lean: window_floor_containment`) shares
/// the `R` machinery — a capacity edge is written exactly as a containment edge
/// is — but never skips a check: a declared capacity statement is judged
/// whether or not a containment subsumes its floor.
pub(super) fn check_capacities(
    view: &FinalStateView<'_, '_, '_>,
    obligations: &IncrementalObligations<'_, '_>,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let FinalStateView { txn, schema, plan } = view;
    let catalog = LmdbPeekCatalog::new(txn);
    let mut checker = Checker::new(&catalog, schema);
    let mut span = obs::span(obs::names::JUDGMENT_CAPACITIES);
    let mut judged = 0u64;
    for check in obligations.capacity_checks() {
        judged += 1;
        let statement = schema.capacity(check.capacity);
        let checks = plan.selections.capacity(check.capacity);
        collect(
            checker.check_capacity(statement, checks, check.parent.as_bytes()),
            violations,
        )?;
    }
    span.set_count(judged);
    span.end();
    Ok(())
}

pub(crate) fn capacity_child_image<'a>(
    statement: &CapacityStatement,
    layout: &FactLayout,
    fact: &[u8],
    out: &'a mut DeterminantImage,
) -> &'a DeterminantImage {
    let projection = statement
        .enforcement
        .key_projection()
        .unwrap_or(&statement.source.projection);
    keys::determinant_image(layout.encoded(fact), projection, out)
}

fn fresh_row_word(determinant: &[u8]) -> Result<u64> {
    let word: [u8; 8] = determinant
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MalformedValue("fresh-row key width")))?;
    Ok(u64::from_be_bytes(word))
}

fn establishing_fact<'t>(
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    txn: &'t WriteTxn<'_>,
    schema: &Schema,
    key: KeyId,
    determinant: &[u8],
) -> Result<&'t [u8]> {
    let statement = schema.key(key);
    if matches!(statement.form(), KeyForm::FreshRow { .. }) {
        return fact_by_row(
            data,
            txn.raw(),
            schema,
            statement.relation,
            fresh_row_word(determinant)?,
        )
        .map(crate::encoding::FactView::bytes);
    }
    let mut buf: KeyBuf = [0; MAX_KEY];
    let u_key = keys::determinant_key(&mut buf, statement.relation, statement.id, determinant);
    let value =
        data.get(txn.raw(), u_key)?
            .ok_or(Error::Corruption(CorruptionError::MalformedValue(
                "re-established U determinant",
            )))?;
    fact_by_row(
        data,
        txn.raw(),
        schema,
        statement.relation,
        decode_row_id(value)?,
    )
    .map(crate::encoding::FactView::bytes)
}

fn closed_source_survivor(
    schema: &Schema,
    plan: &CommitPlan<'_>,
    containment_id: ContainmentId,
    determinant: &[u8],
) -> Option<Box<[u8]>> {
    let statement = schema.containment(containment_id);
    let source = &statement.source;
    let key_projection = statement.enforcement.key_projection()?;
    let relation = schema.relation(source.relation);
    let layout = relation.layout();
    let phi = &plan.selections.containment(containment_id).source;
    let mut derived = keys::DeterminantImage::scratch_with_capacity(determinant.len());
    for row in relation.body().closed_rows()? {
        if !satisfies(phi, layout, &row.fact) {
            continue;
        }
        keys::determinant_image(layout.encoded(&row.fact), key_projection, &mut derived);
        if derived.as_bytes() == determinant {
            return Some(row.fact.clone());
        }
    }
    None
}

pub(crate) struct Probe<'a> {
    pub(crate) statement: StatementId,
    pub(crate) target_relation: RelationId,

    pub(crate) target_key: KeyId,
    pub(crate) target_check: &'a SelectionCheck,

    pub(crate) key_bytes: &'a [u8],

    pub(crate) fact_bytes: &'a [u8],

    pub(crate) direction: Direction,
}

impl<'a> Probe<'a> {
    pub(crate) fn of(
        statement: &crate::schema::ContainmentStatement,
        target_check: &'a SelectionCheck,
        key_bytes: &'a [u8],
        fact_bytes: &'a [u8],
        direction: Direction,
    ) -> Self {
        Self {
            statement: statement.id,
            target_relation: statement.target.relation,
            target_key: statement
                .enforcement
                .target_key()
                .expect("edged containments resolve a target key"),
            target_check,
            key_bytes,
            fact_bytes,
            direction,
        }
    }

    fn unsatisfied(&self, schema: &Schema) -> Check {
        Check::Violated(Violation::containment(
            schema.cite(self.statement),
            self.direction,
            self.fact_bytes.into(),
        ))
    }
}

pub(crate) struct Checker<'a, C: CatalogRead> {
    catalog: &'a C,
    schema: &'a Schema,
    key: KeyBuf,

    fact_scratch: Vec<u8>,

    parent_scratch: Vec<u8>,
}

impl<'a, C: CatalogRead> Checker<'a, C> {
    pub(crate) fn new(catalog: &'a C, schema: &'a Schema) -> Self {
        Self {
            catalog,
            schema,
            key: [0; MAX_KEY],
            fact_scratch: Vec::new(),
            parent_scratch: Vec::new(),
        }
    }

    fn load_row(&mut self, relation: RelationId, row_id: u64, parent: bool) -> Result<()> {
        let stored = self
            .catalog
            .fetch_fact(relation, row_id)?
            .ok_or(Error::Corruption(CorruptionError::MissingFact {
                relation,
                row_id,
            }))?;
        let bytes = stored.as_ref();
        crate::storage::read::check_width(self.schema, relation, row_id, bytes)?;
        let dst = if parent {
            &mut self.parent_scratch
        } else {
            &mut self.fact_scratch
        };
        dst.clear();
        dst.extend_from_slice(bytes);
        Ok(())
    }

    pub(crate) fn check_scalar(&mut self, probe: &Probe<'_>) -> Result<Check> {
        let target_key = self.schema.key(probe.target_key);
        if target_key.form().as_fresh_row().is_some() {
            let row_id = fresh_row_word(probe.key_bytes)?;
            let f_key = keys::fact_key(probe.target_relation, row_id);
            let Some(fact) = self.catalog.get(CatalogMap::Data, &f_key)? else {
                return Ok(probe.unsatisfied(self.schema));
            };
            let bytes = fact.as_ref();
            crate::storage::read::check_width(self.schema, probe.target_relation, row_id, bytes)?;
            return Ok(self.check_fact(probe, bytes));
        }
        let u_key = keys::determinant_key(
            &mut self.key,
            probe.target_relation,
            target_key.id,
            probe.key_bytes,
        );
        let Some(value) = self.catalog.get(CatalogMap::Data, u_key)? else {
            return Ok(probe.unsatisfied(self.schema));
        };
        // decode_row_id copies the word to the stack before the next

        self.check_segment(probe, value.as_ref())
    }

    fn check_scalar_sorted<G: SortedGets>(
        &mut self,
        probe: &Probe<'_>,
        gets: &mut G,
    ) -> Result<Check> {
        let target_key = self.schema.key(probe.target_key);
        if target_key.form().as_fresh_row().is_some() {
            let f_key = keys::fact_key(probe.target_relation, fresh_row_word(probe.key_bytes)?);
            let Some(fact) = gets.get(&f_key)? else {
                return Ok(probe.unsatisfied(self.schema));
            };
            return Ok(self.check_fact(probe, fact.as_ref()));
        }
        let u_key = keys::determinant_key(
            &mut self.key,
            probe.target_relation,
            target_key.id,
            probe.key_bytes,
        );
        let Some(value) = gets.get(u_key)? else {
            return Ok(probe.unsatisfied(self.schema));
        };
        self.check_segment(probe, value.as_ref())
    }

    /// This site owns what enters the walk — the LMDB seeks that locate

    pub(crate) fn check_coverage(
        &mut self,
        disjoint: DisjointDeterminantProof,
        source_tail: ValueType,
        target_tail: ValueType,
        probe: &Probe<'_>,
    ) -> Result<Check> {
        disjoint.authorize_coverage();
        let target_key = self.schema.key(probe.target_key);

        let full_src_len = keys::determinant_key(
            &mut self.key,
            probe.target_relation,
            target_key.id,
            probe.key_bytes,
        )
        .len();
        let group_len = full_src_len - source_tail.width();
        let seek_len = group_len + 8;
        let (source_start, source_end) =
            interval_words(source_tail, &self.key[group_len..full_src_len])
                .expect("the plan derived these key bytes from a validated fact");

        let full_len = group_len + target_tail.width();

        let located = self.locate_coverage_entry(
            &self.key[..seek_len],
            &self.key[..group_len],
            full_len,
            target_tail,
            source_start,
        )?;
        let segments =
            self.collect_coverage_segments(located, &self.key[..group_len], full_len, target_tail)?;
        sweep(
            segments.into_iter().map(Ok),
            Some((source_start, source_end)),
            &mut GapAt {
                checker: self,
                probe,
            },
        )
        .map_or_else(
            |fail| match fail {
                ProbeFail::Cite => Ok(probe.unsatisfied(self.schema)),
                ProbeFail::Infra(error) => Err(error),
            },
            |()| Ok(Check::Holds),
        )
    }

    /// probe — `lean/Bumbledb/Oracle.lean:

    /// descents: both arms bind the full target fact before the verdict —

    /// manufacture parents (`lean/Bumbledb/Capacity.lean:

    pub(crate) fn check_capacity(
        &mut self,
        statement: &CapacityStatement,
        checks: &SideChecks,
        parent_key: &[u8],
    ) -> Result<Check> {
        let parent_fact: &[u8] = match &statement.enforcement {
            CapacityEnforcement::ScalarProbe { target_key, .. } => {
                let key_statement = self.schema.key(*target_key);

                match key_statement.form() {
                    KeyForm::FreshRow { .. } => {
                        if !self.load_fresh_parent(statement.target.relation, parent_key)? {
                            return Ok(Check::Holds);
                        }
                    }
                    KeyForm::Scalar | KeyForm::Pointwise { .. } => {
                        let u_key = keys::determinant_key(
                            &mut self.key,
                            statement.target.relation,
                            key_statement.id,
                            parent_key,
                        );
                        let Some(value) = self.catalog.get(CatalogMap::Data, u_key)? else {
                            return Ok(Check::Holds);
                        };
                        let row_id = decode_row_id(value.as_ref())?;

                        let needs_fact = !matches!(checks.target, SelectionCheck::Empty)
                            || statement.hi.needs_parent_fact();
                        if !needs_fact {
                            let hi = match statement.hi {
                                SealedBound::Unbounded => BoundCeiling::Unbounded,
                                SealedBound::Lit(n) => BoundCeiling::Finite(n),
                                SealedBound::TargetField(_) | SealedBound::Duration { .. } => {
                                    unreachable!("a dependent bound forces the eager holder fetch")
                                }
                            };
                            let measure =
                                self.measure_children(statement, &checks.source, parent_key, hi)?;
                            if measure < u128::from(statement.lo) || exceeds_ceiling(measure, hi) {
                                self.load_row(statement.target.relation, row_id, true)?;
                                return Ok(self.capacity_violation(
                                    statement,
                                    self.parent_scratch.clone().into(),
                                    measure,
                                ));
                            }
                            return Ok(Check::Holds);
                        }
                        self.load_row(statement.target.relation, row_id, true)?;
                    }
                }
                let layout = self.schema.relation(statement.target.relation).layout();
                if !satisfies(&checks.target, layout, &self.parent_scratch) {
                    return Ok(Check::Holds);
                }

                &self.parent_scratch
            }

            CapacityEnforcement::Closed { members } => {
                let Ok(word) = <[u8; 8]>::try_from(parent_key) else {
                    return Err(Error::Corruption(CorruptionError::MalformedValue(
                        "capacity parent key width",
                    )));
                };
                let id = u64::from_be_bytes(word);
                if !AxiomIndex::try_from(id).is_ok_and(|index| members.contains(index)) {
                    return Ok(Check::Holds);
                }
                let rows = self
                    .schema
                    .relation(statement.target.relation)
                    .body()
                    .closed_rows()
                    .expect("the Closed enforcement arm resolves only against a closed target");
                let index = usize::try_from(id).expect("a contained axiom index fits usize");
                &rows[index].fact
            }
        };

        // C10 refusal naming the parent row.
        let hi = self.resolve_hi(statement, parent_fact)?;
        let measure = self.measure_children(statement, &checks.source, parent_key, hi)?;
        if measure < u128::from(statement.lo) || exceeds_ceiling(measure, hi) {
            return Ok(self.capacity_violation(
                statement,
                self.capacity_payload(statement, parent_key),
                measure,
            ));
        }
        Ok(Check::Holds)
    }

    fn capacity_payload(&self, statement: &CapacityStatement, parent_key: &[u8]) -> Box<[u8]> {
        match &statement.enforcement {
            CapacityEnforcement::ScalarProbe { .. } => self.parent_scratch.clone().into(),
            CapacityEnforcement::Closed { .. } => {
                let word: [u8; 8] = parent_key
                    .try_into()
                    .expect("closed parent key width checked above");
                let id = u64::from_be_bytes(word);
                let rows = self
                    .schema
                    .relation(statement.target.relation)
                    .body()
                    .closed_rows()
                    .expect("Closed arm resolves only against a closed target");
                let index = usize::try_from(id).expect("a contained axiom index fits usize");
                rows[index].fact.clone()
            }
        }
    }

    fn capacity_violation(
        &self,
        statement: &CapacityStatement,
        fact: Box<[u8]>,
        measure: u128,
    ) -> Check {
        Check::Violated(Violation::capacity(
            self.schema.cite(statement.id),
            fact,
            measure,
        ))
    }

    /// holder fact (`lean/Bumbledb/Capacity.lean: CapWindow.resolve`):

    fn resolve_hi(
        &self,
        statement: &CapacityStatement,
        parent_fact: &[u8],
    ) -> Result<BoundCeiling> {
        let layout = self.schema.relation(statement.target.relation).layout();
        resolve_bound(statement.hi, layout, parent_fact, statement.id)
    }

    /// u128 (`lean/Bumbledb/Oracle.lean: capacity_plan_consultations`;

    /// (ruled 2026-07-24, C14: the clip serves the verdict, the full sum

    fn measure_children(
        &mut self,
        statement: &CapacityStatement,
        phi: &SelectionCheck,
        parent_key: &[u8],
        hi: BoundCeiling,
    ) -> Result<u128> {
        let source = self.schema.relation(statement.source.relation);
        let layout = source.layout();
        if let Some(rows) = source.body().closed_rows() {
            let mut derived = DeterminantImage::scratch_with_capacity(parent_key.len());
            let mut measure = 0u128;
            for row in rows {
                if !satisfies(phi, layout, &row.fact) {
                    continue;
                }
                capacity_child_image(statement, layout, &row.fact, &mut derived);
                if derived.as_bytes() == parent_key {
                    measure += u128::from(child_weight(statement, layout, &row.fact)?);
                }
            }
            return Ok(measure);
        }

        let floor_only_decided = |measure: u128| {
            matches!(hi, BoundCeiling::Unbounded) && measure >= u128::from(statement.lo)
        };
        let slot = SlotShape::of(statement.weight);
        let prefix_len = keys::reverse_prefix(&mut self.key, statement.id, parent_key).len();
        let bounds = Bounds {
            start: Bound::Included(&self.key[..prefix_len]),
            end: Bound::Unbounded,
        };
        let mut range = self.catalog.range(CatalogMap::Data, bounds)?;
        let mut measure = 0u128;
        while let Some(entry) = ReadCursor::next(&mut range)? {
            if !entry.key.starts_with(&self.key[..prefix_len]) {
                break;
            }
            measure += u128::from(slot.decode(entry.value)?);
            if floor_only_decided(measure) {
                break;
            }
        }
        Ok(measure)
    }

    fn load_fresh_parent(&mut self, relation: RelationId, determinant: &[u8]) -> Result<bool> {
        let row_id = fresh_row_word(determinant)?;
        let f_key = keys::fact_key(relation, row_id);
        match self.catalog.get(CatalogMap::Data, &f_key)? {
            None => Ok(false),
            Some(bytes) => {
                let bytes = bytes.as_ref();
                crate::storage::read::check_width(self.schema, relation, row_id, bytes)?;
                self.parent_scratch.clear();
                self.parent_scratch.extend_from_slice(bytes);
                Ok(true)
            }
        }
    }

    fn check_segment(&mut self, probe: &Probe<'_>, value: &[u8]) -> Result<Check> {
        if matches!(probe.target_check, SelectionCheck::Empty) {
            return Ok(Check::Holds);
        }
        let row_id = decode_row_id(value)?;
        self.load_row(probe.target_relation, row_id, false)?;
        Ok(self.check_fact(probe, &self.fact_scratch))
    }

    fn check_fact(&self, probe: &Probe<'_>, target_fact: &[u8]) -> Check {
        if matches!(probe.target_check, SelectionCheck::Empty) {
            return Check::Holds;
        }
        let layout = self.schema.relation(probe.target_relation).layout();
        if satisfies(probe.target_check, layout, target_fact) {
            Check::Holds
        } else {
            probe.unsatisfied(self.schema)
        }
    }

    fn locate_coverage_entry(
        &self,
        seek: &[u8],
        group: &[u8],
        full_len: usize,
        target_tail: ValueType,
        source_start: u64,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        if let Some(hit) = self.catalog.greater_or_equal(CatalogMap::Data, seek)?
            && hit.key.starts_with(seek)
        {
            return Ok(Some((hit.key.to_vec(), hit.value.to_vec())));
        }
        match self.catalog.lower(CatalogMap::Data, seek)? {
            Some(pred) if pred.key.starts_with(group) => {
                if pred.key.len() != full_len {
                    return Err(Error::Corruption(CorruptionError::MalformedValue(
                        "U determinant key length",
                    )));
                }
                let (_, pred_end) = interval_words(target_tail, &pred.key[group.len()..]).ok_or(
                    Error::Corruption(CorruptionError::MalformedValue("U determinant tail")),
                )?;

                Ok((pred_end > source_start).then(|| (pred.key.to_vec(), pred.value.to_vec())))
            }
            _ => Ok(None),
        }
    }

    fn collect_coverage_segments(
        &self,
        located: Option<(Vec<u8>, Vec<u8>)>,
        group: &[u8],
        full_len: usize,
        target_tail: ValueType,
    ) -> Result<Vec<(u64, u64, Vec<u8>)>> {
        let Some((entry_key, entry_value)) = located else {
            return Ok(Vec::new());
        };
        let Some((_, _, _)) = (entry_key.len() == full_len)
            .then(|| segment_words(&entry_key, &entry_value, target_tail))
            .flatten()
        else {
            return Err(Error::Corruption(CorruptionError::MalformedValue(
                "U determinant key length",
            )));
        };
        let mut segments = Vec::new();
        let (start, end, _) =
            segment_words(&entry_key, &entry_value, target_tail).expect("length-checked above");
        segments.push((start, end, entry_value));
        let bounds = Bounds {
            start: Bound::Excluded(entry_key.as_slice()),
            end: Bound::Unbounded,
        };
        let mut range = self.catalog.range(CatalogMap::Data, bounds)?;
        while let Some(entry) = ReadCursor::next(&mut range)? {
            if !entry.key.starts_with(group) {
                break;
            }
            if entry.key.len() != full_len {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "U determinant key length",
                )));
            }
            let Some((start, end, _)) = segment_words(entry.key, entry.value, target_tail) else {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "U determinant key length",
                )));
            };

            segments.push((start, end, entry.value.to_vec()));
        }
        Ok(segments)
    }
}

type DeterminantSegment<'t> = (u64, u64, &'t [u8]);

fn segment_words<'t>(
    key: &[u8],
    value: &'t [u8],
    tail: ValueType,
) -> Option<DeterminantSegment<'t>> {
    if key.len() < tail.width() {
        return None;
    }
    let (start, end) = interval_words(tail, &key[key.len() - tail.width()..])?;
    Some((start, end, value))
}

struct GapAt<'c, 'a, 'p, C: CatalogRead> {
    checker: &'c mut Checker<'a, C>,
    probe: &'c Probe<'p>,
}

enum ProbeFail {
    Infra(Error),
    Cite,
}

impl From<Error> for ProbeFail {
    fn from(error: Error) -> Self {
        Self::Infra(error)
    }
}

impl<C: CatalogRead> Continuation<u64, Vec<u8>> for GapAt<'_, '_, '_, C> {
    type Error = ProbeFail;

    fn segment(&mut self, value: Vec<u8>) -> std::result::Result<(), ProbeFail> {
        match self.checker.check_segment(self.probe, &value)? {
            Check::Holds => Ok(()),
            Check::Violated(_) => Err(ProbeFail::Cite),
        }
    }

    fn maximal(&mut self, _: u64, _: u64) -> std::result::Result<(), ProbeFail> {
        Err(ProbeFail::Cite)
    }
}
