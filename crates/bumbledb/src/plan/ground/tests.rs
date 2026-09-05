use super::*;
use crate::ir::CmpOp;
use crate::ir::normalize::{NormalizedQuery, OccId, normalize_rules};
use crate::ir::validate::validate;
use crate::ir::{Atom, Comparison, ConditionTree, Query, Rule, Term, Value};
use crate::plan::planner::{OccStats, plan};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, IntervalElement, RelationDescriptor, RelationId, SchemaDescriptor,
    StatementDescriptor, ValueType,
};

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
    }
}

fn fresh(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
    }
}

fn containment(
    source: (u32, &[u16], &[(u16, Value)]),
    target: (u32, &[u16], &[(u16, Value)]),
) -> StatementDescriptor {
    let side = |(relation, projection, selection): (u32, &[u16], &[(u16, Value)])| Side {
        relation: RelationId(relation),
        projection: projection.iter().map(|f| FieldId(*f)).collect(),
        selection: selection
            .iter()
            .map(|(f, v)| {
                (
                    FieldId(*f),
                    bumbledb_theory::schema::LiteralSet::One(v.clone()),
                )
            })
            .collect(),
    };
    StatementDescriptor::Containment {
        source: side(source),
        target: side(target),
    }
}

fn grounded(schema: &Schema, query: &Query) -> NormalizedQuery {
    let witness = validate(schema, query).expect("valid fixture query");
    let mut normalized = normalize_rules(schema, &[], witness.rules()).remove(0);
    ground(&mut normalized, schema, &query.rules()[0].finds);
    normalized
}

fn roles(normalized: &NormalizedQuery) -> Vec<Role> {
    normalized
        .occurrences
        .iter()
        .map(|o| o.role.clone())
        .collect()
}

fn participating_stats(normalized: &NormalizedQuery) -> Vec<OccStats> {
    normalized
        .occurrences
        .iter()
        .filter(|o| o.role.participates())
        .map(|o| OccStats {
            occ_id: o.occ_id,
            rows: 100,
            var_distincts: Vec::new(),
        })
        .collect()
}

/// Posting(id fresh, account u64, amount i64); Account(id fresh, name str);
/// Posting(account) <= Account(id) — statement 2 after the two fresh auto-keys.
fn walk_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    fresh("id"),
                    field("account", ValueType::U64),
                    field("amount", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![fresh("id"), field("name", ValueType::String)],
            },
        ],
        statements: vec![containment((0, &[1], &[]), (1, &[0], &[]))],
    }
    .validate()
    .expect("valid fixture")
}

fn walk_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn existence_walk_eliminates_the_reference_target() {
    let schema = walk_schema();
    let normalized = grounded(&schema, &walk_query());
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Eliminated(StatementId(2))],
        "the Account occurrence is marked with the containment that removed it"
    );
    let order = plan(&normalized, &schema, &participating_stats(&normalized));
    assert_eq!(order.order, vec![OccId(0)], "the DP saw one occurrence");
}

#[test]
fn the_off_switch_bypasses_the_rewrite() {
    let schema = walk_schema();
    let query = walk_query();
    let witness = validate(&schema, &query).expect("valid fixture query");
    let mut normalized = normalize_rules(&schema, &[], witness.rules()).remove(0);
    with_grounding_disabled(|| ground(&mut normalized, &schema, &query.rules()[0].finds));
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
    ground(&mut normalized, &schema, &query.rules()[0].finds);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Eliminated(StatementId(2))],
        "the switch is scoped: the same pass eliminates once re-enabled"
    );
}

/// Grading(id fresh, kind u64 — 0 = Det); Det(grading u64, rate i64) with
/// Det(grading) -> Det; the discriminated-union pair `Grading(id | kind == 0)
/// == Det(grading)` written as its two containments — statements 2 and 3 after
/// Grading's auto-key (0) and the declared key (1).
fn du_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Grading".into(),
                fields: vec![fresh("id"), field("kind", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Det".into(),
                fields: vec![
                    field("grading", ValueType::U64),
                    field("rate", ValueType::I64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            containment((0, &[0], &[(1, Value::U64(0))]), (1, &[0], &[])),
            containment((1, &[0], &[]), (0, &[0], &[(1, Value::U64(0))])),
        ],
    }
    .validate()
    .expect("valid fixture")
}

#[test]
fn du_one_sided_walk_eliminates_the_header() {
    let schema = du_schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Literal(Value::U64(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Eliminated(StatementId(3))],
        "the header falls to the child-to-header containment"
    );
    let order = plan(&normalized, &schema, &participating_stats(&normalized));
    assert_eq!(order.order, vec![OccId(0)], "the DP saw one occurrence");
}

fn chain_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "A".into(),
                fields: vec![fresh("id"), field("b_ref", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "B".into(),
                fields: vec![fresh("id"), field("c_ref", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "C".into(),
                fields: vec![fresh("id")],
            },
        ],
        statements: vec![
            containment((0, &[1], &[]), (1, &[0], &[])),
            containment((1, &[1], &[]), (2, &[0], &[])),
        ],
    }
    .validate()
    .expect("valid fixture")
}

#[test]
fn a_containment_chain_eliminates_both_targets_in_fixpoint() {
    let schema = chain_schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(2)),
                bindings: vec![(FieldId(0), Term::Var(VarId(2)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![
            Role::Positive,
            Role::Eliminated(StatementId(3)),
            Role::Eliminated(StatementId(4)),
        ],
        "each mark carries its own containment"
    );
    let order = plan(&normalized, &schema, &participating_stats(&normalized));
    assert_eq!(order.order, vec![OccId(0)], "the DP saw one occurrence");
}

#[test]
fn a_partial_key_join_refuses() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "D".into(),
                fields: vec![
                    field("k1", ValueType::U64),
                    field("k2", ValueType::U64),
                    field("v", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "E".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            containment((1, &[0, 1], &[]), (0, &[0, 1], &[])),
        ],
    }
    .validate()
    .expect("valid fixture");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

#[test]
fn a_projected_non_key_field_refuses() {
    let schema = walk_schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

#[test]
fn a_negated_atom_referencing_the_target_refuses() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    fresh("id"),
                    field("account", ValueType::U64),
                    field("amount", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![fresh("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                extension: None,
                name: "Blocked".into(),
                fields: vec![field("name", ValueType::String)],
            },
        ],
        statements: vec![containment((0, &[1], &[]), (1, &[0], &[]))],
    }
    .validate()
    .expect("valid fixture");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(2)),
            bindings: vec![(FieldId(0), Term::Var(VarId(2)))],
        }],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Positive, Role::Negated]
    );
}

#[test]
fn a_membership_point_sourced_from_the_target_refuses() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Ledger".into(),
                fields: vec![fresh("id"), field("acct", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Acct".into(),
                fields: vec![fresh("id"), field("ts", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Session".into(),
                fields: vec![
                    field("acct", ValueType::U64),
                    field(
                        "active",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![containment((0, &[1], &[]), (1, &[0], &[]))],
    }
    .validate()
    .expect("valid fixture");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(2)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Positive, Role::Positive]
    );
}

#[test]
fn a_missing_source_selection_refuses() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Grading".into(),
                fields: vec![fresh("id"), field("kind", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Det".into(),
                fields: vec![field("grading", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            containment((0, &[0], &[(1, Value::U64(0))]), (1, &[0], &[])),
        ],
    }
    .validate()
    .expect("valid fixture");
    let query = |kind_filter: bool| {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                Atom {
                    source: crate::ir::AtomSource::Edb(RelationId(0)),
                    bindings: if kind_filter {
                        vec![
                            (FieldId(0), Term::Var(VarId(0))),
                            (FieldId(1), Term::Literal(Value::U64(0))),
                        ]
                    } else {
                        vec![(FieldId(0), Term::Var(VarId(0)))]
                    },
                },
                Atom {
                    source: crate::ir::AtomSource::Edb(RelationId(1)),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                },
            ],
            negated: vec![],
            conditions: vec![],
        })
    };
    let normalized = grounded(&schema, &query(false));
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Positive],
        "without kind == Det the source facts are not all in σφ"
    );
    let normalized = grounded(&schema, &query(true));
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Eliminated(StatementId(2))],
        "the literal filter is exactly φ — the control eliminates"
    );
}

#[test]
fn an_extra_target_selection_refuses() {
    let schema = walk_schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Literal(Value::String(Box::from("joe")))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

#[test]
fn an_interval_typed_pair_refuses() {
    let during = ValueType::Interval {
        element: IntervalElement::U64,
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Room".into(),
                fields: vec![field("room", ValueType::U64), field("during", during)],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![field("room", ValueType::U64), field("span", during)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            containment((1, &[0, 1], &[]), (0, &[0, 1], &[])),
        ],
    }
    .validate()
    .expect("valid fixture");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

fn grounded_main(schema: &Schema, query: &Query) -> Vec<(NormalizedQuery, Vec<FindTerm>)> {
    let witness = validate(schema, query).expect("valid fixture query");
    let mut rules = normalize_rules(schema, &[], witness.rules());
    let finds: Vec<Vec<FindTerm>> = (0..rules.len())
        .map(|idx| witness.rule(idx).rule().finds.clone())
        .collect();
    for (idx, rule) in rules.iter_mut().enumerate() {
        ground(rule, schema, &finds[idx]);
    }
    rules.into_iter().zip(finds).collect()
}

fn residue_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Or(vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Gt,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::I64(30)),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Eq,
                lhs: Term::Var(VarId(2)),
                rhs: Term::Literal(Value::U64(0)),
            }),
        ])],
    })
}

#[test]
fn the_dnf_residue_subsumes_the_filtered_rule() {
    let schema = du_schema();
    let (rules, finds): (Vec<_>, Vec<_>) =
        grounded_main(&schema, &residue_query()).into_iter().unzip();
    assert_eq!(rules.len(), 2, "two disjuncts lower to two rules");
    for rule in &rules {
        assert_eq!(
            roles(rule),
            vec![Role::Positive, Role::Eliminated(StatementId(3))],
            "the grounding runs per rule and eliminates Grading in each"
        );
    }
    let finds: Vec<&[FindTerm]> = finds.iter().map(Vec::as_slice).collect();
    assert_eq!(
        subsume(&rules, &finds),
        vec![Subsumption { rule: 0, by: 1 }],
        "the filterless disjunct subsumes the rate-filtered one"
    );
}

#[test]
fn the_off_switch_covers_subsumption() {
    let schema = du_schema();
    let (rules, finds): (Vec<_>, Vec<_>) =
        grounded_main(&schema, &residue_query()).into_iter().unzip();
    let finds: Vec<&[FindTerm]> = finds.iter().map(Vec::as_slice).collect();
    assert!(
        with_grounding_disabled(|| subsume(&rules, &finds)).is_empty(),
        "the switch bypasses subsumption"
    );
    assert_eq!(
        subsume(&rules, &finds),
        vec![Subsumption { rule: 0, by: 1 }],
        "the switch is scoped: the same pass deletes once re-enabled"
    );
}

#[test]
fn distinct_bodies_refuse_subsumption() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Grading".into(),
                fields: vec![fresh("id"), field("kind", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Det".into(),
                fields: vec![
                    field("grading", ValueType::U64),
                    field("rate", ValueType::I64),
                ],
            },
        ],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(1),
            projection: Box::new([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid fixture");
    let (rules, finds): (Vec<_>, Vec<_>) =
        grounded_main(&schema, &residue_query()).into_iter().unzip();
    for rule in &rules {
        assert_eq!(roles(rule), vec![Role::Positive, Role::Positive]);
    }
    let finds: Vec<&[FindTerm]> = finds.iter().map(Vec::as_slice).collect();
    assert!(
        subsume(&rules, &finds).is_empty(),
        "differing surviving filters refuse both directions"
    );
}

/// Circular support refused: a full `==` pair could certify each occurrence
/// with the other; the support forest keeps exactly one standing whichever
/// direction fires first.
#[test]
fn mutual_containments_never_eliminate_both() {
    let schema = du_schema();

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Literal(Value::U64(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Eliminated(StatementId(2)), Role::Positive],
        "the first-scanned direction eliminates the child; support \
         acyclicity then refuses the header's turn"
    );
}
