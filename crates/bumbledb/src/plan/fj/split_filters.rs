use super::Selection;
use crate::image::view::FilterPredicate;
use crate::ir::CmpOp;

/// Splits an occurrence's lowered predicates into probeable selections
/// (every Eq-against-a-constant, literal or param alike) and the
/// scannable residue (non-Eq compares and every `FieldsCompare` — a
/// repeated in-atom variable is a same-fact condition, not a constant
/// probe). Selections are ordered by field id, stable within a field, so
/// equal queries lower to equal plans.
pub(crate) fn split_filters(filters: &[FilterPredicate]) -> (Vec<Selection>, Vec<FilterPredicate>) {
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
