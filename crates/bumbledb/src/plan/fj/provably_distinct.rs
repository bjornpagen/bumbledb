use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{NormalizedQuery, Polarity};
use crate::schema::{Schema, StatementDescriptor};
use std::collections::BTreeSet;

/// The distinct-bindings elision check (40-execution): every positive
/// occurrence's bound fields — variable-bound or equality-pinned to one
/// constant — cover the projection of one of its keys (`Functionality`
/// statements), so distinct facts imply distinct bindings and the
/// aggregate sink may skip its seen-set. Negated occurrences bind
/// nothing (they only reject), so they cannot break the proof.
///
/// Two guards keep the proof honest:
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
        .filter(|occurrence| occurrence.polarity == Polarity::Positive)
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
                let StatementDescriptor::Functionality { projection, .. } =
                    &schema.statement(*id).descriptor
                else {
                    unreachable!("Relation::keys() indexes Functionality statements")
                };
                projection.iter().all(|f| bound_fields.contains(f))
            })
        })
}
