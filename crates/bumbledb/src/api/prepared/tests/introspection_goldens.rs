//! Byte-exact contract fixtures for introspection v2. These deliberately
//! exercise every plan-class/diagnostic family whose wording is public.

use super::*;
use crate::ir::{AggOp, HeadOp, HeadTerm};

const JOIN_WITH_CLOSED_FOLD: &str = r"introspection v2
query:
(v0, v2) | Reading(id: v0, kind: v1, value: v2), Kind(id: v1, rank == 20);
predicate: (u64, i64)
access path: free join (1 nodes)
  occurrence 0: relation 0 trie schema [[0, 1, 2]] (0 filters)
    estimated from (pinned rows at prepare): 5, filtered-view survivors 5
  occurrence 1: relation 1 trie schema [] (1 filters)
  folded: Kind{rank == 20} → {B, C}
  node 0:
    subatom 0: occ 0 vars [0, 1, 2] cover(true) chosen exact=0 estimate=1 probes hit=0 miss=0
    residuals: 0 placed, pass=0 fail=0
    anti-probes: 0 placed, probed=0 rejected=0
    estimated=5 actual=3 entries=1 skips=0
  distinct_bindings: proven
  emitted bindings: 3, absorbed by the union seen-set: 0
";

const STATICALLY_EMPTY: &str = r"introspection v2
query:
(v0, v2) | Reading(id: v0, kind: v1, value: v2), Kind(id: v1, rank == 99);
predicate: (u64, i64)
access path: statically empty
  distinct_bindings: unproven
  emitted bindings: 0, absorbed by the union seen-set: 0
statically empty: rule 0: folded to ∅: Kind{rank == 99}
";

const KEY_PROBE: &str = r"introspection v2
query:
(v0) | Posting(id == 1, amount: v0);
predicate: (i64)
access path: key probe
  relation: 0
  key statement: 0
  key fields: [0]
  remaining filters: 0
  distinct_bindings: proven
  emitted bindings: 1, absorbed by the union seen-set: 0
";

const AGGREGATE_UNION: &str = r"introspection v2
query:
(v0, Sum(v1)) | Posting(account == 3, memo: v0, amount: v1);
(v0, Sum(v1)) | Posting(account == 7, memo: v0, amount: v1);
predicate: (string, Sum i64)
rule 0:
access path: free join (1 nodes)
  occurrence 0: relation 0 trie schema [[0, 1]] (0 filters)
    estimated from (pinned rows at prepare): 3, filtered-view survivors 1
  node 0:
    subatom 0: occ 0 vars [0, 1] cover(true) chosen exact=0 estimate=1 probes hit=0 miss=0
    residuals: 0 placed, pass=0 fail=0
    anti-probes: 0 placed, probed=0 rejected=0
    estimated=1 actual=2 entries=1 skips=0
  distinct_bindings: unproven
  emitted bindings: 2, absorbed by the union seen-set: 0
rule 1:
access path: free join (1 nodes)
  occurrence 0: relation 0 trie schema [[0, 1]] (0 filters)
    estimated from (pinned rows at prepare): 3, filtered-view survivors 1
  node 0:
    subatom 0: occ 0 vars [0, 1] cover(true) chosen exact=0 estimate=1 probes hit=0 miss=0
    residuals: 0 placed, pass=0 fail=0
    anti-probes: 0 placed, probed=0 rejected=0
    estimated=1 actual=1 entries=1 skips=0
  distinct_bindings: unproven
  emitted bindings: 1, absorbed by the union seen-set: 1
head union: 3 emitted across 2 rules, 1 absorbed
disjoint_rules: unproven
";

const UNRESOLVED_LITERAL: &str = r#"introspection v2
query:
(v0) | Posting(memo == "z-unresolved", amount: v0), Posting(memo == "a-unresolved", amount: v0);
predicate: (i64)
pending literals: "z-unresolved", "a-unresolved" — an unresolved Eq literal empties its rule at execution until latched
access path: free join (2 nodes)
  occurrence 0: relation 0 trie schema [[0], []] (0 filters)
    estimated from (pinned rows at prepare): 1, filtered-view survivors 1
  occurrence 1: relation 0 trie schema [[0]] (0 filters)
    estimated from (pinned rows at prepare): 1, filtered-view survivors 1
  node 0:
    subatom 0: occ 1 vars [0] cover(true) chosen exact=0 estimate=0 probes hit=0 miss=0
    subatom 1: occ 0 vars [0] cover(true) chosen exact=0 estimate=0 probes hit=0 miss=0
    residuals: 0 placed, pass=0 fail=0
    anti-probes: 0 placed, probed=0 rejected=0
    estimated=1 actual=0 entries=0 skips=0
  node 1:
    subatom 0: occ 0 vars [] cover(true) chosen exact=0 estimate=0 probes hit=0 miss=0
    residuals: 0 placed, pass=0 fail=0
    anti-probes: 0 placed, probed=0 rejected=0
    estimated=1 actual=0 entries=0 skips=0
  distinct_bindings: unproven
  emitted bindings: 0, absorbed by the union seen-set: 0
"#;

fn assert_golden(
    prepared: &mut PreparedQuery<'_, ()>,
    txn: &crate::storage::env::ReadTxn<'_>,
    cache: &ImageCache,
    expected: &str,
) {
    let (_, first) = prepared
        .introspect(txn, cache, &[])
        .expect("first introspection");
    let (_, second) = prepared
        .introspect(txn, cache, &[])
        .expect("second introspection");
    assert_eq!(first, second, "identical input must be byte-identical");
    assert_eq!(first, expected);
}

#[test]
fn join_with_closed_fold_golden() {
    let dir = TempDir::new("introspection-golden-fold");
    let schema = super::folded::closed_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    super::folded::insert_readings(&env, &schema, super::folded::READINGS);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared =
        prepare(&txn, &cache, &schema, &super::folded::fold_query(20)).expect("prepare");
    assert_golden(&mut prepared, &txn, &cache, JOIN_WITH_CLOSED_FOLD);
}

#[test]
fn statically_empty_golden() {
    let dir = TempDir::new("introspection-golden-empty");
    let schema = super::folded::closed_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    super::folded::insert_readings(&env, &schema, super::folded::READINGS);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared =
        prepare(&txn, &cache, &schema, &super::folded::fold_query(99)).expect("prepare");
    assert_golden(&mut prepared, &txn, &cache, STATICALLY_EMPTY);
}

#[test]
fn key_probe_golden() {
    let dir = TempDir::new("introspection-golden-key-probe");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "rent", -1200)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(1))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert_golden(&mut prepared, &txn, &cache, KEY_PROBE);
}

#[test]
fn aggregate_union_golden_and_stats_parity() {
    let dir = TempDir::new("introspection-golden-aggregate-union");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[(1, 3, "a", 10), (2, 3, "b", 25), (3, 7, "b", 25)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let rule = |account| Rule {
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
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(HeadOp::Sum)],
        rules: vec![rule(3), rule(7)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let (_, display) = prepared
        .introspect(&txn, &cache, &[])
        .expect("introspection");
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(stats.introspection_version, 2);
    assert_eq!(display.matches("rule ").count(), stats.rules.len());
    assert_eq!(
        display.matches("  node ").count(),
        stats
            .rules
            .iter()
            .map(|rule| rule.nodes.len())
            .sum::<usize>()
    );
    assert_golden(&mut prepared, &txn, &cache, AGGREGATE_UNION);
}

#[test]
fn unresolved_literal_golden() {
    let dir = TempDir::new("introspection-golden-unresolved");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "alice", 10)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: POSTING,
                bindings: vec![
                    (
                        FieldId(2),
                        Term::Literal(Value::String(b"z-unresolved".as_slice().into())),
                    ),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            },
            Atom {
                relation: POSTING,
                bindings: vec![
                    (
                        FieldId(2),
                        Term::Literal(Value::String(b"a-unresolved".as_slice().into())),
                    ),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert_golden(&mut prepared, &txn, &cache, UNRESOLVED_LITERAL);
}
