//! PRD 17: param sets end to end — the `IN` family over selection
//! levels, set-carrying membership, and negated set bindings
//! (docs/architecture/20-query-ir.md § param sets; 40-execution
//! § selection levels).

use super::*;
use crate::api::prepared::ParamArg;
use crate::ir::ParamId;

/// Q(id, amount) :- Posting(id, account = ?set0, amount).
fn by_account_set_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(id, amount) :- Posting(id, account = ?0, amount) — the scalar twin.
fn by_account_scalar_query() -> Query {
    let mut query = by_account_set_query();
    query.rules[0].atoms[0].bindings[1] = (FieldId(1), Term::Param(ParamId(0)));
    query
}

fn id_amount_answers(buffer: &Answers) -> Vec<(u64, i64)> {
    let mut answers: Vec<(u64, i64)> = (0..buffer.len())
        .map(|answer| {
            let AnswerValue::U64(id) = buffer.get(answer, 0) else {
                panic!("column 0 is a u64 id");
            };
            let AnswerValue::I64(amount) = buffer.get(answer, 1) else {
                panic!("column 1 is an i64 amount");
            };
            (id, amount)
        })
        .collect();
    answers.sort_unstable();
    answers
}

/// The `IN` family criterion: over set sizes {0, 1, 2, 200}, the
/// set-bound execution equals the union of per-element scalar-param
/// executions — asserted against that construction — and duplicate
/// elements collapse (sets are sets).
#[test]
fn in_family_equals_the_union_of_per_element_executions() {
    let dir = TempDir::new("prepared-in-family");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // 600 rows over 250 accounts: several accounts carry multiple rows.
    let rows: Vec<(u64, u64, String, i64)> = (0..600u64)
        .map(|i| {
            let amount = i64::try_from(i).expect("small") * 3 - 100;
            (i, i % 250, format!("m{}", i % 5), amount)
        })
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    insert_postings(&env, &schema, &borrowed);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut set_query = prepare(&txn, &cache, &schema, &by_account_set_query()).expect("prepare");
    let mut scalar_query =
        prepare(&txn, &cache, &schema, &by_account_scalar_query()).expect("prepare");

    for size in [0usize, 1, 2, 200] {
        // Accounts stride 7 so misses (>= 250) land in the larger sets:
        // out-of-domain elements must contribute nothing.
        let elements: Vec<Value> = (0..size)
            .map(|k| Value::U64(u64::try_from(k).expect("small") * 7))
            .collect();
        let got = set_query
            .execute_collect_args(&txn, &cache, &[ParamArg::Set(&elements)])
            .expect("set execution");
        // The defining construction: the union of per-element scalar
        // executions (results are sets, so union = sorted dedup).
        let mut union: Vec<(u64, i64)> = Vec::new();
        for element in &elements {
            let Value::U64(account) = element else {
                unreachable!("the elements are U64 accounts")
            };
            let per = scalar_query
                .execute_collect(&txn, &cache, &[BindValue::U64(*account)])
                .expect("scalar execution");
            union.extend(id_amount_answers(&per));
        }
        union.sort_unstable();
        union.dedup();
        assert_eq!(id_amount_answers(&got), union, "size {size}");
        if size == 0 {
            assert!(got.is_empty(), "the empty set matches nothing");
        }
    }

    // Duplicates in the bound slice collapse: sets are sets.
    let dup = [Value::U64(7), Value::U64(7), Value::U64(7)];
    let once = [Value::U64(7)];
    let got_dup = set_query
        .execute_collect_args(&txn, &cache, &[ParamArg::Set(&dup)])
        .expect("execute");
    let got_once = set_query
        .execute_collect_args(&txn, &cache, &[ParamArg::Set(&once)])
        .expect("execute");
    assert_eq!(id_amount_answers(&got_dup), id_amount_answers(&got_once));

    // A scalar value where the set is expected is a precise bind-time
    // error (a ParamId is scalar or set, never both).
    let err = set_query
        .execute_collect(&txn, &cache, &[BindValue::U64(7)])
        .unwrap_err();
    assert!(matches!(err, Error::ParamSetExpected { param } if param.0 == 0));
}

/// Out-of-vocabulary string elements resolve to per-element miss
/// sentinels and contribute nothing; an all-miss set is the empty set.
#[test]
fn out_of_vocabulary_string_elements_contribute_nothing() {
    let dir = TempDir::new("prepared-in-strings");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[
            (1, 7, "rent", -1200),
            (2, 7, "salary", 5000),
            (3, 8, "rent", -900),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(amount) :- Posting(memo = ?set0, amount).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(2), Term::ParamSet(ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");

    let string = |text: &str| Value::String(Box::from(text.as_bytes()));
    let with_ghost = [string("rent"), string("ghost")];
    let rent_only = [string("rent")];
    let all_ghost = [string("ghost"), string("phantom")];

    let got = prepared
        .execute_collect_args(&txn, &cache, &[ParamArg::Set(&with_ghost)])
        .expect("execute");
    let control = prepared
        .execute_collect_args(&txn, &cache, &[ParamArg::Set(&rent_only)])
        .expect("execute");
    assert_eq!(amounts_of(&got), amounts_of(&control));
    assert_eq!(amounts_of(&got), vec![-1200, -900]);

    let empty = prepared
        .execute_collect_args(&txn, &cache, &[ParamArg::Set(&all_ghost)])
        .expect("execute");
    assert!(empty.is_empty(), "an all-miss set matches nothing");
}

/// Payroll(emp u64, during Interval<u64>) + Event(emp u64, at u64).
fn interval_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Payroll".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: crate::schema::IntervalElement::U64,
                        },
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Event".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "at".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const PAYROLL: RelationId = RelationId(0);
const EVENT: RelationId = RelationId(1);

fn insert_interval_fixture(env: &Environment, schema: &Schema) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (emp, start, end) in [(1u64, 10u64, 20u64), (2, 30, 40)] {
        let mut bytes = Vec::new();
        crate::encoding::encode_fact(
            &[
                crate::encoding::ValueRef::U64(emp),
                crate::encoding::ValueRef::IntervalU64(
                    crate::Interval::<u64>::new(start, end).expect("nonempty interval"),
                ),
            ],
            schema.relation(PAYROLL).layout(),
            &mut bytes,
        );
        delta.insert(&view, PAYROLL, &bytes).expect("insert");
    }
    for (emp, at) in [(1u64, 9u64), (1, 10), (1, 19), (1, 20), (2, 35), (3, 15)] {
        let mut bytes = Vec::new();
        crate::encoding::encode_fact(
            &[
                crate::encoding::ValueRef::U64(emp),
                crate::encoding::ValueRef::U64(at),
            ],
            schema.relation(EVENT).layout(),
            &mut bytes,
        );
        delta.insert(&view, EVENT, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

fn u64_pairs(buffer: &Answers) -> Vec<(u64, u64)> {
    let mut answers: Vec<(u64, u64)> = (0..buffer.len())
        .map(|answer| {
            let AnswerValue::U64(a) = buffer.get(answer, 0) else {
                panic!("column 0 is u64");
            };
            let AnswerValue::U64(b) = buffer.get(answer, 1) else {
                panic!("column 1 is u64");
            };
            (a, b)
        })
        .collect();
    answers.sort_unstable();
    answers
}

/// The membership point-var join, whole pipeline: IR membership binding
/// → var-sourced `PointIn` → placed membership probe → the
/// point-membership scan. Both boundaries asserted through the public
/// result: `at == start` survives, `at == end` does not.
#[test]
fn membership_point_var_join_end_to_end() {
    let dir = TempDir::new("prepared-membership");
    let schema = interval_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_interval_fixture(&env, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(emp, at) :- Payroll(emp, during ∋ at), Event(emp, at).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(PAYROLL),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))), // membership: at ∈ during
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(EVENT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let got = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        u64_pairs(&got),
        vec![(1, 10), (1, 19), (2, 35)],
        "start inclusive, end exclusive, per employee"
    );
}

/// A set-bound membership (`during ∋ ?set0`): any element in the
/// interval satisfies the binding — `AnyPointIn`, the kernel-backed
/// two-column composition, boundaries included.
#[test]
fn set_membership_matches_any_element() {
    let dir = TempDir::new("prepared-any-point");
    let schema = interval_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_interval_fixture(&env, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(emp) :- Payroll(emp, during ∋ ?set0).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let emps = |buffer: &Answers| {
        let mut out: Vec<u64> = (0..buffer.len())
            .map(|answer| {
                let AnswerValue::U64(emp) = buffer.get(answer, 0) else {
                    panic!("column 0 is u64");
                };
                emp
            })
            .collect();
        out.sort_unstable();
        out
    };
    let run = |prepared: &mut PreparedQuery<'_, ()>, points: &[u64]| {
        let values: Vec<Value> = points.iter().map(|p| Value::U64(*p)).collect();
        let got = prepared
            .execute_collect_args(&txn, &cache, &[ParamArg::Set(&values)])
            .expect("execute");
        emps(&got)
    };
    assert_eq!(run(&mut prepared, &[10]), vec![1], "start is in");
    assert_eq!(run(&mut prepared, &[20]), Vec::<u64>::new(), "end is out");
    assert_eq!(run(&mut prepared, &[19, 39]), vec![1, 2], "any element");
    assert_eq!(run(&mut prepared, &[25]), Vec::<u64>::new(), "gap");
    assert_eq!(run(&mut prepared, &[]), Vec::<u64>::new(), "empty set");
}

/// Commits one ray fact `Payroll(1, [10, ∞))` and returns the open
/// environment — the point-domain-law fixture.
fn ray_fixture(dir: &TempDir, schema: &Schema) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    let mut bytes = Vec::new();
    crate::encoding::encode_fact(
        &[
            crate::encoding::ValueRef::U64(1),
            crate::encoding::ValueRef::IntervalU64(
                crate::Interval::<u64>::new(10, u64::MAX).expect("nonempty interval"),
            ),
        ],
        schema.relation(PAYROLL).layout(),
        &mut bytes,
    );
    delta.insert(&view, PAYROLL, &bytes).expect("insert");
    drop(view);
    commit(delta, &env).expect("commit");
    env
}

/// Q(emp) :- Payroll(emp, during ∋ point-literal).
fn membership_literal_query(point: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Literal(Value::U64(point))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// The point-domain law, both halves (`docs/architecture/10-data-model.md`):
/// `MAX−1` is the last point — membership in the ray `[10, ∞)` is true —
/// and a point literal of `MAX` is rejected at prepare with the typed
/// error, never a silently-unmatchable query.
#[test]
fn membership_of_the_last_point_in_a_ray_is_true_and_the_ceiling_rejects() {
    let dir = TempDir::new("prepared-ray-membership");
    let schema = interval_schema();
    let env = ray_fixture(&dir, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(
        &txn,
        &cache,
        &schema,
        &membership_literal_query(u64::MAX - 1),
    )
    .expect("prepare");
    let got = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(got.len(), 1, "MAX-1 is a point of [10, \u{221e})");

    let Err(err) = prepare(&txn, &cache, &schema, &membership_literal_query(u64::MAX)) else {
        panic!("the ceiling is not a point");
    };
    assert!(
        matches!(
            err,
            Error::Validation(crate::error::ValidationError::PointLiteralAtCeiling {
                atom: 0,
                field: FieldId(1),
            })
        ),
        "got {err:?}"
    );
}

/// The bind-time half of the point-domain law: a point-position param
/// (element-typed at an interval position) bound to the domain ceiling
/// is the typed bind error, for scalars and per set element alike.
#[test]
fn point_param_at_the_ceiling_is_a_bind_error() {
    let dir = TempDir::new("prepared-ray-point-param");
    let schema = interval_schema();
    let env = ray_fixture(&dir, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(emp) :- Payroll(emp, during ∋ ?0), Event(emp, at = ?0): the
    // scalar-field anchor types ?0 at the element, so the Payroll
    // binding is membership and ?0 is a point param.
    let scalar_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(PAYROLL),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Param(ParamId(0))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(EVENT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Param(ParamId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &scalar_query).expect("prepare");
    let err = prepared
        .execute_collect_args(&txn, &cache, &[ParamArg::Scalar(BindValue::U64(u64::MAX))])
        .expect_err("the ceiling is not a point");
    assert!(
        matches!(err, Error::PointParamAtCeiling { param: ParamId(0) }),
        "got {err:?}"
    );

    // Q(emp) :- Payroll(emp, during ∋ ?set0): a point set — the same
    // rejection per element, and the last point still matches.
    let set_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &set_query).expect("prepare");
    let ceiling = [Value::U64(u64::MAX)];
    let err = prepared
        .execute_collect_args(&txn, &cache, &[ParamArg::Set(&ceiling)])
        .expect_err("the ceiling is not a point");
    assert!(
        matches!(err, Error::PointParamAtCeiling { param: ParamId(0) }),
        "got {err:?}"
    );
    let last_point = [Value::U64(u64::MAX - 1)];
    let got = prepared
        .execute_collect_args(&txn, &cache, &[ParamArg::Set(&last_point)])
        .expect("execute");
    assert_eq!(got.len(), 1, "MAX-1 is a point of [10, \u{221e})");
}

/// Posting(account u64, amount i64) + Block(account u64, kind u64).
fn block_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Block".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "kind".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// Anti-probes with set-carrying negated bindings: a binding is rejected
/// iff the negated occurrence matches under **any** element — the
/// existential reading of `docs/architecture/20-query-ir.md` § param
/// sets ("the term denotes *any element* — a binding position matches
/// iff the field value is **in** the set"), applied inside the negation.
/// The empty set matches under no element, so nothing is rejected.
#[test]
fn negated_set_bindings_reject_under_any_element() {
    let dir = TempDir::new("prepared-anti-set");
    let schema = block_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for (account, amount) in [(7u64, 70i64), (8, 80), (9, 90)] {
        let mut bytes = Vec::new();
        crate::encoding::encode_fact(
            &[
                crate::encoding::ValueRef::U64(account),
                crate::encoding::ValueRef::I64(amount),
            ],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    for (account, kind) in [(7u64, 1u64), (8, 5)] {
        let mut bytes = Vec::new();
        crate::encoding::encode_fact(
            &[
                crate::encoding::ValueRef::U64(account),
                crate::encoding::ValueRef::U64(kind),
            ],
            schema.relation(RelationId(1)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(1), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(amount) :- Posting(account, amount), not Block(account, kind = ?set0).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(1)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
            ],
        }],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let run = |prepared: &mut PreparedQuery<'_, ()>, kinds: &[u64]| {
        let values: Vec<Value> = kinds.iter().map(|k| Value::U64(*k)).collect();
        let got = prepared
            .execute_collect_args(&txn, &cache, &[ParamArg::Set(&values)])
            .expect("execute");
        amounts_of(&got)
    };
    // (7, kind 1) matches {1, 2} through element 1: account 7 rejected.
    assert_eq!(run(&mut prepared, &[1, 2]), vec![80, 90]);
    // (8, kind 5) matches {5}: account 8 rejected.
    assert_eq!(run(&mut prepared, &[5]), vec![70, 90]);
    // Both blocked accounts match under some element.
    assert_eq!(run(&mut prepared, &[1, 5]), vec![90]);
    // No element matches anything: nothing rejected.
    assert_eq!(run(&mut prepared, &[3, 4]), vec![70, 80, 90]);
    // The empty set matches under NO element — never a rejection (and
    // never the positive-side short-circuit, which would be unsound
    // under negation).
    assert_eq!(run(&mut prepared, &[]), vec![70, 80, 90]);
}
