use super::*;
use crate::api::stats::ExecutionStats;
use crate::encoding::{ValueRef, encode_fact};
use crate::exec::colt::Colt;
use crate::exec::dispatch::classify;
use crate::exec::run::{Bindings, Executor, NoopCounters};
use crate::exec::sink::ProjectionSink;
use crate::image::view::{Const, FilterPredicate, apply};
use crate::ir::normalize::{AntiProbe, NormalizedQuery, OccId, Occurrence, Role, SlotWidth};
use crate::ir::{CmpOp, VarId};
use crate::plan::fj::{ValidatedPlan, binary2fj, factor, validate};
use crate::plan::planner::JoinOrder;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

fn schema(relations: usize) -> Schema {
    SchemaDescriptor {
        relations: (0..relations)
            .map(|r| RelationDescriptor {
                extension: None,
                name: format!("R{r}").into(),
                fields: vec![
                    FieldDescriptor {
                        name: "a".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "b".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            })
            .collect(),
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn views_of(
    dir: &TempDir,
    schema: &Schema,
    data: &[Vec<(u64, u64)>],
) -> Vec<Arc<crate::image::RelationImage>> {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (rel, rows) in data.iter().enumerate() {
        let rel_id = RelationId(u32::try_from(rel).expect("small"));
        for (a, b) in rows {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(*a), ValueRef::U64(*b)],
                schema.relation(rel_id).layout(),
                &mut bytes,
            );
            delta.insert(&view, rel_id, &bytes).expect("insert");
        }
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    (0..data.len())
        .map(|rel| {
            let rel_id = RelationId(u32::try_from(rel).expect("small"));
            crate::image::build(&txn, schema, rel_id).expect("build")
        })
        .collect()
}

fn colts_for(plan: &ValidatedPlan, images: &[Arc<crate::image::RelationImage>]) -> Vec<Colt> {
    plan.occurrences()
        .iter()
        .map(|occurrence| {
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
                                .expect("plan vars");
                            usize::from(field.0)
                        })
                        .collect()
                })
                .collect();
            Colt::new(
                apply(
                    &images[usize::try_from(occurrence.relation.0).expect("small")],
                    &[],
                    &[],
                    Vec::new(),
                )
                .expect("no measure filters"),
                &[],
                columns,
            )
        })
        .collect()
}

fn occurrence(occ: u16, relation: u32, vars: &[(u16, u16)]) -> Occurrence {
    Occurrence {
        occ_id: OccId(occ),
        relation: RelationId(relation),
        role: Role::Positive,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
        filters: vec![],
    }
}

/// A negated occurrence: joins no node, probed through its anti-probe.
fn negated(occ: u16, relation: u32, vars: &[(u16, u16)]) -> Occurrence {
    Occurrence {
        role: Role::Negated,
        ..occurrence(occ, relation, vars)
    }
}

/// Assembles a `NormalizedQuery` the way `normalize` would: anti-probe
/// descriptors derived from the negated occurrences, every variable one
/// slot wide (scalar fixtures).
fn normalized(occurrences: Vec<Occurrence>) -> NormalizedQuery {
    let anti_probes = occurrences
        .iter()
        .filter(|o| o.role == Role::Negated)
        .map(|o| AntiProbe {
            occurrence: o.occ_id,
            probe_bindings: o.vars.clone(),
        })
        .collect();
    let slot_widths: BTreeMap<VarId, SlotWidth> = occurrences
        .iter()
        .flat_map(|o| o.vars.iter().map(|(_, v)| (*v, SlotWidth::ONE)))
        .collect();
    NormalizedQuery {
        dead: None,
        occurrences,
        residuals: vec![],
        word_residuals: vec![],
        allen_residuals: vec![],
        duration_residuals: Vec::new(),
        anti_probes,
        slot_widths,
    }
}

#[test]
fn estimates_and_actuals_populate_for_a_join_fixture() {
    let dir = TempDir::new("introspect-join");
    let schema = schema(2);
    // R0: 5 rows; R1: joins on R0's fresh (reference-walk shape).
    let r0: Vec<(u64, u64)> = (0..5).map(|i| (i, i * 10)).collect();
    let r1: Vec<(u64, u64)> = (0..20).map(|i| (i, i % 5)).collect();
    let views = views_of(&dir, &schema, &[r0, r1]);
    let normalized = normalized(vec![
        occurrence(0, 0, &[(0, 0), (1, 1)]),
        occurrence(1, 1, &[(1, 0), (0, 2)]),
    ]);
    let order = JoinOrder {
        order: vec![OccId(0), OccId(1)],
        estimates: vec![5, 20],
    };
    let mut fj = binary2fj(&normalized, &order);
    factor(&mut fj);
    // The sink projects var 2: sink_vars must say so (the production
    // path passes the witness's group key) — with the D2 first-emit
    // skip, an empty set would let the unwind prune real output.
    let sink_vars = BTreeSet::from([VarId(2)]);
    let plan = validate(
        &fj,
        &normalized,
        &schema,
        order.estimates.clone(),
        &sink_vars,
    )
    .expect("valid plan");

    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(2))]);
    let mut counters = CountingCounters::new(&plan);
    Executor::new(&plan)
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");

    // 20 R1 rows each match exactly one R0 row: actual after the last
    // node is 20 emits; estimates rendered beside them.
    assert_eq!(counters.emits(), 20);
    assert!(counters.actual_after(0) > 0);
    let rule = counters.into_rule_stats(&plan, &schema, Vec::new(), 0);
    let report = IntrospectionReport {
        header: None,
        rules: vec![RulePlan::FreeJoin(&plan)],
        stats: ExecutionStats {
            introspection_version: crate::api::stats::INTROSPECTION_VERSION,
            emits: rule.emitted,
            rules: vec![rule],
            disjoint_rules: None,
            subsumed: Vec::new(),
            dead: Vec::new(),
        },
    };
    let text = format!("{report}");
    assert!(text.contains("estimated=5"));
    assert!(text.contains("emitted bindings: 20"));
}

#[test]
fn the_skew_fixture_shows_the_expected_cover_choice() {
    // The correct-but-slow regression detector (50-validation): on a
    // constructed skew fixture, the histogram must show the forced
    // small side chosen with an Exact label.
    let dir = TempDir::new("introspect-skew");
    let schema = schema(2);
    let r: Vec<(u64, u64)> = (0..500).map(|i| (i, i % 250)).collect();
    let s: Vec<(u64, u64)> = vec![(0, 0), (1, 1)];
    let views = views_of(&dir, &schema, &[r, s]);
    let normalized = normalized(vec![
        occurrence(0, 0, &[(1, 0), (0, 1)]),
        occurrence(1, 1, &[(1, 0), (0, 2)]),
    ]);
    // GJ-style node with both as covers.
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![
            crate::plan::fj::Node {
                subatoms: vec![
                    crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(0)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(0)],
                    },
                ],
            },
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(1)],
                }],
            },
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(1),
                    vars: vec![VarId(2)],
                }],
            },
        ],
    };
    let plan =
        validate(&plan, &normalized, &schema, vec![0; 3], &BTreeSet::new()).expect("valid plan");
    let mut colts = colts_for(&plan, &views);
    // Pre-force the tiny side so its Exact(2) beats Estimate(500).
    let s_root = Colt::root();
    colts[1].get(s_root, 0, &[0]);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(0))]);
    let mut counters = CountingCounters::new(&plan);
    Executor::new(&plan)
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");

    // Node 0 chose subatom 1 (the forced small side), labeled Exact.
    assert_eq!(counters.cover_histogram(0, 1)[0], 1);
    assert_eq!(counters.cover_histogram(0, 0), [0, 0]);
    let rule = counters.into_rule_stats(&plan, &schema, Vec::new(), 0);
    let report = IntrospectionReport {
        header: None,
        rules: vec![RulePlan::FreeJoin(&plan)],
        stats: ExecutionStats {
            introspection_version: crate::api::stats::INTROSPECTION_VERSION,
            emits: rule.emitted,
            rules: vec![rule],
            disjoint_rules: None,
            subsumed: Vec::new(),
            dead: Vec::new(),
        },
    };
    assert!(format!("{report}").contains("exact=1"));
}

#[test]
fn key_probe_queries_report_their_classification() {
    let schema = schema(1);
    let normalized = normalized(vec![Occurrence {
        occ_id: OccId(0),
        relation: RelationId(0),
        role: Role::Positive,
        vars: vec![(FieldId(1), VarId(0))],
        filters: vec![FilterPredicate::Compare {
            field: FieldId(0),
            op: CmpOp::Eq,
            value: Const::Word(5),
        }],
    }]);
    let key_probe = classify(&normalized, &schema).expect("key probe");
    let report = IntrospectionReport {
        header: None,
        rules: vec![RulePlan::KeyProbe(&key_probe)],
        stats: ExecutionStats {
            introspection_version: crate::api::stats::INTROSPECTION_VERSION,
            rules: vec![crate::api::stats::RuleStats {
                distinct_bindings: true,
                nodes: Vec::new(),
                eliminated: Vec::new(),
                folded: Vec::new(),
                pinned: Vec::new(),
                emitted: 0,
                absorbed: 0,
                key_probe: Some(crate::api::stats::KeyProbeStats { hit: true }),
            }],
            emits: 0,
            disjoint_rules: None,
            subsumed: Vec::new(),
            dead: Vec::new(),
        },
    };
    let text = format!("{report}");
    assert!(text.contains("key probe"));
    assert!(text.contains("key statement: 0"));
}

#[test]
fn noop_counters_are_zero_sized_and_the_normal_path_carries_no_state() {
    // The type-system proof (40-execution): the release path's counter
    // type occupies no memory and the executor stores no counter field
    // (counters are a call-site parameter, monomorphized away).
    assert_eq!(std::mem::size_of::<NoopCounters>(), 0);
}

/// "Batching engaged" (docs/architecture/60-validation.md): at batch
/// size 64 over hundreds of root tuples, the cover draws batches, not
/// per-tuple iterations — the counted execution proves the vectorized
/// path is live, not silently degenerate.
#[test]
fn the_counted_execution_shows_batching_engaged() {
    let dir = TempDir::new("introspect-batching");
    let schema = schema(2);
    let r0: Vec<(u64, u64)> = (0..300).map(|i| (i, i % 7)).collect();
    let r1: Vec<(u64, u64)> = (0..7).map(|i| (i, i)).collect();
    let views = views_of(&dir, &schema, &[r0, r1]);
    let normalized = normalized(vec![
        occurrence(0, 0, &[(0, 0), (1, 1)]),
        occurrence(1, 1, &[(0, 1), (1, 2)]),
    ]);
    let order = JoinOrder {
        order: vec![OccId(0), OccId(1)],
        estimates: vec![300, 300],
    };
    let mut fj = binary2fj(&normalized, &order);
    factor(&mut fj);
    let sink_vars = BTreeSet::from([VarId(0), VarId(1), VarId(2)]);
    let plan = validate(
        &fj,
        &normalized,
        &schema,
        order.estimates.clone(),
        &sink_vars,
    )
    .expect("valid plan");

    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = ProjectionSink::new(vec![
        plan.slot_of(VarId(0)),
        plan.slot_of(VarId(1)),
        plan.slot_of(VarId(2)),
    ]);
    let mut counters = CountingCounters::new(&plan);
    Executor::with_batch_size(&plan, 64)
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");

    let (batches, entries) = counters.batches(0);
    assert_eq!(entries, 300, "the root drains every tuple");
    assert_eq!(
        batches,
        300 / 64 + 1,
        "64-wide batches, not per-tuple draws"
    );
    assert!(counters.emits() > 0);
}

/// Anti-probe selectivity joins the counted execution (docs/architecture/
/// 40-execution.md, § anti-probe filters): per-node probed vs rejected
/// counts populate the stats and render in the report.
#[test]
fn anti_probe_selectivity_populates_the_counted_execution() {
    let dir = TempDir::new("introspect-anti-probe");
    let schema = schema(2);
    // R0 = postings (fresh, payload); R1 = tags (fresh, posting id):
    // postings 1, 2, 3 are tagged (2 and 3 multiply).
    let r0: Vec<(u64, u64)> = (0..10).map(|i| (i, 100 + i)).collect();
    let r1 = vec![(0u64, 1u64), (1, 2), (2, 2), (3, 3), (4, 3), (5, 3)];
    let views = views_of(&dir, &schema, &[r0, r1]);
    // Q(p, a) :- R0(p, a), ¬R1(_, p).
    let normalized = normalized(vec![
        occurrence(0, 0, &[(0, 0), (1, 1)]),
        negated(1, 1, &[(1, 0)]),
    ]);
    let order = JoinOrder {
        order: vec![OccId(0)],
        estimates: vec![10],
    };
    let mut fj = binary2fj(&normalized, &order);
    factor(&mut fj);
    let sink_vars = BTreeSet::from([VarId(0), VarId(1)]);
    let plan = validate(
        &fj,
        &normalized,
        &schema,
        order.estimates.clone(),
        &sink_vars,
    )
    .expect("valid plan");

    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(0)), plan.slot_of(VarId(1))]);
    let mut counters = CountingCounters::new(&plan);
    Executor::new(&plan)
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");

    assert_eq!(counters.emits(), 7, "three postings rejected");
    let rule = counters.into_rule_stats(&plan, &schema, Vec::new(), 0);
    assert_eq!(rule.nodes[0].anti_probe_probed, 10);
    assert_eq!(rule.nodes[0].anti_probe_rejected, 3);
    let report = IntrospectionReport {
        header: None,
        rules: vec![RulePlan::FreeJoin(&plan)],
        stats: ExecutionStats {
            introspection_version: crate::api::stats::INTROSPECTION_VERSION,
            emits: rule.emitted,
            rules: vec![rule],
            disjoint_rules: None,
            subsumed: Vec::new(),
            dead: Vec::new(),
        },
    };
    let text = format!("{report}");
    assert!(text.contains("anti-probes: 1 placed, probed=10 rejected=3"));
}
