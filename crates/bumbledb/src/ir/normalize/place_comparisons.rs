use super::{
    lower_literal::{lower_literal, point_word},
    IntervalWord, Occurrence, PlacedAllen, PlacedComparison, PlacedWordComparison, VarWord,
};
use crate::allen::AllenMask;
use crate::image::view::{Const, FilterPredicate, MaskConst, ResolvedWordSource};
use crate::ir::validate::ValidatedQuery;
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
fn canonicalize(query: &ValidatedQuery, comparison: &Comparison) -> CmpOp {
    let on_intervals = || {
        [&comparison.lhs, &comparison.rhs]
            .iter()
            .any(|term| matches!(term, Term::Var(var) if interval_typed(query, *var)))
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
#[allow(clippy::too_many_lines)] // one linear pass, each comparison class in order
pub(super) fn place_comparisons(
    query: &ValidatedQuery,
    occurrences: &mut [Occurrence],
) -> (
    Vec<PlacedComparison>,
    Vec<PlacedWordComparison>,
    Vec<PlacedAllen>,
) {
    let mut residuals = Vec::new();
    let mut word_residuals = Vec::new();
    let mut allen_residuals = Vec::new();
    for comparison in &query.query().predicates {
        let op = canonicalize(query, comparison);
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
                        };
                        FilterPredicate::Compare { field, op, value }
                    }
                };
                occurrences[occurrence].filters.push(filter);
            }
            _ => unreachable!("validated: constant comparisons are rejected"),
        }
    }
    (residuals, word_residuals, allen_residuals)
}
