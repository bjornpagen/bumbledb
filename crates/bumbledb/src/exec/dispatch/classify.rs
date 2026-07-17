use super::{KeyProbePlan, KeyProbeVar};
use crate::image::view::{Const, FilterPredicate, ResolvedWordSource};
use crate::ir::CmpOp;
use crate::ir::normalize::NormalizedQuery;
use crate::schema::{Relation, Schema};
use bumbledb_theory::schema::{FieldId, StatementId};

/// Classifies a normalized query: `Some(KeyProbePlan)` iff it is key-probe
/// eligible — exactly one atom occurrence (positive, so no negated atoms
/// exist), no residuals, and the occurrence's by-value constant bindings
/// cover some key (`Functionality`) statement's projection (fresh
/// auto-keys included) or bind every field (the full-fact `M` path).
/// Everything else falls through to Free Join.
///
/// Eligibility **consumes validation's term typing** through the lowered
/// filter kinds and never re-infers it: lowering routes a membership
/// binding into `PointIn`/`FieldsPointIn`, so an `Eq` `Compare` on an
/// interval field is interval-typed by construction — a membership binding
/// is not a key cover because it produces no `Eq` `Compare` at all.
///
/// # Panics
///
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
        || !normalized.duration_residuals.is_empty()
    {
        return None;
    }
    // A measure filter disqualifies the fast path: its evaluation is
    // fallible (the ray raises `MeasureOfRay`) and ordered after the
    // occurrence's other filters — both are the filtered view's job, so
    // the query keeps the Free Join path where the view runs.
    if occurrence.filters.iter().any(|filter| {
        matches!(
            filter,
            FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. }
        )
    }) {
        return None;
    }
    // Decision: a `ParamSet`-bound field disqualifies the fast path in v0
    // — k key-probe gets would be correct, but the selection-level path
    // already serves sets (revisit trigger: a measured k-get win). A
    // plan-constant `WordSet` (the grounding-evaluator's attachment) refuses
    // identically — one set rule, both producers; unreachable from the
    // real pipeline (attachments imply a sibling occurrence, so the
    // table is never single-atom), checked for hand-built queries. A
    // var-sourced point falls through with them (its evaluation home is
    // the executor's membership probes).
    if occurrence.filters.iter().any(|filter| {
        matches!(
            filter,
            FilterPredicate::Compare {
                value: Const::ParamSet(_) | Const::WordSet(_),
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

    // An `Idb` occurrence never key-probes: a predicate has no `U`
    // determinants and no `M` entries — its storage is the fixpoint
    // driver's transient image — so an `Idb`-reading rule always keeps
    // the Free Join path.
    let relation = schema.relation(occurrence.source.edb()?);
    // A closed relation has no `U` determinants and no `M` entries — its
    // storage is the theory (`docs/architecture/50-storage.md` § virtual
    // relations) — so even a fully key-bound single atom classifies as
    // Free Join and hits the synthesized image.
    if relation.is_closed() {
        return None;
    }
    let (statement, key_fields) = key_probe_candidate(relation, schema, &value_of)?;

    let key: Vec<(FieldId, Const)> = key_fields
        .iter()
        .map(|f| (*f, value_of(*f).expect("checked above")))
        .collect();

    // The slot layout over the decoded variables (the `SlotWidth` map,
    // exported by normalization).
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
        relation: occurrence.relation(),
        statement,
        key,
        remaining_filters: unconsumed_filters(&occurrence.filters, key_fields),
        vars,
    })
}

/// Prefer a key-statement probe (one `U` get); fall back to the full-fact
/// membership check when every field is bound by value.
fn key_probe_candidate(
    relation: &Relation,
    schema: &Schema,
    value_of: &impl Fn(FieldId) -> Option<Const>,
) -> Option<(Option<StatementId>, Vec<FieldId>)> {
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
            (Some(key.id), key.projection.to_vec())
        })
        .or_else(|| {
            let all: Vec<FieldId> = (0..relation.fields().len())
                .map(|i| FieldId(u16::try_from(i).expect("field count fits u16")))
                .collect();
            all.iter()
                .all(|f| value_of(*f).is_some())
                .then_some((None, all))
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
