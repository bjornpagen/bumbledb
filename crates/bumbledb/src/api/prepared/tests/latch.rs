use super::*;

fn literal_query(memo: &str) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(2), Term::Literal(Value::String(memo.into()))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn amounts(out: &Answers) -> Vec<i64> {
    let mut amounts: Vec<i64> = out
        .answers()
        .map(|answer| match answer.get(0) {
            AnswerValue::I64(v) => v,
            other => panic!("i64 find, got {other:?}"),
        })
        .collect();
    amounts.sort_unstable();
    amounts
}

#[test]
fn literal_bytes_survive_trim_and_switches_between_resident_and_fallback() {
    let fix = posting_store(
        "prepared-literal-generation",
        &[(1, 7, "alpha", 10), (2, 7, "beta", 20)],
    );
    let mut alpha = fix.prepare(&literal_query("alpha")).expect("alpha plan");
    let mut beta = fix.prepare(&literal_query("beta")).expect("beta plan");
    assert!(
        !alpha.no_text_probe && !beta.no_text_probe,
        "Free Join retains eager text generation binding"
    );
    for fallback in [false, true, false, true] {
        alpha.force_cursor_fallback(fallback);
        let out = fix.execute(&mut alpha, &[] as &[BindValue]).expect("alpha");
        assert_eq!(amounts(&out), vec![10], "fallback={fallback}");
        alpha.release_memory();
        fix.db.clear_cache();
        let out = fix.execute(&mut beta, &[] as &[BindValue]).expect("beta");
        assert_eq!(amounts(&out), vec![20]);
        let out = fix
            .execute(&mut alpha, &[] as &[BindValue])
            .expect("alpha rebound");
        assert_eq!(amounts(&out), vec![10], "rebound fallback={fallback}");
    }
}

#[test]
fn a_str_literal_latches_on_first_execution() {
    let fix = postings(&[(1, 7, "alice", 10), (2, 7, "bob", 20)]);
    let mut prepared = fix.prepare(&literal_query("alice")).expect("prepare");

    let mut out = Answers::new();
    fix.execute_into(&mut prepared, &[] as &[BindValue], &mut out)
        .expect("execute");
    assert_eq!(amounts(&out), vec![10]);
    let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
        panic!("free join fixture");
    };
    assert_eq!(rule.resolution, ResolutionState::Complete);

    let plan = &rule.plan;
    let pending = plan.occurrences().iter().any(|occurrence| {
        occurrence
            .selections
            .iter()
            .any(|selection| matches!(selection.value, Const::PendingIntern { .. }))
            || occurrence.filters.iter().any(|filter| {
                matches!(
                    filter,
                    FilterPredicate::Compare {
                        value: Const::PendingIntern { .. },
                        ..
                    }
                )
            })
    });
    assert!(pending, "immutable literal bytes survive resolver rotation");

    fix.execute_into(&mut prepared, &[] as &[BindValue], &mut out)
        .expect("re-execute");
    assert_eq!(amounts(&out), vec![10]);
}

#[test]
fn an_unmatched_literal_latches_finally_and_matches_later_rows() {
    // The interner never misses: an absent text mints a token on the first
    // resolution (empty result), and the SAME token identifies the text
    // when a later write stores it — append-only latches are final.
    let fix = posting_store("prepared-latch-late-text", &[(1, 7, "alice", 10)]);

    let mut prepared = fix.prepare(&literal_query("carol")).expect("prepare");
    let mut out = Answers::new();
    fix.execute_into(&mut prepared, &[] as &[BindValue], &mut out)
        .expect("execute");
    assert!(out.is_empty(), "no stored row carries the literal yet");
    assert!(
        matches!(
            prepared.pipeline.main_rules(),
            [PreparedRule::FreeJoin(FreeJoinRule {
                resolution: ResolutionState::Complete,
                ..
            })]
        ),
        "a complete resolution arms the fast path"
    );
    let (empty, report) = fix
        .db
        .read(crate::api::db::test_operation(), |instance| {
            prepared.introspect(instance, &[])
        })
        .expect("introspect");
    assert!(empty.is_empty());
    assert!(!report.contains("pending literals:"), "{report}");

    fix.insert_dyn(POSTING, &posting_rows(&[(2, 8, "carol", 30)]));
    fix.execute_into(&mut prepared, &[] as &[BindValue], &mut out)
        .expect("execute after the write");
    assert_eq!(
        amounts(&out),
        vec![30],
        "the latched token identifies the newly stored text"
    );
}
