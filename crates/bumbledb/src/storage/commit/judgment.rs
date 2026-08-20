//! Phase 3, the containment judgment (`docs/architecture/50-storage.md`
//! § commit step 3; `30-dependencies.md` § enforcement). Source side:
//! every inserted fact satisfying a statement's source selection proves
//! its target tuple exists in the final state — scalar tuples by one
//! determinant probe, interval positions by the coverage walk. Target side:
//! every key tuple disestablished by this commit probes its dependent
//! statements' `R` prefixes for surviving sources — a scalar survivor is
//! the violation outright; an interval survivor re-runs the coverage walk
//! against the final `U` state. LMDB write transactions read their own
//! writes, so both sides see exactly the state the commit would persist.
//!
//! Both sides are **scan-complete**: a violation is recorded into the
//! caller's collector and the scan continues — the reject path runs
//! exactly the checks the accept path runs, and the rejection carries
//! the COMPLETE violation set, sealed sorted and deduplicated
//! ([`crate::error::Violations`]; `30-dependencies.md` § judged on
//! final states). The two sides partition the source facts: an inserted
//! source is judged source-side only — the target scan skips survivors
//! this commit inserted, so one statement is never convicted twice
//! through one fact.
//!
//! Also home of the selection machinery the plan derivation gates its
//! `R`-edges with: literals encode once per commit into [`Selections`]
//! (never per fact), and [`satisfies`] is a straight byte compare of
//! selected field slices.
//!
//! The probe machinery ([`Checker`], [`Probe`]) is deliberately
//! transaction-agnostic: `Db::verify_store` runs the same scalar probe
//! and the same coverage walk over a read snapshot to re-verify the
//! judgments globally — one definition, never a sweeper copy. The
//! coverage walk's frontier loop is itself the shared segment sweep
//! ([`crate::interval::sweep`]) with the checker as its gap-at
//! continuation; this file owns entry-segment location and the key-shape
//! trust checks, nothing of the walk.

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

/// The one state dependency judgment may inspect. Phases 1–2 have
/// already applied the plan to this LMDB write transaction, whose
/// read-your-writes view is therefore exactly `base + delta` in final
/// set semantics; operation order is no longer representable here.
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
/// containments (both directions) and capacity statements (per touched
/// parent) — and seals the complete violation set of the phase
/// (`lean/Bumbledb/Txn.lean: rejection_is_complete`, the statement arm).
pub(super) fn judge(view: &FinalStateView<'_, '_, '_>) -> Result<Admission<()>> {
    let obligations = view.plan.incremental_obligations();
    let mut violations = Vec::new(); // construction-time: one collector per commit
    check_source(view, &obligations, &mut violations)?;
    check_target(view, &obligations, &mut violations)?;
    check_capacities(view, &obligations, &mut violations)?;
    Ok(Violations::seal(view.plan.selections.schema(), violations))
}

// CONSTRAINT (C17, measured 2026-08-01 — the slot arm IS the law): a
// WEIGHTED capacity statement's `R` edges carry the child's u64 weight
// (LE) in the value slot, paid once at write time; the measure walk
// reads the entries it already visits. Unit edges keep the empty value.
// The losing fetch-per-child arm (empty values everywhere, one `F` get
// per walked edge) is DELETED with its `CAPACITY_WEIGHT_SLOT` flag —
// the power-budget lane decided it (2026-08-01, min-of-3 ephemeral p50s,
// µs; identical statement-free
// control 18.2 both arms): commit_capacity_sum fetch 35.2 vs slot 32.3
// (judged surface +17.0 → +14.1, −17%); commit_capacity_duration fetch
// 34.2 vs slot 30.8 (+16.0 → +12.6, −21%); the fsync-shadowed durable
// lane agreed in direction (sum p50 5365.1 vs 5093.4). Readers dispatch
// on the statement's DECLARED weight, never on value length — a width
// disagreeing with the declaration is corruption, not a fallback.
// Semantic corner, RULED C20 (owner, 2026-08-03): a ray-valued Duration
// weight refuses at WRITE time (the slot needs a finite u64), strictly
// stronger than C10's judge-time refusal — visible only for a ray child
// under an absent parent, and DOCTRINE there: a row governed by a
// Duration-weighted capacity law must carry a measurable weight whether
// or not any judgment currently demands it (fail-fast; never latent).
// Pinned: tests/marks.rs
// `capacity_duration_ray_under_an_absent_parent_still_refuses`.

/// One source fact's weight under a capacity statement's measure
/// (`lean/Bumbledb/Capacity.lean: Weight.apply`): `Unit` is 1 — the
/// count instance; `Field` reads the u64-encoded SOURCE position;
/// `DurationOf` reads the SOURCE interval position's measure in encoded
/// word space (`end − start` — both element encodings preserve
/// differences, the R5 machinery). A ray-valued Duration weight is the
/// typed C10 refusal naming the row — a ray has no finite measure
/// (`docs/architecture/30-dependencies.md` § weight typing).
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

/// The ONE weight arithmetic behind [`child_weight`] — spelled over the
/// raw `(weight, sealed tail)` pair so validate's closed-constant arm
/// (which measures extension rows before any [`CapacityStatement`]
/// exists) reads THIS definition too: the measure law has exactly one
/// engine definition, never an inline re-implementation
/// (`docs/architecture/60-validation.md` § one mechanism per law).
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
/// (`lean/Bumbledb/Capacity.lean: CapWindow.resolve`) — spelled over the
/// raw `(bound, sealed tail)` pair so validate's closed-constant arm
/// reads THIS definition too, exactly as [`measure_weight`]: a literal
/// passes through; a dependent bound reads the named TARGET-row field —
/// u64 word or interval measure — off the holder fact in hand.
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

/// The expected `R` value-slot payload for one capacity edge of one
/// source fact — the ONE definition the applier writes and both
/// weight-desync sweep directions compare against
/// (`docs/architecture/60-validation.md` § the weight-desync sweep).
/// `None` = the empty value: unit edges carry no weight.
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

/// The C17 `R` value-slot shape, resolved once per statement from
/// [`SealedWeight`] — not re-tested per walked entry.
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

/// The interval measure in encoded word space: `end − start` over the
/// order-preserving column words (I64 endpoints are the sign-flipped
/// biased words, so differences are value differences). A ray
/// (`end == u64::MAX` in both element encodings) has no finite measure —
/// the typed commit refusal naming the row (ruled 2026-07-24, C10; the
/// R6 precedent, `crate::error::Error::MeasureOfRay`'s judge-side twin).
/// An INVERTED general tail (`end < start`) is unrepresentable by
/// construction — the value codec only encodes nonempty intervals — so
/// stored bytes reading back inverted convict corruption, never wrap
/// into a garbage measure (`checked_sub`, not `-`: the subtraction was
/// the one arithmetic in the module that could underflow on hostile
/// bytes).
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
/// (membership among the sealed encodings —
/// `lean/Bumbledb/Schema.lean: Selection.satisfies`, the field's value a
/// MEMBER of the spelled set).
pub(crate) enum FieldCheck {
    /// One literal: a single byte compare.
    One(Box<[u8]>),
    /// A literal set: any-of over the alternatives (canonical order;
    /// never-interned `str` alternatives already dropped).
    AnyOf(Box<[Box<[u8]>]>),
}

impl FieldCheck {
    /// Whether the field's slice satisfies this binding.
    fn matches(&self, actual: &[u8]) -> bool {
        match self {
            Self::One(literal) => actual == &literal[..],
            Self::AnyOf(alternatives) => alternatives.iter().any(|bytes| actual == &bytes[..]),
        }
    }
}

/// One side's selection σ, its literals pre-encoded for byte comparison.
pub(crate) enum SelectionCheck {
    /// σ is empty: every fact satisfies.
    Empty,
    /// Per selected field, the binding's pre-encoded comparison.
    Compare(Box<[(FieldId, FieldCheck)]>),
    /// A binding is unsatisfiable (a String literal — or every literal of
    /// a set binding — was never interned): no stored fact can carry its
    /// id, so no fact satisfies σ.
    Never,
}

/// Both selections of one containment statement.
pub(crate) struct SideChecks {
    pub(crate) source: SelectionCheck,
    pub(crate) target: SelectionCheck,
}

/// Pre-encoded selections for every `Containment` and `Capacity`
/// statement, built once per commit — the commit-local scratch that keeps
/// literal encoding out of the per-fact loops. Carries the schema it was
/// encoded from so `plan_commit` and the later phases derive from one
/// witness, not a matching triple of arguments.
pub(crate) struct Selections<'s> {
    schema: &'s Schema,
    /// Dense by [`ContainmentId`]; every slot is a containment by type.
    checks: Box<[SideChecks]>,
    /// Dense by [`CapacityId`]; every slot is a capacity statement by type.
    capacities: Box<[SideChecks]>,
}

impl<'s> Selections<'s> {
    /// Materializes every containment statement's selection checks from
    /// the sealed compile ([`CompiledCheck`], the staging law): canonical
    /// bytes copy as-is; only `str` literals resolve — through the
    /// delta's pending map, then the committed dictionary — and a double
    /// miss proves no fact can satisfy the selection
    /// ([`SelectionCheck::Never`]).
    pub(crate) fn encode(delta: &'s WriteDelta<'_>, view: &ReadTxn<'_>) -> Result<Self> {
        Self::encode_with(delta.schema(), &mut |raw| delta.resolve(view, raw))
    }

    /// The read-only sibling of [`Selections::encode`] for
    /// `Db::verify_store`: no delta exists, so String literals resolve
    /// through the committed dictionary alone — a miss proves no
    /// *committed* fact can satisfy the selection, exactly the judgment
    /// the sweeper re-checks.
    pub(crate) fn encode_committed(schema: &'s Schema, view: &ReadTxn<'_>) -> Result<Self> {
        Self::encode_with(schema, &mut |raw| crate::storage::dict::lookup(view, raw))
    }

    /// Encodes selections through a caller-supplied intern lookup — the
    /// heap-admit path has a [`crate::storage::catalog::CatalogRead`],
    /// not a [`WriteDelta`].
    pub(crate) fn encode_lookup(
        schema: &'s Schema,
        mut lookup: impl FnMut(&[u8]) -> Result<Option<InternId>>,
    ) -> Result<Self> {
        Self::encode_with(schema, &mut lookup)
    }

    /// Complete-judgment scratch resident with this table — $J$ stand-in.
    #[must_use]
    pub(crate) fn scratch_bytes(&self) -> usize {
        self.checks.len() * std::mem::size_of::<SideChecks>()
            + self.capacities.len() * std::mem::size_of::<SideChecks>()
    }

    /// The shared constructor. Three call sites monomorphize here
    /// (`encode`, `encode_committed`, `encode_lookup`) — a two-arm
    /// enum would still leave the heap-admit lookup as a third
    /// source, so `F: FnMut` is the form that deletes the trait object.
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

    /// The schema these selections were encoded from.
    pub(crate) fn schema(&self) -> &'s Schema {
        self.schema
    }

    /// Rebind to the schema borrow the plan lives on — same pointer,
    /// shorter lifetime, so `plan_commit` stores one `'d`.
    pub(crate) fn bind(self, schema: &Schema) -> Selections<'_> {
        debug_assert!(std::ptr::eq(self.schema, schema));
        Selections {
            schema,
            checks: self.checks,
            capacities: self.capacities,
        }
    }

    /// The checks of a validation-minted containment witness.
    pub(crate) fn containment(&self, id: ContainmentId) -> &SideChecks {
        &self.checks[usize::from(id.0)]
    }

    /// The checks of a validation-minted capacity witness.
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
                    (*field, FieldCheck::One(bytes.clone())) // construction-time: Selections::encode
                }
                EncodableCheck::EncodedSet {
                    field,
                    alternatives,
                } => (*field, FieldCheck::AnyOf(alternatives.clone())), // construction-time: Selections::encode
            })
            .collect(),
    )
}

/// One side's sealed checks into the commit-local form: `Encoded` bytes
/// copy verbatim (encoded once, at validate — never here); `Interned`
/// text resolves through the boundary's dictionary view. A set binding's
/// never-interned `str` alternatives drop out of the disjunction (each is
/// individually unsatisfiable); a binding with nothing left is `Never`.
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
            CompiledCheck::Encoded { field, bytes } => {
                (*field, FieldCheck::One(bytes.clone())) // construction-time: Selections::encode
            }
            CompiledCheck::EncodedSet {
                field,
                alternatives,
            } => (*field, FieldCheck::AnyOf(alternatives.clone())), // construction-time: Selections::encode
            CompiledCheck::Interned { field, text } => match resolve(text.as_bytes())? {
                Some(id) => (*field, FieldCheck::One(Box::new(encode_u64(id.raw())))), // construction-time: Selections::encode
                None => return Ok(SelectionCheck::Never),
            },
            CompiledCheck::InternedSet { field, texts } => {
                let mut alternatives = Vec::with_capacity(texts.len());
                for text in texts {
                    if let Some(id) = resolve(text.as_bytes())? {
                        alternatives.push(Box::new(encode_u64(id.raw())) as Box<[u8]>); // construction-time: Selections::encode
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

/// Whether a fact satisfies a pre-encoded selection: per selected field,
/// one byte compare (singleton) or membership among the sealed
/// alternatives (set), slices out of `fact_bytes` (interval fields compare
/// their whole 16 bytes — `field_bytes` widths come from the layout).
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

/// Folds one probe's outcome into the collector: a judged violation is
/// recorded and the scan continues (the reject path is scan-complete);
/// every other error — corruption, storage — propagates and aborts the
/// judgment outright.
pub(crate) fn collect(outcome: Result<Check>, violations: &mut Vec<Violation>) -> Result<()> {
    match outcome? {
        Check::Holds => Ok(()),
        Check::Violated(violation) => {
            violations.push(violation);
            Ok(())
        }
    }
}

/// The source-side judgment: for each insert op's edges — exactly the
/// facts this commit added that satisfy a containment's source selection,
/// by the plan derivation over the net-disposition delta, so a redundant
/// or out-of-σ insert is never judged here — prove the target tuple
/// present (scalar) or covered (interval) in the final state. The probe
/// list and its pre-permuted target key bytes come whole from the plan;
/// only the probe *results* are read here. Closed-target containments
/// probe nothing: the compiled member set answers in one AND and one
/// test, and an out-of-range word is simply a miss
/// (`docs/architecture/30-dependencies.md`). Violations accumulate into
/// `violations`; the caller seals the complete set. Probes run
/// key-sorted, not in delta order — the measured license and the
/// witness consequence are at the sort below.
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
    // Probes run in target-key order, not the delta's hash order: the T8
    // commit-size sweep (`bumbledb-bench sweep-commit`) measured
    // hash-order probes paying 1.20–1.33x over key-sorted from ~256
    // touched parents up (0.75x span at 4096, neutral below 64) —
    // ascending keys share B-tree upper pages across descents. One exact
    // allocation per commit; the (containment, key, fact) triple is a
    // total order (facts are set-distinct per relation), so the probe
    // order — and the surviving witness, now the KEY-LEAST violator per
    // citation (`tests/witness_stability.rs`, the licensed flip) — is
    // deterministic whatever the sort algorithm does with equal keys.
    let mut worklist: Vec<(ContainmentId, &RKeyOp<MarkWeight>, &[u8])> =
        obligations.source_edges().collect();
    worklist.sort_unstable_by(|(a, a_edge, a_fact), (b, b_edge, b_fact)| {
        (*a, &a_edge.key_bytes, *a_fact).cmp(&(*b, &b_edge.key_bytes, *b_fact))
    });
    // The sort's completion (T8): within one containment group the
    // derived probe keys ascend (one fixed prefix, ascending key bytes),
    // so ONE forward cursor answers the group's scalar probes — a
    // bounded leaf walk between neighbors instead of an independent
    // root-to-leaf descent per probe ([`SortedGets`]). Reset at each
    // group boundary: across containments the derived keys may go
    // backward, and the walker's verdicts are sound only over ascending
    // probes.
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
        // construction-time: plan-owned Check; the collector takes one copy
        // (closed-target memberships, not the per-fact edge loop).
        collect(Ok(membership.check.clone()), violations)?;
    }
    span.set_count(probes);
    span.end();
    Ok(())
}

/// A surviving interval source, parsed where the `R` key entered the
/// coverage scan. Deduped by identity; the walk payload is minted with
/// the element so the walk does not rematch or re-validate.
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

/// The target-side judgment: every key tuple the plan's check set names —
/// deleted in phase 1 and not re-established in phase 2 — probes its
/// dependent containment statements' `R` prefixes for surviving sources.
/// Re-establishment is **per statement, ψ-qualified**
/// (`docs/architecture/50-storage.md` § commit step 3), split across the
/// plan and this phase along the honest boundary: the plan already
/// dropped empty-ψ re-established tuples (the plain set difference) and
/// *marked* the ψ-carrying dependents of a re-landed tuple, because only
/// this phase can read the establishing fact — one `F` get per
/// re-established tuple, shared across that tuple's ψ-carrying
/// dependents; a ψ hit skips the check. A scalar survivor convicts
/// outright: the key statement's determinant was the tuple's one holder and the
/// final state no longer has it. An interval tuple is a disestablished
/// *segment* `(prefix, ts, te)`: each surviving source of the prefix
/// group whose interval intersects the segment re-runs the coverage walk
/// against the final `U` state — a delete whose hole a same-delta insert
/// covers is legal, and only a failed walk convicts.
///
/// Ported subtlety: a source deleted this commit cannot have a surviving
/// `R` entry, because phase 1 removed its outgoing edges — so a survivor
/// is always live in the final state, no disposition re-check is needed,
/// and its `F` row must exist (a miss is corruption, never a race).
///
/// A survivor *inserted this commit* is skipped: the sides partition the
/// final state's sources — inserted facts are the source side's work
/// (their own probes judge the same missing tuple), and the target side
/// convicts through pre-existing survivors only, so the complete set
/// cites each statement once per genuinely violated direction.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the target-side judgment, one phase per block
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
    // The survivor partition's membership test — sources inserted this
    // commit, by canonical bytes (identity = bytes, `10-data-model.md`)
    // — is the plan's byte-sorted insert index (`CommitPlan::
    // inserts_fact`): no per-judgment set is built.
    //
    // Affected sources of interval statements, deduped before any walk:
    // the element is the full surviving `R` key — statement ‖ prefix
    // group ‖ source interval ‖ source identity — so several
    // disestablished segments of one (statement, prefix-group) collapse
    // to one coverage walk per source. The key bytes stay borrowed from
    // the transaction (the judgment writes nothing, so the pages hold).
    let mut affected: BTreeSet<AffectedSource<'_>> = BTreeSet::new();
    for check in obligations.target_checks() {
        let determinant = check.determinant.as_bytes();
        let key_statement = schema.key(check.key);
        // The establishing fact of a re-landed determinant, fetched at most
        // once per tuple and shared by every ψ-carrying dependent.
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
                    // Interval form: conservatively scan the whole prefix
                    // group and filter by intersection. An optimized lower
                    // bound would need the maximum source-interval length,
                    // which we refuse to track — the group is small and this
                    // is the delete path. The disestablished tuple reads at
                    // the TARGET key's tail; each surviving edge's key bytes
                    // read at the SOURCE projection's tail — the two ends of
                    // one seam, each derived from its own field's type.
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
                    // Domain quantification: a constant source writes no `R`
                    // edges — the surviving sources ARE the sealed
                    // extension's φ-rows.
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
                (Survivors::SealedRows, Enforcement::Closed { .. }) => {
                    // Closed-to-closed is validation-discharged; closed
                    // source against a closed target never reaches the
                    // incremental target scan (no dependents on a closed
                    // auto-key).
                }
                (Survivors::SealedRows, Enforcement::IntervalCoverage { .. }) => {
                    unreachable!("closed sources refuse interval containments at validate")
                }
            }
        }
    }
    // The deduped walks, each against the final `U` state. The element
    // was parsed where the `R` key entered — nothing left to re-validate.
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
            // The survivor partition again: an inserted source's
            // coverage demand is the source side's probe, not a
            // target-side conviction.
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

/// The capacity judgment (`docs/architecture/30-dependencies.md`
/// § enforcement): every TOUCHED parent key tuple — every tuple any delta
/// child fact projects to, plus the delta's ψ-selected parents themselves
/// (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`) — resolves
/// its ψ-selected holder in the final state, resolves any dependent bound
/// from the holder's own row, and measures its child group against the
/// window (`lean/Bumbledb/Oracle.lean: capacity_plan_decides` — the
/// walk's measure verdict IS the delta-restricted check). A floor or
/// ceiling miss records into the collector, scan-complete like the
/// containment sides. The floored-Count/containment sharing
/// (`lean/Bumbledb/Subsumption.lean: window_floor_containment`) shares
/// the `R` machinery — a capacity edge is written exactly as a
/// containment edge is — but never skips a check: a declared capacity
/// statement is judged whether or not a containment subsumes its floor.
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

/// Lays one child fact's parent-tuple bytes down in the capacity
/// statement's target-key determinant order — the `R` key-bytes segment
/// of a capacity edge, the child-group walk's prefix, and the source half
/// of the touched-parent set (`docs/architecture/50-storage.md` § key
/// layout). Group keying is weight-blind.
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

/// The R16 8-byte-determinant decode, spelled ONCE: a fresh-keyed
/// statement maintains no `U` tree — its determinant IS the `F` row id,
/// so every probe of such a key starts by reading the determinant as
/// the one 8-byte word (a different width is corrupt key material).
/// The three probe sites ([`establishing_fact`], [`Checker::check_scalar`],
/// [`Checker::check_capacity`]) differ only in their MISS verdicts.
fn fresh_row_word(determinant: &[u8]) -> Result<u64> {
    let word: [u8; 8] = determinant
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MalformedValue("fresh-row key width")))?;
    Ok(u64::from_be_bytes(word))
}

/// The fact that re-established a key determinant in phase 2, reached through
/// the determinant's own `U` entry — the ψ-qualification subject. Both gets
/// hit state this commit just wrote (write txns read their own writes),
/// so a miss is corruption, never a race. A fresh-row key's determinant
/// IS the `F` row id (R16, no `U` tree): one `F` get instead of two.
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

/// The first sealed source axiom inside φ projecting to the
/// disestablished determinant tuple — the domain-quantification survivor scan
/// (its `R`-probe sibling above walks stored edges; the constant source's
/// edges were never stored). Returns the axiom's canonical fact bytes —
/// the violation payload.
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
            return Some(row.fact.clone()); // cold-arm: violation payload
        }
    }
    None
}

/// One (source fact, containment statement) judgment pair: everything a
/// target probe needs, borrowed from the driving loop. Both commit-time
/// sides build these — the source side for each inserted fact inside σ,
/// the target side for each surviving source whose required tuple a
/// delete touched — and `Db::verify_store` builds one per committed
/// source fact inside σ, re-running the same judgment globally.
pub(crate) struct Probe<'a> {
    pub(crate) statement: StatementId,
    pub(crate) target_relation: RelationId,
    /// The `Functionality` statement whose `U` determinant is probed.
    pub(crate) target_key: KeyId,
    pub(crate) target_check: &'a SelectionCheck,
    /// The source fact's projection, already in target determinant order.
    pub(crate) key_bytes: &'a [u8],
    /// The source fact — the violation payload.
    pub(crate) fact_bytes: &'a [u8],
    /// Which side's judgment a miss convicts.
    pub(crate) direction: Direction,
}

impl<'a> Probe<'a> {
    /// One probe minted from the sealed statement — probe kind resolved
    /// at validation (`Enforcement::target_key`).
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

    /// The convicting check — one probe, one violation. The judgment
    /// speaks about sources, so the payload is the source fact: the
    /// inserted fact whose target is missing, or the survivor whose
    /// required target was disestablished.
    fn unsatisfied(&self, schema: &Schema) -> Check {
        Check::Violated(Violation::containment(
            schema.cite(self.statement),
            self.direction,
            self.fact_bytes.into(),
        ))
    }
}

/// Working state threaded through the judgment probes. The scalar probe
/// and the coverage walk have exactly this one implementation, consumed
/// by three callers: the commit path's two sides (over the write
/// transaction's own-writes view) and `Db::verify_store`'s global
/// re-verification (over a read snapshot) — never a copy.
pub(crate) struct Checker<'a, C: CatalogRead> {
    catalog: &'a C,
    schema: &'a Schema,
    key: KeyBuf,
    /// Retained `F` bytes for a probe that does not hold the fact
    /// across the next catalog get. Capacity is the commit high-water.
    fact_scratch: Vec<u8>,
    /// Retained holder-fact bytes that must survive `measure_children`.
    parent_scratch: Vec<u8>,
}

impl<'a, C: CatalogRead> Checker<'a, C> {
    pub(crate) fn new(catalog: &'a C, schema: &'a Schema) -> Self {
        Self {
            catalog,
            schema,
            key: [0; MAX_KEY],
            fact_scratch: Vec::new(),   // construction-time: Checker retain
            parent_scratch: Vec::new(), // construction-time: Checker retain
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

    /// Scalar target probe: one `U` get on the target key's determinant. A miss
    /// is the violation; a hit with a nonempty target selection
    /// additionally checks the found fact against σ (one `F` get). A
    /// fresh-row target key has no `U` tree (the one id allocator, R16):
    /// its determinant IS the `F` row id, so the probe is one `F` get and
    /// the value found is the target fact itself — one descent, σ check
    /// included.
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
        // get; no host copy of the U value.
        self.check_segment(probe, value.as_ref())
    }

    /// [`Checker::check_scalar`]'s sorted-worklist twin (the T8
    /// completion): the same key derivations and the same verdicts, the
    /// one get answered by the caller's [`SortedGets`] walker — legal
    /// only where probe keys ascend within the walker's current group
    /// ([`check_source`]'s sort provides exactly that).
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

    /// The coverage walk (`docs/architecture/30-dependencies.md`
    /// § pointwise lifting): the source interval `[s, e)` must be jointly
    /// covered by the target's determinant entries sharing its scalar prefix.
    /// Sound in one forward pass because `disjoint` was minted when the
    /// target's pointwise key was accepted, proving the prefix group's
    /// intervals are disjoint and start-ordered. All
    /// comparisons are on the 8-byte encoded halves — order-preserving,
    /// so byte compare is numeric compare. Rays by definition, not by
    /// accident (the point-domain law, `docs/architecture/10-data-model.md`):
    /// a ray's end (`MAX` = ∞) is just the largest end word, so a source
    /// ray demands coverage to ∞ — satisfiable only by a chain reaching a
    /// target ray — and the same gap check enforces it with no special
    /// case.
    ///
    /// This site owns what enters the walk — the LMDB seeks that locate
    /// the entry segment and the key-shape corruption checks (trust
    /// boundaries stay where the data enters); the frontier walk itself
    /// is the shared segment sweep ([`crate::interval::sweep`]), driven
    /// through [`GapAt`].
    pub(crate) fn check_coverage(
        &mut self,
        disjoint: DisjointDeterminantProof,
        source_tail: ValueType,
        target_tail: ValueType,
        probe: &Probe<'_>,
    ) -> Result<Check> {
        disjoint.authorize_coverage();
        let target_key = self.schema.key(probe.target_key);
        // The two tails of one seam, each derived from its own field's
        // type: the probe's key bytes end in the SOURCE interval at the
        // source field's encoding; the stored determinant entries end in
        // the TARGET's (the acceptance gate puts both intervals last).
        // The widths MAY DIFFER — Q1's element-domain typing at interval
        // positions admits a fixed-width side against a general (or
        // other-width) side of one element — and the walk is width-blind
        // by construction, because both tails parse to order-preserving
        // words (`docs/architecture/30-dependencies.md` § Q1).
        // The scratch holds the source-shaped determinant key
        // `U | rel | stmt | prefix | source-tail`. Only slices of it are
        // used: the group prefix and the seek key `group ‖ s` (both
        // encodings LEAD with the start half, so the seek prefix is the
        // same 8 bytes whatever the source tail's width).
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
        // Every stored key of the group has exactly this length.
        let full_len = group_len + target_tail.width();

        // Entry location: the one determinant entry that can cover `s`. A
        // segment starting exactly at `s` has full key `seek ‖ its end`
        // (or IS the seek, fixed target), so the ≥ probe lands on it
        // first when it exists; otherwise the group's predecessor — the
        // segment with the largest start below `s` — may still be
        // running at `s`. A predecessor that has ended (`end ≤ s`)
        // proves nothing covers `s` (the group is disjoint and
        // start-ordered), so there is no entry segment and the sweep
        // gaps at `s` over an empty walk.
        // Seek/group are already in `self.key` — a Vec copy was the
        // per-walk tax. Catalog GAT copies below stay (the sweep owns
        // segment values past the next get).
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

    /// One touched parent's capacity judgment: resolve the ψ-selected
    /// holder of the parent tuple in this checker's state (one keyed `U`
    /// probe — `lean/Bumbledb/Oracle.lean:
    /// accepted_target_key_prices_the_probe` is the unit price's license
    /// — or the compiled member set for a closed parent), resolve any
    /// dependent bound from the holder fact already in hand (ZERO extra
    /// descents: both arms bind the full target fact before the verdict —
    /// the ψ check and the violation's fact bytes require it anyway),
    /// then measure its child group and compare against the resolved
    /// window. No holder, nothing to judge — capacity statements never
    /// manufacture parents (`lean/Bumbledb/Capacity.lean:
    /// capacity_of_empty_parent`).
    ///
    /// Shared verbatim by the commit path (over the write transaction's
    /// own-writes view) and `Db::verify_store` (over a read snapshot) —
    /// one definition, never a sweeper copy.
    pub(crate) fn check_capacity(
        &mut self,
        statement: &CapacityStatement,
        checks: &SideChecks,
        parent_key: &[u8],
    ) -> Result<Check> {
        let parent_fact: &[u8] = match &statement.enforcement {
            CapacityEnforcement::ScalarProbe { target_key, .. } => {
                let key_statement = self.schema.key(*target_key);
                // A fresh-row parent key has no `U` tree (R16): the
                // parent tuple IS the `F` row id, one get, the value the
                // holder itself — nothing to defer.
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
                        // The holder's fact bytes are needed eagerly only by
                        // ψ and by a dependent bound. An empty-ψ
                        // literal-bound statement judges its whole hot path
                        // from the `U` probe and the `R` walk alone — its
                        // `F` get defers to the conviction (cold) path,
                        // where the violation payload requires it anyway.
                        let needs_fact = !matches!(checks.target, SelectionCheck::Empty)
                            || statement.hi.needs_parent_fact();
                        if !needs_fact {
                            // ψ is Empty by construction, so the determinant
                            // hit alone proves the holder; the bound is
                            // literal or unbounded, resolved fact-free.
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
                                    self.parent_scratch.clone().into(), // cold-arm: conviction
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
                // Held across measure_children — that walk uses
                // `self.key` / catalog, never `parent_scratch`.
                &self.parent_scratch
            }
            // A closed parent: the member set IS the ψ-selected roster,
            // and the parent tuple is the axiom's 8-byte id encoding.
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
        // Dependent bounds resolve from the holder fact already in hand
        // (C1: by name against TARGET's whole roster; C6: hi slot only) —
        // zero extra descents. A ray-valued Duration bound is the typed
        // C10 refusal naming the parent row.
        let hi = self.resolve_hi(statement, parent_fact)?;
        let measure = self.measure_children(statement, &checks.source, parent_key, hi)?;
        if measure < u128::from(statement.lo) || exceeds_ceiling(measure, hi) {
            // Conviction is the cold arm: one payload copy.
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
            CapacityEnforcement::ScalarProbe { .. } => self.parent_scratch.clone().into(), // cold-arm: conviction
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
                rows[index].fact.clone() // cold-arm: conviction
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

    /// The capacity ceiling, resolved per touched parent against the
    /// holder fact (`lean/Bumbledb/Capacity.lean: CapWindow.resolve`):
    /// a literal passes through; a dependent bound reads the named
    /// TARGET-row field — u64 word or interval measure — off the fact
    /// bytes already fetched for the ψ check. Delegates to
    /// [`resolve_bound`], the one engine definition.
    fn resolve_hi(
        &self,
        statement: &CapacityStatement,
        parent_fact: &[u8],
    ) -> Result<BoundCeiling> {
        let layout = self.schema.relation(statement.target.relation).layout();
        resolve_bound(statement.hi, layout, parent_fact, statement.id)
    }

    /// One parent's child-group measure: the ordered walk of the capacity
    /// statement's `R` bucket at the parent tuple, summing weights in
    /// u128 (`lean/Bumbledb/Oracle.lean: capacity_plan_consultations`;
    /// overflow is unrepresentable — 2^64 max weight × any real group
    /// stays far under 2^128, so no checked arithmetic). The clip and the
    /// witness, per C12/C14: non-negative weights make the running sum
    /// monotone, so a FLOOR-only walk exits the moment `sum ≥ lo` (the
    /// verdict is final and no witness is owed — satisfaction records
    /// nothing); a CEILING walk always completes — deciding `sum ≤ hi`
    /// needs the whole group anyway, and on conviction the full sum IS
    /// the witness, so the reported measure is walk-order-independent
    /// (ruled 2026-07-24, C14: the clip serves the verdict, the full sum
    /// serves the witness). Weights read from the `R` value slot the
    /// applier maintains (C17, measured — the slot law at the constraint
    /// comment above). A CLOSED source stored no edges: the φ-selected
    /// axioms sharing the tuple are summed by an honest ≤256-row
    /// extension scan (domain quantification,
    /// `docs/architecture/30-dependencies.md`).
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
        // Floor-only clip: with no ceiling, `sum ≥ lo` is final — the
        // running sum is monotone under non-negative weights (C12).
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

    /// The R16 fresh-row probe: the determinant decodes to the `F` row
    /// id ([`fresh_row_word`]) and one `F` get answers — the found fact,
    /// or the miss whose verdict belongs to the caller (a violation at
    /// the scalar probe, no-holder at the capacity check).
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

    /// The per-segment target-selection check: with an empty σ the determinant
    /// hit alone is the proof; otherwise the found target fact is fetched
    /// (one `F` get via the determinant's row id) and byte-checked against σ.
    fn check_segment(&mut self, probe: &Probe<'_>, value: &[u8]) -> Result<Check> {
        if matches!(probe.target_check, SelectionCheck::Empty) {
            return Ok(Check::Holds);
        }
        let row_id = decode_row_id(value)?;
        self.load_row(probe.target_relation, row_id, false)?;
        Ok(self.check_fact(probe, &self.fact_scratch))
    }

    /// The σ half of a probe over a target fact already in hand — the
    /// fresh-row probe's tail (the `F` value IS the fact) and
    /// [`Checker::check_segment`]'s.
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
            // RULED: catalog GAT ends at the next get; the walk owns the entry.
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
                // RULED: catalog GAT ends at the next get; the walk owns the pred.
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
            return Ok(Vec::new()); // empty walk: no segments, no alloc of note
        };
        let Some((_, _, _)) = (entry_key.len() == full_len)
            .then(|| segment_words(&entry_key, &entry_value, target_tail))
            .flatten()
        else {
            return Err(Error::Corruption(CorruptionError::MalformedValue(
                "U determinant key length",
            )));
        };
        let mut segments = Vec::new(); // RULED: one walk Vec; GAT values must outlive the next get
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
            // RULED: Continuation<_,_,Vec<u8>> takes owned values; catalog lending ends.
            segments.push((start, end, entry.value.to_vec()));
        }
        Ok(segments)
    }
}

/// One sweep segment out of the determinant adapter: the
/// order-preserving `(start, end)` words off the key's tail, plus the
/// determinant value (the σ payload — a row id for the
/// target-selection re-check).
type DeterminantSegment<'t> = (u64, u64, &'t [u8]);

/// Parses a determinant key into the sweep's word pair through the
/// key's interval [`ValueType`] (the acceptance gate puts the interval last):
/// the general tail splits its 16 bytes; a fixed tail derives the end
/// from the type's width. `None` on a key too short to carry the tail
/// or a fixed start past the Q2 bound — the callers' key-shape
/// corruption path consumes it alongside their length check.
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

/// The checker's continuation shape: gap-at. Any maximal run short of
/// the source window convicts the probe's side, and every consumed
/// segment re-runs the target-selection check (one `F` get when σ is
/// nonempty). `Pack`'s emit-maximal sibling drives the same sweep from
/// its own call site (`docs/architecture/20-query-ir.md`).
struct GapAt<'c, 'a, 'p, C: CatalogRead> {
    checker: &'c mut Checker<'a, C>,
    probe: &'c Probe<'p>,
}

/// Coverage-walk failure: infrastructure vs a cited gap. Never an
/// [`Error`] used as a semantic verdict.
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
