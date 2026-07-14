//! Golden tests: the renderer is deterministic and its output is the
//! documented rule notation, byte-exact — the calendar union query and
//! the Pack/Duration heads pin the grammar (PRD 20's passing criteria);
//! the malformed shapes pin totality (render is the roster's diagnostic
//! surface, so rejected queries must render, never panic); the handle
//! goldens pin closed-reference printing (the vocabulary's names on the
//! read side, with the out-of-range fallback visibly wrong).

use super::render;
use crate::allen::AllenMask;
use crate::ir::validate::validate;
use crate::ir::{
    AggOp, Atom, CmpOp, Comparison, ConditionTree, FindTerm, MaskTerm, ParamId, Query, Rule, Term,
    Value, VarId,
};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Row,
    Schema, SchemaDescriptor, Side, StatementDescriptor, ValueType,
};

/// The calendar fixture: Busy(person, during, kind), Ooo(person, during),
/// with `kind` a closed reference into Kind = { Focus, Break } — the
/// handle goldens' vocabulary.
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
                    field("during", during.clone()),
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
            relation,
            bindings: vec![(PERSON, Term::Var(VarId(0))), (DURING, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Param(ParamId(0)),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(1)),
        })],
    }
}

/// The calendar union query, golden: unavailability is Busy ∪ Ooo
/// against a window param — two rules, each `;`-terminated,
/// newline-separated (the mask is a param here; the literal-mask spelling
/// is pinned below).
#[test]
fn calendar_union_golden() {
    let rule = projection_rule(BUSY);
    let query = Query {
        head: rule.head(),
        rules: vec![rule, projection_rule(OOO)],
    };
    let schema = calendar();
    validate(&schema, &query).expect("the golden query is a real query");
    assert_eq!(
        render(&schema, &query),
        "(v0, v1) | Busy(person: v0, during: v1), Allen(v1, ?0, ?1);\n\
         (v0, v1) | Ooo(person: v0, during: v1), Allen(v1, ?0, ?1);"
    );
}

/// The literal-mask spelling: a workload composite renders by name; the
/// selection binding renders schema-grammar-verbatim — a closed-reference
/// word as its handle; negation is `!`.
#[test]
fn selection_negation_and_literal_mask_golden() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![
                (PERSON, Term::Var(VarId(0))),
                (DURING, Term::Var(VarId(1))),
                (KIND, Term::Literal(Value::U64(1))),
            ],
        }],
        negated: vec![Atom {
            relation: OOO,
            bindings: vec![(PERSON, Term::Var(VarId(0)))],
        }],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::IntervalU64(
                crate::Interval::<u64>::new(100, 200).expect("nonempty interval"),
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

/// The handle goldens: a literal word at a closed-reference position
/// prints its handle — on the referencing field and on the closed
/// relation's own id field alike — and an out-of-range word prints
/// visibly wrong as `Kind(7?)` (rendering hides nothing).
#[test]
fn closed_reference_handles_golden() {
    let selection = |word: u64| {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: BUSY,
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
    // Out of range: no seventh row exists — the fallback names the
    // relation (the engine never learns host newtype names) and keeps
    // the number with the `?` that marks it wrong.
    assert_eq!(
        render(&schema, &selection(7)),
        "(v0) | Busy(person: v0, kind == Kind(7?));"
    );
    // The closed relation's own id field maps to itself.
    let own_id = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: BUSY,
                bindings: vec![(PERSON, Term::Var(VarId(0))), (KIND, Term::Var(VarId(1)))],
            },
            Atom {
                relation: KIND_RELATION,
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

/// The Pack head, golden: relation-shaped coalesce — group key plus one
/// packed interval position.
#[test]
fn pack_head_golden() {
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: BUSY,
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

/// The Duration head, golden: the measure projected and folded, plus a
/// measure comparison — `Duration(v)` in every legal position.
#[test]
fn duration_head_golden() {
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::AggregateMeasure {
                op: AggOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![(PERSON, Term::Var(VarId(0))), (DURING, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Measure(VarId(1)),
            rhs: Term::Literal(Value::U64(3600)),
        })],
    });
    let schema = calendar();
    validate(&schema, &query).expect("the golden query is a real query");
    assert_eq!(
        render(&schema, &query),
        "(v0, Sum(Duration(v1))) | Busy(person: v0, during: v1), Duration(v1) >= 3600;"
    );
}

/// Membership renders point-first (`point in interval`); a param set in
/// a binding is membership too (`field in ?N`); a scalar param binding is
/// the selection form with the param admitted.
#[test]
fn membership_and_param_forms() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: BUSY,
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
    // v0 needs a scalar anchor for validity; this golden pins the
    // notation forms, so it stays a render-only shape.
    assert_eq!(
        render(&calendar(), &query),
        "(v0) | Busy(person in ?0, during: v1, kind == ?1), v0 in v1;"
    );
}

/// Totality on malformed data: unknown ids render as placeholders, the
/// vacuous masks by name, nested trees functionally — the roster's
/// rejections all still render.
#[test]
fn malformed_queries_render_with_placeholders() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![Atom {
            relation: RelationId(9),
            bindings: vec![(FieldId(7), Term::Var(VarId(3)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Or(vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: MaskTerm::Literal(AllenMask::EMPTY),
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

/// A non-composite multi-basic mask joins singleton names with `|` (the
/// mask-level bar is set union over the 13 basics).
#[test]
fn mask_union_spelling() {
    let mask =
        AllenMask::new(AllenMask::BEFORE.bits() | AllenMask::MET_BY.bits()).expect("13-bit mask");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![(PERSON, Term::Var(VarId(0))), (DURING, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(mask),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::IntervalU64(
                crate::Interval::<u64>::new(5, 9).expect("nonempty interval"),
            )),
        })],
    });
    assert_eq!(
        render(&calendar(), &query),
        "(v0) | Busy(person: v0, during: v1), Allen(v1, BEFORE|MET_BY, 5..9);"
    );
}
