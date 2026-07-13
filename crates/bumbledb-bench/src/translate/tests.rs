use std::sync::OnceLock;

use super::*;
use crate::fixture::{field, fresh, var};
use bumbledb::ir::{Atom, CmpOp, Comparison, FindTerm, MaskTerm, PredicateTree, Rule, Term};
use bumbledb::schema::{IntervalElement, RelationDescriptor, SchemaDescriptor, Side, ValueType};
use bumbledb::AggOp;
use bumbledb::AllenMask;

/// Relation and field ids for the test ledger below — declaration order
/// is the id order, no magic numbers in query constructions.
mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const HOLDER: RelationId = RelationId(0);
    pub const ACCOUNT: RelationId = RelationId(1);
    pub const INSTRUMENT: RelationId = RelationId(2);
    pub const POSTING: RelationId = RelationId(4);
    pub const POSTING_TAG: RelationId = RelationId(5);
    pub const ORG_PARENT: RelationId = RelationId(7);
    pub const MANDATE: RelationId = RelationId(8);
    pub const TRANSFER: RelationId = RelationId(9);

    pub mod holder {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const NAME: FieldId = FieldId(1);
    }
    pub mod account {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const HOLDER: FieldId = FieldId(1);
    }
    pub mod instrument {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const SYMBOL: FieldId = FieldId(1);
    }
    pub mod posting {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const ENTRY: FieldId = FieldId(1);
        pub const ACCOUNT: FieldId = FieldId(2);
        pub const INSTRUMENT: FieldId = FieldId(3);
        pub const AMOUNT: FieldId = FieldId(4);
        pub const AT: FieldId = FieldId(5);
    }
    pub mod posting_tag {
        use super::FieldId;
        pub const POSTING: FieldId = FieldId(0);
        pub const TAG: FieldId = FieldId(1);
    }
    pub mod org_parent {
        use super::FieldId;
        pub const CHILD: FieldId = FieldId(0);
        pub const PARENT: FieldId = FieldId(1);
    }
    pub mod mandate {
        use super::FieldId;
        pub const ACCOUNT: FieldId = FieldId(0);
        pub const ORG: FieldId = FieldId(1);
        pub const ACTIVE: FieldId = FieldId(2);
    }
    pub mod transfer {
        use super::FieldId;
        pub const EXTREF: FieldId = FieldId(1);
    }
}

/// The test ledger: the benchmark schema of
/// `docs/architecture/60-validation.md`,
/// plus `Transfer` for a Bytes field — built locally so the translator's
/// goldens depend on nothing but the IR and the schema descriptors.
fn schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Holder".into(),
                    fields: vec![fresh("id"), field("name", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Account".into(),
                    fields: vec![
                        fresh("id"),
                        field("holder", ValueType::U64),
                        field("currency", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Instrument".into(),
                    fields: vec![fresh("id"), field("symbol", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "JournalEntry".into(),
                    fields: vec![
                        fresh("id"),
                        field("source", ValueType::U64),
                        field("created_at", ValueType::I64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Posting".into(),
                    fields: vec![
                        fresh("id"),
                        field("entry", ValueType::U64),
                        field("account", ValueType::U64),
                        field("instrument", ValueType::U64),
                        field("amount", ValueType::I64),
                        field("at", ValueType::I64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "PostingTag".into(),
                    fields: vec![
                        field("posting", ValueType::U64),
                        field("tag", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Org".into(),
                    fields: vec![fresh("id"), field("name", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "OrgParent".into(),
                    fields: vec![
                        field("child", ValueType::U64),
                        field("parent", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Mandate".into(),
                    fields: vec![
                        field("account", ValueType::U64),
                        field("org", ValueType::U64),
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
                    name: "Transfer".into(),
                    fields: vec![
                        fresh("id"),
                        field("extref", ValueType::FixedBytes { len: 32 }),
                    ],
                },
            ],
            statements: vec![],
        }
        .validate()
        .expect("the test ledger validates")
    })
}

#[test]
fn point_matches_its_hand_written_golden() {
    // Q(amount, at) :- Posting(id = ?0, amount, at).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, Term::Param(ParamId(0))),
                (ids::posting::AMOUNT, var(0)),
                (ids::posting::AT, var(1)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::POINT);
    assert_eq!(t.params, vec![ParamSlot::Whole(ParamId(0))]);
}

#[test]
fn containment_walk_matches_its_hand_written_golden() {
    // Q(name, amount) :- Posting(account = ?0, amount),
    //                    Account(id = ?0, holder = h),
    //                    Holder(id = h, name).
    // (The account is pinned by the same param on both sides — the
    // join predicate through ?1 twice, param reused.)
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, Term::Param(ParamId(0))),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![
                    (ids::account::ID, Term::Param(ParamId(0))),
                    (ids::account::HOLDER, var(2)),
                ],
            },
            Atom {
                relation: ids::HOLDER,
                bindings: vec![(ids::holder::ID, var(2)), (ids::holder::NAME, var(0))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::CONTAINMENT_WALK);
    assert_eq!(
        t.params,
        vec![ParamSlot::Whole(ParamId(0))],
        "one placeholder, reused"
    );
}

#[test]
fn balance_matches_its_hand_written_golden() {
    // Q(a, Sum(amount)) :- Posting(id, account = a, amount),
    //                      Account(id = a, holder = ?0).
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ID, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![
                    (ids::account::ID, var(0)),
                    (ids::account::HOLDER, Term::Param(ParamId(0))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::BALANCE);
}

#[test]
fn negated_atoms_match_their_goldens() {
    // no_tag: Q(p) :- Posting(id = p), ¬PostingTag(posting = p, tag = Fee).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::ID, var(0))],
        }],
        negated: vec![Atom {
            relation: ids::POSTING_TAG,
            bindings: vec![
                (ids::posting_tag::POSTING, var(0)),
                (ids::posting_tag::TAG, Term::Literal(Value::U64(0))),
            ],
        }],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::NO_TAG);
    assert!(t.params.is_empty());

    // self_negation: Q(c) :- OrgParent(child = c, parent = p),
    //                        ¬OrgParent(child = p).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::ORG_PARENT,
            bindings: vec![
                (ids::org_parent::CHILD, var(0)),
                (ids::org_parent::PARENT, var(1)),
            ],
        }],
        negated: vec![Atom {
            relation: ids::ORG_PARENT,
            bindings: vec![(ids::org_parent::CHILD, var(1))],
        }],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::SELF_NEGATION);

    // A param inside a negated atom still binds positionally.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::ID, var(0))],
        }],
        negated: vec![Atom {
            relation: ids::POSTING_TAG,
            bindings: vec![
                (ids::posting_tag::POSTING, var(0)),
                (ids::posting_tag::TAG, Term::Param(ParamId(0))),
            ],
        }],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert!(t.sql.contains("n0.\"tag\" = ?1"), "{}", t.sql);
    assert_eq!(t.params, vec![ParamSlot::Whole(ParamId(0))]);
}

#[test]
fn param_sets_render_as_literal_in_lists() {
    // in_three / in_empty: Q(e) :- Posting(entry = e, account ∈ ?set0).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ENTRY, var(0)),
                (ids::posting::ACCOUNT, Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let sets = vec![(
        ParamId(0),
        vec![Value::U64(3), Value::U64(7), Value::U64(9)],
    )];
    let t = translate(&query, schema(), &sets).expect("translates");
    assert_eq!(t.sql, goldens::IN_THREE);
    assert!(
        t.params.is_empty(),
        "set elements are literals, not placeholders"
    );

    let empty = vec![(ParamId(0), Vec::new())];
    let t = translate(&query, schema(), &empty).expect("translates");
    assert_eq!(t.sql, goldens::IN_EMPTY);

    // An unbound set is a named error, never silently-empty SQL.
    let err = translate(&query, schema(), &[]).unwrap_err();
    assert!(err.contains("param set 0"), "{err}");

    // A set inside a negated atom takes the same IN form.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::ACCOUNT,
            bindings: vec![(ids::account::ID, var(0))],
        }],
        negated: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::ENTRY, Term::ParamSet(ParamId(0))),
            ],
        }],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &sets).expect("translates");
    assert!(
        t.sql.contains(
            "NOT EXISTS (SELECT 1 FROM \"Posting\" AS n0 WHERE n0.\"account\" = t0.\"id\" AND n0.\"entry\" IN (3, 7, 9))"
        ),
        "{}",
        t.sql
    );
}

#[test]
fn set_forms_cover_interval_membership_and_predicate_equality() {
    // Membership per element on an interval field: an OR of endpoint
    // tests (IN has no interval form); the empty set is 1 = 0 here too.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::MANDATE,
            bindings: vec![
                (ids::mandate::ORG, var(0)),
                (ids::mandate::ACTIVE, Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let sets = vec![(ParamId(0), vec![Value::I64(1), Value::I64(2)])];
    let t = translate(&query, schema(), &sets).expect("translates");
    assert!(
        t.sql.contains(
            "(t0.\"active_start\" <= 1 AND 1 < t0.\"active_end\" OR t0.\"active_start\" <= 2 AND 2 < t0.\"active_end\")"
        ),
        "{}",
        t.sql
    );
    let empty = vec![(ParamId(0), Vec::new())];
    let t = translate(&query, schema(), &empty).expect("translates");
    assert!(t.sql.ends_with("WHERE 1 = 0"), "{}", t.sql);

    // Eq against a set in a predicate: the variable side's IN.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::ENTRY, var(1)),
            ],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(1),
            rhs: Term::ParamSet(ParamId(0)),
        })],
    });
    let sets = vec![(ParamId(0), vec![Value::U64(3), Value::U64(7)])];
    let t = translate(&query, schema(), &sets).expect("translates");
    assert!(t.sql.contains("t0.\"entry\" IN (3, 7)"), "{}", t.sql);
}

#[test]
fn membership_matches_its_goldens() {
    // Q(o) :- Posting(account = a, at = t),
    //         Mandate(account = a, org = o, active ∋ t).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![(ids::posting::ACCOUNT, var(1)), (ids::posting::AT, var(2))],
            },
            Atom {
                relation: ids::MANDATE,
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(1)),
                    (ids::mandate::ORG, var(0)),
                    (ids::mandate::ACTIVE, var(2)),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::MEMBERSHIP);

    // The param form: Q(o) :- Posting(account = ?0, at = ?1),
    //                         Mandate(account = ?0, org = o, active ∋ ?1).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, Term::Param(ParamId(0))),
                    (ids::posting::AT, Term::Param(ParamId(1))),
                ],
            },
            Atom {
                relation: ids::MANDATE,
                bindings: vec![
                    (ids::mandate::ACCOUNT, Term::Param(ParamId(0))),
                    (ids::mandate::ORG, var(0)),
                    (ids::mandate::ACTIVE, Term::Param(ParamId(1))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::MEMBERSHIP_PARAM);
    assert_eq!(
        t.params,
        vec![ParamSlot::Whole(ParamId(0)), ParamSlot::Whole(ParamId(1))],
        "the instant's placeholder repeats; one bound value"
    );
}

#[test]
fn allen_intersects_matches_its_hand_written_golden() {
    // Q(o1, o2) :- Mandate(account = a, org = o1, active = u),
    //              Mandate(account = a, org = o2, active = v),
    //              Allen(u, v, INTERSECTS).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::MANDATE,
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(2)),
                    (ids::mandate::ORG, var(0)),
                    (ids::mandate::ACTIVE, var(3)),
                ],
            },
            Atom {
                relation: ids::MANDATE,
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(2)),
                    (ids::mandate::ORG, var(1)),
                    (ids::mandate::ACTIVE, var(4)),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: var(3),
            rhs: var(4),
        })],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::INTERSECTS);
}

#[test]
fn contains_matches_both_goldens() {
    // The ⊇ composite against an interval param:
    // Q(o) :- Mandate(org = o, active = v), Allen(v, ?0, COVERS).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::MANDATE,
            bindings: vec![(ids::mandate::ORG, var(0)), (ids::mandate::ACTIVE, var(1))],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::COVERS),
            },
            lhs: var(1),
            rhs: Term::Param(ParamId(0)),
        })],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::COVERS_PARAM);
    assert_eq!(
        t.params,
        vec![ParamSlot::Start(ParamId(0)), ParamSlot::End(ParamId(0))],
        "an interval param binds its two halves"
    );

    // Point containment: Q(o, t) :- Mandate(org = o, active = v),
    //                               Posting(at = t), Contains(v, t).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                relation: ids::MANDATE,
                bindings: vec![(ids::mandate::ORG, var(0)), (ids::mandate::ACTIVE, var(1))],
            },
            Atom {
                relation: ids::POSTING,
                bindings: vec![(ids::posting::AT, var(2))],
            },
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Contains,
            lhs: var(1),
            rhs: var(2),
        })],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::CONTAINS_POINT);
}

#[test]
fn interval_equality_matches_its_goldens() {
    // Predicate form: Q(a1, a2) :- Mandate(account = a1, active = u),
    //                              Mandate(account = a2, active = v),
    //                              Eq(u, v).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::MANDATE,
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(0)),
                    (ids::mandate::ACTIVE, var(2)),
                ],
            },
            Atom {
                relation: ids::MANDATE,
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(1)),
                    (ids::mandate::ACTIVE, var(3)),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(2),
            rhs: var(3),
        })],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::INTERVAL_EQ);

    // Binding form, literal: Q(o) :- Mandate(org = o, active = [1700, 1800)).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::MANDATE,
            bindings: vec![
                (ids::mandate::ORG, var(0)),
                (
                    ids::mandate::ACTIVE,
                    Term::Literal(Value::IntervalI64(1700, 1800)),
                ),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::INTERVAL_EQ_LITERAL);

    // Binding form, param: Q(o) :- Mandate(org = o, active = ?0) — the
    // bivalent anchor resolves to the interval reading, two placeholders.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::MANDATE,
            bindings: vec![
                (ids::mandate::ORG, var(0)),
                (ids::mandate::ACTIVE, Term::Param(ParamId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::INTERVAL_EQ_PARAM);
    assert_eq!(
        t.params,
        vec![ParamSlot::Start(ParamId(0)), ParamSlot::End(ParamId(0))]
    );
}

#[test]
fn count_distinct_matches_its_hand_written_golden() {
    // Q(h, CountDistinct(i)) :- Account(id = a, holder = h),
    //                           Posting(account = a, instrument = i).
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::CountDistinct,
                over: Some(VarId(2)),
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![(ids::account::ID, var(1)), (ids::account::HOLDER, var(0))],
            },
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, var(1)),
                    (ids::posting::INSTRUMENT, var(2)),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::COUNT_DISTINCT);
}

#[test]
fn count_distinct_over_an_interval_concatenates_the_halves() {
    // Q(CountDistinct(u)) :- Mandate(account = a, active = u): the halves
    // fold through an injective decimal rendering (COUNT(DISTINCT ...)
    // takes one expression).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::CountDistinct,
            over: Some(VarId(1)),
        }],
        atoms: vec![Atom {
            relation: ids::MANDATE,
            bindings: vec![
                (ids::mandate::ACCOUNT, var(0)),
                (ids::mandate::ACTIVE, var(1)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert!(
        t.sql.contains("COUNT(DISTINCT v1_start || ',' || v1_end)"),
        "{}",
        t.sql
    );
    assert!(t.sql.ends_with("HAVING COUNT(*) > 0"), "{}", t.sql);
}

#[test]
fn arg_restriction_matches_its_goldens() {
    // Grouped: Q(a, ArgMax_at(p)) :- Posting(id = p, account = a, at = t).
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(2) },
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, var(1)),
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::AT, var(2)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::ARG_MAX);

    // Global: Q(ArgMax_at(p)) :- Posting(id = p, at = t).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::ArgMax { key: VarId(1) },
            over: Some(VarId(0)),
        }],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::ID, var(0)), (ids::posting::AT, var(1))],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::ARG_MAX_GLOBAL);

    // ArgMin swaps the extreme.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::ArgMin { key: VarId(1) },
            over: Some(VarId(0)),
        }],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::ID, var(0)), (ids::posting::AT, var(1))],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert!(t.sql.contains("SELECT MIN(v1) AS mk FROM d"), "{}", t.sql);
}

#[test]
fn an_interval_find_projects_both_halves() {
    // Q(o, u) :- Mandate(org = o, active = u): the decode path
    // reassembles the value from the pair (`crate::sqlmap`).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::MANDATE,
            bindings: vec![(ids::mandate::ORG, var(0)), (ids::mandate::ACTIVE, var(1))],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(
        t.sql,
        "SELECT DISTINCT t0.\"org\", t0.\"active_start\", t0.\"active_end\" FROM \"Mandate\" AS t0"
    );
}

#[test]
fn every_scalar_construct_translates() {
    // Gate atom → EXISTS; literal escaping (string and bytes); same-atom
    // comparisons; every scalar operator.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![(ids::posting::AMOUNT, var(0)), (ids::posting::AT, var(1))],
            },
            Atom {
                relation: ids::INSTRUMENT,
                bindings: vec![(
                    ids::instrument::SYMBOL,
                    Term::Literal(Value::String(b"it's a 'quote'".to_vec().into())),
                )],
            },
            Atom {
                relation: ids::TRANSFER,
                bindings: vec![(
                    ids::transfer::EXTREF,
                    Term::Literal(Value::FixedBytes(vec![0xDE; 32].into())),
                )],
            },
            Atom {
                relation: ids::POSTING_TAG,
                bindings: vec![],
            },
        ],
        negated: vec![],
        predicates: vec![
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: var(1),
            }),
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(1),
                rhs: Term::Literal(Value::I64(-5)),
            }),
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Ne,
                lhs: var(0),
                rhs: Term::Param(ParamId(0)),
            }),
        ],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert!(
        t.sql.contains("EXISTS (SELECT 1 FROM \"PostingTag\")"),
        "{}",
        t.sql
    );
    assert!(t.sql.contains("'it''s a ''quote'''"), "{}", t.sql);
    assert!(
        t.sql.contains(&format!("X'{}'", "DE".repeat(32))),
        "{}",
        t.sql
    );
    assert!(t.sql.contains("t0.\"amount\" < t0.\"at\""), "{}", t.sql);
    assert!(t.sql.contains(">= -5"), "{}", t.sql);
    assert!(t.sql.contains("<> ?1"), "{}", t.sql);
    assert_eq!(t.params, vec![ParamSlot::Whole(ParamId(0))]);

    // Repeated in-atom variable equates its two columns.
    let repeated = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::AMOUNT, var(0)), (ids::posting::AT, var(0))],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&repeated, schema(), &[]).expect("translates");
    assert!(t.sql.contains("t0.\"amount\" = t0.\"at\""), "{}", t.sql);
}

#[test]
fn global_aggregates_carry_the_having_rule() {
    // Q(Count) :- Posting(amount = x): SQL's NULL-row-over-empty must
    // collapse to the engine's empty set.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::AMOUNT, var(0))],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert!(t.sql.ends_with("HAVING COUNT(*) > 0"), "{}", t.sql);
    assert!(t.sql.contains("SELECT DISTINCT"), "{}", t.sql);

    // Min/Max over the distinct binding set, grouped.
    let grouped = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Min,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Max,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::AMOUNT, var(1)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&grouped, schema(), &[]).expect("translates");
    assert!(t.sql.contains("MIN(v1)"), "{}", t.sql);
    assert!(t.sql.contains("MAX(v1)"), "{}", t.sql);
    assert!(t.sql.ends_with("GROUP BY v0"), "{}", t.sql);
}

#[test]
fn errors_name_the_untranslatable_construct() {
    let gates_only = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![Atom {
            relation: ids::POSTING_TAG,
            bindings: vec![],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let err = translate(&gates_only, schema(), &[]).unwrap_err();
    assert!(err.contains("no bound atoms"), "{err}");
}

/// A NUL in a string literal would truncate `SQLite`'s tokenizer
/// mid-statement — the translator rejects it by name instead of emitting
/// silently-shortened SQL.
#[test]
fn a_nul_string_literal_is_a_named_error() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::INSTRUMENT,
            bindings: vec![
                (ids::instrument::ID, Term::Var(VarId(0))),
                (
                    ids::instrument::SYMBOL,
                    Term::Literal(Value::String(b"before\0after".to_vec().into())),
                ),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let err = translate(&query, schema(), &[]).unwrap_err();
    assert!(err.contains("NUL byte in string literal"), "{err}");
}

/// `Pack` is the one query construct in the inexpressible set: `SQLite`
/// has no coalescing aggregate, so a `Pack` head routes to the naive
/// lane — never silently skipped, never translated.
#[test]
fn pack_heads_are_inexpressible_and_route_to_the_naive_lane() {
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: bumbledb::AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: ids::MANDATE,
            bindings: vec![
                (ids::mandate::ACCOUNT, var(0)),
                (ids::mandate::ACTIVE, var(1)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    assert_eq!(
        sqlite_expressible(&LaneCase::Query(&query)),
        Err(Inexpressible::PackAggregate)
    );
    let err = translate(&query, schema(), &[]).unwrap_err();
    assert!(err.contains("Pack is naive-only"), "{err}");
}

/// The `[shape]` criterion pinned as a golden: the inexpressible set is
/// the dependency judgments plus the `Pack` head (its own golden above);
/// every other query construct translates.
#[test]
fn the_inexpressible_set_is_exactly_the_dependency_judgments() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::ID, var(0))],
        }],
        negated: vec![Atom {
            relation: ids::POSTING_TAG,
            bindings: vec![(ids::posting_tag::POSTING, var(0))],
        }],
        predicates: vec![],
    });
    assert_eq!(sqlite_expressible(&LaneCase::Query(&query)), Ok(()));

    let functionality = StatementDescriptor::Functionality {
        relation: ids::MANDATE,
        projection: Box::new([ids::mandate::ACCOUNT, ids::mandate::ACTIVE]),
    };
    assert_eq!(
        sqlite_expressible(&LaneCase::Judgment(&functionality)),
        Err(Inexpressible::FunctionalityJudgment)
    );

    let containment = StatementDescriptor::Containment {
        source: Side {
            relation: ids::MANDATE,
            projection: Box::new([ids::mandate::ACCOUNT]),
            selection: Box::new([]),
        },
        target: Side {
            relation: ids::ACCOUNT,
            projection: Box::new([ids::account::ID]),
            selection: Box::new([]),
        },
    };
    assert_eq!(
        sqlite_expressible(&LaneCase::Judgment(&containment)),
        Err(Inexpressible::ContainmentJudgment)
    );
}

#[test]
fn a_multi_rule_projection_is_one_select_distinct_per_rule_joined_by_union() {
    // Q(x) :- Posting(account = x).
    // Q(x) :- PostingTag(posting = x).
    // One SELECT DISTINCT per rule, joined by UNION — set union, the
    // systematized rules translation.
    let query = Query {
        head: vec![bumbledb::HeadTerm::Var],
        rules: vec![
            Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![Atom {
                    relation: ids::POSTING,
                    bindings: vec![(ids::posting::ACCOUNT, var(0))],
                }],
                negated: vec![],
                predicates: vec![],
            },
            Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![Atom {
                    relation: ids::POSTING_TAG,
                    bindings: vec![(ids::posting_tag::POSTING, var(0))],
                }],
                negated: vec![],
                predicates: vec![],
            },
        ],
    };
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(
        t.sql,
        "SELECT DISTINCT t0.\"account\" FROM \"Posting\" AS t0 \
         UNION \
         SELECT DISTINCT t0.\"posting\" FROM \"PostingTag\" AS t0"
    );
    assert!(t.params.is_empty());
}

#[test]
fn a_multi_rule_aggregate_folds_over_the_unioned_head_projection() {
    // Q(x, Sum(y)) :- Posting(account = x, amount = y).
    // Q(x, Sum(y)) :- Posting(account = x, amount = y), y >= ?0.
    // The union fold: per-rule SELECT DISTINCT head projections
    // (aliased hN), one UNION, the fold grouped by the variable
    // positions. The param is query-global: one ?1 slot.
    let arm = |predicates: Vec<PredicateTree>| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::AMOUNT, var(1)),
            ],
        }],
        negated: vec![],
        predicates,
    };
    let query = Query {
        head: arm(vec![]).head(),
        rules: vec![
            arm(vec![]),
            arm(vec![PredicateTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(1),
                rhs: Term::Param(ParamId(0)),
            })]),
        ],
    };
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(
        t.sql,
        "SELECT h0, SUM(h1) FROM (\
         SELECT DISTINCT t0.\"account\" AS h0, t0.\"amount\" AS h1 FROM \"Posting\" AS t0 \
         UNION \
         SELECT DISTINCT t0.\"account\" AS h0, t0.\"amount\" AS h1 FROM \"Posting\" AS t0 \
         WHERE t0.\"amount\" >= ?1\
         ) GROUP BY h0"
    );
    assert_eq!(t.params, vec![ParamSlot::Whole(ParamId(0))]);
}

#[test]
fn a_param_repeated_across_rules_keeps_one_positional_slot() {
    // Q(x) :- Posting(account = ?0, amount = x).
    // Q(x) :- Posting(instrument = ?0, amount = x).
    // Params are query-global: both occurrences render ?1.
    let arm = |field: bumbledb::FieldId| Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (field, Term::Param(ParamId(0))),
                (ids::posting::AMOUNT, var(0)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let query = Query {
        head: vec![bumbledb::HeadTerm::Var],
        rules: vec![arm(ids::posting::ACCOUNT), arm(ids::posting::INSTRUMENT)],
    };
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.params, vec![ParamSlot::Whole(ParamId(0))]);
    assert_eq!(t.sql.matches("?1").count(), 2, "{}", t.sql);
    assert_eq!(t.sql.matches(" UNION ").count(), 1, "{}", t.sql);
}

#[test]
fn a_duration_find_is_end_minus_start_on_the_stored_columns() {
    // Q(account, Duration(active)) :- Mandate(account, active) — the
    // measure translates to arithmetic over the two interval columns.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
        atoms: vec![Atom {
            relation: ids::MANDATE,
            bindings: vec![
                (ids::mandate::ACCOUNT, var(0)),
                (ids::mandate::ACTIVE, var(1)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(
        t.sql,
        "SELECT DISTINCT t0.\"account\", (t0.\"active_end\" - t0.\"active_start\") \
         FROM \"Mandate\" AS t0"
    );
}
