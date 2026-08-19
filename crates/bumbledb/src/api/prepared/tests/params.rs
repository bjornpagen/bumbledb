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
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let mut out = Answers::new();

    prepared
        .execute(
            &txn,
            &cache,
            &[BindValue::U64(7), BindValue::I64(0)],
            &mut out,
        )
        .expect("execute");
    assert_eq!(answers_of(&out), vec![("salary".to_owned(), 5000)]);

    prepared
        .execute(
            &txn,
            &cache,
            &[BindValue::U64(7), BindValue::I64(i64::MIN)],
            &mut out,
        )
        .expect("execute");
    assert_eq!(
        answers_of(&out),
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
    assert_eq!(answers_of(&out), vec![("coffee".to_owned(), -4)]);
}

#[test]
fn bind_time_checks_reject_bad_params() {
    let dir = TempDir::new("prepared-bind-errors");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let mut out = Answers::new();

    let err = prepared
        .execute(&txn, &cache, &[BindValue::U64(7)], &mut out)
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::ParamCountMismatch {
                mismatch: crate::error::Mismatch {
                    witnessed: 1,
                    required: 2,
                },
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
    let cache = ImageCache::new(&schema);

    // Q(amount) :- Posting(memo = ?0, amount).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let mut out = Answers::new();

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
    assert_eq!(out.get(0, 0), AnswerValue::I64(-55));
}

/// The per-slot param-word memo (docs/architecture/40-execution.md):
/// re-binding the same text serves the word from the slot's memo — a
/// hit is final, the dictionary being append-only — while a different
/// text re-probes and a MISS never memoizes: a text interned by a
/// later commit must be seen by the very next bind.
#[cfg(feature = "trace")]
#[test]
fn param_word_memo_hits_are_final_and_misses_never_memoize() {
    use crate::obs;

    let dir = TempDir::new("prepared-param-word-memo");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "alpha", 10), (2, 7, "beta", 20)]);
    let cache = ImageCache::new(&schema);

    let run =
        |prepared: &mut PreparedQuery<()>, txn: &crate::storage::env::ReadTxn<'_>, text: &str| {
            obs::start_capture();
            let out = prepared
                .execute_collect(txn, &cache, &memo_param(text))
                .expect("execute");
            let events = obs::finish_capture();
            let hits = events
                .iter()
                .filter(|e| e.point() == obs::names::PARAM_WORD_MEMO)
                .count();
            (amounts_of(&out), hits)
        };

    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_memo_query()).expect("prepare");
    assert_eq!(
        run(&mut prepared, &txn, "alpha"),
        (vec![10], 0),
        "the cold bind probes the dictionary"
    );
    assert_eq!(
        run(&mut prepared, &txn, "alpha"),
        (vec![10], 1),
        "the warm re-bind hits the slot memo"
    );
    assert_eq!(
        run(&mut prepared, &txn, "beta"),
        (vec![20], 0),
        "a different text re-probes"
    );
    assert_eq!(run(&mut prepared, &txn, "beta"), (vec![20], 1));

    // The miss-finality law: "gamma" is unknown (empty result, sentinel
    // bind) and must NOT memoize — a commit interns it, and the very
    // next bind sees the fresh word through the ordinary probe.
    assert_eq!(
        run(&mut prepared, &txn, "gamma"),
        (vec![], 0),
        "a dictionary miss binds the sentinel"
    );
    assert_eq!(
        run(&mut prepared, &txn, "gamma"),
        (vec![], 0),
        "misses never memoize"
    );
    drop(txn);
    insert_postings(&env, &schema, &[(3, 7, "gamma", 30)]);
    let txn = env.read_txn().expect("txn");
    assert_eq!(
        run(&mut prepared, &txn, "gamma"),
        (vec![30], 0),
        "the post-intern bind probes and finds the fresh word"
    );
    assert_eq!(
        run(&mut prepared, &txn, "gamma"),
        (vec![30], 1),
        "and only then memoizes"
    );
}
