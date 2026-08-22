use bumbledb::schema::{
    FieldId, RelationDescriptor, Row, SchemaDescriptor, Side, StatementDescriptor, ValueType,
};
use bumbledb::{Direction, RelationId, StatementId, Value};

use crate::fixture::{field, side};
use crate::naive::{Delta, NaiveDb, Tuple, Violation};

const SEVERITY: RelationId = RelationId(0);
const ALERT: RelationId = RelationId(1);
const ESCALATION: RelationId = RelationId(2);
const HANDLER: RelationId = RelationId(3);

const ALERT_SEVERITY: StatementId = StatementId(2);
const ESCALATION_SEVERITY: StatementId = StatementId(3);
const SEVERITY_HANDLED: StatementId = StatementId(4);

fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Low".into(),
                        values: Box::new([Value::Bool(false)]),
                    },
                    Row {
                        handle: "Med".into(),
                        values: Box::new([Value::Bool(true)]),
                    },
                    Row {
                        handle: "High".into(),
                        values: Box::new([Value::Bool(true)]),
                    },
                ])),
                name: "Severity".into(),
                fields: vec![field("pages", ValueType::Bool)],
            },
            RelationDescriptor {
                extension: None,
                name: "Alert".into(),
                fields: vec![field("severity", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Escalation".into(),
                fields: vec![field("severity", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Handler".into(),
                fields: vec![
                    field("severity", ValueType::U64),
                    field("priority", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HANDLER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(ALERT, &[0], &[]),
                target: side(SEVERITY, &[0], &[]),
            },
            StatementDescriptor::Containment {
                source: side(ESCALATION, &[0], &[]),
                target: Side {
                    relation: SEVERITY,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(
                        FieldId(1),
                        bumbledb::schema::LiteralSet::One(Value::Bool(true)),
                    )]),
                },
            },
            StatementDescriptor::Containment {
                source: side(SEVERITY, &[0], &[]),
                target: side(HANDLER, &[0], &[]),
            },
        ],
    }
}

fn insert(relation: RelationId, fact: Vec<Value>) -> Delta {
    Delta {
        deletes: vec![],
        inserts: vec![(relation, fact)],
    }
}

fn all_handlers() -> Delta {
    Delta {
        deletes: vec![],
        inserts: (0..3)
            .map(|severity| (HANDLER, vec![Value::U64(severity), Value::U64(10)]))
            .collect(),
    }
}

#[test]
fn the_extension_seeds_at_construction() {
    let db = NaiveDb::new(&schema());
    let rows: Vec<&Tuple> = db.relation(SEVERITY).iter().collect();
    assert_eq!(
        rows,
        vec![
            &Tuple(vec![Value::U64(0), Value::Bool(false)]),
            &Tuple(vec![Value::U64(1), Value::Bool(true)]),
            &Tuple(vec![Value::U64(2), Value::Bool(true)]),
        ],
        "declaration order, synthetic id first"
    );
    assert_eq!(db.generation(), 0, "seeding is not a commit");
}

/// Writes naming a closed relation are refused with the engine's typed verdict,
/// before anything applies — deletes first, then inserts, exactly the replay
/// order.
#[test]
fn closed_writes_are_refused_typed() {
    let mut db = NaiveDb::new(&schema());
    let before = db.clone();
    for delta in [
        insert(SEVERITY, vec![Value::U64(9), Value::Bool(false)]),
        Delta {
            deletes: vec![(SEVERITY, vec![Value::U64(1), Value::Bool(true)])],
            inserts: vec![],
        },
        // A mixed delta: the closed delete is refused even though the
        Delta {
            deletes: vec![(SEVERITY, vec![Value::U64(0), Value::Bool(false)])],
            inserts: vec![(ALERT, vec![Value::U64(300)])],
        },
    ] {
        assert_eq!(
            db.apply(&delta),
            Err(vec![Violation::ClosedRelationWrite { relation: SEVERITY }]),
        );
        assert_eq!(db, before, "a refusal must not apply");
    }
}

#[test]
fn the_psi_subset_judges_from_the_extension() {
    let descriptor = schema();
    let extension = descriptor.relations[0].extension.as_ref().expect("closed");
    let psi_rows: Vec<u64> = extension
        .iter()
        .enumerate()
        .filter(|(_, row)| row.values[0] == Value::Bool(true))
        .map(|(id, _)| id as u64)
        .collect();
    assert_eq!(psi_rows, vec![1, 2], "the hand-computed sub-vocabulary");

    for id in 0..4u64 {
        let mut db = NaiveDb::new(&descriptor);
        let verdict = db.apply(&insert(ESCALATION, vec![Value::U64(id)]));
        if psi_rows.contains(&id) {
            assert_eq!(verdict, Ok(()), "escalation {id} is inside ψ");
        } else {
            assert_eq!(
                verdict,
                Err(vec![Violation::Containment {
                    statement: ESCALATION_SEVERITY,
                    direction: Direction::SourceUnsatisfied,
                }]),
                "escalation {id} is outside ψ"
            );
        }
    }

    let mut db = NaiveDb::new(&descriptor);
    assert_eq!(
        db.apply(&insert(ESCALATION, vec![Value::U64(300)])),
        Err(vec![Violation::Containment {
            statement: ESCALATION_SEVERITY,
            direction: Direction::SourceUnsatisfied,
        }]),
    );

    for (id, expected) in [(2u64, true), (3u64, false)] {
        let mut db = NaiveDb::new(&descriptor);
        let verdict = db.apply(&insert(ALERT, vec![Value::U64(id)]));
        assert_eq!(
            verdict.is_ok(),
            expected,
            "alert {id} against the unselected vocabulary"
        );
        if let Err(violation) = verdict {
            assert_eq!(
                violation,
                vec![Violation::Containment {
                    statement: ALERT_SEVERITY,
                    direction: Direction::SourceUnsatisfied,
                }]
            );
        }
    }
}

#[test]
fn domain_quantification_judges_target_side() {
    let mut db = NaiveDb::new(&schema());
    db.apply(&all_handlers()).expect("the handlers land");
    let before = db.clone();

    assert_eq!(
        db.apply(&Delta {
            deletes: vec![(HANDLER, vec![Value::U64(2), Value::U64(10)])],
            inserts: vec![],
        }),
        Err(vec![Violation::Containment {
            statement: SEVERITY_HANDLED,
            direction: Direction::TargetRequired,
        }]),
    );
    assert_eq!(db, before, "the abort must not apply");

    db.apply(&Delta {
        deletes: vec![(HANDLER, vec![Value::U64(2), Value::U64(10)])],
        inserts: vec![(HANDLER, vec![Value::U64(2), Value::U64(99)])],
    })
    .expect("the severity-2 tuple re-lands in the same delta");
}

#[test]
fn complete_admission_rejects_unhandled_closed_source() {
    let mut db = NaiveDb::new(&schema());
    assert_eq!(
        db.judge_complete(),
        vec![Violation::Containment {
            statement: SEVERITY_HANDLED,
            direction: Direction::SourceUnsatisfied,
        }],
        "closed Low/Med/High have no handlers — L5 lifts the incremental fence"
    );
    db.apply(&all_handlers()).expect("the handlers land");
    assert!(
        db.judge_complete().is_empty(),
        "every sealed severity has a handler"
    );
}
