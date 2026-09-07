use super::*;
use crate::exec::SCAN_HOIST_THRESHOLD;
use crate::exec::colt::SuffixRun;
use crate::ir::WordCmp;

fn wide_schema(fields: usize) -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "W".into(),
            fields: (0..fields)
                .map(|f| FieldDescriptor {
                    name: format!("f{f}").into(),
                    value_type: bumbledb_theory::schema::ValueType::U64,
                })
                .collect(),
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn wide_views_of(schema: &Schema, rows: &[Vec<u64>]) -> Vec<Arc<crate::image::RelationImage>> {
    let facts: Vec<Vec<crate::ir::Value>> = rows
        .iter()
        .map(|row| row.iter().map(|w| crate::ir::Value::U64(*w)).collect())
        .collect();
    let source = TestSource::new(schema, &[(RelationId(0), facts)]);
    let (_cache, image) = source.image_with_cache(RelationId(0));
    vec![image]
}

fn wide_plan(fields: u16) -> (NormalizedQuery, Vec<(u16, u16)>) {
    let vars: Vec<(u16, u16)> = (0..fields).map(|k| (k, k)).collect();
    (normalized(vec![occurrence(0, 0, &vars)], vec![]), vars)
}

fn scan_rows_of(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    slots: &[usize],
) -> BTreeSet<Vec<u64>> {
    let mut colts = colts_for(plan, views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = ProjectionSinkForTest::new(slots.to_vec());
    let mut executor = Executor::new(plan);
    executor
        .execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        )
        .expect("execute");
    sink.answers().map(<[u64]>::to_vec).collect()
}

fn batch_rows_of(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    slots: &[usize],
) -> BTreeSet<Vec<u64>> {
    run(plan, views)
        .iter()
        .map(|row| slots.iter().map(|slot| row[*slot]).collect())
        .collect()
}

#[test]
fn projection_past_eight_words_over_hoisted_runs() {
    let fields = 10u16;
    let schema = wide_schema(usize::from(fields));
    let rows: Vec<Vec<u64>> = (0..20u64)
        .map(|i| (0..u64::from(fields)).map(|c| i * 100 + c).collect())
        .collect();
    assert!(rows.len() >= SCAN_HOIST_THRESHOLD, "the run must hoist");
    let views = wide_views_of(&schema, &rows);
    let (normalized, _) = wide_plan(fields);
    let plan = planned_with_sinks(&normalized, &schema, &[0], &all_vars(&normalized));
    let slots: Vec<usize> = (0..fields).map(|k| plan.slot_of(VarId(k))).collect();
    let expected: BTreeSet<Vec<u64>> = rows.into_iter().collect();
    assert_eq!(scan_rows_of(&plan, &views, &slots), expected);
    assert_eq!(batch_rows_of(&plan, &views, &slots), expected);
}

#[test]
fn hoisted_and_per_position_arms_agree() {
    let fields = 10u16;
    let schema = wide_schema(usize::from(fields));

    let rows: Vec<Vec<u64>> = (0..24u64)
        .map(|i| {
            std::iter::once(i)
                .chain((1..u64::from(fields)).map(|c| (i % 6) * 100 + c))
                .collect()
        })
        .collect();
    let views = wide_views_of(&schema, &rows);
    let (normalized, _) = wide_plan(fields);
    let plan = planned_with_sinks(&normalized, &schema, &[0], &all_vars(&normalized));
    let slots: Vec<usize> = (1..fields).map(|k| plan.slot_of(VarId(k))).collect();
    let colts = colts_for(&plan, &views);
    let key_slots: Vec<usize> = plan.occurrences()[0].trie_schema[0]
        .iter()
        .map(|var| plan.slot_of(*var))
        .collect();
    let bindings = Bindings::new(plan.slot_count());
    let scan = LeafScan {
        colt: &colts[0],
        level: 0,
        key_slots: &key_slots,
        bindings: &bindings,
    };
    let len = views[0].row_count();
    let mut hoisted = ProjectionSinkForTest::new(slots.clone());
    hoisted.prepare_scan(&key_slots);
    assert_eq!(hoisted.begin_scan(&scan), crate::exec::run::ScanOffer::Open);
    hoisted.scan_run(&scan, SuffixRun::Identity { start: 0, len });
    assert_eq!(hoisted.end_scan(&scan), len as u64);
    let mut per_position = ProjectionSinkForTest::new(slots);
    per_position.prepare_scan(&key_slots);
    assert_eq!(
        per_position.begin_scan(&scan),
        crate::exec::run::ScanOffer::Open
    );
    let positions: Vec<u32> = (0..u32::try_from(len).expect("small")).collect();
    for chunk in positions.chunks(SCAN_HOIST_THRESHOLD - 1) {
        per_position.scan_run(&scan, SuffixRun::Positions(chunk));
    }
    assert_eq!(per_position.end_scan(&scan), len as u64);
    let answers_of = |sink: &ProjectionSinkForTest| -> BTreeSet<Vec<u64>> {
        sink.answers().map(<[u64]>::to_vec).collect()
    };
    assert_eq!(answers_of(&hoisted), answers_of(&per_position));
    assert_eq!(answers_of(&hoisted).len(), 6, "dedup still holds");
}

fn mixed_scan_colt() -> Colt {
    use bumbledb_theory::schema::ValueType;
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Mixed".into(),
            fields: [("flag", ValueType::Bool), ("amount", ValueType::U64)]
                .into_iter()
                .map(|(name, value_type)| FieldDescriptor {
                    name: name.into(),
                    value_type,
                })
                .collect(),
        }],
        statements: vec![],
    }
    .validate()
    .unwrap();
    let rows = [(false, 10), (true, 20), (true, 30)]
        .map(|(flag, amount)| vec![crate::ir::Value::Bool(flag), crate::ir::Value::U64(amount)]);
    let source = TestSource::new(&schema, &[(RelationId(0), rows.into())]);
    let (_cache, image) = source.image_with_cache(RelationId(0));
    Colt::new(
        crate::image::view::View::Bound(crate::image::view::BoundView::All(image)),
        &[],
        vec![vec![0, 1]],
    )
}

fn assert_scan_run(
    sink: &mut ProjectionSinkForTest,
    colt: &Colt,
    key_slots: &[usize],
    bindings: &Bindings,
    run: SuffixRun<'_>,
) {
    let expected = run.len() as u64;
    let scan = LeafScan {
        colt,
        level: 0,
        key_slots,
        bindings,
    };
    assert_eq!(sink.begin_scan(&scan), ScanOffer::Open);
    sink.scan_run(&scan, run);
    assert_eq!(sink.end_scan(&scan), expected);
}

#[test]
fn prepared_scan_routing_survives_batches_and_changing_outer_values() {
    let colt = mixed_scan_colt();
    assert!(matches!(
        colt.suffix_column(0, 0),
        crate::image::ColumnView::Bytes(_)
    ));
    assert!(matches!(
        colt.suffix_column(0, 1),
        crate::image::ColumnView::Words(_)
    ));
    let mut bindings = Bindings::new(3);
    bindings.set(0, 100);
    let mut sink = ProjectionSinkForTest::new(vec![0, 1, 2]);
    sink.prepare_scan(&[1, 2]);
    assert_scan_run(
        &mut sink,
        &colt,
        &[1, 2],
        &bindings,
        SuffixRun::Identity { start: 0, len: 3 },
    );
    // A combined pinned batch maps every output to a key, unlike this
    // scan's outer/byte-column/word-column routing. It must not replace it.
    let batch = LeafBatch {
        keys: &[900, 1, 40],
        arity: 3,
        survivors: &[0],
        key_slots: &[0, 1, 2],
        bindings: &bindings,
    };
    assert!(!sink.emit_batch(&batch).is_terminal());
    bindings.set(0, 200);
    assert_scan_run(
        &mut sink,
        &colt,
        &[1, 2],
        &bindings,
        SuffixRun::Positions(&[2]),
    );
    assert_scan_run(
        &mut sink,
        &colt,
        &[1, 2],
        &bindings,
        SuffixRun::Positions(&[]),
    );
    let expected: BTreeSet<_> = [
        vec![100, 0, 10],
        vec![100, 1, 20],
        vec![100, 1, 30],
        vec![900, 1, 40],
        vec![200, 1, 30],
    ]
    .into();
    assert_eq!(
        sink.answers().map(<[u64]>::to_vec).collect::<BTreeSet<_>>(),
        expected
    );
}

#[test]
fn scan_routing_is_prepared_again_after_rule_aim_and_reset() {
    let colt = mixed_scan_colt();
    let mut bindings = Bindings::new(3);
    bindings.set(0, 100);
    let mut sink = ProjectionSinkForTest::new(vec![0, 1, 2]);
    sink.prepare_scan(&[1, 2]);
    assert_scan_run(
        &mut sink,
        &colt,
        &[1, 2],
        &bindings,
        SuffixRun::Positions(&[0]),
    );
    sink.reset();
    let finds = [2, 0, 1].map(|slot| crate::exec::sink::FindSpec::Var { slot, width: 1 });
    sink.aim(&finds);
    bindings.set(1, 777);
    let scan = LeafScan {
        colt: &colt,
        level: 0,
        key_slots: &[0, 2],
        bindings: &bindings,
    };
    assert_eq!(
        sink.begin_scan(&scan),
        ScanOffer::Declined,
        "aim invalidates old routing"
    );
    sink.prepare_scan(&[0, 2]);
    assert_scan_run(
        &mut sink,
        &colt,
        &[0, 2],
        &bindings,
        SuffixRun::Identity { start: 0, len: 1 },
    );
    bindings.set(1, 888);
    assert_scan_run(
        &mut sink,
        &colt,
        &[0, 2],
        &bindings,
        SuffixRun::Positions(&[2]),
    );
    assert_eq!(
        sink.answers().map(<[u64]>::to_vec).collect::<BTreeSet<_>>(),
        [vec![10, 0, 777], vec![30, 1, 888]].into()
    );
}

#[test]
fn leaf_scan_residuals_past_eight() {
    let schema = schema(2);

    let r0: Vec<(u64, u64)> = (0..6).map(|i| (i % 2 + 1, i % 3)).collect();
    let r1: Vec<(u64, u64)> = (0..36).map(|i| (i % 3, i / 3)).collect();
    let views = views_of(&schema, &[r0.clone(), r1.clone()]);

    let residuals: Vec<FilterPredicate> = (0..9)
        .map(|k| FilterPredicate::FieldsCompare {
            op: if k % 2 == 0 { WordCmp::Ne } else { WordCmp::Ge },
            left: OperandAddr::from(VarId(2)),
            right: OperandAddr::from(VarId(0)),
        })
        .collect();
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
        ],
        residuals,
    );
    let plan = planned_with_sinks(&normalized, &schema, &[0, 1], &all_vars(&normalized));
    let slots: Vec<usize> = (0..3).map(|k| plan.slot_of(VarId(k))).collect();
    let mut expected = BTreeSet::new();
    for (a, x) in &r0 {
        for (x2, b) in &r1 {
            if x2 == x && b != a && b >= a {
                expected.insert(vec![*a, *x, *b]);
            }
        }
    }
    assert!(!expected.is_empty(), "the fixture joins");
    assert_eq!(scan_rows_of(&plan, &views, &slots), expected);
    assert_eq!(batch_rows_of(&plan, &views, &slots), expected);
}

#[test]
fn scan_and_batch_paths_agree_across_fixtures() {
    for (name, fanout, residuals) in [
        ("small-runs", 3u64, 0usize),
        ("hoisted-runs", 12, 0),
        ("small-runs-residuals", 3, 2),
        ("hoisted-runs-residuals", 12, 2),
    ] {
        let schema = schema(2);
        let r0: Vec<(u64, u64)> = (0..6).map(|i| (i % 2, i % 3)).collect();
        let r1: Vec<(u64, u64)> = (0..3 * fanout).map(|i| (i % 3, i / 3)).collect();
        let views = views_of(&schema, &[r0, r1]);
        let residuals: Vec<FilterPredicate> = (0..residuals)
            .map(|k| FilterPredicate::FieldsCompare {
                op: if k % 2 == 0 { WordCmp::Ne } else { WordCmp::Ge },
                left: OperandAddr::from(VarId(2)),
                right: OperandAddr::from(VarId(0)),
            })
            .collect();
        let normalized = normalized(
            vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
            ],
            residuals,
        );
        let plan = planned_with_sinks(&normalized, &schema, &[0, 1], &all_vars(&normalized));
        let slots: Vec<usize> = (0..3).map(|k| plan.slot_of(VarId(k))).collect();
        let scan = scan_rows_of(&plan, &views, &slots);
        assert!(!scan.is_empty(), "{name}: the fixture joins");
        assert_eq!(scan, batch_rows_of(&plan, &views, &slots), "{name}");
    }
}

fn scan_work(units: u64) -> crate::work::WorkContext {
    crate::work::ExecutionPolicy {
        input_bytes: u64::MAX,
        working_bytes: u64::MAX,
        scratch_bytes: u64::MAX,
        result_bytes: u64::MAX,
        rows: u64::MAX,
        work_units: units,
        timeout: std::time::Duration::from_secs(3600),
    }
    .start()
    .expect("valid scan work")
}

#[derive(Default)]
struct ScanCounters {
    cancel: Option<crate::work::WorkContext>,
    residuals: usize,
    batches: Vec<usize>,
    emitted: u64,
}

impl Counters for ScanCounters {
    fn node_entry(&mut self, _: usize) {}
    fn batch(&mut self, _: usize, len: usize) {
        self.batches.push(len);
    }
    fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
    fn probe_hash(&mut self, _: usize, _: usize) {}
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    fn residual(&mut self, _: usize, _: bool) {
        self.residuals += 1;
        if self.residuals == 17
            && let Some(work) = &self.cancel
        {
            work.cancel();
        }
    }
    fn anti_probe(&mut self, _: usize, _: bool) {}
    fn emit(&mut self) {
        self.emitted += 1;
    }
    fn skip(&mut self, _: usize) {}
}

struct FusedScanSink {
    projection: ProjectionSinkForTest,
    prepares: usize,
    begins: usize,
    ends: usize,
    runs: Vec<usize>,
    fail_run: bool,
    fail_end: bool,
    error: Option<crate::error::Error>,
}

impl FusedScanSink {
    fn new(plan: &ValidatedPlan) -> Self {
        Self {
            projection: ProjectionSinkForTest::new(
                [VarId(0), VarId(1)].map(|var| plan.slot_of(var)).to_vec(),
            ),
            prepares: 0,
            begins: 0,
            ends: 0,
            runs: Vec::new(),
            fail_run: false,
            fail_end: false,
            error: None,
        }
    }

    fn fail(&mut self) {
        self.error = Some(crate::error::Error::Corruption(
            crate::error::CorruptionError::MalformedValue("injected fused scan failure"),
        ));
    }
}

impl Sink for FusedScanSink {
    fn prepare_scan(&mut self, key_slots: &[usize]) {
        self.prepares += 1;
        self.projection.prepare_scan(key_slots);
    }

    fn emit(&mut self, _: &Bindings) -> Flow {
        panic!("a supported fused scan must not fall through after refusal")
    }
    fn emit_batch(&mut self, _: &LeafBatch<'_>) -> Flow {
        panic!("a supported fused scan must not fall through to batch execution")
    }
    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> ScanOffer {
        self.begins += 1;
        self.projection.begin_scan(scan)
    }
    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        self.runs.push(run.len());
        self.projection.scan_run(scan, run);
        if self.fail_run {
            self.fail();
        }
    }
    fn end_scan(&mut self, scan: &LeafScan<'_>) -> u64 {
        self.ends += 1;
        if self.fail_end {
            self.fail();
            return 0;
        }
        self.projection.end_scan(scan)
    }
    fn progress(&self) -> crate::exec::sink::SinkProgress {
        if self.error.is_some() {
            crate::exec::sink::SinkProgress::Error
        } else {
            self.projection.progress()
        }
    }
    fn take_error(&mut self) -> Option<crate::error::Error> {
        self.error.take().or_else(|| self.projection.take_error())
    }
}

fn fused_scan_plan(residual: bool) -> (Schema, ValidatedPlan) {
    let schema = schema(1);
    let normalized = normalized(
        vec![occurrence(0, 0, &[(0, 0), (1, 1)])],
        if residual {
            vec![FilterPredicate::FieldsCompare {
                op: WordCmp::Lt,
                left: OperandAddr::from(VarId(0)),
                right: OperandAddr::from(VarId(1)),
            }]
        } else {
            vec![]
        },
    );
    let plan = planned_with_sinks(&normalized, &schema, &[0], &all_vars(&normalized));
    assert!(matches!(
        Executor::new(&plan).leaf,
        LeafPrecompute::Fast { .. }
    ));
    (schema, plan)
}

#[test]
fn rejecting_fused_scan_stops_within_one_quantum_without_sink_or_fallback() {
    use crate::work::{Resource, WorkError};
    let quantum = crate::exec::sink::STEP_QUANTUM as usize;
    let (schema, plan) = fused_scan_plan(true);
    let rows: Vec<_> = (0..2049).map(|i| (i + 1, 0)).collect();
    let views = views_of(&schema, &[rows]);
    for (positions, pending) in [(false, 0), (true, 0), (false, 31), (true, 31)] {
        for cancellation in [false, true] {
            let work = scan_work(if cancellation {
                u64::MAX
            } else {
                (quantum - 1) as u64
            });
            let mut colts = colts_for(&plan, &views);
            if positions {
                drop(colts[0].reset(crate::image::view::View::Bound(
                    crate::image::view::BoundView::Survivors {
                        image: Arc::clone(&views[0]),
                        positions: (0..2049).collect(),
                    },
                )));
            }
            let mut executor = Executor::new(&plan);
            executor.begin_work(&work, &mut colts);
            assert!(executor.note_explored(pending, &colts));
            let mut bindings = Bindings::new(plan.slot_count());
            let mut sink = FusedScanSink::new(&plan);
            let mut counters = ScanCounters {
                cancel: cancellation.then(|| work.clone()),
                ..ScanCounters::default()
            };
            let error = executor
                .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
                .expect_err("bounded rejecting scan refuses");
            let crate::error::Error::Store(error) = error else {
                panic!("expected typed work refusal");
            };
            if cancellation {
                assert!(matches!(
                    *error,
                    crate::storage::store::StoreError::Work(WorkError::Cancelled)
                ));
            } else {
                assert!(matches!(
                    *error,
                    crate::storage::store::StoreError::Work(WorkError::Exhausted {
                        resource: Resource::WorkUnits,
                        ..
                    })
                ));
            }
            assert_eq!(
                counters.residuals,
                quantum - pending,
                "the previous short run cannot stretch the polling quantum"
            );
            assert_eq!(counters.emitted, 0);
            assert_eq!(sink.begins, 1);
            assert_eq!(sink.prepares, 1);
            assert_eq!(sink.ends, 0, "refused scans do not finalize partial groups");
            assert!(sink.runs.is_empty(), "all positions were rejected");
            assert!(
                executor.ledger.is_none(),
                "execute still finishes its ledger"
            );
        }
    }
}

#[test]
fn bounded_fused_scan_preserves_answers_physical_counters_and_tail_work() {
    use crate::work::Resource;
    let quantum = crate::exec::sink::STEP_QUANTUM as usize;
    for residual in [false, true] {
        let (schema, plan) = fused_scan_plan(residual);
        let rows: Vec<_> = (0..1025)
            .map(|i| (i, if i % 2 == 0 { i + 1 } else { 0 }))
            .collect();
        let expected: BTreeSet<_> = rows
            .iter()
            .filter(|(a, b)| !residual || a < b)
            .map(|&(a, b)| vec![a, b])
            .collect();
        let views = views_of(&schema, &[rows]);
        let mut colts = colts_for(&plan, &views);
        let work = scan_work(u64::MAX);
        let mut executor = Executor::new(&plan);
        executor.begin_work(&work, &mut colts);
        let mut bindings = Bindings::new(plan.slot_count());
        let mut sink = FusedScanSink::new(&plan);
        let mut counters = ScanCounters::default();
        executor
            .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
            .expect("successful fused scan");
        let actual: BTreeSet<_> = sink.projection.answers().map(<[u64]>::to_vec).collect();
        assert_eq!(actual, expected);
        assert_eq!(sink.begins, 1);
        assert_eq!(sink.ends, 1);
        assert!(sink.runs.iter().all(|&len| len <= quantum));
        assert_eq!(
            counters.batches,
            vec![1025],
            "polling does not invent COLT batches"
        );
        assert_eq!(counters.residuals, if residual { 1025 } else { 0 });
        assert_eq!(counters.emitted, expected.len() as u64);
        assert_eq!(
            work.used(Resource::WorkUnits),
            1025,
            "end_work flushes the tail once"
        );
        assert!(executor.ledger.is_none());
    }
}

#[test]
fn fused_scan_propagates_run_and_end_failures_without_successful_fallback() {
    let quantum = crate::exec::sink::STEP_QUANTUM as usize;
    for fail_end in [false, true] {
        let (schema, plan) = fused_scan_plan(false);
        let views = views_of(&schema, &[(0..1025).map(|i| (i, i)).collect()]);
        let mut colts = colts_for(&plan, &views);
        let mut executor = Executor::new(&plan);
        let work = scan_work(u64::MAX);
        executor.begin_work(&work, &mut colts);
        let mut bindings = Bindings::new(plan.slot_count());
        let mut sink = FusedScanSink::new(&plan);
        sink.fail_run = !fail_end;
        sink.fail_end = fail_end;
        let mut counters = ScanCounters::default();
        let error = executor
            .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
            .expect_err("sink failure cannot become a successful scan");
        assert!(matches!(
            error,
            crate::error::Error::Corruption(crate::error::CorruptionError::MalformedValue(
                "injected fused scan failure"
            ))
        ));
        if fail_end {
            assert_eq!(sink.runs.iter().sum::<usize>(), 1025);
            assert_eq!(sink.ends, 1);
        } else {
            assert_eq!(sink.runs, vec![quantum], "later windows are not visited");
            assert_eq!(sink.ends, 0);
        }
        assert_eq!(counters.emitted, 0);
        assert!(executor.ledger.is_none());
    }
}

#[test]
fn suffix_traversal_stops_chunk_chains_and_distinguishes_unsupported() {
    use std::ops::ControlFlow;
    let schema = schema(1);
    let views = views_of(&schema, &[(0..1025).map(|i| (0, i)).collect()]);
    let mut colt = Colt::new(
        crate::image::view::View::Bound(crate::image::view::BoundView::All(Arc::clone(&views[0]))),
        &[],
        vec![vec![0], vec![1]],
    );
    let cursor = colt.get(Colt::root(), 0, &[0]).expect("one outer key");
    let mut runs = Vec::new();
    assert_eq!(
        colt.for_each_suffix_run(cursor, |run| {
            runs.push(run.len());
            ControlFlow::<()>::Continue(())
        }),
        ControlFlow::Continue(true)
    );
    assert!(runs.len() > 1, "fixture traverses a real chunk chain");
    assert_eq!(runs.iter().sum::<usize>(), 1025);
    let mut visited = 0;
    assert_eq!(
        colt.for_each_suffix_run(cursor, |_| {
            visited += 1;
            ControlFlow::Break(17)
        }),
        ControlFlow::Break(17)
    );
    assert_eq!(visited, 1, "no later chunk callback after stop");
    assert_eq!(
        colt.for_each_suffix_run(Colt::root(), |_| -> ControlFlow<()> {
            panic!("forced roots are unsupported, not empty scans")
        }),
        ControlFlow::Continue(false)
    );
}
