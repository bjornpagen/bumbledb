use super::*;
use crate::encoding::{encode_fact, ValueRef};
use crate::exec::colt::Colt;
use crate::exec::run::{Bindings, Executor, NoopCounters};
use crate::exec::sink::{AggregateSink, FindSpec, ProjectionSink};
use crate::image::view::apply;
use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence, PlacedComparison};
use crate::ir::{AggOp, CmpOp, ParamId};
use crate::plan::fj::{binary2fj, factor, validate};
use crate::plan::planner::JoinOrder;
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::dict;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use std::collections::BTreeSet;

/// Account(id serial u64, holder u64, name string).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Account".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Serial,
                },
                FieldDescriptor {
                    name: "holder".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "name".into(),
                    value_type: ValueType::String,
                    generation: Generation::None,
                },
            ],
            constraints: vec![],
        }],
    }
    .validate()
    .expect("valid fixture")
}

const ACCOUNT: RelationId = RelationId(0);

fn occurrence(vars: &[(u16, u16)], filters: Vec<FilterPredicate>) -> Occurrence {
    Occurrence {
        occ_id: OccId(0),
        relation: ACCOUNT,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
        filters,
    }
}

fn eq_filter(field: u16, value: Const) -> FilterPredicate {
    FilterPredicate::Compare {
        field: FieldId(field),
        op: CmpOp::Eq,
        value,
    }
}

fn single(occurrence: Occurrence) -> NormalizedQuery {
    NormalizedQuery {
        occurrences: vec![occurrence],
        residuals: vec![],
    }
}

/// Commits accounts (id, holder, name) and returns the environment.
fn populated(dir: &TempDir, schema: &Schema, rows: &[(u64, u64, &str)]) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, holder, name) in rows {
        let name_id = delta.intern_str(&view, name).expect("intern");
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*holder),
                ValueRef::String(name_id),
            ],
            schema.relation(ACCOUNT).layout(),
            &mut bytes,
        );
        delta.insert(&view, ACCOUNT, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    env
}

// ---------- classification ----------

#[test]
fn fully_unique_bound_single_atom_classifies_as_guard_probe() {
    let schema = schema();
    let normalized = single(occurrence(
        &[(1, 0), (2, 1)],
        vec![eq_filter(0, Const::Word(5))], // id = 5, the serial auto-unique
    ));
    let plan = classify(&normalized, &schema).expect("guard probe");
    assert_eq!(plan.constraint, Some(ConstraintId(0)));
    assert_eq!(plan.key, vec![(FieldId(0), Const::Word(5))]);
    assert!(plan.remaining_filters.is_empty());
}

#[test]
fn a_second_atom_or_a_residual_stays_free_join() {
    let schema = schema();
    let occ = occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]);
    let two_atoms = NormalizedQuery {
        occurrences: vec![occ.clone(), occ.clone()],
        residuals: vec![],
    };
    assert!(classify(&two_atoms, &schema).is_none());

    let with_residual = NormalizedQuery {
        occurrences: vec![occurrence(
            &[(1, 0), (2, 1)],
            vec![eq_filter(0, Const::Word(5))],
        )],
        residuals: vec![PlacedComparison {
            op: CmpOp::Lt,
            lhs: VarId(0),
            rhs: VarId(1),
        }],
    };
    assert!(classify(&with_residual, &schema).is_none());
}

#[test]
fn a_partially_bound_unique_stays_free_join() {
    let schema = schema();
    // Only a non-key field is constant: no unique coverage, not full.
    let normalized = single(occurrence(
        &[(0, 0), (2, 1)],
        vec![eq_filter(1, Const::Word(9))],
    ));
    assert!(classify(&normalized, &schema).is_none());
}

#[test]
fn extra_filters_survive_as_remaining() {
    let schema = schema();
    let normalized = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(5)),
            eq_filter(1, Const::Word(7)), // outside the key
        ],
    ));
    let plan = classify(&normalized, &schema).expect("guard probe");
    assert_eq!(plan.remaining_filters, vec![eq_filter(1, Const::Word(7))]);
}

// ---------- execution ----------

fn run_guard(
    plan: &GuardPlan,
    env: &Environment,
    schema: &Schema,
    params: &[Const],
) -> Vec<Vec<u64>> {
    let txn = env.read_txn().expect("txn");
    let mut bindings = Bindings::new(plan.vars.len());
    let mut sink = ProjectionSink::new((0..plan.vars.len()).collect());
    let mut key = Vec::new();
    execute_guard(
        plan,
        &txn,
        schema,
        params,
        &mut key,
        &mut bindings,
        &mut sink,
    )
    .expect("execute");
    sink.rows().map(<[u64]>::to_vec).collect()
}

#[test]
fn hit_miss_and_filter_rejection() {
    let dir = TempDir::new("guard-hit-miss");
    let schema = schema();
    let env = populated(&dir, &schema, &[(5, 7, "alice"), (6, 8, "bob")]);
    let normalized = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]));
    let plan = classify(&normalized, &schema).expect("guard probe");
    assert_eq!(run_guard(&plan, &env, &schema, &[]), vec![vec![7]]);

    // Miss: no such id.
    let missing = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(99))]));
    let plan = classify(&missing, &schema).expect("guard probe");
    assert!(run_guard(&plan, &env, &schema, &[]).is_empty());

    // Hit, but a remaining filter rejects the fetched fact.
    let rejected = single(occurrence(
        &[(1, 0)],
        vec![
            eq_filter(0, Const::Word(5)),
            eq_filter(1, Const::Word(999)), // holder is 7, not 999
        ],
    ));
    let plan = classify(&rejected, &schema).expect("guard probe");
    assert!(run_guard(&plan, &env, &schema, &[]).is_empty());
}

#[test]
fn param_driven_keys_resolve_at_bind_time() {
    let dir = TempDir::new("guard-param");
    let schema = schema();
    let env = populated(&dir, &schema, &[(5, 7, "alice")]);
    let normalized = single(occurrence(
        &[(1, 0)],
        vec![eq_filter(0, Const::Param(ParamId(0)))],
    ));
    let plan = classify(&normalized, &schema).expect("guard probe");
    assert_eq!(
        run_guard(&plan, &env, &schema, &[Const::Word(5)]),
        vec![vec![7]]
    );
    assert!(run_guard(&plan, &env, &schema, &[Const::Word(6)]).is_empty());
}

#[test]
fn pending_intern_miss_is_empty_and_never_interns() {
    let dir = TempDir::new("guard-intern-miss");
    let schema = schema();
    let env = populated(&dir, &schema, &[(5, 7, "alice")]);
    // Full-fact-ish probe via the name field being part of no unique:
    // instead, probe by id but filter on a never-interned name.
    let normalized = single(occurrence(
        &[(1, 0)],
        vec![
            eq_filter(0, Const::Word(5)),
            eq_filter(
                2,
                Const::PendingIntern {
                    tag: 0,
                    bytes: Box::from(&b"ghost"[..]),
                },
            ),
        ],
    ));
    let plan = classify(&normalized, &schema).expect("guard probe");
    assert!(run_guard(&plan, &env, &schema, &[]).is_empty());
    // The read path never interned the ghost string.
    let txn = env.read_txn().expect("txn");
    assert_eq!(dict::lookup_str(&txn, "ghost").expect("lookup"), None);
}

#[test]
fn guard_and_free_join_paths_agree_by_construction() {
    let dir = TempDir::new("guard-equivalence");
    let schema = schema();
    let env = populated(&dir, &schema, &[(5, 7, "alice"), (6, 8, "bob")]);
    let normalized = single(occurrence(
        &[(1, 0), (2, 1)],
        vec![eq_filter(0, Const::Word(6))],
    ));

    // Guard path.
    let guard = classify(&normalized, &schema).expect("guard probe");
    let mut guard_rows = run_guard(&guard, &env, &schema, &[]);
    guard_rows.sort_unstable();

    // Free Join path over the same normalized query.
    let order = JoinOrder {
        order: vec![OccId(0)],
        estimates: vec![0],
    };
    let mut fj = binary2fj(&normalized, &order);
    factor(&mut fj);
    let plan =
        validate(&fj, &normalized, &schema, vec![0], &BTreeSet::new()).expect("valid plan");
    let txn = env.read_txn().expect("txn");
    let image = crate::image::build(&txn, &schema, ACCOUNT).expect("build");
    let view = apply(&image, &normalized.occurrences[0].filters, &[], Vec::new());
    let columns: Vec<Vec<usize>> = plan.occurrences()[0]
        .trie_schema
        .iter()
        .map(|level| {
            level
                .iter()
                .map(|var| {
                    let (field, _) = plan.occurrences()[0]
                        .vars
                        .iter()
                        .find(|(_, v)| v == var)
                        .expect("plan vars");
                    usize::from(field.0)
                })
                .collect()
        })
        .collect();
    let mut colts = vec![Colt::new(view, &[], columns)];
    let mut bindings = Bindings::new(plan.slots().len());
    let mut sink = ProjectionSink::new(
        [VarId(0), VarId(1)]
            .iter()
            .map(|v| plan.slot_of(*v))
            .collect(),
    );
    Executor::new(&plan).execute(
        &plan,
        &mut colts,
        &mut bindings,
        &mut sink,
        &mut NoopCounters,
    );
    let mut fj_rows: Vec<Vec<u64>> = sink.rows().map(<[u64]>::to_vec).collect();
    fj_rows.sort_unstable();

    assert_eq!(guard_rows, fj_rows);
    assert_eq!(guard_rows.len(), 1);
}

#[test]
fn aggregate_over_a_point_lookup_folds_one_binding() {
    let dir = TempDir::new("guard-aggregate");
    let schema = schema();
    let env = populated(&dir, &schema, &[(5, 7, "alice")]);
    let normalized = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]));
    let plan = classify(&normalized, &schema).expect("guard probe");
    let txn = env.read_txn().expect("txn");
    let mut bindings = Bindings::new(1);
    let mut sink = AggregateSink::new(
        vec![FindSpec::Agg {
            op: AggOp::Count,
            over_slot: None,
            signed: false,
        }],
        1,
        true,
    );
    let mut key = Vec::new();
    execute_guard(
        &plan,
        &txn,
        &schema,
        &[],
        &mut key,
        &mut bindings,
        &mut sink,
    )
    .expect("execute");
    assert_eq!(sink.into_rows().expect("rows"), vec![vec![1]]);
}

// No image build can occur on the guard path: `execute_guard` takes no
// image, view, or cache argument — the property holds by API shape, on
// a cold database in every test above.
