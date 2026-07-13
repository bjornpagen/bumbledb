//! The rule loop (docs/architecture/40-execution.md § the rule loop):
//! one head, one sink — rules run sequentially and the sink's seen-set
//! spanning rules IS the union. No merge node exists to test; what these
//! tests pin is the *absence* of duplicates (and the negative control:
//! host-concatenated separate executions DO duplicate), the union fold
//! domain of aggregates, and params binding once for every rule.

use super::*;
use crate::ir::{AggOp, HeadTerm, ParamId};

/// Accounts 3 and 7 overlap on ("b", 25): the amounts of account 3 are
/// {10, 25}, of account 7 {25, 40}; account 9 exists so unfiltered rules
/// see more than the union under test.
fn overlap_postings() -> Vec<(u64, u64, &'static str, i64)> {
    vec![
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "b", 25),
        (4, 7, "c", 40),
        (5, 9, "d", 55),
    ]
}

/// One rule: `Posting(account = <account>, memo, amount), amount >= ?0`
/// — finds (memo, amount). Every rule references the one query-global
/// param.
fn by_account_rule(account: u64) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: POSTING,
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

/// Q(memo, amount) :- account 3's postings ∪ account 7's postings, both
/// under one `amount >= ?0` param.
fn union_query() -> Query {
    Query {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![by_account_rule(3), by_account_rule(7)],
    }
}

#[test]
fn a_multi_rule_program_prepares_with_every_rules_plan() {
    let dir = TempDir::new("prepared-rules-build");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let prepared = prepare(&txn, &cache, &schema, &union_query()).expect("multi-rule builds");
    assert_eq!(prepared.program.rules().len(), 2, "one plan per rule");
    for rule in prepared.program.rules() {
        // Each rule went through the full pipeline: a real plan with the
        // rule's own occurrence scratch exists.
        let PreparedRule::FreeJoin(rule) = rule else {
            panic!("fixture rules use Free Join");
        };
        assert_eq!(rule.resolved_filters.len(), 1, "one occurrence per rule");
    }
    assert_eq!(
        prepared
            .predicate()
            .columns
            .iter()
            .map(|column| &column.ty)
            .collect::<Vec<_>>(),
        vec![&ValueType::String, &ValueType::I64],
        "the head's result row types the program once"
    );
}

/// The union has no duplicates — and the negative control: the same two
/// rules as two separate single-rule executions, concatenated by the
/// host, DO duplicate. That difference is the proof the seen-set spans
/// rules (there is no merge pass that could have deduplicated instead).
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
        rows_of(&union),
        vec![
            ("a".to_owned(), 10),
            ("b".to_owned(), 25),
            ("c".to_owned(), 40),
        ],
        "the overlap ('b', 25) appears once: the union is a set"
    );

    // The negative control: per-rule executions concatenated in the
    // host carry the overlap twice.
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
        concatenated.extend(rows_of(&out));
    }
    concatenated.sort();
    assert_eq!(concatenated.len(), union.len() + 1, "one duplicate");
    assert_eq!(
        concatenated
            .iter()
            .filter(|row| **row == ("b".to_owned(), 25))
            .count(),
        2,
        "host concatenation is not a union"
    );
}

/// Params are query-global: one bind reaches every rule (the param slots
/// live on the prepared query, not on any rule).
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
        rows_of(&out),
        vec![("b".to_owned(), 25), ("c".to_owned(), 40)],
        "the floor filtered account 3's 10 AND account 7's nothing-below-20"
    );
    // Re-bind, same prepared query: both rules see the new value.
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::I64(30)])
        .expect("execute");
    assert_eq!(rows_of(&out), vec![("c".to_owned(), 40)]);
}

/// Aggregates over rules read the head (20-query-ir § aggregation): the
/// fold domain is the union of the rules' binding sets projected to the
/// head, so the overlapping ("b", 25) binding folds ONCE — where the
/// host-side sum of per-rule queries counts it twice. This is the naive
/// model's `union_fold` semantics, hand-computed.
#[test]
fn aggregates_fold_the_union_of_head_projected_bindings() {
    let dir = TempDir::new("prepared-rules-fold");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(Sum(amount), Count) :- rules over accounts 3 and 7.
    let agg_rule = |account: u64| Rule {
        finds: vec![
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(0)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        head: vec![
            HeadTerm::Aggregate(crate::ir::HeadOp::Sum),
            HeadTerm::Aggregate(crate::ir::HeadOp::Count),
        ],
        rules: vec![agg_rule(3), agg_rule(7)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    // Head projection per binding = (amount): {10, 25} ∪ {25, 40} =
    // {10, 25, 40}. Sum = 75 (the duplicate 25 folded once), Count = 3
    // — NOT the per-rule sums 35 + 65 = 100 / counts 2 + 2 = 4.
    assert_eq!(out.get(0, 0), ResultValue::I64(75), "Sum over the union");
    assert_eq!(out.get(0, 1), ResultValue::U64(3), "Count counts the union");
}

/// Grouped fold across rules: the duplicate head binding ("b", 25)
/// reaches its group exactly once, and each rule's exclusive groups
/// land untouched.
#[test]
fn a_grouped_fold_absorbs_the_cross_rule_duplicate() {
    let dir = TempDir::new("prepared-rules-groups");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(memo, Sum(amount)) :- account 3's ∪ account 7's postings.
    let rule = |account: u64| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: POSTING,
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
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(crate::ir::HeadOp::Sum)],
        rules: vec![rule(3), rule(7)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut rows: Vec<(String, i64)> = (0..out.len())
        .map(|row| {
            let ResultValue::String(memo) = out.get(row, 0) else {
                panic!("column 0 is a string");
            };
            let ResultValue::I64(sum) = out.get(row, 1) else {
                panic!("column 1 is an i64");
            };
            (memo.to_owned(), sum)
        })
        .collect();
    rows.sort();
    assert_eq!(
        rows,
        vec![
            ("a".to_owned(), 10),
            // Both rules derive ("b", 25); the union folds it once —
            // 25, never 50.
            ("b".to_owned(), 25),
            ("c".to_owned(), 40),
        ]
    );
}

/// The degenerate all-`Count` head over rules: every binding projects to
/// the empty head tuple, so the union has exactly one element and Count
/// is 1 — the naive model's constant-filler semantics, pinned so the
/// zero-arity dedup key stays representable.
#[test]
fn the_all_count_head_counts_the_singleton_union() {
    let dir = TempDir::new("prepared-rules-count");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let rule = |account: u64| Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        head: vec![HeadTerm::Aggregate(crate::ir::HeadOp::Count)],
        rules: vec![rule(3), rule(7)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::U64(1));
}

/// EXPLAIN over a program: per-rule node stats plus the head-level union
/// accounting — rule 1 re-derives the overlap and the report shows the
/// absorption.
#[test]
fn explain_reports_per_rule_stats_and_the_union_accounting() {
    let dir = TempDir::new("prepared-rules-explain");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &union_query()).expect("prepare");
    let (out, stats) = prepared
        .profile(&txn, &cache, &[BindValue::I64(0)])
        .expect("profile");
    assert_eq!(out.len(), 3, "the union");
    assert_eq!(stats.rules.len(), 2, "per-rule stats");
    assert_eq!(stats.emits, 4, "2 + 2 bindings reached the sink");
    assert_eq!(
        (stats.rules[0].emitted, stats.rules[0].absorbed),
        (2, 0),
        "rule 0 seeds the union"
    );
    assert_eq!(
        (stats.rules[1].emitted, stats.rules[1].absorbed),
        (2, 1),
        "rule 1 re-derives ('b', 25) and the spanning seen-set absorbs it"
    );
    for rule in &stats.rules {
        assert!(!rule.nodes.is_empty(), "per-rule node stats exist");
    }

    let (_, report) = prepared
        .explain(&txn, &cache, &[BindValue::I64(0)])
        .expect("explain");
    assert!(report.contains("rule 0:"), "{report}");
    assert!(report.contains("rule 1:"), "{report}");
    assert!(
        report.contains("emitted bindings: 2, absorbed by the union seen-set: 1"),
        "{report}"
    );
    assert!(
        report.contains("head union: 4 emitted across 2 rules, 1 absorbed"),
        "{report}"
    );
}

/// A guard-probe rule inside a program goes through the sink like any
/// other rule (the union must hear it): its re-derivation of another
/// rule's row is absorbed.
#[test]
fn a_guard_rule_unions_through_the_sink() {
    let dir = TempDir::new("prepared-rules-guard");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Rule 0: account 3's (memo, amount). Rule 1: the point lookup
    // `Posting(id = 2, memo, amount)` — a guard probe re-deriving
    // ("b", 25), which rule 0 already produced.
    let guard_rule = Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: POSTING,
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
    rule0.conditions.clear(); // no param: the guard rule binds none
    let query = Query {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![rule0, guard_rule],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.program.rules()[1], PreparedRule::Guard(_)),
        "rule 1 classifies as the point fast path"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        rows_of(&out),
        vec![("a".to_owned(), 10), ("b".to_owned(), 25)],
        "the guard's re-derivation is absorbed by the spanning seen-set"
    );
}

/// Arg-restriction across rules is refused at validation: the key is a
/// rule-scoped variable outside the head's vocabulary, so the union's
/// extreme is undefined (20-query-ir § aggregation).
#[test]
fn arg_restriction_across_rules_is_the_typed_validation_refusal() {
    let dir = TempDir::new("prepared-rules-arg");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &overlap_postings());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let arg_rule = |account: u64| Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::ArgMax { key: VarId(1) },
            over: Some(VarId(0)),
        }],
        atoms: vec![Atom {
            relation: POSTING,
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
        head: vec![HeadTerm::Aggregate(crate::ir::HeadOp::ArgMax)],
        rules: vec![arg_rule(3), arg_rule(7)],
    };
    let Err(err) = prepare(&txn, &cache, &schema, &query) else {
        panic!("Arg across rules must refuse at validation");
    };
    assert!(
        matches!(
            err,
            Error::Validation(crate::error::ValidationError::ArgAcrossRules { rules: 2 })
        ),
        "typed, named, counted: {err:?}"
    );
}
