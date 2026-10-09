//! The query validation refusal: one variant per rule the IR boundary checks.
use std::fmt;

use crate::error::{AtomIndex, Exceeded, FindIndex, Mismatch, RuleIndex};
use crate::ir::{InteriorId, ParamId, VarId};
use bumbledb_theory::schema::{FieldId, RelationId};

/// A query refused at prepare time. Rules validate one at a time, in order:
/// every rule-local payload (`atom`, comparison `index`, `find`, `var`) names
/// a position inside the first failing rule. `atom` is an occurrence index:
/// positive atoms first, then negated atoms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    EmptyRuleSet,
    ScalarExpression {
        find: FindIndex,
        source: crate::ScalarError,
    },
    TooManyRules {
        count: usize,
    },
    /// Judged across all rules before any disjunct is materialized.
    DnfExceedsRules {
        exceeded: Exceeded<usize>,
    },
    ConditionNestingTooDeep {
        rule: RuleIndex,
        exceeded: Exceeded<usize>,
    },
    HeadArityMismatch {
        rule: RuleIndex,
        mismatch: Mismatch<usize>,
    },
    HeadTypeMismatch {
        rule: RuleIndex,
        position: FindIndex,
    },
    HeadAggregateMismatch {
        rule: RuleIndex,
        position: FindIndex,
    },
    /// A nullary Count in a fold-free multi-rule head is the constant 1.
    CountAcrossRules {
        rules: usize,
    },
    UnknownRelation {
        atom: AtomIndex,
        relation: RelationId,
    },
    UnknownField {
        atom: AtomIndex,
        field: FieldId,
    },
    DuplicateFieldBinding {
        atom: AtomIndex,
        field: FieldId,
    },
    VariableTypeConflict {
        var: VarId,
    },
    LiteralTypeMismatch {
        atom: AtomIndex,
        field: FieldId,
    },
    PointLiteralAtCeiling {
        atom: AtomIndex,
        field: FieldId,
    },
    ParamIdGap {
        param: ParamId,
    },
    ParamTypeConflict {
        param: ParamId,
    },
    ParamScalarAndSet {
        param: ParamId,
    },
    ParamSetComparison {
        index: usize,
    },
    IntervalParamSet {
        param: ParamId,
    },
    IllegalComparison {
        index: usize,
    },
    OrderComparisonOnInterval {
        index: usize,
    },
    OrderComparisonOnFixedBytes {
        index: usize,
    },
    OrderComparisonOnString {
        index: usize,
    },
    OrderComparisonOnClosedReference {
        index: usize,
    },
    ConstantComparison {
        index: usize,
    },
    SelfComparison {
        index: usize,
    },
    ComparisonPointLiteralAtCeiling {
        index: usize,
    },
    EmptyAllenMask {
        index: usize,
    },
    FullAllenMask {
        index: usize,
    },
    MembershipOnlyVariable {
        var: VarId,
    },
    /// A variable in a negated atom must also occur in a positive atom.
    NegatedVariableUnbound {
        var: VarId,
    },
    /// A find or aggregate-input variable must occur in a positive atom.
    UnboundFindVariable {
        var: VarId,
    },
    ComparisonOnlyVariable {
        var: VarId,
    },
    EmptyFinds,
    DuplicateFindTerm {
        index: usize,
    },
    NoPositiveAtoms,
    AggregateInputType {
        find: FindIndex,
    },
    AggregateOverClosedReference {
        find: FindIndex,
    },
    CountWithVariable {
        find: FindIndex,
    },
    AggregateWithoutVariable {
        find: FindIndex,
    },
    AggregateOverGroupKey {
        find: FindIndex,
    },
    MultiplePackTerms {
        find: FindIndex,
    },
    MixedPackAndFold {
        find: FindIndex,
    },
    PackInputType {
        find: FindIndex,
    },
    TooManyAtoms {
        count: usize,
    },
    TooManyVariables {
        count: usize,
    },
    InteriorIdOverflow {
        count: usize,
    },
    EmptyInterior {
        interior: InteriorId,
    },
    EmptyRecursiveBase,
    EmptyRecursiveStep,
    SelfInBase,
    RecArmMissingSelf,
    NonlinearRecArm,
    NegationInRec,
    UnknownInterior {
        atom: AtomIndex,
        interior: InteriorId,
    },
    InteriorColumnOutOfRange {
        atom: AtomIndex,
        field: FieldId,
    },
    InteriorNotPrior {
        interior: InteriorId,
        at: InteriorId,
    },
    AggregateInInterior {
        interior: InteriorId,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRuleSet => write!(f, "the rule set is empty — the empty union is no query"),
            Self::ScalarExpression { find, source } => write!(f, "find {find}: {source}"),
            Self::TooManyRules { count } => {
                write!(f, "{count} rules exceed the rule cap")
            }
            Self::DnfExceedsRules { exceeded } => write!(
                f,
                "DNF distribution produces {} rules against the cap of {}",
                exceeded.observed, exceeded.ceiling
            ),
            Self::ConditionNestingTooDeep { rule, exceeded } => write!(
                f,
                "rule {rule}: condition trees nest {} deep against the cap of {}",
                exceeded.observed, exceeded.ceiling
            ),
            Self::HeadArityMismatch { rule, mismatch } => write!(
                f,
                "rule {rule}: {} find terms against a head of arity {}",
                mismatch.witnessed, mismatch.required
            ),
            Self::HeadTypeMismatch { rule, position } => write!(
                f,
                "rule {rule}: find term {position} disagrees with the head's positional type"
            ),
            Self::HeadAggregateMismatch { rule, position } => write!(
                f,
                "rule {rule}: find term {position} disagrees with the head's shape at that position"
            ),
            Self::CountAcrossRules { rules } => write!(
                f,
                "nullary Count in a fold-free head of a hand-written {rules}-rule \
                 query: the head projection admits one row per group, so the Count \
                 is the constant 1 — write one Count query per disjunct and merge in \
                 the host"
            ),
            Self::UnknownRelation { atom, relation } => {
                write!(f, "atom {atom}: unknown relation {}", relation.0)
            }
            Self::UnknownField { atom, field } => {
                write!(f, "atom {atom}: unknown field {}", field.0)
            }
            Self::DuplicateFieldBinding { atom, field } => {
                write!(f, "atom {atom}: field {} bound twice", field.0)
            }
            Self::VariableTypeConflict { var } => {
                write!(f, "variable {} bound at conflicting types", var.0)
            }
            Self::LiteralTypeMismatch { atom, field } => {
                write!(f, "atom {atom}: literal type mismatch at field {}", field.0)
            }
            Self::PointLiteralAtCeiling { atom, field } => write!(
                f,
                "atom {atom}: point literal at the domain ceiling at field {} — \
                 points are MIN..=MAX-1; MAX is the ray's \u{221e}",
                field.0
            ),
            Self::ParamIdGap { param } => {
                write!(f, "parameter ids are not dense: {} is unused", param.0)
            }
            Self::ParamTypeConflict { param } => {
                write!(f, "parameter {} anchored at conflicting types", param.0)
            }
            Self::ParamScalarAndSet { param } => {
                write!(
                    f,
                    "parameter {} used both as a scalar and as a set",
                    param.0
                )
            }
            Self::ParamSetComparison { index } => {
                write!(f, "comparison {index}: a param set is legal only under Eq")
            }
            Self::IntervalParamSet { param } => write!(
                f,
                "parameter {}: param sets hold points, not intervals",
                param.0
            ),
            Self::IllegalComparison { index } => {
                write!(f, "comparison {index}: type rules violated")
            }
            Self::OrderComparisonOnInterval { index } => write!(
                f,
                "comparison {index}: order operator on an interval — intervals are unordered"
            ),
            Self::OrderComparisonOnFixedBytes { index } => write!(
                f,
                "comparison {index}: order operator on bytes<N> — a digest's \
                 lexicographic order is an encoding artifact; identity only"
            ),
            Self::OrderComparisonOnString { index } => write!(
                f,
                "comparison {index}: order operator on String — strings are equality-only"
            ),
            Self::OrderComparisonOnClosedReference { index } => write!(
                f,
                "comparison {index}: order operator on a closed reference — \
                 declaration-id order is an accident, not semantics"
            ),
            Self::ConstantComparison { index } => {
                write!(f, "comparison {index}: neither side is a variable")
            }
            Self::SelfComparison { index } => {
                write!(f, "comparison {index}: a variable compared with itself")
            }
            Self::ComparisonPointLiteralAtCeiling { index } => write!(
                f,
                "comparison {index}: point literal at the domain ceiling — \
                 points are MIN..=MAX-1; MAX is the ray's \u{221e}"
            ),
            Self::EmptyAllenMask { index } => write!(
                f,
                "comparison {index}: empty Allen mask — no basic relation can hold; \
                 write no query"
            ),
            Self::FullAllenMask { index } => write!(
                f,
                "comparison {index}: full Allen mask — every pair satisfies it; \
                 write no condition"
            ),
            Self::MembershipOnlyVariable { var } => write!(
                f,
                "variable {} is bound only by membership — no enumerable domain",
                var.0
            ),
            Self::NegatedVariableUnbound { var } => write!(
                f,
                "variable {} occurs in a negated atom but in no positive atom",
                var.0
            ),
            Self::UnboundFindVariable { var } => {
                write!(f, "find variable {} bound by no positive atom", var.0)
            }
            Self::ComparisonOnlyVariable { var } => {
                write!(f, "variable {} appears only in comparisons", var.0)
            }
            Self::EmptyFinds => write!(f, "the find list is empty"),
            Self::DuplicateFindTerm { index } => write!(f, "find term {index} is a duplicate"),
            Self::NoPositiveAtoms => write!(f, "the query has no positive atoms"),
            Self::AggregateInputType { find } => {
                write!(
                    f,
                    "find {find}: aggregate input outside the fold's type roster"
                )
            }
            Self::AggregateOverClosedReference { find } => write!(
                f,
                "find {find}: ordering fold over a closed reference — \
                 declaration-id order is an accident, not semantics"
            ),
            Self::CountWithVariable { find } => {
                write!(f, "find {find}: Count is nullary")
            }
            Self::AggregateWithoutVariable { find } => {
                write!(f, "find {find}: this aggregate requires a variable")
            }
            Self::AggregateOverGroupKey { find } => {
                write!(f, "find {find}: aggregate over a group-key variable")
            }
            Self::MultiplePackTerms { find } => {
                write!(f, "find {find}: at most one Pack term per head")
            }
            Self::MixedPackAndFold { find } => {
                write!(f, "find {find}: Pack and fold aggregates may not mix")
            }
            Self::PackInputType { find } => {
                write!(f, "find {find}: Pack folds an interval variable only")
            }
            Self::TooManyAtoms { count } => {
                write!(f, "{count} atom occurrences exceed the planner cap")
            }
            Self::TooManyVariables { count } => {
                write!(f, "{count} distinct variables exceed the 128-bit bitset")
            }
            Self::InteriorIdOverflow { count } => {
                write!(f, "{count} derived tables overflow InteriorId")
            }
            Self::EmptyInterior { interior } => {
                write!(f, "interior {} has no rules", interior.0)
            }
            Self::EmptyRecursiveBase => {
                write!(f, "rec has no base arms — that lfp is empty")
            }
            Self::EmptyRecursiveStep => {
                write!(f, "rec has no rec arms — write an interior")
            }
            Self::SelfInBase => {
                write!(f, "a base arm names the rec")
            }
            Self::RecArmMissingSelf => {
                write!(f, "a rec arm does not name the rec")
            }
            Self::NonlinearRecArm => {
                write!(f, "a rec arm names the rec more than once")
            }
            Self::NegationInRec => {
                write!(f, "negation inside the rec")
            }
            Self::UnknownInterior { atom, interior } => {
                write!(
                    f,
                    "atom {atom}: interior {} is not in the query",
                    interior.0
                )
            }
            Self::InteriorColumnOutOfRange { atom, field } => write!(
                f,
                "atom {atom}: head position {} is beyond the target interior's arity",
                field.0
            ),
            Self::InteriorNotPrior { interior, at } => write!(
                f,
                "interior {} reads interior {} which is not a prior interior",
                at.0, interior.0
            ),
            Self::AggregateInInterior { interior } => write!(
                f,
                "interior {} folds — interior and rec heads project bound variables only",
                interior.0
            ),
        }
    }
}
