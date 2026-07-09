use super::{
    AggregateSink, Bindings, Colt, EitherSink, ExecPlan, Executor, FindSpec, PreparedQuery,
    ProjectionSink, ResolveMemo, Schema, ValueType, ViewMemo, PARKED_SLOTS,
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
pub(crate) fn prepare<'s>(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &'s Schema,
    query: &Query,
) -> Result<PreparedQuery<'s>> {
    let _prepare = obs::span(obs::names::PREPARE, obs::Category::Prepare);
    let witness = {
        let _s = obs::span(obs::names::VALIDATE, obs::Category::Prepare);
        validate(schema, query)?
    };
    let normalized = {
        let _s = obs::span(obs::names::NORMALIZE, obs::Category::Prepare);
        normalize(schema, &witness)
    };

    // Classification first: a guard probe needs no statistics or planning.
    let classified = {
        let _s = obs::span(obs::names::CLASSIFY, obs::Category::Prepare);
        classify(&normalized, schema)
    };
    let exec_plan = if let Some(guard) = classified {
        ExecPlan::GuardProbe(guard)
    } else {
        // Per-occurrence input estimates (docs/architecture/30-execution.md): row counters
        // shaped by the selectivity ladder — schema-exact uniques,
        // resident-image distinct counts (peek only: prepare never
        // builds an image for statistics), documented bounds and floors.
        let mut stats_span = obs::span(obs::names::STATS, obs::Category::Prepare);
        let mut stats = Vec::with_capacity(normalized.occurrences.len());
        for occurrence in &normalized.occurrences {
            let rows = read::row_count(txn, occurrence.relation)?;
            stats.push(crate::plan::selectivity::occurrence_stats(
                txn, cache, schema, occurrence, rows,
            )?);
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
        // (hardening PRD 05; `ValidatedQuery::sink_vars`).
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

    // Dense param typing for bind-time checks (validation rejected gaps,
    // so the id-ordered iteration is positional).
    let param_types: Vec<ValueType> = witness.param_types().map(|(_, ty)| ty.clone()).collect();

    let (executor, slot_count, occurrence_count) = match &exec_plan {
        ExecPlan::FreeJoin(plan) => (
            Some(Executor::new(plan)),
            plan.slots().len(),
            plan.occurrences().len(),
        ),
        ExecPlan::GuardProbe(guard) => (None, guard.vars.len(), 1),
    };

    // BUILD_COLTS is pure column-schema construction since the unbound-
    // views cutover: prepare provably never touches an image (the stats
    // phase peeks, never builds), so a prepared query pins nothing.
    let memo = {
        let _s = obs::span(obs::names::BUILD_COLTS, obs::Category::Prepare);
        build_view_memo(&exec_plan)
    };
    // Sink presizing (docs/perf/ PRD 06): the last node's planner
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
        _not_sync: std::marker::PhantomData,
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
        let columns: Vec<Vec<usize>> = occurrence
            .trie_schema
            .iter()
            .map(|level| {
                level
                    .iter()
                    .map(|var| {
                        let (field, _) = occurrence
                            .vars
                            .iter()
                            .find(|(_, v)| v == var)
                            .expect("plan vars come from the occurrence");
                        usize::from(field.0)
                    })
                    .collect()
            })
            .collect();
        let selections: Vec<usize> = occurrence
            .selections
            .iter()
            .map(|s| usize::from(s.field.0))
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

/// Derives per-find output specs (slots + result types) from the witness
/// and the classified plan.
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
                },
                witness.var_type(*var).clone(),
            ),
            FindTerm::Aggregate { op, over } => {
                let (over_slot, ty) = match over {
                    Some(var) => (
                        Some(exec_plan.slot_of(*var)),
                        witness.var_type(*var).clone(),
                    ),
                    None => (None, ValueType::U64), // Count
                };
                (
                    FindSpec::Agg {
                        op: *op,
                        over_slot,
                        signed: matches!(ty, ValueType::I64),
                    },
                    if *op == AggOp::Count {
                        ValueType::U64
                    } else {
                        ty
                    },
                )
            }
        })
        .collect()
}

/// The guard fast lane's find table (docs/perf/ PRD 11): `Some` for
/// guard plans whose finds are all plain variables.
fn guard_find_table(
    exec_plan: &ExecPlan,
    finds: &[(FindSpec, ValueType)],
) -> Option<Vec<(crate::schema::FieldId, ValueType)>> {
    match exec_plan {
        ExecPlan::GuardProbe(guard) => finds
            .iter()
            .map(|(spec, ty)| match spec {
                FindSpec::Var { slot } => Some((guard.vars[*slot].0, ty.clone())),
                FindSpec::Agg { .. } => None, // aggregate guards keep the sink path
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
    let has_aggregates = finds
        .iter()
        .any(|(spec, _)| matches!(spec, FindSpec::Agg { .. }));
    if has_aggregates {
        EitherSink::Aggregate(Box::new(AggregateSink::with_capacity_hint(
            finds.iter().map(|(spec, _)| *spec).collect(),
            slot_count,
            distinct,
            hint,
        )))
    } else {
        EitherSink::Projection(ProjectionSink::with_capacity_hint(
            finds
                .iter()
                .map(|(spec, _)| match spec {
                    FindSpec::Var { slot } => *slot,
                    FindSpec::Agg { .. } => unreachable!("no aggregates here"),
                })
                .collect(),
            hint,
        ))
    }
}
