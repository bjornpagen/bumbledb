use super::{
    AggregateSink, Bindings, Colt, EitherSink, Executor, FindSpec, FreeJoinRule, KeyProbeRule,
    OccurrencePin, PreparedBody, PreparedInterior, PreparedQuery, PreparedRule, ProjectionSink,
    ResolveMemo, Schema, ValueType, ViewMemo, PARKED_SLOTS,
};

use crate::error::Result;
use crate::exec::dispatch::classify;
use crate::image::cache::ImageCache;
use crate::image::view::View;
use crate::ir::normalize::{normalize_predicate, NormalizedQuery};
use crate::ir::validate::{validate, RuleWitness};
use crate::ir::{AggOp, FindTerm, Query};
use crate::obs;
use crate::plan::fj::{
    binary2fj, factor, fold_split, gj_split, provably_disjoint_rules, provably_distinct,
    DisjointWitness, DistinctWitness,
};
use crate::plan::planner::plan as plan_order;
use crate::storage::env::ReadTxn;

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
/// Only on programmer-invariant violations (`binary2fj` + `factor` +
/// `fold_split` + `gj_split` construct valid plans by construction).
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
    let mut signatures: Vec<&crate::ir::validate::Predicate> = Vec::new();
    let mut interiors = Vec::with_capacity(witness.interiors().len());
    for i in 0..witness.interiors().len() {
        interiors.push(prepare_interior(
            txn,
            cache,
            schema,
            &witness,
            i,
            &signatures,
        )?);
        signatures.push(witness.interiors()[i].predicate());
    }
    let rec = if witness.rec().is_some() {
        signatures.push(witness.rec().expect("rec present").predicate());
        Some(prepare_reach(txn, cache, schema, &witness, &signatures)?)
    } else {
        None
    };
    prepare_witnessed(
        txn,
        cache,
        schema,
        &witness,
        crate::ir::render::render(schema, query),
        interiors,
        rec,
        &signatures,
    )
}

/// The pipeline after interiors and rec are prepared — normalize → ground
/// → per-rule prepare → sink and binding artifacts, over an already-sealed
/// witness. Params are query-global (one binding surface,
/// `docs/architecture/20-query-ir.md` § engine recursion).
#[expect(
    clippy::too_many_lines,
    reason = "the prepare pipeline reads as one protocol: normalize, ground, per-rule prepare, probes, artifacts"
)]
fn prepare_witnessed<'s, S>(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &'s Schema,
    witness: &crate::ir::validate::ValidatedQuery,
    rendered: String,
    interiors: Vec<PreparedInterior>,
    rec: Option<super::reach::ReachDriver>,
    signatures: &[&crate::ir::validate::Predicate],
) -> Result<PreparedQuery<'s, S>> {
    let normalized = {
        let _s = obs::span(obs::names::NORMALIZE, obs::Category::Prepare);
        normalize_predicate(schema, witness, signatures)
    };

    // The disjointness proof runs pre-grounding (the rewrite never changes
    // the denotation, so the proof stands), and pre-deletion: pairwise
    // over a superset holds over whichever rules survive below.
    let disjoint_rules = disjointness(witness, &normalized, schema);

    let (survivors, subsumed) = ground_program(normalized, witness, schema);

    // The predicate the query defines, sealed at validation (the ONE
    // signature derivation) — it exists even when every rule below dies,
    // so the empty program still types its result columns.
    let predicate = witness.predicate().clone();
    let mut rules = Vec::with_capacity(survivors.len());
    // Written-rule provenance per surviving rule (R2): the sink regime
    // splits on it below. The first survivor's witness index feeds the
    // dense group domains (049).
    let mut written = Vec::with_capacity(survivors.len());
    let mut first_rule_idx = None;
    let mut dead = Vec::new();
    for (rule_idx, normalized_rule) in survivors {
        // Rule death (ir/normalize/fold.rs): a statically-empty rule is
        // deleted here — no statistics read, no DP, no plan; the union
        // loses nothing because the rule denotes the empty set. The
        // record keeps the killing condition for introspection.
        if let Some(reason) = &normalized_rule.dead {
            dead.push(crate::api::stats::DeadRule {
                rule: u16::try_from(rule_idx).expect("rule count fits u16"),
                rendered: reason.clone(),
            });
            continue;
        }
        let rule = witness.rule(rule_idx);
        written.push(rule.written());
        first_rule_idx.get_or_insert(rule_idx);
        rules.push(prepare_rule(
            txn,
            cache,
            schema,
            &rule,
            &normalized_rule,
            &predicate.columns,
            signatures,
        )?);
    }
    // A program deletion (subsumption or rule death) shrank to at most
    // one live rule has no pair left to prove (the stats surface's
    // single-rule contract; pairwise over a superset held regardless).
    let disjoint_rules = (rules.len() > 1).then_some(disjoint_rules).flatten();
    let params = param_specs(witness);

    // The one sink configuration — head-owned shape (projection vs
    // aggregate, arity, distinctness), built aimed at rule 0's layout
    // and re-aimed per rule by the rule loop. Presized against the
    // rules' worst estimate (one sink hears every rule). A single-rule
    // aggregate may elide its seen-set under the plan's distinct-bindings
    // proof. Every multi-rule sink keeps one seen-set spanning all rules
    // — that map is the union representation — keyed by provenance
    // (R2): the head projection for a hand-written rule set, the shared
    // slot arrays for a DNF-derived one.
    let output_hint = output_hint(&rules);
    if rules.len() > 1 && dnf_derived(&written) {
        seal_dnf_spans(&mut rules);
    }
    // The dense group domains (finding 049), single-rule sinks only: a
    // hand-written sibling need not share the domain proof, and the
    // re-aim path never reshapes the table.
    let dense_groups = if rules.len() == 1 {
        first_rule_idx.map_or_else(Vec::new, |idx| group_radixes(&witness.rule(idx)))
    } else {
        Vec::new()
    };
    let sink = rules.first().map_or_else(
        || make_sink(&[], 0, SinkRegime::SingleRule(None), 0, &[]),
        |first| {
            let regime = if rules.len() > 1 {
                if dnf_derived(&written) {
                    SinkRegime::DnfUnion(first.dedup_spans())
                } else {
                    SinkRegime::Union
                }
            } else {
                SinkRegime::SingleRule(first.distinct_witness())
            };
            make_sink(
                first.finds(),
                first.slot_count(),
                regime,
                output_hint,
                &dense_groups,
            )
        },
    );
    // The ray probes (the Kleene verdict algebra, ruled 2026-07-23,
    // R6): per written rule with measure conditions, one probe per
    // measured variable plus the rule's compiled verdict fold — the
    // mainline rules drop rays (a ray never Holds), so the probes are
    // the ONE place a Ray verdict is rendered, after the rule loop.
    // A dead query skips them at execution (`PreparedBody::Empty` returns
    // before the rule loop; a dead disjunct's constant refutation is a
    // Fails leaf, so its verdict is never Ray anyway).
    let ray_probes = if rules.is_empty() {
        Vec::new()
    } else {
        prepare_ray_probes(txn, cache, schema, witness, signatures)?
    };

    let interior_slots = interiors.iter().flat_map(|interior| {
        interior.rules.iter().map(PreparedRule::slot_count).chain(
            interior
                .ray_probes
                .iter()
                .flat_map(|set| set.probes.iter().map(|p| p.rule.plan.slot_count())),
        )
    });
    let rec_slots = rec.as_ref().into_iter().flat_map(|driver| {
        driver
            .base
            .iter()
            .chain(&driver.rec)
            .map(PreparedRule::slot_count)
    });
    let bindings = Bindings::new(
        rules
            .iter()
            .map(PreparedRule::slot_count)
            .chain(
                ray_probes
                    .iter()
                    .flat_map(|set| set.probes.iter().map(|p| p.rule.plan.slot_count())),
            )
            .chain(interior_slots)
            .chain(rec_slots)
            .max()
            .unwrap_or(0),
    );

    let unresolved_literals = rules.iter().map(pending_literals).sum::<u32>()
        + ray_probes
            .iter()
            .flat_map(|set| &set.probes)
            .map(|probe| plan_pending_literals(&probe.rule.plan))
            .sum::<u32>()
        + interiors
            .iter()
            .flat_map(|interior| {
                interior.rules.iter().map(pending_literals).chain(
                    interior
                        .ray_probes
                        .iter()
                        .flat_map(|set| &set.probes)
                        .map(|probe| plan_pending_literals(&probe.rule.plan)),
                )
            })
            .sum::<u32>()
        + rec.as_ref().map_or(0, |driver| {
            driver
                .base
                .iter()
                .chain(&driver.rec)
                .map(pending_literals)
                .sum()
        });
    let body = if let Some(mut driver) = rec {
        driver.main = rules;
        PreparedBody::Reach(Box::new(driver))
    } else if rules.is_empty() {
        PreparedBody::Empty
    } else {
        PreparedBody::Rules(rules)
    };
    Ok(PreparedQuery {
        schema,
        env_instance: txn.env_instance(),
        disjoint_rules,
        subsumed,
        dead,
        interiors,
        body,
        rounds_budget: super::reach::DEFAULT_REACH_ROUNDS,
        tuples_budget: super::reach::DEFAULT_DERIVED_TUPLES,
        derived: super::reach::DerivedScratch::default(),
        predicate,
        params,
        resolved_params: Vec::new(),
        unresolved_literals,
        param_word_memo: Vec::new(),
        missed_params: Vec::new(),
        sink,
        ray_probes,
        bindings,
        answer_scratch: Vec::new(),
        resolve_memo: ResolveMemo::new(),
        determinant_key: Vec::new(),
        rendered,
        marker: std::marker::PhantomData,
    })
}

fn prepare_interior(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    witness: &crate::ir::validate::ValidatedQuery,
    index: usize,
    signatures: &[&crate::ir::validate::Predicate],
) -> Result<PreparedInterior> {
    let inner = &witness.interiors()[index];
    let columns = &inner.predicate().columns;
    let witnesses: Vec<_> = witness.interior_rules(index).collect();
    let normalized =
        crate::ir::normalize::normalize_rules(schema, signatures, witnesses.iter().copied());
    let finds: Vec<&[crate::ir::FindTerm]> = witnesses
        .iter()
        .map(|rule| rule.rule().finds.as_slice())
        .collect();
    let (survivors, _subsumed) = ground_rules(normalized, &finds, schema);
    let mut rules = Vec::with_capacity(survivors.len());
    let mut written = Vec::with_capacity(survivors.len());
    for (rule_idx, normalized_rule) in survivors {
        if normalized_rule.dead.is_some() {
            continue;
        }
        let rule = witnesses[rule_idx];
        written.push(rule.written());
        rules.push(prepare_rule(
            txn,
            cache,
            schema,
            &rule,
            &normalized_rule,
            columns,
            signatures,
        )?);
    }
    if rules.len() > 1 && dnf_derived(&written) {
        seal_dnf_spans(&mut rules);
    }
    let units = rules.len();
    let hint = output_hint(&rules);
    let sink = rules.first().map_or_else(
        || crate::exec::sink::ProjectionSink::with_capacity_hint(&[], 0, 0),
        |first| {
            crate::exec::sink::ProjectionSink::with_capacity_hint(
                first.finds(),
                first.slot_count(),
                hint,
            )
        },
    );
    let ray_probes = if rules.is_empty() {
        Vec::new()
    } else {
        prepare_ray_probes_for(txn, cache, schema, &witnesses, signatures)?
    };
    Ok(PreparedInterior {
        rules,
        sink,
        field_types: columns.iter().map(|c| c.ty.type_desc()).collect(),
        units,
        ray_probes,
    })
}

fn prepare_reach(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    witness: &crate::ir::validate::ValidatedQuery,
    signatures: &[&crate::ir::validate::Predicate],
) -> Result<super::reach::ReachDriver> {
    let rec = witness.rec().expect("rec present");
    let columns = &rec.predicate().columns;
    let rec_id = crate::ir::InteriorId(
        u32::try_from(witness.interiors().len()).expect("overflow judged at validate"),
    );
    let base_w: Vec<_> = witness.rec_base_rules().collect();
    let rec_w: Vec<_> = witness.rec_step_rules().collect();
    let base_norm =
        crate::ir::normalize::normalize_rules(schema, signatures, base_w.iter().copied());
    let rec_norm = crate::ir::normalize::normalize_rules(schema, signatures, rec_w.iter().copied());
    let base_finds: Vec<&[crate::ir::FindTerm]> =
        base_w.iter().map(|r| r.rule().finds.as_slice()).collect();
    let rec_finds: Vec<&[crate::ir::FindTerm]> =
        rec_w.iter().map(|r| r.rule().finds.as_slice()).collect();
    let (base_surv, _) = ground_rules(base_norm, &base_finds, schema);
    let (rec_surv, _) = ground_rules(rec_norm, &rec_finds, schema);
    let mut base = Vec::new();
    for (rule_idx, normalized_rule) in base_surv {
        if normalized_rule.dead.is_some() {
            continue;
        }
        base.push(prepare_rule(
            txn,
            cache,
            schema,
            &base_w[rule_idx],
            &normalized_rule,
            columns,
            signatures,
        )?);
    }
    let mut rec_rules = Vec::new();
    for (rule_idx, normalized_rule) in rec_surv {
        if normalized_rule.dead.is_some() {
            continue;
        }
        let delta = normalized_rule
            .occurrences
            .iter()
            .filter(|occ| occ.role.participates())
            .find(|occ| occ.source.interior() == Some(rec_id))
            .map(|occ| occ.occ_id)
            .expect("RecArmMissingSelf judged at validate");
        let prepared = prepare_rule_variant(
            txn,
            cache,
            schema,
            &rec_w[rule_idx],
            &normalized_rule,
            columns,
            signatures,
            Some(delta),
        )?;
        let PreparedRule::FreeJoin(fj) = prepared else {
            unreachable!("an Interior-reading rec arm never classifies as a key probe")
        };
        rec_rules.push(PreparedRule::Recursive(super::RecursiveRule {
            variant: super::DeltaVariant { delta, rule: fj },
        }));
    }
    let units = base.len() + rec_rules.len();
    let hint = output_hint(&base) + output_hint(&rec_rules);
    let sink = base.first().or(rec_rules.first()).map_or_else(
        || crate::exec::sink::ProjectionSink::with_capacity_hint(&[], 0, 0),
        |first| {
            crate::exec::sink::ProjectionSink::with_capacity_hint(
                first.finds(),
                first.slot_count(),
                hint,
            )
        },
    );
    Ok(super::reach::ReachDriver {
        base,
        rec: rec_rules,
        main: Vec::new(),
        field_types: columns.iter().map(|c| c.ty.type_desc()).collect(),
        sink,
        units,
        scratch: super::reach::ReachScratch::default(),
    })
}

fn ground_rules(
    mut normalized: Vec<NormalizedQuery>,
    finds: &[&[crate::ir::FindTerm]],
    schema: &Schema,
) -> (
    Vec<(usize, NormalizedQuery)>,
    Vec<crate::api::stats::SubsumedRule>,
) {
    for (rule_idx, normalized_rule) in normalized.iter_mut().enumerate() {
        if normalized_rule.dead.is_some() {
            continue;
        }
        crate::plan::ground::ground(normalized_rule, schema, finds[rule_idx]);
    }
    let subsumed: Vec<crate::api::stats::SubsumedRule> =
        crate::plan::ground::subsume(&normalized, finds)
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
            // Variant estimates share the floors; variant 0 speaks.
            PreparedRule::Recursive(rule) => {
                let plan = &rule.variant.rule.plan;
                usize::try_from(plan.estimates().last().copied().unwrap_or(0).min(1 << 21))
                    .expect("clamped")
            }
            PreparedRule::KeyProbe(_) => 1,
        })
        .max()
        .unwrap_or(0)
}

/// The rule's `str` literals awaiting dictionary words — the latch
/// counter's initial value ([`PreparedQuery::unresolved_literals`]).
/// `KeyProbePlan` values resolve their key constants per probe and stay outside
/// the latch (the templates the latch rewrites are Free Join plan
/// arrays). Discharged occurrences count nothing: an eliminated one
/// carries no conditions, and a folded one's retained filters are
/// plan-constant by the fold's own conditions (`plan/ground/evaluate.rs`)
/// and never resolved — a fold must not block the fully-latched fast
/// path.
fn pending_literals(rule: &PreparedRule) -> u32 {
    match rule {
        PreparedRule::FreeJoin(rule) => plan_pending_literals(&rule.plan),
        // Each variant carries its own plan templates and latches
        // independently — the counter sums them all.
        PreparedRule::Recursive(rule) => plan_pending_literals(&rule.variant.rule.plan),
        PreparedRule::KeyProbe(_) => 0,
    }
}

/// One Free Join plan's `str` literals awaiting dictionary words.
fn plan_pending_literals(plan: &crate::plan::fj::ValidatedPlan) -> u32 {
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

/// Dense bind contracts (validation rejected gaps). A value param
/// becomes exactly one scalar/set variant carrying its point-domain bit.
fn param_specs(witness: &crate::ir::validate::ValidatedQuery) -> Vec<super::ParamSpec> {
    let value_types: std::collections::BTreeMap<crate::ir::ParamId, &ValueType> =
        witness.param_types().collect();
    let param_count = value_types.len();
    let mut params = Vec::with_capacity(param_count);
    for idx in 0..param_count {
        let id = crate::ir::ParamId(u16::try_from(idx).expect("param ids fit u16"));
        let point = witness.point_params().contains(&id);
        let ty = value_types.get(&id).expect("dense param ids");
        let spec = if witness.set_params().contains(&id) {
            super::ParamSpec::Set {
                elem: (*ty).clone(),
                point,
            }
        } else {
            super::ParamSpec::Scalar {
                ty: (*ty).clone(),
                point,
            }
        };
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
/// the deletion record (the introspection surface).
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
    signatures: &[&crate::ir::validate::Predicate],
) -> Result<PreparedRule> {
    prepare_rule_variant(
        txn, cache, schema, rule, normalized, columns, signatures, None,
    )
}

/// [`prepare_rule`] with interiors/rec: the sealed signatures
/// (`Interior` occurrences' field→column spans) and — for one delta variant
/// of a rec arm — the marked delta occurrence, whose statistics
/// take the ladder's delta floor while other `Interior` occurrences take the
/// accumulated floor (`plan/selectivity.rs`; 40-execution.md § the linear reach driver, the
/// param-plan precedent). The query path passes the empty surface.
#[expect(
    clippy::too_many_arguments,
    reason = "the per-rule pipeline's inputs are clearer unpacked"
)]
fn prepare_rule_variant(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
    columns: &[crate::ir::validate::PredicateColumn],
    signatures: &[&crate::ir::validate::Predicate],
    delta: Option<crate::ir::normalize::OccId>,
) -> Result<PreparedRule> {
    let distinct_witness = provably_distinct(normalized, schema);
    // Classification first: a key probe needs no statistics or planning.
    let classified = {
        let _s = obs::span(obs::names::CLASSIFY, obs::Category::Prepare);
        classify(normalized, schema)
    };
    if let Some(plan) = classified {
        let finds = find_specs(rule, &plan);
        let key_probe_finds = key_probe_find_table(&plan, &finds, columns);
        return Ok(PreparedRule::KeyProbe(KeyProbeRule {
            plan,
            distinct_witness,
            finds,
            // Written by `seal_dnf_spans` iff the query is a
            // DNF-derived union; empty (and never read) otherwise.
            dedup_spans: Box::default(),
            key_probe_finds,
        }));
    }

    // The staleness pin record (`staleness.rs`): the statistics below,
    // kept instead of dropped. Stays empty for key probes — they read
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
        // An `Interior` occurrence pins nothing (20-query-ir.md § engine recursion's
        // consumer table): its row count is prepare-unknowable, so it
        // reads no row counter and costs on the selectivity ladder's
        // floors — the delta floor for the variant's marked occurrence,
        // the accumulated floor for every other predicate read — the
        // staleness surface already knows the shape (negated and
        // grounding-discharged occurrences carry no pin today).
        let Some(relation) = occurrence.source.edb() else {
            let floor = if delta == Some(occurrence.occ_id) {
                crate::plan::selectivity::DELTA_PLANNING_ROWS
            } else {
                crate::plan::selectivity::INTERIOR_PLANNING_ROWS
            };
            stats.push(crate::plan::selectivity::occurrence_stats(
                txn, cache, schema, occurrence, floor,
            )?);
            continue;
        };
        let rows = crate::plan::selectivity::relation_rows(txn, schema, relation)?;
        let occ_stats =
            crate::plan::selectivity::occurrence_stats(txn, cache, schema, occurrence, rows)?;
        pins.push(OccurrencePin {
            occ_id: occurrence.occ_id,
            relation,
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
    let mut estimates = order.estimates.clone();
    // The fold-aware level split, aggregate heads only (a projection
    // has no fold to push down): group variables form their own prefix
    // levels so leaf scan runs are group-constant and the aggregate
    // sink's scan-fold pushdown can fire (`plan/fj/fold_split.rs`).
    if rule.rule().finds.iter().any(|term| {
        matches!(
            term,
            FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. }
        )
    }) {
        let group_key: std::collections::BTreeSet<crate::ir::VarId> = rule
            .rule()
            .finds
            .iter()
            .filter_map(|term| match term {
                FindTerm::Var(var) | FindTerm::Measure(var) => Some(*var),
                FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. } => None,
            })
            .collect();
        fold_split(&mut fj, &group_key, &mut estimates);
    }
    gj_split(&mut fj);
    // Group key for projections; every variable for aggregates —
    // skip-illegality under a fold is encoded in the bits themselves
    // (`RuleWitness::sink_vars`).
    let sink_vars = rule.sink_vars();
    let plan = crate::plan::fj::validate_with_signatures(
        &fj, normalized, schema, signatures, estimates, &sink_vars,
    )
    .expect("binary2fj + factor + fold_split + gj_split construct valid plans");
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
        // Written by `seal_dnf_spans` iff the query is a DNF-derived
        // union; empty (and never read) otherwise.
        dedup_spans: Box::default(),
        resolved_filters: vec![Vec::new(); occurrence_count],
        resolved_selections: vec![Vec::new(); occurrence_count],
        resolution: super::ResolutionState::Pending,
        memo,
        pinned: pins.into_boxed_slice(),
    }))
}

/// The ray probes (the Kleene verdict algebra, ruled 2026-07-23, R6):
/// per written rule with measure conditions, one probe rule per
/// measured interval variable — the rule's atoms, negations, and
/// memberships with every condition replaced by the is-ray filter
/// (`ir/normalize::normalize_ray_probe`) — plus the rule's compiled
/// verdict fold over ALL its lowered disjuncts (dead and subsumed
/// included: a constant refutation is a Fails leaf and folds itself
/// out). Grouping reads the mint set, not `written`: a cross-written
/// collapse erases the latter but unions the former, so each rule
/// folds exactly its own disjunct set. Disjuncts of one written rule
/// share one variable scope — one slot layout serves the group's
/// probes and its verdict.
fn prepare_ray_probes(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    witness: &crate::ir::validate::ValidatedQuery,
    signatures: &[&crate::ir::validate::Predicate],
) -> Result<Vec<super::RayProbeSet>> {
    let members: Vec<_> = witness.rules().collect();
    prepare_ray_probes_for(txn, cache, schema, &members, signatures)
}

fn prepare_ray_probes_for(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    rules: &[crate::ir::validate::RuleWitness<'_>],
    signatures: &[&crate::ir::validate::Predicate],
) -> Result<Vec<super::RayProbeSet>> {
    use crate::ir::validate::ClassifiedComparison;
    let mut groups: Vec<u16> = rules
        .iter()
        .flat_map(|rule| rule.minted().to_vec())
        .collect();
    groups.sort_unstable();
    groups.dedup();
    let mut sets = Vec::new();
    for written in groups {
        let members: Vec<crate::ir::validate::RuleWitness<'_>> = rules
            .iter()
            .copied()
            .filter(|rule| rule.minted().contains(&written))
            .collect();
        let mut measured: Vec<crate::ir::VarId> = members
            .iter()
            .flat_map(crate::ir::validate::RuleWitness::classified_comparisons)
            .filter_map(|comparison| match comparison {
                ClassifiedComparison::Duration { interval, .. } => Some(*interval),
                _ => None,
            })
            .collect();
        measured.sort_unstable();
        measured.dedup();
        if measured.is_empty() {
            continue;
        }
        let template = members[0];
        let mut probes = Vec::with_capacity(measured.len());
        for var in measured {
            let normalized =
                crate::ir::normalize::normalize_ray_probe(schema, signatures, &template, var);
            if normalized.dead.is_some() {
                continue;
            }
            probes.push(prepare_ray_probe(
                txn,
                cache,
                schema,
                &template,
                &normalized,
                var,
                signatures,
            )?);
        }
        if probes.is_empty() {
            continue;
        }
        let disjuncts: Vec<&[ClassifiedComparison]> = members
            .iter()
            .map(crate::ir::validate::RuleWitness::classified_comparisons)
            .collect();
        let plan = &probes[0].rule.plan;
        let verdict = crate::exec::verdict::CompiledVerdict::compile(
            &disjuncts,
            &|var| plan.slot_of(var),
            &|var| plan.width_of(var),
        );
        sets.push(super::RayProbeSet { verdict, probes });
    }
    Ok(sets)
}

/// One probe rule's pipeline tail — `prepare_rule_variant` minus
/// classification (a probe is always Free Join: the arbiter consumes
/// bindings, never a point fetch), minus the fold split (no aggregate),
/// minus pins (probe plan quality never gates staleness), with EVERY
/// variable sink-relevant: the verdict fold reads arbitrary condition
/// slots, so no suffix is skippable — exactly the aggregate plan's
/// relevance rule.
fn prepare_ray_probe(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
    measured: crate::ir::VarId,
    signatures: &[&crate::ir::validate::Predicate],
) -> Result<super::RayProbe> {
    let mut stats = Vec::with_capacity(normalized.occurrences.len());
    for occurrence in normalized
        .occurrences
        .iter()
        .filter(|o| o.role.participates())
    {
        let rows = match occurrence.source.edb() {
            Some(relation) => crate::plan::selectivity::relation_rows(txn, schema, relation)?,
            None => crate::plan::selectivity::INTERIOR_PLANNING_ROWS,
        };
        stats.push(crate::plan::selectivity::occurrence_stats(
            txn, cache, schema, occurrence, rows,
        )?);
    }
    let order = plan_order(normalized, schema, &stats);
    let mut fj = binary2fj(normalized, &order);
    factor(&mut fj);
    let estimates = order.estimates.clone();
    gj_split(&mut fj);
    let sink_vars: std::collections::BTreeSet<crate::ir::VarId> =
        rule.var_types().map(|(var, _)| var).collect();
    let plan = crate::plan::fj::validate_with_signatures(
        &fj, normalized, schema, signatures, estimates, &sink_vars,
    )
    .expect("binary2fj + factor + gj_split construct valid plans");
    let executor = Executor::new(&plan);
    let occurrence_count = plan.occurrences().len();
    let memo = build_view_memo(&plan);
    Ok(super::RayProbe {
        measured_slot: plan.slot_of(measured),
        rule: FreeJoinRule {
            plan,
            executor,
            finds: Vec::new(),
            dedup_spans: Box::default(),
            resolved_filters: vec![Vec::new(); occurrence_count],
            resolved_selections: vec![Vec::new(); occurrence_count],
            resolution: super::ResolutionState::Pending,
            memo,
            pinned: Box::default(),
        },
    })
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
        let columns_of = |field: bumbledb_theory::schema::FieldId| -> Vec<usize> {
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

impl SlotLayout for crate::exec::dispatch::KeyProbePlan {
    fn slot_of(&self, var: crate::ir::VarId) -> usize {
        self.slot_of(var)
    }

    fn width_of(&self, var: crate::ir::VarId) -> usize {
        self.width_of(var)
    }
}

/// Seals the DNF-derived union regime's shared-slot dedup keys (ruled
/// 2026-07-23, R2): per rule, the `VarId`-ordered spans of the vars
/// EVERY clone's plan binds — the disjuncts of one written rule share
/// one variable scope, so the `VarId` order reads the same binding
/// tuple through every clone's own layout, and the re-keyed union folds
/// the written rule's distinct full bindings
/// (`lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`). Grounding
/// may eliminate a **functionally determined** variable from one
/// clone's plan and not another's — its value is 1:1 with the surviving
/// binding either way (`plan/ground.rs`, aggregate safety), so keying
/// the intersection never merges two distinct full bindings and every
/// rule's key reads one shared vocabulary at one shared arity.
fn seal_dnf_spans(rules: &mut [PreparedRule]) {
    let inventory = |rule: &PreparedRule| -> Vec<(crate::ir::VarId, usize, usize)> {
        match rule {
            PreparedRule::FreeJoin(rule) => rule.plan.slot_spans(),
            PreparedRule::KeyProbe(rule) => {
                let mut spans: Vec<(crate::ir::VarId, usize, usize)> = rule
                    .plan
                    .vars
                    .iter()
                    .map(|binding| (binding.var, binding.slot, binding.width))
                    .collect();
                spans.sort_unstable_by_key(|(var, ..)| *var);
                spans
            }
            // A recursive rule's head is projection-shaped (folds are
            // refused through cycles) — no union key to seal.
            PreparedRule::Recursive(_) => Vec::new(),
        }
    };
    let inventories: Vec<Vec<(crate::ir::VarId, usize, usize)>> =
        rules.iter().map(inventory).collect();
    let shared: Vec<crate::ir::VarId> = inventories
        .first()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|(var, ..)| *var)
        .filter(|var| {
            inventories
                .iter()
                .all(|inv| inv.iter().any(|(bound, ..)| bound == var))
        })
        .collect();
    for (rule, inv) in rules.iter_mut().zip(&inventories) {
        let spans: Box<[(usize, usize)]> = shared
            .iter()
            .map(|var| {
                let (_, slot, width) = inv
                    .iter()
                    .find(|(bound, ..)| bound == var)
                    .expect("the shared vocabulary is each inventory's subset");
                (*slot, *width)
            })
            .collect();
        match rule {
            PreparedRule::FreeJoin(rule) => rule.dedup_spans = spans,
            PreparedRule::KeyProbe(rule) => rule.dedup_spans = spans,
            PreparedRule::Recursive(_) => {}
        }
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
            FindTerm::Measure(var) => FindSpec::Duration {
                slot: layout.slot_of(*var),
            },
            FindTerm::AggregateMeasure { op, over } => FindSpec::AggDuration {
                op: match op {
                    AggOp::Sum => crate::exec::sink::FoldOp::Sum,
                    AggOp::Min => crate::exec::sink::FoldOp::Min,
                    AggOp::Max => crate::exec::sink::FoldOp::Max,
                    AggOp::Count | AggOp::Pack => {
                        unreachable!("validated: measure folds are Sum/Min/Max")
                    }
                },
                slot: layout.slot_of(*over),
            },
            FindTerm::Aggregate { op, over } => match op {
                AggOp::Pack => FindSpec::Pack {
                    slot: layout.slot_of(over.expect("validated: Pack carries a variable")),
                },
                AggOp::Count => FindSpec::Agg(crate::exec::sink::AggSpec::Count),
                AggOp::Sum | AggOp::Min | AggOp::Max => {
                    let var = over.expect("validated: Sum/Min/Max carry a variable");
                    let fold = match op {
                        AggOp::Sum => crate::exec::sink::FoldOp::Sum,
                        AggOp::Min => crate::exec::sink::FoldOp::Min,
                        AggOp::Max => crate::exec::sink::FoldOp::Max,
                        AggOp::Count | AggOp::Pack => unreachable!("handled above"),
                    };
                    FindSpec::Agg(crate::exec::sink::AggSpec::Fold {
                        op: fold,
                        slot: layout.slot_of(var),
                        width: layout.width_of(var),
                        signed: matches!(rule.var_type(var), ValueType::I64),
                    })
                }
            },
        })
        .collect()
}

/// The key-probe fast lane's find table: `Some` for key-probe plans whose finds
/// are all plain variables. Types come from the predicate's columns —
/// find order IS column order.
fn key_probe_find_table(
    key_probe: &crate::exec::dispatch::KeyProbePlan,
    finds: &[FindSpec],
    columns: &[crate::ir::validate::PredicateColumn],
) -> Option<Vec<(bumbledb_theory::schema::FieldId, ValueType)>> {
    finds
        .iter()
        .zip(columns)
        .map(|(spec, column)| match spec {
            FindSpec::Var { slot, .. } => {
                let var = key_probe
                    .vars
                    .iter()
                    .find(|v| v.slot == *slot)
                    .expect("find slots come from the key-probe plan's layout");
                Some((var.field, column.ty.clone()))
            }
            // aggregate and measure key_probes keep the sink path
            FindSpec::Agg(_)
            | FindSpec::Pack { .. }
            | FindSpec::Duration { .. }
            | FindSpec::AggDuration { .. } => None,
        })
        .collect::<Option<Vec<_>>>()
}

/// The rule-disjointness proof (docs/architecture/40-execution.md § set
/// semantics) — retained as diagnostic knowledge for introspection, run over the
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

/// The multi-rule provenance judgment (ruled 2026-07-23, R2): a
/// surviving rule set minted wholly by ONE written rule is DNF-derived
/// — [`seal_dnf_spans`] writes its shared-slot dedup keys and the union
/// re-keys on them; any other set is hand-written, keying the head
/// projection.
fn dnf_derived(written: &[Option<u16>]) -> bool {
    written
        .first()
        .copied()
        .flatten()
        .is_some_and(|minting| written.iter().all(|rule| *rule == Some(minting)))
}

/// The dense group domains (finding 049): per group position in find
/// order, the schema-proven radix — every group word must prove one (a
/// closed reference or bool; single-word by construction, so interval
/// and measure group keys stay open) and the product must fit the dense
/// cap, or the sink keeps the open-domain map. Empty = open.
fn group_radixes(rule: &RuleWitness<'_>) -> Vec<u16> {
    let mut radixes = Vec::new();
    for term in &rule.rule().finds {
        match term {
            FindTerm::Var(var) => match rule.dense_domain(*var) {
                Some(radix) if radix > 0 => radixes.push(radix),
                _ => return Vec::new(),
            },
            FindTerm::Measure(_) => return Vec::new(),
            FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. } => {}
        }
    }
    if radixes.is_empty() {
        return Vec::new();
    }
    let capped = radixes.iter().try_fold(1u32, |product, radix| {
        product
            .checked_mul(u32::from(*radix))
            .filter(|product| *product <= crate::exec::sink::DENSE_GROUPS_CAP)
    });
    if capped.is_none() {
        return Vec::new();
    }
    radixes
}

/// Builds the sink matching the head shape (the variant is fixed per
/// prepared query — an enum, not `dyn`), aimed at rule 0's binding
/// layout. The program regime structurally selects single-rule binding
/// dedup, witnessed elision, or the mandatory union seen-set — keyed by
/// provenance (R2). `dense_groups` is the single-rule dense group
/// domain proof (049); empty keeps the open-domain map.
fn make_sink(
    finds: &[FindSpec],
    slot_count: usize,
    regime: SinkRegime<'_>,
    hint: usize,
    dense_groups: &[u16],
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
        let sink = match regime {
            SinkRegime::SingleRule(Some(witness)) => {
                AggregateSink::without_seen_set(finds, slot_count, witness, hint, dense_groups)
            }
            SinkRegime::SingleRule(None) => {
                AggregateSink::with_capacity_hint(finds, slot_count, hint, dense_groups)
            }
            SinkRegime::Union => AggregateSink::for_union(finds, slot_count, hint),
            SinkRegime::DnfUnion(spans) => {
                AggregateSink::for_dnf_union(finds, slot_count, spans, hint)
            }
        };
        EitherSink::Aggregate(Box::new(sink))
    }
}

#[derive(Debug, Clone, Copy)]
enum SinkRegime<'r> {
    SingleRule(Option<DistinctWitness>),
    /// Hand-written multi-rule: the head-projection union key.
    Union,
    /// DNF-derived multi-rule (R2): the shared-slot union key — rule
    /// 0's `VarId`-ordered spans.
    DnfUnion(&'r [(usize, usize)]),
}
