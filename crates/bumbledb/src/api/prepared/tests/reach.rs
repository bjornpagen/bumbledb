use super::*;
use crate::ir::{CmpOp, ProjectionRule};

fn interiors_only() -> Query {
    Query {
        interiors: vec![Interior {
            rules: vec![
                ProjectionRule {
                    finds: vec![VarId(0)],
                    atoms: vec![Atom {
                        source: AtomSource::Edb(POSTING),
                        bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                    }],
                    negated: vec![],
                    conditions: vec![],
                }
                .to_rule(),
            ],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
        rec: None,
    }
}

#[test]
fn an_interiors_only_query_does_not_enter_reach() {
    let fix = postings(&[(1, 7, "a", 100)]);
    let prepared = fix.prepare(&interiors_only()).expect("prepare");
    assert!(
        !matches!(prepared.pipeline, PreparedPipeline::Reach { .. }),
        "interiors-only must not build ReachDriver"
    );
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::Cq { .. }),
        "interiors-only pipeline is Cq"
    );
}

#[test]
fn dead_main_with_live_interiors_still_reports_interior_emits() {
    let fix = postings(&[(1, 7, "a", 100), (2, 7, "b", 200)]);

    let query = Query {
        interiors: interiors_only().interiors().to_vec(),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Edb(POSTING),
                bindings: vec![(FieldId(3), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![
                ConditionTree::Leaf(Comparison {
                    op: CmpOp::Gt,
                    lhs: Term::Var(VarId(0)),
                    rhs: Term::Literal(Value::I64(5)),
                }),
                ConditionTree::Leaf(Comparison {
                    op: CmpOp::Lt,
                    lhs: Term::Var(VarId(0)),
                    rhs: Term::Literal(Value::I64(3)),
                }),
            ],
        }],
        rec: None,
    };
    let prepared = fix.prepare(&query).expect("prepare");
    match &prepared.pipeline {
        PreparedPipeline::Cq { interiors, rules } => {
            assert!(
                !interiors.is_empty(),
                "expected live interiors, got {}",
                interiors.len()
            );
            assert!(
                rules.is_empty(),
                "expected dead main, got {} rules",
                rules.len()
            );
        }
        PreparedPipeline::Reach { .. } => panic!("expected Cq, got Reach"),
        PreparedPipeline::PointProbe { .. } => panic!("expected Cq, got PointProbe"),
    }
}

/// G05 (recursive visited/frontier half): a zero sink-RAM allowance moves
/// the reach driver's seen-set — the recursion's visited state AND the
/// watermark log its per-round frontier drains from — onto the charged
/// scratch relation from row one. The transitive closure is unchanged, the
/// Δ/accumulated images keep their watermark contract across the tier
/// change, and the sealed rec table drains from scratch.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end spilled-recursion scenario"
)]
fn spilled_rec_seen_and_frontier_state_preserves_the_closure() {
    use crate::ir::{NonEmpty, Rec, RecRule, RecStep};
    const EDGE: RelationId = RelationId(0);

    let edge_descriptor = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Edge".into(),
            fields: vec![
                FieldDescriptor {
                    name: "src".into(),
                    value_type: bumbledb_theory::schema::ValueType::U64,
                },
                FieldDescriptor {
                    name: "dst".into(),
                    value_type: bumbledb_theory::schema::ValueType::U64,
                },
            ],
        }],
        statements: vec![],
    };
    // A chain with branches: 0→1→…→40, plus 7→50 and 50→8 (a shortcut
    // rejoining the chain), so rounds produce overlapping derivations the
    // seen-set must absorb in both tiers.
    let mut edges: Vec<Vec<Value>> = (0..40u64)
        .map(|i| vec![Value::U64(i), Value::U64(i + 1)])
        .collect();
    edges.push(vec![Value::U64(7), Value::U64(50)]);
    edges.push(vec![Value::U64(50), Value::U64(8)]);
    let fix = Fix::heap(edge_descriptor, &[(EDGE, edges)]);

    let closure = Query {
        interiors: vec![],
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
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![Atom {
                    source: AtomSource::Edb(EDGE),
                    bindings: vec![
                        (FieldId(0), Term::Var(VarId(0))),
                        (FieldId(1), Term::Var(VarId(1))),
                    ],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
                atoms: vec![Atom {
                    source: AtomSource::Edb(EDGE),
                    bindings: vec![
                        (FieldId(0), Term::Var(VarId(1))),
                        (FieldId(1), Term::Var(VarId(2))),
                    ],
                }],
                conditions: vec![],
            }),
        }),
    };
    let pairs = |answers: &Answers| -> Vec<(u64, u64)> {
        let mut rows: Vec<(u64, u64)> = (0..answers.len())
            .map(|row| {
                let (AnswerValue::U64(src), AnswerValue::U64(dst)) =
                    (answers.get(row, 0), answers.get(row, 1))
                else {
                    panic!("u64 closure pairs")
                };
                (src, dst)
            })
            .collect();
        rows.sort_unstable();
        rows
    };

    let mut resident = fix.prepare(&closure).expect("prepare");
    let expected = pairs(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("resident"),
    );
    assert!(expected.len() > 800, "a real closure, {}", expected.len());
    assert!(
        expected.contains(&(0, 40)) && expected.contains(&(7, 50)) && expected.contains(&(50, 40)),
        "the shortcut rejoins the chain"
    );

    // Forced transitions before the first frontier row (zero allowance),
    // during round 0 (a few rows in), and after several rounds' frontiers
    // already ran resident (G05: not only at a large final size).
    for ram_bytes in [0usize, 256, 4096] {
        let mut spilled = fix.prepare(&closure).expect("prepare");
        spilled.set_sink_ram(ram_bytes);
        let got = pairs(
            &fix.execute(&mut spilled, &[] as &[BindValue])
                .expect("spilled"),
        );
        assert_eq!(
            got, expected,
            "allowance {ram_bytes}: the spilled closure is the closure"
        );

        // Success → success reuse on the same spilled plan (Q-ATOMIC
        // shape): the next run re-creates scratch from a clean reset.
        let again = pairs(
            &fix.execute(&mut spilled, &[] as &[BindValue])
                .expect("re-execute"),
        );
        assert_eq!(again, expected, "allowance {ram_bytes}");
    }

    // The empty-base rec under a zero allowance: no frontier, no rows, no
    // spill artifacts leaking into the answer.
    let empty = Fix::heap(
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "Edge".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "src".into(),
                        value_type: bumbledb_theory::schema::ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "dst".into(),
                        value_type: bumbledb_theory::schema::ValueType::U64,
                    },
                ],
            }],
            statements: vec![],
        },
        &[(EDGE, vec![])],
    );
    let mut prepared = empty.prepare(&closure).expect("prepare");
    prepared.set_sink_ram(0);
    let none = empty
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("empty closure");
    assert_eq!(none.len(), 0);
}
