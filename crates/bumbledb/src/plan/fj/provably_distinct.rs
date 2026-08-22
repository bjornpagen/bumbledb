use crate::ir::normalize::NormalizedQuery;
use crate::plan::fj::OccBind;
use crate::plan::pinned_fields;
use crate::schema::Schema;
use std::collections::BTreeSet;

/// Proof that distinct facts imply distinct bindings for this rule:
/// every participating occurrence's bound fields cover a key of its
/// relation. Carrying this witness is the license to construct an
/// aggregate sink without a binding seen-set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistinctWitness(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Distinctness {
    Proven(DistinctWitness),
    Unproven,
}

pub(crate) fn provably_distinct(
    normalized: &NormalizedQuery,
    schema: &Schema,
) -> Option<DistinctWitness> {
    normalized
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.role.participates())
        .all(|occurrence| {

            let OccBind::Edb(stored) = OccBind::of_occurrence(occurrence) else {
                return false;
            };
            let relation = schema.relation(stored);
            let bound_fields: BTreeSet<bumbledb_theory::schema::FieldId> = occurrence
                .vars
                .iter()
                .map(|(f, _)| *f)
                .chain(pinned_fields(occurrence).map(|(field, _)| field))
                .collect();
            relation.keys().iter().any(|id| {
                schema
                    .key(*id)
                    .projection
                    .iter()
                    .all(|f| bound_fields.contains(f))
            })
        })
        .then_some(DistinctWitness(()))
}
