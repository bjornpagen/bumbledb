use super::{
    lower_literal::{lower_literal, point_word},
    IntervalWord, Occurrence, PlacedAllen, PlacedComparison, PlacedDuration, PlacedWordComparison,
    VarWord,
};
use crate::allen::AllenMask;
use crate::image::view::{Const, FilterPredicate, MaskConst, ResolvedWordSource};
use crate::ir::validate::RuleWitness;
use crate::ir::{CmpOp, Comparison, MaskTerm, Term, VarId};
use crate::schema::{FieldId, ValueType};

/// Mirrors an operator across the comparison when the constant was on the
/// left: `c < x` becomes `x > c`.
fn flip(op: CmpOp) -> CmpOp {
    match op {
        CmpOp::Eq => CmpOp::Eq,
        CmpOp::Ne => CmpOp::Ne,
        CmpOp::Lt => CmpOp::Gt,
        CmpOp::Le => CmpOp::Ge,
        CmpOp::Gt => CmpOp::Lt,
        CmpOp::Ge => CmpOp::Le,
        // The interval operators are lowered to their mask/endpoint
        // shapes before any mirroring (`Allen`'s mirror is the mask's
        // converse; `Contains` has a shape per direction).
        CmpOp::Allen { .. } | CmpOp::Contains => {
            unreachable!("interval operators are lowered before mirroring")
        }
    }
}

/// Whether the variable resolved to an interval type.
fn interval_typed(rule: &RuleWitness<'_>, var: VarId) -> bool {
    matches!(rule.var_type(var), ValueType::Interval { .. })
}

/// The constant side of an interval comparison, as a filter constant
/// (`Const::Interval` from a literal, `Const::Param` resolved at bind).
fn interval_const(constant: &Term) -> Const {
    match constant {
        Term::Param(param) => Const::Param(*param),
        Term::Literal(literal) => lower_literal(literal),
        Term::ParamSet(_) => unreachable!("validated: sets only under Eq"),
        Term::Var(_) => unreachable!("matched the var-var arm above"),
        Term::Duration(_) => unreachable!("measure comparisons lower before this match"),
    }
}

/// The lowered mask side. `mirrored` pre-encodes operand order:
/// `Allen(a, b, m) ≡ Allen(b, a, converse(m))`, so a comparison written
/// constant-first keeps the field on the left and converses the mask —
/// immediately for a literal, deferred to bind for a param.
fn mask_const(mask: MaskTerm, mirrored: bool) -> MaskConst {
    match (mask, mirrored) {
        (MaskTerm::Literal(mask), false) => MaskConst::Mask(mask),
        (MaskTerm::Literal(mask), true) => MaskConst::Mask(mask.converse()),
        (MaskTerm::Param(param), false) => MaskConst::Param(param),
        (MaskTerm::Param(param), true) => MaskConst::ConversedParam(param),
    }
}

/// Interval `Eq`/`Ne` canonicalization: they are the derived facts
/// `Allen(EQUALS)` / `Allen(¬EQUALS)`, so exactly one interval-pair form
/// leaves normalization. Every other comparison passes through.
fn canonicalize(rule: &RuleWitness<'_>, comparison: &Comparison) -> CmpOp {
    let on_intervals = || {
        [&comparison.lhs, &comparison.rhs]
            .iter()
            .any(|term| matches!(term, Term::Var(var) if interval_typed(rule, *var)))
    };
    match comparison.op {
        CmpOp::Eq if on_intervals() => CmpOp::Allen {
            mask: MaskTerm::Literal(AllenMask::EQUALS),
        },
        CmpOp::Ne if on_intervals() => CmpOp::Allen {
            mask: MaskTerm::Literal(AllenMask::EQUALS.complement()),
        },
        op => op,
    }
}

/// The variable's first positive occurrence and the field it reads there.
fn field_of(occurrences: &[Occurrence], var: VarId) -> (usize, FieldId) {
    occurrences
        .iter()
        .enumerate()
        .filter(|(_, occ)| occ.role.participates())
        .find_map(|(occ_idx, occ)| {
            occ.vars
                .iter()
                .find(|(_, v)| *v == var)
                .map(|(field, _)| (occ_idx, *field))
        })
        .expect("validated: comparison variables are atom-bound")
}

fn word(var: VarId, word: IntervalWord) -> VarWord {
    VarWord { var, word }
}

/// Places each comparison. Var-vs-constant pushes down as a filter on the
/// variable's first positive occurrence (sound for multi-occurrence
/// variables — join equality propagates the restriction); same-atom
/// var-vs-var lowers to a per-atom field composition (`FieldsCompare`, or
/// an interval shape); only cross-atom var-vs-var pairs become residuals —
/// whole-value comparisons, except the interval-pair form `Allen`, which
/// stays whole (four endpoint slots + mask, [`PlacedAllen`]), and point
/// containment, which decomposes into word comparisons
/// (docs/architecture/20-query-ir.md).
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one linear pass, each comparison class in order
pub(super) fn place_comparisons(
    rule: &RuleWitness<'_>,
    occurrences: &mut [Occurrence],
) -> (
    Vec<PlacedComparison>,
    Vec<PlacedWordComparison>,
    Vec<PlacedAllen>,
    Vec<PlacedDuration>,
) {
    let mut residuals = Vec::new();
    let mut word_residuals = Vec::new();
    let mut allen_residuals = Vec::new();
    let mut duration_residuals = Vec::new();
    for comparison in &rule.rule().predicates {
        let op = canonicalize(rule, comparison);
        // The measure comparisons first (validation admitted exactly the
        // order operators with one Duration side — 20-query-ir, § the
        // measure): the two Term::Var arms below would otherwise claim
        // the u64 variable side and misread the measure as a constant.
        if let (Term::Duration(interval), other) | (other, Term::Duration(interval)) =
            (&comparison.lhs, &comparison.rhs)
        {
            // The measure stays the left operand; a comparison written
            // measure-second mirrors its operator (`flip`).
            let measure_on_left = matches!(&comparison.lhs, Term::Duration(_));
            let op = if measure_on_left { op } else { flip(op) };
            place_duration(occurrences, &mut duration_residuals, *interval, op, other);
            continue;
        }
        match (&comparison.lhs, &comparison.rhs) {
            (Term::Var(lhs), Term::Var(rhs)) => {
                let same_atom = occurrences
                    .iter()
                    .filter(|occ| occ.role.participates())
                    .find_map(|occ| {
                        let left = occ.vars.iter().find(|(_, v)| v == lhs);
                        let right = occ.vars.iter().find(|(_, v)| v == rhs);
                        match (left, right) {
                            (Some((lf, _)), Some((rf, _))) => Some((occ.occ_id, *lf, *rf)),
                            _ => None,
                        }
                    });
                if let Some((occ_id, left, right)) = same_atom {
                    // The two fixed same-atom interval shapes; every
                    // other operator is a plain field comparison.
                    let filter = match op {
                        CmpOp::Allen { mask } => FilterPredicate::FieldsAllen {
                            left,
                            right,
                            mask: mask_const(mask, false),
                        },
                        CmpOp::Contains => FilterPredicate::FieldsContainPoint {
                            interval: left,
                            point: right,
                        },
                        op => FilterPredicate::FieldsCompare { left, right, op },
                    };
                    let occ = occurrences
                        .iter_mut()
                        .find(|o| o.occ_id == occ_id)
                        .expect("just found");
                    occ.filters.push(filter);
                } else {
                    // Cross-atom: the Allen form stays whole (the mask
                    // residual — four endpoint slots + mask); point
                    // containment decomposes into word comparisons over
                    // slot pairs; everything else is a scalar
                    // whole-value residual.
                    match op {
                        CmpOp::Allen { mask } => allen_residuals.push(PlacedAllen {
                            lhs: *lhs,
                            rhs: *rhs,
                            mask,
                        }),
                        CmpOp::Contains => word_residuals.extend([
                            PlacedWordComparison {
                                op: CmpOp::Le,
                                lhs: word(*lhs, IntervalWord::Start),
                                rhs: word(*rhs, IntervalWord::Start),
                            },
                            PlacedWordComparison {
                                op: CmpOp::Lt,
                                lhs: word(*rhs, IntervalWord::Start),
                                rhs: word(*lhs, IntervalWord::End),
                            },
                        ]),
                        op => residuals.push(PlacedComparison {
                            op,
                            lhs: *lhs,
                            rhs: *rhs,
                        }),
                    }
                }
            }
            (Term::Var(var), constant) | (constant, Term::Var(var)) => {
                let var_on_left = matches!(&comparison.lhs, Term::Var(v) if v == var);
                let (occurrence, field) = field_of(occurrences, *var);
                let filter = match op {
                    // The field stays the left operand; a constant-first
                    // comparison pre-encodes the mirror as the mask's
                    // converse (`mask_const`).
                    CmpOp::Allen { mask } => FilterPredicate::FieldAllen {
                        field,
                        other: interval_const(constant),
                        mask: mask_const(mask, !var_on_left),
                    },
                    // `var ∋ constant`: the constant is a point
                    // (element-typed by validation).
                    CmpOp::Contains if var_on_left => match constant {
                        Term::Param(param) => FilterPredicate::PointIn {
                            field,
                            point: ResolvedWordSource::Param(*param),
                        },
                        Term::Literal(value) => FilterPredicate::PointIn {
                            field,
                            point: ResolvedWordSource::Word(point_word(value)),
                        },
                        Term::ParamSet(_) => unreachable!("validated: sets only under Eq"),
                        Term::Var(_) => unreachable!("matched the var-var arm above"),
                        Term::Duration(_) => {
                            unreachable!("measure comparisons lower before this match")
                        }
                    },
                    // `constant ∋ var`: the variable's scalar field lies
                    // within the constant interval.
                    CmpOp::Contains => FilterPredicate::FieldWithin {
                        field,
                        outer: interval_const(constant),
                    },
                    op => {
                        let op = if var_on_left { op } else { flip(op) };
                        let value = match constant {
                            Term::Param(param) => Const::Param(*param),
                            // `Eq` only (validated) — the selection-level
                            // set marker (docs/architecture/20-query-ir.md,
                            // § param sets).
                            Term::ParamSet(param) => Const::ParamSet(*param),
                            Term::Literal(literal) => lower_literal(literal),
                            Term::Var(_) => unreachable!("matched the var-var arm above"),
                            Term::Duration(_) => {
                                unreachable!("measure comparisons lower before this match")
                            }
                        };
                        FilterPredicate::Compare { field, op, value }
                    }
                };
                occurrences[occurrence].filters.push(filter);
            }
            _ => unreachable!("validated: constant comparisons are rejected"),
        }
    }
    (
        residuals,
        word_residuals,
        allen_residuals,
        duration_residuals,
    )
}

/// Places one measure comparison, `Duration(interval) <op> other` (the
/// measure already on the left). Constant sides and same-atom variable
/// sides push down as filters on the measured variable's first positive
/// occurrence — where the filter-order law holds: an occurrence's other
/// filters (an `Allen` guard, a bounded-end filter) run first, so a
/// guarded fact never reaches the subtraction
/// ([`crate::image::view::FilterPredicate::DurationCompare`]). Only the
/// cross-atom variable side becomes a residual ([`PlacedDuration`]).
fn place_duration(
    occurrences: &mut [Occurrence],
    duration_residuals: &mut Vec<PlacedDuration>,
    interval: VarId,
    op: CmpOp,
    other: &Term,
) {
    let (occ_idx, interval_field) = field_of(occurrences, interval);
    match other {
        Term::Var(scalar) => {
            // Same-atom when the u64 variable is bound on the measured
            // variable's occurrence; cross-atom is the residual.
            let same_atom = occurrences[occ_idx]
                .vars
                .iter()
                .find(|(_, v)| v == scalar)
                .map(|(field, _)| *field);
            match same_atom {
                Some(scalar_field) => {
                    occurrences[occ_idx]
                        .filters
                        .push(FilterPredicate::DurationFieldsCompare {
                            interval: interval_field,
                            op,
                            scalar: scalar_field,
                        });
                }
                None => duration_residuals.push(PlacedDuration {
                    interval,
                    op,
                    scalar: *scalar,
                }),
            }
        }
        Term::Param(param) => {
            occurrences[occ_idx]
                .filters
                .push(FilterPredicate::DurationCompare {
                    field: interval_field,
                    op,
                    value: Const::Param(*param),
                });
        }
        Term::Literal(literal) => {
            occurrences[occ_idx]
                .filters
                .push(FilterPredicate::DurationCompare {
                    field: interval_field,
                    op,
                    value: lower_literal(literal),
                });
        }
        Term::ParamSet(_) => unreachable!("validated: sets only under Eq"),
        Term::Duration(_) => unreachable!("validated: one measure side per comparison"),
    }
}
