use super::*;
use crate::ir::{CmpOp, NonEmpty, ParamId, ProjectionRule, Rec, RecRule, RecStep};

fn chain_fixture() -> Fix {
    Fix::heap(
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "Edge".into(),
                fields: ["src", "dst"]
                    .map(|name| FieldDescriptor {
                        name: name.into(),
                        value_type: ValueType::U64,
                    })
                    .to_vec(),
            }],
            statements: vec![],
        },
        &[(
            RelationId(0),
            (0..3)
                .map(|src| vec![Value::U64(src), Value::U64(src + 1)])
                .collect(),
        )],
    )
}

fn source_reach(through_interior: bool) -> Query {
    let edge = |source, src, dst| Atom {
        source,
        bindings: vec![(FieldId(0), src), (FieldId(1), dst)],
    };
    let interiors = if through_interior {
        vec![Interior {
            rules: vec![
                ProjectionRule {
                    finds: vec![VarId(0), VarId(1)],
                    atoms: vec![edge(
                        AtomSource::Edb(RelationId(0)),
                        Term::Var(VarId(0)),
                        Term::Var(VarId(1)),
                    )],
                    negated: vec![],
                    conditions: vec![],
                }
                .to_rule(),
            ],
        }]
    } else {
        vec![]
    };
    let source = if through_interior {
        AtomSource::Interior(InteriorId(0))
    } else {
        AtomSource::Edb(RelationId(0))
    };
    let rec_id = InteriorId(u32::from(through_interior));
    Query {
        interiors,
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![edge(source, Term::Param(ParamId(0)), Term::Var(VarId(0)))],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(1)],
                self_bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                atoms: vec![edge(source, Term::Var(VarId(0)), Term::Var(VarId(1)))],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(rec_id),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

fn reach_limits(prepared: &mut PreparedQuery<T>, rounds: u32, tuples: u64) {
    let PreparedPipeline::Reach { rounds_budget, .. } = &mut prepared.pipeline else {
        panic!("recursive pipeline");
    };
    *rounds_budget = rounds;
    prepared.tuples_budget = tuples;
}

fn node_ids(answers: &Answers) -> Vec<u64> {
    let mut ids: Vec<_> = (0..answers.len())
        .map(|row| {
            let AnswerValue::U64(id) = answers.get(row, 0) else {
                panic!("node id")
            };
            id
        })
        .collect();
    ids.sort_unstable();
    ids
}

/// Limits count interiors plus the distinct closure, not the sum of
/// repeated prefixes. Exact boundaries also discriminate the round
/// ceiling from the tuple ceiling, with atomic failure and handle reuse.
#[test]
fn reach_budgets_are_exact_and_refusal_does_not_poison_reuse() {
    let fix = chain_fixture();
    for through_interior in [false, true] {
        let total = if through_interior { 6 } else { 3 };
        for fallback in [false, true] {
            for ram in [0, usize::MAX] {
                let mut prepared = fix.prepare(&source_reach(through_interior)).unwrap();
                prepared.force_cursor_fallback(fallback);
                prepared.set_sink_ram(ram);
                let mut out = Answers::new();
                let params = [BindValue::U64(0)];
                for (rounds, tuples, succeeds) in [
                    (3, total, true),
                    (3, total - 1, false),
                    (2, total, false),
                    (3, total, true),
                ] {
                    reach_limits(&mut prepared, rounds, tuples);
                    let result = fix.execute_into(&mut prepared, &params, &mut out);
                    if succeeds {
                        result.unwrap();
                        assert_eq!(node_ids(&out), [1, 2, 3]);
                    } else {
                        assert!(
                            matches!(result, Err(Error::DerivedBudgetExceeded { rounds: 2, tuples }) if tuples == total),
                            "{result:?}"
                        );
                        assert!(out.is_empty(), "a refusal publishes no partial answers");
                    }
                    let PreparedPipeline::Reach { driver, .. } = &mut prepared.pipeline else {
                        unreachable!()
                    };
                    assert!(
                        driver.frontier.is_uniquely_owned(),
                        "a round must release every frontier consumer"
                    );
                }
            }
        }
    }
}

#[test]
fn an_empty_frontier_needs_no_round_or_tuple_allowance() {
    let fix = chain_fixture();
    let mut prepared = fix.prepare(&source_reach(false)).unwrap();
    reach_limits(&mut prepared, 0, 0);
    let answers = fix.execute(&mut prepared, &[BindValue::U64(99)]).unwrap();
    assert!(answers.is_empty());
}

#[test]
fn recursive_arms_share_one_immutable_frontier_and_one_exact_set() {
    let fix = chain_fixture();
    let mut query = source_reach(false);
    let rec = query.rec.as_mut().unwrap();
    let reverse = RecStep {
        finds: vec![VarId(1)],
        self_bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        atoms: vec![Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(1))),
                (FieldId(1), Term::Var(VarId(0))),
            ],
        }],
        conditions: vec![],
    };
    rec.rec.rest.push(reverse);
    for fallback in [false, true] {
        for ram in [0, usize::MAX] {
            let mut prepared = fix.prepare(&query).unwrap();
            prepared.force_cursor_fallback(fallback);
            prepared.set_sink_ram(ram);
            reach_limits(&mut prepared, 3, 4);
            for _ in 0..2 {
                let answers = fix.execute(&mut prepared, &[BindValue::U64(0)]).unwrap();
                assert_eq!(node_ids(&answers), [0, 1, 2, 3]);
                let PreparedPipeline::Reach { driver, .. } = &mut prepared.pipeline else {
                    unreachable!()
                };
                assert_eq!(
                    driver.rec.len(),
                    2,
                    "both different recursive arms survive planning"
                );
                assert!(driver.frontier.is_uniquely_owned());
            }
        }
    }
}

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
/// frontier keeps its watermark contract across the tier
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
