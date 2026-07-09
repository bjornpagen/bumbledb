use super::{lower_literal::lower_literal, Occurrence, PlacedComparison};
use crate::image::view::{Const, FilterPredicate};
use crate::ir::validate::ValidatedQuery;
use crate::ir::{CmpOp, Term};

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
        // todo-by-PRD-13: interval operators decompose into word
        // comparisons over start/end rather than mirroring as one op.
        CmpOp::Overlaps | CmpOp::Contains => todo!("todo-by-PRD-13"),
    }
}

/// Places each comparison. Var-vs-constant pushes down as a filter on the
/// variable's first occurrence (sound for multi-occurrence variables —
/// join equality propagates the restriction); same-atom var-vs-var lowers
/// to a per-atom `FieldsCompare` filter; only cross-atom var-vs-var pairs
/// become residuals (docs/architecture/20-query-ir.md).
pub(super) fn place_comparisons(
    query: &ValidatedQuery,
    occurrences: &mut [Occurrence],
) -> Vec<PlacedComparison> {
    let mut residuals = Vec::new();
    for comparison in &query.query().predicates {
        match (&comparison.lhs, &comparison.rhs) {
            (Term::Var(lhs), Term::Var(rhs)) => {
                let same_atom = occurrences.iter().find_map(|occ| {
                    let left = occ.vars.iter().find(|(_, v)| v == lhs);
                    let right = occ.vars.iter().find(|(_, v)| v == rhs);
                    match (left, right) {
                        (Some((lf, _)), Some((rf, _))) => Some((occ.occ_id, *lf, *rf)),
                        _ => None,
                    }
                });
                if let Some((occ_id, left, right)) = same_atom {
                    let occ = occurrences
                        .iter_mut()
                        .find(|o| o.occ_id == occ_id)
                        .expect("just found");
                    occ.filters.push(FilterPredicate::FieldsCompare {
                        left,
                        right,
                        op: comparison.op,
                    });
                } else {
                    residuals.push(PlacedComparison {
                        op: comparison.op,
                        lhs: *lhs,
                        rhs: *rhs,
                    });
                }
            }
            (Term::Var(var), constant) | (constant, Term::Var(var)) => {
                let var_on_left = matches!(&comparison.lhs, Term::Var(v) if v == var);
                let op = if var_on_left {
                    comparison.op
                } else {
                    flip(comparison.op)
                };
                let value = match constant {
                    Term::Param(param) => Const::Param(*param),
                    // todo-by-PRD-13: an `Eq`-against-set lowers to an
                    // any-element filter (with PRD 17's executor support).
                    Term::ParamSet(_) => todo!("todo-by-PRD-13"),
                    Term::Literal(literal) => lower_literal(literal),
                    Term::Var(_) => unreachable!("matched the var-var arm above"),
                };
                let (occurrence, field) = occurrences
                    .iter()
                    .enumerate()
                    .find_map(|(occ_idx, occ)| {
                        occ.vars
                            .iter()
                            .find(|(_, v)| v == var)
                            .map(|(field, _)| (occ_idx, *field))
                    })
                    .expect("validated: comparison variables are atom-bound");
                occurrences[occurrence]
                    .filters
                    .push(FilterPredicate::Compare { field, op, value });
            }
            _ => unreachable!("validated: constant comparisons are rejected"),
        }
    }
    residuals
}
