use super::*;
use crate::error::StatementErrorKind;

fn member_set(indices: &[u8]) -> MemberSet {
    let mut members = MemberSet::empty();
    for &index in indices {
        members.insert(AxiomIndex(index));
    }
    members
}

#[test]
fn valid_schema_constructs_with_statement_indices() {
    let schema = ledger_slice().validate().expect("valid schema");
    let holder = schema.relation(RelationId(0));

    assert_eq!(holder.keys(), &[KeyId(0)]);
    assert_eq!(holder.outgoing(), &[]);

    assert_eq!(schema.dependents(KeyId(0)), &[ContainmentId(0)]);

    let account = schema.relation(RelationId(1));
    assert_eq!(account.keys(), &[KeyId(1)]);
    assert_eq!(account.outgoing(), &[ContainmentId(0)]);
    assert_eq!(schema.dependents(KeyId(1)), &[]);

    assert_eq!(account.layout().fact_width(), 24);
}

#[test]
fn a_redundant_pointwise_superkey_remains_sealed() {
    let mut descriptor = one_relation(vec![
        field("id", ValueType::U64),
        field(
            "span",
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        ),
    ]);
    descriptor.statements = vec![
        fd(RelationId(0), &[FieldId(0)]),
        fd(RelationId(0), &[FieldId(0), FieldId(1)]),
    ];
    let schema = descriptor
        .validate()
        .expect("a redundant superkey remains accepted");

    assert_eq!(schema.keys().len(), 2, "both keys remain sealed");
    assert!(!schema.key(KeyId(0)).form().is_pointwise());
    assert!(schema.key(KeyId(1)).form().is_pointwise());
}

#[test]
fn statement_ids_are_auto_fds_first_then_declared_order() {
    let mut decl = ledger_slice();
    decl.statements.push(StatementDescriptor::Functionality {
        relation: RelationId(1),
        projection: Box::new([FieldId(1), FieldId(2)]),
    });
    let materialized = decl.materialized_statements();
    assert_eq!(
        materialized,
        vec![

            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            },

            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },

            StatementDescriptor::Containment {
                source: side(RelationId(1), &[FieldId(1)]),
                target: side(RelationId(0), &[FieldId(0)]),
            },

            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(1), FieldId(2)]),
            },
        ]
    );

    let schema = decl.validate().expect("valid schema");
    for (index, descriptor) in materialized.iter().enumerate() {
        let id = StatementId(u16::try_from(index).expect("small fixture"));
        match (schema.statement(id), descriptor) {
            (
                StatementView::Key(_, sealed),
                StatementDescriptor::Functionality {
                    relation,
                    projection,
                },
            ) => {
                assert_eq!(sealed.relation, *relation);
                assert_eq!(sealed.projection, *projection);
            }
            (
                StatementView::Containment(_, sealed),
                StatementDescriptor::Containment { source, target },
            ) => {
                assert_eq!(sealed.source, *source);
                assert_eq!(sealed.target, *target);
            }
            _ => panic!("materialized descriptor and typed arena disagree"),
        }
    }
    assert_eq!(schema.relation(RelationId(1)).keys(), &[KeyId(1), KeyId(2)]);
}

#[test]
fn statement_order_preserves_materialized_identity() {
    let schema = ledger_slice().validate().expect("valid schema");
    for index in 0..3 {
        let id = StatementId(index);
        assert_eq!(schema.statement(id).id(), id);
        assert_eq!(
            schema.statement_checked(id).map(StatementView::id),
            Some(id)
        );
    }
    assert!(schema.statement_checked(StatementId(3)).is_none());
}

#[test]
fn dependents_are_typed_total_witnesses() {
    let schema = ledger_slice().validate().expect("valid schema");
    let key = schema.relation(RelationId(0)).keys()[0];
    assert_eq!(schema.key(key).id, StatementId(0));
    assert_eq!(
        schema.key_checked(key).map(|statement| statement.id),
        Some(StatementId(0))
    );
    for dependent in schema.dependents(key) {
        assert_eq!(schema.containment(*dependent).id, StatementId(2));
        assert_eq!(
            schema
                .containment_checked(*dependent)
                .map(|statement| statement.id),
            Some(StatementId(2))
        );
    }
}

#[test]
fn nullary_relation_constructs() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Flag".into(),
            fields: vec![],
        }],
        statements: vec![],
    }
    .validate()
    .expect("nullary relations are legal");
    assert_eq!(schema.relation(RelationId(0)).layout().fact_width(), 0);
}

#[test]
fn example_schema_resolves_exactly() {
    let savings = Value::U64(1); 
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![fresh_field("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![
                    fresh_field("id"),
                    field("holder", ValueType::U64),
                    field("kind", ValueType::U64),
                    field(
                        "active",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "SavingsTerms".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field("rate_bps", ValueType::I64),
                ],
            },
        ],
        statements: vec![

            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),

            containment(
                side_where(
                    RelationId(1),
                    &[FieldId(0)],
                    vec![(FieldId(2), savings.clone())],
                ),
                side(RelationId(2), &[FieldId(0)]),
            ),
            containment(
                side(RelationId(2), &[FieldId(0)]),
                side_where(RelationId(1), &[FieldId(0)], vec![(FieldId(2), savings)]),
            ),

            fd(RelationId(2), &[FieldId(0)]),
        ],
    }
    .validate()
    .expect("the 30-dependencies example schema is valid");

    assert!(schema.keys().iter().all(|key| !key.form().is_pointwise()));
    let probe = |target_key: u16, source: u16| Enforcement::ScalarProbe {
        target_key: KeyId(target_key),
        key_projection: Box::new([FieldId(source)]),
    };
    assert_eq!(
        schema
            .containments()
            .iter()
            .map(|statement| &statement.enforcement)
            .collect::<Vec<_>>(),
        vec![
            &probe(0, 1), 
            &probe(2, 0), 
            &probe(1, 0), 
        ]
    );

    let mirrors: Vec<Option<StatementId>> = schema
        .containments()
        .iter()
        .map(|statement| statement.mirror_id(&schema))
        .collect();
    assert_eq!(
        mirrors,
        vec![None, Some(StatementId(4)), Some(StatementId(3)),]
    );

    assert_eq!(schema.dependents(KeyId(0)), &[ContainmentId(0)]);
    assert_eq!(schema.dependents(KeyId(1)), &[ContainmentId(2)]);
    assert_eq!(schema.dependents(KeyId(2)), &[ContainmentId(1)]);
}

#[test]
fn pointwise_key_and_containment_resolve() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![field("room", ValueType::U64), field("during", iv)],
            },
            RelationDescriptor {
                extension: None,
                name: "Request".into(),
                fields: vec![field("room", ValueType::U64), field("span", iv)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0), FieldId(1)]),
            containment(
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("pointwise key and coverage containment are valid");

    assert!(schema.key(KeyId(0)).form().is_pointwise());
    assert!(matches!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::IntervalCoverage {
            target_key: KeyId(0),
            ref key_projection,
            ..
        } if **key_projection == [FieldId(0), FieldId(1)]
    ));
    assert_eq!(schema.dependents(KeyId(0)), &[ContainmentId(0)]);
}

#[test]
fn permuted_target_projection_resolves_with_permutation() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::I64)],
            },
            RelationDescriptor {
                extension: None,
                name: "S".into(),
                fields: vec![field("x", ValueType::I64), field("y", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0), FieldId(1)]), 

            containment(
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
                side(RelationId(0), &[FieldId(1), FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("a permuted target projection resolves");

    assert_eq!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::ScalarProbe {
            target_key: KeyId(0),
            key_projection: Box::new([FieldId(1), FieldId(0)]),
        }
    );
}

#[test]
fn permutation_is_stored_inverse_determinant_position_to_projection_index() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![
                    field("a", ValueType::U64),
                    field("b", ValueType::I64),
                    field("c", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "S".into(),
                fields: vec![
                    field("x", ValueType::U64),
                    field("y", ValueType::U64),
                    field("z", ValueType::I64),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0), FieldId(1), FieldId(2)]), 
            // S(x, y, z) <= T(c, a, b): a 3-cycle against the key.
            containment(
                side(RelationId(1), &[FieldId(0), FieldId(1), FieldId(2)]),
                side(RelationId(0), &[FieldId(2), FieldId(0), FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("a 3-cycle target projection resolves");

    assert_eq!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::ScalarProbe {
            target_key: KeyId(0),
            key_projection: Box::new([FieldId(1), FieldId(2), FieldId(0)]),
        }
    );
}

fn currency() -> RelationDescriptor {
    closed(
        "Currency",
        vec![field("minor_units", ValueType::U64)],
        vec![
            row("Usd", vec![Value::U64(2)]),
            row("Eur", vec![Value::U64(2)]),
        ],
    )
}

#[test]
fn a_closed_relation_seals_pre_encoded_ground_axioms() {
    let schema = SchemaDescriptor {
        relations: vec![currency()],
        statements: vec![],
    }
    .validate()
    .expect("a closed relation validates");
    let relation = schema.relation(RelationId(0));
    assert!(relation.body().closed_rows().is_some());

    assert_eq!(relation.fields()[0].name.as_ref(), "id");
    assert_eq!(relation.fields()[0].value_type, ValueType::U64);
    assert_eq!(relation.fields()[0].generation, Generation::None);
    assert_eq!(relation.fields()[1].name.as_ref(), "minor_units");
    assert_eq!(relation.layout().fact_width(), 16);

    let rows = relation.body().closed_rows().expect("closed");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].handle.as_ref(), "Usd");
    assert_eq!(rows[1].handle.as_ref(), "Eur");
    let fact = |id: u64, units: u64| {
        let mut fact = Vec::new();
        fact.extend_from_slice(&id.to_be_bytes());
        fact.extend_from_slice(&units.to_be_bytes());
        fact.into_boxed_slice()
    };
    assert_eq!(rows[0].fact, fact(0, 2));
    assert_eq!(rows[1].fact, fact(1, 2));

    assert_eq!(relation.keys(), &[KeyId(0)]);
}

/// Holder declares AFTER Currency so the fresh/closed grouping (not relation
/// order) is what the assertion pins; the declared containment also proves the
/// closed auto-key targetable like any key.
#[test]
fn closed_auto_keys_sit_between_fresh_auto_fds_and_declared_statements() {
    let decl = SchemaDescriptor {
        relations: vec![
            currency(),
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![fresh_field("id"), field("currency", ValueType::U64)],
            },
        ],
        statements: vec![containment(
            side(RelationId(1), &[FieldId(1)]),
            side(RelationId(0), &[FieldId(0)]),
        )],
    };
    assert_eq!(
        decl.materialized_statements(),
        vec![

            fd(RelationId(1), &[FieldId(0)]),

            fd(RelationId(0), &[FieldId(0)]),

            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ]
    );
    let schema = decl.validate().expect("valid");

    assert_eq!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::Closed {
            members: member_set(&[0, 1])
        }
    );

    assert_eq!(schema.dependents(KeyId(1)), &[]);
}

#[test]
fn a_psi_selected_closed_containment_compiles_its_member_set() {
    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Severity",
                vec![field("pages", ValueType::Bool)],
                vec![
                    row("Low", vec![Value::Bool(false)]),
                    row("Med", vec![Value::Bool(true)]),
                    row("High", vec![Value::Bool(true)]),
                ],
            ),
            RelationDescriptor {
                extension: None,
                name: "Escalation".into(),
                fields: vec![field("severity", ValueType::U64)],
            },
        ],
        statements: vec![containment(
            side(RelationId(1), &[FieldId(0)]),
            side_where(
                RelationId(0),
                &[FieldId(0)],
                vec![(FieldId(1), Value::Bool(true))],
            ),
        )],
    };
    let schema = decl.validate().expect("valid");

    assert_eq!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::Closed {
            members: member_set(&[1, 2])
        }
    );
}

#[test]
fn a_satisfied_closed_to_closed_containment_validates() {
    let decl = SchemaDescriptor {
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
    };
    decl.validate()
        .expect("every Kind severity is an axiom of Severity");
}

fn task_tree() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Parent".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Task".into(),
                fields: vec![
                    field("parent", ValueType::U64),
                    field("pos", ValueType::U64),
                    field("prio", ValueType::U64),
                    field("flag", ValueType::Bool),
                ],
            },
        ],
        statements: vec![fd(RelationId(0), &[FieldId(0)])],
    }
}

/// `Parent(id) <={1..3} Task(parent)` seals into the window arena with the
/// containment target-key rule reused — the acceptance premise of
/// `lean/Bumbledb/Admission.lean: capacityForm`, and the plan the gate promises
/// is `lean/Bumbledb/Oracle.lean: capacity_plan_decides`.
#[test]
fn a_capacity_statement_over_a_declared_key_validates() {
    let mut decl = task_tree();
    decl.statements.push(capacity(
        side(RelationId(1), &[FieldId(0)]),
        1,
        Some(3),
        side(RelationId(0), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("the window passes the gate");
    assert_eq!(schema.capacities().len(), 1);
    let window = schema.capacity(CapacityId(0));
    assert_eq!(window.id, StatementId(1));
    assert_eq!(window.lo, 1);
    assert_eq!(window.hi.to_bound(), Some(Bound::Lit(3)));

    assert!(matches!(
        schema.statement(StatementId(1)),
        StatementView::Capacity(CapacityId(0), _)
    ));
}

/// `{2..*}` — `hi = None` is the `*` spelling, the only spelling of "no upper
/// bound" (`lean/Bumbledb/Schema.lean: Window`).
#[test]
fn a_star_window_validates_with_no_ceiling() {
    let mut decl = task_tree();
    decl.statements.push(capacity(
        side(RelationId(1), &[FieldId(0)]),
        2,
        None,
        side(RelationId(0), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("the floored window validates");
    assert_eq!(schema.capacity(CapacityId(0)).hi.to_bound(), None);
}

#[test]
fn an_exclusion_window_validates() {
    let mut decl = task_tree();
    decl.statements.push(capacity(
        side(RelationId(1), &[FieldId(0)]),
        0,
        Some(0),
        side(RelationId(0), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("the exclusion passes the gate");
    assert_eq!(schema.capacity(CapacityId(0)).lo, 0);
    assert_eq!(
        schema.capacity(CapacityId(0)).hi.to_bound(),
        Some(Bound::Lit(0))
    );
}

#[test]
fn a_window_into_a_closed_target_validates() {
    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Severity",
                vec![],
                vec![row("Low", vec![]), row("High", vec![])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Handler".into(),
                fields: vec![field("severity", ValueType::U64)],
            },
        ],
        statements: vec![capacity(
            side(RelationId(1), &[FieldId(0)]),
            2,
            None,
            side(RelationId(0), &[FieldId(0)]),
        )],
    };
    decl.validate()
        .expect("every severity demands at least two handlers");
}

/// Both sides constant and the counts inside the window: decided at validate,
/// satisfied, sealed (`lean/Bumbledb/Schema.lean: den_closed_constant`).
#[test]
fn a_satisfied_closed_to_closed_window_validates() {
    let decl = SchemaDescriptor {
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
        statements: vec![capacity(
            side(RelationId(0), &[FieldId(1)]),
            1,
            Some(1),
            side(RelationId(1), &[FieldId(0)]),
        )],
    };
    decl.validate()
        .expect("each severity counts exactly one kind axiom");
}

fn power_tree() -> SchemaDescriptor {
    let interval = ValueType::Interval {
        element: IntervalElement::U64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Pool".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("supply", ValueType::U64),
                    field("span", interval),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![
                    field("pool", ValueType::U64),
                    field("watts", ValueType::U64),
                    field("busy", interval),
                ],
            },
        ],
        statements: vec![fd(RelationId(0), &[FieldId(0)])],
    }
}

#[test]
fn a_weighted_capacity_with_a_dependent_bound_validates() {
    let mut decl = power_tree();
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::Field(FieldId(1)),
        0,
        Some(Bound::TargetField(FieldId(1))),
        side(RelationId(1), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("the power budget passes the gate");
    let statement = schema.capacity(CapacityId(0));
    assert_eq!(statement.weight.to_weight(), Weight::Field(FieldId(1)));
    assert_eq!(
        statement.hi.to_bound(),
        Some(Bound::TargetField(FieldId(1)))
    );
    assert!(matches!(statement.weight, SealedWeight::Field(_)));
    assert!(matches!(statement.hi, SealedBound::TargetField(_)));
}

#[test]
fn a_calendar_capacity_validates_and_seals_its_tails() {
    let mut decl = power_tree();
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::DurationOf(FieldId(2)),
        0,
        Some(Bound::TargetDuration(FieldId(2))),
        side(RelationId(1), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("the calendar law passes the gate");
    let statement = schema.capacity(CapacityId(0));
    assert!(matches!(statement.weight, SealedWeight::Duration { .. }));
    assert!(matches!(statement.hi, SealedBound::Duration { .. }));
}

/// `<=[w]{1..*}` — the weighted floor of 1 is LEGAL: "positive total" is no
/// existence claim over rows (zero-weight rows satisfy nothing), so the
/// containment-respelled ban fires on the unit instance only (the per-aggregate
/// ban law, ruled 2026-07-24; the unit refusal is the reject suite's
/// `rejects_the_containment_respelled_as_a_window`).
#[test]
fn a_weighted_floor_of_one_validates() {
    let mut decl = power_tree();
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::Field(FieldId(1)),
        1,
        None,
        side(RelationId(1), &[FieldId(0)]),
    ));
    let schema = decl
        .validate()
        .expect("a positive total is a law of its own");
    assert_eq!(schema.capacity(CapacityId(0)).lo, 1);
}

#[test]
fn a_duration_weight_under_a_literal_ceiling_validates() {
    let mut decl = power_tree();
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::DurationOf(FieldId(2)),
        0,
        Some(Bound::Lit(720)),
        side(RelationId(1), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("hours under a literal budget");
    let statement = schema.capacity(CapacityId(0));
    assert!(matches!(
        statement.weight,
        super::super::SealedWeight::Duration { .. }
    ));
    assert!(matches!(statement.hi, super::super::SealedBound::Lit(_)));
}

#[test]
fn a_satisfied_weighted_closed_pair_validates() {
    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Pool",
                vec![field("cap", ValueType::U64)],
                vec![row("P0", vec![Value::U64(9)])],
            ),
            closed(
                "Dev",
                vec![
                    field("pool", ValueType::U64),
                    field("watts", ValueType::U64),
                ],
                vec![
                    row("D0", vec![Value::U64(0), Value::U64(3)]),
                    row("D1", vec![Value::U64(0), Value::U64(4)]),
                ],
            ),
        ],
        statements: vec![capacity_weighted(
            side(RelationId(0), &[FieldId(0)]),
            Weight::Field(FieldId(2)),
            0,
            Some(Bound::TargetField(FieldId(1))),
            side(RelationId(1), &[FieldId(1)]),
        )],
    };
    decl.validate()
        .expect("7 watts inside a 9-watt budget, per the sealed axiom");
}

/// A literal-set σ seals — and seals CANONICALLY: the sealed side sorts the
/// set, so both spellings of one set are one statement and one fingerprint
/// (`lean/Bumbledb/Schema.lean: Selection` — the set is the binding's identity,
/// not its spelling).
#[test]
fn a_literal_set_selection_seals_sorted() {
    let build = |literals: Vec<Value>| {
        let mut decl = task_tree();
        decl.statements.push(containment(
            side_where_sets(
                RelationId(1),
                &[FieldId(0)],
                vec![(FieldId(2), LiteralSet::Many(literals.into_boxed_slice()))],
            ),
            side(RelationId(0), &[FieldId(0)]),
        ));
        decl.validate().expect("the set binding passes the gate")
    };
    let ascending = build(vec![Value::U64(1), Value::U64(2)]);
    let descending = build(vec![Value::U64(2), Value::U64(1)]);
    let sealed = |schema: &Schema| {
        let statement = schema.containment(ContainmentId(0));
        statement.source.selection[0].1.clone()
    };
    assert_eq!(
        sealed(&ascending),
        LiteralSet::Many(Box::new([Value::U64(1), Value::U64(2)]))
    );
    assert_eq!(sealed(&ascending), sealed(&descending));
    assert_eq!(
        crate::schema::fingerprint::fingerprint(&ascending),
        crate::schema::fingerprint::fingerprint(&descending),
    );
}

#[test]
fn a_reordered_literal_set_is_a_duplicate_statement() {
    let selected = |literals: Vec<Value>| {
        containment(
            side_where_sets(
                RelationId(1),
                &[FieldId(0)],
                vec![(FieldId(2), LiteralSet::Many(literals.into_boxed_slice()))],
            ),
            side(RelationId(0), &[FieldId(0)]),
        )
    };
    let mut decl = task_tree();
    decl.statements
        .push(selected(vec![Value::U64(1), Value::U64(2)]));
    decl.statements
        .push(selected(vec![Value::U64(2), Value::U64(1)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DuplicateStatement {
            earlier: StatementId(1)
        }
        .at(StatementId(2))
    );
}

/// Q1 — element-domain typing at interval positions: a fixed-width interval
/// position against a GENERAL one of the same element domain matches
/// positionally (widths free; the pointwise judgments quantify over points,
/// which carry an element domain and never a width —
/// `lean/Bumbledb/Schema.lean: Value.points_one_tag_u64`), and the containment
/// resolves the same pointwise coverage plan.
#[test]
fn mixed_width_interval_positions_of_one_element_domain_resolve() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Playlist".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Slot".into(),
                fields: vec![
                    field("playlist", ValueType::U64),
                    field(
                        "slot",
                        ValueType::FixedInterval {
                            element: IntervalElement::U64,
                            width: 1,
                        },
                    ),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0), FieldId(1)]),
            fd(RelationId(1), &[FieldId(0), FieldId(1)]),

            containment(
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
            ),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("mixed widths of one element domain validate (Q1)");
    assert!(schema.key(KeyId(0)).form().is_pointwise());
    assert!(schema.key(KeyId(1)).form().is_pointwise());
    assert!(matches!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::IntervalCoverage { .. }
    ));
    assert!(matches!(
        schema.containment(ContainmentId(1)).enforcement,
        Enforcement::IntervalCoverage { .. }
    ));
}
