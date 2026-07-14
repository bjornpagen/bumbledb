//! The literal latch (PRD 09): the dictionary is append-only, so `str`
//! literal resolution is monotone — a hit rewrites the plan template
//! once, permanently; a miss stays live. With zero pending literals and
//! zero params, `resolve_filters` is skipped entirely (the
//! fully-latched fast path).

use super::*;

/// Q(amount) :- Posting(memo == <literal>, amount) — param-free, one str
/// literal, the latch's minimal habitat.
fn literal_query(memo: &str) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (
                    FieldId(2),
                    Term::Literal(Value::String(memo.as_bytes().into())),
                ),
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
    let dir = TempDir::new("latch-hit");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "alice", 10), (2, 7, "bob", 20)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &literal_query("alice")).expect("prepare");
    assert_eq!(prepared.unresolved_literals, 1, "counted at prepare");

    let mut out = Answers::new();
    prepared
        .execute(&txn, &cache, &[], &mut out)
        .expect("execute");
    assert_eq!(amounts(&out), vec![10]);
    assert_eq!(prepared.unresolved_literals, 0, "the hit latched");
    let [PreparedRule::FreeJoin(rule)] = prepared.program.rules() else {
        panic!("free join fixture");
    };
    assert!(rule.resolved_complete);

    // The latch IS the rewrite: the template slot now carries the word —
    // no parallel resolution state exists to consult.
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

    // Subsequent executions ride the fast path with identical results.
    prepared
        .execute(&txn, &cache, &[], &mut out)
        .expect("re-execute");
    assert_eq!(amounts(&out), vec![10]);
}

#[test]
fn a_miss_stays_live_and_latches_after_interning() {
    let dir = TempDir::new("latch-miss");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "alice", 10)]);
    let cache = ImageCache::new(&schema);

    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &literal_query("carol")).expect("prepare");
    let mut out = Answers::new();
    prepared
        .execute(&txn, &cache, &[], &mut out)
        .expect("execute");
    assert!(out.is_empty(), "an uninterned Eq literal empties the rule");
    assert_eq!(prepared.unresolved_literals, 1, "a miss never latches");
    assert!(
        matches!(
            prepared.program.rules(),
            [PreparedRule::FreeJoin(FreeJoinRule {
                resolved_complete: false,
                ..
            })]
        ),
        "a short-circuited pass does not arm the skip"
    );
    drop(txn);

    // Something interned it since: the miss becomes a hit — monotone,
    // one way, never the reverse.
    insert_postings(&env, &schema, &[(2, 8, "carol", 30)]);
    cache.evict_older_than(crate::GenerationId::from_storage(2));
    let txn = env.read_txn().expect("txn");
    prepared
        .execute(&txn, &cache, &[], &mut out)
        .expect("execute");
    assert_eq!(amounts(&out), vec![30]);
    assert_eq!(prepared.unresolved_literals, 0);

    // Third execution: the fast path, same answer.
    prepared
        .execute(&txn, &cache, &[], &mut out)
        .expect("execute");
    assert_eq!(amounts(&out), vec![30]);
}

/// The trace-level pins: the latch event fires exactly once per distinct
/// literal, and the fully-latched + param-free execution skips
/// `resolve_filters` provably (its span is absent) with results
/// identical to the slow path on the same snapshot.
#[cfg(feature = "trace")]
#[test]
fn the_latch_fires_once_and_the_fast_path_skips_resolution() {
    use crate::obs;

    let dir = TempDir::new("latch-trace");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "alice", 10), (2, 7, "bob", 20)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &literal_query("alice")).expect("prepare");
    let mut out = Answers::new();

    obs::start_capture();
    prepared
        .execute(&txn, &cache, &[], &mut out)
        .expect("execute");
    let events = obs::finish_capture();
    let slow = amounts(&out);
    assert_eq!(
        events
            .iter()
            .filter(|e| e.name == obs::names::LITERAL_LATCH)
            .count(),
        1,
        "one latch per distinct literal"
    );
    assert!(
        events.iter().any(|e| e.name == obs::names::RESOLVE_FILTERS),
        "the first execution resolves"
    );

    obs::start_capture();
    prepared
        .execute(&txn, &cache, &[], &mut out)
        .expect("execute");
    let events = obs::finish_capture();
    assert_eq!(amounts(&out), slow, "fast path, identical results");
    assert!(
        !events.iter().any(|e| e.name == obs::names::LITERAL_LATCH),
        "a latch fires once, ever"
    );
    assert!(
        !events.iter().any(|e| e.name == obs::names::RESOLVE_FILTERS),
        "resolve_filters provably skipped"
    );
}
