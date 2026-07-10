use super::{
    AggregateSink, Bindings, Colt, EitherSink, ExecPlan, Executor, FindSpec, OccurrencePin,
    PreparedQuery, ProjectionSink, ResolveMemo, Schema, ValueType, ViewMemo, PARKED_SLOTS,
};

use crate::error::Result;
use crate::exec::dispatch::classify;
use crate::image::cache::ImageCache;
use crate::image::view::View;
use crate::ir::normalize::normalize;
use crate::ir::validate::validate;
use crate::ir::{AggOp, FindTerm, Query};
use crate::obs;
use crate::plan::fj::{binary2fj, factor};
use crate::plan::planner::plan as plan_order;
use crate::storage::env::ReadTxn;
use crate::storage::read;

/// Prepares a query: the one-time pipeline, allocation-sanctioned.
///
/// # Errors
///
/// `Validation` at the IR boundary; planner caps; `Lmdb`/`Corruption` from
/// the statistics reads.
///
/// # Panics
///
/// Only on programmer-invariant violations (`binary2fj` + `factor`
/// construct valid plans by construction).
#[allow(clippy::too_many_lines)] // the pipeline's stages in order; each
                                 // is one span-wrapped step
pub(crate) fn prepare<'s, S>(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &'s Schema,
    query: &Query,
) -> Result<PreparedQuery<'s, S>> {
    let _prepare = obs::span(obs::names::PREPARE, obs::Category::Prepare);
    let witness = {
        let _s = obs::span(obs::names::VALIDATE, obs::Category::Prepare);
        validate(schema, query)?
    };
    let mut normalized = {
        let _s = obs::span(obs::names::NORMALIZE, obs::Category::Prepare);
        normalize(schema, &witness)
    };
    // The chase (plan/chase.rs): containment-implied occurrence
    // elimination, after normalization and before statistics and the DP
    // (docs/architecture/40-execution.md planner placement). Eliminated
    // occurrences keep their ids and are skipped by every downstream
    // path through the one participates-in-planning predicate.
    crate::plan::chase::chase(&mut normalized, schema, &query.finds);

    // Classification first: a guard probe needs no statistics or planning.
    let classified = {
        let _s = obs::span(obs::names::CLASSIFY, obs::Category::Prepare);
        classify(&normalized, schema)
    };
    // The staleness pin record (`staleness.rs`): the statistics below,
    // kept instead of dropped. Stays empty for guard probes — they read
    // no statistics, so there is nothing to drift.
    let mut pins = Vec::new();
    let exec_plan = if let Some(guard) = classified {
        ExecPlan::GuardProbe(guard)
    } else {
        // Per-occurrence input estimates (docs/architecture/40-execution.md): row counters
        // shaped by the selectivity ladder — key-exact counts,
        // resident-image distinct counts (peek only: prepare never
        // builds an image for statistics), documented bounds and floors.
        // Participating occurrences only: negated occurrences enter no
        // DP state and chase-eliminated occurrences left planning
        // entirely, so neither earns a statistics read — and, by the
        // same token, neither earns a pin.
        let mut stats_span = obs::span(obs::names::STATS, obs::Category::Prepare);
        let mut stats = Vec::with_capacity(normalized.occurrences.len());
        for occurrence in normalized
            .occurrences
            .iter()
            .filter(|o| o.role.participates())
        {
            let rows = read::row_count(txn, occurrence.relation)?;
            let occ_stats =
                crate::plan::selectivity::occurrence_stats(txn, cache, schema, occurrence, rows)?;
            pins.push(OccurrencePin {
                occ_id: occurrence.occ_id,
                relation: occurrence.relation,
                rows,
                survivors: (!occurrence.filters.is_empty()).then_some(occ_stats.rows),
            });
            stats.push(occ_stats);
        }
        stats_span.set_args(stats.len() as u64, 0);
        stats_span.end();
        let order = {
            let _s = obs::span(obs::names::PLAN_DP, obs::Category::Prepare);
            plan_order(&normalized, schema, &stats)
        };
        let lower_span = obs::span(obs::names::LOWER, obs::Category::Prepare);
        let mut fj = binary2fj(&normalized, &order);
        factor(&mut fj);
        // Group key for projections; every variable for aggregates —
        // skip-illegality under a fold is encoded in the bits themselves
        // (`ValidatedQuery::sink_vars`).
        let sink_vars = witness.sink_vars();
        let validated = crate::plan::fj::validate(
            &fj,
            &normalized,
            schema,
            order.estimates.clone(),
            &sink_vars,
        )
        .expect("binary2fj + factor construct valid plans");
        lower_span.end();
        ExecPlan::FreeJoin(validated)
    };

    let finds = find_specs(query, &witness, &exec_plan);

    // Dense param typing for bind-time checks (validation rejected gaps
    // — jointly across value and mask params — so the id-ordered merge
    // is positional). A set param records its element type plus the
    // set-ness bit — bind expects a slice for it. The point-ness bit
    // marks element-typed params at interval positions: bind rejects
    // their domain ceiling (the point-domain law). Mask params (`Allen`
    // mask positions) are absent from the witness's value typing and
    // fill their slots with the mask shape.
    let value_types: std::collections::BTreeMap<crate::ir::ParamId, &ValueType> =
        witness.param_types().collect();
    let param_count = value_types.len() + witness.mask_params().len();
    let mut param_types = Vec::with_capacity(param_count);
    let mut param_is_set = Vec::with_capacity(param_count);
    let mut param_is_point = Vec::with_capacity(param_count);
    for idx in 0..param_count {
        let id = crate::ir::ParamId(u16::try_from(idx).expect("param ids fit u16"));
        param_types.push(value_types.get(&id).map_or_else(
            || {
                debug_assert!(witness.mask_params().contains(&id), "dense param ids");
                super::ParamShape::AllenMask
            },
            |ty| super::ParamShape::Value((*ty).clone()),
        ));
        param_is_set.push(witness.set_params().contains(&id));
        param_is_point.push(witness.point_params().contains(&id));
    }

    // Binding slots are WORDS: an interval variable holds two (the
    // SlotWidth layout) — `slot_count`, never the variable count.
    let (executor, slot_count, occurrence_count) = match &exec_plan {
        ExecPlan::FreeJoin(plan) => (
            Some(Executor::new(plan)),
            plan.slot_count(),
            plan.occurrences().len(),
        ),
        ExecPlan::GuardProbe(guard) => (None, guard.slot_count(), 1),
    };

    // BUILD_COLTS is pure column-schema construction since the unbound-
    // views cutover: prepare provably never touches an image (the stats
    // phase peeks, never builds), so a prepared query pins nothing.
    let memo = {
        let _s = obs::span(obs::names::BUILD_COLTS, obs::Category::Prepare);
        build_view_memo(&exec_plan)
    };
    // Sink presizing: the last node's planner
    // estimate bounds the binding stream the sink consumes.
    let output_hint = match &exec_plan {
        ExecPlan::FreeJoin(plan) => {
            usize::try_from(plan.estimates().last().copied().unwrap_or(0).min(1 << 21))
                .expect("clamped")
        }
        ExecPlan::GuardProbe(_) => 1,
    };
    let sink = make_sink(
        &finds,
        slot_count,
        exec_plan.distinct_bindings(),
        output_hint,
    );

    let all_words = finds
        .iter()
        .all(|(_, ty)| !matches!(ty, ValueType::String | ValueType::Bytes));
    let guard_finds = guard_find_table(&exec_plan, &finds);
    Ok(PreparedQuery {
        schema,
        env_instance: txn.env_instance(),
        plan: exec_plan,
        executor,
        bindings: Bindings::new(slot_count),
        finds,
        param_types,
        param_is_set,
        param_is_point,
        resolved_params: Vec::new(),
        missed_params: Vec::new(),
        resolved_filters: vec![Vec::new(); occurrence_count],
        resolved_selections: vec![Vec::new(); occurrence_count],
        memo,
        sink,
        row_scratch: Vec::new(),
        all_words,
        guard_finds,
        resolve_memo: ResolveMemo::new(),
        guard_key: Vec::new(),
        pinned: pins.into_boxed_slice(),
        marker: std::marker::PhantomData,
    })
}

/// COLT sources with their fixed column schemas over [`View::Unbound`]:
/// prepare touches no image — the first execution binds every view via
/// the ordinary memo-miss path (a `None` generation never matches),
/// paying the image build exactly where a cold execution already pays
/// it. Pure column-schema construction; nothing here can fail.
fn build_view_memo(exec_plan: &ExecPlan) -> ViewMemo {
    let mut memo = ViewMemo {
        colts: Vec::new(),
        generation: Vec::new(),
        filters: Vec::new(),
        parked: Vec::new(),
        spare_buffers: Vec::new(),
        tick: 0,
    };
    let ExecPlan::FreeJoin(plan) = exec_plan else {
        return memo; // guard probes never touch views
    };
    for occurrence in plan.occurrences() {
        // Field→column through the span map (docs/architecture/
        // 50-storage.md image layout): an interval field contributes its
        // start/end column pair, and every field after one is shifted —
        // spans, never raw field indices.
        let columns_of = |field: crate::schema::FieldId| -> Vec<usize> {
            let span = occurrence.spans[usize::from(field.0)];
            let first = usize::from(span.first_column);
            match span.width {
                crate::image::ColumnWidth::WordPair => vec![first, first + 1],
                crate::image::ColumnWidth::Word | crate::image::ColumnWidth::Byte => vec![first],
            }
        };
        let columns: Vec<Vec<usize>> = occurrence
            .trie_schema
            .iter()
            .map(|level| {
                level
                    .iter()
                    .flat_map(|var| {
                        let (field, _) = occurrence
                            .vars
                            .iter()
                            .find(|(_, v)| v == var)
                            .expect("plan vars come from the occurrence");
                        columns_of(*field)
                    })
                    .collect()
            })
            .collect();
        // Selection levels: columns plus set-ness — a `ParamSet` value
        // marks a set-bound level, probed once per element with the
        // survivor union (docs/architecture/40-execution.md, § selection
        // levels; set-ness is a plan fact, never per-execution data).
        let selections: Vec<crate::exec::colt::SelectionLevel> = occurrence
            .selections
            .iter()
            .map(|s| crate::exec::colt::SelectionLevel {
                columns: columns_of(s.field),
                set: matches!(s.value, crate::image::view::Const::ParamSet(_)),
            })
            .collect();
        memo.colts
            .push(Colt::new(View::Unbound, &selections, columns));
        memo.generation.push(None);
        memo.filters.push(Vec::new());
        memo.parked.push((0..PARKED_SLOTS).map(|_| None).collect());
        memo.spare_buffers.push(Vec::new());
    }
    memo
}

/// Derives per-find output specs (slot spans + result types) from the
/// witness and the classified plan. Slots and widths both come from the
/// plan's binding-slot layout (`slot_of`/`width_of` — the `SlotWidth` map):
/// an interval variable's find spans two words, and no consumer assumes
/// width 1.
fn find_specs(
    query: &Query,
    witness: &crate::ir::validate::ValidatedQuery,
    exec_plan: &ExecPlan,
) -> Vec<(FindSpec, ValueType)> {
    query
        .finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => (
                FindSpec::Var {
                    slot: exec_plan.slot_of(*var),
                    width: exec_plan.width_of(*var),
                },
                witness.var_type(*var).clone(),
            ),
            FindTerm::Aggregate { op, over } => match op {
                // Arg-restriction: the carry's span plus the shared key
                // slot (orderable — validated U64/I64, one word).
                AggOp::ArgMax { key } | AggOp::ArgMin { key } => {
                    let carry = over.expect("validated: Arg carries a variable");
                    (
                        FindSpec::Arg {
                            slot: exec_plan.slot_of(carry),
                            width: exec_plan.width_of(carry),
                            key_slot: exec_plan.slot_of(*key),
                            max: matches!(op, AggOp::ArgMax { .. }),
                        },
                        witness.var_type(carry).clone(),
                    )
                }
                AggOp::Sum | AggOp::Min | AggOp::Max | AggOp::Count | AggOp::CountDistinct => {
                    let (over_slot, over_width, over_ty) = match over {
                        Some(var) => (
                            Some(exec_plan.slot_of(*var)),
                            exec_plan.width_of(*var),
                            witness.var_type(*var).clone(),
                        ),
                        None => (None, 1, ValueType::U64), // Count
                    };
                    let (fold, result_ty) = match op {
                        AggOp::Sum => (crate::exec::sink::FoldOp::Sum, over_ty.clone()),
                        AggOp::Min => (crate::exec::sink::FoldOp::Min, over_ty.clone()),
                        AggOp::Max => (crate::exec::sink::FoldOp::Max, over_ty.clone()),
                        AggOp::Count => (crate::exec::sink::FoldOp::Count, ValueType::U64),
                        AggOp::CountDistinct => {
                            (crate::exec::sink::FoldOp::CountDistinct, ValueType::U64)
                        }
                        AggOp::ArgMax { .. } | AggOp::ArgMin { .. } => {
                            unreachable!("handled above")
                        }
                    };
                    (
                        FindSpec::Agg {
                            op: fold,
                            over_slot,
                            over_width,
                            signed: matches!(over_ty, ValueType::I64),
                        },
                        result_ty,
                    )
                }
            },
        })
        .collect()
}

/// The guard fast lane's find table: `Some` for
/// guard plans whose finds are all plain variables.
fn guard_find_table(
    exec_plan: &ExecPlan,
    finds: &[(FindSpec, ValueType)],
) -> Option<Vec<(crate::schema::FieldId, ValueType)>> {
    match exec_plan {
        ExecPlan::GuardProbe(guard) => finds
            .iter()
            .map(|(spec, ty)| match spec {
                FindSpec::Var { slot, .. } => {
                    let var = guard
                        .vars
                        .iter()
                        .find(|v| v.slot == *slot)
                        .expect("find slots come from the guard plan's layout");
                    Some((var.field, ty.clone()))
                }
                // aggregate guards keep the sink path
                FindSpec::Agg { .. } | FindSpec::Arg { .. } => None,
            })
            .collect::<Option<Vec<_>>>(),
        ExecPlan::FreeJoin(_) => None,
    }
}

/// Builds the sink matching the find shape (the variant is fixed per
/// prepared query — an enum, not `dyn`).
fn make_sink(
    finds: &[(FindSpec, ValueType)],
    slot_count: usize,
    distinct: bool,
    hint: usize,
) -> EitherSink {
    let all_plain = finds
        .iter()
        .all(|(spec, _)| matches!(spec, FindSpec::Var { .. }));
    if all_plain {
        // Word-level slot expansion through the layout map: an interval
        // find contributes its two consecutive slots, so the projection
        // sink's rows are word rows the finalize pass re-assembles by
        // find type.
        let slots = finds
            .iter()
            .flat_map(|(spec, _)| match spec {
                FindSpec::Var { slot, width } => *slot..slot + width,
                FindSpec::Agg { .. } | FindSpec::Arg { .. } => unreachable!("no aggregates here"),
            })
            .collect();
        EitherSink::Projection(ProjectionSink::with_capacity_hint(slots, hint))
    } else {
        EitherSink::Aggregate(Box::new(AggregateSink::with_capacity_hint(
            finds.iter().map(|(spec, _)| *spec).collect(),
            slot_count,
            distinct,
            hint,
        )))
    }
}
