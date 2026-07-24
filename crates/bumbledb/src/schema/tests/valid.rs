use super::*;
use crate::error::StatementErrorKind;

fn member_set(indices: &[u16]) -> MemberSet {
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
    // The fresh fields auto-materialized ordinary, visible Functionality
    // statements; the declared Containment follows them.
    assert_eq!(holder.keys(), &[KeyId(0)]);
    assert_eq!(holder.outgoing(), &[]);
    // ...and Holder's key is the declared Containment's resolved target —
    // the target-side reverse-edge check set.
    assert_eq!(schema.dependents(KeyId(0)), &[ContainmentId(0)]);

    let account = schema.relation(RelationId(1));
    assert_eq!(account.keys(), &[KeyId(1)]);
    assert_eq!(account.outgoing(), &[ContainmentId(0)]);
    assert_eq!(schema.dependents(KeyId(1)), &[]);
    // Layout: id 8 + holder 8 + status 8, dense.
    assert_eq!(account.layout().fact_width(), 24);
}

#[test]
fn a_redundant_pointwise_superkey_seals_with_a_warning() {
    let mut descriptor = one_relation(vec![
        field("id", ValueType::U64),
        field(
            "span",
            ValueType::Interval {
                element: IntervalElement::I64,
                width: None,
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
    assert!(!schema.key(KeyId(0)).pointwise());
    assert!(schema.key(KeyId(1)).pointwise());
    assert_eq!(
        schema.warnings(),
        &[SchemaWarning::RedundantSuperkey {
            relation: RelationId(0),
            key: KeyId(1),
            implied_by: KeyId(0),
        }]
    );
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
    // The sealed schema preserves the same materialized identity spine.
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

/// The `docs/architecture/30-dependencies.md` example schema — Holder /
/// Account / `SavingsTerms` with its three declared statements (`==` lowered
/// to two mirrored Containments) plus the fresh auto-keys — validates,
/// with every typed statement's enforcement exact. The mirrored pair (ids 3 and 4)
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
                            width: None,
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

    assert!(schema.keys().iter().all(|key| !key.pointwise()));
    let probe = |target_key: u16| Enforcement::ScalarProbe {
        target_key: KeyId(target_key),
        key_permutation: Box::new([0]),
    };
    assert_eq!(
        schema
            .containments()
            .iter()
            .map(|statement| &statement.enforcement)
            .collect::<Vec<_>>(),
        vec![
            &probe(0), // id 2: Account(holder) <= Holder(id)
            &probe(2), // id 3: Account(id | Savings) <= SavingsTerms(account)
            &probe(1), // id 4: SavingsTerms(account) <= Account(id | Savings)
        ]
    );

    // The sealed `==` pairing: the lowered pair links symmetrically;
    // The one-way containment (id 2) carries `None`; keys have no mirror
    // field at all.
    let mirrors: Vec<Option<StatementId>> = schema
        .containments()
        .iter()
        .map(|statement| statement.mirror)
        .collect();
    assert_eq!(
        mirrors,
        vec![None, Some(StatementId(4)), Some(StatementId(3)),]
    );

    // The target_key -> dependents reverse index (the target-side
    // reverse-edge check set).
    assert_eq!(schema.dependents(KeyId(0)), &[ContainmentId(0)]);
    assert_eq!(schema.dependents(KeyId(1)), &[ContainmentId(2)]);
    assert_eq!(schema.dependents(KeyId(2)), &[ContainmentId(1)]);
}

/// Pointwise resolution: an interval key records its interval position, and
/// an interval containment resolves to it with the shared position.
#[test]
fn pointwise_key_and_containment_resolve() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
        width: None,
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

    assert!(schema.key(KeyId(0)).pointwise());
    assert!(matches!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::IntervalCoverage {
            target_key: KeyId(0),
            ref key_permutation,
            ..
        } if **key_permutation == [0, 1]
    ));
    assert_eq!(schema.dependents(KeyId(0)), &[ContainmentId(0)]);
}

/// The target projection may be any permutation of the key: the recorded
/// permutation is the INVERSE form — determinant position → statement
/// projection index — so the per-fact encoder is a straight indexed
/// gather (`keys::permuted_determinant_image`).
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
            fd(RelationId(0), &[FieldId(0), FieldId(1)]), // determinant order (a, b)
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
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::ScalarProbe {
            target_key: KeyId(0),
            key_permutation: Box::new([1, 0]),
        }
    );
}

/// The inverse direction pinned by a 3-cycle (a 2-field permutation is an
/// involution — identical under either convention): determinant order
/// (a, b, c) against projection order (c, a, b) stores `[1, 2, 0]`, the
/// projection index per determinant position.
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
            fd(RelationId(0), &[FieldId(0), FieldId(1), FieldId(2)]), // (a, b, c)
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
            key_permutation: Box::new([1, 2, 0]),
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
    // shift by one — determinants, statements, and queries address FieldId(0)
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
    assert_eq!(relation.keys(), &[KeyId(0)]);
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
    // (`docs/architecture/30-dependencies.md`).
    assert_eq!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::Closed {
            members: member_set(&[0, 1])
        }
    );
    // No dependents ride the closed auto-key: the target side is vacuous
    // by construction (axioms never delete), so no R traffic exists for
    // the statement class.
    assert_eq!(schema.dependents(KeyId(1)), &[]);
}

/// PRD 04's validate-time criterion: the member set is computed at
/// validate — construct the schema, read `Enforcement` directly, assert
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
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::Closed {
            members: member_set(&[1, 2])
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

/// The task/parent fixture the extension-form tests share: Parent(id key)
/// and Task(parent, pos, prio, flag) — the window's target key plus
/// selectable payloads.
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

/// `Parent(id) <={1..3} Task(parent)` seals into the window arena with
/// the containment target-key rule reused — the acceptance premise of
/// `lean/Bumbledb/Admission.lean: cardinalityForm`, and the plan the gate
/// promises is `lean/Bumbledb/Oracle.lean: cardinality_plan_decides`.
#[test]
fn a_cardinality_window_over_a_declared_key_validates() {
    let mut decl = task_tree();
    decl.statements.push(cardinality(
        side(RelationId(1), &[FieldId(0)]),
        1,
        Some(3),
        side(RelationId(0), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("the window passes the gate");
    assert_eq!(schema.windows().len(), 1);
    let window = schema.window(WindowId(0));
    assert_eq!(window.id, StatementId(1));
    assert_eq!((window.lo, window.hi), (1, Some(3)));
    // The spine resolves the materialized id to the typed arena arm.
    assert!(matches!(
        schema.statement(StatementId(1)),
        StatementView::Cardinality(WindowId(0), _)
    ));
}

/// `{2..*}` — `hi = None` is the `*` spelling, the only spelling of "no
/// upper bound" (`lean/Bumbledb/Schema.lean: Window`). The floor starts
/// at 2: `{1..*}` is the bare containment's duplicate spelling and
/// `{0..*}` the vacuous window, both rejected (the canonical-utterance
/// law — the reject suite pins each).
#[test]
fn a_star_window_validates_with_no_ceiling() {
    let mut decl = task_tree();
    decl.statements.push(cardinality(
        side(RelationId(1), &[FieldId(0)]),
        2,
        None,
        side(RelationId(0), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("the floored window validates");
    assert_eq!(schema.window(WindowId(0)).hi, None);
}

/// `{0}` — the exclusion window: lo = hi = 0 is a legal exact count
/// (no σ-selected child may exist per parent), sealed like any window.
#[test]
fn an_exclusion_window_validates() {
    let mut decl = task_tree();
    decl.statements.push(cardinality(
        side(RelationId(1), &[FieldId(0)]),
        0,
        Some(0),
        side(RelationId(0), &[FieldId(0)]),
    ));
    let schema = decl.validate().expect("the exclusion passes the gate");
    assert_eq!(
        (schema.window(WindowId(0)).lo, schema.window(WindowId(0)).hi),
        (0, Some(0))
    );
}

/// A window into a closed target compiles the member-set plan through the
/// same key rule containments use — the closed-side mirror.
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
        statements: vec![cardinality(
            side(RelationId(1), &[FieldId(0)]),
            2,
            None,
            side(RelationId(0), &[FieldId(0)]),
        )],
    };
    decl.validate()
        .expect("every severity demands at least two handlers");
}

/// Both sides constant and the counts inside the window: decided at
/// validate, satisfied, sealed
/// (`lean/Bumbledb/Schema.lean: den_closed_constant`).
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
        statements: vec![cardinality(
            side(RelationId(0), &[FieldId(1)]),
            1,
            Some(1),
            side(RelationId(1), &[FieldId(0)]),
        )],
    };
    decl.validate()
        .expect("each severity counts exactly one kind axiom");
}

/// A literal-set σ seals — and seals CANONICALLY: the sealed side sorts
/// the set, so both spellings of one set are one statement and one
/// fingerprint (`lean/Bumbledb/Schema.lean: Selection` — the set is the
/// binding's identity, not its spelling).
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

/// Two spellings of one literal set are one statement — the duplicate
/// rule compares canonical sets, not written order.
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

/// Q1 — element-domain typing at interval positions: a fixed-width
/// interval position against a GENERAL one of the same element domain
/// matches positionally (widths free; the pointwise judgments quantify
/// over points, which carry an element domain and never a width —
/// `lean/Bumbledb/Schema.lean: Value.points_one_tag_u64`), and the
/// containment resolves the same pointwise coverage plan. This is the
/// playlist recipe's typing seam: `Slot(playlist, slot: interval<u64, 1>)
/// == Playlist(id, span: interval<u64>)`.
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
                            width: None,
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
                        ValueType::Interval {
                            element: IntervalElement::U64,
                            width: Some(1),
                        },
                    ),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0), FieldId(1)]),
            fd(RelationId(1), &[FieldId(0), FieldId(1)]),
            // The exact-partition ==, spelled as its two containments.
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
    assert!(schema.key(KeyId(0)).pointwise());
    assert!(schema.key(KeyId(1)).pointwise());
    assert!(matches!(
        schema.containment(ContainmentId(0)).enforcement,
        Enforcement::IntervalCoverage { .. }
    ));
    assert!(matches!(
        schema.containment(ContainmentId(1)).enforcement,
        Enforcement::IntervalCoverage { .. }
    ));
}
