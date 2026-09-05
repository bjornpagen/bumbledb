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
fn a_str_literal_latches_on_first_execution() {
    let fix = postings(&[(1, 7, "alice", 10), (2, 7, "bob", 20)]);
    let mut prepared = fix.prepare(&literal_query("alice")).expect("prepare");
    assert_eq!(prepared.latch.remaining(), 1, "counted at prepare");

    let mut out = Answers::new();
    fix.execute_into(&mut prepared, &[] as &[BindValue], &mut out)
        .expect("execute");
    assert_eq!(amounts(&out), vec![10]);
    assert_eq!(prepared.latch.remaining(), 0, "the latch is final");
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
    assert!(!pending, "the template slot was rewritten in place");

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
    assert_eq!(
        prepared.latch.remaining(),
        0,
        "interning latched the literal on its first resolution"
    );
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
        .read(|instance| prepared.introspect(instance, &[]))
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

#[cfg(feature = "trace")]
#[test]
fn the_latch_fires_once_and_the_fast_path_skips_resolution() {
    use crate::obs;

    let fix = postings(&[(1, 7, "alice", 10), (2, 7, "bob", 20)]);
    let mut prepared = fix.prepare(&literal_query("alice")).expect("prepare");
    let mut out = Answers::new();

    obs::start_capture();
    fix.execute_into(&mut prepared, &[] as &[BindValue], &mut out)
        .expect("execute");
    let events = obs::finish_capture();
    let slow = amounts(&out);
    assert_eq!(
        events
            .iter()
            .filter(|e| e.point() == obs::names::LITERAL_LATCH)
            .count(),
        1,
        "one latch per distinct literal"
    );
    assert!(
        events
            .iter()
            .any(|e| e.point() == obs::names::RESOLVE_FILTERS),
        "the first execution resolves"
    );

    obs::start_capture();
    fix.execute_into(&mut prepared, &[] as &[BindValue], &mut out)
        .expect("execute");
    let events = obs::finish_capture();
    assert_eq!(amounts(&out), slow, "fast path, identical results");
    assert!(
        !events
            .iter()
            .any(|e| e.point() == obs::names::LITERAL_LATCH),
        "a latch fires once, ever"
    );
    assert!(
        !events
            .iter()
            .any(|e| e.point() == obs::names::RESOLVE_FILTERS),
        "resolve_filters provably skipped"
    );
}
