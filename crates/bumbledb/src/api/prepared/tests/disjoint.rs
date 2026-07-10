//! The exclusivity elision (docs/architecture/40-execution.md § set
//! semantics): rules that select different values of one discriminator
//! are provably disjoint, so the cross-rule seen-set guards nothing and
//! is deleted at plan time. What these tests pin: the DU-arm union
//! proves (and an unselected arm unproves), EXPLAIN names the witness,
//! the aggregate composition runs with ZERO seen-set insertions, and the
//! differential guard — elided vs forced-off byte-identical across a
//! randomized rule-query corpus — because the elision is never semantic.

use super::*;
use crate::ir::{AggOp, HeadOp, HeadTerm};

/// Item(id fresh u64, kind enum{note, event, task}, payload u64) — the
/// discriminated-union parent shape; the fresh id materializes the
/// auto-key whose columns the arms' heads carry.
fn du_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Item".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::Enum {
                        variants: vec!["note".into(), "event".into(), "task".into()]
                            .into_boxed_slice(),
                    },
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
                ValueRef::Enum(*kind),
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
            relation: ITEM,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Literal(Value::Enum(kind))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    }
}

fn du_query(rules: Vec<Rule>) -> Query {
    Query {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules,
    }
}

/// Every result row, debug-rendered and sorted — the differential
/// guard's comparison surface (results are sets; order is not part of
/// the answer).
fn table(out: &ResultBuffer, arity: usize) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = (0..out.len())
        .map(|row| {
            (0..arity)
                .map(|column| format!("{:?}", out.get(row, column)))
                .collect()
        })
        .collect();
    rows.sort();
    rows
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
    let cache = ImageCache::new();
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
    same_kind.predicates = vec![PredicateTree::Leaf(Comparison {
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

/// EXPLAIN names the proof — `disjoint_rules: proven (Item.kind)` — and
/// the structured stats carry the same witness; the unproven program
/// says so.
#[test]
fn explain_names_the_disjointness_witness() {
    let dir = TempDir::new("prepared-disjoint-explain");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(
        &txn,
        &cache,
        &schema,
        &du_query(vec![arm_rule(0), arm_rule(1)]),
    )
    .expect("prepare");
    let (out, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
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

    let mut open = arm_rule(1);
    open.atoms[0].bindings.remove(1);
    let mut unproven =
        prepare(&txn, &cache, &schema, &du_query(vec![arm_rule(0), open])).expect("prepare");
    let (_, report) = unproven.explain(&txn, &cache, &[]).expect("explain");
    assert!(report.contains("disjoint_rules: unproven"), "{report}");
}

/// The aggregate composition: `Count` over a proven-disjoint union with
/// per-rule key-covered bindings runs with ZERO seen-set insertions —
/// the fold seen-set is elided outright (the counting surface holds
/// nothing, before and after the execution) — and matches the naive
/// model: the fold domain is the union of head projections, one `(id)`
/// per item of the selected kinds.
#[test]
fn count_over_a_proven_disjoint_union_elides_the_fold_seen_set() {
    let dir = TempDir::new("prepared-disjoint-count");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    // Q(id, Count) :- one rule per kind; the rule binds ONLY the key
    // variable (kind is pinned), so bindings are key-covered and the
    // head reads every slot.
    let rule = |kind: u8| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![Atom {
            relation: ITEM,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Literal(Value::Enum(kind))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let query = Query {
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(HeadOp::Count)],
        rules: vec![rule(0), rule(1)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(prepared.disjoint_rules(), "the arms prove disjoint");
    assert!(
        prepared.distinct_bindings(),
        "the composition elides the union seen-set"
    );
    let EitherSink::Aggregate(sink) = &prepared.sink else {
        panic!("Count builds the aggregate sink");
    };
    assert!(sink.seen_elided(), "no seen-set exists to insert into");

    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    // The counting surface: an elided seen-set holds nothing — zero
    // insertions happened because no set exists (`None`, not `Some(0)`).
    assert_eq!(
        prepared.sink.distinct_seen(),
        None,
        "zero seen-set insertions, structurally"
    );
    // The naive model: fold domain = ∪ head-projected bindings; per
    // group (id) the projection is the singleton (id), so Count = 1 for
    // each note/event item and the task never appears.
    let mut rows: Vec<(u64, u64)> = (0..out.len())
        .map(|row| {
            let (ResultValue::U64(id), ResultValue::U64(count)) =
                (out.get(row, 0), out.get(row, 1))
            else {
                panic!("U64 columns");
            };
            (id, count)
        })
        .collect();
    rows.sort_unstable();
    assert_eq!(rows, vec![(1, 1), (2, 1), (3, 1), (4, 1)]);

    // The differential guard on this exact shape: forced off, the
    // spanning seen-set returns and the rows do not move.
    let mut forced = prepare(&txn, &cache, &schema, &query).expect("prepare");
    forced.force_disjoint_off();
    let EitherSink::Aggregate(sink) = &forced.sink else {
        panic!("Count builds the aggregate sink");
    };
    assert!(!sink.seen_elided(), "the override reinstated the seen-set");
    let control = forced.execute_collect(&txn, &cache, &[]).expect("execute");
    assert_eq!(table(&out, 2), table(&control, 2));
}

/// A deterministic LCG (the house randomized-test shape).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

/// The differential guard across the randomized rule-query corpus:
/// elided vs forced-off byte-identical on every covered query. The
/// corpus draws multi-rule programs over the DU shape — arms with
/// selected, repeated, or missing discriminators (proven AND unproven
/// programs, both asserted present), projection and aggregate heads,
/// heads that do and do not carry the key — and every one must render
/// the same table under both regimes, because the elision is never
/// semantic.
#[test]
fn the_randomized_corpus_is_byte_identical_elided_vs_forced_off() {
    let dir = TempDir::new("prepared-disjoint-corpus");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Duplicated payloads across kinds and within a kind: cross-rule
    // and within-rule head collisions are live wherever the proof does
    // not forbid them.
    let mut rows = Vec::new();
    let mut lcg = Lcg(0xB0B5_CAFE);
    for id in 1..=40u64 {
        rows.push((id, (lcg.next() % 3) as u8, 10 + lcg.next() % 4));
    }
    insert_items(&env, &schema, &rows);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    // One corpus rule: bind the key (or not), select a kind (or not),
    // and shape the head. Kind 3 = no selection.
    let body = |bind_id: bool, kind: u8, vars: &[u16]| -> Atom {
        let mut bindings = Vec::new();
        if bind_id {
            bindings.push((FieldId(0), Term::Var(VarId(vars[0]))));
        }
        if kind < 3 {
            bindings.push((FieldId(1), Term::Literal(Value::Enum(kind))));
        }
        bindings.push((FieldId(2), Term::Var(VarId(vars[1]))));
        Atom {
            relation: ITEM,
            bindings,
        }
    };

    let mut proven = 0usize;
    let mut unproven = 0usize;
    for seed in 0..48u64 {
        let mut lcg = Lcg(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) + 1);
        let head_shape = lcg.next() % 3;
        let rule_count = 2 + (lcg.next() % 2) as usize;
        let (head, finds): (Vec<HeadTerm>, Vec<FindTerm>) = match head_shape {
            // Q(id, payload) — the DU read; the key flows to the head.
            0 => (
                vec![HeadTerm::Var, HeadTerm::Var],
                vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            ),
            // Q(payload) — the key never reaches the head: unprovable.
            1 => (vec![HeadTerm::Var], vec![FindTerm::Var(VarId(1))]),
            // Q(id, Sum(payload)) — the aggregate composition.
            _ => (
                vec![HeadTerm::Var, HeadTerm::Aggregate(HeadOp::Sum)],
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Sum,
                        over: Some(VarId(1)),
                    },
                ],
            ),
        };
        let rules: Vec<Rule> = (0..rule_count)
            .map(|_| Rule {
                finds: finds.clone(),
                atoms: vec![body(head_shape != 1, (lcg.next() % 4) as u8, &[0, 1])],
                negated: vec![],
                predicates: vec![],
            })
            .collect();
        let query = Query { head, rules };
        let arity = query.head.len();

        let mut elided = prepare(&txn, &cache, &schema, &query).expect("prepare");
        if elided.disjoint_rules() {
            proven += 1;
        } else {
            unproven += 1;
        }
        let mut forced = prepare(&txn, &cache, &schema, &query).expect("prepare");
        forced.force_disjoint_off();

        let fast = elided.execute_collect(&txn, &cache, &[]).expect("execute");
        let slow = forced.execute_collect(&txn, &cache, &[]).expect("execute");
        assert_eq!(
            table(&fast, arity),
            table(&slow, arity),
            "seed {seed}: the elision changed the answer"
        );
    }
    // The corpus guards its own vacuousness: both regimes must occur.
    assert!(proven > 0, "no seed proved disjoint — the corpus is dead");
    assert!(
        unproven > 0,
        "every seed proved — the negative space is dead"
    );
}

/// The projection sink under the proof: a three-arm union (two drain
/// boundaries) returns every item exactly once, and per-rule dedup
/// still absorbs a within-rule duplicate (a payload-only head over one
/// kind can repeat).
#[test]
fn a_three_arm_union_survives_the_per_rule_drains() {
    let dir = TempDir::new("prepared-disjoint-drain");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    let query = du_query(vec![arm_rule(0), arm_rule(1), arm_rule(2)]);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        prepared.disjoint_rules(),
        "all three pairs share the witness"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut rows: Vec<(u64, u64)> = (0..out.len())
        .map(|row| {
            let (ResultValue::U64(id), ResultValue::U64(payload)) =
                (out.get(row, 0), out.get(row, 1))
            else {
                panic!("U64 columns");
            };
            (id, payload)
        })
        .collect();
    rows.sort_unstable();
    assert_eq!(
        rows,
        vec![(1, 10), (2, 20), (3, 20), (4, 40), (5, 50)],
        "the whole union, exactly once each — including the equal payloads"
    );
}
