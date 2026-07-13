use super::{
    IntervalWord, Occurrence, PlacedAllen, PlacedComparison, PlacedDuration, PlacedWordComparison,
    VarWord, lower_literal::lower_literal, lower_literal::point_word,
};
use crate::image::view::{Const, FilterPredicate, MaskConst, ResolvedWordSource};
use crate::ir::validate::{ClassifiedComparison, DurationOperand, SealedConst};
use crate::ir::{CmpOp, MaskTerm, VarId};
use crate::schema::FieldId;

/// The lowered constant of a sealed comparison side. String stays a
/// pending intern, `bytes<N>` self-encodes, intervals lower to their two
/// column words — [`lower_literal`] owns every case; a param stays a
/// bind-time marker.
fn sealed_const(constant: &SealedConst) -> Const {
    match constant {
        SealedConst::Param(param) => Const::Param(*param),
        SealedConst::Literal(literal) => lower_literal(literal),
    }
}

/// The same-atom mask side of an `Allen` variable pair: the field kept on
/// the left (both variables are the atom's fields), so no mirror applies.
fn same_atom_mask(mask: MaskTerm) -> MaskConst {
    match mask {
        MaskTerm::Literal(mask) => MaskConst::Mask(mask),
        MaskTerm::Param(param) => MaskConst::Param(param),
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

/// The occurrence (by table index) binding both variables of a pair, if
/// any — the same-atom test — with each side's first field there.
fn same_atom(
    occurrences: &[Occurrence],
    lhs: VarId,
    rhs: VarId,
) -> Option<(usize, FieldId, FieldId)> {
    occurrences
        .iter()
        .enumerate()
        .filter(|(_, occ)| occ.role.participates())
        .find_map(|(idx, occ)| {
            let left = occ.vars.iter().find(|(_, v)| *v == lhs);
            let right = occ.vars.iter().find(|(_, v)| *v == rhs);
            match (left, right) {
                (Some((lf, _)), Some((rf, _))) => Some((idx, *lf, *rf)),
                _ => None,
            }
        })
}

fn word(var: VarId, word: IntervalWord) -> VarWord {
    VarWord { var, word }
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
/// [`PlacedAllen`]), and point membership, which decomposes into word
/// comparisons (docs/architecture/20-query-ir.md).
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one arm per classified shape, each constructing its placement
pub(super) fn place_comparisons(
    comparisons: &[ClassifiedComparison],
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
                    None => residuals.push(PlacedComparison {
                        op: *op,
                        lhs: *lhs,
                        rhs: *rhs,
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
                        op: CmpOp::Eq,
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
                    None => allen_residuals.push(PlacedAllen {
                        lhs: *lhs,
                        rhs: *rhs,
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
                        other: sealed_const(other),
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
                        PlacedWordComparison {
                            op: CmpOp::Le,
                            lhs: word(*interval, IntervalWord::Start),
                            rhs: word(*point, IntervalWord::Start),
                        },
                        PlacedWordComparison {
                            op: CmpOp::Lt,
                            lhs: word(*point, IntervalWord::Start),
                            rhs: word(*interval, IntervalWord::End),
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
                    SealedConst::Param(param) => ResolvedWordSource::Param(*param),
                    SealedConst::Literal(value) => ResolvedWordSource::Word(point_word(value)),
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
                        outer: sealed_const(outer),
                    });
            }
            // The measure, operator sealed measure-on-left: constant and
            // same-atom variable sides push down as filters on the
            // measured variable's first positive occurrence (where the
            // filter-order law holds — an occurrence's other filters run
            // first, so a guarded fact never reaches the subtraction);
            // only the cross-atom variable side is a residual.
            ClassifiedComparison::Duration {
                interval,
                op,
                other,
            } => place_duration(occurrences, &mut duration_residuals, *interval, *op, other),
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
/// operator already sealed measure-on-left). Constant sides and same-atom
/// variable sides push down as filters on the measured variable's first
/// positive occurrence; only the cross-atom variable side becomes a
/// residual ([`PlacedDuration`]).
fn place_duration(
    occurrences: &mut [Occurrence],
    duration_residuals: &mut Vec<PlacedDuration>,
    interval: VarId,
    op: CmpOp,
    other: &DurationOperand,
) {
    let (occ_idx, interval_field) = field_of(occurrences, interval);
    match other {
        DurationOperand::Var(scalar) => {
            // Same-atom when the u64 variable is bound on the measured
            // variable's occurrence; cross-atom is the residual.
            let same = occurrences[occ_idx]
                .vars
                .iter()
                .find(|(_, v)| v == scalar)
                .map(|(field, _)| *field);
            match same {
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
        DurationOperand::Const(value) => {
            occurrences[occ_idx]
                .filters
                .push(FilterPredicate::DurationCompare {
                    field: interval_field,
                    op,
                    value: sealed_const(value),
                });
        }
    }
}
