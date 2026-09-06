//! Indexed input to the ordinary image/COLT pipeline. Routing only narrows
//! candidates: the executor still applies every exact selection. No new
//! physical index, executor, or shared partial-image cache.
use std::sync::Arc;

use super::{RelationImage, ResidentAdmit, SourceImages};
use crate::api::prepared::source::{QuerySource, compile_error};
use crate::error::Result;
use crate::plan::fj::Selection;
use crate::schema::{DistinctnessWitness, Schema, ValueType, VisitControl};
use bumbledb_theory::schema::RelationId;

// Candidate77's hot bucket spent ~90% of execution in random row lookup:
// two indexed passes cost ~29ms versus ~8ms for a sequential image. Count
// only index entries and conservatively admit buckets <=1/8 of the relation.
// This is an access-cost crossover to measure, never a correctness bound.
const INDEXED_ROW_COST: u64 = 8;

pub(super) fn build(
    images: &SourceImages<'_>,
    schema: &Schema,
    relation: RelationId,
    selections: &[Selection],
    keys: &[Vec<u64>],
) -> Result<Option<ResidentAdmit<Arc<RelationImage>>>> {
    let source = images.source();
    if selections.is_empty()
        || !matches!(source, QuerySource::Store { .. })
        || schema.relation(relation).body().closed_rows().is_some()
    {
        return Ok(None);
    }
    let theory = schema.compiled_theory().map_err(compile_error)?;
    // Every scalar coordinate must have a singleton equality. Prefer the
    // most bound coordinates; no independence/cardinality assumption is
    // used for correctness. Sets, text and multiword selection adapters
    // retain the full-image path until they have a native bucket union.
    let Some(projection) = theory
        .projections_of_relation(relation)
        .iter()
        .filter_map(|&id| theory.projection(id))
        .filter(|p| {
            p.interval_position.is_none()
                && !p.projection.is_empty()
                && p.scalar_fields.iter().all(|field| {
                    matches!(
                        field.value_type,
                        ValueType::Bool | ValueType::U64 | ValueType::I64 | ValueType::F64
                    )
                })
                && p.projection.iter().all(|field| {
                    selections
                        .iter()
                        .position(|s| s.field == *field)
                        .is_some_and(|index| keys[index].len() == 1)
                })
        })
        .max_by_key(|p| p.projection.len())
    else {
        return Ok(None);
    };
    let words: Vec<u64> = projection
        .projection
        .iter()
        .map(|field| {
            let index = selections
                .iter()
                .position(|s| s.field == *field)
                .expect("bound projection coordinate");
            keys[index][0]
        })
        .collect();
    let limit = (source.row_count(relation)? / INDEXED_ROW_COST).max(1);
    let Some(count) = source.compiled_count_bounded(projection, &words, limit)? else {
        return Ok(None);
    };
    let scan = |sink: &mut dyn FnMut(&[u8]) -> Result<()>| -> Result<()> {
        let visited = source.consume_compiled_visits(
            schema,
            relation,
            DistinctnessWitness::FullRowEquality,
            &projection.projection,
            &words,
            &mut |row| {
                sink(row)?;
                Ok(VisitControl::Continue)
            },
        )?;
        debug_assert!(
            visited.is_some(),
            "selected a supported compiled projection"
        );
        Ok(())
    };
    // The exact count and fill share one pinned snapshot. Only this pass
    // fetches bodies; large buckets already selected a sequential scan.
    super::build::build_from_scan(
        schema,
        images.generation(),
        relation,
        count,
        source.work(),
        scan,
    )
    .map(Some)
}
