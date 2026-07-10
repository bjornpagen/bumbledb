use super::*;
use crate::ir::normalize::{normalize, NormalizedQuery, OccId};
use crate::ir::validate::validate;
use crate::ir::{Atom, Query, Rule, Term, Value};
use crate::plan::planner::{plan, OccStats};
use crate::schema::{
    FieldDescriptor, Generation, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, ValueType,
};

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn fresh(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
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
            .map(|(f, v)| (FieldId(*f), v.clone()))
            .collect(),
    };
    StatementDescriptor::Containment {
        source: side(source),
        target: side(target),
    }
}

/// Runs the full honest pipeline: validate → normalize → chase.
fn chased(schema: &Schema, query: &Query) -> NormalizedQuery {
    let witness = validate(schema, query).expect("valid fixture query");
    let mut normalized = normalize(schema, &witness).remove(0);
    chase(&mut normalized, schema, &query.rules[0].finds);
    normalized
}

fn roles(normalized: &NormalizedQuery) -> Vec<Role> {
    normalized.occurrences.iter().map(|o| o.role).collect()
}

/// Hand-built DP stats for the participating occurrences (unit fanout).
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

/// Posting(id fresh, account u64, amount i64); Account(id fresh,
/// name str); Posting(account) <= Account(id) — statement 2 after the
/// two fresh auto-keys.
fn walk_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Posting".into(),
                fields: vec![
                    fresh("id"),
                    field("account", ValueType::U64),
                    field("amount", ValueType::I64),
                ],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![fresh("id"), field("name", ValueType::String)],
            },
        ],
        statements: vec![containment((0, &[1], &[]), (1, &[0], &[]))],
    }
    .validate()
    .expect("valid fixture")
}

/// Q(amount) :- Posting(account = x, amount), Account(id = x) — the
/// existence-walk shape: the reference target joined only to certify
/// the reference the containment already certifies.
fn walk_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(1),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    })
}

/// The existence-walk shape eliminates the reference target, and the DP
/// plans over the reduced occurrence set.
#[test]
fn existence_walk_eliminates_the_reference_target() {
    let schema = walk_schema();
    let normalized = chased(&schema, &walk_query());
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Eliminated(StatementId(2))],
        "the Account occurrence is marked with the containment that removed it"
    );
    let order = plan(&normalized, &schema, &participating_stats(&normalized));
    assert_eq!(order.order, vec![OccId(0)], "the DP saw one occurrence");
}

/// The test-only off switch bypasses the rewrite (and restores itself).
#[test]
fn the_off_switch_bypasses_the_rewrite() {
    let schema = walk_schema();
    let query = walk_query();
    let witness = validate(&schema, &query).expect("valid fixture query");
    let mut normalized = normalize(&schema, &witness).remove(0);
    with_chase_disabled(|| chase(&mut normalized, &schema, &query.rules[0].finds));
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
    chase(&mut normalized, &schema, &query.rules[0].finds);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Eliminated(StatementId(2))],
        "the switch is scoped: the same pass eliminates once re-enabled"
    );
}

/// Grading(id fresh, kind enum{Det, Custom}); Det(grading u64, rate
/// i64) with Det(grading) -> Det; the discriminated-union pair
/// `Grading(id | kind == Det) == Det(grading)` written as its two
/// containments — statements 2 and 3 after Grading's auto-key (0) and
/// the declared key (1).
fn du_schema() -> Schema {
    let kind = ValueType::Enum {
        variants: ["Det", "Custom"].iter().map(|v| Box::from(*v)).collect(),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Grading".into(),
                fields: vec![fresh("id"), field("kind", kind)],
            },
            RelationDescriptor {
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
            containment((0, &[0], &[(1, Value::Enum(0))]), (1, &[0], &[])),
            containment((1, &[0], &[]), (0, &[0], &[(1, Value::Enum(0))])),
        ],
    }
    .validate()
    .expect("valid fixture")
}

/// Q(rate) :- Det(grading = g, rate), Grading(id = g, kind == Det) —
/// the one-sided discriminated-union walk: the header joined only to
/// re-check the arm the `==` pair already certifies. The header falls;
/// the child stays (its `rate` is projected, and support may not be
/// circular).
#[test]
fn du_one_sided_walk_eliminates_the_header() {
    let schema = du_schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Literal(Value::Enum(0))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Eliminated(StatementId(3))],
        "the header falls to the child-to-header containment"
    );
    let order = plan(&normalized, &schema, &participating_stats(&normalized));
    assert_eq!(order.order, vec![OccId(0)], "the DP saw one occurrence");
}

/// `A(id fresh, b_ref u64)`; `B(id fresh, c_ref u64)`; `C(id fresh)`;
/// `A(b_ref) <= B(id)` (statement 3), `B(c_ref) <= C(id)` (statement 4).
fn chain_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "A".into(),
                fields: vec![fresh("id"), field("b_ref", ValueType::U64)],
            },
            RelationDescriptor {
                name: "B".into(),
                fields: vec![fresh("id"), field("c_ref", ValueType::U64)],
            },
            RelationDescriptor {
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

/// `Q(a) :- A(id = a, b_ref = x), B(id = x, c_ref = w), C(id = w)` — the
/// `A<=B<=C` chain. `B` is blocked while `C` still reads its `c_ref`
/// variable; eliminating `C` frees it — both fall only in fixpoint.
#[test]
fn a_containment_chain_eliminates_both_targets_in_fixpoint() {
    let schema = chain_schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
            Atom {
                relation: RelationId(2),
                bindings: vec![(FieldId(0), Term::Var(VarId(2)))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
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

/// Condition 1 negative — a partial-key join: D's key is (k1, k2) and
/// the containment pairs both positions, but the query joins on k1
/// alone. Uniqueness needs the whole key; refuse.
#[test]
fn a_partial_key_join_refuses() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "D".into(),
                fields: vec![
                    field("k1", ValueType::U64),
                    field("k2", ValueType::U64),
                    field("v", ValueType::I64),
                ],
            },
            RelationDescriptor {
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
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(0),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

/// Condition 2 negative — a projected non-Y field: the walk query also
/// finds Account's name, so the target still produces output. Refuse.
#[test]
fn a_projected_non_key_field_refuses() {
    let schema = walk_schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

/// Condition 2 negative — a negated atom referencing the target: the
/// anti-probe reads a variable only `B` binds, so removal would leave
/// the probe keyless. Refuse.
#[test]
fn a_negated_atom_referencing_the_target_refuses() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Posting".into(),
                fields: vec![
                    fresh("id"),
                    field("account", ValueType::U64),
                    field("amount", ValueType::I64),
                ],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![fresh("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
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
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![Atom {
            relation: RelationId(2),
            bindings: vec![(FieldId(0), Term::Var(VarId(2)))],
        }],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Positive, Role::Negated]
    );
}

/// Condition 2 negative — a membership point sourced from the target:
/// another occurrence's interval field contains a point variable only
/// `B` binds (the `PointIn` filter's `Var` source). Refuse.
#[test]
fn a_membership_point_sourced_from_the_target_refuses() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Ledger".into(),
                fields: vec![fresh("id"), field("acct", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Acct".into(),
                fields: vec![fresh("id"), field("ts", ValueType::U64)],
            },
            RelationDescriptor {
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
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
            Atom {
                relation: RelationId(2),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Positive, Role::Positive]
    );
}

/// Condition 2 negative — missing φ on the source side: the statement's
/// source selection must appear literally in the `A` occurrence's own
/// filter list (set containment, never inference). The same query with
/// the literal present is the positive control.
#[test]
fn a_missing_source_selection_refuses() {
    let kind = ValueType::Enum {
        variants: ["Det", "Custom"].iter().map(|v| Box::from(*v)).collect(),
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Grading".into(),
                fields: vec![fresh("id"), field("kind", kind)],
            },
            RelationDescriptor {
                name: "Det".into(),
                fields: vec![field("grading", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            containment((0, &[0], &[(1, Value::Enum(0))]), (1, &[0], &[])),
        ],
    }
    .validate()
    .expect("valid fixture");
    let query = |kind_filter: bool| {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                Atom {
                    relation: RelationId(0),
                    bindings: if kind_filter {
                        vec![
                            (FieldId(0), Term::Var(VarId(0))),
                            (FieldId(1), Term::Literal(Value::Enum(0))),
                        ]
                    } else {
                        vec![(FieldId(0), Term::Var(VarId(0)))]
                    },
                },
                Atom {
                    relation: RelationId(1),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                },
            ],
            negated: vec![],
            predicates: vec![],
        })
    };
    let normalized = chased(&schema, &query(false));
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Positive],
        "without kind == Det the source facts are not all in σφ"
    );
    let normalized = chased(&schema, &query(true));
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, Role::Eliminated(StatementId(2))],
        "the literal filter is exactly φ — the control eliminates"
    );
}

/// Condition 2 negative — an extra selection on the target beyond ψ:
/// the containment says nothing about `name`, so the filter must be
/// evaluated against a real fact. Refuse.
#[test]
fn an_extra_target_selection_refuses() {
    let schema = walk_schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (
                        FieldId(1),
                        Term::Literal(Value::String(Box::from(&b"joe"[..]))),
                    ),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

/// Condition 4 negative — an interval-typed pair: pointwise coverage is
/// not 1:1 fact-to-fact, so the containment proves existence of
/// covering facts, not of a joinable equal fact. Refuse (v0).
#[test]
fn an_interval_typed_pair_refuses() {
    let during = ValueType::Interval {
        element: IntervalElement::U64,
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Room".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field("during", during.clone()),
                ],
            },
            RelationDescriptor {
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
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

/// Circular support refused: a full `==` pair could certify each
/// occurrence with the other; the support forest keeps exactly one
/// standing whichever direction fires first.
#[test]
fn mutual_containments_never_eliminate_both() {
    let schema = du_schema();
    // Q(g) :- Det(grading = g), Grading(id = g, kind == Det) — neither
    // side binds anything beyond the join variable, so both directions'
    // conditions hold; support acyclicity must keep one.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: RelationId(1),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            },
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Literal(Value::Enum(0))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Eliminated(StatementId(2)), Role::Positive],
        "the first-scanned direction eliminates the child; support \
         acyclicity then refuses the header's turn"
    );
}
