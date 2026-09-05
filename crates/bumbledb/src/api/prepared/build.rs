use super::{
    AggregateSink, Binding, Bindings, Colt, EitherSink, Executor, FindSpec, FreeJoinRule,
    KeyProbeRule, OccurrencePin, PreparedInterior, PreparedPipeline, PreparedQuery, PreparedRule,
    ProjectionSink, ResolveMemo, Schema, ValueType, ViewMemo,
};

use super::source::{PinnedSource, QuerySource};
use crate::api::db::{OwnedInstance, ReadInstance};
use crate::error::Result;
use crate::exec::dispatch::classify;
use crate::image::SourceImages;
use crate::image::cache::ImageCache;
use crate::image::view::View;
use crate::ir::normalize::{NormalizedQuery, normalize_rules};
use crate::ir::validate::{RuleWitness, validate};
use crate::ir::{FindTerm, Query};
use crate::obs;
use crate::plan::fj::{
    DistinctWitness, binary2fj, factor, fold_split, gj_split, provably_distinct,
};
use crate::plan::planner::plan as plan_order;
use std::sync::Arc;

/// Prepare against one committed snapshot lease (the C05 entry
/// `ReadInstance::prepare` calls).
/// # Errors
/// Validation, statistics-read storage failure or stopped work.
/// # Panics
/// Only on programmer-invariant violations (`binary2fj` + `factor` +
/// `fold_split` + `gj_split` construct valid plans by construction).
pub(crate) fn prepare_on<S>(
    instance: &ReadInstance<'_, S>,
    query: &Query,
) -> Result<PreparedQuery<S>> {
    let source = QuerySource::store(instance.snapshot(), instance.work());
    prepare_source(
        instance.schema_arc(),
        Arc::clone(instance.cache()),
        &source,
        query,
    )
}

impl<S> PreparedQuery<S> {
    /// As [`ReadInstance::prepare`], under the CALLER's work context
    /// instead of the lease's embedded one — the native runtime threads
    /// each wire operation's bounded `WorkContext` (deadline, cancellation
    /// and byte/step budgets) through here so prepare-time statistics
    /// reads and image/interner admission observe the operation's own
    /// policy, not the long-lived session lease's unbounded ledger.
    /// # Errors
    /// As [`prepare_on`].
    ///
    /// `doc(hidden)` bridge seam (P06/W3-SESSION); embedders use
    /// [`ReadInstance::prepare`].
    #[doc(hidden)]
    pub fn prepare_with_work(
        instance: &ReadInstance<'_, S>,
        work: &crate::work::WorkContext,
        query: &Query,
    ) -> Result<Self> {
        let source = QuerySource::store(instance.snapshot(), work);
        prepare_source(
            instance.schema_arc(),
            Arc::clone(instance.cache()),
            &source,
            query,
        )
    }
}

/// Prepare against one admitted heap instance. The prepared query pins
/// `PinnedSource::Heap` and rebuilds its images per execution.
/// # Errors
/// As [`prepare_on`].
pub(crate) fn prepare_owned<S>(
    instance: &OwnedInstance<S>,
    query: &Query,
) -> Result<PreparedQuery<S>> {
    let source = QuerySource::heap(instance, 0, super::source::heap_default_work());
    prepare_source(
        instance.schema_arc(),
        Arc::clone(instance.cache()),
        &source,
        query,
    )
}

fn prepare_source<S>(
    schema: &Arc<Schema>,
    cache: Arc<ImageCache>,
    source: &QuerySource<'_>,
    query: &Query,
) -> Result<PreparedQuery<S>> {
    let schema = Arc::clone(schema);
    let images = SourceImages::bind(source, &cache);
    let _prepare = obs::span(obs::names::PREPARE);
    let witness = {
        let _s = obs::span(obs::names::VALIDATE);
        validate(&schema, query)?
    };
    let mut signatures: Vec<&crate::ir::validate::Signature> = Vec::new();
    let mut interiors = Vec::with_capacity(witness.interiors().len());
    for i in 0..witness.interiors().len() {
        interiors.push(prepare_interior(
            &images,
            &schema,
            &witness,
            i,
            &signatures,
        )?);
        signatures.push(witness.interiors()[i].signature());
    }
    let reach = match witness.rec() {
        None => PreparedReach::Cq,
        Some(rec) => {
            let rec_id = witness.rec_id().expect("Reach has rec_id");
            signatures.push(rec.signature());
            PreparedReach::Reach {
                driver: Box::new(prepare_reach(
                    &images,
                    &schema,
                    &witness,
                    rec,
                    rec_id,
                    &signatures,
                )?),
                rec_id,
                derived_count: witness.derived_count(),
            }
        }
    };
    let rendered = crate::ir::render::render(&schema, query);
    let pinned = source.pinned();
    prepare_witnessed(
        pinned,
        &images,
        Arc::clone(&cache),
        schema,
        &witness,
        rendered,
        interiors,
        reach,
        &signatures,
    )
}

enum PreparedReach {
    Cq,
    Reach {
        driver: Box<super::reach::ReachDriver>,
        rec_id: crate::ir::InteriorId,
        derived_count: u32,
    },
}

/// The pipeline after interiors and rec are prepared — normalize → ground →
/// per-rule prepare → sink and binding artifacts, over an already-sealed
/// witness.
#[expect(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "the prepare pipeline reads as one protocol: normalize, ground, per-rule prepare, probes, artifacts"
)]
fn prepare_witnessed<S>(
    pinned_source: PinnedSource,
    images: &SourceImages<'_>,
    cache: Arc<ImageCache>,
    schema: Arc<Schema>,
    witness: &crate::ir::validate::ValidatedQuery,
    rendered: String,
    interiors: Vec<PreparedInterior>,
    reach: PreparedReach,
    signatures: &[&crate::ir::validate::Signature],
) -> Result<PreparedQuery<S>> {
    let normalized = {
        let _s = obs::span(obs::names::NORMALIZE);
        normalize_rules(&schema, signatures, witness.rules())
    };

    let survivors = ground_main(normalized, witness, &schema);

    let signature = witness.signature().clone();
    let mut rules = Vec::with_capacity(survivors.len());

    let mut written = Vec::with_capacity(survivors.len());
    let mut first_rule_idx = None;
    for (rule_idx, normalized_rule) in survivors {
        if normalized_rule.dead.is_some() {
            continue;
        }
        let rule = witness.rule(rule_idx);
        written.push(rule.written());
        first_rule_idx.get_or_insert(rule_idx);
        rules.push(prepare_rule(
            images,
            &schema,
            &rule,
            &normalized_rule,
            &signature.columns,
            signatures,
        )?);
    }
    let params = param_specs(witness);

    let output_hint = output_hint(&rules);
    if rules.len() > 1 && dnf_derived(&written) {
        seal_dnf_spans(&mut rules);
    }

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
    let interior_slots = interiors
        .iter()
        .flat_map(|interior| interior.rules.iter().map(PreparedRule::slot_count));
    let rec_max = match &reach {
        PreparedReach::Cq => 0,
        PreparedReach::Reach { driver, .. } => driver
            .base
            .iter()
            .map(PreparedRule::slot_count)
            .chain(driver.rec.iter().map(|arm| arm.rule.plan.slot_count()))
            .max()
            .unwrap_or(0),
    };
    let bindings = Bindings::new(
        rules
            .iter()
            .map(PreparedRule::slot_count)
            .chain(interior_slots)
            .max()
            .unwrap_or(0)
            .max(rec_max),
    );

    let unresolved_literals = rules.iter().map(pending_literals).sum::<u32>()
        + interiors
            .iter()
            .flat_map(|interior| interior.rules.iter().map(pending_literals))
            .sum::<u32>()
        + match &reach {
            PreparedReach::Cq => 0,
            PreparedReach::Reach { driver, .. } => {
                driver.base.iter().map(pending_literals).sum::<u32>()
                    + driver
                        .rec
                        .iter()
                        .map(|arm| plan_pending_literals(&arm.rule.plan))
                        .sum::<u32>()
            }
        };
    let numeric_outputs = {
        let first_compute = |finds: &[FindSpec]| {
            finds.iter().find_map(|spec| match spec {
                FindSpec::Compute(program) => Some(crate::error::FindIndex(program.find)),
                _ => None,
            })
        };
        rules
            .iter()
            .map(PreparedRule::finds)
            .chain(
                interiors
                    .iter()
                    .flat_map(|interior| interior.rules.iter().map(PreparedRule::finds)),
            )
            .chain(match &reach {
                PreparedReach::Cq => Vec::new(),
                PreparedReach::Reach { driver, .. } => driver
                    .base
                    .iter()
                    .map(PreparedRule::finds)
                    .chain(driver.rec.iter().map(|arm| arm.rule.finds.as_slice()))
                    .collect(),
            })
            .find_map(first_compute)
    };
    let pipeline = match reach {
        PreparedReach::Cq => seal_cq_pipeline(interiors, rules, &signature.columns),
        PreparedReach::Reach {
            driver,
            rec_id,
            derived_count,
        } => PreparedPipeline::Reach {
            interiors,
            driver,
            main: rules,
            rounds_budget: super::reach::DEFAULT_REACH_ROUNDS,
            rec_id,
            derived_count,
        },
    };
    Ok(PreparedQuery {
        schema,
        pinned: pinned_source,
        cache,
        heap_tick: 0,
        forced_fallback: false,
        sink_ram: crate::exec::scratch::DEFAULT_RAM_BYTES,
        pipeline,
        tuples_budget: super::reach::DEFAULT_DERIVED_TUPLES,
        derived: super::reach::DerivedImages::default(),
        signature,
        params,
        resolved_params: Vec::new(),
        latch: super::Latch::from_count(unresolved_literals),
        param_word_memo: Vec::new(),
        missed_params: Vec::new(),
        sink,
        bindings,
        answer_scratch: Vec::new(),
        resolve_memo: ResolveMemo::new(),
        key_scratch: Vec::new(),
        numeric_outputs,
        rendered,
        nonresident: None,
        #[cfg(test)]
        last_visits: 0,
        #[cfg(test)]
        used_nonresident_text: false,
        marker: std::marker::PhantomData,
    })
}

fn prepare_interior(
    images: &SourceImages<'_>,
    schema: &Schema,
    witness: &crate::ir::validate::ValidatedQuery,
    index: usize,
    signatures: &[&crate::ir::validate::Signature],
) -> Result<PreparedInterior> {
    let inner = &witness.interiors()[index];
    let columns = &inner.signature().columns;
    let witnesses: Vec<_> = witness.interior_rules(index).collect();
    let normalized =
        crate::ir::normalize::normalize_rules(schema, signatures, witnesses.iter().copied());
    let finds: Vec<&[crate::ir::FindTerm]> = witnesses
        .iter()
        .map(|rule| rule.rule().finds.as_slice())
        .collect();
    let survivors = ground_rules(normalized, &finds, schema);
    let mut rules = Vec::with_capacity(survivors.len());
    let mut written = Vec::with_capacity(survivors.len());
    for (rule_idx, normalized_rule) in survivors {
        if normalized_rule.dead.is_some() {
            continue;
        }
        let rule = witnesses[rule_idx];
        written.push(rule.written());
        rules.push(prepare_rule(
            images,
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
    // An interior is a full stage: projection, aggregate, or computed —
    // the same sink selection as main (chapter 12's uniform nonrecursive
    // composition; the projection-only wall is deleted).
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
            make_sink(first.finds(), first.slot_count(), regime, hint, &[])
        },
    );
    Ok(PreparedInterior {
        rules,
        sink,
        field_types: columns.iter().map(|c| *c.ty()).collect(),
        units,
    })
}

fn prepare_reach(
    images: &SourceImages<'_>,
    schema: &Schema,
    witness: &crate::ir::validate::ValidatedQuery,
    rec: &crate::ir::validate::ValidatedRec,
    rec_id: crate::ir::InteriorId,
    signatures: &[&crate::ir::validate::Signature],
) -> Result<super::reach::ReachDriver> {
    let columns = &rec.signature().columns;
    let base_w: Vec<_> = rec.base_rules(witness).collect();
    let rec_w: Vec<_> = rec.step_rules(witness).collect();
    let base_norm =
        crate::ir::normalize::normalize_rules(schema, signatures, base_w.iter().copied());
    let rec_norm = crate::ir::normalize::normalize_rules(schema, signatures, rec_w.iter().copied());
    let base_finds: Vec<&[crate::ir::FindTerm]> =
        base_w.iter().map(|r| r.rule().finds.as_slice()).collect();
    let rec_finds: Vec<&[crate::ir::FindTerm]> =
        rec_w.iter().map(|r| r.rule().finds.as_slice()).collect();
    let base_surv = ground_rules(base_norm, &base_finds, schema);
    let rec_surv = ground_rules(rec_norm, &rec_finds, schema);
    let mut base = Vec::new();
    for (rule_idx, normalized_rule) in base_surv {
        if normalized_rule.dead.is_some() {
            continue;
        }
        base.push(prepare_rule(
            images,
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
        let delta = rec.arm(rule_idx).self_occ();
        rec_rules.push(prepare_rec_arm(
            images,
            schema,
            &rec_w[rule_idx],
            &normalized_rule,
            columns,
            signatures,
            rec_id,
            delta,
        )?);
    }
    let units = base.len() + rec_rules.len();
    let hint = output_hint(&base)
        + rec_rules
            .iter()
            .map(|arm| free_join_hint(&arm.rule))
            .max()
            .unwrap_or(0);
    let sink = base.first().map_or_else(
        || {
            rec_rules.first().map_or_else(
                || crate::exec::sink::ProjectionSink::with_capacity_hint(&[], 0, 0),
                |first| {
                    crate::exec::sink::ProjectionSink::with_capacity_hint(
                        &first.rule.finds,
                        first.rule.plan.slot_count(),
                        hint,
                    )
                },
            )
        },
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
        field_types: columns.iter().map(|c| *c.ty()).collect(),
        sink,
        units,
        scratch: super::reach::RecPingPong::default(),
    })
}

fn ground_rules(
    mut normalized: Vec<NormalizedQuery>,
    finds: &[&[crate::ir::FindTerm]],
    schema: &Schema,
) -> Vec<(usize, NormalizedQuery)> {
    for (rule_idx, normalized_rule) in normalized.iter_mut().enumerate() {
        if normalized_rule.dead.is_some() {
            continue;
        }
        crate::plan::ground::ground(normalized_rule, schema, finds[rule_idx]);
    }
    let subsumed: std::collections::HashSet<usize> =
        crate::plan::ground::subsume(&normalized, finds)
            .into_iter()
            .map(|deletion| deletion.rule)
            .collect();
    normalized
        .into_iter()
        .enumerate()
        .filter(|(idx, _)| !subsumed.contains(idx))
        .collect()
}

fn output_hint(rules: &[PreparedRule]) -> usize {
    rules
        .iter()
        .map(|rule| match rule {
            PreparedRule::FreeJoin(rule) => free_join_hint(rule),
            PreparedRule::KeyProbe(_) => 1,
        })
        .max()
        .unwrap_or(0)
}

fn free_join_hint(rule: &super::FreeJoinRule) -> usize {
    usize::try_from(
        rule.plan
            .estimates()
            .last()
            .copied()
            .unwrap_or(0)
            .min(1 << 21),
    )
    .expect("clamped")
}

/// Discharged occurrences count nothing: an eliminated one carries no
/// conditions, and a folded one's retained filters are plan-constant by the
/// fold's own conditions (`plan/ground/evaluate.rs`) and never resolved — a
/// fold must not block the fully-latched fast path.
fn pending_literals(rule: &PreparedRule) -> u32 {
    match rule {
        PreparedRule::FreeJoin(rule) => plan_pending_literals(&rule.plan),
        PreparedRule::KeyProbe(_) => 0,
    }
}

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
            super::ParamSpec::Set { elem: **ty, point }
        } else {
            super::ParamSpec::Scalar { ty: **ty, point }
        };
        params.push(spec);
    }
    params
}

/// The theory's query rewrite (`plan/ground.rs`): the
/// elimination-and-evaluation fixpoint per rule, independently — after
/// normalization and before statistics and the DP, with no cross-rule state; a
/// rule shrinking below its cover requirements re-validates like any rule (the
/// per-rule pipeline re-runs plan validation regardless).
fn ground_main(
    mut normalized: Vec<NormalizedQuery>,
    witness: &crate::ir::validate::ValidatedQuery,
    schema: &Schema,
) -> Vec<(usize, NormalizedQuery)> {
    for (rule_idx, normalized_rule) in normalized.iter_mut().enumerate() {
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
    let subsumed: std::collections::HashSet<usize> =
        crate::plan::ground::subsume(&normalized, &finds)
            .into_iter()
            .map(|deletion| deletion.rule)
            .collect();
    normalized
        .into_iter()
        .enumerate()
        .filter(|(idx, _)| !subsumed.contains(idx))
        .collect()
}

fn prepare_rule(
    images: &SourceImages<'_>,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
    columns: &[crate::ir::validate::SignatureColumn],
    signatures: &[&crate::ir::validate::Signature],
) -> Result<PreparedRule> {
    prepare_rule_variant(images, schema, rule, normalized, columns, signatures)
}

/// Prepare one rec arm: stamp the unique self-occurrence as `RecDelta` and
/// every other self-read as `RecAcc` before statistics run.
#[expect(
    clippy::too_many_arguments,
    reason = "the rec-arm pipeline's inputs are clearer unpacked"
)]
fn prepare_rec_arm(
    images: &SourceImages<'_>,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
    columns: &[crate::ir::validate::SignatureColumn],
    signatures: &[&crate::ir::validate::Signature],
    rec_id: crate::ir::InteriorId,
    delta: crate::ir::normalize::OccId,
) -> Result<super::RecArm> {
    let mut normalized = normalized.clone();
    stamp_rec_bind(&mut normalized, rec_id, delta);
    debug_assert!(
        normalized
            .occurrences
            .iter()
            .any(|occ| matches!(occ.bind, crate::ir::normalize::OccBind::RecDelta(_))),
        "self_occ is the rec atom normalize numbered"
    );
    let prepared = prepare_rule_variant(images, schema, rule, &normalized, columns, signatures)?;
    let PreparedRule::FreeJoin(fj) = prepared else {
        unreachable!("an Interior-reading rec arm never classifies as a key probe")
    };
    Ok(super::RecArm { delta, rule: fj })
}

fn stamp_rec_bind(
    normalized: &mut NormalizedQuery,
    rec_id: crate::ir::InteriorId,
    delta: crate::ir::normalize::OccId,
) {
    for occ in &mut normalized.occurrences {
        let Some(id) = occ.bind.interior() else {
            continue;
        };
        if id != rec_id {
            continue;
        }
        occ.bind = if occ.occ_id == delta {
            crate::ir::normalize::OccBind::RecDelta(id)
        } else {
            crate::ir::normalize::OccBind::RecAcc(id)
        };
    }
}

fn prepare_rule_variant(
    images: &SourceImages<'_>,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
    _columns: &[crate::ir::validate::SignatureColumn],
    signatures: &[&crate::ir::validate::Signature],
) -> Result<PreparedRule> {
    let distinct_witness = provably_distinct(normalized, schema);

    let classified = {
        let _s = obs::span(obs::names::CLASSIFY);
        classify(normalized, schema)
    };
    if let Some(plan) = classified {
        let finds = find_specs(rule, &plan);
        return Ok(PreparedRule::KeyProbe(KeyProbeRule {
            plan,
            distinct_witness,
            finds,

            dedup_spans: Box::default(),
        }));
    }

    let mut pins = Vec::new();

    let mut stats_span = obs::span(obs::names::STATS);
    let mut stats = Vec::with_capacity(normalized.occurrences.len());
    for occurrence in normalized
        .occurrences
        .iter()
        .filter(|o| o.role.participates())
    {
        if occurrence.bind.edb().is_none() {
            stats.push(crate::plan::selectivity::occurrence_stats_on(
                images, schema, occurrence, 0,
            )?);
            continue;
        }
        let relation = occurrence
            .bind
            .edb()
            .expect("EDB bind is a stored relation");
        let rows = crate::plan::selectivity::relation_rows_on(images.source(), schema, relation)?;
        let occ_stats =
            crate::plan::selectivity::occurrence_stats_on(images, schema, occurrence, rows)?;
        pins.push(OccurrencePin {
            occ_id: occurrence.occ_id,
            relation,
            rows,
            survivors: (!occurrence.filters.is_empty()).then_some(occ_stats.rows),
        });
        stats.push(occ_stats);
    }
    stats_span.set_count(stats.len() as u64);
    stats_span.end();
    let order = {
        let _s = obs::span(obs::names::PLAN_DP);
        plan_order(normalized, schema, &stats)
    };
    let lower_span = obs::span(obs::names::LOWER);
    let mut fj = binary2fj(normalized, &order);
    factor(&mut fj);

    if rule.rule().finds.iter().any(|term| {
        matches!(
            term,
            FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. }
        )
    }) {
        let group_key: std::collections::BTreeSet<crate::ir::VarId> = rule
            .rule()
            .finds
            .iter()
            .filter_map(|term| match term {
                FindTerm::Var(var) => Some(*var),
                // A computed output is not a plan-level group variable
                // (validation's group-key law, `ir/validate/validate.rs`);
                // its inputs reach the sink through complete bindings, and
                // the computed sink declines every scan-fold pushdown.
                FindTerm::Compute(_)
                | FindTerm::Count
                | FindTerm::Aggregate { .. }
                | FindTerm::Pack { .. } => None,
            })
            .collect();
        fold_split(&mut fj, &group_key);
    }
    gj_split(&mut fj);

    let sink_vars = rule.sink_vars();
    let plan =
        crate::plan::fj::validate_with_signatures(&fj, normalized, schema, signatures, &sink_vars)
            .expect("binary2fj + factor + fold_split + gj_split construct valid plans");
    lower_span.end();

    let finds = find_specs(rule, &plan);
    let executor = Executor::new(&plan);
    let occurrence_count = plan.occurrences().len();

    let memo = {
        let _s = obs::span(obs::names::BUILD_COLTS);
        build_view_memo(&plan)
    };
    let fallback = crate::api::prepared::fallback::FallbackRule::seal(normalized, &plan, |var| {
        *rule.var_type(var)
    });
    Ok(PreparedRule::FreeJoin(FreeJoinRule {
        plan,
        executor,
        fallback,
        finds,

        dedup_spans: Box::default(),
        resolved_filters: vec![Vec::new(); occurrence_count],
        resolved_selections: vec![Vec::new(); occurrence_count],
        resolution: super::ResolutionState::Pending,
        memo,
        pinned: pins.into_boxed_slice(),
    }))
}

fn build_view_memo(plan: &crate::plan::fj::ValidatedPlan) -> ViewMemo {
    let mut memo = ViewMemo::new();
    for occurrence in plan.occurrences() {
        // ⌈N/8⌉ words), and every field after one is shifted — spans,

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

        let selections: Vec<crate::exec::colt::SelectionLevel> = occurrence
            .selections
            .iter()
            .map(|s| {
                if matches!(
                    s.value,
                    crate::image::view::Const::ParamSet(_) | crate::image::view::Const::WordSet(_)
                ) {
                    crate::exec::colt::SelectionLevel::Set {
                        columns: columns_of(s.field),
                    }
                } else {
                    crate::exec::colt::SelectionLevel::Point {
                        columns: columns_of(s.field),
                    }
                }
            })
            .collect();
        let active = if occurrence.bind.edb().is_none() {
            Binding::Derived
        } else {
            Binding::Unbound
        };
        memo.push(Colt::new(View::Unbound, &selections, columns), active);
    }
    memo
}

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
/// 2026-07-23, R2): per rule, the `VarId`-ordered spans of the vars EVERY
/// clone's plan binds — the disjuncts of one written rule share one variable
/// scope, so the `VarId` order reads the same binding tuple through every
/// clone's own layout, and the re-keyed union folds the written rule's distinct
/// full bindings (`lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`).
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
        }
    }
}

fn find_specs(rule: &RuleWitness<'_>, layout: &impl SlotLayout) -> Vec<FindSpec> {
    rule.rule()
        .finds
        .iter()
        .enumerate()
        .map(|(find_idx, term)| match term {
            FindTerm::Var(var) => FindSpec::Var {
                slot: layout.slot_of(*var),
                width: layout.width_of(*var),
            },
            FindTerm::Compute(expr) => {
                let inputs = expr
                    .variables()
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .map(|var| (var, layout.slot_of(var), *rule.var_type(var)))
                    .collect();
                FindSpec::Compute(Arc::new(crate::api::prepared::computed::OutputProgram {
                    find: find_idx,
                    expression: expr.clone(),
                    inputs,
                }))
            }
            FindTerm::Count => FindSpec::Agg(crate::exec::sink::AggSpec::Count),
            FindTerm::Pack { over } => FindSpec::Pack {
                slot: layout.slot_of(*over),
            },
            FindTerm::Aggregate { op, over }
                if *rule.var_type(*over) == ValueType::F64
                    && matches!(op, crate::ir::FoldOp::Sum | crate::ir::FoldOp::Mean) =>
            {
                FindSpec::Agg(crate::exec::sink::AggSpec::Float {
                    op: *op,
                    slot: layout.slot_of(*over),
                })
            }
            FindTerm::Aggregate { op, over } => FindSpec::Agg(crate::exec::sink::AggSpec::Fold {
                op: *op,
                slot: layout.slot_of(*over),
                width: layout.width_of(*over),
                signed: matches!(rule.var_type(*over), ValueType::I64),
            }),
        })
        .collect()
}

fn seal_cq_pipeline(
    interiors: Vec<PreparedInterior>,
    mut rules: Vec<PreparedRule>,
    columns: &[crate::ir::validate::SignatureColumn],
) -> PreparedPipeline {
    if interiors.is_empty() && rules.len() == 1 {
        match rules.pop() {
            Some(PreparedRule::KeyProbe(rule)) => {
                if let Some(finds) = key_probe_find_table(&rule.plan, &rule.finds, columns) {
                    return PreparedPipeline::PointProbe { rule, finds };
                }
                rules.push(PreparedRule::KeyProbe(rule));
            }
            Some(other) => rules.push(other),
            None => {}
        }
    }
    PreparedPipeline::Cq { interiors, rules }
}

fn key_probe_find_table(
    key_probe: &crate::exec::dispatch::KeyProbePlan,
    finds: &[FindSpec],
    columns: &[crate::ir::validate::SignatureColumn],
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
                Some((var.field, *column.ty()))
            }
            FindSpec::Compute(_) | FindSpec::Agg(_) | FindSpec::Pack { .. } => None,
        })
        .collect::<Option<Vec<_>>>()
}

fn dnf_derived(written: &[Option<u16>]) -> bool {
    written
        .first()
        .copied()
        .flatten()
        .is_some_and(|minting| written.iter().all(|rule| *rule == Some(minting)))
}

fn group_radixes(rule: &RuleWitness<'_>) -> Vec<u16> {
    let mut radixes = Vec::new();
    for term in &rule.rule().finds {
        match term {
            FindTerm::Var(var) => match rule.dense_domain(*var) {
                Some(radix) if radix > 0 => radixes.push(radix),
                _ => return Vec::new(),
            },
            // A computed output joins the group key through an appended
            // slot the radix table cannot cover: stay hashed.
            FindTerm::Compute(_) => return Vec::new(),
            FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. } => {}
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

fn make_sink(
    finds: &[FindSpec],
    slot_count: usize,
    regime: SinkRegime<'_>,
    hint: usize,
    dense_groups: &[u16],
) -> EitherSink {
    if finds
        .iter()
        .any(|spec| matches!(spec, FindSpec::Compute(_)))
    {
        // Computed outputs run through the adapter: lower every Compute
        // to an appended output slot, build the inner sink over the
        // widened layout, and let the adapter evaluate programs per
        // surviving binding (after all input predicates — chapter 12's
        // stage error boundary). Dense-group radixes never cover the
        // appended slots, so the inner sink stays hashed.
        let (lowered, programs, total) = crate::api::prepared::computed::lower(finds, slot_count);
        let inner = make_plain_sink(&lowered, total, regime, hint, &[]);
        return EitherSink::Computed(Box::new(crate::api::prepared::computed::ComputedSink::new(
            inner, programs, slot_count, total,
        )));
    }
    make_plain_sink(finds, slot_count, regime, hint, dense_groups)
}

fn make_plain_sink(
    finds: &[FindSpec],
    slot_count: usize,
    regime: SinkRegime<'_>,
    hint: usize,
    dense_groups: &[u16],
) -> EitherSink {
    let all_plain = finds
        .iter()
        .all(|spec| matches!(spec, FindSpec::Var { .. }));
    if all_plain {
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

    Union,

    DnfUnion(&'r [(usize, usize)]),
}
