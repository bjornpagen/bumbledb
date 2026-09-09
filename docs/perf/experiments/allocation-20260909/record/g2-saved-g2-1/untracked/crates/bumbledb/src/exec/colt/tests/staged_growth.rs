use super::*;
use crate::work::{WorkContext, WorkError};
use std::cell::Cell;

thread_local! {
    static FAULT: Cell<Option<(usize, WorkError)>> = const { Cell::new(None) };
    static SEEN: Cell<usize> = const { Cell::new(0) };
}

// Test-only deterministic refusals at the real growth checkpoints. Allocation
// errors here model propagation; the overflow test exercises a real refusal.
pub(in crate::exec::colt) fn checkpoint() -> Result<(), WorkError> {
    SEEN.set(SEEN.get() + 1);
    match FAULT.get() {
        Some((0, error)) => {
            FAULT.set(None);
            Err(error)
        }
        Some((left, error)) => {
            FAULT.set(Some((left - 1, error)));
            Ok(())
        }
        None => Ok(()),
    }
}

fn injected<T>(after: usize, error: WorkError, run: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            FAULT.set(None);
        }
    }
    let _reset = Reset;
    FAULT.set(Some((after, error)));
    SEEN.set(0);
    (run(), SEEN.get())
}

fn key(width: usize, value: u64) -> Vec<u64> {
    (0..width)
        .map(|i| {
            value
                .wrapping_mul(31)
                .rotate_left(u32::try_from(i * 9).unwrap())
        })
        .collect()
}

fn unpublished(colt: &mut Colt, width: usize) -> Map {
    let map = Map {
        arity: width,
        nbuckets: 8,
        len: 0,
        ctrl_start: colt.ctrl.len(),
        bucket_start: colt.buckets.len(),
        dense_start: colt.dense.len(),
    };
    colt.ctrl.resize(map.ctrl_start + 64, 0);
    colt.buckets.resize(map.bucket_start + 8 * map.stride(), 0);
    map
}

#[test]
fn staged_growth_keeps_bases_order_and_every_packed_child_bit() {
    let view = view_of(&schema(), &[(7, 9)]);
    for width in [0, 1, 2, 3, 4, 5, 8] {
        for count in [0, 1, 25] {
            if width == 0 && count > 1 {
                continue;
            }
            let mut colt = Colt::new(all(&view), &[], vec![vec![0]]);
            colt.force_root().unwrap();
            let prefix = (colt.ctrl.clone(), colt.buckets.clone(), colt.dense.clone());
            let mut map = unpublished(&mut colt, width);
            for id in 0..count {
                let words = key(width, id);
                colt.ingest_one(
                    &mut map,
                    &words,
                    hash_words(&words),
                    u32::try_from(id).unwrap(),
                )
                .unwrap();
                let (_, idx) = colt.probe_hashed(&map, &words, hash_words(&words));
                colt.buckets[map.child_at(idx)] = pack_child(if id % 2 == 0 {
                    Cursor::Row(u32::MAX)
                } else {
                    Cursor::Node(NodeRef(u32::MAX))
                });
            }
            let starts = (map.ctrl_start, map.bucket_start, map.dense_start);
            for _ in 0..3 {
                let old_buckets = map.nbuckets;
                colt.grow_map(&mut map).unwrap();
                assert_eq!(map.nbuckets, old_buckets * 2);
                assert_eq!((map.ctrl_start, map.bucket_start, map.dense_start), starts);
                assert!(colt.scratch.is_empty());
                assert!(colt.scratch.capacity() >= usize::try_from(count).unwrap() * (width + 1));
                assert_eq!(&colt.ctrl[..starts.0], prefix.0);
                assert_eq!(&colt.buckets[..starts.1], prefix.1);
                assert_eq!(&colt.dense[..starts.2], prefix.2);
                assert_eq!(colt.ctrl.len(), starts.0 + map.nbuckets * 8);
                assert_eq!(colt.buckets.len(), starts.1 + map.nbuckets * map.stride());
                assert_eq!(colt.dense.len(), starts.2 + usize::try_from(count).unwrap());
                for id in 0..count {
                    let words = key(width, id);
                    let (found, idx) = colt.probe_hashed(&map, &words, hash_words(&words));
                    assert!(found);
                    assert_eq!(
                        colt.dense[starts.2 + usize::try_from(id).unwrap()] as usize,
                        idx
                    );
                    assert_eq!(
                        unpack_child(colt.buckets[map.child_at(idx)]),
                        if id % 2 == 0 {
                            Cursor::Row(u32::MAX)
                        } else {
                            Cursor::Node(NodeRef(u32::MAX))
                        }
                    );
                }
            }
        }
    }
}

fn lengths(colt: &Colt) -> [usize; 7] {
    let m = colt.pool_mark();
    [
        m.nodes,
        m.chunks,
        m.chunk_positions,
        m.maps,
        m.ctrl,
        m.buckets,
        m.dense,
    ]
}

fn older_maps(view: &Arc<crate::image::RelationImage>, warmed: bool) -> (Colt, Cursor, Cursor) {
    let mut colt = Colt::new(all(view), &[], vec![vec![0], vec![1]]);
    colt.bind(Some(&WorkContext::new()));
    if warmed {
        colt.force_root().unwrap();
        let child = colt.get(Colt::root(), 0, &[1]).unwrap();
        colt.ensure_forced(child, 1).unwrap();
        drop(colt.reset(all(view)));
    }
    colt.force_root().unwrap();
    let old = colt.get(Colt::root(), 0, &[0]).unwrap();
    colt.ensure_forced(old, 1).unwrap();
    let fresh = colt.get(Colt::root(), 0, &[1]).unwrap();
    (colt, old, fresh)
}

#[test]
fn every_growth_checkpoint_rolls_back_unpublished_construction_and_restores_scratch() {
    let rows: Vec<_> = (0..10)
        .map(|i| (0, i))
        .chain((0..1025).map(|i| (1, i)))
        .collect();
    let view = view_of(&schema(), &rows);
    for warmed in [false, true] {
        let (mut control, _, fresh) = older_maps(&view, warmed);
        let (ok, checkpoints) = injected(usize::MAX, WorkError::Cancelled, || {
            control.ensure_forced(fresh, 1)
        });
        ok.unwrap();
        assert!(checkpoints > 20, "multiple growth and reinsertion batches");
        for error in [WorkError::Cancelled, WorkError::Allocation] {
            for after in 0..checkpoints {
                let (mut colt, old, fresh) = older_maps(&view, warmed);
                let before = lengths(&colt);
                let prefix = (colt.ctrl.clone(), colt.buckets.clone(), colt.dense.clone());
                let scratch_capacity = colt.scratch.capacity();
                let (result, seen) = injected(after, error, || colt.ensure_forced(fresh, 1));
                assert_eq!(result, Err(error), "checkpoint {after}, warmed={warmed}");
                assert_eq!(seen, after + 1);
                assert_eq!(lengths(&colt), before);
                assert_eq!(
                    (&colt.ctrl, &colt.buckets, &colt.dense),
                    (&prefix.0, &prefix.1, &prefix.2)
                );
                assert!(colt.scratch.is_empty());
                assert!(colt.scratch.capacity() >= scratch_capacity);
                let Cursor::Node(node) = fresh else {
                    panic!("child is a node")
                };
                assert!(matches!(
                    colt.nodes[node.0 as usize],
                    NodeState::Unforced(_)
                ));
                assert_eq!(colt.get(Colt::root(), 0, &[0]), Some(old));
                assert_eq!(
                    drain(&mut colt, old, 1)
                        .iter()
                        .map(|(k, _)| k[0])
                        .collect::<Vec<_>>(),
                    (0..10).collect::<Vec<_>>()
                );
                colt.ensure_forced(fresh, 1).unwrap();
                assert_eq!(
                    drain(&mut colt, fresh, 1)
                        .iter()
                        .map(|(k, _)| k[0])
                        .collect::<Vec<_>>(),
                    (0..1025).collect::<Vec<_>>()
                );
                assert!(colt.scratch.is_empty());
            }
        }
        eprintln!("checked {checkpoints} growth refusal sites for each error; warmed={warmed}");
    }
}

#[test]
fn staged_growth_reuses_scratch_on_reset_rebind_and_releases_it_explicitly() {
    let rows: Vec<_> = (0..1025).map(|i| (i, i)).collect();
    let view = view_of(&schema(), &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0]]);
    colt.force_root().unwrap();
    let retained = colt.retained_bytes();
    let scratch_capacity = colt.scratch.capacity();
    assert!(scratch_capacity >= 2 * 819);
    for _ in 0..4 {
        drop(colt.reset(all(&view)));
        colt.bind(Some(&WorkContext::new()));
        #[cfg(feature = "alloc-counter")]
        let before = crate::alloc_counter::snapshot().window;
        colt.force_root().unwrap();
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().window, before);
        assert_eq!(colt.retained_bytes(), retained);
        assert_eq!(colt.scratch.capacity(), scratch_capacity);
        assert!(colt.scratch.is_empty());
    }
    let mut cloned = Colt::new(all(&view), &[], vec![vec![0]]);
    drop(cloned.clone_bound_from(&colt, Vec::new()).unwrap());
    assert_eq!(
        cloned.scratch.capacity(),
        0,
        "scratch is not part of the published map"
    );
    assert_eq!(
        drain(&mut cloned, Colt::root(), 0),
        drain(&mut colt, Colt::root(), 0)
    );
    let node_capacity = colt.nodes.capacity();
    colt.release_memory();
    assert_eq!(colt.retained_bytes(), 0);
    assert_eq!(colt.scratch.capacity(), 0);
    drop(colt.reset(all(&view)));
    colt.force_root().unwrap();
    // Vec::push after release starts with its ordinary minimum capacity;
    // the original vec![root] had capacity one. This is not rehash scratch.
    assert_eq!(
        colt.retained_bytes(),
        retained + (colt.nodes.capacity() - node_capacity) * std::mem::size_of::<NodeState>()
    );
    assert_eq!(colt.scratch.capacity(), scratch_capacity);
}

#[test]
fn older_resume_tokens_survive_later_child_growth() {
    let rows: Vec<_> = (0..10)
        .map(|i| (0, i))
        .chain((0..1025).map(|i| (1, i)))
        .collect();
    let view = view_of(&schema(), &rows);
    let (mut colt, old, fresh) = older_maps(&view, false);
    let mut keys = [0; 3];
    let mut children = [Cursor::Row(0); 3];
    let (n, root_token) = colt
        .iter_batch(
            Colt::root(),
            0,
            BatchToken::default(),
            &mut keys,
            &mut children,
            1,
        )
        .unwrap();
    assert_eq!((n, keys[0]), (1, 0));
    let (n, child_token) = colt
        .iter_batch(old, 1, BatchToken::default(), &mut keys, &mut children, 3)
        .unwrap();
    assert_eq!(n, 3);
    assert_eq!(keys, [0, 1, 2]);
    colt.ensure_forced(fresh, 1).unwrap();
    let (n, _) = colt
        .iter_batch(Colt::root(), 0, root_token, &mut keys, &mut children, 1)
        .unwrap();
    assert_eq!((n, keys[0], children[0]), (1, 1, fresh));
    let (n, _) = colt
        .iter_batch(old, 1, child_token, &mut keys, &mut children, 3)
        .unwrap();
    assert_eq!(n, 3);
    assert_eq!(keys, [3, 4, 5]);
}

#[test]
fn partial_control_clear_keeps_published_prefix_and_can_be_discarded() {
    let view = view_of(&schema(), &[(7, 9)]);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0]]);
    colt.force_root().unwrap();
    let mark = colt.pool_mark();
    let prefix = (colt.ctrl.clone(), colt.buckets.clone(), colt.dense.clone());
    let mut map = unpublished(&mut colt, 1);
    for id in 0..26_215 {
        colt.ingest_one(
            &mut map,
            &[id],
            hash_words(&[id]),
            u32::try_from(id).unwrap(),
        )
        .unwrap();
    }
    let ctrl = colt.ctrl.clone();
    let scratch_capacity = colt.scratch.capacity();
    let control_chunk = 64 * 1024;
    assert_eq!(map.nbuckets * 8, control_chunk * 2);
    let bucket_extensions = (map.nbuckets * map.stride() * 8).div_ceil(control_chunk);
    let stage_batches = (map.len as usize).div_ceil(super::super::force::FORCE_BATCH);
    let after = stage_batches + bucket_extensions + 4;
    let (result, seen) = injected(after, WorkError::Cancelled, || colt.grow_map(&mut map));
    assert_eq!(result, Err(WorkError::Cancelled));
    assert_eq!(seen, after + 1);
    assert!(
        colt.ctrl[map.ctrl_start..map.ctrl_start + control_chunk]
            .iter()
            .all(|&c| c == 0)
    );
    assert_eq!(
        &colt.ctrl[map.ctrl_start + control_chunk..],
        &ctrl[map.ctrl_start + control_chunk..]
    );
    assert!(colt.scratch.is_empty());
    assert!(colt.scratch.capacity() >= scratch_capacity);
    // Internal unpublished-map seam: the actual force rollback is exercised
    // separately at every checkpoint by the public child construction test.
    colt.truncate_to(mark);
    assert_eq!(
        (colt.ctrl.clone(), colt.buckets.clone(), colt.dense.clone()),
        prefix
    );
    assert_eq!(drain(&mut colt, Colt::root(), 0)[0].0, [7]);
}

#[test]
fn unrepresentable_growth_fails_before_any_reservation_or_arena_mutation() {
    let view = view_of(&schema(), &[(0, 0)]);
    let base = Map {
        arity: 1,
        nbuckets: 8,
        len: 1,
        ctrl_start: 0,
        bucket_start: 0,
        dense_start: 0,
    };
    for mut map in [
        Map {
            arity: usize::MAX,
            ..base
        },
        Map {
            arity: isize::MAX as usize / 64,
            ..base
        },
        Map {
            nbuckets: usize::MAX,
            ..base
        },
        Map {
            nbuckets: 1 << 29,
            ..base
        },
        Map {
            nbuckets: 0,
            ..base
        },
        Map {
            ctrl_start: usize::MAX,
            ..base
        },
        Map {
            bucket_start: isize::MAX as usize / 8,
            ..base
        },
        Map {
            dense_start: usize::MAX,
            ..base
        },
    ] {
        let mut colt = Colt::new(all(&view), &[], vec![vec![0]]);
        colt.scratch = Vec::with_capacity(123);
        let retained = colt.retained_bytes();
        let before = lengths(&colt);
        assert_eq!(colt.grow_map(&mut map), Err(WorkError::Allocation));
        assert_eq!(colt.retained_bytes(), retained);
        assert_eq!(lengths(&colt), before);
        assert_eq!(colt.scratch.capacity(), 123);
        assert!(colt.scratch.is_empty());
    }
}
