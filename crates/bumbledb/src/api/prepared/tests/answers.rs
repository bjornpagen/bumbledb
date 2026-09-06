use super::*;
use crate::error::FindIndex;
use crate::ir::FoldOp;

fn finalize_mixed_test_rows(
    rows: &[[u64; 4]],
    out: &mut Answers,
    charged: bool,
) -> crate::error::Result<()> {
    use crate::exec::run::{Bindings, Sink};
    use crate::ir::validate::SignatureColumn;

    let mut projection = ProjectionSink::new(vec![0, 1, 2, 3]);
    let mut bindings = Bindings::new(4);
    for row in rows {
        for (slot, word) in row.iter().enumerate() {
            bindings.set(slot, *word);
        }
        assert!(!projection.emit(&bindings).is_terminal());
    }
    let work = crate::api::db::test_operation().unwrap();
    let interner = crate::image::intern::InternerHandle::without_text(&work);
    let columns = [
        ValueType::U64,
        ValueType::FixedBytes { len: 9 },
        ValueType::F64,
    ]
    .map(|ty| SignatureColumn::Project { ty });
    let mut charge = charged.then(|| super::super::result::ResultCharge::new(&work, usize::MAX));
    super::super::finalize::finalize(
        &mut EitherSink::Projection(projection),
        &mut Vec::new(),
        &mut ResolveMemo::new(),
        &interner,
        None,
        &columns,
        out,
        charge.as_mut(),
    )
}

#[test]
fn bulk_finalize_keeps_initialized_prefix_on_error_and_matches_rowwise_retry() {
    let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    let rows = [
        [
            7,
            0x0102_0304_0506_0708,
            0x0900_0000_0000_0000,
            0xbff0_0000_0000_0000,
        ],
        [
            8,
            0x0102_0304_0506_0708,
            0x0900_0000_0000_0000,
            0xc000_0000_0000_0000,
        ],
    ];
    let mut bad = rows;
    bad[1][3] = 0; // Noncanonical F64 order key, after the first float succeeds.
    let mut out = Answers::new();
    out.begin(3);
    out.push_value(&AnswerValue::U64(99));
    out.push_value(&AnswerValue::FixedBytes(&bytes));
    out.push_value(&AnswerValue::F64(crate::F64::from(3.0)));
    let failed = finalize_mixed_test_rows(&bad, &mut out, false);
    assert!(matches!(failed, Err(Error::Corruption(_))));
    assert_eq!(
        out.len(),
        1,
        "no partially initialized appended row is published"
    );
    assert_eq!(out.get(0, 0), AnswerValue::U64(99));
    assert_eq!(out.get(0, 1), AnswerValue::FixedBytes(&bytes));
    assert_eq!(out.get(0, 2), AnswerValue::F64(crate::F64::from(3.0)));

    // Both layouts must agree with literals, including the two-word byte
    // find between single-word finds and reuse after a failed bulk fill.
    for charged in [false, true, false] {
        out.begin(3);
        finalize_mixed_test_rows(&rows, &mut out, charged).unwrap();
        assert_eq!(out.len(), 2);
        for (row, id, value) in [(0, 7, 1.0), (1, 8, 2.0)] {
            assert_eq!(out.get(row, 0), AnswerValue::U64(id));
            assert_eq!(out.get(row, 1), AnswerValue::FixedBytes(&bytes));
            assert_eq!(out.get(row, 2), AnswerValue::F64(crate::F64::from(value)));
        }
    }
}

#[test]
fn overflow_errors_leave_answers_reusable() {
    let fix = postings(&[(1, 7, "a", i64::MAX), (2, 7, "b", 1), (3, 8, "c", 4)]);

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
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
    let mut prepared = fix.prepare(&query).expect("prepares");
    let mut out = Answers::new();
    for _ in 0..2 {
        let err = fix
            .execute_into(&mut prepared, &[] as &[BindValue], &mut out)
            .expect_err("account 7 overflows");
        assert!(
            matches!(
                err,
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: FindIndex(1) })
            ),
            "{err:?}"
        );
    }

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
    let mut ok = fix.prepare(&ok_query).expect("prepares");
    fix.execute_into(&mut ok, &[] as &[BindValue], &mut out)
        .expect("executes");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(8));
    assert_eq!(out.get(0, 1), AnswerValue::I64(4));
}

#[test]
fn answer_reuse_retains_capacity_and_answers_stay_identical() {
    let fix = postings(&[(1, 7, "one", 1), (2, 7, "two", 2), (3, 7, "three", 3)]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let mut out = Answers::new();
    let params = [BindValue::U64(7), BindValue::I64(0)];

    fix.execute_into(&mut prepared, &params, &mut out)
        .expect("execute");
    let first = answers_of(&out);
    let (cells_cap, text_cap) = (out.cells.capacity(), out.text.capacity());
    assert!(cells_cap > 0 && text_cap > 0);

    fix.execute_into(&mut prepared, &params, &mut out)
        .expect("execute");
    assert_eq!(answers_of(&out), first);

    assert!(out.cells.capacity() >= cells_cap);
    assert!(out.text.capacity() >= text_cap);
    assert_eq!(first.len(), 3);
}

#[test]
fn finalize_materializes_each_distinct_intern_once() {
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
    let fix = postings(&borrowed);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");

    let resolves = |prepared: &mut PreparedQuery<T>, account: u64| {
        let out = fix
            .execute(prepared, &[BindValue::U64(account), BindValue::I64(-1)])
            .expect("execute");
        let count = prepared.resolve_memo.ranges.len();
        (out, count)
    };

    let (out, count) = resolves(&mut prepared, 1);
    assert_eq!(out.len(), 64);
    assert_eq!(count, 1, "one memo entry for the shared intern");
    assert_eq!(out.byte_len(), "shared-memo".len(), "bytes stored once");

    let (out, count) = resolves(&mut prepared, 2);
    assert_eq!(out.len(), 16);
    assert_eq!(count, 16);

    let (out, count) = resolves(&mut prepared, 2);
    assert_eq!(
        count, 16,
        "each finalize re-resolves into the charged answer heap"
    );
    assert_eq!(out.len(), 16);
    let mut memos: Vec<String> = (0..out.len())
        .map(|answer| {
            let AnswerValue::String(memo) = out.get(answer, 0) else {
                panic!("column 0 is a string");
            };
            memo.to_owned()
        })
        .collect();
    memos.sort();
    let expected: Vec<String> = {
        let mut memos: Vec<String> = (0..16).map(|i| format!("m{i}")).collect();
        memos.sort();
        memos
    };
    assert_eq!(
        memos, expected,
        "charged answer heap materializes the same text"
    );
}

/// Forced work refusal during text compare fails the query. It does
/// not return a successful empty or wrong answer. Verification: `NotRun`.
#[test]
fn text_compare_refusal_fails_the_query() {
    let fix = postings(&[(1, 7, "alpha", 10), (2, 7, "beta", 20)]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    prepared.force_cursor_fallback(true);
    let work = crate::api::prepared::source::UNBOUNDED_POLICY
        .start()
        .expect("work");
    work.cancel();
    let source = crate::api::prepared::source::QuerySource::heap(&fix.instance, 1, work);
    let mut out = Answers::new();
    let err = prepared.execute_source(&source, &[BindValue::U64(7), BindValue::I64(0)], &mut out);
    assert!(err.is_err(), "refusal must fail the query");
}
