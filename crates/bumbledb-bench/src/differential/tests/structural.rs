//! Generated stage graphs compared with the independent
//! endpoint-cell and scalar model. A disagreement shrinks its input facts.
use super::{Rng, atom, field, var};
use crate::differential::{Op, run};
use crate::fixture::TempDir;
use crate::naive::{Delta, NaiveDb};
use bumbledb::schema::{IntervalElement, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    Atom, AtomSource, CmpOp, Comparison, ConditionTree, Db, FieldId, FindTerm, FoldOp, Interior,
    InteriorId, Interval, Query, RelationId, Rounding, Rule, ScalarExpr as E, SegmentOp, Value,
    VarId,
};

const INPUT: RelationId = RelationId(0);
const BLOCK: RelationId = RelationId(1);
const TOKEN: RelationId = RelationId(2);

fn descriptor(element: IntervalElement) -> SchemaDescriptor {
    let interval = ValueType::Interval { element };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Input".into(),
                extension: None,
                fields: vec![
                    field("id", ValueType::U64),
                    field("group", ValueType::U64),
                    field("a", interval),
                    field("b", interval),
                    field("c", interval),
                ],
            },
            RelationDescriptor {
                name: "Block".into(),
                extension: None,
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Token".into(),
                extension: None,
                fields: vec![field("id", ValueType::U64)],
            },
        ],
        statements: vec![],
    }
}

fn measure(var: u16, element: IntervalElement) -> E {
    let expr = E::Measure(Box::new(E::Var(VarId(var))));
    if element == IntervalElement::F64 {
        E::Cast {
            kind: bumbledb::NumericCast::ToU64Exact,
            expr: Box::new(expr),
        }
    } else {
        expr
    }
}

fn stage(index: u16, fields: u16) -> Atom {
    Atom {
        source: AtomSource::Interior(InteriorId(u32::from(index))),
        bindings: (0..fields).map(|i| (FieldId(i), var(i))).collect(),
    }
}

fn rule(finds: Vec<FindTerm>, source: Atom) -> Rule {
    Rule {
        finds,
        atoms: vec![source],
        negated: vec![],
        conditions: vec![],
    }
}

/// Bits choose both operators, a dependent producer, and duplicate rule arms.
/// The consumer independently chooses projection, every fold, or coalescing.
fn composed(bits: u8, consumer: u8, element: IntervalElement) -> Query {
    let op = |bit| {
        if bits & bit == 0 {
            SegmentOp::Intersection
        } else {
            SegmentOp::Difference
        }
    };
    let first = rule(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Segments {
                op: op(1),
                left: VarId(2),
                right: VarId(3),
            },
            FindTerm::Segments {
                op: op(2),
                left: VarId(2),
                right: VarId(4),
            },
        ],
        atom(
            INPUT,
            &[
                (0, var(0)),
                (1, var(1)),
                (2, var(2)),
                (3, var(3)),
                (4, var(4)),
            ],
        ),
    );
    let mut interiors = vec![Interior {
        rules: if bits & 8 == 0 {
            vec![first]
        } else {
            vec![first.clone(), first]
        },
    }];
    if bits & 4 != 0 {
        interiors.push(Interior {
            rules: vec![rule(
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Var(VarId(1)),
                    FindTerm::Segments {
                        op: SegmentOp::Difference,
                        left: VarId(2),
                        right: VarId(3),
                    },
                    FindTerm::Var(VarId(3)),
                ],
                stage(0, 4),
            )],
        });
    }
    let source = u16::try_from(interiors.len() - 1).unwrap();
    let expression = weighted_expression(element, consumer);
    let mut measured = rule(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
            FindTerm::Var(VarId(3)),
            FindTerm::Compute(expression),
        ],
        stage(source, 4),
    );
    measured.atoms.push(atom(TOKEN, &[(0, var(0))]));
    measured.negated.push(atom(BLOCK, &[(0, var(0))]));
    if consumer & 1 != 0 {
        measured.conditions.push(ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: bumbledb::AllenMask::INTERSECTS,
            },
            lhs: var(2),
            rhs: var(3),
        }));
    }
    interiors.push(Interior {
        rules: vec![measured],
    });
    let output = output_fields(consumer);
    let last = rule(
        output,
        stage(u16::try_from(interiors.len() - 1).unwrap(), 5),
    );
    Query {
        interiors,
        head: last.head(),
        rules: vec![last],
        rec: None,
    }
}

fn weighted_expression(element: IntervalElement, consumer: u8) -> E {
    let expression = E::MulDiv {
        a: Box::new(E::Add(
            Box::new(measure(2, element)),
            Box::new(measure(3, element)),
        )),
        b: Box::new(E::Literal(Value::U64(3))),
        divisor: Box::new(E::Literal(Value::U64(2))),
        rounding: match consumer % 3 {
            0 => Rounding::TowardZero,
            1 => Rounding::NearestTiesAwayFromZero,
            _ => Rounding::NearestTiesToEven,
        },
    };
    if consumer == 7 {
        E::Cast {
            kind: bumbledb::NumericCast::ToF64Exact,
            expr: Box::new(expression),
        }
    } else {
        expression
    }
}

fn output_fields(consumer: u8) -> Vec<FindTerm> {
    match consumer {
        0 => vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(2)),
            FindTerm::Var(VarId(3)),
            FindTerm::Var(VarId(4)),
        ],
        1 => vec![FindTerm::Var(VarId(4))], // deliberate set projection
        2 => vec![FindTerm::Var(VarId(1)), FindTerm::Pack { over: VarId(2) }],
        3 => vec![FindTerm::Var(VarId(1)), FindTerm::Count],
        _ => vec![
            FindTerm::Var(VarId(1)),
            FindTerm::Aggregate {
                over: VarId(4),
                op: match consumer {
                    4 => FoldOp::Sum,
                    5 => FoldOp::Min,
                    6 => FoldOp::Max,
                    _ => FoldOp::Mean,
                },
            },
        ],
    }
}

fn corpus(seed: u64, element: IntervalElement) -> Delta {
    let mut rng = Rng(seed);
    let mut inserts = vec![];
    for pair in 0..8 {
        let mut span = || {
            let start = i64::try_from(rng.below(20)).unwrap() - 10;
            Value::IntervalI64(
                Interval::new(start, start + 1 + i64::try_from(rng.below(12)).unwrap()).unwrap(),
            )
        };
        let a = span();
        let b = span();
        let c = span();
        for id in [pair * 2, pair * 2 + 1] {
            inserts.push((
                INPUT,
                vec![
                    Value::U64(id),
                    Value::U64(pair % 3),
                    a.clone(),
                    b.clone(),
                    c.clone(),
                ],
            ));
            if id % 5 != 0 {
                inserts.push((TOKEN, vec![Value::U64(id)]));
            }
            if id % 7 == 0 {
                inserts.push((BLOCK, vec![Value::U64(id)]));
            }
        }
    }
    for (_, row) in &mut inserts {
        for value in row {
            if let Value::IntervalI64(span) = value {
                *value = match element {
                    IntervalElement::I64 => Value::IntervalI64(*span),
                    IntervalElement::U64 => Value::IntervalU64(
                        Interval::new(
                            u64::try_from(span.start() + 16).unwrap(),
                            u64::try_from(span.end() + 16).unwrap(),
                        )
                        .unwrap(),
                    ),
                    IntervalElement::F64 => Value::IntervalF64(
                        Interval::new(
                            bumbledb::F64::from(f64::from(i32::try_from(span.start()).unwrap())),
                            bumbledb::F64::from(f64::from(i32::try_from(span.end()).unwrap())),
                        )
                        .unwrap(),
                    ),
                };
            }
        }
    }
    Delta {
        inserts,
        deletes: vec![],
    }
}

fn agrees(facts: &Delta, query: &Query, element: IntervalElement) -> bool {
    let dir = TempDir::new("structural-shrink");
    let db = Db::create(
        dir.path(),
        descriptor(element),
        crate::harness::bench_work(),
    )
    .unwrap()
    .unwrap();
    let mut model = NaiveDb::new(&descriptor(element));
    run(
        &db,
        &mut model,
        &[
            Op::Write(facts.clone()),
            Op::Query {
                query: query.clone(),
                params: vec![],
            },
        ],
    )
    .is_ok()
}

fn shrink(mut facts: Delta, query: &Query, element: IntervalElement) -> Delta {
    let mut index = 0;
    while index < facts.inserts.len() {
        let mut candidate = facts.clone();
        candidate.inserts.remove(index);
        if agrees(&candidate, query, element) {
            index += 1;
        } else {
            facts = candidate;
        }
    }
    facts
}

#[test]
fn generated_structural_stages_match_independent_model() {
    for element in [
        IntervalElement::I64,
        IntervalElement::U64,
        IntervalElement::F64,
    ] {
        for seed in 0..12 {
            let facts = corpus(seed, element);
            let dir = TempDir::new("structural-composition");
            let db = Db::create(
                dir.path(),
                descriptor(element),
                crate::harness::bench_work(),
            )
            .unwrap()
            .unwrap();
            let mut model = NaiveDb::new(&descriptor(element));
            run(&db, &mut model, &[Op::Write(facts.clone())]).unwrap();
            for bits in 0..16 {
                for consumer in 0..8 {
                    let query = composed(bits, consumer, element);
                    if let Err(error) = run(
                        &db,
                        &mut model,
                        &[Op::Query {
                            query: query.clone(),
                            params: vec![],
                        }],
                    ) {
                        panic!(
                            "element={element:?} seed={seed} bits={bits} consumer={consumer}: {error:?}\nminimal facts: {:?}\nquery: {query:?}",
                            shrink(facts.clone(), &query, element)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn dense_measure_boundaries_match_independent_rational_arithmetic() {
    use crate::verify::f64_oracle::{INF, MAX_FINITE, NEG_INF, SIGN};
    let endpoints = [
        NEG_INF,
        SIGN | MAX_FINITE,
        SIGN | 0x3ff0_0000_0000_0000,
        SIGN | 1,
        0,
        1,
        0x3ff0_0000_0000_0000,
        0x3ff0_0000_0000_0001,
        MAX_FINITE,
        INF,
    ];
    let mut inserts = Vec::new();
    for (index, &start) in endpoints.iter().enumerate() {
        for &end in &endpoints[index + 1..] {
            let id = u64::try_from(inserts.len()).unwrap();
            let span = Value::IntervalF64(
                Interval::new(
                    bumbledb::F64::from_bits(start),
                    bumbledb::F64::from_bits(end),
                )
                .unwrap(),
            );
            inserts.push((
                INPUT,
                vec![
                    Value::U64(id),
                    Value::U64(0),
                    span.clone(),
                    span.clone(),
                    span,
                ],
            ));
        }
    }
    let count = inserts.len();
    let dir = TempDir::new("dense-measure-boundaries");
    let schema = descriptor(IntervalElement::F64);
    let db = Db::create(dir.path(), schema.clone(), crate::harness::bench_work())
        .unwrap()
        .unwrap();
    let mut model = NaiveDb::new(&schema);
    run(
        &db,
        &mut model,
        &[Op::Write(Delta {
            inserts,
            deletes: vec![],
        })],
    )
    .unwrap();
    for id in 0..count {
        let r = rule(
            vec![FindTerm::Compute(E::Measure(Box::new(E::Var(VarId(0)))))],
            atom(
                INPUT,
                &[
                    (
                        0,
                        bumbledb::Term::Literal(Value::U64(u64::try_from(id).unwrap())),
                    ),
                    (2, var(0)),
                ],
            ),
        );
        let query = Query {
            head: r.head(),
            rules: vec![r],
            interiors: vec![],
            rec: None,
        };
        run(
            &db,
            &mut model,
            &[Op::Query {
                query,
                params: vec![],
            }],
        )
        .unwrap();
    }
}
