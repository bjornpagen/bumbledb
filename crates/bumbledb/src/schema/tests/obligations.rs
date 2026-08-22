use super::*;
use crate::schema::{CompleteObligation, Pairing};

fn closed_source_ordinary_target() -> Schema {
    SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![],
                vec![row("Soft", vec![]), row("Hard", vec![])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Bucket".into(),
                fields: vec![field("id", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("closed source against an ordinary target validates")
}

#[test]
fn complete_roster_skips_closed_constant_and_keeps_instance_dependent() {
    let schema = closed_source_ordinary_target();

    // closed→ordinary containment are instance-dependent.
    let roster: Vec<_> = schema.complete_obligations().iter().collect();
    assert_eq!(roster.len(), 2, "{roster:?}");
    assert!(matches!(
        roster.as_slice(),
        [
            CompleteObligation::Key { .. },
            CompleteObligation::Containment { .. },
        ]
    ));
    assert_eq!(roster[0].statement_ref(), StatementRef::Key(KeyId(1)));
    assert_eq!(roster[0].statement_id(), StatementId(1));
    assert_eq!(
        roster[1].statement_ref(),
        StatementRef::Containment(ContainmentId(0))
    );
    assert_eq!(roster[1].statement_id(), StatementId(2));
    let closed_key = schema.statement(StatementId(0));
    assert!(
        schema.closed_constant(closed_key),
        "closed functionality is validation-discharged"
    );
}

#[test]
fn closed_to_closed_containment_is_not_a_complete_obligation() {
    let schema = SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![field("severity", ValueType::U64)],
                vec![
                    row("Soft", vec![Value::U64(0)]),
                    row("Hard", vec![Value::U64(1)]),
                ],
            ),
            closed(
                "Severity",
                vec![],
                vec![row("Low", vec![]), row("High", vec![])],
            ),
        ],
        statements: vec![containment(
            side(RelationId(0), &[FieldId(1)]),
            side(RelationId(1), &[FieldId(0)]),
        )],
    }
    .validate()
    .expect("satisfied closed-to-closed validates");
    assert!(
        schema.complete_obligations().iter().next().is_none(),
        "closed-constant containments are validation-discharged"
    );
}

#[test]
fn equality_pair_seals_typed_containment_ids() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "P".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Q".into(),
                fields: vec![field("pid", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
            containment(
                side(RelationId(1), &[FieldId(0)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("== pair validates");
    assert_eq!(
        schema.containment(ContainmentId(0)).pairing,
        Pairing::Mirror(ContainmentId(1))
    );
    assert_eq!(
        schema.containment(ContainmentId(1)).pairing,
        Pairing::Mirror(ContainmentId(0))
    );
    assert_eq!(
        schema.containment(ContainmentId(0)).mirror_id(&schema),
        Some(StatementId(3))
    );
}
