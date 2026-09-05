//! Prepare-time row-count estimation: per-occurrence
//! input estimates for the join-order DP and the introspection/report
//! honesty numbers. Three sources, strongest first — schema structure
//! (free and exact), resident-image exact distinct counts, documented
//! constant floors. Prepare **never builds** an image for statistics
//! (the cache is peeked); a cold prepare degrades to bounds and floors,
use crate::api::prepared::source::QuerySource;
use crate::image::ColumnWidth;
use crate::image::ImageBind;
use crate::image::SourceImages;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::WordCmp;
use crate::ir::normalize::Occurrence;
use crate::plan::fj::OccBind;
use crate::plan::fj::split_filters;
use crate::plan::planner::OccStats;
use crate::schema::Schema;
use bumbledb_theory::schema::FieldId;

/// A closed relation's rows ARE its sealed extension — the option is the kind
/// (`schema/relation.rs`) — and its stored `S` counter never exists (closed
/// relations are storage-virtual and write-refused), so a raw counter read
/// prices it at 0.
/// # Errors
pub(crate) fn relation_rows_on(
    source: &QuerySource<'_>,
    schema: &Schema,
    relation: bumbledb_theory::schema::RelationId,
) -> crate::error::Result<u64> {
    let rows = match schema.relation(relation).body().closed_rows() {
        Some(rows) => u64::try_from(rows.len()).expect("bounded extension"),
        None => source.row_count(relation)?,
    };
    crate::obs::event(
        crate::obs::names::RELATION_ROWS,
        crate::obs::TraceArgs::Pair(u64::from(relation.0), rows),
    );
    Ok(rows)
}

pub(crate) const DEFAULT_EQ_DISTINCT: u64 = 64;

pub(crate) const RANGE_KEEP_DEN: u64 = 4;

fn allen_keep(estimate: u64, mask: crate::image::view::MaskConst) -> u64 {
    (estimate.saturating_mul(u64::from(mask.popcount())) / 13).max(1)
}

pub(crate) const FIELDS_EQ_KEEP_DEN: u64 = 64;

pub(crate) const PARAM_SET_PLANNING_ROWS: u64 = 16;

pub(crate) const DELTA_PLANNING_ROWS: u64 = 1;

pub(crate) const ACCUMULATED_PLANNING_ROWS: u64 = 16;

/// # Errors
pub(crate) fn occurrence_stats_on(
    images: &SourceImages<'_>,
    schema: &Schema,
    occurrence: &Occurrence,
    rows: u64,
) -> crate::error::Result<OccStats> {
    match OccBind::of_occurrence(occurrence) {
        OccBind::RecDelta(_) => {
            let floor = DELTA_PLANNING_ROWS.max(1);
            Ok(OccStats {
                occ_id: occurrence.occ_id,
                rows: floor,
                var_distincts: occurrence
                    .vars
                    .iter()
                    .map(|(_, var)| (*var, floor))
                    .collect(),
            })
        }
        OccBind::Finished(_) | OccBind::RecAcc(_) => {
            let floor = ACCUMULATED_PLANNING_ROWS.max(1);
            Ok(OccStats {
                occ_id: occurrence.occ_id,
                rows: floor,
                var_distincts: occurrence
                    .vars
                    .iter()
                    .map(|(_, var)| (*var, floor))
                    .collect(),
            })
        }
        OccBind::Edb(relation) => {
            let image = images.peek(schema, relation)?;
            let mut var_distincts = Vec::with_capacity(occurrence.vars.len());
            for (field, var) in &occurrence.vars {
                let distinct = distinct_of(
                    images.source(),
                    schema,
                    relation,
                    *field,
                    image.as_deref(),
                    rows,
                )?;
                var_distincts.push((*var, distinct));
            }
            let estimate = occurrence_estimate(
                images.source(),
                schema,
                occurrence,
                relation,
                image.as_deref(),
                rows,
            )?;
            Ok(OccStats {
                occ_id: occurrence.occ_id,
                rows: estimate,
                var_distincts,
            })
        }
    }
}

fn selection_matches(value: &Const) -> u64 {
    match value {
        Const::ParamSet(_) => PARAM_SET_PLANNING_ROWS,
        Const::WordSet(words) => u64::try_from(words.len()).expect("bounded set").max(1),
        _ => 1,
    }
}

fn occurrence_estimate(
    source: &QuerySource<'_>,
    schema: &Schema,
    occurrence: &Occurrence,
    relation: bumbledb_theory::schema::RelationId,
    image: Option<&crate::image::RelationImage>,
    rows: u64,
) -> crate::error::Result<u64> {
    let (selections, residuals) = split_filters(&occurrence.filters);
    let mut estimate = rows;
    for selection in &selections {
        let distinct = distinct_of(source, schema, relation, selection.field, image, rows)?;
        estimate =
            (estimate.saturating_mul(selection_matches(&selection.value)) / distinct.max(1)).max(1);
    }

    let mut folded_range_fields: Vec<FieldId> = Vec::new();
    for residual in &residuals {
        if let FilterPredicate::FieldsAllen { mask, .. }
        | FilterPredicate::FieldAllen { mask, .. } = residual
        {
            estimate = allen_keep(estimate, *mask);
            continue;
        }

        if let FilterPredicate::Compare {
            field,
            op: WordCmp::Eq,
            value,
        } = residual
        {
            let distinct = distinct_of(source, schema, relation, field.field(), image, rows)?;
            estimate = (estimate.saturating_mul(selection_matches(value)) / distinct.max(1)).max(1);
            continue;
        }
        if let FilterPredicate::Compare {
            field,
            op: WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge,
            value: Const::Word(_),
        } = residual
        {
            if folded_range_fields.contains(&field.field()) {
                continue;
            }
            folded_range_fields.push(field.field());
            estimate = (estimate / RANGE_KEEP_DEN).max(1);
            continue;
        }
        let keep_den = match residual {
            FilterPredicate::Compare { op, .. } => match op {
                WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge => RANGE_KEEP_DEN,
                WordCmp::Ne => 1,
                WordCmp::Eq => unreachable!("Eq residuals priced above"),
            },
            FilterPredicate::FieldsCompare { op, .. } => match op {
                WordCmp::Eq => FIELDS_EQ_KEEP_DEN,
                WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge => RANGE_KEEP_DEN,
                WordCmp::Ne => 1,
            },

            FilterPredicate::PointIn { .. }
            | FilterPredicate::AnyPointIn { .. }
            | FilterPredicate::FieldsPointIn { .. }
            | FilterPredicate::FieldWithin { .. } => RANGE_KEEP_DEN,
            FilterPredicate::FieldsAllen { .. } | FilterPredicate::FieldAllen { .. } => {
                unreachable!("handled above")
            }
        };
        estimate = (estimate / keep_den).max(1);

        if matches!(residual, FilterPredicate::AnyPointIn { .. }) {
            estimate = estimate.saturating_mul(PARAM_SET_PLANNING_ROWS);
        }
    }
    Ok(estimate.clamp(1, rows.max(1)))
}

fn distinct_of(
    source: &QuerySource<'_>,
    schema: &Schema,
    relation: bumbledb_theory::schema::RelationId,
    field: FieldId,
    image: Option<&crate::image::RelationImage>,
    rows: u64,
) -> crate::error::Result<u64> {
    let descriptor = schema.relation(relation);
    let keyed = descriptor
        .keys()
        .iter()
        .any(|id| schema.key(*id).projection.as_ref() == [field]);
    if keyed {
        let distinct = rows.max(1);
        ladder_event(0, distinct);
        return Ok(distinct);
    }
    if let Some(image) = image {
        let span = image.span(field);
        let first = usize::from(span.first_column);
        let distinct = match span.width {
            ColumnWidth::Byte | ColumnWidth::Word => image.distinct_count(first),

            ColumnWidth::WordPair | ColumnWidth::Words { .. } => (first
                ..first + usize::from(span.width.column_count()))
                .map(|column| image.distinct_count(column))
                .max()
                .expect("at least one column"),
        };
        let distinct = distinct.max(1);
        ladder_event(1, distinct);
        return Ok(distinct);
    }

    let mut containment_bound: Option<u64> = None;
    for id in descriptor.outgoing() {
        let statement = schema.containment(*id);
        if statement.source.projection.as_ref() == [field] && statement.source.selection.is_empty()
        {
            let target_rows = relation_rows_on(source, schema, statement.target.relation)?;
            containment_bound =
                Some(containment_bound.map_or(target_rows, |bound| bound.min(target_rows)));
        }
    }
    if let Some(bound) = containment_bound {
        let distinct = bound.min(rows).max(1);
        ladder_event(2, distinct);
        return Ok(distinct);
    }
    let distinct = match &descriptor.field(field).value_type {
        bumbledb_theory::schema::ValueType::Bool => 2,
        _ => DEFAULT_EQ_DISTINCT,
    };
    ladder_event(3, distinct);
    Ok(distinct)
}

#[inline]
fn ladder_event(rung: u64, distinct: u64) {
    crate::obs::event(
        crate::obs::names::DISTINCT_LADDER,
        crate::obs::TraceArgs::Pair(rung, distinct),
    );
}

#[cfg(test)]
mod tests;
