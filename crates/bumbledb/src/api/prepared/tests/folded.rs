use super::*;
use bumbledb_theory::schema::Row;

pub(super) fn closed_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Reading".into(),
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
                        name: "value".into(),
                        value_type: ValueType::I64,
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
                }],
            },
        ],
        statements: vec![bumbledb_theory::schema::StatementDescriptor::Containment {
            source: bumbledb_theory::schema::Side {
                relation: RelationId(0),
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: bumbledb_theory::schema::Side {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
}

const READING: RelationId = RelationId(0);
const KIND: RelationId = RelationId(1);

pub(super) fn readings(rows: &[(u64, u64, i64)]) -> Fix {
    let facts: Vec<Vec<Value>> = rows
        .iter()
        .map(|(id, kind, value)| vec![Value::U64(*id), Value::U64(*kind), Value::I64(*value)])
        .collect();
    Fix::heap(closed_descriptor(), &[(READING, facts)])
}

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

pub(super) const READINGS: &[(u64, u64, i64)] = &[
    (1, 0, 100),
    (2, 1, 210),
    (3, 1, 211),
    (4, 2, 220),
    (5, 3, 300),
];

#[test]
fn a_folded_plan_answers_and_keeps_the_latched_fast_path() {
    let fix = readings(READINGS);
    let mut prepared = fix.prepare(&fold_query(20)).expect("prepare");
    assert_eq!(
        prepared.latch.remaining(),
        0,
        "a plan-constant set is pre-resolved — it must not block the latch"
    );
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(
        values_of(&out),
        vec![210, 211, 220],
        "kinds 1 and 2 (rank 20)"
    );

    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("warm execute");
    assert_eq!(values_of(&out), vec![210, 211, 220]);
}

#[cfg(feature = "trace")]
#[test]
fn a_folded_occurrence_builds_no_image_and_binds_no_view() {
    use crate::obs;

    let fix = readings(READINGS);
    let mut prepared = fix.prepare(&fold_query(20)).expect("prepare");
    obs::start_capture();
    fix.execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let events = obs::finish_capture();
    let count = |name| events.iter().filter(|e| e.point() == name).count();
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

#[test]
fn introspection_reports_the_fold_with_its_filters_and_handles() {
    let fix = readings(READINGS);
    let store = StoreFix::store("folded-introspect", closed_descriptor());
    let mut prepared = store.prepare(&fold_query(20)).expect("prepare");
    let (_, report) = store
        .db
        .read(|instance| prepared.introspect(instance, &[]))
        .expect("introspect");
    assert!(report.contains("query:"), "{report}");
    drop(fix);
}

#[test]
fn an_empty_fold_prepares_the_statically_empty_query() {
    let fix = readings(READINGS);
    let mut prepared = fix.prepare(&fold_query(99)).expect("prepare");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::Cq { ref rules, .. } if rules.is_empty()),
        "no Kind row has rank 99: the rule died at prepare"
    );
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 0);
}
