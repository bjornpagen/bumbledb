//! Interval machinery in the executor — the membership point-var join
//! (`PlanNode::point_probes`), the Allen mask residuals over two-slot
//! interval variables (four endpoint slots + mask, classify-then-test),
//! and the decomposed point-containment word residuals
//! (docs/architecture/20-query-ir.md, § the Allen operator and
//! § normalization; 40-execution, § access paths).

use super::*;
use crate::allen::AllenMask;
use crate::image::view::{FilterPredicate, ResolvedWordSource};
use crate::ir::normalize::{IntervalWord, PlacedAllen, PlacedWordComparison, SlotWidth, VarWord};
use crate::ir::MaskTerm;
use crate::schema::ValueType;

/// Two relations of shape R(tag u64, during Interval<u64>).
fn tagged_interval_schema(relations: usize) -> Schema {
    SchemaDescriptor {
        relations: (0..relations)
            .map(|r| RelationDescriptor {
                name: format!("R{r}").into(),
                fields: vec![
                    FieldDescriptor {
                        name: "tag".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: crate::schema::IntervalElement::U64,
                        },
                        generation: Generation::None,
                    },
                ],
            })
            .collect(),
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// Commits (tag, [start, end)) rows per relation and builds images.
fn tagged_interval_views(
    dir: &TempDir,
    schema: &Schema,
    data: &[Vec<(u64, u64, u64)>],
) -> Vec<Arc<crate::image::RelationImage>> {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (rel, rows) in data.iter().enumerate() {
        let rel_id = RelationId(u32::try_from(rel).expect("small"));
        for (tag, start, end) in rows {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(*tag), ValueRef::IntervalU64(*start, *end)],
                schema.relation(rel_id).layout(),
                &mut bytes,
            );
            delta.insert(&view, rel_id, &bytes).expect("insert");
        }
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    (0..data.len())
        .map(|rel| {
            let rel_id = RelationId(u32::try_from(rel).expect("small"));
            crate::image::build(&txn, schema, rel_id).expect("build")
        })
        .collect()
}

/// A hand-assembled two-occurrence interval query:
/// `A(ta, i), B(tb, j)` with the given residuals — exactly the shapes
/// normalization emits for a cross-atom `Allen` (the mask residual) and
/// `Contains` point form (word comparisons), both pinned in
/// `ir/normalize/tests.rs`.
fn interval_pair_query(
    word_residuals: Vec<PlacedWordComparison>,
    allen_residuals: Vec<PlacedAllen>,
) -> NormalizedQuery {
    let occurrences = vec![
        Occurrence {
            occ_id: OccId(0),
            relation: RelationId(0),
            role: Role::Positive,
            vars: vec![(FieldId(0), VarId(0)), (FieldId(1), VarId(1))],
            filters: vec![],
        },
        Occurrence {
            occ_id: OccId(1),
            relation: RelationId(1),
            role: Role::Positive,
            vars: vec![(FieldId(0), VarId(2)), (FieldId(1), VarId(3))],
            filters: vec![],
        },
    ];
    let slot_widths: BTreeMap<VarId, SlotWidth> = [
        (VarId(0), SlotWidth::One),
        (VarId(1), SlotWidth::Two),
        (VarId(2), SlotWidth::One),
        (VarId(3), SlotWidth::Two),
    ]
    .into_iter()
    .collect();
    NormalizedQuery {
        occurrences,
        residuals: vec![],
        word_residuals,
        allen_residuals,
        anti_probes: vec![],
        slot_widths,
    }
}

fn side(var: u16, word: IntervalWord) -> VarWord {
    VarWord {
        var: VarId(var),
        word,
    }
}

/// The thirteen Allen configurations of A's interval against B's fixed
/// `[10, 20)`, tagged 1..=13 in Allen order.
const ALLEN: &[(u64, u64, u64)] = &[
    (1, 1, 5),    // before
    (2, 5, 10),   // meets (half-open: no shared point)
    (3, 5, 15),   // overlaps
    (4, 5, 20),   // finished-by
    (5, 5, 25),   // contains
    (6, 10, 15),  // starts
    (7, 10, 20),  // equals
    (8, 10, 25),  // started-by
    (9, 12, 18),  // during
    (10, 15, 20), // finishes
    (11, 15, 25), // overlapped-by
    (12, 20, 25), // met-by
    (13, 25, 30), // after
];

/// Runs an interval-pair query and returns the surviving A tags.
fn surviving_tags(
    name: &str,
    word_residuals: Vec<PlacedWordComparison>,
    allen_residuals: Vec<PlacedAllen>,
    order: &[u16],
) -> BTreeSet<u64> {
    let dir = TempDir::new(name);
    let schema = tagged_interval_schema(2);
    let views = tagged_interval_views(&dir, &schema, &[ALLEN.to_vec(), vec![(100, 10, 20)]]);
    let query = interval_pair_query(word_residuals, allen_residuals);
    let plan = planned_with_sinks(&query, &schema, order, &all_vars(&query));
    let rows = run(&plan, &views);
    let ta_slot = plan.slot_of(VarId(0));
    rows.iter().map(|row| row[ta_slot]).collect()
}

/// One Allen mask residual between the two occurrences' intervals.
fn allen_residual(mask: AllenMask) -> Vec<PlacedAllen> {
    vec![PlacedAllen {
        lhs: VarId(1),
        rhs: VarId(3),
        mask: MaskTerm::Literal(mask),
    }]
}

/// The point-containment word residuals (`Contains`' surviving point
/// form, `docs/architecture/20-query-ir.md` § normalization):
/// `i.start ≤ p AND p < i.end` over slot words — `p` reads B's interval
/// start word (10), so exactly the configurations containing the point
/// 10 survive, half-open at both boundaries.
#[test]
fn point_containment_word_residuals_evaluate_over_slot_words() {
    let contains_point = vec![
        PlacedWordComparison {
            op: CmpOp::Le,
            lhs: side(1, IntervalWord::Start),
            rhs: side(3, IntervalWord::Start),
        },
        PlacedWordComparison {
            op: CmpOp::Lt,
            lhs: side(3, IntervalWord::Start),
            rhs: side(1, IntervalWord::End),
        },
    ];
    let expected: BTreeSet<u64> = [3, 4, 5, 6, 7, 8].into_iter().collect();
    assert_eq!(
        surviving_tags(
            "run-contains-point-01",
            contains_point.clone(),
            vec![],
            &[0, 1]
        ),
        expected
    );
    assert_eq!(
        surviving_tags("run-contains-point-10", contains_point, vec![], &[1, 0]),
        expected
    );
}

/// `Allen(i, j, INTERSECTS)` — of the thirteen pairwise Allen
/// configurations exactly the nine sharing ones survive (meets/met-by
/// share no point under half-open intervals).
#[test]
fn intersects_mask_residual_keeps_exactly_the_nine_sharing_configurations() {
    let expected: BTreeSet<u64> = (3..=11).collect();
    assert_eq!(
        surviving_tags(
            "run-intersects-01",
            vec![],
            allen_residual(AllenMask::INTERSECTS),
            &[0, 1]
        ),
        expected
    );
    // Same answer with the join order flipped (placement recomputes).
    assert_eq!(
        surviving_tags(
            "run-intersects-10",
            vec![],
            allen_residual(AllenMask::INTERSECTS),
            &[1, 0]
        ),
        expected
    );
}

/// Every singleton mask keeps exactly its one configuration — the mask
/// residual pass classifies and bit-tests per element, so the thirteen
/// singletons partition the fixture (JEPD at the executor level). The
/// tag map follows the [`ALLEN`] fixture's row order.
#[test]
fn each_singleton_mask_residual_keeps_exactly_its_configuration() {
    use crate::allen::Basic;
    let expected_tag = [
        (Basic::Before, 1u64),
        (Basic::Meets, 2),
        (Basic::Overlaps, 3),
        (Basic::FinishedBy, 4),
        (Basic::Contains, 5),
        (Basic::Starts, 6),
        (Basic::Equals, 7),
        (Basic::StartedBy, 8),
        (Basic::During, 9),
        (Basic::Finishes, 10),
        (Basic::OverlappedBy, 11),
        (Basic::MetBy, 12),
        (Basic::After, 13),
    ];
    for (basic, tag) in expected_tag {
        let mask = AllenMask::new(basic.bit()).expect("in range");
        assert_eq!(
            surviving_tags(
                &format!("run-allen-basic-{tag}"),
                vec![],
                allen_residual(mask),
                &[0, 1]
            ),
            [tag].into_iter().collect::<BTreeSet<u64>>(),
            "{basic:?}"
        );
    }
}

/// `Allen(i, j, COVERS)` — exactly the ⊇ configurations survive:
/// finished-by, contains, equals, started-by.
#[test]
fn covers_mask_residual_keeps_exactly_the_containment_configurations() {
    let expected: BTreeSet<u64> = [4, 5, 7, 8].into_iter().collect();
    assert_eq!(
        surviving_tags(
            "run-covers-01",
            vec![],
            allen_residual(AllenMask::COVERS),
            &[0, 1]
        ),
        expected
    );
    assert_eq!(
        surviving_tags(
            "run-covers-10",
            vec![],
            allen_residual(AllenMask::COVERS),
            &[1, 0]
        ),
        expected
    );
}

/// Payroll(emp u64, during Interval<u64>); Event(emp u64, at u64).
fn membership_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Payroll".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: crate::schema::IntervalElement::U64,
                        },
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                name: "Event".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "at".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// The membership point-var join fixture
/// (`Payroll(emp, during ∋ t), Event(emp, at = t)`): the membership
/// lowers to a var-sourced `PointIn`, routed to a `point_probes` entry
/// at plan validation and evaluated as the point-membership scan inside
/// the join. Returns exactly the events whose time falls in the payroll
/// interval — both boundaries asserted (start in, end out).
#[test]
fn membership_point_var_join_keeps_exactly_the_contained_events() {
    let dir = TempDir::new("run-membership");
    let schema = membership_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for (emp, start, end) in [(1u64, 10u64, 20u64), (2, 30, 40)] {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(emp), ValueRef::IntervalU64(start, end)],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    for (emp, at) in [
        (1u64, 9u64),
        (1, 10), // == start: IN (half-open)
        (1, 15),
        (1, 19),
        (1, 20), // == end: OUT (half-open)
        (2, 30),
        (2, 39),
        (2, 40),
        (3, 35), // no payroll at all
    ] {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(emp), ValueRef::U64(at)],
            schema.relation(RelationId(1)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(1), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    let views: Vec<Arc<crate::image::RelationImage>> = (0..2)
        .map(|rel| crate::image::build(&txn, &schema, RelationId(rel)).expect("build"))
        .collect();

    // Exactly `normalize`'s lowering for the fixture (pinned in
    // ir/normalize/tests.rs): the membership binding binds no variable —
    // it is a var-sourced PointIn filter on the payroll occurrence.
    let x = VarId(0);
    let t = VarId(1);
    let occurrences = vec![
        Occurrence {
            occ_id: OccId(0),
            relation: RelationId(0),
            role: Role::Positive,
            vars: vec![(FieldId(0), x)],
            filters: vec![FilterPredicate::PointIn {
                field: FieldId(1),
                point: ResolvedWordSource::Var(t),
            }],
        },
        Occurrence {
            occ_id: OccId(1),
            relation: RelationId(1),
            role: Role::Positive,
            vars: vec![(FieldId(0), x), (FieldId(1), t)],
            filters: vec![],
        },
    ];
    let slot_widths: BTreeMap<VarId, SlotWidth> = [(x, SlotWidth::One), (t, SlotWidth::One)]
        .into_iter()
        .collect();
    let query = NormalizedQuery {
        occurrences,
        residuals: vec![],
        word_residuals: vec![],
        allen_residuals: Vec::new(),
        anti_probes: vec![],
        slot_widths,
    };

    let expected: BTreeSet<(u64, u64)> = [(1, 10), (1, 15), (1, 19), (2, 30), (2, 39)]
        .into_iter()
        .collect();
    for order in [[0u16, 1u16], [1, 0]] {
        let plan = planned_with_sinks(&query, &schema, &order, &all_vars(&query));
        // The var-sourced filter left the view filters for a placed
        // membership probe.
        assert!(plan.occurrences()[0].filters.is_empty());
        assert_eq!(plan.occurrences()[0].point_filters, vec![(FieldId(1), t)]);
        assert_eq!(
            plan.nodes()
                .iter()
                .map(|n| n.point_probes.len())
                .sum::<usize>(),
            1,
            "one membership probe, attached once"
        );
        let rows = run(&plan, &views);
        let got: BTreeSet<(u64, u64)> = rows
            .iter()
            .map(|row| (row[plan.slot_of(x)], row[plan.slot_of(t)]))
            .collect();
        assert_eq!(got, expected, "order {order:?}");
    }
}

/// The carried-cursor path: with a third atom between the payroll scan
/// and the point variable's binding node, the membership probe attaches
/// two nodes past the occurrence's last subatom — its advanced cursor
/// must ride the pipeline's carried set to the attachment node (the
/// pipe tables' cursor-USES extension).
#[test]
#[allow(clippy::too_many_lines)] // one fixture: the full three-node pipeline, linear
fn membership_probe_reads_a_carried_cursor_across_middle_nodes() {
    let dir = TempDir::new("run-membership-carried");
    // Payroll(emp, during); Dept(emp, dept); Event(emp, at) — reuse the
    // tagged-interval shape for payroll and the binary U64 shape for the
    // scalar relations.
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Payroll".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: crate::schema::IntervalElement::U64,
                        },
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                name: "Dept".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "dept".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                name: "Event".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "at".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for (emp, start, end) in [(1u64, 10u64, 20u64), (2, 30, 40)] {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(emp), ValueRef::IntervalU64(start, end)],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    for (rel, rows) in [
        (1u32, vec![(1u64, 100u64), (2, 200), (3, 300)]),
        (2, vec![(1, 9), (1, 10), (1, 19), (1, 20), (2, 39), (3, 15)]),
    ] {
        for (a, b) in rows {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(a), ValueRef::U64(b)],
                schema.relation(RelationId(rel)).layout(),
                &mut bytes,
            );
            delta
                .insert(&view, RelationId(rel), &bytes)
                .expect("insert");
        }
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    let views: Vec<Arc<crate::image::RelationImage>> = (0..3)
        .map(|rel| crate::image::build(&txn, &schema, RelationId(rel)).expect("build"))
        .collect();

    let (x, d, t) = (VarId(0), VarId(1), VarId(2));
    let occurrences = vec![
        Occurrence {
            occ_id: OccId(0),
            relation: RelationId(0),
            role: Role::Positive,
            vars: vec![(FieldId(0), x)],
            filters: vec![FilterPredicate::PointIn {
                field: FieldId(1),
                point: ResolvedWordSource::Var(t),
            }],
        },
        Occurrence {
            occ_id: OccId(1),
            relation: RelationId(1),
            role: Role::Positive,
            vars: vec![(FieldId(0), x), (FieldId(1), d)],
            filters: vec![],
        },
        Occurrence {
            occ_id: OccId(2),
            relation: RelationId(2),
            role: Role::Positive,
            vars: vec![(FieldId(0), x), (FieldId(1), t)],
            filters: vec![],
        },
    ];
    let slot_widths: BTreeMap<VarId, SlotWidth> = [
        (x, SlotWidth::One),
        (d, SlotWidth::One),
        (t, SlotWidth::One),
    ]
    .into_iter()
    .collect();
    let query = NormalizedQuery {
        occurrences,
        residuals: vec![],
        word_residuals: vec![],
        allen_residuals: Vec::new(),
        anti_probes: vec![],
        slot_widths,
    };
    // Payroll first, Dept between, Event last: T binds at the leaf, two
    // nodes past payroll's only subatom.
    let plan = planned_with_sinks(&query, &schema, &[0, 1, 2], &all_vars(&query));
    assert!(
        plan.nodes().last().expect("nonempty").point_probes.len() == 1,
        "the probe attaches at the node binding t"
    );
    let rows = run(&plan, &views);
    let got: BTreeSet<(u64, u64, u64)> = rows
        .iter()
        .map(|row| {
            (
                row[plan.slot_of(x)],
                row[plan.slot_of(d)],
                row[plan.slot_of(t)],
            )
        })
        .collect();
    let expected: BTreeSet<(u64, u64, u64)> = [(1, 100, 10), (1, 100, 19), (2, 200, 39)]
        .into_iter()
        .collect();
    assert_eq!(got, expected);
}

/// Negated membership (`not Payroll(emp, during ∋ t)`): the anti-probe
/// evaluates the var-sourced membership inside the probe — a binding is
/// rejected only if a payroll fact matches the key AND holds the point
/// (the existential reading over the negated occurrence's facts,
/// docs/architecture/20-query-ir.md).
#[test]
fn negated_membership_rejects_only_covered_events() {
    let dir = TempDir::new("run-anti-membership");
    let schema = membership_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for (emp, start, end) in [(1u64, 10u64, 20u64), (2, 30, 40)] {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(emp), ValueRef::IntervalU64(start, end)],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    let events = [(1u64, 9u64), (1, 10), (1, 19), (1, 20), (2, 35), (3, 15)];
    for (emp, at) in events {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(emp), ValueRef::U64(at)],
            schema.relation(RelationId(1)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(1), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    let views: Vec<Arc<crate::image::RelationImage>> = (0..2)
        .map(|rel| crate::image::build(&txn, &schema, RelationId(rel)).expect("build"))
        .collect();

    let (x, t) = (VarId(0), VarId(1));
    // Positive Event first (OccId 0), negated Payroll after — the
    // occurrence-numbering rule.
    let occurrences = vec![
        Occurrence {
            occ_id: OccId(0),
            relation: RelationId(1),
            role: Role::Positive,
            vars: vec![(FieldId(0), x), (FieldId(1), t)],
            filters: vec![],
        },
        Occurrence {
            occ_id: OccId(1),
            relation: RelationId(0),
            role: Role::Negated,
            vars: vec![(FieldId(0), x)],
            filters: vec![FilterPredicate::PointIn {
                field: FieldId(1),
                point: ResolvedWordSource::Var(t),
            }],
        },
    ];
    let query = NormalizedQuery {
        anti_probes: vec![AntiProbe {
            occurrence: OccId(1),
            probe_bindings: vec![(FieldId(0), x)],
        }],
        occurrences,
        residuals: vec![],
        word_residuals: vec![],
        allen_residuals: vec![],
        slot_widths: [(x, SlotWidth::One), (t, SlotWidth::One)]
            .into_iter()
            .collect(),
    };
    let plan = planned_with_sinks(&query, &schema, &[0], &all_vars(&query));
    let rows = run(&plan, &views);
    let got: BTreeSet<(u64, u64)> = rows
        .iter()
        .map(|row| (row[plan.slot_of(x)], row[plan.slot_of(t)]))
        .collect();
    // The complement of the covered events: boundaries flip — `at ==
    // start` is covered (rejected), `at == end` is not (kept).
    let expected: BTreeSet<(u64, u64)> = [(1, 9), (1, 20), (3, 15)].into_iter().collect();
    assert_eq!(got, expected);
}
