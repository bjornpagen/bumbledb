use super::*;
use crate::error::SchemaError;

#[test]
fn rejects_duplicate_relation_name() {
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "R".into(),
                fields: vec![],
                constraints: vec![],
            },
            RelationDescriptor {
                name: "R".into(),
                fields: vec![],
                constraints: vec![],
            },
        ],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateRelationName { name: "R".into() }
    );
}

#[test]
fn rejects_duplicate_field_name() {
    let decl = one_relation(
        vec![field("x", ValueType::U64), field("x", ValueType::I64)],
        vec![],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateFieldName {
            relation: RelationId(0),
            name: "x".into()
        }
    );
}

#[test]
fn rejects_duplicate_constraint_name_including_auto_unique_collision() {
    // A declared constraint colliding with a serial auto-unique's name is
    // the same duplicate-name error — auto-uniques are ordinary.
    let decl = one_relation(
        vec![serial_field("id")],
        vec![ConstraintDescriptor::Unique {
            name: "id".into(),
            fields: Box::new([FieldId(0)]),
        }],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateConstraintName {
            relation: RelationId(0),
            name: "id".into()
        }
    );
}

#[test]
fn rejects_enum_without_variants() {
    let decl = one_relation(vec![field("e", enum_type(&[]))], vec![]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::EnumWithoutVariants {
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

#[test]
fn rejects_enum_with_more_than_256_variants() {
    let names: Vec<String> = (0..257).map(|i| format!("V{i}")).collect();
    let decl = one_relation(
        vec![field(
            "e",
            enum_type(&names.iter().map(String::as_str).collect::<Vec<_>>()),
        )],
        vec![],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::EnumTooManyVariants {
            relation: RelationId(0),
            field: FieldId(0),
            count: 257
        }
    );
}

#[test]
fn rejects_duplicate_enum_variant() {
    let decl = one_relation(vec![field("e", enum_type(&["A", "A"]))], vec![]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateEnumVariant {
            relation: RelationId(0),
            field: FieldId(0),
            variant: "A".into()
        }
    );
}

#[test]
fn rejects_serial_on_non_u64() {
    let decl = one_relation(
        vec![FieldDescriptor {
            name: "id".into(),
            value_type: ValueType::I64,
            generation: Generation::Serial,
        }],
        vec![],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::SerialOnNonU64 {
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

#[test]
fn rejects_unknown_constraint_field() {
    let decl = one_relation(
        vec![field("x", ValueType::U64)],
        vec![ConstraintDescriptor::Unique {
            name: "u".into(),
            fields: Box::new([FieldId(7)]),
        }],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::UnknownConstraintField {
            relation: RelationId(0),
            constraint: ConstraintId(0),
            field: FieldId(7)
        }
    );
}

#[test]
fn rejects_unique_without_fields() {
    let decl = one_relation(
        vec![field("x", ValueType::U64)],
        vec![ConstraintDescriptor::Unique {
            name: "u".into(),
            fields: Box::new([]),
        }],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::UniqueWithoutFields {
            relation: RelationId(0),
            constraint: ConstraintId(0)
        }
    );
}

#[test]
fn rejects_unique_with_duplicate_field() {
    let decl = one_relation(
        vec![field("x", ValueType::U64)],
        vec![ConstraintDescriptor::Unique {
            name: "u".into(),
            fields: Box::new([FieldId(0), FieldId(0)]),
        }],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::ConstraintDuplicateField {
            relation: RelationId(0),
            constraint: ConstraintId(0),
            field: FieldId(0)
        }
    );
}

#[test]
fn rejects_unknown_fk_target_relation() {
    let decl = one_relation(
        vec![field("x", ValueType::U64)],
        vec![ConstraintDescriptor::ForeignKey {
            name: "fk".into(),
            fields: Box::new([FieldId(0)]),
            target_relation: RelationId(9),
            target_constraint: ConstraintId(0),
        }],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::UnknownFkTargetRelation {
            relation: RelationId(0),
            constraint: ConstraintId(0),
            target: RelationId(9)
        }
    );
}

#[test]
fn rejects_unknown_fk_target_constraint() {
    let decl = one_relation(
        vec![field("x", ValueType::U64)],
        vec![ConstraintDescriptor::ForeignKey {
            name: "fk".into(),
            fields: Box::new([FieldId(0)]),
            target_relation: RelationId(0),
            target_constraint: ConstraintId(9),
        }],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::UnknownFkTargetConstraint {
            relation: RelationId(0),
            constraint: ConstraintId(0),
            target: ConstraintId(9)
        }
    );
}

#[test]
fn rejects_fk_targeting_a_foreign_key() {
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "T".into(),
                fields: vec![field("x", ValueType::U64)],
                constraints: vec![
                    ConstraintDescriptor::Unique {
                        name: "x".into(),
                        fields: Box::new([FieldId(0)]),
                    },
                    ConstraintDescriptor::ForeignKey {
                        name: "self_fk".into(),
                        fields: Box::new([FieldId(0)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(0),
                    },
                ],
            },
            RelationDescriptor {
                name: "S".into(),
                fields: vec![field("y", ValueType::U64)],
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "bad".into(),
                    fields: Box::new([FieldId(0)]),
                    target_relation: RelationId(0),
                    target_constraint: ConstraintId(1), // T's FK, not a unique
                }],
            },
        ],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::FkTargetNotUnique {
            relation: RelationId(1),
            constraint: ConstraintId(0),
            target: ConstraintId(1)
        }
    );
}

#[test]
fn rejects_fk_arity_mismatch() {
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "T".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "ab".into(),
                    fields: Box::new([FieldId(0), FieldId(1)]),
                }],
            },
            RelationDescriptor {
                name: "S".into(),
                fields: vec![field("a", ValueType::U64)],
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "fk".into(),
                    fields: Box::new([FieldId(0)]),
                    target_relation: RelationId(0),
                    target_constraint: ConstraintId(0),
                }],
            },
        ],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::FkArityMismatch {
            relation: RelationId(1),
            constraint: ConstraintId(0)
        }
    );
}

#[test]
fn rejects_fk_positional_type_mismatch() {
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "T".into(),
                fields: vec![field("a", ValueType::U64)],
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "a".into(),
                    fields: Box::new([FieldId(0)]),
                }],
            },
            RelationDescriptor {
                name: "S".into(),
                fields: vec![field("a", ValueType::I64)],
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "fk".into(),
                    fields: Box::new([FieldId(0)]),
                    target_relation: RelationId(0),
                    target_constraint: ConstraintId(0),
                }],
            },
        ],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::FkFieldTypeMismatch {
            relation: RelationId(1),
            constraint: ConstraintId(0),
            position: 0
        }
    );
}

// `Schema` is unconstructible outside this module: its fields and
// `Relation`'s fields are private, and no public constructor exists —
// the only path in is `SchemaDescriptor::validate`. (Compile-time
// property; recorded here as the sealing contract.)

#[test]
fn rejects_two_uniques_over_one_field_set() {
    let err = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "R".into(),
            fields: vec![FieldDescriptor {
                name: "a".into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            }],
            constraints: vec![
                ConstraintDescriptor::Unique {
                    name: "first".into(),
                    fields: Box::new([FieldId(0)]),
                },
                ConstraintDescriptor::Unique {
                    name: "second".into(),
                    fields: Box::new([FieldId(0)]),
                },
            ],
        }],
    }
    .validate()
    .unwrap_err();
    assert!(matches!(err, SchemaError::DuplicateConstraintFields { .. }));
}

#[test]
fn a_declared_unique_duplicating_a_serial_auto_unique_is_rejected() {
    // The auto-unique on the serial field covers [FieldId(0)]; a
    // declared unique over the same set is double guard maintenance.
    let err = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "R".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
                generation: Generation::Serial,
            }],
            constraints: vec![ConstraintDescriptor::Unique {
                name: "extra".into(),
                fields: Box::new([FieldId(0)]),
            }],
        }],
    }
    .validate()
    .unwrap_err();
    assert!(matches!(err, SchemaError::DuplicateConstraintFields { .. }));
}

#[test]
fn rejects_duplicate_fields_in_an_fk_list() {
    let err = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "T".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "x".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "y".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "xy".into(),
                    fields: Box::new([FieldId(0), FieldId(1)]),
                }],
            },
            RelationDescriptor {
                name: "S".into(),
                fields: vec![FieldDescriptor {
                    name: "a".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                }],
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "typo".into(),
                    fields: Box::new([FieldId(0), FieldId(0)]),
                    target_relation: RelationId(0),
                    target_constraint: ConstraintId(0),
                }],
            },
        ],
    }
    .validate()
    .unwrap_err();
    assert!(matches!(err, SchemaError::ConstraintDuplicateField { .. }));
}
