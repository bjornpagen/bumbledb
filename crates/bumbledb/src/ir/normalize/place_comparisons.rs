use super::{IntervalWord, Occurrence, lower_literal::lower_literal, lower_literal::point_word};
use crate::image::view::{
    Const, FilterPredicate, IntervalConst, MaskConst, OperandAddr, ViewWordSource,
};
use crate::ir::VarId;
use crate::ir::validate::{ClassifiedComparison, SealedConst};
use bumbledb_theory::allen::AllenMask;

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

fn same_atom_mask(mask: AllenMask) -> MaskConst {
    mask
}

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

#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
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
