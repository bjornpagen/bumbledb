use super::*;
use crate::error::FindIndex;
use crate::ir::FoldOp;

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

#[cfg(feature = "trace")]
#[test]
fn finalize_resolves_each_distinct_intern_once() {
    use crate::obs;

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
        obs::start_capture();
        let out = fix
            .execute(prepared, &[BindValue::U64(account), BindValue::I64(-1)])
            .expect("execute");
        let events = obs::finish_capture();
        let count = events
            .iter()
            .filter(|e| e.point() == obs::names::DICT_RESOLVE)
            .count();
        (out, count)
    };

    let (out, count) = resolves(&mut prepared, 1);
    assert_eq!(out.len(), 64);
    assert_eq!(count, 1, "one distinct intern, one resolution");
    assert_eq!(out.byte_len(), "shared-memo".len(), "bytes stored once");

    let (out, count) = resolves(&mut prepared, 2);
    assert_eq!(out.len(), 16);
    assert_eq!(count, 16);

    let (out, count) = resolves(&mut prepared, 2);
    assert_eq!(count, 0, "the arena tier survives across finalizes");
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
    assert_eq!(memos, expected, "arena copies materialize the same text");
}
