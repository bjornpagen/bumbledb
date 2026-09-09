use super::*;
use crate::interval::overlap::p3_cache_observer::{CacheSnapshot, costs, reset_costs};
use std::collections::{BTreeMap, BTreeSet};

crate::schema! {
    pub Temporal;
    relation Key { id: u64 as TpKeyId, }
    relation Span { id: u64 as TpSpanId, key: u64 as TpKeyId, span: interval<i64>, weight: i64, }
    Key(id) -> Key;
    Span(id) -> Span;
    Span(key) <= Key(id);
}

fn report(label: &str, snapshot: &CacheSnapshot) {
    let mut live = 0;
    let mut retained = 0;
    for &(owner, len, cap, word_bytes) in &snapshot.owners {
        println!(
            "OWNER {label} {owner} len={len} capacity={cap} word_bytes={word_bytes} live_bytes={} retained_bytes={}",
            len * word_bytes,
            cap * word_bytes
        );
        live += len * word_bytes;
        retained += cap * word_bytes;
    }
    println!(
        "CACHE {label} built={} tallied={} live_payload_bytes={live} retained_payload_bytes={retained}",
        snapshot.groups.len(),
        snapshot.tallied
    );
    for (key, len, base, p) in &snapshot.groups {
        println!("GROUP {label} key={key:?} len={len} tree_base={base} p={p}");
    }
}

fn snapshot(prepared: &PreparedQuery<Temporal>) -> CacheSnapshot {
    let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
        panic!("the saved temporal query must use Free Join");
    };
    rule.executor.saved_overlap_snapshot()
}

#[test]
#[ignore = "saved-corpus ownership/allocation audit; no timing or full trace"]
fn saved_temporal_cache_ownership() {
    assert!(cfg!(feature = "alloc-counter"));
    let path = std::env::var_os("BUMBLEDB_OWNER_DB").expect("private saved-corpus copy");
    let expected = std::env::var("BUMBLEDB_OWNER_ANSWER")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let counts_path = std::env::var_os("BUMBLEDB_OWNER_GROUPS").unwrap();
    let counts: BTreeMap<u64, usize> = std::fs::read_to_string(counts_path)
        .unwrap()
        .lines()
        .map(|line| {
            let (key, len) = line.split_once(',').unwrap();
            (key.parse().unwrap(), len.parse().unwrap())
        })
        .collect();
    let work = crate::WorkContext::new();
    let db = crate::Db::open(std::path::Path::new(&path), Temporal, work.clone()).unwrap();
    let v = |id| Term::Var(VarId(id));
    let query = Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(RelationId(1)),
                bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(2)), (FieldId(2), v(3))],
            },
            Atom {
                source: AtomSource::Edb(RelationId(1)),
                bindings: vec![(FieldId(0), v(1)), (FieldId(1), v(2)), (FieldId(2), v(4))],
            },
        ],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: v(0),
                rhs: v(1),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: crate::AllenMask::INTERSECTS,
                },
                lhs: v(3),
                rhs: v(4),
            }),
        ],
    });
    let mut prepared = db.prepare(&query, work.clone()).unwrap();
    if let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() {
        for (index, node) in rule.plan.nodes().iter().enumerate() {
            println!("NODE {index} {node:?}");
        }
        for occurrence in rule.plan.occurrences() {
            println!("OCCURRENCE {occurrence:?}");
        }
    }
    let mut out = Answers::new();
    let mut first: Option<CacheSnapshot> = None;
    for label in ["cold", "warm", "released"] {
        if label == "released" {
            prepared.release_memory();
            let released = snapshot(&prepared);
            report("after-release", &released);
            assert!(
                released
                    .owners
                    .iter()
                    .all(|(_, len, cap, _)| *len == 0 && *cap == 0)
            );
        }
        reset_costs();
        crate::alloc_counter::reset();
        let before = crate::alloc_counter::snapshot();
        db.read(work.clone(), |snap| {
            snap.execute(&mut prepared, &[] as &[BindValue], &mut out)
        })
        .unwrap();
        let after = crate::alloc_counter::snapshot();
        let cache_costs = costs();
        println!("ALLOC {label} before={before:?} after={after:?} cache_probe={cache_costs:?}");
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), AnswerValue::U64(expected));
        let state = snapshot(&prepared);
        report(label, &state);
        assert_eq!(state.tallied, 0);
        let mut actual = BTreeMap::new();
        let mut covers = BTreeSet::new();
        for (key, len, _, _) in &state.groups {
            let [cover, key] = key.as_slice() else {
                panic!("audit the actual prefix before assuming per-key groups");
            };
            covers.insert(*cover);
            assert!(
                actual.insert(*key, *len).is_none(),
                "no duplicate per-key cache group"
            );
        }
        assert_eq!(covers.len(), 1);
        assert_eq!(
            actual, counts,
            "actual executor groups must match the independent saved SQL groups"
        );
        if label == "warm" {
            assert_eq!(
                cache_costs.allocs, 0,
                "warm probe/build reuses every cache owner"
            );
            assert_eq!(&state, first.as_ref().unwrap());
        }
        if first.is_none() {
            first = Some(state);
        }
        println!("PASS {label} count={expected}");
    }
}

#[test]
#[ignore = "saved mixed-mask branch census and exact SQL pairs; no timing"]
fn saved_mixed_mask_paths() {
    use crate::interval::overlap::p3_cache_observer::query_counts;
    let path = std::env::var_os("BUMBLEDB_OWNER_DB").unwrap();
    let pairs_path = std::env::var_os("BUMBLEDB_MIXED_PAIRS").unwrap();
    let mut expected = BTreeMap::<u64, Vec<(u64, u64)>>::new();
    for line in std::fs::read_to_string(pairs_path).unwrap().lines() {
        let mut fields = line.split(',').map(|s| s.parse::<u64>().unwrap());
        let key = fields.next().unwrap();
        let lhs = fields.next().unwrap();
        let rhs = fields.next().unwrap();
        assert!(fields.next().is_none());
        expected.entry(key).or_default().push((lhs, rhs));
    }
    let work = crate::WorkContext::new();
    let db = crate::Db::open(std::path::Path::new(&path), Temporal, work.clone()).unwrap();
    let v = |id| Term::Var(VarId(id));
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), v(0)),
                    (FieldId(1), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(2), v(2)),
                ],
            },
            Atom {
                source: AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), v(1)),
                    (FieldId(1), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(2), v(3)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: crate::AllenMask::DURING | crate::AllenMask::MEETS,
            },
            lhs: v(2),
            rhs: v(3),
        })],
    });
    let mut prepared = db.prepare(&query, work.clone()).unwrap();
    let mut out = Answers::new();
    // Same parameter order as the ordinary scenario; a second cycle observes
    // reuse after all four bindings, not four separately prepared queries.
    let mut first = BTreeMap::new();
    for cycle in 0..2 {
        for key in [0, 1, 5, 1_000_000] {
            reset_costs();
            db.read(work.clone(), |snap| {
                snap.execute(&mut prepared, &[BindValue::U64(key)], &mut out)
            })
            .unwrap();
            let (flat_calls, tree_calls) = query_counts();
            let probe_calls = costs().calls;
            // All inspection/expected-output work is outside the query.
            let mut actual: Vec<_> = (0..out.len())
                .map(|row| {
                    let (AnswerValue::U64(lhs), AnswerValue::U64(rhs)) =
                        (out.get(row, 0), out.get(row, 1))
                    else {
                        panic!("Span ids are u64");
                    };
                    (lhs, rhs)
                })
                .collect();
            actual.sort_unstable();
            assert_eq!(actual, expected.get(&key).cloned().unwrap_or_default());
            let state = snapshot(&prepared);
            let groups: Vec<_> = state.groups.iter().map(|(_, len, _, _)| *len).collect();
            let identity = (flat_calls, tree_calls, probe_calls, out.len(), groups.clone());
            if cycle == 0 {
                first.insert(key, identity);
            } else {
                assert_eq!(first[&key], identity);
            }
            println!(
                "MIXED cycle={cycle} key={key} rows={} flat_calls={flat_calls} tree_calls={tree_calls} probe_calls={probe_calls} groups={groups:?}",
                out.len()
            );
        }
    }
    println!("PASS mixed exact-pairs and repeated branch counts");
}
