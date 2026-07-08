use super::GuardPlan;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::NormalizedQuery;
use crate::ir::CmpOp;
use crate::schema::{ConstraintId, FieldId, Schema};

/// Classifies a normalized query: `Some(GuardPlan)` iff it is guard-probe
/// eligible — exactly one atom occurrence, no residuals, and the
/// occurrence's Eq-constant fields cover a unique constraint (including
/// serial auto-uniques) or the full fact.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema id widths).
#[must_use]
pub fn classify(normalized: &NormalizedQuery, schema: &Schema) -> Option<GuardPlan> {
    let [occurrence] = normalized.occurrences.as_slice() else {
        return None;
    };
    if !normalized.residuals.is_empty() {
        return None;
    }
    let relation = schema.relation(occurrence.relation);

    // The fields pinned to constants by Eq filters, with their constants.
    let constant_of = |field: FieldId| {
        occurrence.filters.iter().find_map(|f| match f {
            FilterPredicate::Compare {
                field: candidate,
                op: CmpOp::Eq,
                value,
            } if *candidate == field => Some(value.clone()),
            _ => None,
        })
    };

    // Prefer a unique-constraint probe; fall back to the full-fact
    // membership check when every field is constant.
    let (constraint, key_fields): (Option<ConstraintId>, Vec<FieldId>) = relation
        .unique_constraints()
        .iter()
        .find(|cid| {
            relation
                .constraint(**cid)
                .fields()
                .iter()
                .all(|f| constant_of(*f).is_some())
        })
        .map(|cid| (Some(*cid), relation.constraint(*cid).fields().to_vec()))
        .or_else(|| {
            let all: Vec<FieldId> = (0..relation.fields().len())
                .map(|i| FieldId(u16::try_from(i).expect("validated schema")))
                .collect();
            all.iter()
                .all(|f| constant_of(*f).is_some())
                .then_some((None, all))
        })?;

    let key: Vec<(FieldId, Const)> = key_fields
        .iter()
        .map(|f| (*f, constant_of(*f).expect("checked above")))
        .collect();
    // Filters not consumed by the key: everything except one Eq filter per
    // key field (the consumed constant).
    let mut consumed: Vec<FieldId> = key_fields;
    let remaining_filters: Vec<FilterPredicate> = occurrence
        .filters
        .iter()
        .filter(|f| match f {
            FilterPredicate::Compare {
                field,
                op: CmpOp::Eq,
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
        .collect();

    Some(GuardPlan {
        relation: occurrence.relation,
        constraint,
        key,
        remaining_filters,
        vars: occurrence.vars.clone(),
    })
}
