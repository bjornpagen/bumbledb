//! View-memo behavior on the successor substrate: epoch-keyed hits within
//! one generation, rebuilds across generations, parked-binding rotation,
//! heap ticks that never memoize, and trim. The old write-path `advance`
//! lineage hook is deleted with the transitional storage — invalidation is
//! generation-keyed now.
use super::*;

fn rotating_rows() -> Vec<(u64, u64, &'static str, i64)> {
    (0..32u64)
        .map(|i| {
            let memo: &'static str = match i % 3 {
                0 => "m0",
                1 => "m1",
                _ => "m2",
            };
            (i, i % 4, memo, i64::try_from(i).expect("small") * 3 - 40)
        })
        .collect()
}

#[test]
fn same_generation_executions_memo_hit_and_new_generations_rebuild() {
    let fix = posting_store("view-memo-generations", &rotating_rows());
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");

    let run = |fix: &StoreFix, prepared: &mut PreparedQuery<T>, memo: &str| {
        let out = fix.execute(prepared, &memo_param(memo)).expect("execute");
        let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
            panic!("free join fixture")
        };
        let Binding::Bound(bound) = &rule.memo.occs[0].active else {
            panic!("executed binding")
        };
        (out.len(), bound.epoch, bound.last_used)
    };

    let (rows_a, epoch, built_at) = run(&fix, &mut prepared, "m0");
    assert!(rows_a > 0);
    assert_eq!(run(&fix, &mut prepared, "m0"), (rows_a, epoch, built_at));
    fix.insert_dyn(POSTING, &posting_rows(&[(1000, 0, "m0", 999)]));
    let (rows_b, next_epoch, rebuilt_at) = run(&fix, &mut prepared, "m0");
    assert_ne!(next_epoch, epoch);
    assert!(rebuilt_at > built_at);
    assert_eq!(rows_b, rows_a + 1);
}

#[test]
fn heap_executions_use_fresh_view_epochs_without_resident_cache_entries() {
    let fix = postings(&[(1, 7, "a", 10)]);
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");
    let mut epochs = Vec::new();
    for _ in 0..2 {
        let out = fix
            .execute(&mut prepared, &memo_param("a"))
            .expect("execute");
        assert_eq!(amounts_of(&out), [10]);
        let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
            panic!("free join fixture");
        };
        let Binding::Bound(bound) = &rule.memo.occs[0].active else {
            panic!("executed heap binding");
        };
        assert!(matches!(bound.epoch, crate::image::ViewEpoch::Heap(_)));
        epochs.push(bound.epoch);
        assert_eq!(prepared.cache.image_count(), 0);
    }
    assert_ne!(
        epochs[0], epochs[1],
        "heap executions cannot hit an old view"
    );
}

#[test]
fn a_heap_prepared_plan_refuses_other_sources() {
    let rows: &[(u64, u64, &str, i64)] = &[(1, 7, "a", 10)];
    let heap = postings(rows);
    let store = posting_store("view-memo-foreign", rows);
    let mut prepared = heap.prepare(&by_memo_query()).expect("prepare");
    let refused = store.execute(&mut prepared, &memo_param("a"));
    assert!(matches!(refused, Err(Error::ForeignPreparedQuery)));
}

#[test]
fn trim_drops_parked_views_and_preserves_answers() {
    let fix = posting_store("view-memo-trim", &rotating_rows());
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let params = [BindValue::U64(0), BindValue::I64(-100)];
    let before = answers_of(&fix.execute(&mut prepared, &params).expect("execute"));
    prepared.release_memory();
    let after = answers_of(&fix.execute(&mut prepared, &params).expect("re-execute"));
    assert_eq!(before, after, "trim changes cost, never answers");
}

#[test]
fn query_release_deallocates_active_join_pools_without_clearing_shared_cache() {
    let rows: Vec<_> = (0..8192).map(|id| (id, id % 4, "memo", 1)).collect();
    let fix = posting_store("view-memo-release-pools", &rows);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let params = [BindValue::U64(0), BindValue::I64(-100)];
    let before = answers_of(&fix.execute(&mut prepared, &params).expect("execute"));
    let retained = |prepared: &PreparedQuery<T>| {
        let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
            panic!("free join fixture")
        };
        rule.memo
            .colts
            .iter()
            .map(Colt::retained_bytes)
            .sum::<usize>()
    };
    let warm_bytes = retained(&prepared);
    assert!(warm_bytes > 8192, "fixture must grow the join pools");
    let generation = prepared.cache.cache_generation();
    let images = prepared.cache.image_count();
    prepared.release_memory();
    eprintln!(
        "join pool bytes: warm={warm_bytes}, released={}",
        retained(&prepared)
    );
    assert_eq!(
        retained(&prepared),
        0,
        "release must drop active pool capacity"
    );
    assert_eq!(prepared.cache.cache_generation(), generation);
    assert_eq!(prepared.cache.image_count(), images);
    let after = answers_of(
        &fix.execute(&mut prepared, &params)
            .expect("reuse prepared plan"),
    );
    assert_eq!(before, after);
    prepared.release_memory();
    prepared.release_memory();
    assert_eq!(retained(&prepared), 0, "release is idempotent");
}

fn memo_window(draw: u64) -> [FilterPredicate; 2] {
    [
        FilterPredicate::Compare {
            field: FieldId(0).into(),
            op: crate::ir::WordCmp::Ge,
            value: Const::Word(draw * 64),
        },
        FilterPredicate::Compare {
            field: FieldId(0).into(),
            op: crate::ir::WordCmp::Lt,
            value: Const::Word((draw + 1) * 64),
        },
    ]
}

fn rebuild_memo_window(
    memo: &mut ViewMemo,
    image: &Arc<crate::image::RelationImage>,
    epoch: crate::image::ViewEpoch,
    draw: u64,
) {
    let filters = memo_window(draw);
    let buffer = std::mem::take(memo.spare_mut(0));
    let view =
        crate::image::view::apply(image, &filters, &[], buffer, image.generation().text_eq())
            .expect("numeric window");
    *memo.spare_mut(0) = memo.colts[0].reset(view).recycle();
    memo.set_bound(0, epoch, &filters, None);
}

/// The operation context belongs to the active execution slot, whereas
/// cached contents and retained pools travel through the LRU.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table-driven activation protocol checks all context and ownership outcomes"
)]
fn four_draw_memo_activation_preserves_current_work_and_pool_ownership() {
    use crate::image::view::View;
    use crate::schema::ValidateDescriptor as _;
    use crate::work::WorkError;

    let schema = descriptor().validate().expect("fixture schema");
    let rows: Vec<_> = (0..320).map(|id| (id, 0, "m", 0)).collect();
    let source =
        crate::image::testsupport::TestSource::new(&schema, &[(POSTING, posting_rows(&rows))]);
    let (_cache, image) = source.image_with_cache(POSTING);
    let epoch =
        crate::image::ViewEpoch::Store(crate::storage::store::RelationVersion::from_storage(1));

    for activation in ["bound-hit", "unbound-hit", "lru-miss"] {
        for refusal in ["cancelled-prior", "cancelled-current", "fresh-current"] {
            let mut memo = ViewMemo::new();
            memo.push(
                Colt::new(View::Unbound, &[], vec![vec![0]]),
                Binding::Unbound,
            );
            let mut owners = Vec::new();
            for draw in 0..4 {
                let work = crate::work::WorkContext::new();
                memo.colts[0].bind(Some(&work));
                memo.tick += 1;
                assert!(!memo.bind(0, epoch, &memo_window(draw), &[]));
                owners.push(work);
                // Model a real aborted fourth build: bind has already
                // parked the active COLT and installed its fresh sibling.
                if activation == "unbound-hit" && draw == 3 {
                    assert!(matches!(memo.occs[0].active, Binding::Unbound));
                    break;
                }
                rebuild_memo_window(&mut memo, &image, epoch, draw);
                memo.colts[0].force_root().expect("warm actual trie");
            }
            assert_eq!(
                memo.occs[0].parked.iter().flatten().count(),
                3,
                "all three parked slots are populated"
            );
            let retained = |memo: &ViewMemo| {
                memo.colts.iter().map(Colt::retained_bytes).sum::<usize>()
                    + memo
                        .occs
                        .iter()
                        .flat_map(|occ| occ.parked.iter().flatten())
                        .map(|parked| parked.colt.retained_bytes())
                        .sum::<usize>()
                    + memo
                        .occs
                        .iter()
                        .map(|occ| occ.spare.capacity() * 4)
                        .sum::<usize>()
            };
            let before = retained(&memo);
            assert!(before > 0);
            if refusal == "cancelled-prior" {
                for work in &owners {
                    work.cancel();
                }
            }
            let current = crate::work::WorkContext::new();
            if refusal == "cancelled-current" {
                current.cancel();
            }

            // This is run_join's pre-force binding order. No execute-time
            // rebind is allowed to repair a stale context after this point.
            memo.colts[0].bind(Some(&current));
            memo.tick += 1;
            let draw = if activation == "lru-miss" { 4 } else { 0 };
            let hit = memo.bind(0, epoch, &memo_window(draw), &[]);
            assert_eq!(hit, activation != "lru-miss", "{activation}");
            assert!(
                retained(&memo) <= before,
                "activation never creates larger pools"
            );
            if hit {
                // Rebuild the same cached view in retained pools: an already
                // forced hit must not hide a stale/cancelled operation binding.
                let view = memo.colts[0].reset(View::Unbound);
                drop(memo.colts[0].reset(view));
            } else {
                rebuild_memo_window(&mut memo, &image, epoch, draw);
            }

            let result = memo.colts[0].force_root();
            match refusal {
                "cancelled-prior" | "fresh-current" => {
                    result.unwrap_or_else(|error| panic!(
                        "{activation}: parked prior context leaked into fresh operation: {error:?}"
                    ));
                    assert_eq!(memo.colts[0].key_count(Colt::root()).magnitude(), 64);
                    assert!(
                        memo.colts[0]
                            .get_prehashed(
                                Colt::root(),
                                0,
                                &[draw * 64],
                                crate::exec::colt::hash_key(&[draw * 64])
                            )
                            .unwrap()
                            .is_some()
                    );
                    assert_eq!(current.checkpoint(), Ok(()));
                }
                "cancelled-current" => {
                    assert_eq!(result, Err(WorkError::Cancelled), "{activation}");
                }
                _ => unreachable!(),
            }
            let retained = retained(&memo);
            let before = crate::alloc_counter::snapshot().window;
            drop(memo);
            let after = crate::alloc_counter::snapshot().window;
            #[cfg(feature = "alloc-counter")]
            assert!(after.dealloc_bytes - before.dealloc_bytes >= retained as u64);
            let _ = (retained, before, after);
        }
    }
}
