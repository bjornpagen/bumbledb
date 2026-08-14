//! The signature table (the crucible PRD 04, policy 8): one row per
//! head form × each legal input type, the expected values read off the
//! pre-refactor result-row/sink behavior and hand-verified against
//! `docs/architecture/20-query-ir.md`'s aggregate typing prose. The
//! TABLE is the pin — it was landed green against the triple derivation
//! and must stay green, byte-identical, over the reified predicate.

use crate::ir::{AggOp, Atom, FindTerm, Query, Rule, Term, VarId};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor,
    ValueType,
};

/// R(id fresh u64, b bool, u u64, i i64, s string, x bytes<8>,
///   pu interval<u64>, pi interval<i64>, ku u64, ki i64) — every value
/// type at one field, plus one orderable key field per key type (an Arg
/// row's carry and key are distinct fields).
fn sig_schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
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
                        width: None,
                    },
                ),
                field(
                    "pi",
                    ValueType::Interval {
                        element: IntervalElement::I64,
                        width: None,
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

// Field positions in R, by declaration order above.
const B: u16 = 1;
const U: u16 = 2;
const I: u16 = 3;
const S: u16 = 4;
const X: u16 = 5;
const PU: u16 = 6;
const PI: u16 = 7;

/// Every (field, type) pair of the fixture — the per-type row generators
/// below iterate this roster so no type is skippable by omission.
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
                width: None,
            },
        ),
        (
            PI,
            ValueType::Interval {
                element: IntervalElement::I64,
                width: None,
            },
        ),
    ]
}

fn interval_u64() -> ValueType {
    ValueType::Interval {
        element: IntervalElement::U64,
        width: None,
    }
}

fn interval_i64() -> ValueType {
    ValueType::Interval {
        element: IntervalElement::I64,
        width: None,
    }
}

/// One table row: a single-rule query (finds + one atom's `(field, var)`
/// bindings) and the signature it must derive.
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

fn fold(op: AggOp, over: u16) -> FindTerm {
    FindTerm::Aggregate {
        op,
        over: Some(VarId(over)),
    }
}

/// The exhaustive table: one row per `FindTerm`/`AggOp` form × each
/// legal input type.
#[expect(
    clippy::too_many_lines,
    reason = "the exhaustive pin table is clearer kept together"
)]
fn cases() -> Vec<Case> {
    let mut cases = Vec::new();

    // Plain projection, every type: the column is the variable's type.
    for (field, ty) in type_roster() {
        cases.push(case(
            format!("var over {ty:?}"),
            vec![FindTerm::Var(VarId(0))],
            vec![(field, 0)],
            vec![ty],
        ));
    }

    // Nullary Count: U64 whatever the rule binds.
    cases.push(case(
        "count",
        vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        vec![(U, 0)],
        vec![ValueType::U64],
    ));

    // The arithmetic folds, both integer types: the input's type.
    for op in [AggOp::Sum, AggOp::Min, AggOp::Max] {
        for (field, ty) in [(U, ValueType::U64), (I, ValueType::I64)] {
            cases.push(case(
                format!("{op:?} over {ty:?}"),
                vec![fold(op, 0)],
                vec![(field, 0)],
                vec![ty],
            ));
        }
    }

    // The measure, projected: one U64 word per binding, both elements.
    for field in [PU, PI] {
        cases.push(case(
            format!("duration over field {field}"),
            vec![FindTerm::Measure(VarId(0))],
            vec![(field, 0)],
            vec![ValueType::U64],
        ));
    }

    // The measure, folded (Sum/Min/Max of Duration): U64, both elements.
    for op in [AggOp::Sum, AggOp::Min, AggOp::Max] {
        for field in [PU, PI] {
            cases.push(case(
                format!("{op:?} duration over field {field}"),
                vec![FindTerm::AggregateMeasure { op, over: VarId(0) }],
                vec![(field, 0)],
                vec![ValueType::U64],
            ));
        }
    }

    // Pack: the packed segment shares its input's interval type.
    cases.push(case(
        "pack over interval<u64>",
        vec![fold(AggOp::Pack, 0)],
        vec![(PU, 0)],
        vec![interval_u64()],
    ));
    cases.push(case(
        "pack over interval<i64>",
        vec![fold(AggOp::Pack, 0)],
        vec![(PI, 0)],
        vec![interval_i64()],
    ));

    // Multi-column heads mixing the forms (group keys + folds, group
    // keys + measures, group keys + Pack).
    cases.push(case(
        "group key + sum + count",
        vec![
            FindTerm::Var(VarId(0)),
            fold(AggOp::Sum, 1),
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        vec![(U, 0), (I, 1)],
        vec![ValueType::U64, ValueType::I64, ValueType::U64],
    ));
    cases.push(case(
        "projected measure + folded measure",
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Measure(VarId(1)),
            FindTerm::AggregateMeasure {
                op: AggOp::Max,
                over: VarId(2),
            },
        ],
        vec![(U, 0), (PU, 1), (PI, 2)],
        vec![ValueType::U64, ValueType::U64, ValueType::U64],
    ));
    cases.push(case(
        "bool group key + pack",
        vec![FindTerm::Var(VarId(0)), fold(AggOp::Pack, 1)],
        vec![(B, 0), (PU, 1)],
        vec![ValueType::Bool, interval_u64()],
    ));
    cases.push(case(
        "interval group key + count",
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
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

/// The observation seam the pin swings on: the query's derived output
/// signature. Pinned against the pre-refactor triple derivation (read
/// through `prepare`'s result-column iterator), re-anchored to the
/// validation-sealed predicate — the table never moved.
fn signature_of(schema: &Schema, query: &Query) -> Vec<ValueType> {
    let witness = crate::ir::validate::validate(schema, query).expect("validate");
    witness
        .signature()
        .columns
        .iter()
        .map(|column| column.ty.clone())
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
