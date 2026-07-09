use super::*;

/// PRD 08 (docs/perf/): a finalize-time Overflow leaves the buffer
/// discardable — the same prepared query re-executes cleanly into
/// the same buffer (deterministic error), and a passing query then
/// fills that buffer with exactly its own rows.
#[test]
fn overflow_errors_leave_the_buffer_reusable() {
    let dir = TempDir::new("prepared-overflow-reuse");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[(1, 7, "a", i64::MAX), (2, 7, "b", 1), (3, 8, "c", 4)],
    );
    // Sum by account: account 7 overflows at finalize.
    let query = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: crate::ir::AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new();
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    let mut out = ResultBuffer::new();
    for _ in 0..2 {
        let err = prepared
            .execute(&txn, &cache, &[], &mut out)
            .expect_err("account 7 overflows");
        assert!(matches!(err, Error::Overflow { find: 1 }), "{err:?}");
    }
    // A passing query fills the same buffer with exactly its rows.
    let ok_query = Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Eq,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Literal(crate::ir::Value::U64(8)),
        }],
    };
    let mut ok = prepare(&txn, &cache, &schema, &ok_query).expect("prepares");
    ok.execute(&txn, &cache, &[], &mut out).expect("executes");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::U64(8));
    assert_eq!(out.get(0, 1), ResultValue::I64(4));
}

#[test]
fn buffer_reuse_retains_capacity_and_results_stay_identical() {
    let dir = TempDir::new("prepared-buffer-reuse");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[(1, 7, "one", 1), (2, 7, "two", 2), (3, 7, "three", 3)],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let mut out = ResultBuffer::new();
    let params = [Value::U64(7), Value::I64(0)];

    prepared
        .execute(&txn, &cache, &params, &mut out)
        .expect("execute");
    let first = rows_of(&out);
    let (cells_cap, bytes_cap) = (out.cells.capacity(), out.bytes.capacity());
    assert!(cells_cap > 0 && bytes_cap > 0);

    prepared
        .execute(&txn, &cache, &params, &mut out)
        .expect("execute");
    assert_eq!(rows_of(&out), first);
    // Capacity is retained across reuse (the zero-alloc path).
    assert!(out.cells.capacity() >= cells_cap);
    assert!(out.bytes.capacity() >= bytes_cap);
    assert_eq!(first.len(), 3);
}

/// Finalize resolves each distinct intern once per finalize and
/// stores its bytes once per buffer (docs/architecture/40-execution.md).
#[cfg(feature = "trace")]
#[test]
fn finalize_resolves_each_distinct_intern_once() {
    use crate::obs;

    let dir = TempDir::new("prepared-resolve-memo");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // 64 rows sharing one memo (distinct amounts keep the rows
    // distinct under set semantics), plus 16 rows over 16 memos.
    let rows: Vec<(u64, u64, String, i64)> = (0..64)
        .map(|id| {
            (
                id,
                1,
                "shared-memo".to_owned(),
                i64::try_from(id).expect("fits"),
            )
        })
        .chain((0..16).map(|i| (64 + i, 2, format!("m{i}"), i64::try_from(i).expect("fits"))))
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    insert_postings(&env, &schema, &borrowed);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");

    let resolves = |prepared: &mut PreparedQuery<'_>, account: u64| {
        obs::start_capture();
        let out = prepared
            .execute_collect(&txn, &cache, &[Value::U64(account), Value::I64(-1)])
            .expect("execute");
        let events = obs::finish_capture();
        let count = events
            .iter()
            .filter(|e| e.name == obs::names::DICT_RESOLVE)
            .count();
        (out, count)
    };

    // 64 rows, one distinct memo: one resolution, one byte copy.
    let (out, count) = resolves(&mut prepared, 1);
    assert_eq!(out.len(), 64);
    assert_eq!(count, 1, "one distinct intern, one resolution");
    assert_eq!(out.byte_len(), "shared-memo".len(), "bytes stored once");

    // 16 rows over 16 memos: sixteen resolutions.
    let (out, count) = resolves(&mut prepared, 2);
    assert_eq!(out.len(), 16);
    assert_eq!(count, 16);
    // A second execution memoizes per finalize, not across them.
    let (_, count) = resolves(&mut prepared, 2);
    assert_eq!(count, 16, "the memo clears per finalize");
}
