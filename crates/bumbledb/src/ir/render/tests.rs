//! Golden tests: the renderer is deterministic and its output is the
//! documented rule notation, byte-exact — the calendar union query and
//! the Pack/Duration heads pin the grammar (PRD 20's passing criteria);
//! the malformed shapes pin totality (render is the roster's diagnostic
//! surface, so rejected queries must render, never panic).

use super::render;
use crate::allen::AllenMask;
use crate::ir::validate::validate;
use crate::ir::{
    AggOp, Atom, CmpOp, Comparison, FindTerm, MaskTerm, ParamId, PredicateTree, Query, Rule, Term,
    Value, VarId,
};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, ValueType,
};

/// The calendar fixture: Busy(person, during, kind), Ooo(person, during).
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
                name: "Busy".into(),
                fields: vec![
                    field("person", ValueType::U64),
                    field("during", during.clone()),
                    field(
                        "kind",
                        ValueType::Enum {
                            variants: ["Meeting", "Focus"].iter().map(|v| Box::from(*v)).collect(),
                        },
                    ),
                ],
            },
            RelationDescriptor {
                name: "Ooo".into(),
                fields: vec![field("person", ValueType::U64), field("during", during)],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const BUSY: RelationId = RelationId(0);
const OOO: RelationId = RelationId(1);
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
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Param(ParamId(0)),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(1)),
        })],
    }
}

/// The calendar union query, golden: unavailability is Busy ∪ Ooo
/// against a window param — two clauses, one per rule, `;`-terminated,
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
/// selection binding renders schema-grammar-verbatim with the variant
/// name resolved; negation is `!`.
#[test]
fn selection_negation_and_literal_mask_golden() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![
                (PERSON, Term::Var(VarId(0))),
                (DURING, Term::Var(VarId(1))),
                (KIND, Term::Literal(Value::Enum(1))),
            ],
        }],
        negated: vec![Atom {
            relation: OOO,
            bindings: vec![(PERSON, Term::Var(VarId(0)))],
        }],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::IntervalU64(100, 200)),
        })],
    });
    let schema = calendar();
    validate(&schema, &query).expect("the golden query is a real query");
    assert_eq!(
        render(&schema, &query),
        "(v0, v1) | Busy(person: v0, during: v1, kind == Focus), !Ooo(person: v0), \
         Allen(v1, INTERSECTS, 100..200);"
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
        predicates: vec![],
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
            FindTerm::AggregateDuration {
                op: AggOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![(PERSON, Term::Var(VarId(0))), (DURING, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Duration(VarId(1)),
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
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Contains,
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
        predicates: vec![PredicateTree::Or(vec![
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: MaskTerm::Literal(AllenMask::EMPTY),
                },
                lhs: Term::Var(VarId(3)),
                rhs: Term::Var(VarId(4)),
            }),
            PredicateTree::And(vec![]),
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
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(mask),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::IntervalU64(5, 9)),
        })],
    });
    assert_eq!(
        render(&calendar(), &query),
        "(v0) | Busy(person: v0, during: v1), Allen(v1, BEFORE|MET_BY, 5..9);"
    );
}
