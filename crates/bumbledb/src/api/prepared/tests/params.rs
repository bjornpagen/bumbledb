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
        .execute(&txn, &cache, &[Value::U64(7), Value::I64(0)], &mut out)
        .expect("execute");
    assert_eq!(rows_of(&out), vec![("salary".to_owned(), 5000)]);

    prepared
        .execute(
            &txn,
            &cache,
            &[Value::U64(7), Value::I64(i64::MIN)],
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
            &[Value::U64(8), Value::I64(i64::MIN)],
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
        .execute(&txn, &cache, &[Value::U64(7)], &mut out)
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
        .execute(&txn, &cache, &[Value::I64(7), Value::I64(0)], &mut out)
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
    let query = Query {
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
    };
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let mut out = ResultBuffer::new();

    // Never-interned value: empty, not an error.
    prepared
        .execute(
            &txn,
            &cache,
            &[Value::String(Box::from(&b"groceries"[..]))],
            &mut out,
        )
        .expect("execute");
    assert!(out.is_empty());
    drop(txn);

    // A later commit interns it; the SAME prepared query now finds rows
    // (per-execution resolution — no stale-resolution trap).
    insert_postings(&env, &schema, &[(2, 9, "groceries", -55)]);
    let txn = env.read_txn().expect("txn");
    prepared
        .execute(
            &txn,
            &cache,
            &[Value::String(Box::from(&b"groceries"[..]))],
            &mut out,
        )
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::I64(-55));
}
