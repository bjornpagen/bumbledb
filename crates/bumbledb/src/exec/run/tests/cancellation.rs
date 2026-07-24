use super::*;

/// D2 under the pipeline — two parents
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
    for batch in [1usize, 2, 128] {
        let mut executor = Executor::with_batch_size(&plan, batch);
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slot_count());
        let mut sink = ProjectionSinkForTest::new(vec![plan.slot_of(VarId(0))]);
        let mut counters = SkipCounterRun::default();
        executor
            .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
            .expect("execute");
        let mut rows: Vec<u64> = sink.rows_first_col();
        rows.sort_unstable();
        assert_eq!(rows, vec![1, 2], "batch {batch}: both x groups present");
        assert!(counters.skips > 0, "batch {batch}: skips fired");
    }
}

/// The randomized D2 differential: subset projections force real
/// D2 skips through the pipeline — random instances, orders, and
/// batch sizes against the nested-loop oracle's projected sets.
/// (This is the harness specified to catch origin-tagging bugs.)
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // three shapes, three oracles, one sweep
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
            executor
                .execute(
                    &plan,
                    &mut colts,
                    &mut bindings,
                    &mut sink,
                    &mut NoopCounters,
                )
                .expect("execute");
            let got: BTreeSet<Vec<u64>> = sink.answers().map(<[u64]>::to_vec).collect();
            assert_eq!(
                got, expected,
                "case {case} shape {shape} order {order:?} keep {keep:?} batch {batch}"
            );
        }
    }
}

/// The epoch wrap guard (`Executor::advance_cancel_epoch`): the D2
/// cancellation table is stamped, never cleared per execution, and the
/// u32 epoch recycles its space once per 2³² executions — a stamp from
/// the previous cycle must not alias the recycled value, or a live
/// origin's whole subtree is silently skipped (answers missing that
/// `lean/Bumbledb/Exec/Plan.lean: valid_plan_sound` requires). The
/// test walks one full cycle with the middle jumped: the stamp is laid
/// at epoch 1, the counter runs wrap-free to `u32::MAX` (the stamp
/// legitimately survives — no later epoch equals 1), and the two
/// advances that cross the wrap and return to the stamp's value must
/// find the table cleared.
#[test]
fn epoch_wrap_never_aliases_a_stale_cancellation() {
    // Any plan makes an executor; the bookkeeping under test is
    // plan-independent.
    let schema = schema(2);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let mut executor = Executor::new(&plan);

    // Execution at epoch 1 cancels origin 3.
    executor.advance_cancel_epoch();
    assert_eq!(executor.cancel_epoch, 1);
    executor.cancel_origin(3);
    assert!(executor.origin_cancelled(3), "stamped in its own epoch");

    // The wrap-free middle of the cycle, jumped: epochs 2..=u32::MAX
    // never equal the stamp, so the stale entry is inert.
    executor.cancel_epoch = u32::MAX;
    assert!(
        !executor.origin_cancelled(3),
        "a later epoch never reads it"
    );

    // Crossing the wrap clears the table; returning to the stamp's
    // recycled value must NOT resurrect the cancellation.
    executor.advance_cancel_epoch();
    assert_eq!(executor.cancel_epoch, 0);
    executor.advance_cancel_epoch();
    assert_eq!(executor.cancel_epoch, 1, "the stamp's value, recycled");
    assert!(
        !executor.origin_cancelled(3),
        "a stale stamp from the previous epoch cycle must not cancel a live origin"
    );

    // The recycled epoch still cancels normally.
    executor.cancel_origin(3);
    assert!(executor.origin_cancelled(3));
}

/// The whole-execution D2 skip (absorb = None: a boolean/existential
/// head licenses every node) stops the top-level cover draw MID-ENTRY:
/// node 0 holds exactly one pending entry — the virtual root — so only
/// pump's inner batch loop can see the poison. Before the check landed
/// there, a first-batch witness still iterated and probed the entire
/// remaining node-0 cover, batch by fully-priced batch.
#[test]
fn whole_execution_skip_stops_the_cover_draw_mid_entry() {
    /// Counts cover batches drawn at node 0.
    #[derive(Default)]
    struct RootBatches {
        batches: usize,
    }
    impl Counters for RootBatches {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, node: usize, _: usize) {
            if node == 0 {
                self.batches += 1;
            }
        }
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn anti_probe(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    let dir = TempDir::new("run-whole-skip");
    let schema = schema(2);
    // R fans 600 rows (>> one 128 batch); S matches every y, so the
    // very first leaf emit witnesses the empty projection.
    let r: Vec<(u64, u64)> = (0..600).map(|i| (i, i % 7)).collect();
    let s: Vec<(u64, u64)> = (0..7).map(|y| (y, 900 + y)).collect();
    let views = views_of(&dir, &schema, &[r, s]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
        ],
        vec![],
    );
    // Zero sink vars: every node is skip-licensed, absorb is None — the
    // first witness fixes the whole execution's answer.
    let plan = planned_with_sinks(&normalized, &schema, &[0, 1], &BTreeSet::new());
    let mut executor = Executor::new(&plan);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = ProjectionSinkForTest::new(vec![]);
    let mut counters = RootBatches::default();
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");
    assert_eq!(
        sink.answers().count(),
        1,
        "the boolean head has its one witness"
    );
    assert_eq!(
        counters.batches, 1,
        "the poison stopped node 0's cover draw after the witnessing batch"
    );
}
