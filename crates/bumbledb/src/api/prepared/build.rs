use super::{
    AggregateSink, Binding, Bindings, Colt, EitherSink, Executor, FindSpec, FreeJoinRule,
    KeyProbeRule, PreparedInterior, PreparedPipeline, PreparedQuery, PreparedRule, ProjectionSink,
    ResolveMemo, Schema, ValueType, ViewMemo,
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
    prepare_source(instance.schema_arc(), instance.cache(), &source, query)
}

impl<S> PreparedQuery<S> {
    /// As [`ReadInstance::prepare`], under the CALLER's work context
    /// instead of the lease's embedded one — the native runtime threads
    /// each wire operation's `WorkContext` through here so preparation,
    /// statistics reads and image construction observe that operation's
    /// cancellation, not the long-lived session lease's context.
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
        prepare_source(instance.schema_arc(), instance.cache(), &source, query)
    }
}

/// Prepare against one admitted heap instance. The prepared query pins
/// the canonical schema identity and rebuilds its images per execution.
/// # Errors
/// As [`prepare_on`].
pub(crate) fn prepare_owned<S>(
    instance: &OwnedInstance<S>,
    query: &Query,
) -> Result<PreparedQuery<S>> {
    let source = QuerySource::heap(instance, 0, super::source::heap_default_work());
    prepare_source(instance.schema_arc(), instance.cache(), &source, query)
}

fn prepare_source<S>(
    schema: &Arc<Schema>,
    cache: &Arc<ImageCache>,
    source: &QuerySource<'_>,
    query: &Query,
) -> Result<PreparedQuery<S>> {
    let schema = Arc::clone(schema);
    let images = SourceImages::bind(source, cache);
    let witness = { validate(&schema, query)? };
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
        Arc::clone(cache),
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
    let normalized = { normalize_rules(&schema, signatures, witness.rules()) };

    let survivors = ground_main(normalized, witness, &schema);

    let signature = witness.signature().clone();
    let mut rules = Vec::with_capacity(survivors.len());

    let mut written = Vec::with_capacity(survivors.len());
    let mut first_rule_idx = None;
    let mut projection_distinct = None;
    for (rule_idx, normalized_rule) in survivors {
        if normalized_rule.dead.is_some() {
            continue;
        }
        let rule = witness.rule(rule_idx);
        projection_distinct = crate::plan::fj::provably_distinct_projection(
            &normalized_rule,
            &schema,
            &rule.rule().finds,
        );
        written.push(rule.written());
        first_rule_idx.get_or_insert(rule_idx);
        rules.push(prepare_rule(
            images,
            &schema,
            &rule,
            &normalized_rule,
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
    let mut sink = sink_seed(&rules).map_or_else(
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
    // The initial experiment licenses only a store-pinned singleton head,
    // evaluated once. Heap execution deliberately remains outside this
    // initial scope, even though its runtime schema identity is checked.
    // Union/DNF, interiors, reach and computed sinks keep ordinary sets.
    // A main singleton never retargets its sink with aim.
    if rules.len() == 1
        && matches!(pinned_source, PinnedSource::Store(_))
        && interiors.is_empty()
        && matches!(reach, PreparedReach::Cq)
        && let Some(witness) = projection_distinct
        && let EitherSink::Projection(projection) = &mut sink
    {
        projection.elide_output_hashing(witness);
    }
    let interior_slots = interiors
        .iter()
        .flat_map(|interior| interior.rules.iter().map(PreparedRule::slot_count));
    let rec_max = match &reach {
        PreparedReach::Cq => 0,
        PreparedReach::Reach { driver, .. } => driver
            .base
            .iter()
            .map(PreparedRule::slot_count)
            .chain(driver.rec.iter().map(|rule| rule.plan.slot_count()))
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
                    .chain(driver.rec.iter().map(|rule| rule.finds.as_slice()))
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
            rec_id,
            derived_count,
        },
    };
    let no_text_probe = seal_no_text_probe(&pipeline, &schema, &params);
    Ok(PreparedQuery {
        schema,
        pinned: pinned_source,
        cache,
        heap_tick: 0,
        forced_fallback: false,
        execution_texts: crate::image::TextOwners::default(),
        pipeline,
        derived: super::reach::DerivedImages::default(),
        signature,
        params,
        resolved_params: Vec::new(),
        text_generation: None,
        param_word_memo: Vec::new(),
        missed_params: Vec::new(),
        sink,
        bindings,
        answer_scratch: Vec::new(),
        resolve_memo: ResolveMemo::new(),
        key_scratch: crate::image::view::ResolvedWords::default(),
        numeric_outputs,
        no_text_probe,
        rendered,
        #[cfg(test)]
        last_visits: 0,
        marker: std::marker::PhantomData,
    })
}

/// Validation anchors every probe literal/predicate to its row field and
/// every find to a variable in that row. Check the COMPLETE row, not just
/// projected fields: the shared canonical decoder visits unprojected fields
/// too. Check every declared parameter, even one normalization eliminated.
/// Other pipeline kinds retain the ordinary eager generation protocol.
pub(super) fn seal_no_text_probe(
    pipeline: &PreparedPipeline,
    schema: &Schema,
    params: &[super::ParamSpec],
) -> bool {
    let PreparedPipeline::PointProbe { rule, finds } = pipeline else {
        return false;
    };
    !rule.row.has_text()
        && schema
            .relation(rule.plan.relation)
            .fields()
            .iter()
            .all(|field| field.value_type != ValueType::String)
        && finds.iter().all(|(_, ty)| *ty != ValueType::String)
        && params.iter().all(|param| match param {
            super::ParamSpec::Scalar { ty, .. } => *ty != ValueType::String,
            super::ParamSpec::Set { elem, .. } => *elem != ValueType::String,
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
    let sink = sink_seed(&rules).map_or_else(
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
            signatures,
            rec_id,
            delta,
        )?);
    }
    let units = base.len() + rec_rules.len();
    let hint = output_hint(&base) + rec_rules.iter().map(free_join_hint).max().unwrap_or(0);
    let sink = base.first().map_or_else(
        || {
            rec_rules.first().map_or_else(
                || crate::exec::sink::ProjectionSink::with_capacity_hint(&[], 0, 0),
                |first| {
                    crate::exec::sink::ProjectionSink::with_capacity_hint(
                        &first.finds,
                        first.plan.slot_count(),
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
        frontier: crate::image::TransientImage::default(),
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

/// Stamp the validated unique self-occurrence for the planner's frontier
/// estimate. At execution it uses the same stage environment as interiors.
fn prepare_rec_arm(
    images: &SourceImages<'_>,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
    signatures: &[&crate::ir::validate::Signature],
    rec_id: crate::ir::InteriorId,
    delta: crate::ir::normalize::OccId,
) -> Result<super::FreeJoinRule> {
    let mut normalized = normalized.clone();
    stamp_rec_bind(&mut normalized, rec_id, delta);
    debug_assert!(
        normalized
            .occurrences
            .iter()
            .any(|occ| matches!(occ.bind, crate::ir::normalize::OccBind::RecDelta(_))),
        "self_occ is the rec atom normalize numbered"
    );
    let prepared = prepare_rule(images, schema, rule, &normalized, signatures)?;
    let PreparedRule::FreeJoin(fj) = prepared else {
        unreachable!("an Interior-reading rec arm never classifies as a key probe")
    };
    Ok(fj)
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
        assert_eq!(occ.occ_id, delta, "validation admits one self-read");
        occ.bind = crate::ir::normalize::OccBind::RecDelta(id);
    }
}

fn prepare_key_rule(
    schema: &Schema,
    rule: &RuleWitness<'_>,
    plan: crate::exec::dispatch::KeyProbePlan,
    distinct_witness: Option<crate::plan::fj::DistinctWitness>,
) -> KeyProbeRule {
    let finds = find_specs(rule, &plan);
    let field_types: Vec<_> = schema
        .relation(plan.relation)
        .fields()
        .iter()
        .map(|field| field.value_type)
        .collect();
    let row = crate::image::canon::RowWords::prepared(&field_types);
    KeyProbeRule {
        plan,
        row,
        distinct_witness,
        finds,
        dedup_spans: Box::default(),
    }
}

fn prepare_rule(
    images: &SourceImages<'_>,
    schema: &Schema,
    rule: &RuleWitness<'_>,
    normalized: &NormalizedQuery,
    signatures: &[&crate::ir::validate::Signature],
) -> Result<PreparedRule> {
    let distinct_witness = provably_distinct(normalized, schema);

    let classified = { classify(normalized, schema) };
    if let Some(plan) = classified {
        return Ok(PreparedRule::KeyProbe(prepare_key_rule(
            schema,
            rule,
            plan,
            distinct_witness,
        )));
    }

    let mut stats = Vec::with_capacity(normalized.occurrences.len());
    let join_variables = crate::plan::selectivity::join_variables(normalized);
    for occurrence in normalized
        .occurrences
        .iter()
        .filter(|o| o.role.participates())
    {
        if occurrence.bind.edb().is_none() {
            stats.push(crate::plan::selectivity::occurrence_stats_on(
                images,
                schema,
                occurrence,
                0,
                &join_variables,
            )?);
            continue;
        }
        let relation = occurrence
            .bind
            .edb()
            .expect("EDB bind is a stored relation");
        let rows = crate::plan::selectivity::relation_rows_on(images.source(), schema, relation)?;
        let occ_stats = crate::plan::selectivity::occurrence_stats_on(
            images,
            schema,
            occurrence,
            rows,
            &join_variables,
        )?;
        stats.push(occ_stats);
    }
    let order = { plan_order(normalized, schema, &stats) };
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
                FindTerm::Segments { .. }
                | FindTerm::Compute(_)
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

    let finds = find_specs(rule, &plan);
    let executor = Executor::new(&plan);
    let occurrence_count = plan.occurrences().len();

    let memo = { build_view_memo(&plan) };
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
    }))
}

fn build_view_memo(plan: &crate::plan::fj::ValidatedPlan) -> ViewMemo {
    let mut memo = ViewMemo::new();
    for occurrence in plan.occurrences() {
        // Logical fields may occupy several physical columns. Use the
        // validated spans for both the field's width and its first column.
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

/// Seal shared-slot dedup keys for a DNF-derived union. Every disjunct shares
/// one variable scope; `VarId`-ordered spans read the same full binding through
/// each plan layout. Distinct bindings that project to equal heads still fold.
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
                    expression: term.clone(),
                    inputs,
                }))
            }
            FindTerm::Segments { left, right, .. } => {
                FindSpec::Compute(Arc::new(crate::api::prepared::computed::OutputProgram {
                    find: find_idx,
                    expression: term.clone(),
                    inputs: [*left, *right]
                        .into_iter()
                        .map(|var| (var, layout.slot_of(var), *rule.var_type(var)))
                        .collect(),
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
            FindTerm::Segments { .. } | FindTerm::Compute(_) => return Vec::new(),
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

/// Every arm retargets one shared sink. Seed it with a computed arm when
/// present so the existing output adapter can also accept plain projections.
fn sink_seed(rules: &[PreparedRule]) -> Option<&PreparedRule> {
    rules
        .iter()
        .find(|rule| {
            rule.finds()
                .iter()
                .any(|find| matches!(find, FindSpec::Compute(_)))
        })
        .or_else(|| rules.first())
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
