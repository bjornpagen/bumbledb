use super::{GuardPlan, GuardVar};
use crate::image::view::{Const, FilterPredicate, ResolvedWordSource};
use crate::ir::normalize::NormalizedQuery;
use crate::ir::CmpOp;
use crate::schema::{FieldId, Schema, StatementDescriptor, StatementId};

/// Classifies a normalized query: `Some(GuardPlan)` iff it is guard-probe
/// eligible — exactly one atom occurrence (positive, so no negated atoms
/// exist), no residuals, and the occurrence's by-value constant bindings
/// cover some key (`Functionality`) statement's projection (serial
/// auto-keys included) or bind every field (the full-fact `M` path).
/// Everything else falls through to Free Join.
///
/// Eligibility **consumes validation's term typing** through the lowered
/// filter kinds and never re-infers it: lowering routes a membership
/// binding into `PointIn`/`FieldsContainPoint`, so an `Eq` `Compare` on an
/// interval field is interval-typed by construction — a membership binding
/// is not a key cover because it produces no `Eq` `Compare` at all.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema id widths).
#[must_use]
pub fn classify(normalized: &NormalizedQuery, schema: &Schema) -> Option<GuardPlan> {
    let [occurrence] = normalized.occurrences.as_slice() else {
        return None;
    };
    debug_assert!(
        occurrence.role.participates(),
        "validated: at least one positive atom, positives order first, and \
         the chase cannot eliminate a sourceless single occurrence"
    );
    if !normalized.residuals.is_empty() || !normalized.word_residuals.is_empty() {
        return None;
    }
    // Decision: a `ParamSet`-bound field disqualifies the fast path in v0
    // — k guard gets would be correct, but the selection-level path
    // already serves sets (revisit trigger: a measured k-get win). A
    // var-sourced point falls through with it (its evaluation home is the
    // executor's membership probes).
    if occurrence.filters.iter().any(|filter| {
        matches!(
            filter,
            FilterPredicate::Compare {
                value: Const::ParamSet(_),
                ..
            } | FilterPredicate::AnyPointIn { .. }
                | FilterPredicate::PointIn {
                    point: ResolvedWordSource::Var(_),
                    ..
                }
        )
    }) {
        return None;
    }

    // The fields bound BY VALUE: pinned to a constant by an Eq filter.
    let value_of = |field: FieldId| {
        occurrence.filters.iter().find_map(|f| match f {
            FilterPredicate::Compare {
                field: candidate,
                op: CmpOp::Eq,
                value,
            } if *candidate == field => Some(value.clone()),
            _ => None,
        })
    };

    let relation = schema.relation(occurrence.relation);
    // Prefer a key-statement probe (one `U` get); fall back to the
    // full-fact membership check when every field is bound by value.
    let (statement, key_fields): (Option<StatementId>, Vec<FieldId>) = relation
        .keys()
        .iter()
        .find(|id| {
            key_projection(schema, **id)
                .iter()
                .all(|f| value_of(*f).is_some())
        })
        .map(|id| (Some(*id), key_projection(schema, *id).to_vec()))
        .or_else(|| {
            let all: Vec<FieldId> = (0..relation.fields().len())
                .map(|i| FieldId(u16::try_from(i).expect("validated schema")))
                .collect();
            all.iter()
                .all(|f| value_of(*f).is_some())
                .then_some((None, all))
        })?;

    let key: Vec<(FieldId, Const)> = key_fields
        .iter()
        .map(|f| (*f, value_of(*f).expect("checked above")))
        .collect();

    // The slot layout over the decoded variables (the `SlotWidth` map,
    // exported by normalization).
    let mut slot = 0usize;
    let vars: Vec<GuardVar> = occurrence
        .vars
        .iter()
        .map(|(field, var)| {
            let width = normalized.slot_widths[var].slots();
            let entry = GuardVar {
                field: *field,
                var: *var,
                slot,
                width,
            };
            slot += width;
            entry
        })
        .collect();

    Some(GuardPlan {
        relation: occurrence.relation,
        statement,
        key,
        remaining_filters: unconsumed_filters(&occurrence.filters, key_fields),
        vars,
    })
}

/// Filters not consumed by the key: everything except one Eq filter per
/// key field (the consumed constant).
fn unconsumed_filters(
    filters: &[FilterPredicate],
    mut consumed: Vec<FieldId>,
) -> Vec<FilterPredicate> {
    filters
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
        .collect()
}

/// A key statement's projection (the guard-byte field order).
fn key_projection(schema: &Schema, id: StatementId) -> &[FieldId] {
    match &schema.statement(id).descriptor {
        StatementDescriptor::Functionality { projection, .. } => projection,
        StatementDescriptor::Containment { .. } => {
            unreachable!("Relation::keys() indexes Functionality statements")
        }
    }
}
