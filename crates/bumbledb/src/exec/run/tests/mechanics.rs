use super::*;

#[test]
fn dynamic_cover_prefers_the_forced_small_side() {
    let dir = TempDir::new("run-cover-choice");
    let schema = schema(2);
    // R: huge with duplicate x; S: tiny. Node 0 = [R(x), S(x)] via a
    // GJ-style hand plan where both are covers.
    let r: Vec<(u64, u64)> = (0..500).map(|i| (i % 250, i)).collect();
    let s: Vec<(u64, u64)> = vec![(0, 0), (1, 1)];
    let views = views_of(&dir, &schema, &[r, s]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    // Hand-build the GJ plan: [[R(x), S(x)], [R(a)], [S(b)]].
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![
            crate::plan::fj::Node {
                subatoms: vec![
                    crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(0)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(0)],
                    },
                ],
            },
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(1)],
                }],
            },
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(1),
                    vars: vec![VarId(2)],
                }],
            },
        ],
    };
    let plan =
        validate(&plan, &normalized, &schema, vec![0; 3], &BTreeSet::new()).expect("valid plan");

    // Pre-force S's root so its Exact(2) beats R's Estimate(500).
    let mut colts = colts_for(&plan, &views);
    let s_root = Colt::root();
    colts[1].get(s_root, 0, &[0]);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = RecordingCounters::default();
    Executor::new(&plan).execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);

    // Node 0's first choice: subatom 1 (S), whose count is Exact.
    let (node, subatom, exact) = counters.cover_choices[0];
    assert_eq!((node, subatom, exact), (0, 1, true));
    assert!(!sink.rows.is_empty());
}

/// Regression for the cover-soundness deviation
/// (docs/architecture/30-execution.md): a subatom carrying an
/// already-bound variable must never be a runtime-eligible cover. In
/// the triangle below, node 1 = [S(z), T(x, z)]; with skew, T's tiny
/// key count would win the dynamic choice, and iterating T(x, z)
/// rebinds x over R's binding without re-probing R — producing a row
/// where the correct answer is empty.
#[test]
fn covers_never_rebind_an_already_bound_variable() {
    let dir = TempDir::new("run-cover-rebind");
    let schema = schema(3);
    let r = vec![(1, 1)];
    let s: Vec<(u64, u64)> = (0..100).map(|z| (1, z)).collect();
    let t = vec![(2, 5)];
    let views = views_of(&dir, &schema, &[r, s, t]);

    // Q(x,y,z) :- R(x,y), S(y,z), T(x,z), order [R, S, T].
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1, 2]);

    // The mixed-var subatom T(x, z) must not be listed as a cover of
    // its node (x is bound by node 0).
    for node in plan.nodes() {
        for &cover in &node.covers {
            let vars = &node.subatoms[cover as usize].vars;
            assert_eq!(
                vars.len(),
                node.new_vars.len(),
                "a cover must bind exactly the node's new vars"
            );
        }
    }

    let results = run(&plan, &views);
    assert!(
        results.is_empty(),
        "T binds x=2, R binds x=1: joining them must be empty, got {results:?}"
    );
}

#[test]
fn backtracking_restores_sources_across_sequential_executions() {
    let dir = TempDir::new("run-backtrack");
    let schema = schema(2);
    let r: Vec<(u64, u64)> = (0..20).map(|i| (i % 4, i)).collect();
    let s: Vec<(u64, u64)> = (0..4).map(|i| (i, i * 10)).collect();
    let views = views_of(&dir, &schema, &[r, s]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut executor = Executor::new(&plan);

    let mut first = CollectSink::default();
    executor.execute(
        &plan,
        &mut colts,
        &mut bindings,
        &mut first,
        &mut NoopCounters,
    );
    let mut second = CollectSink::default();
    executor.execute(
        &plan,
        &mut colts,
        &mut bindings,
        &mut second,
        &mut NoopCounters,
    );
    assert_eq!(first.rows, second.rows);
    assert!(!first.rows.is_empty());
}

#[test]
fn results_are_identical_across_batch_sizes() {
    // Skew, empty relations, partial final batches, and batch > row
    // count are all covered by these fixtures x sizes.
    let dir = TempDir::new("run-batch-equality");
    let schema = schema(3);
    let r: Vec<(u64, u64)> = (0..150).map(|i| (i % 7, i % 11)).collect();
    let s: Vec<(u64, u64)> = (0..90).map(|i| (i % 11, i % 5)).collect();
    let t: Vec<(u64, u64)> = (0..40).map(|i| (i % 5, i)).collect();
    let views = views_of(&dir, &schema, &[r, s, t]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 3)]),
        ],
        vec![PlacedComparison {
            op: CmpOp::Ne,
            lhs: VarId(0),
            rhs: VarId(3),
        }],
    );
    let plan = planned(&normalized, &schema, &[0, 1, 2]);
    let reference = run_batched(&plan, &views, 1);
    assert!(!reference.is_empty());
    for batch in [2usize, 64, 128, 1024] {
        assert_eq!(
            run_batched(&plan, &views, batch),
            reference,
            "batch size {batch} must match the scalar degenerate case"
        );
    }

    // An empty relation, every batch size.
    let dir2 = TempDir::new("run-batch-empty");
    let views = views_of(&dir2, &schema, &[vec![(1, 2)], vec![], vec![(0, 0)]]);
    for batch in [1usize, 2, 64, 128, 256, 1024] {
        assert!(run_batched(&plan, &views, batch).is_empty());
    }
}

#[test]
fn phase_one_hashes_the_whole_batch_before_any_phase_two_probe() {
    let dir = TempDir::new("run-two-phase");
    let schema = schema(2);
    let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i)).collect();
    let s: Vec<(u64, u64)> = (0..10).map(|i| (i, i * 2)).collect();
    let views = views_of(&dir, &schema, &[r, s]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = PhaseOrderCounters::default();
    Executor::with_batch_size(&plan, 128).execute(
        &plan,
        &mut colts,
        &mut bindings,
        &mut sink,
        &mut counters,
    );

    // All 10 root entries fit one batch: every hash of node 0's sibling
    // pass must precede its first probe.
    let first_probe = counters
        .events
        .iter()
        .position(|(kind, node, _)| *kind == "probe" && *node == 0)
        .expect("probes happened");
    let hashes_before = counters.events[..first_probe]
        .iter()
        .filter(|(kind, node, _)| *kind == "hash" && *node == 0)
        .count();
    assert_eq!(
        hashes_before, 10,
        "the entire batch is hashed before the first bucket load"
    );
    assert!(!sink.rows.is_empty());
}

/// PRD 05 (docs/hardening): a pinned sibling (`Cursor::Row`) probes
/// by field equality — phase 1 computes no hash for it, and EXPLAIN's
/// `hashes` counts only hashes computed for map probes. Probes still
/// count; results are unchanged.
#[test]
fn pinned_siblings_probe_without_hashing() {
    let dir = TempDir::new("run-pinned-hash");
    let schema = schema(3);
    // A(a,b) drives; B and C each have exactly one row per probe key,
    // so both pin to Cursor::Row after node 0. At node 1 both B(c)
    // and C(c) are covers with count 1; the tie keeps the incumbent
    // (B, the lower subatom index), leaving C as the pinned sibling.
    let a_rows: Vec<(u64, u64)> = vec![(1, 10), (2, 20)];
    let b_rows: Vec<(u64, u64)> = vec![(1, 100), (2, 200)];
    let c_rows: Vec<(u64, u64)> = vec![(10, 100), (20, 200)];
    let views = views_of(&dir, &schema, &[a_rows, b_rows, c_rows]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]), // A(a, b)
            occurrence(1, 1, &[(0, 0), (1, 2)]), // B(a, c)
            occurrence(2, 2, &[(0, 1), (1, 2)]), // C(b, c)
        ],
        vec![],
    );
    // Hand-built: node 0 probes both B(a) and C(b) — C's second
    // appearance at node 1 is then a probe against its pinned child.
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![
            crate::plan::fj::Node {
                subatoms: vec![
                    crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(0), VarId(1)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(0)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(2),
                        vars: vec![VarId(1)],
                    },
                ],
            },
            crate::plan::fj::Node {
                subatoms: vec![
                    crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(2)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(2),
                        vars: vec![VarId(2)],
                    },
                ],
            },
        ],
    };
    let plan =
        validate(&plan, &normalized, &schema, vec![0; 2], &BTreeSet::new()).expect("valid plan");
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = PhaseOrderCounters::default();
    Executor::new(&plan).execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);

    let count = |kind: &str, node: usize, subatom: usize| {
        counters
            .events
            .iter()
            .filter(|(k, n, s)| *k == kind && *n == node && *s == subatom)
            .count()
    };
    // Node 0's siblings probe root nodes: hashed.
    assert!(count("hash", 0, 1) > 0, "B's root probe hashes");
    assert!(count("hash", 0, 2) > 0, "C's root probe hashes");
    // Node 1's pinned sibling (C, subatom 1): probed, never hashed.
    assert_eq!(count("hash", 1, 1), 0, "pinned probes compute no hash");
    assert_eq!(count("probe", 1, 1), 2, "both entries still probe C");
    // Results unchanged: the two consistent binding triples.
    assert_eq!(
        sink.rows,
        BTreeSet::from([vec![1, 10, 100], vec![2, 20, 200]])
    );
}

/// The magnitude-first cover rule (docs/architecture/30-execution.md), table-tested: the
/// smaller side wins whatever its label; Exact breaks ties; a full
/// tie keeps the incumbent.
#[test]
fn cover_choice_is_magnitude_first() {
    use KeyCount::{Estimate, Exact};
    // The measured bug: a 7-row unforced view must beat a 500-key
    // forced map.
    assert!(better_cover(Estimate(7), Exact(500)));
    assert!(!better_cover(Exact(500), Estimate(7)));
    // Magnitude wins in both label directions.
    assert!(better_cover(Exact(7), Estimate(500)));
    assert!(!better_cover(Estimate(500), Exact(7)));
    // Equal magnitudes: Exact displaces Estimate, never vice versa,
    // and same-label ties keep the incumbent (deterministic order).
    assert!(better_cover(Exact(9), Estimate(9)));
    assert!(!better_cover(Estimate(9), Exact(9)));
    assert!(!better_cover(Exact(9), Exact(9)));
    assert!(!better_cover(Estimate(9), Estimate(9)));
}
