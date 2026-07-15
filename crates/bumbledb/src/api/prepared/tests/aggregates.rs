//! PRD 18 criteria at the API boundary: `CountDistinct` and
//! Arg-restriction end to end (validate → plan → execute → result
//! buffer), the seen-set elision fixture on the stats counters, and the
//! interval find round-trip.

use super::*;
use crate::ir::AggOp;
use crate::schema::IntervalElement;

/// The shared fixture: account 3 holds ("a", 10), ("b", 10), ("a", 25);
/// account 7 holds ("c", 25). Fresh ids 1..=4.
fn posting_fixture(env: &Environment, schema: &Schema) {
    insert_postings(
        env,
        schema,
        &[
            (1, 3, "a", 10),
            (2, 3, "b", 10),
            (3, 3, "a", 25),
            (4, 7, "c", 25),
        ],
    );
}

/// Q(account, `CountDistinct`(amount), `CountDistinct`(memo)) :-
/// Posting(id, account, memo, amount) — ids bound, so every fact is a
/// distinct binding; the value sets do the collapsing, per group, for
/// integer words and intern-id (string) words alike.
#[test]
fn count_distinct_collapses_multiplicities_per_group_and_over_strings() {
    let dir = TempDir::new("prepared-count-distinct");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    posting_fixture(&env, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::CountDistinct,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::CountDistinct,
                over: Some(VarId(2)),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(3))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(2))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut answers: Vec<(u64, u64, u64)> = (0..out.len())
        .map(
            |answer| match (out.get(answer, 0), out.get(answer, 1), out.get(answer, 2)) {
                (AnswerValue::U64(a), AnswerValue::U64(n), AnswerValue::U64(m)) => (a, n, m),
                other => panic!("all-U64 answer: {other:?}"),
            },
        )
        .collect();
    answers.sort_unstable();
    // Account 3: 3 postings, 2 distinct amounts, 2 distinct memos;
    // account 7: 1 each. Amount 25 and memo "a" appearing in both
    // groups counts per group (scoping).
    assert_eq!(answers, vec![(3, 2, 2), (7, 1, 1)]);
}

/// The elision fixture (PRD 18): a fresh-keyed query proves distinct
/// bindings, so the plan elides the binding seen-set — the introspection
/// regime observable — while `CountDistinct` still collapses values. The
/// stats counters carry both halves: `emits` counts every binding
/// (nothing was deduped upstream), the result holds the collapsed
/// distinct count.
#[test]
fn elision_skips_binding_dedup_but_count_distinct_still_collapses() {
    let dir = TempDir::new("prepared-elision-count-distinct");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Account 3: amounts {10, 10, 25} across three fresh-distinct
    // postings.
    insert_postings(
        &env,
        &schema,
        &[(1, 3, "a", 10), (2, 3, "b", 10), (3, 3, "c", 25)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::CountDistinct,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))), // fresh id bound: key covered
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        prepared.distinct_bindings(),
        "the fresh key is covered: the binding seen-set is elided"
    );
    let (out, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert!(stats.rules[0].distinct_bindings);
    assert_eq!(
        stats.emits, 3,
        "every binding reaches the sink — elision dropped none"
    );
    assert_eq!(out.len(), 1);
    match (out.get(0, 0), out.get(0, 1)) {
        (AnswerValue::U64(3), AnswerValue::U64(2)) => {}
        other => panic!("CountDistinct still collapsed 3 bindings to 2 values: {other:?}"),
    }
    let (_, report) = prepared
        .introspect(&txn, &cache, &[])
        .expect("introspection");
    assert!(report.contains("distinct_bindings: proven"), "{report}");

    // Same answer, deliberately unkeyed: dropping the fresh id makes the
    // two amount-10 facts one binding, so the binding seen-set is required.
    let unkeyed = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::CountDistinct,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &unkeyed).expect("prepare");
    assert!(!prepared.distinct_bindings());
    let (out, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert!(!stats.rules[0].distinct_bindings);
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(3));
    assert_eq!(out.get(0, 1), AnswerValue::U64(2));
    let (_, report) = prepared
        .introspect(&txn, &cache, &[])
        .expect("introspection");
    assert!(report.contains("distinct_bindings: unproven"), "{report}");
}

/// `ArgMax` over the fresh id — latest posting per account — plus the
/// `ArgMin` mirror and the global (no group key) form.
#[test]
fn arg_max_picks_the_latest_posting_per_account() {
    let dir = TempDir::new("prepared-argmax-latest");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    posting_fixture(&env, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let atoms = vec![Atom {
        source: crate::ir::AtomSource::Edb(POSTING),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(2))),
            (FieldId(1), Term::Var(VarId(0))),
            (FieldId(3), Term::Var(VarId(1))),
        ],
    }];
    let arg = |max: bool| {
        Query::single(Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: if max {
                        AggOp::ArgMax { key: VarId(2) }
                    } else {
                        AggOp::ArgMin { key: VarId(2) }
                    },
                    over: Some(VarId(1)),
                },
            ],
            atoms: atoms.clone(),
            negated: vec![],
            conditions: vec![],
        })
    };

    let amounts = |out: &Answers| {
        let mut answers: Vec<(u64, i64)> = (0..out.len())
            .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
                (AnswerValue::U64(a), AnswerValue::I64(v)) => (a, v),
                other => panic!("(u64, i64) answer: {other:?}"),
            })
            .collect();
        answers.sort_unstable();
        answers
    };

    // Latest per account: id 3 (amount 25) and id 4 (amount 25).
    let mut prepared = prepare(&txn, &cache, &schema, &arg(true)).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        amounts(&out),
        vec![(3, 25), (7, 25)],
        "single winner per group"
    );

    // Earliest per account: id 1 (amount 10) and id 4 (amount 25).
    let mut prepared = prepare(&txn, &cache, &schema, &arg(false)).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(amounts(&out), vec![(3, 10), (7, 25)], "ArgMin mirrors");

    // Global group: the latest posting overall.
    let global = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::ArgMax { key: VarId(2) },
            over: Some(VarId(1)),
        }],
        atoms,
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &global).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(
        out.get(0, 0),
        AnswerValue::I64(25),
        "id 4 is globally latest"
    );
}

/// Arg ties are set-honest end to end: equal keys with different
/// carries yield both rows; bindings projecting EQUAL rows collapse to
/// one; the key variable itself may be carried.
#[test]
fn arg_ties_are_set_honest() {
    let dir = TempDir::new("prepared-arg-ties");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Account 3 ties at amount 25 with DIFFERENT memos (ids 2, 3) and
    // account 7 ties at amount 9 with the SAME memo (ids 4, 5).
    insert_postings(
        &env,
        &schema,
        &[
            (1, 3, "old", 10),
            (2, 3, "x", 25),
            (3, 3, "y", 25),
            (4, 7, "dup", 9),
            (5, 7, "dup", 9),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(account, ArgMax_amount(memo)) — carries the memo.
    let carry_memo = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(1) },
                over: Some(VarId(2)),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(3))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(2))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &carry_memo).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut answers: Vec<(u64, String)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(a), AnswerValue::String(m)) => (a, m.to_owned()),
            other => panic!("(u64, string) answer: {other:?}"),
        })
        .collect();
    answers.sort();
    assert_eq!(
        answers,
        vec![
            (3, "x".to_owned()),
            (3, "y".to_owned()),
            (7, "dup".to_owned()),
        ],
        "different carries keep both attaining answers; equal answers collapse to one"
    );

    // Key-also-projected: the carry IS the key variable — ties project
    // equal answers and collapse.
    let carry_key = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(1) },
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &carry_key).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut answers: Vec<(u64, i64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(a), AnswerValue::I64(v)) => (a, v),
            other => panic!("(u64, i64) answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(
        answers,
        vec![(3, 25), (7, 9)],
        "key-projected ties collapse"
    );
}

/// Payroll(id fresh u64, emp u64, during Interval<I64>).
fn interval_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Payroll".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "emp".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                        width: None,
                    },
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const PAYROLL: RelationId = RelationId(0);

fn insert_payroll(env: &Environment, schema: &Schema, rows: &[(u64, u64, (i64, i64))]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, emp, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*emp),
                ValueRef::IntervalI64(
                    crate::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
            ],
            schema.relation(PAYROLL).layout(),
            &mut bytes,
        );
        delta.insert(&view, PAYROLL, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// The interval find round-trip: a projected interval variable
/// materializes as `Value::IntervalI64` rows equal to the stored
/// facts', and the predicate's signature reports the interval type.
#[test]
fn interval_find_round_trips_through_answers() {
    let dir = TempDir::new("prepared-interval-roundtrip");
    let schema = interval_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let stored = [
        (1u64, 10u64, (5i64, 9i64)),
        (2, 10, (-3, 4)),
        (3, 11, (i64::MIN, i64::MAX)),
    ];
    insert_payroll(&env, &schema, &stored);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(emp, during) :- Payroll(emp, during).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let types: Vec<ValueType> = prepared
        .predicate()
        .columns
        .iter()
        .map(|column| column.ty.clone())
        .collect();
    assert_eq!(
        types,
        vec![
            ValueType::U64,
            ValueType::Interval {
                element: IntervalElement::I64,
                width: None
            },
        ],
        "the predicate reports the interval type"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut answers: Vec<(u64, i64, i64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(emp), AnswerValue::IntervalI64(iv)) => (emp, iv.start(), iv.end()),
            other => panic!("(u64, interval) answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();
    let mut expected: Vec<(u64, i64, i64)> = stored
        .iter()
        .map(|(_, emp, (start, end))| (*emp, *start, *end))
        .collect();
    expected.sort_unstable();
    assert_eq!(answers, expected, "stored bounds round-trip exactly");
}

/// `CountDistinct` over an interval variable end to end: value identity
/// (both words), never overlap.
#[test]
fn count_distinct_over_intervals_uses_value_identity() {
    let dir = TempDir::new("prepared-interval-count-distinct");
    let schema = interval_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Emp 10: [5,9), [5,9), [6,9) — two distinct values (the third
    // overlaps the first but is not equal to it).
    insert_payroll(
        &env,
        &schema,
        &[
            (1, 10, (5, 9)),
            (2, 10, (5, 9)),
            (3, 10, (6, 9)),
            (4, 11, (5, 9)),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::CountDistinct,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut answers: Vec<(u64, u64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(emp), AnswerValue::U64(n)) => (emp, n),
            other => panic!("(u64, u64) answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(answers, vec![(10, 2), (11, 1)]);
}
