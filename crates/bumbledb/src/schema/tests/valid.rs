use super::*;

#[test]
fn valid_schema_constructs_with_statement_indices() {
    let schema = ledger_slice().validate().expect("valid schema");
    let holder = schema.relation(RelationId(0));
    // The serial fields auto-materialized ordinary, visible Functionality
    // statements; the declared Containment follows them.
    assert_eq!(holder.keys(), &[StatementId(0)]);
    assert_eq!(holder.outgoing(), &[]);
    // ...and Holder's key is the declared Containment's resolved target —
    // the target-side reverse-edge check set.
    assert_eq!(schema.dependents(StatementId(0)), &[StatementId(2)]);

    let account = schema.relation(RelationId(1));
    assert_eq!(account.keys(), &[StatementId(1)]);
    assert_eq!(account.outgoing(), &[StatementId(2)]);
    assert_eq!(schema.dependents(StatementId(1)), &[]);
    // Layout: id 8 + holder 8 + status 1, dense.
    assert_eq!(account.layout().fact_width(), 17);
}

/// The materialization-order pin: two relations with one serial
/// field each plus two declared statements — auto-FDs take ids 0 and 1
/// (relation declaration order, then field order), declared statements
/// take 2 and 3 (declaration order).
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
            // id 0: Holder's serial auto-FD.
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            },
            // id 1: Account's serial auto-FD.
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            // id 2: the declared Containment.
            StatementDescriptor::Containment {
                source: side(RelationId(1), &[FieldId(1)]),
                target: side(RelationId(0), &[FieldId(0)]),
            },
            // id 3: the declared Functionality.
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(1), FieldId(2)]),
            },
        ]
    );
    // The sealed schema holds the same list; StatementId = index into it.
    let schema = decl.validate().expect("valid schema");
    let sealed: Vec<&StatementDescriptor> =
        schema.statements().iter().map(|s| &s.descriptor).collect();
    assert_eq!(sealed, materialized.iter().collect::<Vec<_>>());
    assert_eq!(
        schema.relation(RelationId(1)).keys(),
        &[StatementId(1), StatementId(3)]
    );
}

#[test]
fn structural_enum_equality_is_the_identity() {
    // Same ordered variant list, different declaring contexts: equal type.
    assert_eq!(enum_type(&["A", "B"]), enum_type(&["A", "B"]));
    // Different order: different type (ordinal encoding differs).
    assert_ne!(enum_type(&["A", "B"]), enum_type(&["B", "A"]));
}

#[test]
fn nullary_relation_constructs() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Flag".into(),
            fields: vec![],
        }],
        statements: vec![],
    }
    .validate()
    .expect("nullary relations are legal");
    assert_eq!(schema.relation(RelationId(0)).layout().fact_width(), 0);
}

/// The `docs/architecture/30-dependencies.md` example schema — Holder /
/// Account / `SavingsTerms` with its three declared statements (`==` lowered
/// to two mirrored Containments) plus the serial auto-keys — validates,
/// with every statement's `Resolved` exact. The mirrored pair (ids 3 and 4)
/// pins independent per-direction resolution, and id 3 resolves a key
/// declared *after* it (forward reference).
#[test]
fn example_schema_resolves_exactly() {
    let savings = Value::Enum(1); // ["Checking", "Savings"]
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![serial_field("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    serial_field("id"),
                    field("holder", ValueType::U64),
                    field("kind", enum_type(&["Checking", "Savings"])),
                    field(
                        "active",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                name: "SavingsTerms".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field("rate_bps", ValueType::I64),
                ],
            },
        ],
        statements: vec![
            // Account(holder) <= Holder(id)
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
            // Account(id | kind == Savings) == SavingsTerms(account), lowered:
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
            // SavingsTerms(account) -> SavingsTerms
            fd(RelationId(2), &[FieldId(0)]),
        ],
    }
    .validate()
    .expect("the 30-dependencies example schema is valid");

    let resolved: Vec<&Resolved> = schema.statements().iter().map(|s| &s.resolved).collect();
    let scalar_key = Resolved::Functionality {
        interval_position: None,
    };
    let probe = |target_key: u16| Resolved::Containment {
        target_key: StatementId(target_key),
        key_permutation: Box::new([0]),
        interval_position: None,
    };
    assert_eq!(
        resolved,
        vec![
            &scalar_key, // id 0: Holder(id), serial auto-key
            &scalar_key, // id 1: Account(id), serial auto-key
            &probe(0),   // id 2: Account(holder) <= Holder(id)
            &probe(5),   // id 3: Account(id | Savings) <= SavingsTerms(account)
            &probe(1),   // id 4: SavingsTerms(account) <= Account(id | Savings)
            &scalar_key, // id 5: SavingsTerms(account) -> SavingsTerms
        ]
    );

    // The sealed `==` pairing: the lowered pair links symmetrically;
    // every FD and the one-way containment (id 2) carry `None`.
    let mirrors: Vec<Option<StatementId>> = schema.statements().iter().map(|s| s.mirror).collect();
    assert_eq!(
        mirrors,
        vec![
            None,
            None,
            None,
            Some(StatementId(4)),
            Some(StatementId(3)),
            None
        ]
    );

    // The target_key -> dependents reverse index (the target-side
    // reverse-edge check set).
    assert_eq!(schema.dependents(StatementId(0)), &[StatementId(2)]);
    assert_eq!(schema.dependents(StatementId(1)), &[StatementId(4)]);
    assert_eq!(schema.dependents(StatementId(5)), &[StatementId(3)]);
    for id in [2, 3, 4] {
        assert_eq!(schema.dependents(StatementId(id)), &[]);
    }
}

/// Pointwise resolution: an interval key records its interval position, and
/// an interval containment resolves to it with the shared position.
#[test]
fn pointwise_key_and_containment_resolve() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Booking".into(),
                fields: vec![field("room", ValueType::U64), field("during", iv.clone())],
            },
            RelationDescriptor {
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

    assert_eq!(
        schema.statement(StatementId(0)).resolved,
        Resolved::Functionality {
            interval_position: Some(1)
        }
    );
    assert_eq!(
        schema.statement(StatementId(1)).resolved,
        Resolved::Containment {
            target_key: StatementId(0),
            key_permutation: Box::new([0, 1]),
            interval_position: Some(1)
        }
    );
    assert_eq!(schema.dependents(StatementId(0)), &[StatementId(1)]);
}

/// The target projection may be any permutation of the key: the recorded
/// permutation maps statement projection order to the key's guard order.
#[test]
fn permuted_target_projection_resolves_with_permutation() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "T".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::I64)],
            },
            RelationDescriptor {
                name: "S".into(),
                fields: vec![field("x", ValueType::I64), field("y", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0), FieldId(1)]), // guard order (a, b)
            // S(x, y) <= T(b, a): projected against the key permuted.
            containment(
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
                side(RelationId(0), &[FieldId(1), FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("a permuted target projection resolves");

    assert_eq!(
        schema.statement(StatementId(1)).resolved,
        Resolved::Containment {
            target_key: StatementId(0),
            key_permutation: Box::new([1, 0]),
            interval_position: None
        }
    );
}

#[test]
fn accepts_enum_with_exactly_256_variants() {
    let names: Vec<String> = (0..256).map(|i| format!("V{i}")).collect();
    let decl = one_relation(vec![field(
        "e",
        enum_type(&names.iter().map(String::as_str).collect::<Vec<_>>()),
    )]);
    decl.validate().expect("256 variants fit one byte");
}
