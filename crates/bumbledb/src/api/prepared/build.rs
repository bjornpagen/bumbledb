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

    // The head's result-type row, derived from the witness alone —
    // validation pinned the positional alignment, so rule 0 speaks for
    // every rule, it exists even when every rule below dies, and every
    // prepared rule re-derives exactly it (the debug assert).
    let column_types: Vec<ValueType> = result_types(&witness.rule(0));
    let mut rules = Vec::with_capacity(survivors.len());
    let mut dead = Vec::new();
    for (rule_idx, normalized_rule) in survivors {
        // Rule death (ir/normalize/fold.rs): a statically-empty rule is
        // deleted here — no statistics read, no DP, no plan; the union
        // loses nothing because the rule denotes the empty set. The
        // record keeps the killing predicate for EXPLAIN.
        if let Some(reason) = &normalized_rule.dead {
            dead.push(crate::api::stats::DeadRule {
                rule: u16::try_from(rule_idx).expect("rule count fits u16"),
                rendered: reason.clone(),
            });
            continue;
        }
        let rule = witness.rule(rule_idx);
        let (prepared, types) = prepare_rule(txn, cache, schema, &rule, &normalized_rule)?;
        debug_assert_eq!(
            types, column_types,
            "live rules compute the head's pinned type row"
        );
        rules.push(prepared);
    }
    // A program deletion (subsumption or rule death) shrank to at most
    // one live rule has no pair left to prove (the stats surface's
    // single-rule contract; pairwise over a superset held regardless).
    let disjoint_rules = (rules.len() > 1).then_some(disjoint_rules).flatten();
    if rules.is_empty() {
        // Every rule died: the program is stage-2-known empty and
        // prepares to the one `ExecPlan::Empty` artifact — execution
        // binds params (errors surface), then touches nothing
        // (docs/architecture/40-execution.md, § access paths).
        rules.push(empty_rule());
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
            // Nothing ever emits under the empty plan.
            ExecPlan::Empty => 0,
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

    // The byte-heap types keep the resolving finalize: String resolves
    // through the dictionary; a bytes<N> find re-assembles its slot words
    // into the byte heap (no dictionary — inline values).
    let all_words = column_types
        .iter()
        .all(|ty| !matches!(ty, ValueType::String | ValueType::FixedBytes { .. }));
    let unresolved_literals = rules.iter().map(pending_literals).sum();
    Ok(PreparedQuery {
        schema,
        env_instance: txn.env_instance(),
        disjoint_rules,
        union_elided,
        subsumed,
        dead,
        rules,
        column_types,
        param_types,
        param_is_set,
        param_is_point,
        resolved_params: Vec::new(),
        unresolved_literals,
        missed_params: Vec::new(),
        sink,
        bindings,
        row_scratch: Vec::new(),
        all_words,
        resolve_memo: ResolveMemo::new(),
        guard_key: Vec::new(),
        rendered: crate::ir::render::render(schema, query),
        marker: std::marker::PhantomData,
    })
}

/// The head's result-type row from one rule's find terms alone — the
/// types half of [`find_specs`], computable without a plan (the
/// all-dead program must still type its empty result columns), and
/// identical across rules by validation's positional alignment.
fn result_types(rule: &RuleWitness<'_>) -> Vec<ValueType> {
    rule.rule()
        .finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => rule.var_type(*var).clone(),
            // The measure positions are u64 by definition (|[s, e)| =
            // e − s — 20-query-ir § the measure).
            FindTerm::Duration(_) | FindTerm::AggregateDuration { .. } => ValueType::U64,
            FindTerm::Aggregate { op, over } => match op {
                AggOp::ArgMax { .. } | AggOp::ArgMin { .. } | AggOp::Pack => rule
                    .var_type(over.expect("validated: Arg and Pack carry a variable"))
                    .clone(),
                AggOp::Sum | AggOp::Min | AggOp::Max => rule
                    .var_type(over.expect("validated: folds carry a variable"))
                    .clone(),
                AggOp::Count | AggOp::CountDistinct => ValueType::U64,
            },
        })
        .collect()
}

/// The all-dead program's one prepared artifact: the `ExecPlan::Empty`
/// plan with nothing attached — no executor, no finds (the sink is
/// never fed and finalize never runs: the rule loop reports nothing
/// ran, exactly the Eq-miss short-circuit's empty-result path), no
/// view memo entries, no pins (nothing was read, so nothing drifts).
fn empty_rule() -> PreparedRule {
    PreparedRule {
        plan: ExecPlan::Empty,
        executor: None,
        finds: Vec::new(),
        resolved_filters: Vec::new(),
        resolved_selections: Vec::new(),
        resolved_complete: false,
        memo: build_view_memo(&ExecPlan::Empty),
        guard_finds: None,
        pinned: Box::new([]),
    }
}

/// The rule's `str` literals awaiting dictionary words — the latch
/// counter's initial value ([`PreparedQuery::unresolved_literals`]).
/// Guard plans resolve their key constants per probe and stay outside
/// the latch (the templates the latch rewrites are Free Join plan
/// arrays). Discharged occurrences count nothing: an eliminated one
/// carries no predicates, and a folded one's retained filters are
/// plan-constant by the fold's own conditions (`plan/chase/evaluate.rs`)
/// and never resolved — a fold must not block the fully-latched fast
/// path.
fn pending_literals(rule: &PreparedRule) -> u32 {
    let ExecPlan::FreeJoin(plan) = &rule.plan else {
        return 0;
    };
    let pending = |value: &crate::image::view::Const| {
        matches!(value, crate::image::view::Const::PendingIntern { .. })
    };
    plan.occurrences()
        .iter()
        .filter(|occurrence| !occurrence.role.discharged())
        .map(|occurrence| {
            let filters = occurrence
                .filters
                .iter()
                .filter(|filter| {
                    matches!(filter, crate::image::view::FilterPredicate::Compare { value, .. } if pending(value))
                })
                .count();
            let selections = occurrence
                .selections
                .iter()
                .filter(|selection| pending(&selection.value))
                .count();
            u32::try_from(filters + selections).expect("occurrence literal count fits u32")
        })
        .sum()
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

/// The theory's program rewrite (`plan/chase.rs`): the
/// elimination-and-evaluation fixpoint per rule, independently — after
/// normalization and before statistics and the DP
/// (docs/architecture/40-execution.md planner placement), with no
/// cross-rule state; a rule shrinking below its cover requirements
/// re-validates like any rule (the per-rule pipeline re-runs plan
/// validation regardless). Eliminated and folded occurrences keep
/// their ids and are skipped by every downstream path through the one
/// participates-in-planning predicate (and its execution-side sibling
/// `Role::discharged`). The evaluator may also kill a rule outright
/// (`folded to ∅` — the fold's `dead` channel, read by the survivors
/// loop below exactly like a normalize-time death). Then rule
/// subsumption: a rule whose post-elimination body a sibling contains
/// modulo eliminated filters is deleted — the union loses nothing.
/// Returns the surviving rules with their lowered-rule indices plus
/// the deletion record (the EXPLAIN surface).
fn chase_program(
    mut normalized: Vec<NormalizedQuery>,
    witness: &crate::ir::validate::ValidatedQuery,
    schema: &Schema,
) -> (
    Vec<(usize, NormalizedQuery)>,
    Vec<crate::api::stats::SubsumedRule>,
) {
    for (rule_idx, normalized_rule) in normalized.iter_mut().enumerate() {
        // A statically-empty rule (ir/normalize/fold.rs) is deleted at
        // prepare — nothing to rewrite; the subsumption pass skips it
        // symmetrically (`plan/chase.rs`).
        if normalized_rule.dead.is_some() {
            continue;
        }
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
        ExecPlan::Empty => unreachable!("dead rules never reach the per-rule pipeline"),
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
            resolved_complete: false,
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
        // 50-storage.md image layout): a multi-word field contributes its
        // whole column run (interval start/end pair, a bytes<N> field's
        // ⌈N/8⌉ words), and every field after one is shifted — spans,
        // never raw field indices.
        let columns_of = |field: crate::schema::FieldId| -> Vec<usize> {
            let span = occurrence.spans[usize::from(field.0)];
            let first = usize::from(span.first_column);
            (first..first + usize::from(span.width.column_count())).collect()
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
        // levels; set-ness is a plan fact, never per-execution data). A
        // plan-constant `WordSet` (the chase-evaluator's fold,
        // `plan/chase/evaluate.rs`) is the same level shape with the
        // elements already resolved — one machinery, two producers.
        let selections: Vec<crate::exec::colt::SelectionLevel> = occurrence
            .selections
            .iter()
            .map(|s| crate::exec::colt::SelectionLevel {
                columns: columns_of(s.field),
                set: matches!(
                    s.value,
                    crate::image::view::Const::ParamSet(_) | crate::image::view::Const::WordSet(_)
                ),
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
                        | AggOp::ArgMin { .. }
                        | AggOp::Pack => {
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
                // Pack: the interval variable's two-slot span; the result
                // position is interval-typed (the packed segment shares
                // its input's type).
                AggOp::Pack => {
                    let over = over.expect("validated: Pack carries a variable");
                    (
                        FindSpec::Pack {
                            slot: exec_plan.slot_of(over),
                        },
                        rule.var_type(over).clone(),
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
                        AggOp::ArgMax { .. } | AggOp::ArgMin { .. } | AggOp::Pack => {
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
                | FindSpec::Pack { .. }
                | FindSpec::Duration { .. }
                | FindSpec::AggDuration { .. } => None,
            })
            .collect::<Option<Vec<_>>>(),
        ExecPlan::FreeJoin(_) | ExecPlan::Empty => None,
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
            // Two-slot readers: a Pack position reads its claim's two
            // words raw (the fold-time dedup key is injective there,
            // unlike the measure), and a measure reads its interval
            // variable's two slots — moot for the elision, which
            // `union_elision` already refused on measure heads, but
            // recorded for the read set.
            FindSpec::Pack { slot }
            | FindSpec::Duration { slot }
            | FindSpec::AggDuration { slot, .. } => (*slot, 2),
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
