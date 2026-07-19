use super::Selection;
use crate::image::view::FilterPredicate;
use crate::ir::CmpOp;

/// Splits an occurrence's lowered conditions into probeable selections
/// (every Eq-against-a-constant, literal or param alike) and the
/// scannable residue (non-Eq compares and every `FieldsCompare` — a
/// repeated in-atom variable is a same-fact condition, not a constant
/// probe). Selections are ordered by field id, stable within a field, so
/// equal queries lower to equal plans.
///
/// A measure predicate pins the whole list residual: the filter-order
/// law (`docs/architecture/20-query-ir.md` § the measure) promises the
/// subtraction runs only on survivors of the atom's *other* predicates,
/// and `image/view/apply.rs` honors it over the VIEW's filter list —
/// but a selection level probes only after the view (measure refinement
/// included) is built, so an Eq lifted into a selection would let a row
/// it excludes reach the subtraction and raise `MeasureOfRay` for a
/// fact the query filtered out. Correctness owns the split: the atom
/// keeps its Eq-constants as scannable filters and pays with scans
/// instead of probes.
pub(crate) fn split_filters(filters: &[FilterPredicate]) -> (Vec<Selection>, Vec<FilterPredicate>) {
    let measured = filters.iter().any(|f| {
        matches!(
            f,
            FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. }
        )
    });
    if measured {
        return (Vec::new(), filters.to_vec());
    }
    let mut selections: Vec<Selection> = filters
        .iter()
        .filter_map(|f| match f {
            FilterPredicate::Compare {
                field,
                op: CmpOp::Eq,
                value,
            } => Some(Selection {
                field: *field,
                value: value.clone(),
            }),
            _ => None,
        })
        .collect();
    selections.sort_by_key(|s| s.field);
    let residuals: Vec<FilterPredicate> = filters
        .iter()
        .filter(|f| !matches!(f, FilterPredicate::Compare { op: CmpOp::Eq, .. }))
        .cloned()
        .collect();
    (selections, residuals)
}
