use super::*;

/// A finalize-time Overflow leaves `Answers`
/// discardable — the same prepared query re-executes cleanly into
/// the same carrier (deterministic error), and a passing query then
/// fills that carrier with exactly its own answers.
#[test]
fn overflow_errors_leave_answers_reusable() {
    let dir = TempDir::new("prepared-overflow-reuse");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[(1, 7, "a", i64::MAX), (2, 7, "b", 1), (3, 8, "c", 4)],
    );
    // Sum by account: account 7 overflows at finalize.
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: crate::ir::AggOp::Sum,
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
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new(&schema);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    let mut out = Answers::new();
    for _ in 0..2 {
        let err = prepared
            .execute(&txn, &cache, &[], &mut out)
            .expect_err("account 7 overflows");
        assert!(
            matches!(
                err,
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: 1 })
            ),
            "{err:?}"
        );
    }
    // A passing query fills the same carrier with exactly its answers.
    let ok_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Literal(crate::ir::Value::U64(8)),
        })],
    });
    let mut ok = prepare(&txn, &cache, &schema, &ok_query).expect("prepares");
    ok.execute(&txn, &cache, &[], &mut out).expect("executes");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(8));
    assert_eq!(out.get(0, 1), AnswerValue::I64(4));
}

#[test]
fn answer_reuse_retains_capacity_and_answers_stay_identical() {
    let dir = TempDir::new("prepared-buffer-reuse");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[(1, 7, "one", 1), (2, 7, "two", 2), (3, 7, "three", 3)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let mut out = Answers::new();
    let params = [BindValue::U64(7), BindValue::I64(0)];

    prepared
        .execute(&txn, &cache, &params, &mut out)
        .expect("execute");
    let first = answers_of(&out);
    let (cells_cap, bytes_cap) = (out.cells.capacity(), out.bytes.capacity());
    assert!(cells_cap > 0 && bytes_cap > 0);

    prepared
        .execute(&txn, &cache, &params, &mut out)
        .expect("execute");
    assert_eq!(answers_of(&out), first);
    // Capacity is retained across reuse (the zero-alloc path).
    assert!(out.cells.capacity() >= cells_cap);
    assert!(out.bytes.capacity() >= bytes_cap);
    assert_eq!(first.len(), 3);
}

/// Finalize resolves each distinct intern once per finalize and
/// stores its bytes once per answer carrier (docs/architecture/40-execution.md).
#[cfg(feature = "trace")]
#[test]
fn finalize_resolves_each_distinct_intern_once() {
    use crate::obs;

    let dir = TempDir::new("prepared-resolve-memo");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // 64 facts sharing one memo (distinct amounts keep the answers
    // distinct under set semantics), plus 16 facts over 16 memos.
    let facts: Vec<(u64, u64, String, i64)> = (0..64)
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
    let borrowed: Vec<(u64, u64, &str, i64)> = facts
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    insert_postings(&env, &schema, &borrowed);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");

    let resolves = |prepared: &mut PreparedQuery<'_, ()>, account: u64| {
        obs::start_capture();
        let out = prepared
            .execute_collect(&txn, &cache, &[BindValue::U64(account), BindValue::I64(-1)])
            .expect("execute");
        let events = obs::finish_capture();
        let count = events
            .iter()
            .filter(|e| e.name == obs::names::DICT_RESOLVE)
            .count();
        (out, count)
    };

    // 64 answers, one distinct memo: one resolution, one byte copy.
    let (out, count) = resolves(&mut prepared, 1);
    assert_eq!(out.len(), 64);
    assert_eq!(count, 1, "one distinct intern, one resolution");
    assert_eq!(out.byte_len(), "shared-memo".len(), "bytes stored once");

    // 16 answers over 16 memos: sixteen resolutions.
    let (out, count) = resolves(&mut prepared, 2);
    assert_eq!(out.len(), 16);
    assert_eq!(count, 16);
    // A second execution memoizes per finalize, not across them.
    let (_, count) = resolves(&mut prepared, 2);
    assert_eq!(count, 16, "the memo clears per finalize");
}
