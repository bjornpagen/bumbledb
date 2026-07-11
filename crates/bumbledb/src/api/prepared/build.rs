use super::{
    AggregateSink, Bindings, Colt, EitherSink, ExecPlan, Executor, FindSpec, OccurrencePin,
    PreparedQuery, PreparedRule, ProjectionSink, ResolveMemo, Schema, ValueType, ViewMemo,
    PARKED_SLOTS,
};

use crate::error::Result;
use crate::exec::dispatch::classify;
use crate::image::cache::ImageCache;
use crate::image::view::View;
use crate::ir::normalize::{normalize, NormalizedQuery};
use crate::ir::validate::{validate, RuleWitness};
use crate::ir::{AggOp, FindTerm, Query};
use crate::obs;
use crate::plan::fj::{binary2fj, factor, provably_disjoint_rules, DisjointWitness};
use crate::plan::planner::plan as plan_order;
use crate::storage::env::ReadTxn;
use crate::storage::read;

/// Prepares a query: the one-time pipeline, allocation-sanctioned.
/// Validation and normalization see the whole program; everything after —
/// statistics, the DP, lowering, plan validation — runs **per rule**, and
/// the prepared query carries one [`PreparedRule`] per rule under one
/// head-owned sink configuration.
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
    let normalized = {
        let _s = obs::span(obs::names::NORMALIZE, obs::Category::Prepare);
        normalize(schema, &witness)
    };

    // The disjointness proof runs pre-chase (the rewrite never changes
    // the denotation, so the proof stands), and pre-deletion: pairwise
    // over a superset holds over whichever rules survive below.
    let disjoint_rules = disjointness(&witness, &normalized, schema);

    let (survivors, subsumed) = chase_program(normalized, &witness, schema);
    // A program subsumption shrank to one rule has no pair left to
    // prove (the stats surface's single-rule contract).
    let disjoint_rules = (survivors.len() > 1).then_some(disjoint_rules).flatten();

    let mut rules = Vec::with_capacity(survivors.len());
    let mut column_types = Vec::new();
    for (position, (rule_idx, normalized_rule)) in survivors.into_iter().enumerate() {
        let rule = witness.rule(rule_idx);
        let (prepared, types) = prepare_rule(txn, cache, schema, &rule, &normalized_rule)?;
        if position == 0 {
            // The head's result-type row — validation pinned the
            // positional alignment, so every rule computes this same row.
            column_types = types;
        } else {
            // The head-alignment re-check after subsumption deletion:
            // deleting a rule never changes the head, and every survivor
            // still computes the pinned row.
            debug_assert_eq!(
                types, column_types,
                "survivors compute the head's pinned type row"
            );
        }
        rules.push(prepared);
    }

    let (param_types, param_is_set, param_is_point) = param_tables(&witness);

    // The one sink configuration — head-owned shape (projection vs
    // aggregate, arity, distinctness), built aimed at rule 0's layout
    // and re-aimed per rule by the rule loop. Presized against the
    // rules' worst estimate (one sink hears every rule). Dedup is
    // per-query-shape: a single-rule aggregate elides its seen-set under
    // the plan's distinct-bindings proof; a multi-rule one keys head
    // projections and elides only under the rule-disjointness
    // composition below — correct first, elided when proven.
    let output_hint = rules
        .iter()
        .map(|rule| match &rule.plan {
            // Sink presizing: the last node's planner estimate bounds
            // the binding stream the sink consumes.
            ExecPlan::FreeJoin(plan) => {
                usize::try_from(plan.estimates().last().copied().unwrap_or(0).min(1 << 21))
                    .expect("clamped")
            }
            ExecPlan::GuardProbe(_) => 1,
        })
        .max()
        .expect("at least one rule");
    let union = rules.len() > 1;
    let union_elided = union && disjoint_rules.is_some() && union_elision(&rules);
    let first = &rules[0];
    let sink = make_sink(
        &first.finds,
        first.plan.slot_count(),
        if union {
            union_elided
        } else {
            first.plan.distinct_bindings()
        },
        union,
        union && disjoint_rules.is_some(),
        output_hint,
    );
    // The rule-shared binding-slot scratch, sized at the rules'
    // high-water so the per-rule resize never allocates.
    let bindings = Bindings::new(
        rules
            .iter()
            .map(|rule| rule.plan.slot_count())
            .max()
            .expect("at least one rule"),
    );

    let all_words = column_types
        .iter()
        .all(|ty| !matches!(ty, ValueType::String | ValueType::Bytes));
    Ok(PreparedQuery {
        schema,
        env_instance: txn.env_instance(),
        disjoint_rules,
        union_elided,
        subsumed,
        rules,
        column_types,
        param_types,
        param_is_set,
        param_is_point,
        resolved_params: Vec::new(),
        missed_params: Vec::new(),
        sink,
        bindings,
        row_scratch: Vec::new(),
        all_words,
        resolve_memo: ResolveMemo::new(),
        guard_key: Vec::new(),
        marker: std::marker::PhantomData,
    })
}

/// Dense param typing for bind-time checks (validation rejected gaps —
/// jointly across value and mask params, across all rules — so the
/// id-ordered merge is positional): per param, its expected shape,
/// set-ness, and point-ness. A set param records its element type plus
/// the set-ness bit — bind expects a slice for it. The point-ness bit
/// marks element-typed params at interval positions: bind rejects their
/// domain ceiling (the point-domain law). Mask params (`Allen` mask
/// positions) are absent from the witness's value typing and fill their
/// slots with the mask shape.
fn param_tables(
    witness: &crate::ir::validate::ValidatedQuery,
) -> (Vec<super::ParamShape>, Vec<bool>, Vec<bool>) {
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
    (param_types, param_is_set, param_is_point)
}

/// The theory's program rewrite (`plan/chase.rs`): the elimination
/// fixpoint per rule, independently — after normalization and before
/// statistics and the DP (docs/architecture/40-execution.md planner
/// placement), with no cross-rule state; a rule shrinking below its
/// cover requirements re-validates like any rule (the per-rule pipeline
/// re-runs plan validation regardless). Eliminated occurrences keep
/// their ids and are skipped by every downstream path through the one
/// participates-in-planning predicate. Then rule subsumption: a rule
/// whose post-elimination body a sibling contains modulo eliminated
/// filters is deleted — the union loses nothing. Returns the surviving
/// rules with their lowered-rule indices plus the deletion record (the
/// EXPLAIN surface).
fn chase_program(
    mut normalized: Vec<NormalizedQuery>,
    witness: &crate::ir::validate::ValidatedQuery,
    schema: &Schema,
) -> (
    Vec<(usize, NormalizedQuery)>,
    Vec<crate::api::stats::SubsumedRule>,
) {
    for (rule_idx, normalized_rule) in normalized.iter_mut().enumerate() {
        crate::plan::chase::chase(
            normalized_rule,
            schema,
            &witness.rule(rule_idx).rule().finds,
        );
    }
    let finds: Vec<&[FindTerm]> = (0..normalized.len())
        .map(|idx| witness.rule(idx).rule().finds.as_slice())
        .collect();
    let subsumed: Vec<crate::api::stats::SubsumedRule> =
        crate::plan::chase::subsume(&normalized, &finds)
            .into_iter()
            .map(|deletion| crate::api::stats::SubsumedRule {
                rule: u16::try_from(deletion.rule).expect("rule count fits u16"),
                by: u16::try_from(deletion.by).expect("rule count fits u16"),
            })
            .collect();
    let survivors = normalized
        .into_iter()
        .enumerate()
        .filter(|(idx, _)| !subsumed.iter().any(|s| usize::from(s.rule) == *idx))
        .collect();
    (survivors, subsumed)
}

/// The per-rule pipeline tail: classify → statistics → DP → lowering →
/// plan validation — the conjunctive query's pipeline, with zero
/// changes, over one already-chased rule. Returns the rule's prepared
/// artifact and its result-type row (the head's; identical across
/// rules).
fn prepare_rule(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
) -> Result<(PreparedRule, Vec<ValueType>)> {
    // Classification first: a guard probe needs no statistics or planning.
    let classified = {
        let _s = obs::span(obs::names::CLASSIFY, obs::Category::Prepare);
        classify(normalized, schema)
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
            plan_order(normalized, schema, &stats)
        };
        let lower_span = obs::span(obs::names::LOWER, obs::Category::Prepare);
        let mut fj = binary2fj(normalized, &order);
        factor(&mut fj);
        // Group key for projections; every variable for aggregates —
        // skip-illegality under a fold is encoded in the bits themselves
        // (`RuleWitness::sink_vars`).
        let sink_vars = rule.sink_vars();
        let validated =
            crate::plan::fj::validate(&fj, normalized, schema, order.estimates.clone(), &sink_vars)
                .expect("binary2fj + factor construct valid plans");
        lower_span.end();
        ExecPlan::FreeJoin(validated)
    };

    let finds = find_specs(rule, &exec_plan);

    let (executor, occurrence_count) = match &exec_plan {
        ExecPlan::FreeJoin(plan) => (Some(Executor::new(plan)), plan.occurrences().len()),
        ExecPlan::GuardProbe(_) => (None, 1),
    };

    // BUILD_COLTS is pure column-schema construction since the unbound-
    // views cutover: prepare provably never touches an image (the stats
    // phase peeks, never builds), so a prepared query pins nothing.
    let memo = {
        let _s = obs::span(obs::names::BUILD_COLTS, obs::Category::Prepare);
        build_view_memo(&exec_plan)
    };
    let guard_finds = guard_find_table(&exec_plan, &finds);
    let (specs, types) = finds.into_iter().unzip();
    Ok((
        PreparedRule {
            plan: exec_plan,
            executor,
            finds: specs,
            resolved_filters: vec![Vec::new(); occurrence_count],
            resolved_selections: vec![Vec::new(); occurrence_count],
            memo,
            guard_finds,
            pinned: pins.into_boxed_slice(),
        },
        types,
    ))
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

/// Derives one rule's per-find output specs (slot spans + result types)
/// from its witness slice and classified plan. Slots and widths both come
/// from the rule's binding-slot layout (`slot_of`/`width_of` — the
/// `SlotWidth` map): an interval variable's find spans two words, and no
/// consumer assumes width 1. The types are the head's (validation aligned
/// every rule's row); the specs are this rule's.
fn find_specs(rule: &RuleWitness<'_>, exec_plan: &ExecPlan) -> Vec<(FindSpec, ValueType)> {
    rule.rule()
        .finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => (
                FindSpec::Var {
                    slot: exec_plan.slot_of(*var),
                    width: exec_plan.width_of(*var),
                },
                rule.var_type(*var).clone(),
            ),
            // The measure positions: one u64 word computed from the
            // interval variable's two-slot span (the sinks own the
            // subtraction and the ray check — `exec::sink`).
            FindTerm::Duration(var) => (
                FindSpec::Duration {
                    slot: exec_plan.slot_of(*var),
                },
                ValueType::U64,
            ),
            FindTerm::AggregateDuration { op, over } => (
                FindSpec::AggDuration {
                    op: match op {
                        AggOp::Sum => crate::exec::sink::FoldOp::Sum,
                        AggOp::Min => crate::exec::sink::FoldOp::Min,
                        AggOp::Max => crate::exec::sink::FoldOp::Max,
                        AggOp::Count
                        | AggOp::CountDistinct
                        | AggOp::ArgMax { .. }
                        | AggOp::ArgMin { .. } => {
                            unreachable!("validated: measure folds are Sum/Min/Max")
                        }
                    },
                    slot: exec_plan.slot_of(*over),
                },
                ValueType::U64,
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
                        rule.var_type(carry).clone(),
                    )
                }
                AggOp::Sum | AggOp::Min | AggOp::Max | AggOp::Count | AggOp::CountDistinct => {
                    let (over_slot, over_width, over_ty) = match over {
                        Some(var) => (
                            Some(exec_plan.slot_of(*var)),
                            exec_plan.width_of(*var),
                            rule.var_type(*var).clone(),
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
                // aggregate and measure guards keep the sink path
                FindSpec::Agg { .. }
                | FindSpec::Arg { .. }
                | FindSpec::Duration { .. }
                | FindSpec::AggDuration { .. } => None,
            })
            .collect::<Option<Vec<_>>>(),
        ExecPlan::FreeJoin(_) => None,
    }
}

/// The rule-disjointness proof (docs/architecture/40-execution.md § set
/// semantics) — the exclusivity theorem's third consumer, run over the
/// whole program before the pipeline goes per-rule (the chase rewrites
/// occurrences but never the denotation, so the pre-chase proof stands).
/// Single-rule programs have no pair to prove and no union to elide.
fn disjointness(
    witness: &crate::ir::validate::ValidatedQuery,
    normalized: &[NormalizedQuery],
    schema: &Schema,
) -> Option<DisjointWitness> {
    (normalized.len() > 1)
        .then(|| {
            let inputs: Vec<(&[FindTerm], &NormalizedQuery)> = normalized
                .iter()
                .enumerate()
                .map(|(idx, rule)| (witness.rule(idx).rule().finds.as_slice(), rule))
                .collect();
            provably_disjoint_rules(&inputs, schema)
        })
        .flatten()
}

/// The union elision's per-rule legs (docs/architecture/40-execution.md
/// § set semantics), composed on top of the disjointness proof: distinct
/// bindings (each binding emitted once) and a head projection reading
/// every slot (distinct bindings ⇒ distinct head tuples) make each
/// rule's dedup-key stream duplicate-free, and the witness forbids
/// cross-rule collisions — so the union seen-set guards nothing and is
/// deleted at plan time.
fn union_elision(rules: &[PreparedRule]) -> bool {
    rules.iter().all(|rule| {
        // A measure position breaks the within-rule leg outright:
        // distinct bindings project through a NON-injective map (two
        // distinct intervals may share one measure), so the dedup-key
        // stream is not proven duplicate-free — the seen-set stays.
        let no_measures = rule.finds.iter().all(|find| {
            !matches!(
                find,
                FindSpec::Duration { .. } | FindSpec::AggDuration { .. }
            )
        });
        no_measures
            && rule.plan.distinct_bindings()
            && head_reads_every_slot(&rule.finds, rule.plan.slot_count())
    })
}

/// Whether the head projection reads every binding slot — the
/// within-rule leg of the union elision: with every slot read, distinct
/// bindings project to distinct head tuples, so a rule's dedup-key
/// stream inherits the distinct-bindings proof whole. A rule binding an
/// existential the head never reads fails here and keeps the seen-set.
fn head_reads_every_slot(finds: &[FindSpec], slot_count: usize) -> bool {
    let mut read = vec![false; slot_count];
    for find in finds {
        let (slot, width) = match find {
            FindSpec::Var { slot, width } => (*slot, *width),
            FindSpec::Agg {
                over_slot: Some(slot),
                over_width,
                ..
            } => (*slot, *over_width),
            // A measure reads its interval variable's two slots — but
            // `union_elision` already refused measure heads, so the
            // coverage answer here is moot; recorded for the read set.
            FindSpec::Duration { slot } | FindSpec::AggDuration { slot, .. } => (*slot, 2),
            // The nullary Count reads nothing; Arg never crosses rules
            // (validation), so a multi-rule head cannot carry it.
            FindSpec::Agg {
                over_slot: None, ..
            }
            | FindSpec::Arg { .. } => continue,
        };
        read[slot..slot + width].fill(true);
    }
    read.iter().all(|slot_read| *slot_read)
}

/// Builds the sink matching the head shape (the variant is fixed per
/// prepared query — an enum, not `dyn`), aimed at rule 0's binding
/// layout. `union` is the multi-rule regime (head-projection dedup
/// keys); `distinct` is the proof the dedup-key stream is duplicate-free
/// (single-rule: the plan flag; multi-rule: the rule-disjointness
/// composition), which elides the aggregate seen-set; `disjoint` is the
/// bare disjointness proof, which drops the projection sink's cross-rule
/// guard (per-rule dedup stays — docs/architecture/40-execution.md § set
/// semantics).
fn make_sink(
    finds: &[FindSpec],
    slot_count: usize,
    distinct: bool,
    union: bool,
    disjoint: bool,
    hint: usize,
) -> EitherSink {
    let all_plain = finds
        .iter()
        .all(|spec| matches!(spec, FindSpec::Var { .. } | FindSpec::Duration { .. }));
    if all_plain {
        // Word-level source expansion through the layout map: an
        // interval find contributes its two consecutive slots and a
        // measure find one computed word, so the projection sink's rows
        // are word rows the finalize pass re-assembles by find type.
        EitherSink::Projection(ProjectionSink::with_capacity_hint(
            crate::exec::sink::sources_of(finds),
            hint,
            union && disjoint,
        ))
    } else {
        EitherSink::Aggregate(Box::new(AggregateSink::with_capacity_hint(
            finds.to_vec(),
            slot_count,
            distinct,
            union,
            hint,
        )))
    }
}
