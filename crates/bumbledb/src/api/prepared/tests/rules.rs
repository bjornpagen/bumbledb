//! The rules-shaped prepared query: a multi-rule program **builds** —
//! the whole plan pipeline runs per rule, so every rule's plan exists on
//! the prepared query — while executing 2+ rules is the typed
//! `Error::MultiRuleExecution` refusal (the union-driving loop is PRD
//! ALG-07's). The single-rule program stays the fully executable
//! degenerate case.

use super::*;
use crate::ir::HeadTerm;

/// A two-rule program over one head: postings of account ?0-like shape —
/// rule 0 selects account 3's amounts, rule 1 account 7's.
fn two_rule_query() -> Query {
    let by_account = |account: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(1), Term::Literal(Value::U64(account))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    Query {
        head: vec![HeadTerm::Var],
        rules: vec![by_account(3), by_account(7)],
    }
}

#[test]
fn a_multi_rule_program_prepares_with_every_rules_plan() {
    let dir = TempDir::new("prepared-rules-build");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 3, "a", 10), (2, 7, "b", 25)]);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    let prepared = prepare(&txn, &cache, &schema, &two_rule_query()).expect("multi-rule builds");
    assert_eq!(prepared.rules.len(), 2, "one plan per rule");
    for rule in &prepared.rules {
        // Each rule went through the full pipeline: a real plan with the
        // rule's own occurrence scratch exists.
        assert_eq!(rule.resolved_filters.len(), 1, "one occurrence per rule");
    }
    assert_eq!(
        prepared.column_types().collect::<Vec<_>>(),
        vec![&ValueType::I64],
        "the head's result row types the program once"
    );
}

#[test]
fn executing_two_rules_is_the_typed_refusal_not_a_wrong_answer() {
    let dir = TempDir::new("prepared-rules-gate");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 3, "a", 10), (2, 7, "b", 25)]);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    let mut prepared =
        prepare(&txn, &cache, &schema, &two_rule_query()).expect("multi-rule builds");
    let err = prepared
        .execute_collect(&txn, &cache, &[])
        .expect_err("execution of 2+ rules is gated until ALG 07");
    assert!(
        matches!(err, Error::MultiRuleExecution { rules: 2 }),
        "typed, named, counted: {err:?}"
    );

    // The single-rule degenerate case executes in full through the same
    // machinery.
    let single = Query::single(two_rule_query().rules.remove(0));
    let mut prepared = prepare(&txn, &cache, &schema, &single).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("single-rule executes");
    assert_eq!(amounts_of(&out), vec![10]);
}
