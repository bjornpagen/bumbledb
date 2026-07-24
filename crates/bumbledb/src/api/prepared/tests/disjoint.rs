//! The rule-disjointness proof (docs/architecture/40-execution.md § set
//! semantics): rules selecting different values of one discriminator are
//! provably disjoint. The proof remains visible in introspection, while execution
//! deliberately keeps one seen-set spanning the rules. These tests pin the
//! proof and show that the spanning set absorbs nothing across proven arms.

use super::*;
use crate::ir::{AggOp, HeadOp, HeadTerm};

/// Item(id fresh u64, kind u64 — 0 note, 1 event, 2 task, payload u64) —
/// the discriminated-union parent shape; the fresh id materializes the
/// auto-key whose columns the arms' heads carry.
fn du_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Item".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "payload".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const ITEM: RelationId = RelationId(0);

fn insert_items(env: &Environment, schema: &Schema, rows: &[(u64, u8, u64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, kind, payload) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(u64::from(*kind)),
                ValueRef::U64(*payload),
            ],
            schema.relation(ITEM).layout(),
            &mut bytes,
        );
        delta.insert(&view, ITEM, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// Two notes, two events, one task — the task is the negative space
/// every two-arm union must exclude.
fn item_rows() -> Vec<(u64, u8, u64)> {
    vec![(1, 0, 10), (2, 0, 20), (3, 1, 20), (4, 1, 40), (5, 2, 50)]
}

/// One DU arm: `Item(id, kind = <kind>, payload)` — finds (id, payload).
fn arm_rule(kind: u8) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(ITEM),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Literal(Value::U64(u64::from(kind)))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    }
}

fn du_query(rules: Vec<Rule>) -> Query {
    Query {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules,
    }
}

/// The DU-arm union (two arms, `kind`-selected) proves disjoint;
/// removing one rule's selection unproves it — the pair has no witness,
/// so the flag conservatively stays off.
#[test]
fn the_du_arm_union_proves_and_an_unselected_arm_unproves() {
    let dir = TempDir::new("prepared-disjoint-proof");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let proven = prepare(
        &txn,
        &cache,
        &schema,
        &du_query(vec![arm_rule(0), arm_rule(1)]),
    )
    .expect("prepare");
    assert!(
        proven.disjoint_rules(),
        "different kind literals over the id-keyed occurrence prove the pair"
    );

    // The same query with rule 1's selection removed: the kind is open,
    // so nothing pins the pair apart.
    let mut open = arm_rule(1);
    open.atoms[0].bindings.remove(1);
    let unproven =
        prepare(&txn, &cache, &schema, &du_query(vec![arm_rule(0), open])).expect("prepare");
    assert!(!unproven.disjoint_rules(), "no witness, no proof");

    // Equal literals pin the pair TOGETHER, not apart (the rules differ
    // structurally so DNF dedup keeps both — the pair really is judged).
    let mut same_kind = arm_rule(0);
    same_kind.conditions = vec![ConditionTree::Leaf(Comparison {
        op: CmpOp::Ge,
        lhs: Term::Var(VarId(1)),
        rhs: Term::Literal(Value::U64(0)),
    })];
    let same = prepare(
        &txn,
        &cache,
        &schema,
        &du_query(vec![arm_rule(0), same_kind]),
    )
    .expect("prepare");
    assert!(!same.disjoint_rules(), "equal literals are not different");
}

/// introspection names the proof — `disjoint_rules: proven (Item.kind)` — and
/// the structured stats carry the same witness; the unproven program
/// says so.
#[test]
fn introspection_names_the_disjointness_witness() {
    let dir = TempDir::new("prepared-disjoint-introspect");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(
        &txn,
        &cache,
        &schema,
        &du_query(vec![arm_rule(0), arm_rule(1)]),
    )
    .expect("prepare");
    let (out, report) = prepared.introspect(&txn, &cache, &[]).expect("introspect");
    assert_eq!(out.len(), 4, "both arms, the task excluded");
    assert!(
        report.contains("disjoint_rules: proven (Item.kind)"),
        "{report}"
    );
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(
        stats.disjoint_rules,
        Some(crate::api::stats::DisjointRules {
            relation: "Item".to_owned(),
            field: "kind".to_owned(),
        }),
    );

    // The open arm reads `kind` as a variable: no pinned literal, so
    // the pair is unproven — and the extra variable position keeps the
    // bodies structurally distinct, out of subsumption's witness (a
    // plainly kind-free arm would contain the pinned one and delete it,
    // leaving no pair to report).
    let mut open = arm_rule(1);
    open.atoms[0].bindings[1] = (FieldId(1), Term::Var(VarId(2)));
    let mut unproven =
        prepare(&txn, &cache, &schema, &du_query(vec![arm_rule(0), open])).expect("prepare");
    let (_, report) = unproven.introspect(&txn, &cache, &[]).expect("introspect");
    assert!(report.contains("disjoint_rules: unproven"), "{report}");
}

/// A fold over a proven-disjoint union retains the spanning seen-set,
/// which absorbs zero answers because the theorem is true, and matches
/// the naive model: per id, the sum of its head-projected payloads. The
/// fold-free nullary `Count` on this shape is refused instead (R1,
/// pinned below) — the disjointness proof cannot make a constant
/// informative.
#[test]
fn a_fold_over_a_proven_disjoint_union_absorbs_nothing() {
    let dir = TempDir::new("prepared-disjoint-count");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(id, Sum(payload)) :- one rule per kind; the rules bind the key
    // variable (kind is pinned) plus the fold input, so bindings are
    // key-covered and the head reads every slot.
    let rule = |kind: u8| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(ITEM),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Literal(Value::U64(u64::from(kind)))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(HeadOp::Sum)],
        rules: vec![rule(0), rule(1)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(prepared.disjoint_rules(), "the arms prove disjoint");
    assert!(!prepared.distinct_bindings(), "unions always retain dedup");
    let EitherSink::Aggregate(sink) = &prepared.sink else {
        panic!("Sum builds the aggregate sink");
    };
    assert!(!sink.seen_elided(), "the spanning seen-set exists");

    let (out, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(
        prepared.sink.distinct_seen(),
        Some(4),
        "all four head projections inhabit the spanning set"
    );
    assert!(stats.rules.iter().all(|rule| rule.absorbed == 0));
    // The naive model: fold domain = ∪ head-projected bindings; per
    // group (id) the projection is the singleton (id, payload), so the
    // Sum is the payload and the kind-2 item never appears.
    let mut answers: Vec<(u64, u64)> = (0..out.len())
        .map(|answer| {
            let (AnswerValue::U64(id), AnswerValue::U64(sum)) =
                (out.get(answer, 0), out.get(answer, 1))
            else {
                panic!("U64 columns");
            };
            (id, sum)
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(answers, vec![(1, 10), (2, 20), (3, 20), (4, 40)]);

    // The R1 corollary: the same proven-disjoint shape under a
    // fold-free nullary Count refuses — provable disjointness is
    // diagnostic knowledge, never a semantics.
    let count_rule = |kind: u8| {
        let mut rule = rule(kind);
        rule.finds[1] = FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        };
        rule
    };
    let refused = Query {
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(HeadOp::Count)],
        rules: vec![count_rule(0), count_rule(1)],
    };
    let Err(err) = prepare(&txn, &cache, &schema, &refused) else {
        panic!("fold-free nullary Count across written rules refuses");
    };
    assert!(
        matches!(
            err,
            Error::Validation(crate::error::ValidationError::CountAcrossRules { rules: 2 })
        ),
        "typed, named, counted: {err:?}"
    );
}

/// The projection sink under the proof: a three-arm union returns every
/// item exactly once and the spanning set absorbs nothing across arms.
#[test]
fn a_three_arm_union_absorbs_nothing_across_rules() {
    let dir = TempDir::new("prepared-disjoint-spanning");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = du_query(vec![arm_rule(0), arm_rule(1), arm_rule(2)]);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        prepared.disjoint_rules(),
        "all three pairs share the witness"
    );
    let (out, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert!(stats.rules.iter().all(|rule| rule.absorbed == 0));
    let mut answers: Vec<(u64, u64)> = (0..out.len())
        .map(|answer| {
            let (AnswerValue::U64(id), AnswerValue::U64(payload)) =
                (out.get(answer, 0), out.get(answer, 1))
            else {
                panic!("U64 columns");
            };
            (id, payload)
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(
        answers,
        vec![(1, 10), (2, 20), (3, 20), (4, 40), (5, 50)],
        "the whole union, exactly once each — including the equal payloads"
    );
}
