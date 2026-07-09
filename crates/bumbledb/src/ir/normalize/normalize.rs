use super::{
    lower_literal::lower_literal, place_comparisons::place_comparisons, NormalizedQuery, OccId,
    Occurrence,
};
use crate::image::view::{Const, FilterPredicate};
use crate::ir::validate::ValidatedQuery;
use crate::ir::{CmpOp, Term, VarId};
use crate::schema::FieldId;

/// Lowers the witness into paper form.
///
/// # Panics
///
/// Only on programmer-invariant violations already excluded by validation
/// (e.g. a comparison variable bound by no atom).
#[must_use]
pub fn normalize(query: &ValidatedQuery) -> NormalizedQuery {
    let mut occurrences: Vec<Occurrence> = query
        .query()
        .atoms
        .iter()
        .enumerate()
        .map(|(idx, atom)| {
            let occ_id = OccId(u16::try_from(idx).expect("validated: atom count fits u16"));
            let mut vars: Vec<(FieldId, VarId)> = Vec::new();
            let mut filters = Vec::new();
            for (field, term) in &atom.bindings {
                match term {
                    Term::Var(var) => {
                        // A repeated variable keeps its first field binding
                        // as the variable position; subsequent positions
                        // lower to same-fact equality filters.
                        if let Some((first_field, _)) = vars.iter().find(|(_, v)| v == var) {
                            filters.push(FilterPredicate::FieldsCompare {
                                left: *first_field,
                                right: *field,
                                op: CmpOp::Eq,
                            });
                        } else {
                            vars.push((*field, *var));
                        }
                    }
                    Term::Param(param) => filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: Const::Param(*param),
                    }),
                    // todo-by-PRD-13: a set binding lowers to a per-atom
                    // any-element filter (with PRD 17's executor support).
                    Term::ParamSet(_) => todo!("todo-by-PRD-13"),
                    Term::Literal(value) => filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: lower_literal(value),
                    }),
                }
            }
            Occurrence {
                occ_id,
                relation: atom.relation,
                vars,
                filters,
            }
        })
        .collect();

    let residuals = place_comparisons(query, &mut occurrences);

    NormalizedQuery {
        occurrences,
        residuals,
    }
}
