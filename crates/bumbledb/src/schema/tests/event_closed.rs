use super::*;
use crate::Event;
use crate::error::{SchemaError, StatementErrorKind};
use crate::event::{
    ArithmeticLimits, Control, DensityPiece, Error as EventError, ExactArithmetic, ExactRational,
    LawLimits, Limits, Space, SpaceId,
};

fn space(reverse: bool) -> Space {
    Space::with_order(
        SpaceId([125; 32]),
        if reverse { &[1, 0] } else { &[0, 1] },
        Limits::default(),
        &(),
    )
    .unwrap()
}

fn roster(name: &str, events: &[Event]) -> RelationDescriptor {
    closed(
        name,
        vec![
            field("group", ValueType::U64),
            field("when", ValueType::Event),
        ],
        events
            .iter()
            .enumerate()
            .map(|(i, event)| {
                row(
                    &format!("Row{i}"),
                    vec![Value::U64(7), Value::Event(event.clone())],
                )
            })
            .collect(),
    )
}

fn partition(events: &[Event]) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![roster("Branch", events)],
        statements: vec![fd(RelationId(0), &[FieldId(1), FieldId(2)])],
    }
}

fn ground(
    sources: &[Event],
    targets: &[Event],
    full_source: bool,
    full_target: bool,
) -> SchemaDescriptor {
    let projection = |full| {
        if full {
            Projection::EventFull(Box::new([FieldId(1)]))
        } else {
            vec![FieldId(1), FieldId(2)].into()
        }
    };
    SchemaDescriptor {
        relations: vec![roster("Source", sources), roster("Target", targets)],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: projection(full_target),
            },
            containment(
                Side {
                    projection: projection(full_source),
                    ..side(RelationId(0), &[])
                },
                Side {
                    projection: projection(full_target),
                    ..side(RelationId(1), &[])
                },
            ),
        ],
    }
}

#[test]
fn portable_ground_identity_preserves_values_and_ignores_resident_owners() {
    let a = space(false).coordinate(0, &()).unwrap();
    let b = space(true).coordinate(0, &()).unwrap();
    assert_ne!(a, b);
    let left = partition(std::slice::from_ref(&a)).validate().unwrap();
    let right = partition(&[b]).validate().unwrap();
    assert_eq!(
        fingerprint::fingerprint(&left),
        fingerprint::fingerprint(&right)
    );
    let relation = left.relation(RelationId(0));
    let rows = relation.body().closed_rows().unwrap();
    assert_eq!(
        rows,
        right.relation(RelationId(0)).body().closed_rows().unwrap()
    );
    let bytes = a.to_bytes(&()).unwrap();
    let mut expected = [0_u64.to_be_bytes(), 7_u64.to_be_bytes()].concat();
    expected.extend_from_slice(&u32::try_from(bytes.len()).unwrap().to_le_bytes());
    expected.extend_from_slice(&bytes);
    assert_eq!(&*rows[0].fact, expected);
    assert_eq!(
        rows[0].field_bytes(relation.layout(), 1),
        7_u64.to_be_bytes()
    );
    assert_eq!(rows[0].event(2).to_bytes(&()).unwrap(), bytes);
}

#[test]
fn closed_event_keys_and_full_coverage_match_explicit_legal_worlds() {
    let base = space(false);
    for support in [15, 6, 1] {
        let source = base
            .restrict(&base.table(3, &[support], &()).unwrap(), &())
            .unwrap();
        for a in 0..16 {
            for b in 0..16 {
                let left = source.table(3, &[a], &()).unwrap();
                let right = Event::from_bytes(
                    &source.table(3, &[b], &()).unwrap().to_bytes(&()).unwrap(),
                    &(),
                )
                .unwrap();
                assert_eq!(
                    partition(&[left.clone(), right.clone()]).validate().is_ok(),
                    a & b & support == 0
                );
                assert_eq!(
                    ground(&[source.full()], &[left, right], true, false)
                        .validate()
                        .is_ok(),
                    a & b & support == 0 && (a | b) & support == support,
                    "support={support}, a={a}, b={b}"
                );
            }
        }
    }
}

#[test]
fn ground_coverage_handles_permutations_empty_and_contextual_full() {
    let base = space(false);
    let a = base.coordinate(0, &()).unwrap();
    let b = space(true).coordinate(0, &()).unwrap().complement();
    let mut desc = ground(&[base.full(), base.empty()], &[a.clone(), b], false, false);
    let StatementDescriptor::Containment { source, target } = &mut desc.statements[1] else {
        unreachable!()
    };
    source.projection = vec![FieldId(2), FieldId(1)].into();
    target.projection = vec![FieldId(2), FieldId(1)].into();
    desc.validate().unwrap();
    ground(&[a.clone(), base.empty()], &[base.full()], false, true)
        .validate()
        .unwrap();
    let mut absent = ground(&[base.empty()], &[a], false, false);
    absent.relations[1].extension.as_mut().unwrap()[0].values[0] = Value::U64(8);
    absent.clone().validate().unwrap();
    absent.relations[0].extension.as_mut().unwrap()[0].values[1] = Value::Event(base.full());
    assert!(matches!(
        absent.validate(),
        Err(SchemaError::Statement {
            kind: StatementErrorKind::ClosedStatementRefuted { .. },
            ..
        })
    ));
}

#[test]
fn empty_ground_events_cannot_hide_incompatible_contexts() {
    let base = space(false);
    let foreign = Space::new(SpaceId([126; 32]), 2, &()).unwrap();
    assert!(matches!(
        partition(&[base.empty(), foreign.empty()]).validate(),
        Err(SchemaError::EventLiteral(EventError::SpaceMismatch))
    ));
    for target_full in [false, true] {
        let desc = ground(
            &[base.empty(), foreign.empty()],
            &[base.empty()],
            false,
            target_full,
        );
        assert!(matches!(
            desc.clone().validate(),
            Err(SchemaError::EventLiteral(EventError::SpaceMismatch))
        ));
        let mut absent = desc;
        absent.relations[1].extension.as_mut().unwrap()[0].values[0] = Value::U64(8);
        assert!(matches!(
            absent.validate(),
            Err(SchemaError::EventLiteral(EventError::SpaceMismatch))
        ));
    }
    let mut separate = partition(&[base.empty(), foreign.empty()]);
    separate.relations[0].extension.as_mut().unwrap()[1].values[0] = Value::U64(8);
    separate.validate().unwrap();
}

#[test]
fn possible_zero_mass_ground_events_still_overlap_and_need_coverage() {
    let base = space(false);
    let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), &());
    let law = base
        .with_density(
            &[DensityPiece {
                region: base.coordinate(0, &()).unwrap(),
                density: ExactRational::fraction("1", "2", &mut arithmetic).unwrap(),
            }],
            LawLimits::default(),
            &mut arithmetic,
        )
        .unwrap();
    let impossible_mass = law.coordinate(0, &()).unwrap().complement();
    assert!(!impossible_mass.is_empty());
    assert!(
        partition(&[impossible_mass.clone(), impossible_mass.clone()])
            .validate()
            .is_err()
    );
    assert!(
        ground(&[impossible_mass], &[law.empty()], false, false)
            .validate()
            .is_err()
    );
}

#[test]
fn exact_event_selections_filter_ground_members_and_scalar_capacity() {
    let base = space(false);
    let a = base.coordinate(0, &()).unwrap();
    let copy = space(true).coordinate(0, &()).unwrap();
    let selected = |rel| {
        side_where(
            RelationId(rel),
            &[FieldId(0)],
            vec![(FieldId(2), Value::Event(copy.clone()))],
        )
    };
    let mut desc = SchemaDescriptor {
        relations: vec![
            roster("Source", &[a.clone(), base.full()]),
            roster("Target", &[a.clone(), base.empty()]),
        ],
        statements: vec![
            containment(selected(0), selected(1)),
            capacity(selected(0), 1, Some(1), selected(1)),
        ],
    };
    desc.clone().validate().unwrap();
    desc.relations[0].extension.as_mut().unwrap()[1].values[1] = Value::Event(a);
    assert!(matches!(
        desc.validate(),
        Err(SchemaError::Statement {
            kind: StatementErrorKind::ClosedStatementRefuted { .. },
            ..
        })
    ));
}

#[test]
fn ground_sealing_observes_control_after_admission_begins() {
    struct StopAfter(std::cell::Cell<usize>);
    impl Control for StopAfter {
        fn checkpoint(&self) -> crate::event::Result<()> {
            let count = self.0.get();
            self.0.set(count.saturating_sub(1));
            if count == 0 {
                Err(EventError::Cancelled)
            } else {
                Ok(())
            }
        }
    }
    let base = space(false);
    let desc = partition(&[
        base.coordinate(0, &()).unwrap(),
        base.coordinate(1, &()).unwrap(),
    ]);
    assert!(matches!(
        desc.validate_with_control(&StopAfter(std::cell::Cell::new(5))),
        Err(SchemaError::EventLiteral(EventError::Cancelled))
    ));
}

#[test]
fn scalar_weights_and_bounds_after_event_fields_keep_their_offsets() {
    let a = space(false).coordinate(0, &()).unwrap();
    let desc = SchemaDescriptor {
        relations: vec![
            closed(
                "Child",
                vec![
                    field("parent", ValueType::U64),
                    field("when", ValueType::Event),
                    field("weight", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
                vec![row(
                    "Only",
                    vec![
                        Value::U64(0),
                        Value::Event(a.clone()),
                        Value::U64(3),
                        Value::IntervalU64(crate::Interval::new(2, 5).unwrap()),
                    ],
                )],
            ),
            closed(
                "Parent",
                vec![
                    field("when", ValueType::Event),
                    field("limit", ValueType::U64),
                ],
                vec![row("Only", vec![Value::Event(a), Value::U64(3)])],
            ),
        ],
        statements: vec![
            capacity_weighted(
                side(RelationId(1), &[FieldId(0)]),
                Weight::Field(FieldId(3)),
                3,
                Some(Bound::TargetField(FieldId(2))),
                side(RelationId(0), &[FieldId(1)]),
            ),
            capacity_weighted(
                side(RelationId(1), &[FieldId(0)]),
                Weight::DurationOf(FieldId(4)),
                3,
                Some(Bound::TargetField(FieldId(2))),
                side(RelationId(0), &[FieldId(1)]),
            ),
        ],
    };
    desc.clone().validate().unwrap();
    let mut wrong = desc;
    wrong.relations[1].extension.as_mut().unwrap()[0].values[1] = Value::U64(2);
    assert!(wrong.validate().is_err());
}

#[test]
fn event_fields_count_both_physical_columns_before_sealing() {
    let desc = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Wide".into(),
            fields: (0..32768)
                .map(|n| field(&format!("v{n}"), ValueType::Event))
                .collect(),
            extension: None,
        }],
        statements: vec![],
    };
    assert!(matches!(
        desc.validate(),
        Err(SchemaError::RelationTooManyColumns { columns: 65536, .. })
    ));
}
