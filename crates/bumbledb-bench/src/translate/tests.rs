use bumbledb::schema::ValidateDescriptor as _;
use std::sync::OnceLock;

use super::*;
use crate::fixture::{field, fresh, var};
use bumbledb::AllenMask;
use bumbledb::FoldOp;
use bumbledb::ir::{Atom, CmpOp, Comparison, ConditionTree, FindTerm, Rule, Term};
use bumbledb::schema::{IntervalElement, RelationDescriptor, SchemaDescriptor, Side, ValueType};
use bumbledb::{FieldId, HeadTerm, InteriorId, NonEmpty, Query, Rec, RecRule, RecStep, VarId};

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
    pub const ORG: RelationId = RelationId(6);

    pub mod org {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
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
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ID, Term::Param(ParamId(0))),
                (ids::posting::AMOUNT, var(0)),
                (ids::posting::AT, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::POINT);
    assert_eq!(t.params, vec![ParamSlot::Whole(ParamId(0))]);
}

#[test]
fn containment_walk_matches_its_hand_written_golden() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ACCOUNT, Term::Param(ParamId(0))),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![
                    (ids::account::ID, Term::Param(ParamId(0))),
                    (ids::account::HOLDER, var(2)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::HOLDER),
                bindings: vec![(ids::holder::ID, var(2)), (ids::holder::NAME, var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
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
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ID, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![
                    (ids::account::ID, var(0)),
                    (ids::account::HOLDER, Term::Param(ParamId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::BALANCE);
}

#[test]
fn negated_atoms_match_their_goldens() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![(ids::posting::ID, var(0))],
        }],
        negated: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING_TAG),
            bindings: vec![
                (ids::posting_tag::POSTING, var(0)),
                (ids::posting_tag::TAG, Term::Literal(Value::U64(0))),
            ],
        }],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::NO_TAG);
    assert!(t.params.is_empty());

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::ORG_PARENT),
            bindings: vec![
                (ids::org_parent::CHILD, var(0)),
                (ids::org_parent::PARENT, var(1)),
            ],
        }],
        negated: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::ORG_PARENT),
            bindings: vec![(ids::org_parent::CHILD, var(1))],
        }],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::SELF_NEGATION);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![(ids::posting::ID, var(0))],
        }],
        negated: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING_TAG),
            bindings: vec![
                (ids::posting_tag::POSTING, var(0)),
                (ids::posting_tag::TAG, Term::Param(ParamId(0))),
            ],
        }],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert!(t.sql.contains("n0.\"tag\" = ?1"), "{}", t.sql);
    assert_eq!(t.params, vec![ParamSlot::Whole(ParamId(0))]);
}

#[test]
fn param_sets_render_as_literal_in_lists() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ENTRY, var(0)),
                (ids::posting::ACCOUNT, Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
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

    let err = translate(&query, schema(), &[]).unwrap_err();
    assert!(err.contains("param set 0"), "{err}");

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::ACCOUNT),
            bindings: vec![(ids::account::ID, var(0))],
        }],
        negated: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::ENTRY, Term::ParamSet(ParamId(0))),
            ],
        }],
        conditions: vec![],
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
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::MANDATE),
            bindings: vec![
                (ids::mandate::ORG, var(0)),
                (ids::mandate::ACTIVE, Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
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

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::ENTRY, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
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
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![(ids::posting::ACCOUNT, var(1)), (ids::posting::AT, var(2))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(1)),
                    (ids::mandate::ORG, var(0)),
                    (ids::mandate::ACTIVE, var(2)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::MEMBERSHIP);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![
                    (ids::posting::ACCOUNT, Term::Param(ParamId(0))),
                    (ids::posting::AT, Term::Param(ParamId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, Term::Param(ParamId(0))),
                    (ids::mandate::ORG, var(0)),
                    (ids::mandate::ACTIVE, Term::Param(ParamId(1))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
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
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(2)),
                    (ids::mandate::ORG, var(0)),
                    (ids::mandate::ACTIVE, var(3)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(2)),
                    (ids::mandate::ORG, var(1)),
                    (ids::mandate::ACTIVE, var(4)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::INTERSECTS,
            },
            lhs: var(3),
            rhs: var(4),
        })],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::INTERSECTS);
}

#[test]
fn point_in_matches_both_goldens() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::MANDATE),
            bindings: vec![(ids::mandate::ORG, var(0)), (ids::mandate::ACTIVE, var(1))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::COVERS,
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

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![(ids::mandate::ORG, var(0)), (ids::mandate::ACTIVE, var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![(ids::posting::AT, var(2))],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::PointIn,
            lhs: var(1),
            rhs: var(2),
        })],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::POINT_IN);
}

#[test]
fn interval_equality_matches_its_goldens() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(0)),
                    (ids::mandate::ACTIVE, var(2)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::MANDATE),
                bindings: vec![
                    (ids::mandate::ACCOUNT, var(1)),
                    (ids::mandate::ACTIVE, var(3)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(2),
            rhs: var(3),
        })],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::INTERVAL_EQ);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::MANDATE),
            bindings: vec![
                (ids::mandate::ORG, var(0)),
                (
                    ids::mandate::ACTIVE,
                    Term::Literal(Value::IntervalI64(
                        bumbledb::Interval::<i64>::new(1700, 1800).expect("nonempty interval"),
                    )),
                ),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::INTERVAL_EQ_LITERAL);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::MANDATE),
            bindings: vec![
                (ids::mandate::ORG, var(0)),
                (ids::mandate::ACTIVE, Term::Param(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::INTERVAL_EQ_PARAM);
    assert_eq!(
        t.params,
        vec![ParamSlot::Start(ParamId(0)), ParamSlot::End(ParamId(0))]
    );
}

#[test]
fn an_interval_find_projects_both_halves() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::MANDATE),
            bindings: vec![(ids::mandate::ORG, var(0)), (ids::mandate::ACTIVE, var(1))],
        }],
        negated: vec![],
        conditions: vec![],
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

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING),
                bindings: vec![(ids::posting::AMOUNT, var(0)), (ids::posting::AT, var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::INSTRUMENT),
                bindings: vec![(
                    ids::instrument::SYMBOL,
                    Term::Literal(Value::String("it's a 'quote'".into())),
                )],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::TRANSFER),
                bindings: vec![(
                    ids::transfer::EXTREF,
                    Term::Literal(Value::FixedBytes(vec![0xDE; 32].into())),
                )],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::POSTING_TAG),
                bindings: vec![],
            },
        ],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: var(1),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(1),
                rhs: Term::Literal(Value::I64(-5)),
            }),
            ConditionTree::Leaf(Comparison {
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

    let repeated = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![(ids::posting::AMOUNT, var(0)), (ids::posting::AT, var(0))],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let t = translate(&repeated, schema(), &[]).expect("translates");
    assert!(t.sql.contains("t0.\"amount\" = t0.\"at\""), "{}", t.sql);
}

#[test]
fn global_aggregates_carry_the_having_rule() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![(ids::posting::AMOUNT, var(0))],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let t = translate(&query, schema(), &[]).expect("translates");
    assert!(t.sql.ends_with("HAVING COUNT(*) > 0"), "{}", t.sql);
    assert!(t.sql.contains("SELECT DISTINCT"), "{}", t.sql);

    let grouped = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Min,
                over: VarId(1),
            },
            FindTerm::Aggregate {
                op: FoldOp::Max,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::AMOUNT, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let t = translate(&grouped, schema(), &[]).expect("translates");
    assert!(t.sql.contains("MIN(v1)"), "{}", t.sql);
    assert!(t.sql.contains("MAX(v1)"), "{}", t.sql);
    assert!(t.sql.ends_with("GROUP BY v0"), "{}", t.sql);
}

#[test]
fn errors_name_the_untranslatable_construct() {
    let gates_only = Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING_TAG),
            bindings: vec![],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let err = translate(&gates_only, schema(), &[]).unwrap_err();
    assert!(err.contains("no bound atoms"), "{err}");
}

#[test]
fn a_nul_string_literal_is_a_named_error() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::INSTRUMENT),
            bindings: vec![
                (ids::instrument::ID, Term::Var(VarId(0))),
                (
                    ids::instrument::SYMBOL,
                    Term::Literal(Value::String("before\0after".into())),
                ),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let err = translate(&query, schema(), &[]).unwrap_err();
    assert!(err.contains("NUL byte in string literal"), "{err}");
}

#[test]
fn pack_heads_are_inexpressible_and_route_to_the_naive_lane() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::MANDATE),
            bindings: vec![
                (ids::mandate::ACCOUNT, var(0)),
                (ids::mandate::ACTIVE, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    assert_eq!(
        sqlite_expressible(&LaneCase::Query(&query)),
        Err(Inexpressible::PackAggregate)
    );
    let err = translate(&query, schema(), &[]).unwrap_err();
    assert!(err.contains("Pack is naive-only"), "{err}");
}

#[test]
fn the_inexpressible_set_is_exactly_the_dependency_judgments() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![(ids::posting::ID, var(0))],
        }],
        negated: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING_TAG),
            bindings: vec![(ids::posting_tag::POSTING, var(0))],
        }],
        conditions: vec![],
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

    // equally inexpressible (the SUM is a query, not a typed refusal);

    let capacity = StatementDescriptor::Capacity {
        target: Side {
            relation: ids::POSTING,
            projection: Box::new([ids::posting::ID]),
            selection: Box::new([]),
        },
        weight: bumbledb::schema::Weight::Field(ids::posting_tag::TAG),
        lo: 0,
        hi: Some(bumbledb::schema::Bound::Lit(3)),
        source: Side {
            relation: ids::POSTING_TAG,
            projection: Box::new([ids::posting_tag::POSTING]),
            selection: Box::new([]),
        },
    };
    assert_eq!(
        sqlite_expressible(&LaneCase::Judgment(&capacity)),
        Err(Inexpressible::CapacityJudgment)
    );
}

#[test]
fn a_multi_rule_projection_is_one_select_distinct_per_rule_joined_by_union() {
    let query = Query {
        interiors: vec![],
        head: vec![bumbledb::HeadTerm::Var],
        rules: vec![
            Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::POSTING),
                    bindings: vec![(ids::posting::ACCOUNT, var(0))],
                }],
                negated: vec![],
                conditions: vec![],
            },
            Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::POSTING_TAG),
                    bindings: vec![(ids::posting_tag::POSTING, var(0))],
                }],
                negated: vec![],
                conditions: vec![],
            },
        ],
        rec: None,
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
    let arm = |conditions: Vec<ConditionTree>| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::AMOUNT, var(1)),
            ],
        }],
        negated: vec![],
        conditions,
    };
    let query = Query {
        interiors: vec![],
        head: arm(vec![]).head(),
        rules: vec![
            arm(vec![]),
            arm(vec![ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(1),
                rhs: Term::Param(ParamId(0)),
            })]),
        ],
        rec: None,
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
    let arm = |field: bumbledb::FieldId| Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (field, Term::Param(ParamId(0))),
                (ids::posting::AMOUNT, var(0)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![bumbledb::HeadTerm::Var],
        rules: vec![arm(ids::posting::ACCOUNT), arm(ids::posting::INSTRUMENT)],
        rec: None,
    };
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.params, vec![ParamSlot::Whole(ParamId(0))]);
    assert_eq!(t.sql.matches("?1").count(), 2, "{}", t.sql);
    assert_eq!(t.sql.matches(" UNION ").count(), 1, "{}", t.sql);
}

fn closure_query() -> Query {
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::ORG_PARENT),
                    bindings: vec![
                        (ids::org_parent::CHILD, var(0)),
                        (ids::org_parent::PARENT, var(1)),
                    ],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![(FieldId(0), var(1)), (FieldId(1), var(2))],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::ORG_PARENT),
                    bindings: vec![
                        (ids::org_parent::CHILD, var(0)),
                        (ids::org_parent::PARENT, var(1)),
                    ],
                }],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(1))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

#[test]
fn the_linear_closure_matches_its_hand_written_golden() {
    let query = closure_query();
    assert_eq!(sqlite_expressible(&LaneCase::Query(&query)), Ok(()));
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::CLOSURE);
    assert!(t.params.is_empty());
}

#[test]
fn negation_of_finished_rec_matches_its_hand_written_golden() {
    let mut query = closure_query();
    match &mut query {
        Query {
            head,
            rules,
            rec: None,
            ..
        }
        | Query { head, rules, .. } => {
            *head = vec![HeadTerm::Var];
            *rules = vec![Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::ORG),
                    bindings: vec![(ids::org::ID, var(0))],
                }],
                negated: vec![Atom {
                    source: bumbledb::AtomSource::Interior(InteriorId(0)),
                    bindings: vec![(FieldId(0), var(0))],
                }],
                conditions: vec![],
            }];
        }
    }
    assert_eq!(sqlite_expressible(&LaneCase::Query(&query)), Ok(()));
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::CLOSURE_ROOTS);
}

#[test]
fn the_parameterized_reachable_set_matches_its_hand_written_golden() {
    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::ORG_PARENT),
                    bindings: vec![
                        (ids::org_parent::CHILD, Term::Param(ParamId(0))),
                        (ids::org_parent::PARENT, var(0)),
                    ],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(1)],
                self_bindings: vec![(FieldId(0), var(0))],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::ORG_PARENT),
                    bindings: vec![
                        (ids::org_parent::CHILD, var(0)),
                        (ids::org_parent::PARENT, var(1)),
                    ],
                }],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), var(0))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    };
    assert_eq!(sqlite_expressible(&LaneCase::Query(&query)), Ok(()));
    let t = translate(&query, schema(), &[]).expect("translates");
    assert_eq!(t.sql, goldens::CLOSURE_FROM_PARAM);
    assert_eq!(
        t.params,
        vec![ParamSlot::Whole(ParamId(0))],
        "one placeholder, shared across the rec's arms"
    );
}

#[test]
fn interval_derived_columns_error_by_name() {
    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::MANDATE),
                    bindings: vec![(ids::mandate::ACTIVE, var(0))],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0)],
                self_bindings: vec![(FieldId(0), var(0))],
                atoms: vec![],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), var(0))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    };
    assert_eq!(
        sqlite_expressible(&LaneCase::Query(&query)),
        Err(Inexpressible::IntervalDerivedColumn)
    );
    let err = translate(&query, schema(), &[]).unwrap_err();
    assert!(err.contains("interval-typed derived column"), "{err}");
}
