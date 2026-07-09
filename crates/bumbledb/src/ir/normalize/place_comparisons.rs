use super::{
    lower_literal::{lower_literal, point_word},
    IntervalWord, Occurrence, PlacedComparison, PlacedWordComparison, Polarity, VarWord,
};
use crate::image::view::{Const, FilterPredicate, ResolvedWordSource};
use crate::ir::validate::ValidatedQuery;
use crate::ir::{CmpOp, Term, Value, VarId};
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
        // Interval operators are lowered to their fixed filter shapes
        // before any mirroring (`Overlaps` is symmetric; `Contains` has a
        // shape per direction).
        CmpOp::Overlaps | CmpOp::Contains => {
            unreachable!("interval operators are lowered before mirroring")
        }
    }
}

/// Whether the variable resolved to an interval type.
fn interval_typed(query: &ValidatedQuery, var: VarId) -> bool {
    matches!(query.var_type(var), ValueType::Interval { .. })
}

/// The constant side of an interval comparison, as a filter constant
/// (`Const::Interval` from a literal, `Const::Param` resolved at bind).
fn interval_const(constant: &Term) -> Const {
    match constant {
        Term::Param(param) => Const::Param(*param),
        Term::Literal(literal) => lower_literal(literal),
        Term::ParamSet(_) => unreachable!("validated: sets only under Eq"),
        Term::Var(_) => unreachable!("matched the var-var arm above"),
    }
}

/// The variable's first positive occurrence and the field it reads there.
fn field_of(occurrences: &[Occurrence], var: VarId) -> (usize, FieldId) {
    occurrences
        .iter()
        .enumerate()
        .filter(|(_, occ)| occ.polarity == Polarity::Positive)
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
/// whole-value comparisons, except `Overlaps`/`Contains`, which decompose
/// into word comparisons over slot pairs
/// (docs/architecture/20-query-ir.md).
#[allow(clippy::too_many_lines)] // one linear pass, each comparison class in order
pub(super) fn place_comparisons(
    query: &ValidatedQuery,
    occurrences: &mut [Occurrence],
) -> (Vec<PlacedComparison>, Vec<PlacedWordComparison>) {
    let mut residuals = Vec::new();
    let mut word_residuals = Vec::new();
    for comparison in &query.query().predicates {
        match (&comparison.lhs, &comparison.rhs) {
            (Term::Var(lhs), Term::Var(rhs)) => {
                let same_atom = occurrences
                    .iter()
                    .filter(|occ| occ.polarity == Polarity::Positive)
                    .find_map(|occ| {
                        let left = occ.vars.iter().find(|(_, v)| v == lhs);
                        let right = occ.vars.iter().find(|(_, v)| v == rhs);
                        match (left, right) {
                            (Some((lf, _)), Some((rf, _))) => Some((occ.occ_id, *lf, *rf)),
                            _ => None,
                        }
                    });
                if let Some((occ_id, left, right)) = same_atom {
                    // The three fixed same-atom interval shapes; every
                    // other operator is a plain field comparison.
                    let filter = match comparison.op {
                        CmpOp::Overlaps => FilterPredicate::FieldsOverlap { left, right },
                        CmpOp::Contains => {
                            if interval_typed(query, *rhs) {
                                FilterPredicate::FieldsContain {
                                    outer: left,
                                    inner: right,
                                }
                            } else {
                                FilterPredicate::FieldsContainPoint {
                                    interval: left,
                                    point: right,
                                }
                            }
                        }
                        op => FilterPredicate::FieldsCompare { left, right, op },
                    };
                    let occ = occurrences
                        .iter_mut()
                        .find(|o| o.occ_id == occ_id)
                        .expect("just found");
                    occ.filters.push(filter);
                } else {
                    // Cross-atom: interval predicates decompose into word
                    // comparisons over slot pairs; everything else is a
                    // whole-value residual (interval Eq/Ne compare
                    // pairwise).
                    match comparison.op {
                        CmpOp::Overlaps => word_residuals.extend([
                            PlacedWordComparison {
                                op: CmpOp::Lt,
                                lhs: word(*lhs, IntervalWord::Start),
                                rhs: word(*rhs, IntervalWord::End),
                            },
                            PlacedWordComparison {
                                op: CmpOp::Lt,
                                lhs: word(*rhs, IntervalWord::Start),
                                rhs: word(*lhs, IntervalWord::End),
                            },
                        ]),
                        CmpOp::Contains if interval_typed(query, *rhs) => word_residuals.extend([
                            PlacedWordComparison {
                                op: CmpOp::Le,
                                lhs: word(*lhs, IntervalWord::Start),
                                rhs: word(*rhs, IntervalWord::Start),
                            },
                            PlacedWordComparison {
                                op: CmpOp::Le,
                                lhs: word(*rhs, IntervalWord::End),
                                rhs: word(*lhs, IntervalWord::End),
                            },
                        ]),
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
                let filter = match comparison.op {
                    // Symmetric: `Overlaps(c, x)` is `Overlaps(x, c)`.
                    CmpOp::Overlaps => FilterPredicate::Compare {
                        field,
                        op: CmpOp::Overlaps,
                        value: interval_const(constant),
                    },
                    // `var ⊇ constant`: an interval constant compares as
                    // `Contains`; an element constant is point membership.
                    CmpOp::Contains if var_on_left => match constant {
                        Term::Param(param) => {
                            if matches!(query.param_type(*param), ValueType::Interval { .. }) {
                                FilterPredicate::Compare {
                                    field,
                                    op: CmpOp::Contains,
                                    value: Const::Param(*param),
                                }
                            } else {
                                FilterPredicate::PointIn {
                                    field,
                                    point: ResolvedWordSource::Param(*param),
                                }
                            }
                        }
                        Term::Literal(value @ (Value::U64(_) | Value::I64(_))) => {
                            FilterPredicate::PointIn {
                                field,
                                point: ResolvedWordSource::Word(point_word(value)),
                            }
                        }
                        Term::Literal(value) => FilterPredicate::Compare {
                            field,
                            op: CmpOp::Contains,
                            value: lower_literal(value),
                        },
                        Term::ParamSet(_) => unreachable!("validated: sets only under Eq"),
                        Term::Var(_) => unreachable!("matched the var-var arm above"),
                    },
                    // `constant ⊇ var`: the reversed containment — the
                    // variable's field (interval or point) lies within the
                    // constant interval.
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
                        };
                        FilterPredicate::Compare { field, op, value }
                    }
                };
                occurrences[occurrence].filters.push(filter);
            }
            _ => unreachable!("validated: constant comparisons are rejected"),
        }
    }
    (residuals, word_residuals)
}
