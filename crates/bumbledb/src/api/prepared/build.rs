use super::{
    AggregateSink, Bindings, Colt, EitherSink, Executor, FindSpec, FreeJoinRule, GuardRule,
    OccurrencePin, PARKED_SLOTS, PreparedQuery, PreparedRule, Program, ProjectionSink, ResolveMemo,
    Schema, ValueType, ViewMemo,
};

use crate::error::Result;
use crate::exec::dispatch::classify;
use crate::image::cache::ImageCache;
use crate::image::view::View;
use crate::ir::normalize::{NormalizedQuery, normalize};
use crate::ir::validate::{RuleWitness, validate};
use crate::ir::{AggOp, FindTerm, Query};
use crate::obs;
use crate::plan::fj::{DisjointWitness, binary2fj, factor, provably_disjoint_rules};
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

    // The disjointness proof runs pre-grounding (the rewrite never changes
    // the denotation, so the proof stands), and pre-deletion: pairwise
    // over a superset holds over whichever rules survive below.
    let disjoint_rules = disjointness(&witness, &normalized, schema);

    let (survivors, subsumed) = ground_program(normalized, &witness, schema);

    // The predicate the query defines, sealed at validation (the ONE
    // signature derivation) — it exists even when every rule below dies,
    // so the empty program still types its result columns.
    let predicate = witness.predicate().clone();
    let mut rules = Vec::with_capacity(survivors.len());
    let mut dead = Vec::new();
    for (rule_idx, normalized_rule) in survivors {
        // Rule death (ir/normalize/fold.rs): a statically-empty rule is
        // deleted here — no statistics read, no DP, no plan; the union
        // loses nothing because the rule denotes the empty set. The
        // record keeps the killing condition for EXPLAIN.
        if let Some(reason) = &normalized_rule.dead {
            dead.push(crate::api::stats::DeadRule {
                rule: u16::try_from(rule_idx).expect("rule count fits u16"),
                rendered: reason.clone(),
            });
            continue;
        }
        let rule = witness.rule(rule_idx);
        rules.push(prepare_rule(
            txn,
            cache,
            schema,
            &rule,
            &normalized_rule,
            &predicate.columns,
        )?);
    }
    // A program deletion (subsumption or rule death) shrank to at most
    // one live rule has no pair left to prove (the stats surface's
    // single-rule contract; pairwise over a superset held regardless).
    let disjoint_rules = (rules.len() > 1).then_some(disjoint_rules).flatten();
    let params = param_specs(&witness);

    // The one sink configuration — head-owned shape (projection vs
    // aggregate, arity, distinctness), built aimed at rule 0's layout
    // and re-aimed per rule by the rule loop. Presized against the
    // rules' worst estimate (one sink hears every rule). A single-rule
    // aggregate may elide its seen-set under the plan's distinct-bindings
    // proof. Every multi-rule sink keeps one head-projection seen-set
    // spanning all rules: that map is the union representation.
    let output_hint = output_hint(&rules);
    let union = rules.len() > 1;
    let sink = rules.first().map_or_else(
        || make_sink(&[], 0, true, false, 0),
        |first| {
            make_sink(
                first.finds(),
                first.slot_count(),
                !union && first.distinct_bindings(),
                union,
                output_hint,
            )
        },
    );
    // The rule-shared binding-slot scratch, sized at the rules'
    // high-water so the per-rule resize never allocates.
    let bindings = Bindings::new(
        rules
            .iter()
            .map(PreparedRule::slot_count)
            .max()
            .unwrap_or(0),
    );

    // The byte-heap types keep the resolving finalize: String resolves
    // through the dictionary; a bytes<N> find re-assembles its slot words
    // into the byte heap (no dictionary — inline values).
    let all_words = predicate
        .columns
        .iter()
        .all(|column| !matches!(column.ty, ValueType::String | ValueType::FixedBytes { .. }));
    let unresolved_literals = rules.iter().map(pending_literals).sum();
    let program = if rules.is_empty() {
        Program::Empty
    } else {
        Program::Rules(rules)
    };
    Ok(PreparedQuery {
        schema,
        env_instance: txn.env_instance(),
        disjoint_rules,
        subsumed,
        dead,
        program,
        predicate,
        params,
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

/// The shared sink's capacity hint, derived only from the already-frozen
/// rule plans.
fn output_hint(rules: &[PreparedRule]) -> usize {
    rules
        .iter()
        .map(|rule| match rule {
            // Sink presizing: the last node's planner estimate bounds
            // the binding stream the sink consumes.
            PreparedRule::FreeJoin(rule) => {
                let plan = &rule.plan;
                usize::try_from(plan.estimates().last().copied().unwrap_or(0).min(1 << 21))
                    .expect("clamped")
            }
            PreparedRule::Guard(_) => 1,
        })
        .max()
        .unwrap_or(0)
}

/// The rule's `str` literals awaiting dictionary words — the latch
/// counter's initial value ([`PreparedQuery::unresolved_literals`]).
/// Guard plans resolve their key constants per probe and stay outside
/// the latch (the templates the latch rewrites are Free Join plan
/// arrays). Discharged occurrences count nothing: an eliminated one
/// carries no conditions, and a folded one's retained filters are
/// plan-constant by the fold's own conditions (`plan/ground/evaluate.rs`)
/// and never resolved — a fold must not block the fully-latched fast
/// path.
fn pending_literals(rule: &PreparedRule) -> u32 {
    let PreparedRule::FreeJoin(rule) = rule else {
        return 0;
    };
    let plan = &rule.plan;
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

/// Dense bind contracts (validation rejected gaps jointly across value
/// and mask params, across all rules). A value param becomes exactly one
/// scalar/set variant carrying its point-domain bit; a mask becomes the
/// typeless mask variant.
fn param_specs(witness: &crate::ir::validate::ValidatedQuery) -> Vec<super::ParamSpec> {
    let value_types: std::collections::BTreeMap<crate::ir::ParamId, &ValueType> =
        witness.param_types().collect();
    let param_count = value_types.len() + witness.mask_params().len();
    let mut params = Vec::with_capacity(param_count);
    for idx in 0..param_count {
        let id = crate::ir::ParamId(u16::try_from(idx).expect("param ids fit u16"));
        let point = witness.point_params().contains(&id);
        let spec = value_types.get(&id).map_or_else(
            || {
                debug_assert!(witness.mask_params().contains(&id), "dense param ids");
                super::ParamSpec::Mask
            },
            |ty| {
                if witness.set_params().contains(&id) {
                    super::ParamSpec::Set {
                        elem: (*ty).clone(),
                        point,
                    }
                } else {
                    super::ParamSpec::Scalar {
                        ty: (*ty).clone(),
                        point,
                    }
                }
            },
        );
        params.push(spec);
    }
    params
}

/// The theory's program rewrite (`plan/ground.rs`): the
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
fn ground_program(
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
        // symmetrically (`plan/ground.rs`).
        if normalized_rule.dead.is_some() {
            continue;
        }
        crate::plan::ground::ground(
            normalized_rule,
            schema,
            &witness.rule(rule_idx).rule().finds,
        );
    }
    let finds: Vec<&[FindTerm]> = (0..normalized.len())
        .map(|idx| witness.rule(idx).rule().finds.as_slice())
        .collect();
    let subsumed: Vec<crate::api::stats::SubsumedRule> =
        crate::plan::ground::subsume(&normalized, &finds)
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
/// changes, over one already-grounded rule. Returns the rule's prepared
/// artifact; result types are the query's predicate ([`super::Predicate`]),
/// never re-derived here.
fn prepare_rule(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
    columns: &[crate::ir::validate::PredicateColumn],
) -> Result<PreparedRule> {
    // Classification first: a guard probe needs no statistics or planning.
    let classified = {
        let _s = obs::span(obs::names::CLASSIFY, obs::Category::Prepare);
        classify(normalized, schema)
    };
    if let Some(plan) = classified {
        let finds = find_specs(rule, &plan);
        let guard_finds = guard_find_table(&plan, &finds, columns);
        return Ok(PreparedRule::Guard(GuardRule {
            plan,
            finds,
            guard_finds,
        }));
    }

    // The staleness pin record (`staleness.rs`): the statistics below,
    // kept instead of dropped. Stays empty for guard probes — they read
    // no statistics, so there is nothing to drift.
    let mut pins = Vec::new();
    // Per-occurrence input estimates (docs/architecture/40-execution.md): row counters
    // shaped by the selectivity ladder — key-exact counts,
    // resident-image distinct counts (peek only: prepare never
    // builds an image for statistics), documented bounds and floors.
    // Participating occurrences only: negated occurrences enter no
    // DP state and grounding-eliminated occurrences left planning
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
    let plan =
        crate::plan::fj::validate(&fj, normalized, schema, order.estimates.clone(), &sink_vars)
            .expect("binary2fj + factor construct valid plans");
    lower_span.end();

    let finds = find_specs(rule, &plan);
    let executor = Executor::new(&plan);
    let occurrence_count = plan.occurrences().len();

    // BUILD_COLTS is pure column-schema construction since the unbound-
    // views cutover: prepare provably never touches an image (the stats
    // phase peeks, never builds), so a prepared query pins nothing.
    let memo = {
        let _s = obs::span(obs::names::BUILD_COLTS, obs::Category::Prepare);
        build_view_memo(&plan)
    };
    Ok(PreparedRule::FreeJoin(FreeJoinRule {
        plan,
        executor,
        finds,
        resolved_filters: vec![Vec::new(); occurrence_count],
        resolved_selections: vec![Vec::new(); occurrence_count],
        resolved_complete: false,
        memo,
        pinned: pins.into_boxed_slice(),
    }))
}

/// COLT sources with their fixed column schemas over [`View::Unbound`]:
/// prepare touches no image — the first execution binds every view via
/// the ordinary memo-miss path (a `None` generation never matches),
/// paying the image build exactly where a cold execution already pays
/// it. Pure column-schema construction; nothing here can fail.
fn build_view_memo(plan: &crate::plan::fj::ValidatedPlan) -> ViewMemo {
    let mut memo = ViewMemo {
        colts: Vec::new(),
        generation: Vec::new(),
        filters: Vec::new(),
        parked: Vec::new(),
        spare_buffers: Vec::new(),
        tick: 0,
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
        // plan-constant `WordSet` (the grounding-evaluator's fold,
        // `plan/ground/evaluate.rs`) is the same level shape with the
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

/// Derives one rule's per-find output specs (slot spans) from its
/// witness slice and classified plan. Slots and widths both come
/// from the rule's binding-slot layout (`slot_of`/`width_of` — the
/// `SlotWidth` map): an interval variable's find spans two words, and no
/// consumer assumes width 1. Result types are NOT derived here — they
/// are the query's predicate (`ir/validate`); the specs are this rule's.
trait SlotLayout {
    fn slot_of(&self, var: crate::ir::VarId) -> usize;
    fn width_of(&self, var: crate::ir::VarId) -> usize;
}

impl SlotLayout for crate::plan::fj::ValidatedPlan {
    fn slot_of(&self, var: crate::ir::VarId) -> usize {
        self.slot_of(var)
    }

    fn width_of(&self, var: crate::ir::VarId) -> usize {
        self.width_of(var)
    }
}

impl SlotLayout for crate::exec::dispatch::GuardPlan {
    fn slot_of(&self, var: crate::ir::VarId) -> usize {
        self.slot_of(var)
    }

    fn width_of(&self, var: crate::ir::VarId) -> usize {
        self.width_of(var)
    }
}

fn find_specs(rule: &RuleWitness<'_>, layout: &impl SlotLayout) -> Vec<FindSpec> {
    rule.rule()
        .finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => FindSpec::Var {
                slot: layout.slot_of(*var),
                width: layout.width_of(*var),
            },
            // The measure positions: one u64 word computed from the
            // interval variable's two-slot span (the sinks own the
            // subtraction and the ray check — `exec::sink`).
            FindTerm::Duration(var) => FindSpec::Duration {
                slot: layout.slot_of(*var),
            },
            FindTerm::AggregateDuration { op, over } => FindSpec::AggDuration {
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
                slot: layout.slot_of(*over),
            },
            FindTerm::Aggregate { op, over } => match op {
                // Arg-restriction: the carry's span plus the shared key
                // slot (orderable — validated U64/I64, one word).
                AggOp::ArgMax { key } | AggOp::ArgMin { key } => {
                    let carry = over.expect("validated: Arg carries a variable");
                    FindSpec::Arg {
                        slot: layout.slot_of(carry),
                        width: layout.width_of(carry),
                        key_slot: layout.slot_of(*key),
                        max: matches!(op, AggOp::ArgMax { .. }),
                    }
                }
                // Pack: the interval variable's two-slot span.
                AggOp::Pack => FindSpec::Pack {
                    slot: layout.slot_of(over.expect("validated: Pack carries a variable")),
                },
                AggOp::Sum | AggOp::Min | AggOp::Max | AggOp::Count | AggOp::CountDistinct => {
                    let (over_slot, over_width, over_ty) = match over {
                        Some(var) => (
                            Some(layout.slot_of(*var)),
                            layout.width_of(*var),
                            rule.var_type(*var).clone(),
                        ),
                        None => (None, 1, ValueType::U64), // Count
                    };
                    let fold = match op {
                        AggOp::Sum => crate::exec::sink::FoldOp::Sum,
                        AggOp::Min => crate::exec::sink::FoldOp::Min,
                        AggOp::Max => crate::exec::sink::FoldOp::Max,
                        AggOp::Count => crate::exec::sink::FoldOp::Count,
                        AggOp::CountDistinct => crate::exec::sink::FoldOp::CountDistinct,
                        AggOp::ArgMax { .. } | AggOp::ArgMin { .. } | AggOp::Pack => {
                            unreachable!("handled above")
                        }
                    };
                    FindSpec::Agg {
                        op: fold,
                        over_slot,
                        over_width,
                        // The fold INPUT's signedness (a rule-local
                        // fact, not the signature's): Sum must decode
                        // the biased word form before accumulating.
                        signed: matches!(over_ty, ValueType::I64),
                    }
                }
            },
        })
        .collect()
}

/// The guard fast lane's find table: `Some` for guard plans whose finds
/// are all plain variables. Types come from the predicate's columns —
/// find order IS column order.
fn guard_find_table(
    guard: &crate::exec::dispatch::GuardPlan,
    finds: &[FindSpec],
    columns: &[crate::ir::validate::PredicateColumn],
) -> Option<Vec<(crate::schema::FieldId, ValueType)>> {
    finds
        .iter()
        .zip(columns)
        .map(|(spec, column)| match spec {
            FindSpec::Var { slot, .. } => {
                let var = guard
                    .vars
                    .iter()
                    .find(|v| v.slot == *slot)
                    .expect("find slots come from the guard plan's layout");
                Some((var.field, column.ty.clone()))
            }
            // aggregate and measure guards keep the sink path
            FindSpec::Agg { .. }
            | FindSpec::Arg { .. }
            | FindSpec::Pack { .. }
            | FindSpec::Duration { .. }
            | FindSpec::AggDuration { .. } => None,
        })
        .collect::<Option<Vec<_>>>()
}

/// The rule-disjointness proof (docs/architecture/40-execution.md § set
/// semantics) — retained as diagnostic knowledge for EXPLAIN, run over the
/// whole program before the pipeline goes per-rule (the grounding rewrites
/// occurrences but never the denotation, so the pre-grounding proof stands).
/// Single-rule programs have no pair to prove.
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

/// Builds the sink matching the head shape (the variant is fixed per
/// prepared query — an enum, not `dyn`), aimed at rule 0's binding
/// layout. `union` is the multi-rule regime (head-projection dedup
/// keys); `distinct` is the proof the dedup-key stream is duplicate-free
/// for a single rule, which elides the aggregate seen-set. It is always
/// false for a union: one spanning seen-set is the union representation.
fn make_sink(
    finds: &[FindSpec],
    slot_count: usize,
    distinct: bool,
    union: bool,
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
        EitherSink::Projection(ProjectionSink::with_capacity_hint(finds, slot_count, hint))
    } else {
        EitherSink::Aggregate(Box::new(AggregateSink::with_capacity_hint(
            finds, slot_count, distinct, union, hint,
        )))
    }
}
