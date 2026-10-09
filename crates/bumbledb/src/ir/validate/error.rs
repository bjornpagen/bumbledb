//! The query validation refusal. Variants group by the position they cite
//! (rule, atom field, variable, parameter, comparison, find, rec); the leaf
//! enum names the rule that failed.
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
    TooMany {
        limit: Limit,
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
    Head {
        rule: RuleIndex,
        position: FindIndex,
        mismatch: HeadMismatch,
    },
    /// A nullary Count in a fold-free multi-rule head is the constant 1.
    CountAcrossRules {
        rules: usize,
    },
    UnknownRelation {
        atom: AtomIndex,
        relation: RelationId,
    },
    UnknownInterior {
        atom: AtomIndex,
        interior: InteriorId,
    },
    Field {
        atom: AtomIndex,
        field: FieldId,
        refusal: FieldRefusal,
    },
    Variable {
        var: VarId,
        refusal: VariableRefusal,
    },
    ParamIdGap {
        param: ParamId,
    },
    Param {
        param: ParamId,
        refusal: ParamRefusal,
    },
    Comparison {
        index: usize,
        refusal: ComparisonRefusal,
    },
    EmptyFinds,
    DuplicateFindTerm {
        index: usize,
    },
    NoPositiveAtoms,
    AggregateInputType {
        find: FindIndex,
    },
    Aggregate {
        find: FindIndex,
        refusal: AggregateRefusal,
    },
    EmptyInterior {
        interior: InteriorId,
    },
    InteriorNotPrior {
        interior: InteriorId,
        at: InteriorId,
    },
    AggregateInInterior {
        interior: InteriorId,
    },
    Rec(RecRefusal),
    /// The planner built no valid plan for an accepted rule: an engine defect,
    /// refused instead of executed.
    Unplannable,
}

/// A counted query dimension with a fixed cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    Rules,
    Atoms,
    Variables,
    DerivedTables,
}

/// How a find term disagrees with the declared head at its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadMismatch {
    Type,
    Aggregate,
}

/// Why one atom binding is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRefusal {
    Unknown,
    DuplicateBinding,
    LiteralType,
    /// Points are `MIN..=MAX-1`; `MAX` is the ray's infinity.
    PointLiteralAtCeiling,
    InteriorColumnOutOfRange,
}

/// Why one variable is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRefusal {
    TypeConflict,
    /// Bound only by interval membership: no enumerable domain.
    MembershipOnly,
    /// Occurs in a negated atom but in no positive atom.
    NegatedUnbound,
    /// A find or aggregate input bound by no positive atom.
    UnboundFind,
    ComparisonOnly,
}

/// Why one parameter is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamRefusal {
    TypeConflict,
    ScalarAndSet,
    /// Parameter sets hold points, not intervals.
    IntervalSet,
}

/// Why one comparison is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonRefusal {
    /// A parameter set is legal only under `Eq`.
    ParamSet,
    IllegalTypes,
    /// An integer variable against an F64 variable: no exact common order
    /// without an explicit conversion.
    MixedNumeric,
    Unordered(Unordered),
    Constant,
    SelfComparison,
    /// Points are `MIN..=MAX-1`; `MAX` is the ray's infinity.
    PointLiteralAtCeiling,
    EmptyAllenMask,
    FullAllenMask,
}

/// An operand whose order is not semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unordered {
    Interval,
    FixedBytes,
    String,
    /// Declaration-id order is an accident.
    ClosedReference,
}

/// Why one aggregate find term is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateRefusal {
    /// An ordering fold over a closed reference: declaration-id order is an
    /// accident.
    ClosedReference,
    CountWithVariable,
    WithoutVariable,
    OverGroupKey,
    MultiplePack,
    MixedPackAndFold,
    PackInputType,
}

/// Why the recursive component is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecRefusal {
    EmptyBase,
    EmptyStep,
    SelfInBase,
    ArmMissingSelf,
    NonlinearArm,
    Negation,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRuleSet => write!(f, "the rule set is empty — the empty union is no query"),
            Self::ScalarExpression { find, source } => write!(f, "find {find}: {source}"),
            Self::TooMany { limit, count } => match limit {
                Limit::Rules => write!(f, "{count} rules exceed the rule cap"),
                Limit::Atoms => write!(f, "{count} atom occurrences exceed the planner cap"),
                Limit::Variables => {
                    write!(f, "{count} distinct variables exceed the 128-bit bitset")
                }
                Limit::DerivedTables => write!(f, "{count} derived tables overflow InteriorId"),
            },
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
            Self::Head {
                rule,
                position,
                mismatch,
            } => write!(
                f,
                "rule {rule}: find term {position} disagrees with the head's {} at that position",
                match mismatch {
                    HeadMismatch::Type => "type",
                    HeadMismatch::Aggregate => "shape",
                }
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
            Self::UnknownInterior { atom, interior } => write!(
                f,
                "atom {atom}: interior {} is not in the query",
                interior.0
            ),
            Self::Field {
                atom,
                field,
                refusal,
            } => {
                let field = field.0;
                match refusal {
                    FieldRefusal::Unknown => write!(f, "atom {atom}: unknown field {field}"),
                    FieldRefusal::DuplicateBinding => {
                        write!(f, "atom {atom}: field {field} bound twice")
                    }
                    FieldRefusal::LiteralType => {
                        write!(f, "atom {atom}: literal type mismatch at field {field}")
                    }
                    FieldRefusal::PointLiteralAtCeiling => write!(
                        f,
                        "atom {atom}: point literal at the domain ceiling at field {field} — \
                         points are MIN..=MAX-1; MAX is the ray's \u{221e}"
                    ),
                    FieldRefusal::InteriorColumnOutOfRange => write!(
                        f,
                        "atom {atom}: head position {field} is beyond the target interior's arity"
                    ),
                }
            }
            Self::Variable { var, refusal } => {
                let var = var.0;
                match refusal {
                    VariableRefusal::TypeConflict => {
                        write!(f, "variable {var} bound at conflicting types")
                    }
                    VariableRefusal::MembershipOnly => write!(
                        f,
                        "variable {var} is bound only by membership — no enumerable domain"
                    ),
                    VariableRefusal::NegatedUnbound => write!(
                        f,
                        "variable {var} occurs in a negated atom but in no positive atom"
                    ),
                    VariableRefusal::UnboundFind => {
                        write!(f, "find variable {var} bound by no positive atom")
                    }
                    VariableRefusal::ComparisonOnly => {
                        write!(f, "variable {var} appears only in comparisons")
                    }
                }
            }
            Self::ParamIdGap { param } => {
                write!(f, "parameter ids are not dense: {} is unused", param.0)
            }
            Self::Param { param, refusal } => {
                let param = param.0;
                match refusal {
                    ParamRefusal::TypeConflict => {
                        write!(f, "parameter {param} anchored at conflicting types")
                    }
                    ParamRefusal::ScalarAndSet => {
                        write!(f, "parameter {param} used both as a scalar and as a set")
                    }
                    ParamRefusal::IntervalSet => write!(
                        f,
                        "parameter {param}: param sets hold points, not intervals"
                    ),
                }
            }
            Self::Comparison { index, refusal } => match refusal {
                ComparisonRefusal::ParamSet => {
                    write!(f, "comparison {index}: a param set is legal only under Eq")
                }
                ComparisonRefusal::IllegalTypes => {
                    write!(f, "comparison {index}: type rules violated")
                }
                ComparisonRefusal::MixedNumeric => write!(
                    f,
                    "comparison {index}: an integer variable against an F64 variable — \
                     convert the integer side with toF64Exact"
                ),
                ComparisonRefusal::Unordered(operand) => write!(
                    f,
                    "comparison {index}: order operator on {}",
                    match operand {
                        Unordered::Interval => "an interval — intervals are unordered",
                        Unordered::FixedBytes =>
                            "bytes<N> — a digest's lexicographic order is an encoding \
                             artifact; identity only",
                        Unordered::String => "String — strings are equality-only",
                        Unordered::ClosedReference =>
                            "a closed reference — declaration-id order is an accident, \
                             not semantics",
                    }
                ),
                ComparisonRefusal::Constant => {
                    write!(f, "comparison {index}: neither side is a variable")
                }
                ComparisonRefusal::SelfComparison => {
                    write!(f, "comparison {index}: a variable compared with itself")
                }
                ComparisonRefusal::PointLiteralAtCeiling => write!(
                    f,
                    "comparison {index}: point literal at the domain ceiling — \
                     points are MIN..=MAX-1; MAX is the ray's \u{221e}"
                ),
                ComparisonRefusal::EmptyAllenMask => write!(
                    f,
                    "comparison {index}: empty Allen mask — no basic relation can hold; \
                     write no query"
                ),
                ComparisonRefusal::FullAllenMask => write!(
                    f,
                    "comparison {index}: full Allen mask — every pair satisfies it; \
                     write no condition"
                ),
            },
            Self::EmptyFinds => write!(f, "the find list is empty"),
            Self::DuplicateFindTerm { index } => write!(f, "find term {index} is a duplicate"),
            Self::NoPositiveAtoms => write!(f, "the query has no positive atoms"),
            Self::AggregateInputType { find } => write!(
                f,
                "find {find}: aggregate input outside the fold's type roster"
            ),
            Self::Aggregate { find, refusal } => match refusal {
                AggregateRefusal::ClosedReference => write!(
                    f,
                    "find {find}: ordering fold over a closed reference — \
                     declaration-id order is an accident, not semantics"
                ),
                AggregateRefusal::CountWithVariable => write!(f, "find {find}: Count is nullary"),
                AggregateRefusal::WithoutVariable => {
                    write!(f, "find {find}: this aggregate requires a variable")
                }
                AggregateRefusal::OverGroupKey => {
                    write!(f, "find {find}: aggregate over a group-key variable")
                }
                AggregateRefusal::MultiplePack => {
                    write!(f, "find {find}: at most one Pack term per head")
                }
                AggregateRefusal::MixedPackAndFold => {
                    write!(f, "find {find}: Pack and fold aggregates may not mix")
                }
                AggregateRefusal::PackInputType => {
                    write!(f, "find {find}: Pack folds an interval variable only")
                }
            },
            Self::EmptyInterior { interior } => {
                write!(f, "interior {} has no rules", interior.0)
            }
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
            Self::Rec(refusal) => f.write_str(match refusal {
                RecRefusal::EmptyBase => "rec has no base arms — that lfp is empty",
                RecRefusal::EmptyStep => "rec has no rec arms — write an interior",
                RecRefusal::SelfInBase => "a base arm names the rec",
                RecRefusal::ArmMissingSelf => "a rec arm does not name the rec",
                RecRefusal::NonlinearArm => "a rec arm names the rec more than once",
                RecRefusal::Negation => "negation inside the rec",
            }),
            Self::Unplannable => write!(
                f,
                "the planner built no valid plan for this query (an engine defect)"
            ),
        }
    }
}
