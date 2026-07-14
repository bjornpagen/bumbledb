//! The scan-fold pushdown's width freedom (PRD 04): the hoisted
//! projection emit and the leaf residual filter are column-hoisted over
//! the plan's own lists — no projection-arity cap, no residual-count
//! cap. These tests pin the >8-word and >8-residual classes over runs
//! long enough to engage the hoisted arms
//! ([`crate::exec::SCAN_HOIST_THRESHOLD`]), against both the
//! per-position arm and the generic batch path.

use super::*;
use crate::exec::SCAN_HOIST_THRESHOLD;
use crate::exec::colt::SuffixRun;

/// One relation of `fields` U64 columns f0..fn — the wide-projection
/// fixture ([`schema`] is binary-only).
fn wide_schema(fields: usize) -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "W".into(),
            fields: (0..fields)
                .map(|f| FieldDescriptor {
                    name: format!("f{f}").into(),
                    value_type: crate::schema::ValueType::U64,
                    generation: Generation::None,
                })
                .collect(),
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// Commits word rows into the wide relation and returns its view.
fn wide_views_of(
    dir: &TempDir,
    schema: &Schema,
    rows: &[Vec<u64>],
) -> Vec<Arc<crate::image::RelationImage>> {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for row in rows {
        let values: Vec<ValueRef> = row.iter().map(|w| ValueRef::U64(*w)).collect();
        let mut bytes = Vec::new();
        encode_fact(&values, schema.relation(RelationId(0)).layout(), &mut bytes);
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    vec![crate::image::build(&txn, schema, RelationId(0)).expect("build")]
}

/// The single-atom all-columns plan over the wide relation, plus the
/// projection slots in field order.
fn wide_plan(fields: u16) -> (NormalizedQuery, Vec<(u16, u16)>) {
    let vars: Vec<(u16, u16)> = (0..fields).map(|k| (k, k)).collect();
    (normalized(vec![occurrence(0, 0, &vars)], vec![]), vars)
}

/// Runs a plan through the real projection sink (the scan path) and
/// returns the distinct projected answers, `slots`-ordered.
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

/// Runs the same plan through [`CollectSink`] (declines scans — the
/// generic batch path) and projects its full bindings to `slots`.
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

/// A projection past 8 words over leaf runs past the hoist threshold —
/// the class the deleted `[ColumnView; 8]` scratch panicked on. The
/// hoisted emit is width-unbounded by construction; the batch path
/// (`CollectSink` projected) is the equality oracle.
#[test]
fn projection_past_eight_words_over_hoisted_runs() {
    let dir = TempDir::new("run-scan-wide");
    let fields = 10u16;
    let schema = wide_schema(usize::from(fields));
    let rows: Vec<Vec<u64>> = (0..20u64)
        .map(|i| (0..u64::from(fields)).map(|c| i * 100 + c).collect())
        .collect();
    assert!(rows.len() >= SCAN_HOIST_THRESHOLD, "the run must hoist");
    let views = wide_views_of(&dir, &schema, &rows);
    let (normalized, _) = wide_plan(fields);
    let plan = planned_with_sinks(&normalized, &schema, &[0], &all_vars(&normalized));
    let slots: Vec<usize> = (0..fields).map(|k| plan.slot_of(VarId(k))).collect();
    let expected: BTreeSet<Vec<u64>> = rows.into_iter().collect();
    assert_eq!(scan_rows_of(&plan, &views, &slots), expected);
    assert_eq!(batch_rows_of(&plan, &views, &slots), expected);
}

/// Hoisted vs per-position equality on IDENTICAL positions: one sink
/// consumes the whole span as a single hoisted run, the other the same
/// positions as fanout-sized runs (below the threshold — the
/// per-position arm). Same colt, same rows out.
#[test]
fn hoisted_and_per_position_arms_agree() {
    let dir = TempDir::new("run-scan-arms");
    let fields = 10u16;
    let schema = wide_schema(usize::from(fields));
    // Distinct facts (f0 = i); the projected suffix f1..f9 repeats per
    // `i % 6`, so the 9-word (>8) projection also exercises dedup.
    let rows: Vec<Vec<u64>> = (0..24u64)
        .map(|i| {
            std::iter::once(i)
                .chain((1..u64::from(fields)).map(|c| (i % 6) * 100 + c))
                .collect()
        })
        .collect();
    let views = wide_views_of(&dir, &schema, &rows);
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
    assert!(hoisted.begin_scan(&scan));
    hoisted.scan_run(&scan, SuffixRun::Identity { start: 0, len });
    assert_eq!(hoisted.end_scan(&scan), len as u64);
    let mut per_position = ProjectionSinkForTest::new(slots);
    assert!(per_position.begin_scan(&scan));
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

/// More leaf residuals than the deleted table held (9 > 8), over runs
/// past the hoist threshold: the residual-hoisted arm iterates the
/// plan's own list at any length. Verified against a naive reference
/// AND the batch path.
#[test]
fn leaf_scan_residuals_past_eight() {
    let dir = TempDir::new("run-scan-residuals");
    let schema = schema(2);
    // R0(a, x): a ∈ {1, 2}; R1(x, b): fanout 12 ≥ the threshold.
    let r0: Vec<(u64, u64)> = (0..6).map(|i| (i % 2 + 1, i % 3)).collect();
    let r1: Vec<(u64, u64)> = (0..36).map(|i| (i % 3, i / 3)).collect();
    let views = views_of(&dir, &schema, &[r0.clone(), r1.clone()]);
    // Nine residuals (the old cap was eight), jointly `b > a`: duplicate
    // specs are semantically idempotent, so the count is the only thing
    // under test.
    let residuals: Vec<PlacedComparison> = (0..9)
        .map(|k| PlacedComparison {
            op: if k % 2 == 0 { CmpOp::Ne } else { CmpOp::Ge },
            lhs: VarId(2),
            rhs: VarId(0),
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

/// Scan-path vs batch-path equality across the fixture set: fanouts
/// below and above the hoist threshold, with and without leaf
/// residuals — the projection sink's scan output always equals the
/// batch path's projected bindings.
#[test]
fn scan_and_batch_paths_agree_across_fixtures() {
    for (name, fanout, residuals) in [
        ("small-runs", 3u64, 0usize),
        ("hoisted-runs", 12, 0),
        ("small-runs-residuals", 3, 2),
        ("hoisted-runs-residuals", 12, 2),
    ] {
        let dir = TempDir::new(&format!("run-scan-equality-{name}"));
        let schema = schema(2);
        let r0: Vec<(u64, u64)> = (0..6).map(|i| (i % 2, i % 3)).collect();
        let r1: Vec<(u64, u64)> = (0..3 * fanout).map(|i| (i % 3, i / 3)).collect();
        let views = views_of(&dir, &schema, &[r0, r1]);
        let residuals: Vec<PlacedComparison> = (0..residuals)
            .map(|k| PlacedComparison {
                op: if k % 2 == 0 { CmpOp::Ne } else { CmpOp::Ge },
                lhs: VarId(2),
                rhs: VarId(0),
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
