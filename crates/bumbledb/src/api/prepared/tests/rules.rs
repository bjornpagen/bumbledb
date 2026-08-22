use super::*;
use crate::ir::FoldOp;
use crate::ir::{HeadTerm, ParamId};

fn overlap_postings() -> Vec<(u64, u64, &'static str, i64)> {
    vec![
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "b", 25),
        (4, 7, "c", 40),
        (5, 9, "d", 55),
    ]
}

fn by_account_rule(account: u64) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(2), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    }
}

fn union_query() -> Query {
    Query {
        interiors: vec![],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![by_account_rule(3), by_account_rule(7)],
        rec: None,
    }
}

#[test]
fn a_multi_rule_query_prepares_with_every_rules_plan() {
    let dir = TempDir::new("prepared-rules-build");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let prepared = prepare(&txn, &cache, &schema, &union_query()).expect("multi-rule builds");
    assert_eq!(prepared.pipeline.main_rules().len(), 2, "one plan per rule");
    for rule in prepared.pipeline.main_rules() {

        let PreparedRule::FreeJoin(rule) = rule else {
            panic!("fixture rules use Free Join");
        };
        assert_eq!(rule.resolved_filters.len(), 1, "one occurrence per rule");
    }
    assert_eq!(
        prepared
            .signature()
            .columns
            .iter()
            .map(crate::ir::validate::SignatureColumn::ty)
            .collect::<Vec<_>>(),
        vec![&ValueType::String, &ValueType::I64],
        "the head's answer tuple types the query once"
    );
}

#[test]
fn an_overlapping_union_has_no_duplicates_and_host_concatenation_does() {
    let dir = TempDir::new("prepared-rules-union");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let floor = vec![BindValue::I64(0)];

    let mut prepared = prepare(&txn, &cache, &schema, &union_query()).expect("prepare");
    let union = prepared
        .execute_collect(&txn, &cache, &floor)
        .expect("execute");
    assert_eq!(
        answers_of(&union),
        vec![
            ("a".to_owned(), 10),
            ("b".to_owned(), 25),
            ("c".to_owned(), 40),
        ],
        "the overlap ('b', 25) appears once: the union is a set"
    );

    let mut concatenated = Vec::new();
    for account in [3, 7] {
        let mut single = prepare(
            &txn,
            &cache,
            &schema,
            &Query::single(by_account_rule(account)),
        )
        .expect("prepare");
        let out = single
            .execute_collect(&txn, &cache, &floor)
            .expect("execute");
        concatenated.extend(answers_of(&out));
    }
    concatenated.sort();
    assert_eq!(concatenated.len(), union.len() + 1, "one duplicate");
    assert_eq!(
        concatenated
            .iter()
            .filter(|answer| **answer == ("b".to_owned(), 25))
            .count(),
        2,
        "host concatenation is not a union"
    );
}

#[test]
fn params_bind_once_and_reach_all_rules() {
    let dir = TempDir::new("prepared-rules-params");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &union_query()).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::I64(20)])
        .expect("execute");
    assert_eq!(
        answers_of(&out),
        vec![("b".to_owned(), 25), ("c".to_owned(), 40)],
        "the floor filtered account 3's 10 AND account 7's nothing-below-20"
    );

    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::I64(30)])
        .expect("execute");
    assert_eq!(answers_of(&out), vec![("c".to_owned(), 40)]);
}

#[test]
fn aggregates_fold_the_union_of_head_projected_bindings() {
    let dir = TempDir::new("prepared-rules-fold");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let agg_rule = |account: u64| Rule {
        finds: vec![
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(0),
            },
            FindTerm::Count,
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![
            HeadTerm::Aggregate(crate::ir::HeadOp::Sum),
            HeadTerm::Aggregate(crate::ir::HeadOp::Count),
        ],
        rules: vec![agg_rule(3), agg_rule(7)],
        rec: None,
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);

    assert_eq!(out.get(0, 0), AnswerValue::I64(75), "Sum over the union");
    assert_eq!(out.get(0, 1), AnswerValue::U64(3), "Count counts the union");
}

#[test]
fn a_grouped_fold_absorbs_the_cross_rule_duplicate() {
    let dir = TempDir::new("prepared-rules-groups");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let rule = |account: u64| Rule {
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
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(2), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(crate::ir::HeadOp::Sum)],
        rules: vec![rule(3), rule(7)],
        rec: None,
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(String, i64)> = (0..out.len())
        .map(|answer| {
            let AnswerValue::String(memo) = out.get(answer, 0) else {
                panic!("column 0 is a string");
            };
            let AnswerValue::I64(sum) = out.get(answer, 1) else {
                panic!("column 1 is an i64");
            };
            (memo.to_owned(), sum)
        })
        .collect();
    answers.sort();
    assert_eq!(
        answers,
        vec![
            ("a".to_owned(), 10),

            ("b".to_owned(), 25),
            ("c".to_owned(), 40),
        ]
    );
}

/// Cross-rule fold-free nullary `Count` is refused at validation (ruled
/// 2026-07-23, R1): under the head-projection law every binding projects to the
/// empty head tuple, so the union is a singleton and the Count is
/// definitionally the constant 1 — an uninformative query, made unrepresentable
/// beside `ArgAcrossRules` with the same modeling answer: one Count per
/// disjunct, host-merged.
#[test]
fn the_all_count_head_across_rules_is_the_typed_validation_refusal() {
    let dir = TempDir::new("prepared-rules-count");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let rule = |account: u64| Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Aggregate(crate::ir::HeadOp::Count)],
        rules: vec![rule(3), rule(7)],
        rec: None,
    };
    let Err(err) = prepare(&txn, &cache, &schema, &query) else {
        panic!("fold-free nullary Count across written rules must refuse at validation");
    };
    assert!(
        matches!(
            err,
            Error::Validation(crate::error::ValidationError::CountAcrossRules { rules: 2 })
        ),
        "typed, named, counted: {err:?}"
    );
}

#[test]
fn a_grouped_count_head_across_rules_is_the_typed_validation_refusal() {
    let dir = TempDir::new("prepared-rules-grouped-count");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let rule = |account: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Count],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(2), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(crate::ir::HeadOp::Count)],
        rules: vec![rule(3), rule(7)],
        rec: None,
    };
    let Err(err) = prepare(&txn, &cache, &schema, &query) else {
        panic!("grouped fold-free Count across written rules must refuse at validation");
    };
    assert!(
        matches!(
            err,
            Error::Validation(crate::error::ValidationError::CountAcrossRules { rules: 2 })
        ),
        "typed, named, counted: {err:?}"
    );
}

/// The or-transparency law (ruled 2026-07-23, R2): a DNF-derived rule set
/// re-keys the union dedup on the shared slot arrays, so surface `or` never
/// changes a fold domain — distinct full bindings that project to EQUAL head
/// rows all fold, and the nullary Count counts the written rule's binding set
/// (`lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`).
#[test]
fn an_or_spelled_fold_keeps_the_written_rules_full_binding_domain() {
    let dir = TempDir::new("prepared-rules-dnf-fold");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let rule = Rule {
        finds: vec![
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(0),
            },
            FindTerm::Count,
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(1))),
                (FieldId(2), Term::Var(VarId(2))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Or(vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(0)),
                rhs: Term::Literal(Value::I64(25)),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(0)),
                rhs: Term::Literal(Value::I64(55)),
            }),
        ])],
    };
    let query = Query {
        interiors: vec![],
        head: vec![
            HeadTerm::Aggregate(crate::ir::HeadOp::Sum),
            HeadTerm::Aggregate(crate::ir::HeadOp::Count),
        ],
        rules: vec![rule],
        rec: None,
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert_eq!(
        prepared.pipeline.main_rules().len(),
        2,
        "the or lowered to two disjunct rules"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);

    assert_eq!(
        out.get(0, 0),
        AnswerValue::I64(145),
        "or moved no fold domain"
    );
    assert_eq!(
        out.get(0, 1),
        AnswerValue::U64(4),
        "Count counts full bindings"
    );
}

#[test]
fn introspection_reports_per_rule_stats_and_the_union_accounting() {
    let dir = TempDir::new("prepared-rules-introspect");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &union_query()).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[ParamArg::Scalar(BindValue::I64(0))])
        .expect("execute");
    assert_eq!(out.len(), 3, "the union");
    let (_, report) = prepared
        .introspect(&txn, &cache, &[ParamArg::Scalar(BindValue::I64(0))])
        .expect("introspect");
    assert!(report.contains("query:"), "{report}");
}

#[test]
fn a_key_probe_rule_unions_through_the_sink() {
    let dir = TempDir::new("prepared-rules-key_probe");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let key_probe_rule = Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(2))),
                (FieldId(2), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let mut rule0 = by_account_rule(3);
    rule0.conditions.clear(); 
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![rule0, key_probe_rule],
        rec: None,
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.pipeline.main_rules()[1], PreparedRule::KeyProbe(_)),
        "rule 1 classifies as the point fast path"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(
        answers_of(&out),
        vec![("a".to_owned(), 10), ("b".to_owned(), 25)],
        "the key_probe's re-derivation is absorbed by the spanning seen-set"
    );
}
