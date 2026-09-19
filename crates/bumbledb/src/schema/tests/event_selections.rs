use super::*;
use crate::error::{SchemaError, StatementErrorKind};
use crate::event::{
    ArithmeticLimits, Control, DensityPiece, Error as EventError, ExactArithmetic, ExactRational,
    LawLimits, Limits, Space, SpaceId,
};
use crate::schema::judge::{JudgeBudget, Judgment, MapState, judge_complete};
use crate::{Event, WorkContext};

fn space(reverse: bool) -> Space {
    Space::with_order(
        SpaceId([119; 32]),
        if reverse { &[1, 0] } else { &[0, 1] },
        Limits::default(),
        &(),
    )
    .unwrap()
}
fn selected(relation: u32, values: &[Event]) -> Side {
    side_where_sets(
        RelationId(relation),
        &[FieldId(0)],
        vec![(
            FieldId(1),
            if values.len() == 1 {
                LiteralSet::One(Value::Event(values[0].clone()))
            } else {
                LiteralSet::Many(values.iter().cloned().map(Value::Event).collect())
            },
        )],
    )
}
fn descriptor(values: &[Event]) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: ["Child", "Parent"]
            .iter()
            .map(|name| RelationDescriptor {
                name: (*name).into(),
                extension: None,
                fields: vec![
                    field("group", ValueType::U64),
                    field("condition", ValueType::Event),
                ],
            })
            .collect(),
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            containment(selected(0, values), selected(1, values)),
        ],
    }
}
fn judge(schema: &Schema, source: &Event, target: Option<&Event>) -> Judgment {
    let mut state = MapState::new();
    state
        .insert(
            RelationId(0),
            vec![Value::U64(1), Value::Event(source.clone())],
        )
        .unwrap();
    if let Some(target) = target {
        state
            .insert(
                RelationId(1),
                vec![Value::U64(1), Value::Event(target.clone())],
            )
            .unwrap();
    }
    judge_complete(schema, &state, &WorkContext::new(), JudgeBudget::default()).unwrap()
}

#[test]
fn event_selection_identity_is_portable_complete_value_equality() {
    let a = space(false).coordinate(0, &()).unwrap();
    let b = space(true).coordinate(0, &()).unwrap();
    assert_ne!(a, b, "resident handles stay owner-scoped");
    let schema = descriptor(std::slice::from_ref(&a)).validate().unwrap();
    assert!(matches!(judge(&schema, &b, None), Judgment::Rejected(_)));
    assert_eq!(judge(&schema, &b, Some(&a)), Judgment::Admitted);
    // Overlap/fullness is not equality, and a foreign context is a different
    // scalar literal rather than an attempted joint-world operation.
    for other in [
        space(false).full(),
        space(false).empty(),
        space(false).coordinate(1, &()).unwrap(),
        Space::new(SpaceId([120; 32]), 2, &())
            .unwrap()
            .coordinate(0, &())
            .unwrap(),
    ] {
        assert_eq!(judge(&schema, &other, None), Judgment::Admitted);
        assert!(matches!(
            judge(&schema, &a, Some(&other)),
            Judgment::Rejected(_)
        ));
    }
    let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), &());
    let density = ExactRational::fraction("1", "4", &mut arithmetic).unwrap();
    let base = space(false);
    let law = base
        .with_density(
            &[DensityPiece {
                region: base.full(),
                density,
            }],
            LawLimits::default(),
            &mut arithmetic,
        )
        .unwrap();
    assert_eq!(
        judge(&schema, &law.coordinate(0, &()).unwrap(), None),
        Judgment::Admitted,
        "same region with a different law is a different value"
    );
    let restricted = base
        .restrict(&base.coordinate(0, &()).unwrap(), &())
        .unwrap();
    assert_eq!(
        judge(&schema, &restricted.full(), None),
        Judgment::Admitted,
        "support belongs to the value"
    );
}

#[test]
fn event_selection_empty_and_zero_mass_values_still_select() {
    let base = space(false);
    let empty = base.empty();
    let schema = descriptor(&[empty]).validate().unwrap();
    assert!(matches!(
        judge(&schema, &space(true).empty(), None),
        Judgment::Rejected(_)
    ));
    let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), &());
    let half = ExactRational::fraction("1", "2", &mut arithmetic).unwrap();
    let coordinate = base.coordinate(0, &()).unwrap();
    let law = base
        .with_density(
            &[DensityPiece {
                region: coordinate.clone(),
                density: half,
            }],
            LawLimits::default(),
            &mut arithmetic,
        )
        .unwrap();
    let zero_mass = law.coordinate(0, &()).unwrap().complement();
    assert!(!zero_mass.is_empty());
    let decoded = Event::from_bytes(&zero_mass.to_bytes(&()).unwrap(), &()).unwrap();
    let schema = descriptor(&[zero_mass]).validate().unwrap();
    assert!(matches!(
        judge(&schema, &decoded, None),
        Judgment::Rejected(_)
    ));
}

#[test]
fn event_selection_sets_fingerprints_duplicates_and_mirrors_are_canonical() {
    let a = space(false).coordinate(0, &()).unwrap();
    let b = space(true).coordinate(1, &()).unwrap();
    let a2 = space(true).coordinate(0, &()).unwrap();
    let left = descriptor(&[a.clone(), b.clone()]).validate().unwrap();
    let right = descriptor(&[b.clone(), a2.clone()]).validate().unwrap();
    assert_eq!(
        fingerprint::fingerprint(&left),
        fingerprint::fingerprint(&right)
    );
    assert_eq!(
        render::render(&left, StatementId(2)),
        render::render(&right, StatementId(2))
    );
    let spelling = render::render(&left, StatementId(2));
    assert!(spelling.contains("event:0x42455654"));
    assert!(matches!(
        descriptor(&[a.clone(), a2.clone()]).validate(),
        Err(SchemaError::Statement {
            kind: StatementErrorKind::DuplicateSelectionLiteral { .. },
            ..
        })
    ));
    let mut duplicate = descriptor(&[a.clone(), b.clone()]);
    duplicate.statements.push(containment(
        selected(0, &[b.clone(), a2.clone()]),
        selected(1, &[b.clone(), a2.clone()]),
    ));
    assert!(matches!(
        duplicate.validate(),
        Err(SchemaError::Statement {
            kind: StatementErrorKind::DuplicateStatement {
                earlier: StatementId(2)
            },
            ..
        })
    ));
    let mut mirror = descriptor(&[a, b.clone()]);
    mirror.statements.push(containment(
        selected(1, &[b.clone(), a2.clone()]),
        selected(0, &[b, a2]),
    ));
    let mirror = mirror.validate().unwrap();
    assert_eq!(
        mirror.containments()[0].mirror_id(&mirror),
        Some(StatementId(3))
    );
}

#[test]
fn event_selection_filters_capacity_counts_by_exact_value() {
    let a = space(false).coordinate(0, &()).unwrap();
    let mut desc = descriptor(std::slice::from_ref(&a));
    desc.statements[2] = capacity(
        selected(0, std::slice::from_ref(&a)),
        1,
        Some(1),
        selected(1, std::slice::from_ref(&a)),
    );
    let schema = desc.validate().unwrap();
    assert_eq!(
        judge(&schema, &space(true).coordinate(0, &()).unwrap(), Some(&a)),
        Judgment::Admitted
    );
    assert!(matches!(
        judge(&schema, &space(false).full(), Some(&a)),
        Judgment::Rejected(_)
    ));
}

#[test]
fn event_literal_admission_and_matching_propagate_control_failures() {
    struct Stopped(EventError);
    impl Control for Stopped {
        fn checkpoint(&self) -> crate::event::Result<()> {
            Err(self.0)
        }
    }
    let a = space(false).coordinate(0, &()).unwrap();
    for error in [
        EventError::Cancelled,
        EventError::Allocation,
        EventError::Capacity(crate::event::Capacity::OperationSteps),
    ] {
        assert!(
            matches!(descriptor(std::slice::from_ref(&a)).validate_with_control(&Stopped(error)), Err(SchemaError::EventLiteral(actual)) if actual == error)
        );
        let schema = descriptor(std::slice::from_ref(&a)).validate().unwrap();
        assert_eq!(
            schema.event_literals.matches(
                &schema.containments()[0].source,
                &[Value::U64(1), Value::Event(a.clone())],
                &Stopped(error)
            ),
            Err(error)
        );
    }
}

#[test]
fn event_selection_keeps_projected_regions_whole() {
    let base = space(false);
    let filter = base.coordinate(0, &()).unwrap();
    let other = base.full();
    for interval in [false, true] {
        let region_type = if interval {
            ValueType::Interval {
                element: IntervalElement::U64,
            }
        } else {
            ValueType::Event
        };
        let region = |start, end| {
            if interval {
                Value::IntervalU64(crate::Interval::new(start, end).unwrap())
            } else {
                Value::Event(
                    base.table(3, &[((1 << end) - 1) ^ ((1 << start) - 1)], &())
                        .unwrap(),
                )
            }
        };
        let mut desc = descriptor(std::slice::from_ref(&filter));
        for relation in &mut desc.relations {
            relation.fields.push(field("region", region_type));
        }
        desc.statements = vec![
            fd(RelationId(1), &[FieldId(0), FieldId(2)]),
            containment(
                Side {
                    projection: [FieldId(0), FieldId(2)].as_slice().into(),
                    ..selected(0, std::slice::from_ref(&filter))
                },
                Side {
                    projection: [FieldId(0), FieldId(2)].as_slice().into(),
                    ..selected(1, std::slice::from_ref(&filter))
                },
            ),
        ];
        let schema = desc.validate().unwrap();
        let mut state = MapState::new();
        state
            .insert(
                RelationId(0),
                vec![Value::U64(1), Value::Event(filter.clone()), region(0, 4)],
            )
            .unwrap();
        state
            .insert(
                RelationId(1),
                vec![Value::U64(1), Value::Event(filter.clone()), region(0, 2)],
            )
            .unwrap();
        state
            .insert(
                RelationId(1),
                vec![Value::U64(1), Value::Event(other.clone()), region(2, 4)],
            )
            .unwrap();
        assert!(
            matches!(
                judge_complete(&schema, &state, &WorkContext::new(), JudgeBudget::default())
                    .unwrap(),
                Judgment::Rejected(_)
            ),
            "nonselected rows do not cover a gap; interval={interval}"
        );
    }
}
