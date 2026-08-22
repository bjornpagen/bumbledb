use super::Selection;
use crate::image::view::FilterPredicate;
use crate::ir::WordCmp;

pub(crate) fn split_filters(filters: &[FilterPredicate]) -> (Vec<Selection>, Vec<FilterPredicate>) {
    let mut selections: Vec<Selection> = filters
        .iter()
        .filter_map(|f| match f {
            FilterPredicate::Compare {
                field,
                op: WordCmp::Eq,
                value,
            } => Some(Selection {
                field: field.field(),
                value: value.clone(),
            }),
            _ => None,
        })
        .collect();
    selections.sort_by_key(|s| s.field);
    let residuals: Vec<FilterPredicate> = filters
        .iter()
        .filter(|f| {
            !matches!(
                f,
                FilterPredicate::Compare {
                    op: WordCmp::Eq,
                    ..
                }
            )
        })
        .cloned()
        .collect();
    (selections, residuals)
}
