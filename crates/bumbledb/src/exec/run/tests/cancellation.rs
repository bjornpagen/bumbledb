use super::*;

/// PRD 10 (docs/perf/): D2 under the pipeline — two parents
/// interleave in one batch, one parent's suffix skips, and the other
/// parent's rows all emit. The absorb node sits above a
/// non-sink-relevant middle node, so cancellation crosses a level.
#[test]
fn pipelined_d2_cancels_one_origin_and_spares_the_rest() {
    let dir = TempDir::new("run-pipe-d2");
    let schema = schema(3);
    // R(x, y): two x groups fan out through y; S(y, z) multiplies
    // witnesses; T(z, w) leaf binds nothing projected. Projecting x
    // only: n0 (binds x, y? — order [0,1,2] makes n0 bind x,y) is
    // sink-relevant via x; n1 (z) and n2 (w) are not — a leaf skip
    // cancels one n0-element subtree.
    let r: Vec<(u64, u64)> = vec![(1, 10), (1, 11), (2, 10)];
    let s: Vec<(u64, u64)> = (0..40).map(|i| (10 + (i % 2), i)).collect();
    let t: Vec<(u64, u64)> = (0..40).map(|i| (i, 900 + i)).collect();
    let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 3)]),
        ],
        vec![],
    );
    // Sink vars: x only.
    let sinks: BTreeSet<VarId> = [VarId(0)].into();
    let plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2], &sinks);
    assert!(!plan.skip_free(), "the D2 shape");
    for batch in [1usize, 2, 128] {
        let mut executor = Executor::with_batch_size(&plan, batch);
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slot_count());
        let mut sink = ProjectionSinkForTest::new(vec![plan.slot_of(VarId(0))]);
        let mut counters = SkipCounterRun::default();
        executor.execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);
        let mut rows: Vec<u64> = sink.rows_first_col();
        rows.sort_unstable();
        assert_eq!(rows, vec![1, 2], "batch {batch}: both x groups present");
        assert!(counters.skips > 0, "batch {batch}: skips fired");
    }
}

/// PRD 10's randomized differential: subset projections force real
/// D2 skips through the pipeline — random instances, orders, and
/// batch sizes against the nested-loop oracle's projected sets.
/// (This is the harness specified to catch origin-tagging bugs.)
#[test]
#[allow(clippy::too_many_lines)] // three shapes, three oracles, one sweep
fn randomized_subset_projections_match_the_oracle_under_d2() {
    let mut state = 0xBEEF_CAFE_1234_5678u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 33
    };
    let schema = schema(3);
    for case in 0..200u32 {
        let domain = 1 + next() % 6;
        let mut data: Vec<Vec<(u64, u64)>> = Vec::new();
        for _ in 0..3 {
            let rows = 1 + next() % 30;
            let mut rel = Vec::new();
            for _ in 0..rows {
                rel.push((next() % domain, next() % domain));
            }
            rel.sort_unstable();
            rel.dedup();
            data.push(rel);
        }
        let dir = TempDir::new(&format!("run-d2-diff-{case}"));
        let views = views_of(&dir, &schema, &data);
        let shape = case % 3;
        let occurrences = match shape {
            0 => vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
            ],
            1 => vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 0), (1, 2)]),
            ],
            _ => vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 3)]),
            ],
        };
        let n_vars = if shape == 2 { 4u16 } else { 3 };
        let n = occurrences.len();
        let normalized = normalized(occurrences, vec![]);
        let mut order: Vec<u16> = (0..u16::try_from(n).expect("small")).collect();
        for i in (1..order.len()).rev() {
            let j = usize::try_from(next()).expect("64-bit") % (i + 1);
            order.swap(i, j);
        }
        // Project a random nonempty strict subset of the vars.
        let keep: Vec<VarId> = (0..n_vars).filter(|_| next() % 2 == 0).map(VarId).collect();
        let keep = if keep.is_empty() || keep.len() == usize::from(n_vars) {
            vec![VarId(0)]
        } else {
            keep
        };
        let sinks: BTreeSet<VarId> = keep.iter().copied().collect();
        let plan = planned_with_sinks(&normalized, &schema, &order, &sinks);

        // Oracle: full joins, then project.
        let mut expected: BTreeSet<Vec<u64>> = BTreeSet::new();
        let full = |expected: &mut BTreeSet<Vec<u64>>, vals: &[u64]| {
            expected.insert(keep.iter().map(|v| vals[usize::from(v.0)]).collect());
        };
        match shape {
            0 => {
                for (a, b) in &data[0] {
                    for (c, d) in &data[1] {
                        if b == c {
                            full(&mut expected, &[*a, *b, *d]);
                        }
                    }
                }
            }
            1 => {
                for (a, b) in &data[0] {
                    for (c, d) in &data[1] {
                        for (e, g) in &data[2] {
                            if b == c && a == e && d == g {
                                full(&mut expected, &[*a, *b, *d]);
                            }
                        }
                    }
                }
            }
            _ => {
                for (a, b) in &data[0] {
                    for (c, d) in &data[1] {
                        for (e, g) in &data[2] {
                            if b == c && d == e {
                                full(&mut expected, &[*a, *b, *d, *g]);
                            }
                        }
                    }
                }
            }
        }
        for batch in [1usize, 7, 128] {
            let mut executor = Executor::with_batch_size(&plan, batch);
            let mut colts = colts_for(&plan, &views);
            let mut bindings = Bindings::new(plan.slot_count());
            let mut sink =
                ProjectionSinkForTest::new(keep.iter().map(|v| plan.slot_of(*v)).collect());
            executor.execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut NoopCounters,
            );
            let got: BTreeSet<Vec<u64>> = sink.rows().map(<[u64]>::to_vec).collect();
            assert_eq!(
                got, expected,
                "case {case} shape {shape} order {order:?} keep {keep:?} batch {batch}"
            );
        }
    }
}
