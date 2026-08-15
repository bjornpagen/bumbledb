use super::*;
use crate::image::view::FilterPredicate;
use crate::ir::normalize::OccBind;
use bumbledb_theory::schema::ValueType;

/// The batched membership probe at a MIDDLE node (`probe_pass`'s point
/// loop): a pinned-row cursor names one fact, so its point-in-interval
/// conjunction batch-evaluates over gathered interval columns; a node
/// cursor (payroll fanout) keeps the existential position walk. One
/// run exercises both splits with both half-open boundaries, against
/// the same query attached at the leaf as the regression twin.
#[test]
#[allow(clippy::too_many_lines)]
fn middle_node_membership_batches_pinned_rows_and_walks_fanouts() {
    let dir = TempDir::new("run-membership-batched");
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
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
                            element: bumbledb_theory::schema::IntervalElement::U64,
                        },
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
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
                extension: None,
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
    // emp 1 holds TWO payroll intervals (a node cursor at the probe —
    // the scalar walk); emp 2 holds one (a pinned row — the batch).
    for (emp, start, end) in [(1u64, 10u64, 20u64), (1, 30, 40), (2, 50, 60)] {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(emp),
                ValueRef::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
                ),
            ],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    for (rel, rows) in [
        (1u32, vec![(1u64, 100u64), (2, 200), (3, 300)]),
        (
            2,
            vec![
                (1u64, 9u64), // below both: OUT
                (1, 10),      // == start: IN (half-open)
                (1, 25),      // the gap between intervals: OUT
                (1, 30),      // second interval's start: IN
                (1, 39),      // interior: IN
                (1, 40),      // == end: OUT (half-open)
                (2, 50),      // == start (pinned row): IN
                (2, 55),      // interior (pinned row): IN
                (2, 60),      // == end (pinned row): OUT
                (3, 5),       // no payroll at all: OUT
            ],
        ),
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
            bind: OccBind::Edb(RelationId(0)),
            role: Role::Positive,
            vars: vec![(FieldId(0), x)],
            filters: vec![FilterPredicate::PointVar {
                field: FieldId(1),
                var: t,
            }],
        },
        Occurrence {
            occ_id: OccId(1),
            bind: OccBind::Edb(RelationId(1)),
            role: Role::Positive,
            vars: vec![(FieldId(0), x), (FieldId(1), d)],
            filters: vec![],
        },
        Occurrence {
            occ_id: OccId(2),
            bind: OccBind::Edb(RelationId(2)),
            role: Role::Positive,
            vars: vec![(FieldId(0), x), (FieldId(1), t)],
            filters: vec![],
        },
    ];
    let slot_widths: BTreeMap<VarId, SlotWidth> = [
        (x, SlotWidth::ONE),
        (d, SlotWidth::ONE),
        (t, SlotWidth::ONE),
    ]
    .into_iter()
    .collect();
    let query = NormalizedQuery {
        dead: None,
        occurrences,
        residuals: vec![],
        word_residuals: vec![],
        allen_residuals: Vec::new(),
        duration_residuals: Vec::new(),
        anti_probes: vec![],
        slot_widths,
    };
    let expected: BTreeSet<(u64, u64, u64)> = [
        (1, 100, 10),
        (1, 100, 30),
        (1, 100, 39),
        (2, 200, 50),
        (2, 200, 55),
    ]
    .into_iter()
    .collect();
    // Payroll, Event, Dept: t binds at node 1 — a MIDDLE node, the
    // batched loop under test. Payroll, Dept, Event attaches at the
    // leaf — run_node's loop, the regression twin.
    for (order, middle) in [([0u16, 2, 1], true), ([0, 1, 2], false)] {
        let plan = planned_with_sinks(&query, &schema, &order, &all_vars(&query));
        let probe_node = plan
            .nodes()
            .iter()
            .position(|n| !n.point_probes.is_empty())
            .expect("one membership probe attached");
        assert_eq!(
            probe_node < plan.nodes().len() - 1,
            middle,
            "order {order:?} pins the intended loop"
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
        assert_eq!(got, expected, "order {order:?}");
    }
}

/// The pipeline's gather/assembly work is phase-attributed (Gap B):
/// under `PhaseTimers`, `pump`'s cover iteration and `probe_pass`'s
/// batch assembly land in `jp_gather` windows instead of vanishing
/// between the timed phases — a deep plan's formerly unattributed
/// half. Zero-cost off stands untouched: the window calls are
/// `Counters` defaults, compiled to nothing under `NoopCounters`.
#[cfg(feature = "trace")]
#[test]
fn pump_gather_windows_are_attributed() {
    let dir = TempDir::new("run-gather-phase");
    let schema = schema(3);
    let r: Vec<(u64, u64)> = (0..64u64).map(|i| (i, i % 8)).collect();
    let s: Vec<(u64, u64)> = (0..8u64)
        .flat_map(|y| (0..8u64).map(move |j| (y, y * 8 + j)))
        .collect();
    let t: Vec<(u64, u64)> = (0..64u64).map(|z| (z, z)).collect();
    let views = views_of(&dir, &schema, &[r, s, t]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 3)]),
        ],
        vec![],
    );
    let sinks = all_vars(&normalized);
    let plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2], &sinks);
    let mut executor = Executor::new(&plan);
    assert!(
        matches!(executor.drive, super::super::Drive::Pipeline(_)),
        "pipeline dispatched"
    );
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    crate::obs::start_capture();
    let mut timers = PhaseTimers::new();
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut timers)
        .expect("execute");
    timers.flush();
    let events = crate::obs::finish_capture();
    assert!(!sink.rows.is_empty());
    // Both pumped nodes attribute gather windows: the virtual root's
    // pass (node 0) and the middle node's (node 1).
    for name in ["jp_gather_n0", "jp_gather_n1"] {
        let event = events
            .iter()
            .find(|e| e.name() == name)
            .unwrap_or_else(|| panic!("{name} attributed"));
        assert!(event.a1() > 0, "{name} counts its windows");
    }
}

/// Zero-yield cover draws are not batches. An entry whose cover holds an
/// exact multiple of the batch size drains at one full batch plus one
/// empty resume draw (the token must be re-presented to learn the entry
/// is exhausted), and `pump` used to count that empty draw — its
/// `run_node` twin breaks before counting — skewing the
/// `batches/batch_entries` observable ("batching engaged" means
/// batches ≪ entries) low on exact-fit fanouts.
#[test]
fn zero_yield_draws_are_not_batches() {
    #[derive(Default)]
    struct BatchLens {
        batches: u64,
        entries: u64,
        zero_len: u64,
    }
    impl Counters for BatchLens {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, len: usize) {
            self.batches += 1;
            self.entries += u64::try_from(len).expect("batch fits u64");
            self.zero_len += u64::from(len == 0);
        }
        fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn anti_probe(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    let dir = TempDir::new("run-batch-accounting");
    let schema = schema(3);
    // Every fanout is an exact multiple of the batch size 4: the root
    // holds 8 R rows (two full draws), and each middle-node entry's S
    // cover holds exactly 4 children — one full draw plus the empty
    // resume the counter must not book.
    let r: Vec<(u64, u64)> = (0..8u64).map(|i| (i, i % 2)).collect();
    let s: Vec<(u64, u64)> = (0..2u64)
        .flat_map(|y| (0..4u64).map(move |j| (y, y * 4 + j)))
        .collect();
    let t: Vec<(u64, u64)> = (0..8u64).map(|z| (z, z)).collect();
    let views = views_of(&dir, &schema, &[r, s, t]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 3)]),
        ],
        vec![],
    );
    let sinks = all_vars(&normalized);
    let plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2], &sinks);
    let mut executor = Executor::with_batch_size(&plan, 4);
    assert!(
        matches!(executor.drive, super::super::Drive::Pipeline(_)),
        "pipeline dispatched"
    );
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = BatchLens::default();
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");
    assert_eq!(sink.rows.len(), 32, "8 parents x 4 children, T total");
    assert!(counters.batches > 0, "the join drew batches");
    assert_eq!(
        counters.zero_len, 0,
        "an exhausted resume draw is not a batch (batches {}, entries {})",
        counters.batches, counters.entries
    );
}
