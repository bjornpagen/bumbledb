use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::NormalizedQuery;
use crate::schema::Schema;
use std::collections::BTreeSet;

/// The distinct-bindings elision check (40-execution): every participating
/// occurrence's bound fields — variable-bound or equality-pinned to one
/// constant — cover the projection of one of its keys (`Functionality`
/// statements), so distinct facts imply distinct bindings and the
/// aggregate sink may skip its seen-set. Only participating occurrences
/// are quantified: negated occurrences bind nothing (they only reject)
/// and grounding-eliminated occurrences contribute no facts at all
/// (`plan/ground.rs`), so neither can break the proof.
///
/// Two checks keep the proof honest:
/// - **Pointwise keys**: coverage requires the interval field bound **by
///   value** — `vars` holds value bindings only (membership positions
///   lowered to filters and never enter it), and membership filter kinds
///   are not counted below, so a scalar-prefix-only binding fails
///   coverage: two facts may share the prefix with disjoint intervals.
/// - **Set-bound fields pin nothing**: an Eq against a `ParamSet`/
///   `WordSet` matches any element, so two distinct facts can differ on
///   that field while producing one binding — sets are excluded from the
///   pinned-constant field set.
pub(super) fn provably_distinct(normalized: &NormalizedQuery, schema: &Schema) -> bool {
    normalized
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.role.participates())
        .all(|occurrence| {
            let relation = schema.relation(occurrence.relation);
            let bound_fields: BTreeSet<crate::schema::FieldId> = occurrence
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
                            | Const::Interval { .. }
                            | Const::Param(_)
                            | Const::PendingIntern { .. },
                    } => Some(*field),
                    _ => None,
                }))
                .collect();
            relation.keys().iter().any(|id| {
                schema
                    .key(*id)
                    .projection
                    .iter()
                    .all(|f| bound_fields.contains(f))
            })
        })
}
