use super::{IntervalWord, Occurrence, lower_literal::lower_literal, lower_literal::point_word};
use crate::image::view::{
    Const, FilterPredicate, IntervalConst, MaskConst, OperandAddr, ViewWordSource,
};
use crate::ir::VarId;
use crate::ir::validate::{ClassifiedComparison, SealedConst};
use bumbledb_theory::allen::AllenMask;

/// The lowered constant of a sealed comparison side. String stays a
/// pending intern, `bytes<N>` self-encodes, intervals lower to their two
/// column words — [`lower_literal`] owns every case; a param stays a
/// bind-time marker.
fn sealed_interval(constant: &SealedConst) -> IntervalConst {
    match sealed_const(constant) {
        Const::Interval { start, end } => IntervalConst::Interval { start, end },
        Const::Param(param) => IntervalConst::Param(param),
        _ => unreachable!("validated: Allen/within constants are intervals"),
    }
}

fn sealed_const(constant: &SealedConst) -> Const {
    match constant {
        SealedConst::Param(param) => Const::Param(*param),
        SealedConst::Literal(literal) => lower_literal(literal),
    }
}

/// The same-atom mask side of an `Allen` variable pair: the field kept on
/// the left (both variables are the atom's fields), so no mirror applies.
fn same_atom_mask(mask: AllenMask) -> MaskConst {
    mask
}

/// The variable's first positive occurrence and the field it reads there.
fn field_of(occurrences: &[Occurrence], var: VarId) -> (usize, OperandAddr) {
    occurrences
        .iter()
        .enumerate()
        .filter(|(_, occ)| occ.role.participates())
        .find_map(|(occ_idx, occ)| {
            occ.vars
                .iter()
                .find(|(_, v)| *v == var)
                .map(|(field, _)| (occ_idx, OperandAddr::from(*field)))
        })
        .expect("validated: comparison variables are atom-bound")
}

/// The occurrence (by table index) binding both variables of a pair, if
/// any — the same-atom test — with each side's first field there.
fn same_atom(
    occurrences: &[Occurrence],
    lhs: VarId,
    rhs: VarId,
) -> Option<(usize, OperandAddr, OperandAddr)> {
    occurrences
        .iter()
        .enumerate()
        .filter(|(_, occ)| occ.role.participates())
        .find_map(|(idx, occ)| {
            let left = occ.vars.iter().find(|(_, v)| *v == lhs);
            let right = occ.vars.iter().find(|(_, v)| *v == rhs);
            match (left, right) {
                (Some((lf, _)), Some((rf, _))) => {
                    Some((idx, OperandAddr::from(*lf), OperandAddr::from(*rf)))
                }
                _ => None,
            }
        })
}

/// Places each classified comparison — a **total** consumer of
/// validation's sealed proofs ([`ClassifiedComparison`]): the shape,
/// operator, resolved variables, and sealed constants are all decided,
/// so every arm constructs placement and nothing re-derives a
/// comparison's form. Var-vs-constant pushes down as a filter on the
/// variable's first positive occurrence (sound for multi-occurrence
/// variables — join equality propagates the restriction); same-atom
/// var-vs-var lowers to a per-atom field composition (`FieldsCompare`,
/// or an interval shape); only cross-atom var-vs-var pairs become
/// residuals — whole-value comparisons, except the interval-pair form
/// `Allen`, which stays whole (four endpoint slots + mask,
/// [`FilterPredicate::FieldsAllen`]), and point membership, which
/// decomposes into word comparisons (docs/architecture/20-query-ir.md).
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one arm per classified shape, each constructing its placement
pub(super) fn place_comparisons(
    comparisons: &[ClassifiedComparison],
    occurrences: &mut [Occurrence],
) -> (
    Vec<FilterPredicate>,
    Vec<FilterPredicate>,
    Vec<FilterPredicate>,
) {
    let mut residuals = Vec::new();
    let mut word_residuals = Vec::new();
    let mut allen_residuals = Vec::new();
    for comparison in comparisons {
        match comparison {
            // Scalar var-vs-var: same-atom is a per-atom field
            // composition; cross-atom is a whole-value residual.
            ClassifiedComparison::VarVar { op, lhs, rhs } => {
                match same_atom(occurrences, *lhs, *rhs) {
                    Some((occurrence, left, right)) => {
                        occurrences[occurrence]
                            .filters
                            .push(FilterPredicate::FieldsCompare {
                                left,
                                right,
                                op: *op,
                            });
                    }
                    None => residuals.push(FilterPredicate::FieldsCompare {
                        left: OperandAddr::from(*lhs),
                        right: OperandAddr::from(*rhs),
                        op: *op,
                    }),
                }
            }
            // Var-vs-constant: pushes down on the variable's first
            // positive occurrence — the operator is sealed
            // variable-on-left.
            ClassifiedComparison::VarConst { op, var, value } => {
                let (occurrence, field) = field_of(occurrences, *var);
                occurrences[occurrence]
                    .filters
                    .push(FilterPredicate::Compare {
                        field,
                        op: *op,
                        value: sealed_const(value),
                    });
            }
            // The set marker: the selection-level `Eq` compare the plan
            // routes into `selections` (docs/architecture/20-query-ir.md,
            // § param sets).
            ClassifiedComparison::VarInSet { var, set } => {
                let (occurrence, field) = field_of(occurrences, *var);
                occurrences[occurrence]
                    .filters
                    .push(FilterPredicate::Compare {
                        field,
                        op: crate::ir::WordCmp::Eq,
                        value: Const::ParamSet(*set),
                    });
            }
            // Interval-pair `Allen`: same-atom rides the mask-carrying
            // filter kind; cross-atom stays whole as the mask residual
            // (four endpoint slots + mask).
            ClassifiedComparison::AllenVarVar { lhs, rhs, mask } => {
                match same_atom(occurrences, *lhs, *rhs) {
                    Some((occurrence, left, right)) => {
                        occurrences[occurrence]
                            .filters
                            .push(FilterPredicate::FieldsAllen {
                                left,
                                right,
                                mask: same_atom_mask(*mask),
                            });
                    }
                    None => allen_residuals.push(FilterPredicate::FieldsAllen {
                        left: OperandAddr::from(*lhs),
                        right: OperandAddr::from(*rhs),
                        mask: *mask,
                    }),
                }
            }
            // Interval `Allen` against a constant — the field stays the
            // left operand; the mask is sealed field-on-left already.
            ClassifiedComparison::AllenVarConst { var, other, mask } => {
                let (occurrence, field) = field_of(occurrences, *var);
                occurrences[occurrence]
                    .filters
                    .push(FilterPredicate::FieldAllen {
                        field,
                        other: sealed_interval(other),
                        mask: *mask,
                    });
            }
            // `interval ∋ point`: same-atom is the field composition;
            // cross-atom decomposes into two word comparisons over slot
            // pairs (`a.start ≤ p AND p < a.end`).
            ClassifiedComparison::PointInVarVar { interval, point } => {
                match same_atom(occurrences, *interval, *point) {
                    Some((occurrence, interval_field, point_field)) => occurrences[occurrence]
                        .filters
                        .push(FilterPredicate::FieldsPointIn {
                            interval: interval_field,
                            point: point_field,
                        }),
                    None => word_residuals.extend([
                        FilterPredicate::FieldsCompare {
                            op: crate::ir::WordCmp::Le,
                            left: OperandAddr::var_word(*interval, IntervalWord::Start.offset()),
                            right: OperandAddr::var_word(*point, IntervalWord::Start.offset()),
                        },
                        FilterPredicate::FieldsCompare {
                            op: crate::ir::WordCmp::Lt,
                            left: OperandAddr::var_word(*point, IntervalWord::Start.offset()),
                            right: OperandAddr::var_word(*interval, IntervalWord::End.offset()),
                        },
                    ]),
                }
            }
            // `interval-var ∋ constant point`: the point is
            // element-typed by validation — a point membership on the
            // interval field.
            ClassifiedComparison::PointInVarPoint { interval, point } => {
                let (occurrence, field) = field_of(occurrences, *interval);
                let point = match point {
                    SealedConst::Param(param) => ViewWordSource::Param(*param),
                    SealedConst::Literal(value) => ViewWordSource::Word(point_word(value)),
                };
                occurrences[occurrence]
                    .filters
                    .push(FilterPredicate::PointIn { field, point });
            }
            // `constant interval ∋ var`: the variable's scalar field lies
            // within the constant interval.
            ClassifiedComparison::VarWithin { var, outer } => {
                let (occurrence, field) = field_of(occurrences, *var);
                occurrences[occurrence]
                    .filters
                    .push(FilterPredicate::FieldWithin {
                        field,
                        outer: sealed_interval(outer),
                    });
            }
        }
    }
    (residuals, word_residuals, allen_residuals)
}
