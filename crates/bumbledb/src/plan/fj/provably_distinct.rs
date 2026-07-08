use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::NormalizedQuery;
use crate::schema::Schema;
use std::collections::BTreeSet;

/// The distinct-bindings elision check (30-execution): every occurrence's
/// bound fields — variable-bound or equality-filtered to a constant —
/// cover one of its unique constraints, so distinct facts imply distinct
/// bindings and the aggregate sink may skip its seen-set.
pub(super) fn provably_distinct(normalized: &NormalizedQuery, schema: &Schema) -> bool {
    normalized.occurrences.iter().all(|occurrence| {
        let relation = schema.relation(occurrence.relation);
        let bound_fields: BTreeSet<crate::schema::FieldId> =
            occurrence
                .vars
                .iter()
                .map(|(f, _)| *f)
                .chain(occurrence.filters.iter().filter_map(|f| match f {
                    FilterPredicate::Compare {
                        field,
                        op: crate::ir::CmpOp::Eq,
                        value:
                            Const::Word(_)
                            | Const::Byte(_)
                            | Const::Param(_)
                            | Const::PendingIntern { .. },
                    } => Some(*field),
                    _ => None,
                }))
                .collect();
        relation.unique_constraints().iter().any(|cid| {
            relation
                .constraint(*cid)
                .fields()
                .iter()
                .all(|f| bound_fields.contains(f))
        })
    })
}
