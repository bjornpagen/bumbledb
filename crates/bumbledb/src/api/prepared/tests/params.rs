use super::*;

#[test]
fn prepare_once_execute_many_with_varying_params() {
    let fix = postings(&[
        (1, 7, "rent", -1200),
        (2, 7, "salary", 5000),
        (3, 8, "coffee", -4),
    ]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let mut out = Answers::new();

    fix.execute_into(
        &mut prepared,
        &[BindValue::U64(7), BindValue::I64(0)],
        &mut out,
    )
    .expect("execute");
    assert_eq!(answers_of(&out), vec![("salary".to_owned(), 5000)]);

    fix.execute_into(
        &mut prepared,
        &[BindValue::U64(7), BindValue::I64(i64::MIN)],
        &mut out,
    )
    .expect("execute");
    assert_eq!(
        answers_of(&out),
        vec![("rent".to_owned(), -1200), ("salary".to_owned(), 5000)]
    );

    fix.execute_into(
        &mut prepared,
        &[BindValue::U64(8), BindValue::I64(i64::MIN)],
        &mut out,
    )
    .expect("execute");
    assert_eq!(answers_of(&out), vec![("coffee".to_owned(), -4)]);
}

#[test]
fn bind_time_checks_reject_bad_params() {
    let fix = postings(&[]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let mut out = Answers::new();

    let err = fix
        .execute_into(&mut prepared, &[BindValue::U64(7)], &mut out)
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

    let err = fix
        .execute_into(
            &mut prepared,
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
    let fix = posting_store("prepared-string-params", &[(1, 7, "rent", -1200)]);

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
    let mut prepared = fix.prepare(&query).expect("prepare");
    let mut out = Answers::new();

    fix.execute_into(&mut prepared, &[BindValue::Str("groceries")], &mut out)
        .expect("execute");
    assert!(out.is_empty());

    fix.insert_dyn(POSTING, &posting_rows(&[(2, 9, "groceries", -55)]));
    fix.execute_into(&mut prepared, &[BindValue::Str("groceries")], &mut out)
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::I64(-55));
}

#[cfg(feature = "trace")]
#[test]
fn param_word_memo_hits_are_final() {
    use crate::obs;

    // Interning latches finally: the first bind of any text mints/finds its
    // token and memoizes; every re-bind of the same text hits the memo —
    // including texts no stored row carries (a fresh token matches
    // nothing, and a later write of that text meets the SAME token).
    let fix = posting_store(
        "prepared-param-word-memo",
        &[(1, 7, "alpha", 10), (2, 7, "beta", 20)],
    );

    let run = |fix: &StoreFix, prepared: &mut PreparedQuery<T>, text: &str| {
        obs::start_capture();
        let out = fix.execute(prepared, &memo_param(text)).expect("execute");
        let events = obs::finish_capture();
        let hits = events
            .iter()
            .filter(|e| e.point() == obs::names::PARAM_WORD_MEMO)
            .count();
        (amounts_of(&out), hits)
    };

    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");
    assert_eq!(
        run(&fix, &mut prepared, "alpha"),
        (vec![10], 0),
        "the cold bind interns"
    );
    assert_eq!(
        run(&fix, &mut prepared, "alpha"),
        (vec![10], 1),
        "the warm re-bind hits the slot memo"
    );
    assert_eq!(
        run(&fix, &mut prepared, "beta"),
        (vec![20], 0),
        "a different text re-interns"
    );
    assert_eq!(run(&fix, &mut prepared, "beta"), (vec![20], 1));

    assert_eq!(
        run(&fix, &mut prepared, "gamma"),
        (vec![], 0),
        "an unstored text binds a fresh final token"
    );
    assert_eq!(
        run(&fix, &mut prepared, "gamma"),
        (vec![], 1),
        "final latches memoize immediately"
    );
    fix.insert_dyn(POSTING, &posting_rows(&[(3, 7, "gamma", 30)]));
    assert_eq!(
        run(&fix, &mut prepared, "gamma"),
        (vec![30], 1),
        "the memoized token identifies the newly stored text"
    );
}
