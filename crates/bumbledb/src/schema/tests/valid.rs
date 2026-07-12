use super::*;

#[test]
fn valid_schema_constructs_with_statement_indices() {
    let schema = ledger_slice().validate().expect("valid schema");
    let holder = schema.relation(RelationId(0));
    // The fresh fields auto-materialized ordinary, visible Functionality
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
    // Layout: id 8 + holder 8 + status 8, dense.
    assert_eq!(account.layout().fact_width(), 24);
}

/// The materialization-order pin: two relations with one fresh
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
            // id 0: Holder's fresh auto-FD.
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            },
            // id 1: Account's fresh auto-FD.
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

/// The `docs/architecture/30-dependencies.md` example schema — Holder /
/// Account / `SavingsTerms` with its three declared statements (`==` lowered
/// to two mirrored Containments) plus the fresh auto-keys — validates,
/// with every statement's `Resolved` exact. The mirrored pair (ids 3 and 4)
/// pins independent per-direction resolution, and id 3 resolves a key
/// declared *after* it (forward reference).
#[test]
fn example_schema_resolves_exactly() {
    let savings = Value::U64(1); // kind 1 = Savings
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
            &scalar_key, // id 0: Holder(id), fresh auto-key
            &scalar_key, // id 1: Account(id), fresh auto-key
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
                extension: None,
                name: "Booking".into(),
                fields: vec![field("room", ValueType::U64), field("during", iv.clone())],
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

/// Currency { `minor_units`: u64 } = { Usd(2), Eur(2) } — the closed
/// fixture shared by the valid-side tests.
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
    assert!(relation.is_closed());
    // The synthetic id field opens the sealed list; declared columns
    // shift by one — guards, statements, and queries address FieldId(0)
    // uniformly.
    assert_eq!(relation.fields()[0].name.as_ref(), "id");
    assert_eq!(relation.fields()[0].value_type, ValueType::U64);
    assert_eq!(relation.fields()[0].generation, Generation::None);
    assert_eq!(relation.fields()[1].name.as_ref(), "minor_units");
    assert_eq!(relation.layout().fact_width(), 16);
    // Rows sealed as full canonical fact bytes (id ‖ values), encoded
    // once at validate and never again.
    let rows = relation.extension().expect("closed");
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
    // The closed auto-key materialized: `Currency(id) -> Currency`.
    assert_eq!(relation.keys(), &[StatementId(0)]);
}

/// The materialization-order pin, closed arm: ALL fresh auto-FDs first,
/// then closed auto-keys (relation declaration order), then declared
/// statements — the order is a fingerprint input, pinned by PRD 01 and
/// never revisited. Holder declares AFTER Currency so the fresh/closed
/// grouping (not relation order) is what the assertion pins; the declared
/// containment also proves the closed auto-key targetable like any key.
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
            // id 0: Holder's fresh auto-FD — fresh first, though Holder
            // declares second.
            fd(RelationId(1), &[FieldId(0)]),
            // id 1: Currency's closed auto-key on the synthetic id.
            fd(RelationId(0), &[FieldId(0)]),
            // id 2: the declared containment.
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ]
    );
    let schema = decl.validate().expect("valid");
    // The containment compiles to the answer set itself — no key search,
    // no permutation: Currency's two rows, both unselected survivors
    // (`docs/prd-comptime/04-compiled-subsets.md`).
    assert_eq!(
        schema.statement(StatementId(2)).resolved,
        Resolved::ClosedContainment {
            members: [0b11, 0, 0, 0]
        }
    );
    // No dependents ride the closed auto-key: the target side is vacuous
    // by construction (axioms never delete), so no R traffic exists for
    // the statement class.
    assert_eq!(schema.dependents(StatementId(1)), &[]);
}

/// PRD 04's validate-time criterion: the member set is computed at
/// validate — construct the schema, read `Resolved` directly, assert
/// bits, no Db anywhere. ψ (`pages == true`) selects the sub-vocabulary
/// {Med, High} = rows 1 and 2 = 0b110.
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
    // Statement 0 is Severity's closed auto-key; 1 the declared statement.
    assert_eq!(
        schema.statement(StatementId(1)).resolved,
        Resolved::ClosedContainment {
            members: [0b110, 0, 0, 0]
        }
    );
}

/// A closed→closed containment the axioms satisfy validates: both sides
/// constant, the judgment decided at declaration, nothing left for any
/// commit to do.
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
