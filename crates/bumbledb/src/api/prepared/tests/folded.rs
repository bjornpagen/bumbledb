//! The grounding-evaluator's execution shape (docs/architecture/
//! 40-execution.md, § the ground: elimination and evaluation): a folded
//! occurrence never builds an image or binds a view, its plan-constant
//! set rides the param-set selection machinery (and never counts as an
//! unresolved literal — the PRD 09 latch), introspection carries the fold
//! line, and the |S| == 0 verdict prepares to the statically-empty
//! plan.

use super::*;
use crate::schema::Row;

/// Reading(id fresh, kind u64, value i64) referencing the closed
/// Kind(rank u64; ranks 10/20/20/30) through Reading(kind) <= Kind(id).
pub(super) fn closed_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Reading".into(),
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
                        name: "value".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "A".into(),
                        values: Box::new([Value::U64(10)]),
                    },
                    Row {
                        handle: "B".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "C".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "D".into(),
                        values: Box::new([Value::U64(30)]),
                    },
                ])),
                name: "Kind".into(),
                fields: vec![FieldDescriptor {
                    name: "rank".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                }],
            },
        ],
        statements: vec![crate::schema::StatementDescriptor::Containment {
            source: crate::schema::Side {
                relation: RelationId(0),
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: crate::schema::Side {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
    .validate()
    .expect("valid fixture")
}

const READING: RelationId = RelationId(0);
const KIND: RelationId = RelationId(1);

pub(super) fn insert_readings(env: &Environment, schema: &Schema, rows: &[(u64, u64, i64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, kind, value) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*kind),
                ValueRef::I64(*value),
            ],
            schema.relation(READING).layout(),
            &mut bytes,
        );
        delta.insert(&view, READING, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// `Q(id, value) :- Reading(id, kind = x, value), Kind(id = x, rank == <rank>)`.
pub(super) fn fold_query(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(READING),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                    (FieldId(2), Term::Var(VarId(2))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(KIND),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Literal(Value::U64(rank))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn values_of(buffer: &Answers) -> Vec<i64> {
    let mut values: Vec<i64> = (0..buffer.len())
        .map(|answer| {
            let AnswerValue::I64(value) = buffer.get(answer, 1) else {
                panic!("column 1 is an i64");
            };
            value
        })
        .collect();
    values.sort_unstable();
    values
}

/// The readings fixture: kinds 0..=3, values tagged by kind.
pub(super) const READINGS: &[(u64, u64, i64)] = &[
    (1, 0, 100),
    (2, 1, 210),
    (3, 1, 211),
    (4, 2, 220),
    (5, 3, 300),
];

/// The fold executes correctly and its plan-constant set never counts
/// as an unresolved literal — the fully-latched fast path stays open
/// (zero pending literals, zero params ⇒ `resolve_filters` is
/// skipped from the second execution on).
#[test]
fn a_folded_plan_answers_and_keeps_the_latched_fast_path() {
    let dir = TempDir::new("folded-answers");
    let schema = closed_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_readings(&env, &schema, READINGS);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &fold_query(20)).expect("prepare");
    assert_eq!(
        prepared.unresolved_literals, 0,
        "a plan-constant set is pre-resolved — it must not block the latch"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        values_of(&out),
        vec![210, 211, 220],
        "kinds 1 and 2 (rank 20)"
    );
    // Warm re-execution rides the fully-latched fast path (the resolved
    // tables are final) and answers identically.
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("warm execute");
    assert_eq!(values_of(&out), vec![210, 211, 220]);
}

/// The `[shape]` leg: the folded occurrence never builds an image and
/// never binds a view — exactly one `VIEW_BUILD` (the Reading occurrence)
/// and one `IMAGE_BUILD` (Reading's) appear; the closed relation's
/// synthesized image is never touched.
#[cfg(feature = "trace")]
#[test]
fn a_folded_occurrence_builds_no_image_and_binds_no_view() {
    use crate::obs;

    let dir = TempDir::new("folded-no-images");
    let schema = closed_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_readings(&env, &schema, READINGS);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &fold_query(20)).expect("prepare");
    obs::start_capture();
    prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let events = obs::finish_capture();
    let count = |name: &str| events.iter().filter(|e| e.name == name).count();
    assert_eq!(
        count(obs::names::VIEW_BUILD),
        1,
        "one view binds: the Reading occurrence — never the folded Kind"
    );
    assert_eq!(
        count(obs::names::IMAGE_BUILD),
        1,
        "one image builds: Reading's — the sealed extension was read at prepare"
    );
}

/// introspection carries the fold line (the Eliminated-reporting precedent),
/// and the structured stats mirror it — the surviving set as handles,
/// the vocabulary's names (the handle set IS the payload).
#[test]
fn introspection_reports_the_fold_with_its_filters_and_handles() {
    let dir = TempDir::new("folded-introspect");
    let schema = closed_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_readings(&env, &schema, READINGS);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &fold_query(20)).expect("prepare");
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(stats.rules.len(), 1);
    let folded = &stats.rules[0].folded;
    assert_eq!(folded.len(), 1);
    assert_eq!(folded[0].relation, "Kind");
    assert_eq!(folded[0].rendered, "Kind{rank == 20}");
    assert_eq!(folded[0].handles, vec!["B".to_owned(), "C".to_owned()]);
    assert!(!folded[0].negated);
    let (_, report) = prepared.introspect(&txn, &cache, &[]).expect("introspect");
    assert!(
        report.contains("folded: Kind{rank == 20} → {B, C}"),
        "{report}"
    );
}

/// |S| == 0 is the statically-empty channel: the rule dies at prepare
/// with the evaluator's rendered reason, and an all-dead program
/// prepares to the empty program.
#[test]
fn an_empty_fold_prepares_the_statically_empty_program() {
    let dir = TempDir::new("folded-empty");
    let schema = closed_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_readings(&env, &schema, READINGS);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let mut prepared = prepare(&txn, &cache, &schema, &fold_query(99)).expect("prepare");
    assert!(
        matches!(prepared.program, Program::Empty),
        "no Kind row has rank 99: the rule died at prepare"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 0);
    let (_, report) = prepared.introspect(&txn, &cache, &[]).expect("introspect");
    assert!(
        report.contains("statically empty: rule 0: folded to ∅: Kind{rank == 99}"),
        "{report}"
    );
}
