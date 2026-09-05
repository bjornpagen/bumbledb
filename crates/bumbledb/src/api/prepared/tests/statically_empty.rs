use super::*;
use crate::ir::HeadTerm;
use crate::ir::normalize::with_fold_disabled;
use bumbledb_theory::schema::{IntervalElement, SchemaDescriptor};

fn event_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Event".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                },
                FieldDescriptor {
                    name: "score".into(),
                    value_type: ValueType::I64,
                },
            ],
        }],
        statements: vec![],
    }
}

const EVENT: RelationId = RelationId(0);

fn events(rows: &[(u64, u64, (i64, i64), i64)]) -> Fix {
    let facts: Vec<Vec<Value>> = rows
        .iter()
        .map(|(id, kind, (start, end), score)| {
            vec![
                Value::U64(*id),
                Value::U64(*kind),
                Value::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
                Value::I64(*score),
            ]
        })
        .collect();
    Fix::heap(event_descriptor(), &[(EVENT, facts)])
}

fn by_kind_rule(kind: u64, conditions: Vec<Comparison>) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(EVENT),
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(kind))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: conditions.into_iter().map(ConditionTree::Leaf).collect(),
    }
}

fn score_cmp(op: CmpOp, value: i64) -> Comparison {
    Comparison {
        op,
        lhs: Term::Var(VarId(0)),
        rhs: Term::Literal(Value::I64(value)),
    }
}

fn contradiction() -> Vec<Comparison> {
    vec![score_cmp(CmpOp::Gt, 5), score_cmp(CmpOp::Lt, 3)]
}

fn scores_of(buffer: &Answers) -> Vec<i64> {
    let mut scores: Vec<i64> = (0..buffer.len())
        .map(|answer| {
            let AnswerValue::I64(score) = buffer.get(answer, 0) else {
                panic!("column 0 is an i64");
            };
            score
        })
        .collect();
    scores.sort_unstable();
    scores
}

#[test]
fn a_dead_rule_beside_a_live_one_runs_the_live_one_only() {
    let fix = events(&[
        (1, 3, (0, 10), 10),
        (2, 3, (0, 10), 25),
        (3, 7, (0, 10), 40),
    ]);

    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![by_kind_rule(3, contradiction()), by_kind_rule(7, vec![])],
        rec: None,
    };
    let mut prepared = fix.prepare(&query).expect("prepare");

    assert_eq!(
        prepared.pipeline.main_rules().len(),
        1,
        "the dead rule prepared no plan"
    );
    assert!(matches!(
        prepared.pipeline.main_rules(),
        [PreparedRule::FreeJoin(_)]
    ));

    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(scores_of(&out), vec![40], "kind 7's row; kind 3 never ran");
}

#[cfg(feature = "trace")]
#[test]
fn a_dead_rule_opens_no_rule_span() {
    use crate::obs;

    let fix = events(&[(1, 7, (0, 10), 40)]);

    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![by_kind_rule(3, contradiction()), by_kind_rule(7, vec![])],
        rec: None,
    };
    let mut prepared = fix.prepare(&query).expect("prepare");
    obs::start_capture();
    fix.execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let events = obs::finish_capture();
    let rule_spans: Vec<obs::TracePoint> = events
        .iter()
        .map(|e| e.point())
        .filter(|p| matches!(p, obs::TracePoint::Rule(_)))
        .collect();
    assert_eq!(
        rule_spans,
        vec![obs::names::RULE[0]],
        "one rule span: the live rule"
    );
}

#[cfg(feature = "trace")]
#[test]
fn the_empty_query_builds_no_image_and_binds_no_view() {
    use crate::obs;

    let fix = events(&[(1, 3, (0, 10), 10)]);

    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![by_kind_rule(3, contradiction())],
        rec: None,
    };
    let mut prepared = fix.prepare(&query).expect("prepare");
    assert!(matches!(
        prepared.pipeline,
        PreparedPipeline::Cq { ref rules, .. } if rules.is_empty()
    ));

    obs::start_capture();
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let events = obs::finish_capture();
    assert_eq!(out.len(), 0);
    let names: Vec<obs::TracePoint> = events.iter().map(|e| e.point()).collect();
    for touched in [
        obs::names::IMAGE_BUILD,
        obs::names::CACHE_HIT,
        obs::names::VIEW_BUILD,
        obs::names::JOIN,
    ] {
        assert!(
            !names.contains(&touched),
            "the empty query must not reach {touched}: {names:?}"
        );
    }
}

#[test]
fn folded_and_unfolded_executions_agree_on_random_single_slot_filters() {
    let rows: Vec<(u64, u64, (i64, i64), i64)> = (0..40u64)
        .map(|i| {
            let score = i64::try_from(i).expect("small") - 20;
            (i + 1, i % 5, (0, 10), score)
        })
        .collect();
    let fix = events(&rows);

    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for round in 0..64 {
        let count = next() % 4 + 1;
        let conditions: Vec<Comparison> = (0..count)
            .map(|_| {
                let op = match next() % 6 {
                    0 => CmpOp::Lt,
                    1 => CmpOp::Le,
                    2 => CmpOp::Gt,
                    3 => CmpOp::Ge,
                    4 => CmpOp::Eq,
                    _ => CmpOp::Ne,
                };
                let value = i64::try_from(next() % 17).expect("small") - 8;
                score_cmp(op, value)
            })
            .collect();
        let query = Query::single(by_kind_rule(next() % 5, conditions));

        let mut folded = fix.prepare(&query).expect("prepare folded");
        let mut unfolded = with_fold_disabled(|| fix.prepare(&query)).expect("prepare raw");
        let folded_answers = scores_of(
            &fix.execute(&mut folded, &[] as &[BindValue])
                .expect("folded"),
        );
        let unfolded_answers = scores_of(
            &fix.execute(&mut unfolded, &[] as &[BindValue])
                .expect("unfolded"),
        );
        assert_eq!(
            folded_answers, unfolded_answers,
            "round {round}: the fold changed the denotation"
        );
    }
}
