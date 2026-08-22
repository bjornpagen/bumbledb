use super::{KeyProbePlan, KeyProbeVar};
use crate::image::view::{Const, FilterPredicate};
use crate::ir::WordCmp;
use crate::ir::normalize::NormalizedQuery;
use crate::plan::fj::OccBind;
use crate::schema::{Relation, Schema};
use bumbledb_theory::schema::FieldId;

/// Classifies a normalized query: `Some(KeyProbePlan)` iff it is key-probe
/// eligible — exactly one atom occurrence (positive, so no negated atoms
/// exist), no residuals, and the occurrence's by-value constant bindings
/// cover some key (`Functionality`) statement's projection (fresh
/// auto-keys included) or bind every field (the full-fact `M` path).
/// Everything else falls through to Free Join.
/// Eligibility **consumes validation's term typing** through the lowered
/// filter kinds and never re-infers it: lowering routes a membership
/// # Panics
/// Only on programmer-invariant violations (validated-schema id widths).
#[must_use]
pub fn classify(normalized: &NormalizedQuery, schema: &Schema) -> Option<KeyProbePlan> {
    let [occurrence] = normalized.occurrences.as_slice() else {
        return None;
    };
    debug_assert!(
        occurrence.role.participates(),
        "validated: at least one positive atom, positives order first, and \
         the grounding cannot eliminate a sourceless single occurrence"
    );
    if !normalized.residuals.is_empty()
        || !normalized.word_residuals.is_empty()
        || !normalized.allen_residuals.is_empty()
    {
        return None;
    }

    if !occurrence.point_vars.is_empty()
        || occurrence.filters.iter().any(|filter| {
            matches!(
                filter,
                FilterPredicate::Compare {
                    value: Const::ParamSet(_) | Const::WordSet(_),
                    ..
                } | FilterPredicate::AnyPointIn { .. }
            )
        })
    {
        return None;
    }

    let value_of = |field: FieldId| {
        occurrence.filters.iter().find_map(|f| match f {
            FilterPredicate::Compare {
                field: candidate,
                op: WordCmp::Eq,
                value,
            } if *candidate == field => Some(value.clone()),
            _ => None,
        })
    };

    let OccBind::Edb(relation_id) = OccBind::of_occurrence(occurrence) else {
        return None;
    };
    let relation = schema.relation(relation_id);

    if relation.body().closed_rows().is_some() {
        return None;
    }
    let kind = key_probe_candidate(relation, schema, &value_of)?;
    let key_fields: Vec<FieldId> = kind.key().iter().map(|(f, _)| *f).collect();

    let mut slot = 0usize;
    let vars: Vec<KeyProbeVar> = occurrence
        .vars
        .iter()
        .map(|(field, var)| {
            let width = normalized.slot_widths[var].slots();
            let entry = KeyProbeVar {
                field: *field,
                var: *var,
                slot,
                width,
            };
            slot += width;
            entry
        })
        .collect();

    Some(KeyProbePlan {
        relation: relation_id,
        kind,
        remaining_filters: unconsumed_filters(&occurrence.filters, key_fields),
        vars,
    })
}

fn key_probe_candidate(
    relation: &Relation,
    schema: &Schema,
    value_of: &impl Fn(FieldId) -> Option<Const>,
) -> Option<super::KeyProbeKind> {
    relation
        .keys()
        .iter()
        .find(|id| {
            schema
                .key(**id)
                .projection
                .iter()
                .all(|f| value_of(*f).is_some())
        })
        .map(|id| {
            let key = schema.key(*id);
            super::KeyProbeKind::Uniqueness {
                statement: key.id,
                key: key
                    .projection
                    .iter()
                    .map(|f| (*f, value_of(*f).expect("checked above")))
                    .collect(),
            }
        })
        .or_else(|| {
            let all: Vec<FieldId> = (0..relation.fields().len())
                .map(|i| FieldId(u16::try_from(i).expect("field count fits u16")))
                .collect();
            all.iter()
                .all(|f| value_of(*f).is_some())
                .then(|| super::KeyProbeKind::Membership {
                    key: all
                        .iter()
                        .map(|f| (*f, value_of(*f).expect("checked above")))
                        .collect(),
                })
        })
}

fn unconsumed_filters(
    filters: &[FilterPredicate],
    mut consumed: Vec<FieldId>,
) -> Vec<FilterPredicate> {
    filters
        .iter()
        .filter(|f| match f {
            FilterPredicate::Compare {
                field,
                op: WordCmp::Eq,
                ..
            } => {
                if let Some(idx) = consumed.iter().position(|c| c == field) {
                    consumed.swap_remove(idx);
                    false
                } else {
                    true
                }
            }
            _ => true,
        })
        .cloned()
        .collect()
}
