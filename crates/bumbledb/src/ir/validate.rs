//! The single validation boundary: IR in, [`ValidatedQuery`]
//! witness out. Everything downstream trusts the witness and re-checks
//! nothing (post-mortem §38: v5 validated one plan four times).
//! The roster, transcribed from and
//! checked off in code order below — it is exhaustive by contract.
//! The query shape first (rules are validated one at a time; every
//! across rules after each rule's own fixpoint):

use std::collections::{BTreeMap, BTreeSet};

use crate::allen::AllenMask;
use crate::error::{AtomIndex, ValidationError};
use crate::image::view::MaskConst;
use crate::ir::normalize::{LoweredRule, OccId};
use crate::ir::{FindTerm, InteriorId, ParamId, Value, VarId};
use bumbledb_theory::schema::{FieldId, IntervalElement, ValueType};

mod context;
mod finds;
#[expect(
    clippy::module_inception,
    reason = "the nested module owns the operation named by its parent"
)]
mod validate;

pub use validate::validate;

/// The signature a query defines — anonymous (names live in the host,
/// exactly like relations pre-`as`), its typed output signature derived
/// ONCE at validation and sealed. The single authority for sink
/// construction, result-buffer typing, finalize's all-words decision,
/// and introspection's header. Referenced only by [`InteriorId`], from
/// inside the same [`crate::ir::Query`] — the named-view refusal stands
/// (no stored, named, or cross-query reference exists), and the one
/// reference form is the `Interior` atom, typed against these sealed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub columns: Box<[SignatureColumn]>,
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        for (index, column) in self.columns.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            if let Some(op) = column.op() {
                write!(f, "{op} ")?;
            }
            match column.ty() {
                ValueType::Bool => f.write_str("bool")?,
                ValueType::U64 => f.write_str("u64")?,
                ValueType::I64 => f.write_str("i64")?,
                ValueType::String => f.write_str("string")?,
                ValueType::FixedBytes { len } => write!(f, "bytes<{len}>")?,
                ValueType::Interval { element } => {
                    let element = match element {
                        IntervalElement::U64 => "u64",
                        IntervalElement::I64 => "i64",
                    };
                    write!(f, "interval<{element}>")?;
                }
                ValueType::FixedInterval { element, width } => {
                    let element = match element {
                        IntervalElement::U64 => "u64",
                        IntervalElement::I64 => "i64",
                    };
                    write!(f, "interval<{element}, {width}>")?;
                }
            }
        }
        f.write_str(")")
    }
}

/// One column of the sealed signature: a projection, or a fold. The
/// two are a sum — `op: Option` would re-admit a fold without a type
/// or a projection carrying a fold kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureColumn {
    Project { ty: ValueType },

    Fold { op: AggKind, ty: ValueType },
}

impl SignatureColumn {
    #[must_use]
    pub fn ty(&self) -> &ValueType {
        match self {
            Self::Project { ty } | Self::Fold { ty, .. } => ty,
        }
    }

    #[must_use]
    pub fn op(&self) -> Option<AggKind> {
        match self {
            Self::Project { .. } => None,
            Self::Fold { op, .. } => Some(*op),
        }
    }
}

/// The fold producing a signature column, by kind alone: an Arg key is a
/// rule-scoped variable outside the signature's vocabulary, so the head
/// owns the payload-free kind (a projected measure is a plain column —
/// `None` — while a folded measure carries its fold's kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggKind {
    Sum,

    Min,

    Max,

    Count,

    Pack,
}

impl std::fmt::Display for AggKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Sum => "Sum",
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Count => "Count",
            Self::Pack => "Pack",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClassifiedComparison {
    VarVar {
        op: crate::ir::WordCmp,
        lhs: VarId,
        rhs: VarId,
    },

    VarConst {
        op: crate::ir::WordCmp,
        var: VarId,
        value: SealedConst,
    },

    VarInSet {
        var: VarId,
        set: ParamId,
    },

    AllenVarVar {
        lhs: VarId,
        rhs: VarId,
        mask: AllenMask,
    },

    AllenVarConst {
        var: VarId,
        other: SealedConst,
        mask: MaskConst,
    },

    PointInVarVar {
        interval: VarId,
        point: VarId,
    },

    PointInVarPoint {
        interval: VarId,
        point: SealedConst,
    },

    VarWithin {
        var: VarId,
        outer: SealedConst,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SealedConst {
    Param(ParamId),
    Literal(Value),
}

pub(super) struct InteriorSignatures<'a> {
    interiors: &'a [Signature],

    derived_count: usize,
    phase: InteriorPhase<'a>,
}

enum InteriorPhase<'a> {
    Cq { reader: Option<InteriorId> },

    ReachOpen { reader: Option<InteriorId> },

    ReachSealed { rec: &'a Signature },
}

impl<'a> InteriorSignatures<'a> {
    fn cq(interiors: &'a [Signature], reader: Option<InteriorId>, derived_count: usize) -> Self {
        Self {
            interiors,
            derived_count,
            phase: InteriorPhase::Cq { reader },
        }
    }

    fn reach_open(
        interiors: &'a [Signature],
        reader: Option<InteriorId>,
        derived_count: usize,
    ) -> Self {
        Self {
            interiors,
            derived_count,
            phase: InteriorPhase::ReachOpen { reader },
        }
    }

    fn reach_sealed(interiors: &'a [Signature], rec: &'a Signature, derived_count: usize) -> Self {
        Self {
            interiors,
            derived_count,
            phase: InteriorPhase::ReachSealed { rec },
        }
    }
}

impl InteriorSignatures<'_> {
    fn interiors(&self) -> &[Signature] {
        self.interiors
    }

    fn derived_count(&self) -> usize {
        self.derived_count
    }

    fn reader(&self) -> Option<InteriorId> {
        match self.phase {
            InteriorPhase::Cq { reader } | InteriorPhase::ReachOpen { reader } => reader,
            InteriorPhase::ReachSealed { .. } => None,
        }
    }

    pub(super) fn screen(&self, atom: usize, interior: InteriorId) -> Result<(), ValidationError> {
        let index = usize::try_from(interior.0).expect("64-bit usize");
        if index >= self.derived_count() {
            return Err(ValidationError::UnknownInterior {
                atom: AtomIndex(atom),
                interior,
            });
        }
        if let Some(at) = self.reader() {
            let at_idx = usize::try_from(at.0).expect("64-bit usize");
            if index >= at_idx {
                return Err(ValidationError::InteriorNotPrior { interior, at });
            }
        }
        Ok(())
    }

    /// base is a roster refusal). Rec lookup is unrepresentable on
    fn lookup(&self, interior: InteriorId) -> &Signature {
        let index = usize::try_from(interior.0).expect("64-bit usize");
        let interiors = self.interiors();
        if index < interiors.len() {
            &interiors[index]
        } else {
            match &self.phase {
                InteriorPhase::ReachSealed { rec } => rec,
                InteriorPhase::Cq { .. } | InteriorPhase::ReachOpen { .. } => {
                    unreachable!("screen admitted this id; rec lookup only after the rec is sealed")
                }
            }
        }
    }

    fn column(
        &self,
        atom: usize,
        interior: InteriorId,
        field: FieldId,
    ) -> Result<&ValueType, ValidationError> {
        debug_assert!(
            self.screen(atom, interior).is_ok(),
            "check_atoms screens before column"
        );
        self.lookup(interior)
            .columns
            .get(usize::from(field.0))
            .map(SignatureColumn::ty)
            .ok_or(ValidationError::InteriorColumnOutOfRange {
                atom: AtomIndex(atom),
                field,
            })
    }
}

/// One sealed interior: lowered rules, signature, per-rule typing.
/// Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedInterior {
    lowered: Vec<LoweredRule>,
    signature: Signature,
    rules: Vec<RuleTyping>,
}

impl ValidatedInterior {
    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

pub(crate) use crate::ir::NonEmpty;

/// One lowered rec *base* arm: the rule plus its typing. Base arms
/// cannot name self ([`ValidationError::SelfInBase`]).
#[derive(Debug)]
pub struct ValidatedBaseArm {
    rule: LoweredRule,
    typing: RuleTyping,
}

/// One lowered rec *step* arm: the unique positive self-atom's
/// occurrence ([`Self::self_occ`]) plus the rule and its typing.
/// Missing/nonlinear self are roster refusals; the witness cannot
/// spell them.
#[derive(Debug)]
pub struct ValidatedRecArm {
    self_occ: OccId,
    rule: LoweredRule,
    typing: RuleTyping,
}

impl ValidatedRecArm {
    #[must_use]
    pub(crate) fn self_occ(&self) -> OccId {
        self.self_occ
    }
}

/// The sealed rec SCC: base then rec arms, one signature.
/// Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedRec {
    base: NonEmpty<ValidatedBaseArm>,
    rec: NonEmpty<ValidatedRecArm>,
    signature: Signature,
}

impl ValidatedRec {
    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    #[must_use]
    pub fn base_count(&self) -> usize {
        self.base.len()
    }

    #[must_use]
    pub fn rec_count(&self) -> usize {
        self.rec.len()
    }

    #[must_use]
    pub(crate) fn arm(&self, index: usize) -> &ValidatedRecArm {
        self.rec.get(index).expect("index in range")
    }

    #[must_use]
    pub(crate) fn base_rule<'a>(
        &'a self,
        query: &'a ValidatedQuery,
        index: usize,
    ) -> RuleWitness<'a> {
        let arm = self.base.get(index).expect("index in range");
        RuleWitness {
            rule: &arm.rule,
            typing: &arm.typing,
            query,
        }
    }

    #[must_use]
    pub(crate) fn step_rule<'a>(
        &'a self,
        query: &'a ValidatedQuery,
        index: usize,
    ) -> RuleWitness<'a> {
        let arm = self.rec.get(index).expect("index in range");
        RuleWitness {
            rule: &arm.rule,
            typing: &arm.typing,
            query,
        }
    }

    pub(crate) fn base_rules<'a>(
        &'a self,
        query: &'a ValidatedQuery,
    ) -> impl Iterator<Item = RuleWitness<'a>> {
        (0..self.base.len()).map(|index| self.base_rule(query, index))
    }

    pub(crate) fn step_rules<'a>(
        &'a self,
        query: &'a ValidatedQuery,
    ) -> impl Iterator<Item = RuleWitness<'a>> {
        (0..self.rec.len()).map(|index| self.step_rule(query, index))
    }
}

/// The sealed main query: lowered rules, answer signature, per-rule typing.
/// Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedMain {
    lowered: Vec<LoweredRule>,
    signature: Signature,
    rules: Vec<RuleTyping>,
}

impl ValidatedMain {
    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// The sealed witness: query-global param tables plus a shape sum.
/// Unconstructible outside this module.
/// Variables are rule-scoped, so their typing lives per rule
/// ([`RuleTyping`]); params are query-global, so their tables live here
/// once — unified across every interior, rec arm, and main rule.
/// Rec-absence is `rec: None`; rec-presence is `rec: Some`. Shared
/// fields live on the struct. `rec_id` and `derived_count` are methods
/// — they are determined by interiors and whether rec is present.
#[derive(Debug)]
pub struct ValidatedQuery {
    interiors: Vec<ValidatedInterior>,
    main: ValidatedMain,
    param_types: BTreeMap<ParamId, ValueType>,

    set_params: BTreeSet<ParamId>,

    point_params: BTreeSet<ParamId>,
    rec: Option<ValidatedRec>,
}

#[derive(Debug)]
struct RuleTyping {
    var_types: BTreeMap<VarId, ValueType>,

    group_key: BTreeSet<VarId>,

    classified: Vec<ClassifiedComparison>,

    closed_vars: BTreeMap<VarId, u16>,
}

impl ValidatedQuery {
    #[must_use]
    pub fn interiors(&self) -> &[ValidatedInterior] {
        &self.interiors
    }

    #[must_use]
    pub fn main(&self) -> &ValidatedMain {
        &self.main
    }

    #[must_use]
    pub fn rec(&self) -> Option<&ValidatedRec> {
        self.rec.as_ref()
    }

    /// # Panics

    /// On a programmer-invariant violation: the interior count overflowed
    /// `u32` (validation already refused that).
    #[must_use]
    pub fn rec_id(&self) -> Option<InteriorId> {
        self.rec.as_ref().map(|_| {
            InteriorId(u32::try_from(self.interiors.len()).expect("derived count fits u32"))
        })
    }

    /// # Panics

    /// On a programmer-invariant violation: the derived count overflowed
    /// `u32` (validation already refused that).
    #[must_use]
    pub fn derived_count(&self) -> u32 {
        u32::try_from(self.interiors.len() + usize::from(self.rec.is_some()))
            .expect("derived count fits u32")
    }

    #[must_use]
    pub fn signature(&self) -> &Signature {
        self.main().signature()
    }

    /// # Panics

    /// On a programmer-invariant violation: an index at or beyond
    #[must_use]
    pub fn rule(&self, index: usize) -> RuleWitness<'_> {
        self.main_rule(index)
    }

    #[must_use]
    pub(crate) fn main_rule(&self, index: usize) -> RuleWitness<'_> {
        let main = self.main();
        RuleWitness {
            rule: &main.lowered[index],
            typing: &main.rules[index],
            query: self,
        }
    }

    #[must_use]
    pub(crate) fn interior_rule(&self, interior: usize, index: usize) -> RuleWitness<'_> {
        let inner = &self.interiors()[interior];
        RuleWitness {
            rule: &inner.lowered[index],
            typing: &inner.rules[index],
            query: self,
        }
    }

    pub fn rules(&self) -> impl Iterator<Item = RuleWitness<'_>> {
        let n = self.main().rules.len();
        (0..n).map(|index| self.main_rule(index))
    }

    pub(crate) fn interior_rules(&self, interior: usize) -> impl Iterator<Item = RuleWitness<'_>> {
        let n = self.interiors()[interior].rules.len();
        (0..n).map(move |index| self.interior_rule(interior, index))
    }

    fn param_types_map(&self) -> &BTreeMap<ParamId, ValueType> {
        &self.param_types
    }

    fn set_params_set(&self) -> &BTreeSet<ParamId> {
        &self.set_params
    }

    fn point_params_set(&self) -> &BTreeSet<ParamId> {
        &self.point_params
    }

    /// # Panics

    /// On a programmer-invariant violation: an unknown `ParamId` (the
    #[must_use]
    pub fn param_type(&self, param: ParamId) -> &ValueType {
        &self.param_types_map()[&param]
    }

    pub fn param_types(&self) -> impl Iterator<Item = (ParamId, &ValueType)> {
        self.param_types_map().iter().map(|(p, t)| (*p, t))
    }

    #[must_use]
    pub fn set_params(&self) -> &BTreeSet<ParamId> {
        self.set_params_set()
    }

    #[must_use]
    pub fn point_params(&self) -> &BTreeSet<ParamId> {
        self.point_params_set()
    }
}

/// One rule of the witness: the lowered (Or-free) rule plus its own
/// typing tables, with the query-global param tables reachable through
/// it. Everything downstream of validation runs per rule and consumes
/// exactly this view.
#[derive(Clone, Copy)]
pub struct RuleWitness<'a> {
    rule: &'a LoweredRule,
    typing: &'a RuleTyping,
    query: &'a ValidatedQuery,
}

impl<'a> RuleWitness<'a> {
    #[must_use]
    pub fn rule(&self) -> &'a LoweredRule {
        self.rule
    }

    /// This lowered rule's written-rule provenance (ruled 2026-07-23,
    #[must_use]
    pub fn written(&self) -> Option<u16> {
        self.rule.written
    }

    #[must_use]
    pub fn minted(&self) -> &[u16] {
        &self.rule.minted
    }

    /// # Panics

    /// On a programmer-invariant violation: an unknown `VarId` (the witness
    #[must_use]
    pub fn var_type(&self, var: VarId) -> &ValueType {
        &self.typing.var_types[&var]
    }

    pub fn var_types(&self) -> impl Iterator<Item = (VarId, &ValueType)> {
        self.typing.var_types.iter().map(|(v, t)| (*v, t))
    }

    #[must_use]
    pub(crate) fn classified_comparisons(&self) -> &'a [ClassifiedComparison] {
        &self.typing.classified
    }

    /// # Panics
    #[must_use]
    pub fn param_type(&self, param: ParamId) -> &ValueType {
        self.query.param_type(param)
    }

    #[must_use]
    pub fn sink_vars(&self) -> BTreeSet<VarId> {
        let has_aggregate = self.rule.finds.iter().any(|term| {
            matches!(
                term,
                FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. }
            )
        });
        if has_aggregate {
            self.typing.var_types.keys().copied().collect()
        } else {
            self.typing.group_key.clone()
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn group_key(&self) -> &BTreeSet<VarId> {
        &self.typing.group_key
    }

    #[must_use]
    pub fn dense_domain(&self, var: VarId) -> Option<u16> {
        if let Some(rows) = self.typing.closed_vars.get(&var) {
            return Some(*rows);
        }
        matches!(self.var_type(var), ValueType::Bool).then_some(2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TypeSlot {
    Mono(ValueType),

    Bivalent { interval: ValueType },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParamKind {
    Scalar,
    Set,
}

#[derive(Default)]
struct Context {
    var_slots: BTreeMap<VarId, TypeSlot>,

    var_types: BTreeMap<VarId, ValueType>,

    param_slots: BTreeMap<ParamId, TypeSlot>,

    param_kinds: BTreeMap<ParamId, ParamKind>,

    atom_vars: BTreeSet<VarId>,

    /// refuse them (ruled 2026-07-23, R4); the row count is the proven
    closed_vars: BTreeMap<VarId, u16>,

    scalar_bound_vars: BTreeSet<VarId>,

    negated_vars: BTreeSet<VarId>,

    interval_position_params: BTreeSet<ParamId>,
}

#[cfg(test)]
mod tests;
