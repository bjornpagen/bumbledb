use super::{
    AggregateSink, Bindings, Colt, EitherSink, Executor, FindSpec, FreeJoinRule, KeyProbeRule,
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
use crate::plan::fj::{
    DisjointWitness, DistinctWitness, binary2fj, factor, fold_split, gj_split,
    provably_disjoint_rules, provably_distinct,
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
    prepare_witnessed(
        txn,
        cache,
        schema,
        &witness,
        crate::ir::render::render(schema, query),
    )
}

/// The pipeline after the roster — normalize → ground → per-rule prepare
/// → sink and binding artifacts, over an already-sealed witness. Two
/// callers, one roster each: [`prepare`] seals under the query roster;
/// [`prepare_program`]'s degenerate route passes the program witness's
/// output predicate, already sealed under the PROGRAM roster. The
/// distinction is load-bearing for params: they are program-global (one
/// binding surface, `docs/architecture/20-query-ir.md` § engine
/// recursion), so a degenerate program's output predicate may be locally
/// gapped yet roster-valid, and its bind contract must carry the unified
/// table — re-validating it under the query roster would refuse the
/// former and shrink the latter.
fn prepare_witnessed<'s, S>(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &'s Schema,
    witness: &crate::ir::validate::ValidatedQuery,
    rendered: String,
) -> Result<PreparedQuery<'s, S>> {
    let normalized = {
        let _s = obs::span(obs::names::NORMALIZE, obs::Category::Prepare);
        normalize(schema, witness)
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
        || make_sink(&[], 0, SinkProgram::SingleRule(None), 0, &[]),
        |first| {
            let program = if rules.len() > 1 {
                if dnf_derived(&written) {
                    SinkProgram::DnfUnion(first.dedup_spans())
                } else {
                    SinkProgram::Union
                }
            } else {
                SinkProgram::SingleRule(first.distinct_witness())
            };
            make_sink(
                first.finds(),
                first.slot_count(),
                program,
                output_hint,
                &dense_groups,
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
        answer_scratch: Vec::new(),
        resolve_memo: ResolveMemo::new(),
        determinant_key: Vec::new(),
        rendered,
        marker: std::marker::PhantomData,
    })
}

/// Prepares a program — the recursion cut's prepare surface
/// (`docs/architecture/40-execution.md` § the fixpoint driver).
/// The whole program validates under the
/// program roster; then:
///
/// * **The degenerate form takes zero new code paths**: a no-`Idb`
///   program IS its output predicate's query
///   (`lean/Bumbledb/Exec/Fixpoint.lean: degenerate_embedding`), and it
///   routes through the one shared pipeline ([`prepare_witnessed`])
///   carrying the program witness's output predicate — same pipeline,
///   same artifact, and the program-global param table intact (one
///   binding surface; the query roster must never re-judge what the
///   program roster sealed).
/// * **A recursive program prepares per predicate through the ordinary
///   per-rule pipeline** (strata above the output's are never
///   evaluated — `evalProgramAt`'s reading — and never prepared): each
///   recursive rule mints its k delta-variant plans (`DeltaVariant`,
///   one per same-stratum positive `Idb` atom), delta and accumulated
///   occurrences costed on the selectivity ladder's floors; interior
///   predicates get projection-shaped seen-set sinks; the output keeps
///   the head-owned sink. Pin-at-prepare: no round ever re-plans.
///
/// # Errors
///
/// The program roster's `Validation` errors; planner caps;
/// `Lmdb`/`Corruption` from the statistics reads.
///
/// # Panics
///
/// Only on programmer-invariant violations (plan construction; an
/// `Idb`-reading rule can never classify as a key probe).
#[expect(
    clippy::too_many_lines,
    reason = "the program prepare reads as one protocol: degenerate embedding, per-predicate pipeline, whole-query artifacts"
)]
pub(crate) fn prepare_program<'s, S>(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &'s Schema,
    program: &crate::ir::Program,
) -> Result<PreparedQuery<'s, S>> {
    let _prepare = obs::span(obs::names::PREPARE, obs::Category::Prepare);
    let witness = crate::ir::validate::validate_program(schema, program)?;
    let has_idb = program
        .predicates
        .iter()
        .flat_map(|def| def.rules.iter())
        .flat_map(|rule| rule.atoms.iter().chain(&rule.negated))
        .any(|atom| atom.source.idb().is_some());
    if !has_idb {
        // The degenerate embedding: the output predicate, as the query
        // it is — prepared from the PROGRAM witness, never re-validated
        // under the query roster. Params are program-global (one
        // binding surface), so the output predicate alone may be
        // locally gapped yet roster-valid, and its bind contract must
        // demand exactly the unified table — the same surface the
        // fixpoint arm enforces.
        return prepare_witnessed(
            txn,
            cache,
            schema,
            witness.output_witness(),
            crate::ir::render::render_program(schema, program),
        );
    }

    let count = program.predicates.len();
    let output = witness.output();
    let strata = witness.strata();
    let top_stratum = strata[usize::from(output.0)];
    let signatures: Vec<&crate::ir::validate::Predicate> = (0..count)
        .map(|p| {
            witness
                .witness(crate::ir::PredId(u16::try_from(p).expect("capped")))
                .predicate()
        })
        .collect();

    let mut predicates = Vec::with_capacity(count);
    let mut subsumed_record = Vec::new();
    let mut dead_record = Vec::new();
    let mut disjoint_rules = None;
    // The output predicate's surviving written-rule provenance (R2) —
    // the whole-query sink regime splits on it below.
    let mut output_written = Vec::new();
    for p in 0..count {
        let pred_id = crate::ir::PredId(u16::try_from(p).expect("capped"));
        let stratum = strata[p];
        if stratum > top_stratum {
            // Above the output's stratum: never evaluated
            // (`evalProgramAt` runs strata through the output's own),
            // so never prepared — no statistics read, no plan, no sink.
            predicates.push(crate::api::prepared::fixpoint::FixpointPredicate {
                stratum,
                recursive: false,
                field_types: Vec::new(),
                rules: Vec::new(),
                sink: None,
                units: 0,
            });
            continue;
        }
        let wq = witness.witness(pred_id);
        let normalized = crate::ir::normalize::normalize_predicate(schema, wq, &signatures);
        if pred_id == output {
            // The rule-disjointness diagnostic, output predicate only
            // (the record surfaces on the query-level stats).
            disjoint_rules = disjointness(wq, &normalized, schema);
        }
        let (survivors, subsumed) = ground_program(normalized, wq, schema);
        if pred_id == output {
            subsumed_record = subsumed;
        }
        let predicate = wq.predicate().clone();
        let mut rules: Vec<PreparedRule> = Vec::with_capacity(survivors.len());
        for (rule_idx, normalized_rule) in survivors {
            if let Some(reason) = &normalized_rule.dead {
                if pred_id == output {
                    dead_record.push(crate::api::stats::DeadRule {
                        rule: u16::try_from(rule_idx).expect("rule count fits u16"),
                        rendered: reason.clone(),
                    });
                }
                continue;
            }
            let rule = wq.rule(rule_idx);
            if pred_id == output {
                output_written.push(rule.written());
            }
            // The recursive atoms: positive occurrences reading this
            // predicate's own stratum (same SCC — the strata judge's
            // witness). Negated and fold-input same-stratum reads were
            // refused at validation, so positives are the whole set.
            let recursive_occs: Vec<crate::ir::normalize::OccId> = normalized_rule
                .occurrences
                .iter()
                .filter(|occ| occ.role.participates())
                .filter_map(|occ| occ.source.idb().map(|q| (occ.occ_id, q)))
                .filter(|(_, q)| strata[usize::from(q.0)] == stratum)
                .map(|(occ_id, _)| occ_id)
                .collect();
            if recursive_occs.is_empty() {
                rules.push(prepare_rule_variant(
                    txn,
                    cache,
                    schema,
                    &rule,
                    &normalized_rule,
                    &predicate.columns,
                    &signatures,
                    None,
                )?);
                continue;
            }
            // The typed variant sum (40-execution.md § the fixpoint driver): k variants
            // through the ordinary pipeline, minted by this one parse
            // and consumed totally by the driver.
            let mut variants = Vec::with_capacity(recursive_occs.len());
            for delta in recursive_occs {
                let prepared = prepare_rule_variant(
                    txn,
                    cache,
                    schema,
                    &rule,
                    &normalized_rule,
                    &predicate.columns,
                    &signatures,
                    Some(delta),
                )?;
                let PreparedRule::FreeJoin(fj) = prepared else {
                    unreachable!("an Idb-reading rule never classifies as a key probe")
                };
                variants.push(super::DeltaVariant { delta, rule: fj });
            }
            rules.push(PreparedRule::Recursive(super::RecursiveRule {
                variants: variants.into_boxed_slice(),
            }));
        }
        let units: usize = rules
            .iter()
            .map(|rule| match rule {
                PreparedRule::Recursive(rule) => rule.variants.len(),
                PreparedRule::FreeJoin(_) | PreparedRule::KeyProbe(_) => 1,
            })
            .sum();
        let recursive = rules
            .iter()
            .any(|rule| matches!(rule, PreparedRule::Recursive(_)));
        // Interior predicates own projection-shaped seen-sets (the
        // strata roster keeps folds out of interior heads — the
        // executable-class item); the output keeps the main sink.
        let sink = if pred_id == output {
            None
        } else {
            let hint = output_hint(&rules);
            Some(rules.first().map_or_else(
                || crate::exec::sink::ProjectionSink::with_capacity_hint(&[], 0, 0),
                |first| {
                    crate::exec::sink::ProjectionSink::with_capacity_hint(
                        first.finds(),
                        first.slot_count(),
                        hint,
                    )
                },
            ))
        };
        predicates.push(crate::api::prepared::fixpoint::FixpointPredicate {
            stratum,
            recursive,
            field_types: signatures[p]
                .columns
                .iter()
                .map(|column| column.ty.type_desc())
                .collect(),
            rules,
            sink,
            units,
        });
    }

    // The whole-query artifacts, aimed at the OUTPUT predicate: sink
    // shape, result typing, bind contracts (params are program-global —
    // every per-predicate witness carries the one unified table).
    let out_wq = witness.output_witness();
    let predicate = out_wq.predicate().clone();
    let params = param_specs(out_wq);
    if predicates[usize::from(output.0)].rules.len() > 1 && dnf_derived(&output_written) {
        seal_dnf_spans(&mut predicates[usize::from(output.0)].rules);
    }
    let output_rules = &predicates[usize::from(output.0)].rules;
    let output_hint_rows = output_hint(output_rules);
    let sink = output_rules.first().map_or_else(
        || make_sink(&[], 0, SinkProgram::SingleRule(None), 0, &[]),
        |first| {
            let regime = if output_rules.len() > 1 {
                if dnf_derived(&output_written) {
                    SinkProgram::DnfUnion(first.dedup_spans())
                } else {
                    SinkProgram::Union
                }
            } else {
                SinkProgram::SingleRule(first.distinct_witness())
            };
            // Program sinks keep the open-domain group map: the output
            // predicate may be recursive and the dense proof is the
            // single-rule QUERY path's optimization (finding 049).
            make_sink(
                first.finds(),
                first.slot_count(),
                regime,
                output_hint_rows,
                &[],
            )
        },
    );
    let disjoint_rules = (output_rules.len() > 1).then_some(disjoint_rules).flatten();
    let bindings = Bindings::new(
        predicates
            .iter()
            .flat_map(|pred| pred.rules.iter())
            .map(PreparedRule::slot_count)
            .max()
            .unwrap_or(0),
    );
    let unresolved_literals = predicates
        .iter()
        .flat_map(|pred| pred.rules.iter())
        .map(pending_literals)
        .sum();
    // The program in the rule notation (`ir/render::render_program`):
    // interior predicates named `p{id}`, output rules bare — the
    // notation's own program form.
    let rendered = crate::ir::render::render_program(schema, program);
    // The per-stratum membership table, computed once so the driver's
    // stratum walk allocates nothing.
    let strata_members: Vec<Vec<usize>> = (0..=top_stratum)
        .map(|s| {
            (0..count)
                .filter(|&p| strata[p] == s)
                .collect::<Vec<usize>>()
        })
        .collect();

    Ok(PreparedQuery {
        schema,
        env_instance: txn.env_instance(),
        disjoint_rules,
        subsumed: subsumed_record,
        dead: dead_record,
        program: Program::Fixpoint(Box::new(crate::api::prepared::fixpoint::FixpointProgram {
            predicates,
            output,
            top_stratum,
            strata_members,
            rounds_budget: crate::api::prepared::fixpoint::DEFAULT_FIXPOINT_ROUNDS,
            tuples_budget: crate::api::prepared::fixpoint::DEFAULT_FIXPOINT_TUPLES,
            scratch: crate::api::prepared::fixpoint::FixpointScratch::default(),
        })),
        predicate,
        params,
        resolved_params: Vec::new(),
        unresolved_literals,
        missed_params: Vec::new(),
        sink,
        bindings,
        answer_scratch: Vec::new(),
        resolve_memo: ResolveMemo::new(),
        determinant_key: Vec::new(),
        rendered,
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
            // Variant estimates share the floors; variant 0 speaks.
            PreparedRule::Recursive(rule) => {
                let plan = &rule.variants[0].rule.plan;
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
        PreparedRule::Recursive(rule) => rule
            .variants
            .iter()
            .map(|variant| plan_pending_literals(&variant.rule.plan))
            .sum(),
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
) -> Result<PreparedRule> {
    prepare_rule_variant(txn, cache, schema, rule, normalized, columns, &[], None)
}

/// [`prepare_rule`] with the program surface: the sealed signatures
/// (`Idb` occurrences' field→column spans) and — for one delta variant
/// of a recursive rule — the marked delta occurrence, whose statistics
/// take the ladder's delta floor while other `Idb` occurrences take the
/// accumulated floor (`plan/selectivity.rs`; 40-execution.md § the fixpoint driver, the
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
            // Written by `seal_dnf_spans` iff the program is a
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
        // An `Idb` occurrence pins nothing (20-query-ir.md § engine recursion's
        // consumer table): its cardinality is prepare-unknowable, so it
        // reads no row counter and costs on the selectivity ladder's
        // floors — the delta floor for the variant's marked occurrence,
        // the accumulated floor for every other predicate read — the
        // staleness surface already knows the shape (negated and
        // grounding-discharged occurrences carry no pin today).
        let Some(relation) = occurrence.source.edb() else {
            let floor = if delta == Some(occurrence.occ_id) {
                crate::plan::selectivity::DELTA_PLANNING_CARDINALITY
            } else {
                crate::plan::selectivity::ACCUMULATED_PLANNING_CARDINALITY
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
    if rule
        .rule()
        .finds
        .iter()
        .any(|term| matches!(term, FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. }))
    {
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
        &fj,
        normalized,
        schema,
        signatures,
        estimates,
        &sink_vars,
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
        // Written by `seal_dnf_spans` iff the program is a DNF-derived
        // union; empty (and never read) otherwise.
        dedup_spans: Box::default(),
        resolved_filters: vec![Vec::new(); occurrence_count],
        resolved_selections: vec![Vec::new(); occurrence_count],
        resolution: super::ResolutionState::Pending,
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
                // — a key variable's slot, or the interval measure's
                // two-slot span (the sink parses it onto a derived word
                // with ray poisoning — R5).
                AggOp::ArgMax { key } | AggOp::ArgMin { key } => {
                    let carry = over.expect("validated: Arg carries a variable");
                    FindSpec::Arg {
                        slot: layout.slot_of(carry),
                        width: layout.width_of(carry),
                        key: match key {
                            crate::ir::ArgKey::Var(var) => {
                                crate::exec::sink::ProjSource::Slot(layout.slot_of(*var))
                            }
                            crate::ir::ArgKey::Measure(var) => {
                                crate::exec::sink::ProjSource::Measure {
                                    start: layout.slot_of(*var),
                                }
                            }
                        },
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
            FindSpec::Agg { .. }
            | FindSpec::Arg { .. }
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
    program: SinkProgram<'_>,
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
        let sink = match program {
            SinkProgram::SingleRule(Some(witness)) => {
                AggregateSink::without_seen_set(finds, slot_count, witness, hint, dense_groups)
            }
            SinkProgram::SingleRule(None) => {
                AggregateSink::with_capacity_hint(finds, slot_count, hint, dense_groups)
            }
            SinkProgram::Union => AggregateSink::for_union(finds, slot_count, hint),
            SinkProgram::DnfUnion(spans) => {
                AggregateSink::for_dnf_union(finds, slot_count, spans, hint)
            }
        };
        EitherSink::Aggregate(Box::new(sink))
    }
}

#[derive(Debug, Clone, Copy)]
enum SinkProgram<'r> {
    SingleRule(Option<DistinctWitness>),
    /// Hand-written multi-rule: the head-projection union key.
    Union,
    /// DNF-derived multi-rule (R2): the shared-slot union key — rule
    /// 0's `VarId`-ordered spans.
    DnfUnion(&'r [(usize, usize)]),
}
