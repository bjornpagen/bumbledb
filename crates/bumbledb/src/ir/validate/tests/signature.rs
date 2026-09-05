use crate::ir::{Atom, FindTerm, FoldOp, Query, Rule, Term, VarId};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, IntervalElement, RelationDescriptor, SchemaDescriptor, ValueType,
};

fn sig_schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                field("b", ValueType::Bool),
                field("u", ValueType::U64),
                field("i", ValueType::I64),
                field("s", ValueType::String),
                field("x", ValueType::FixedBytes { len: 8 }),
                field(
                    "pu",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                ),
                field(
                    "pi",
                    ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                ),
                field("ku", ValueType::U64),
                field("ki", ValueType::I64),
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const B: u16 = 1;
const U: u16 = 2;
const I: u16 = 3;
const S: u16 = 4;
const X: u16 = 5;
const PU: u16 = 6;
const PI: u16 = 7;

fn type_roster() -> Vec<(u16, ValueType)> {
    vec![
        (B, ValueType::Bool),
        (U, ValueType::U64),
        (I, ValueType::I64),
        (S, ValueType::String),
        (X, ValueType::FixedBytes { len: 8 }),
        (
            PU,
            ValueType::Interval {
                element: IntervalElement::U64,
            },
        ),
        (
            PI,
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        ),
    ]
}

fn interval_u64() -> ValueType {
    ValueType::Interval {
        element: IntervalElement::U64,
    }
}

fn interval_i64() -> ValueType {
    ValueType::Interval {
        element: IntervalElement::I64,
    }
}

struct Case {
    name: String,
    finds: Vec<FindTerm>,
    bindings: Vec<(u16, u16)>,
    expected: Vec<ValueType>,
}

fn case(
    name: impl Into<String>,
    finds: Vec<FindTerm>,
    bindings: Vec<(u16, u16)>,
    expected: Vec<ValueType>,
) -> Case {
    Case {
        name: name.into(),
        finds,
        bindings,
        expected,
    }
}

fn fold(op: FoldOp, over: u16) -> FindTerm {
    FindTerm::Aggregate {
        op,
        over: VarId(over),
    }
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();

    for (field, ty) in type_roster() {
        cases.push(case(
            format!("var over {ty:?}"),
            vec![FindTerm::Var(VarId(0))],
            vec![(field, 0)],
            vec![ty],
        ));
    }

    cases.push(case(
        "count",
        vec![FindTerm::Count],
        vec![(U, 0)],
        vec![ValueType::U64],
    ));

    for op in [FoldOp::Sum, FoldOp::Min, FoldOp::Max] {
        for (field, ty) in [(U, ValueType::U64), (I, ValueType::I64)] {
            cases.push(case(
                format!("{op:?} over {ty:?}"),
                vec![fold(op, 0)],
                vec![(field, 0)],
                vec![ty],
            ));
        }
    }

    cases.push(case(
        "pack over interval<u64>",
        vec![FindTerm::Pack { over: VarId(0) }],
        vec![(PU, 0)],
        vec![interval_u64()],
    ));
    cases.push(case(
        "pack over interval<i64>",
        vec![FindTerm::Pack { over: VarId(0) }],
        vec![(PI, 0)],
        vec![interval_i64()],
    ));

    cases.push(case(
        "group key + sum + count",
        vec![
            FindTerm::Var(VarId(0)),
            fold(FoldOp::Sum, 1),
            FindTerm::Count,
        ],
        vec![(U, 0), (I, 1)],
        vec![ValueType::U64, ValueType::I64, ValueType::U64],
    ));
    cases.push(case(
        "bool group key + pack",
        vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        vec![(B, 0), (PU, 1)],
        vec![ValueType::Bool, interval_u64()],
    ));
    cases.push(case(
        "interval group key + count",
        vec![FindTerm::Var(VarId(0)), FindTerm::Count],
        vec![(PI, 0)],
        vec![interval_i64(), ValueType::U64],
    ));

    cases
}

fn query_of(case: &Case) -> Query {
    Query::single(Rule {
        finds: case.finds.clone(),
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(bumbledb_theory::schema::RelationId(0)),
            bindings: case
                .bindings
                .iter()
                .map(|(field, var)| (FieldId(*field), Term::Var(VarId(*var))))
                .collect(),
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn signature_of(schema: &Schema, query: &Query) -> Vec<ValueType> {
    let witness = crate::ir::validate::validate(schema, query).expect("validate");
    witness
        .signature()
        .columns
        .iter()
        .map(|column| *column.ty())
        .collect()
}

#[test]
fn the_signature_table_pins_every_head_form() {
    let schema = sig_schema();
    for case in cases() {
        assert_eq!(
            signature_of(&schema, &query_of(&case)),
            case.expected,
            "{}",
            case.name
        );
    }
}
