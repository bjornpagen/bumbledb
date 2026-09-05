use super::render;
use crate::ir::validate::validate;
use crate::ir::{
    Atom, CmpOp, Comparison, ConditionTree, FindTerm, ParamId, Query, Rule, Term, Value, VarId,
};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, ValueType,
};

#[test]
fn f64_literal_uses_canonical_ieee_bits() {
    for (input, expected) in [
        (0x8000_0000_0000_0000, "f64:0x0000000000000000"),
        (0x7ff0_0000_0000_0001, "f64:0x7ff8000000000000"),
        (0xfff0_0000_0000_0000, "f64:0xfff0000000000000"),
        (0x0000_0000_0000_0001, "f64:0x0000000000000001"),
        (0x3ff0_0000_0000_0000, "f64:0x3ff0000000000000"),
    ] {
        let mut text = String::new();
        super::literal(&mut text, &Value::F64(crate::F64::from_bits(input)));
        assert_eq!(text, expected);
    }
}

fn calendar() -> Schema {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    let during = ValueType::Interval {
        element: IntervalElement::U64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Busy".into(),
                fields: vec![
                    field("person", ValueType::U64),
                    field("during", during),
                    field("kind", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Ooo".into(),
                fields: vec![field("person", ValueType::U64), field("during", during)],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Focus".into(),
                        values: Box::new([]),
                    },
                    Row {
                        handle: "Break".into(),
                        values: Box::new([]),
                    },
                ])),
                name: "Kind".into(),
                fields: vec![],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(0),
                projection: Box::new([FieldId(2)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(2),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
    .validate()
    .expect("valid fixture")
}

const BUSY: RelationId = RelationId(0);
const OOO: RelationId = RelationId(1);
const KIND_RELATION: RelationId = RelationId(2);
const PERSON: FieldId = FieldId(0);
const DURING: FieldId = FieldId(1);
const KIND: FieldId = FieldId(2);

fn projection_rule(relation: RelationId) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(relation),
            bindings: vec![(PERSON, Term::Var(VarId(0))), (DURING, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::INTERSECTS,
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    }
}

#[test]
fn calendar_union_golden() {
    let rule = projection_rule(BUSY);
    let query = Query {
        interiors: vec![],
        head: rule.head(),
        rules: vec![rule, projection_rule(OOO)],
        rec: None,
    };
    let schema = calendar();
    validate(&schema, &query).expect("the golden query is a real query");
    assert_eq!(
        render(&schema, &query),
        "(v0, v1) | Busy(person: v0, during: v1), Allen(v1, INTERSECTS, ?0);\n\
         (v0, v1) | Ooo(person: v0, during: v1), Allen(v1, INTERSECTS, ?0);"
    );
}

#[test]
fn selection_negation_and_literal_mask_golden() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(BUSY),
            bindings: vec![
                (PERSON, Term::Var(VarId(0))),
                (DURING, Term::Var(VarId(1))),
                (KIND, Term::Literal(Value::U64(1))),
            ],
        }],
        negated: vec![Atom {
            source: crate::ir::AtomSource::Edb(OOO),
            bindings: vec![(PERSON, Term::Var(VarId(0)))],
        }],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::INTERSECTS,
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(100, 200).expect("nonempty interval"),
            )),
        })],
    });
    let schema = calendar();
    validate(&schema, &query).expect("the golden query is a real query");
    assert_eq!(
        render(&schema, &query),
        "(v0, v1) | Busy(person: v0, during: v1, kind == Break), !Ooo(person: v0), \
         Allen(v1, INTERSECTS, 100..200);"
    );
}

#[test]
fn closed_reference_handles_golden() {
    let selection = |word: u64| {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: crate::ir::AtomSource::Edb(BUSY),
                bindings: vec![
                    (PERSON, Term::Var(VarId(0))),
                    (KIND, Term::Literal(Value::U64(word))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        })
    };
    let schema = calendar();
    validate(&schema, &selection(0)).expect("the golden query is a real query");
    assert_eq!(
        render(&schema, &selection(0)),
        "(v0) | Busy(person: v0, kind == Focus);"
    );

    assert_eq!(
        render(&schema, &selection(7)),
        "(v0) | Busy(person: v0, kind == Kind(7?));"
    );

    let own_id = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(BUSY),
                bindings: vec![(PERSON, Term::Var(VarId(0))), (KIND, Term::Var(VarId(1)))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(KIND_RELATION),
                bindings: vec![(FieldId(0), Term::Literal(Value::U64(1)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    assert_eq!(
        render(&schema, &own_id),
        "(v0) | Busy(person: v0, kind: v1), Kind(id == Break);"
    );
}

#[test]
fn pack_head_golden() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(BUSY),
            bindings: vec![(PERSON, Term::Var(VarId(0))), (DURING, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let schema = calendar();
    validate(&schema, &query).expect("the golden query is a real query");
    assert_eq!(
        render(&schema, &query),
        "(v0, Pack(v1)) | Busy(person: v0, during: v1);"
    );
}

#[test]
fn membership_and_param_forms() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(BUSY),
            bindings: vec![
                (PERSON, Term::ParamSet(ParamId(0))),
                (DURING, Term::Var(VarId(1))),
                (KIND, Term::Param(ParamId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::PointIn,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Var(VarId(0)),
        })],
    });

    assert_eq!(
        render(&calendar(), &query),
        "(v0) | Busy(person in ?0, during: v1, kind == ?1), v0 in v1;"
    );
}

#[test]
fn malformed_queries_render_with_placeholders() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(9)),
            bindings: vec![(FieldId(7), Term::Var(VarId(3)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Or(vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: AllenMask::EMPTY,
                },
                lhs: Term::Var(VarId(3)),
                rhs: Term::Var(VarId(4)),
            }),
            ConditionTree::And(vec![]),
        ])],
    });
    assert_eq!(
        render(&calendar(), &query),
        "(Count) | relation#9(field#7: v3), or(Allen(v3, EMPTY, v4), and());"
    );
}

#[test]
fn mask_union_spelling() {
    let mask =
        AllenMask::new(AllenMask::BEFORE.bits() | AllenMask::MET_BY.bits()).expect("13-bit mask");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(BUSY),
            bindings: vec![(PERSON, Term::Var(VarId(0))), (DURING, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen { mask },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(5, 9).expect("nonempty interval"),
            )),
        })],
    });
    assert_eq!(
        render(&calendar(), &query),
        "(v0) | Busy(person: v0, during: v1), Allen(v1, BEFORE|MET_BY, 5..9);"
    );
}
