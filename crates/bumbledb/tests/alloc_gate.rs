//! The allocation gate: the doc's protocol as a contract of warm
//! INVARIANT: this binary holds exactly ONE test function, and check.sh

#![cfg(feature = "alloc-counter")]

use bumbledb::alloc_counter;
use bumbledb::ir::{
    Atom, AtomSource, CmpOp, Comparison, FindTerm, FoldOp, HeadTerm, Interior, InteriorId, ParamId,
    Query, Rec, RecRule, RecStep, Rule, Term, Value, VarId,
};
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, ValueType, Weight,
};
use bumbledb::{
    Answers, BindValue, ConditionTree, Db, NonEmpty, ParamArg, PreparedQuery, ProjectionRule,
    ReadInstance,
};

mod common;

#[expect(
    clippy::too_many_lines,
    reason = "the one fixture schema — a linear declaration table"
)]
fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "memo".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "holder".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Busy".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "person".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "slot".into(),
                        value_type: ValueType::Interval {
                            element: bumbledb::schema::IntervalElement::U64,
                        },
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "doc".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "pos".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "note".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Blob".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "digest".into(),
                        value_type: ValueType::FixedBytes { len: 16 },
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Containment {
                source: Side {
                    relation: RelationId(0),
                    projection: Box::new([FieldId(1)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: RelationId(1),
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
            StatementDescriptor::Capacity {
                target: Side {
                    relation: RelationId(1),
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                weight: Weight::Unit,
                lo: 1,
                hi: Some(Bound::Lit(4096)),
                source: Side {
                    relation: RelationId(3),
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
        ],
    }
}

const POSTING: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const BUSY: RelationId = RelationId(2);
const ITEM: RelationId = RelationId(3);
const BLOB: RelationId = RelationId(4);

fn digest16(id: u64) -> [u8; 16] {
    let seed = u8::try_from(id % 251).expect("mod 251 fits u8");
    std::array::from_fn(|i| {
        seed.wrapping_mul(31)
            .wrapping_add(u8::try_from(i).expect("i < 16"))
    })
}

const ITEM_CHAIN: u64 = 8;

const ITEM_LADDER: [u64; 5] = [6, 24, 72, 240, 660];

bumbledb::schema! {
    pub GateLedger;
    relation GateItem {
        id: u64 as GateItemId, fresh,
        memo: str,
    }
    GateItem(memo) -> GateItem;
}

const LADDER: [u64; 5] = [6, 24, 72, 240, 660];

fn populate(db: &Db<SchemaDescriptor>) {
    db.write(|tx| {
        for account in 0..20u64 {
            tx.insert_dyn(ACCOUNT, [&[Value::U64(account), Value::U64(account % 5)]])?;
        }
        for id in 0..500u64 {
            tx.insert_dyn(
                POSTING,
                [&[
                    Value::U64(id),
                    Value::U64(id % 20),
                    Value::I64((id.cast_signed() % 100) - 50),
                    Value::String(format!("memo-{}", id % 4).into()),
                ]],
            )?;
        }

        for id in 0..120u64 {
            let person = id % 6;
            let start = (id * 7) % 40;
            let end = if id % 5 == 4 {
                u64::MAX
            } else {
                start + 1 + id % 9
            };
            tx.insert_dyn(
                BUSY,
                [&[
                    Value::U64(id),
                    Value::U64(person),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
                    ),
                ]],
            )?;
        }
        let mut id = 500u64;
        for ((account, holder), count) in (20u64..).zip(5u64..).zip(LADDER) {
            tx.insert_dyn(ACCOUNT, [&[Value::U64(account), Value::U64(holder)]])?;
            for _ in 0..count {
                tx.insert_dyn(
                    POSTING,
                    [&[
                        Value::U64(id),
                        Value::U64(account),
                        Value::I64((id.cast_signed() % 100) - 50),
                        Value::String(format!("memo-{}", id % 4).into()),
                    ]],
                )?;
                id += 1;
            }
        }

        for id in 0..32u64 {
            tx.insert_dyn(
                BLOB,
                [&[Value::U64(id), Value::FixedBytes(Box::from(digest16(id)))]],
            )?;
        }

        for doc in 0..20u64 {
            item_chain(tx, doc, ITEM_CHAIN)?;
        }
        for (doc, len) in (20u64..).zip(ITEM_LADDER) {
            item_chain(tx, doc, len)?;
        }
        Ok(())
    })
    .expect("populate")
    .expect("accepted");
}

fn item_chain(
    tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>,
    doc: u64,
    len: u64,
) -> Result<(), bumbledb::Error> {
    for pos in 1..=len {
        tx.insert_dyn(
            ITEM,
            [&[
                Value::U64(doc),
                Value::U64(pos),
                Value::U64(doc * 10_000 + pos),
            ]],
        )?;
    }
    Ok(())
}

fn join_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    })
}

fn aggregate_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
            FindTerm::Count,
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(3))),
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    })
}

fn string_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(3))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(3))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ne,
            lhs: Term::Var(VarId(3)),
            rhs: Term::Literal(Value::String(Box::from("memo-0"))),
        })],
    })
}

fn minmax_query() -> Query {
    Query::single(Rule {
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
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(3))),
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn latch_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(POSTING),
            bindings: vec![
                (
                    FieldId(3),
                    Term::Literal(Value::String(Box::from("memo-1"))),
                ),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(amount):- Posting(memo = ?0, amount) — the selection shape: a rotating Eq
/// param on a non-key field probes the COLT's selection level; after the
/// rotation's first cycle forces every probed subtrie, further rotation must
/// not touch the allocator.
fn selection_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(3), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(memo, amount):- Posting(account = ?0, memo, amount) — string results
/// across rotating params: the finalize memo and the buffer byte heap must both
/// sit at their high-water after warmup.
fn string_rotation_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(1), Term::Param(ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn escalation_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Param(ParamId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn recursive_query() -> Query {
    let account = |a: u16, h: u16| Atom {
        source: AtomSource::Edb(ACCOUNT),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(a))),
            (FieldId(1), Term::Var(VarId(h))),
        ],
    };
    let cap = ConditionTree::Leaf(Comparison {
        op: CmpOp::Le,
        lhs: Term::Var(VarId(0)),
        rhs: Term::Param(ParamId(0)),
    });
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![account(0, 1)],
                conditions: vec![cap.clone()],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![account(0, 1)],
                conditions: vec![cap],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

fn interiors_only_query() -> Query {
    let join = Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: AtomSource::Edb(ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    Query {
        interiors: vec![Interior {
            rules: vec![ProjectionRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: join.atoms,
                negated: join.negated,
                conditions: join.conditions,
            }],
        }],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
        rec: None,
    }
}

fn union_rules_query() -> Query {
    let rule = |op: CmpOp| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    Query {
        interiors: vec![],
        head: vec![bumbledb::HeadTerm::Var, bumbledb::HeadTerm::Var],
        rules: vec![rule(CmpOp::Ge), rule(CmpOp::Le)],
        rec: None,
    }
}

fn union_aggregate_query() -> Query {
    let rule = |op: CmpOp| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
            FindTerm::Count,
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    Query {
        interiors: vec![],
        head: vec![
            bumbledb::HeadTerm::Var,
            bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Sum),
            bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Count),
        ],
        rules: vec![rule(CmpOp::Ge), rule(CmpOp::Le)],
        rec: None,
    }
}

fn pack_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(BUSY),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn key_probe_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn blob_probe_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(BLOB),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn blob_selection_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(BLOB),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Param(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn blob_ne_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(BLOB),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ne,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Param(ParamId(0)),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ne,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::FixedBytes(Box::from([0xAA; 16]))),
            }),
        ],
    })
}

fn blob_set_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(BLOB),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn marks_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ITEM),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// The capacity-heavy write family: warm commits that churn the marks machinery
/// — per round, five capacity parents each take a tail append (group measure
/// +1), a net-nothing delete-reinsert of the head (the touched-parent re-judge
/// — the measure walk runs on a delta that nets to nothing,
/// `lean/Bumbledb/Txn/DeltaRestriction.lean: delta_restricted_commit_sound`),
/// and then a restoring commit removes the tails. The write delta's arena is
/// per-commit by design — the family's assertions are the judgment's (every
/// round commits green through live capacity laws) and the read windows' (the
/// caller re-runs the steady-state gate after the churn: post-commit rebuild is
/// sanctioned in warmup, then the pools must re-converge to zero).
fn marks_write_family(db: &Db<SchemaDescriptor>) {
    for round in 0..8u64 {
        db.write(|tx| {
            for doc in 0..5u64 {
                tx.insert_dyn(
                    ITEM,
                    [&[
                        Value::U64(doc),
                        Value::U64(ITEM_CHAIN + 1),
                        Value::U64(round),
                    ]],
                )?;

                let head = [Value::U64(doc), Value::U64(1), Value::U64(doc * 10_000 + 1)];
                tx.delete_dyn(ITEM, [&head])?;
                tx.insert_dyn(ITEM, [&head])?;
            }
            Ok(())
        })
        .expect("marks write round commits through live capacity laws")
        .expect("accepted");
        db.write(|tx| {
            for doc in 0..5u64 {
                tx.delete_dyn(
                    ITEM,
                    [&[
                        Value::U64(doc),
                        Value::U64(ITEM_CHAIN + 1),
                        Value::U64(round),
                    ]],
                )?;
            }
            Ok(())
        })
        .expect("marks restore round commits")
        .expect("accepted");
    }
}

/// The gate protocol for one prepared query and its fixed param set.
fn gate(
    label: &str,
    prepared: &mut PreparedQuery<SchemaDescriptor>,
    snap: &ReadInstance<'_, SchemaDescriptor>,
    param_set: &[Vec<BindValue<'_>>],
) {
    let mut out = Answers::new();

    for _ in 0..8 {
        for params in param_set {
            snap.execute(prepared, params, &mut out).expect(label);
        }
    }

    alloc_counter::reset();
    for _ in 0..8 {
        for params in param_set {
            snap.execute(prepared, params, &mut out).expect(label);
        }
    }
    assert_eq!(
        alloc_counter::count(),
        0,
        "{label}: a warm execution allocated"
    );
    assert_eq!(
        alloc_counter::dealloc_count(),
        0,
        "{label}: a warm execution freed retained capacity"
    );
    let bytes = alloc_counter::snapshot();
    assert_eq!(
        (bytes.window.alloc_bytes, bytes.window.dealloc_bytes),
        (0, 0),
        "{label}: warm byte totals must be zero too"
    );
    assert!(!out.is_empty(), "{label}: the fixture produced rows");
}

/// The gate protocol over mixed scalar/set arguments (the [`ParamArg`] entry):
/// N=8 warmups, M=8 measured runs at zero — the set-bearing twin of [`gate`].
fn gate_args(
    label: &str,
    prepared: &mut PreparedQuery<SchemaDescriptor>,
    snap: &ReadInstance<'_, SchemaDescriptor>,
    arg_set: &[Vec<ParamArg<'_>>],
) {
    let mut out = Answers::new();
    for _ in 0..8 {
        for args in arg_set {
            snap.execute(prepared, args, &mut out).expect(label);
        }
    }
    alloc_counter::reset();
    for _ in 0..8 {
        for args in arg_set {
            snap.execute(prepared, args, &mut out).expect(label);
        }
    }
    assert_eq!(
        alloc_counter::count(),
        0,
        "{label}: a warm execution allocated"
    );
    assert_eq!(
        alloc_counter::dealloc_count(),
        0,
        "{label}: a warm execution freed retained capacity"
    );
    let bytes = alloc_counter::snapshot();
    assert_eq!(
        (bytes.window.alloc_bytes, bytes.window.dealloc_bytes),
        (0, 0),
        "{label}: warm byte totals must be zero too"
    );
    assert!(!out.is_empty(), "{label}: the fixture produced rows");
}

/// One measured execution that must not touch the allocator at all — events and
/// bytes, both directions ([`escalation_gate`]'s repeat steps).
fn silent(
    label: &str,
    step: &str,
    prepared: &mut PreparedQuery<SchemaDescriptor>,
    snap: &ReadInstance<'_, SchemaDescriptor>,
    params: &[BindValue<'_>],
    out: &mut Answers,
) {
    alloc_counter::reset();
    snap.execute(prepared, params, out).expect(label);
    let bytes = alloc_counter::snapshot();
    assert_eq!(
        (
            bytes.window.allocs,
            bytes.window.deallocs,
            bytes.window.alloc_bytes,
            bytes.window.dealloc_bytes
        ),
        (0, 0, 0, 0),
        "{label}: {step} must be allocation-silent"
    );
}

/// Mutation demonstration (the gate is not theater; no test-only injection
/// point lives in the hot path, so the check was done manually during
/// development): a temporary
/// `std::hint::black_box(Vec::<u64>::with_capacity(1));` at the top of
/// `Executor::execute` (`exec/run/execute.rs`) — one heap allocation per
/// execution — made this variant (run first, ahead of the steady-state
/// scenarios) fail at its first repeat step: `escalation: repeat of params[1]
/// right after its high-water run must be allocation-silent` with `(1, 1, 8, 8)
/// != (0, 0, 0, 0)`; in normal order the steady-state gate caught the same
/// mutation at its first measured scenario (`join/batch1: a warm execution
/// allocated: 32 != 0`).
fn escalation_gate(
    label: &str,
    prepared: &mut PreparedQuery<SchemaDescriptor>,
    snap: &ReadInstance<'_, SchemaDescriptor>,
    params: &[Vec<BindValue<'_>>],
) {
    let mut out = Answers::new();

    for _ in 0..8 {
        snap.execute(prepared, &params[0], &mut out).expect(label);
    }
    let mut growth_events = 0u64;
    for i in 1..params.len() {
        alloc_counter::reset();
        snap.execute(prepared, &params[i], &mut out).expect(label);
        if alloc_counter::count() > 0 {
            growth_events += 1;
        }

        silent(
            label,
            &format!("repeat of params[{i}] right after its high-water run"),
            prepared,
            snap,
            &params[i],
            &mut out,
        );

        for (j, previous) in params.iter().enumerate().take(i) {
            silent(
                label,
                &format!("repeat of params[{j}] under params[{i}]'s high-water"),
                prepared,
                snap,
                previous,
                &mut out,
            );
        }
    }

    assert!(
        growth_events >= 1,
        "{label}: the escalation observed no growth event — the fixture is vacuous"
    );
    assert!(!out.is_empty(), "{label}: the fixture produced rows");
}

fn borrowed_struct_gate() {
    let dir = common::TempDir::new("alloc-gate-borrowed");
    let db = Db::create(dir.path(), GateLedger)
        .expect("create")
        .expect("accepted");
    let item = common::expect_admitted(db.write(|tx| {
        let id: GateItemId = tx.reserve(1)?.start().expect("nonempty");
        tx.insert([&GateItem {
            id,
            memo: "memo-borrowed",
        }])?;
        Ok(id)
    }));
    db.write(|tx| {
        tx.insert([&GateItem {
            id: item,
            memo: "memo-borrowed",
        }])?;
        alloc_counter::reset();
        let fact = GateItem {
            id: item,
            memo: "memo-borrowed",
        };
        tx.insert([&fact])?;
        let got = tx.get(item)?.expect("present");
        assert_eq!(got.memo, "memo-borrowed");
        let bytes = alloc_counter::snapshot();
        assert_eq!(
            (
                bytes.window.allocs,
                bytes.window.deallocs,
                bytes.window.alloc_bytes,
                bytes.window.dealloc_bytes
            ),
            (0, 0, 0, 0),
            "borrowed-struct insert + get must be host-allocation-free"
        );
        Ok(())
    })
    .expect("borrowed-struct gate")
    .expect("accepted");

    // The snapshot twin (ruled 2026-07-23, R15): the committed-state

    db.read(|snap| {
        let key = GateItemByMemo {
            memo: "memo-borrowed",
        };
        let warm = snap.get(key)?.expect("present");
        assert!(snap.contains(&warm)?);
        alloc_counter::reset();
        let got = snap.get(key)?.expect("present");
        assert_eq!(got.memo, "memo-borrowed");
        assert!(snap.contains(&warm)?);
        assert!(
            snap.get(GateItemByMemo {
                memo: "memo-never-interned",
            })?
            .is_none()
        );
        let bytes = alloc_counter::snapshot();
        assert_eq!(
            (
                bytes.window.allocs,
                bytes.window.deallocs,
                bytes.window.alloc_bytes,
                bytes.window.dealloc_bytes
            ),
            (0, 0, 0, 0),
            "snapshot keyed get (hit, contains, miss) must be host-allocation-free"
        );
        Ok(())
    })
    .expect("snapshot point-read gate");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test function by the binary's invariant — every gate \
              scenario lives inside it"
)]
fn zero_warm_allocation_gate() {
    let dir = common::TempDir::new("alloc-gate");
    let db = Db::create(dir.path(), schema())
        .expect("create")
        .expect("accepted");
    populate(&db);

    let join_params = vec![
        vec![BindValue::I64(-10)],
        vec![BindValue::I64(0)],
        vec![BindValue::I64(25)],
        vec![BindValue::I64(40)],
    ];

    let key_probe_params = vec![
        vec![BindValue::U64(9999)],
        vec![BindValue::U64(5)],
        vec![BindValue::U64(499)],
    ];

    db.read(|snap| {
        for batch in [1usize, 2, 64, 128] {
            let mut join = db.prepare(&join_query())?;
            join.set_batch_size(batch);
            gate(&format!("join/batch{batch}"), &mut join, snap, &join_params);

            let mut aggregate = db.prepare(&aggregate_query())?;
            aggregate.set_batch_size(batch);
            gate(
                &format!("aggregate/batch{batch}"),
                &mut aggregate,
                snap,
                &join_params,
            );
        }

        let mut interiors_only = db.prepare(&interiors_only_query())?;
        gate("interiors-only", &mut interiors_only, snap, &join_params);

        let no_params = vec![vec![]];
        let mut strings = db.prepare(&string_query())?;
        gate("string", &mut strings, snap, &no_params);
        let mut minmax = db.prepare(&minmax_query())?;
        gate("minmax", &mut minmax, snap, &no_params);

        let mut pack = db.prepare(&pack_query())?;
        gate("pack", &mut pack, snap, &no_params);

        let mut key_probe = db.prepare(&key_probe_query())?;
        gate("key_probe", &mut key_probe, snap, &key_probe_params);

        let blob_digests: Vec<[u8; 16]> = (0..4u64).map(digest16).collect();
        let blob_digest_params: Vec<Vec<BindValue<'_>>> = blob_digests
            .iter()
            .map(|digest| vec![BindValue::FixedBytes(digest)])
            .collect();
        let mut blob_selection = db.prepare(&blob_selection_query())?;
        gate(
            "bytes-selection",
            &mut blob_selection,
            snap,
            &blob_digest_params,
        );
        let mut blob_ne = db.prepare(&blob_ne_query())?;
        gate("bytes-ne-filter", &mut blob_ne, snap, &blob_digest_params);

        let blob_probe_params = vec![
            vec![BindValue::U64(9999)],
            vec![BindValue::U64(3)],
            vec![BindValue::U64(7)],
        ];
        let mut blob_probe = db.prepare(&blob_probe_query())?;
        gate("bytes-key-probe", &mut blob_probe, snap, &blob_probe_params);

        let set_a: Vec<Value> = [0u64, 2, 1, 2]
            .iter()
            .map(|id| Value::FixedBytes(Box::from(digest16(*id))))
            .collect();
        let set_b: Vec<Value> = [3u64, 1]
            .iter()
            .map(|id| Value::FixedBytes(Box::from(digest16(*id))))
            .collect();
        let blob_set_args = vec![vec![ParamArg::Set(&set_a)], vec![ParamArg::Set(&set_b)]];
        let mut blob_set = db.prepare(&blob_set_query())?;
        gate_args("bytes-set", &mut blob_set, snap, &blob_set_args);

        let marks_params: Vec<Vec<BindValue<'_>>> =
            (0..4u64).map(|doc| vec![BindValue::U64(doc)]).collect();
        let mut marks = db.prepare(&marks_query())?;
        gate("marks", &mut marks, snap, &marks_params);

        // aggregate regime) all sit at their high-water after warmup.
        let mut union_rules = db.prepare(&union_rules_query())?;
        gate("union-rules", &mut union_rules, snap, &join_params);
        let mut union_aggregate = db.prepare(&union_aggregate_query())?;
        gate("union-aggregate", &mut union_aggregate, snap, &join_params);

        // measured rotations must not touch the allocator.

        let memo_texts: Vec<String> = (0..4).map(|m| format!("memo-{m}")).collect();
        let selection_params: Vec<Vec<BindValue<'_>>> = memo_texts
            .iter()
            .map(|text| vec![BindValue::Str(text)])
            .collect();
        let mut selection = db.prepare(&selection_query())?;
        gate("selection", &mut selection, snap, &selection_params);

        let mut latch = db.prepare(&latch_query())?;
        gate("literal-latch", &mut latch, snap, &no_params);

        let account_params: Vec<Vec<BindValue<'_>>> =
            (0..4).map(|a| vec![BindValue::U64(a)]).collect();
        let mut string_rotation = db.prepare(&string_rotation_query())?;
        gate(
            "string-rotation",
            &mut string_rotation,
            snap,
            &account_params,
        );

        // iteration shape) high-water after warmup.
        let recursive_params: Vec<Vec<BindValue<'_>>> = [5u64, 10, 15, 20]
            .iter()
            .map(|cap| vec![BindValue::U64(*cap)])
            .collect();
        let mut recursive = db.prepare(&recursive_query())?;
        gate("recursive", &mut recursive, snap, &recursive_params);

        // gate protocol): holders 5..10 bind the ladder accounts —

        let escalation_params: Vec<Vec<BindValue<'_>>> =
            (5..10u64).map(|h| vec![BindValue::U64(h)]).collect();
        let mut escalation = db.prepare(&escalation_query())?;
        escalation_gate("escalation", &mut escalation, snap, &escalation_params);

        let recursive_escalation: Vec<Vec<BindValue<'_>>> = [4u64, 9, 14, 19, 24]
            .iter()
            .map(|cap| vec![BindValue::U64(*cap)])
            .collect();
        let mut recursive_escalation_q = db.prepare(&recursive_query())?;
        escalation_gate(
            "recursive-escalation",
            &mut recursive_escalation_q,
            snap,
            &recursive_escalation,
        );

        let mut fresh = db.prepare(&join_query())?;
        let mut out = Answers::new();
        let mut per_round = Vec::new();
        for _ in 0..3 {
            alloc_counter::reset();
            for params in &join_params {
                snap.execute(&mut fresh, params, &mut out)?;
            }
            per_round.push(alloc_counter::count());
        }
        assert_eq!(
            per_round[2], 0,
            "third warmup round must be silent: {per_round:?}"
        );
        Ok(())
    })
    .expect("gate");

    // to zero after the sanctioned post-commit rebuild (warmup), and the

    marks_write_family(&db);
    db.read(|snap| {
        let marks_params: Vec<Vec<BindValue<'_>>> =
            (0..4u64).map(|doc| vec![BindValue::U64(doc)]).collect();
        let mut marks = db.prepare(&marks_query())?;
        gate("marks-postwrite", &mut marks, snap, &marks_params);

        let marks_escalation: Vec<Vec<BindValue<'_>>> =
            (20..25u64).map(|doc| vec![BindValue::U64(doc)]).collect();
        let mut marks_escalation_q = db.prepare(&marks_query())?;
        escalation_gate(
            "marks-escalation",
            &mut marks_escalation_q,
            snap,
            &marks_escalation,
        );
        Ok(())
    })
    .expect("marks windows");

    // string is committed (and its scratch warmed) before the measured

    borrowed_struct_gate();
}
