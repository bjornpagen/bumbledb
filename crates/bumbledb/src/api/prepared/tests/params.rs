use super::*;

#[test]
fn prepare_once_execute_many_with_varying_params() {
    let dir = TempDir::new("prepared-many");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[
            (1, 7, "rent", -1200),
            (2, 7, "salary", 5000),
            (3, 8, "coffee", -4),
        ],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let mut out = ResultBuffer::new();

    prepared
        .execute(
            &txn,
            &cache,
            &[BindValue::U64(7), BindValue::I64(0)],
            &mut out,
        )
        .expect("execute");
    assert_eq!(rows_of(&out), vec![("salary".to_owned(), 5000)]);

    prepared
        .execute(
            &txn,
            &cache,
            &[BindValue::U64(7), BindValue::I64(i64::MIN)],
            &mut out,
        )
        .expect("execute");
    assert_eq!(
        rows_of(&out),
        vec![("rent".to_owned(), -1200), ("salary".to_owned(), 5000)]
    );

    prepared
        .execute(
            &txn,
            &cache,
            &[BindValue::U64(8), BindValue::I64(i64::MIN)],
            &mut out,
        )
        .expect("execute");
    assert_eq!(rows_of(&out), vec![("coffee".to_owned(), -4)]);
}

#[test]
fn bind_time_checks_reject_bad_params() {
    let dir = TempDir::new("prepared-bind-errors");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let mut out = ResultBuffer::new();

    let err = prepared
        .execute(&txn, &cache, &[BindValue::U64(7)], &mut out)
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::ParamCountMismatch {
                expected: 2,
                supplied: 1
            }
        ),
        "{err:?}"
    );

    let err = prepared
        .execute(
            &txn,
            &cache,
            &[BindValue::I64(7), BindValue::I64(0)],
            &mut out,
        )
        .unwrap_err();
    assert!(
        matches!(err, Error::ParamTypeMismatch { param, .. } if param.0 == 0),
        "{err:?}"
    );
}

#[test]
fn string_params_resolve_per_execution() {
    let dir = TempDir::new("prepared-string-param");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "rent", -1200)]);
    let cache = ImageCache::new();

    // Q(amount) :- Posting(memo = ?0, amount).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let mut out = ResultBuffer::new();

    // Never-interned value: empty, not an error.
    prepared
        .execute(&txn, &cache, &[BindValue::Str("groceries")], &mut out)
        .expect("execute");
    assert!(out.is_empty());
    drop(txn);

    // A later commit interns it; the SAME prepared query now finds rows
    // (per-execution resolution — no stale-resolution trap).
    insert_postings(&env, &schema, &[(2, 9, "groceries", -55)]);
    let txn = env.read_txn().expect("txn");
    prepared
        .execute(&txn, &cache, &[BindValue::Str("groceries")], &mut out)
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::I64(-55));
}

/// Mandate(account u64, active interval<u64>) — the mask-param fixture.
fn mandate_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Mandate".into(),
            fields: vec![
                FieldDescriptor {
                    name: "account".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "active".into(),
                    value_type: ValueType::Interval {
                        element: crate::schema::IntervalElement::U64,
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

fn insert_mandates(env: &Environment, schema: &Schema, rows: &[(u64, u64, u64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (account, start, end) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(*account), ValueRef::IntervalU64(*start, *end)],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

fn accounts_of(buffer: &ResultBuffer) -> Vec<u64> {
    let mut accounts: Vec<u64> = (0..buffer.len())
        .map(|row| match buffer.get(row, 0) {
            ResultValue::U64(v) => v,
            other => panic!("expected u64, got {other:?}"),
        })
        .collect();
    accounts.sort_unstable();
    accounts
}

/// The mask is a bind-time argument: one prepared `Allen(v, [10,20), ?0)`
/// query answers different temporal questions per execution — the
/// singleton, composite, and rebind cases, plus the ∅/full/shape bind
/// rejections (`docs/architecture/20-query-ir.md`, § the Allen operator).
#[test]
fn a_mask_param_rebinds_the_temporal_relation_per_execution() {
    use crate::allen::AllenMask;
    use crate::ir::MaskTerm;

    let dir = TempDir::new("prepared-mask-param");
    let schema = mandate_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Against the constant [10, 20): before / covered-by (during) /
    // covers (contains) / after.
    insert_mandates(
        &env,
        &schema,
        &[(1, 1, 5), (2, 12, 18), (3, 5, 25), (4, 25, 30)],
    );
    let cache = ImageCache::new();

    // Q(a) :- Mandate(account = a, active = v), Allen(v, [10,20), ?0).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: RelationId(0),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Param(crate::ir::ParamId(0)),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::IntervalU64(10, 20)),
        })],
    });
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let mut out = ResultBuffer::new();

    let mut run = |mask: AllenMask| {
        prepared
            .execute(&txn, &cache, &[BindValue::AllenMask(mask)], &mut out)
            .expect("execute");
        accounts_of(&out)
    };
    assert_eq!(run(AllenMask::BEFORE), vec![1]);
    assert_eq!(run(AllenMask::DURING), vec![2]);
    assert_eq!(run(AllenMask::CONTAINS), vec![3]);
    assert_eq!(run(AllenMask::INTERSECTS), vec![2, 3]);
    assert_eq!(run(AllenMask::DISJOINT), vec![1, 4]);
    // Warm rebind back to a singleton: the same prepared query.
    assert_eq!(run(AllenMask::AFTER), vec![4]);

    // The ∅/full vacuity rules and the shape rule, at bind.
    assert!(matches!(
        prepared.execute(&txn, &cache, &[BindValue::AllenMask(AllenMask::EMPTY)], &mut out),
        Err(Error::EmptyAllenMaskParam { param }) if param.0 == 0
    ));
    assert!(matches!(
        prepared.execute(&txn, &cache, &[BindValue::AllenMask(AllenMask::FULL)], &mut out),
        Err(Error::FullAllenMaskParam { param }) if param.0 == 0
    ));
    assert!(matches!(
        prepared.execute(&txn, &cache, &[BindValue::U64(7)], &mut out),
        Err(Error::AllenMaskParamExpected { param }) if param.0 == 0
    ));
}

/// A cross-atom `Allen` with a param mask rides the executor's mask
/// residual (four endpoint slots + mask, resolved per execution by
/// `bind_allen_masks`): the same prepared join keeps intersecting pairs
/// under `INTERSECTS` and non-sharing pairs under `DISJOINT`.
#[test]
fn a_cross_atom_mask_param_resolves_into_the_executors_residual() {
    use crate::allen::AllenMask;
    use crate::ir::MaskTerm;

    let dir = TempDir::new("prepared-mask-param-residual");
    let schema = mandate_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_mandates(&env, &schema, &[(1, 10, 20), (2, 15, 25), (3, 30, 40)]);
    let cache = ImageCache::new();

    // Q(a, b) :- Mandate(account = a, active = u),
    //            Mandate(account = b, active = v),
    //            Allen(u, v, ?0), a < b.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(3))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: MaskTerm::Param(crate::ir::ParamId(0)),
                },
                lhs: Term::Var(VarId(1)),
                rhs: Term::Var(VarId(3)),
            }),
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: Term::Var(VarId(0)),
                rhs: Term::Var(VarId(2)),
            }),
        ],
    });
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let mut out = ResultBuffer::new();
    let mut run = |mask: AllenMask| {
        prepared
            .execute(&txn, &cache, &[BindValue::AllenMask(mask)], &mut out)
            .expect("execute");
        let mut pairs: Vec<(u64, u64)> = (0..out.len())
            .map(|row| match (out.get(row, 0), out.get(row, 1)) {
                (ResultValue::U64(a), ResultValue::U64(b)) => (a, b),
                other => panic!("expected u64 pair, got {other:?}"),
            })
            .collect();
        pairs.sort_unstable();
        pairs
    };
    // [10,20) ∩ [15,25) share points; [30,40) shares with neither.
    assert_eq!(run(AllenMask::INTERSECTS), vec![(1, 2)]);
    assert_eq!(run(AllenMask::DISJOINT), vec![(1, 3), (2, 3)]);
    // Warm rebind: back and forth, same prepared query.
    assert_eq!(run(AllenMask::INTERSECTS), vec![(1, 2)]);
}
