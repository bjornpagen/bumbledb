use std::collections::HashMap;

use super::*;

#[test]
fn growth_layout_refuses_unrepresentable_slots_and_words_before_allocation() {
    use crate::exec::wordmap::grow::growth_layout;

    assert_eq!(growth_layout(0, 0), (WINDOW, 0));
    assert_eq!(growth_layout(8, 3), (16, 48));
    let final_capacity = usize::try_from(1_u64 << 32).expect("64-bit targets");
    assert_eq!(
        growth_layout(final_capacity / 2, 1),
        (final_capacity, final_capacity)
    );
    for (capacity, arity) in [(final_capacity, 1), (usize::MAX, 1), (8, usize::MAX)] {
        assert!(std::panic::catch_unwind(|| growth_layout(capacity, arity)).is_err());
    }
    // In an optimized build this product used to wrap to zero and construct
    // a map whose key backing could not hold even one declared-width key.
    assert!(
        std::panic::catch_unwind(|| {
            WordMap::<()>::with_capacity_hint(usize::MAX / WINDOW + 1, 2)
        })
        .is_err()
    );
}

#[test]
fn panicking_constructor_never_publishes_a_fresh_or_stale_slot() {
    use std::num::NonZeroU64;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    // Arity nine exercises the dynamic entry path; zero and eight bound
    // the constant-arity dispatch. The hint keeps this about publication,
    // not growth, so the entire pre-panic occupancy must remain identical.
    for arity in [0, 1, 8, 9] {
        for recycled in [false, true] {
            let mut map = WordMap::<NonZeroU64>::with_capacity_hint(arity, 8);
            let key = vec![7; arity];
            if recycled {
                map.get_or_insert_with(&key, || NonZeroU64::new(11).unwrap());
                map.clear();
                assert_eq!(map.stale, 1, "exercise a retained stale slot");
            }
            let before_ctrl = map.ctrl.clone();
            let before_stamps = map.stamps.clone();
            let before_dense = map.dense.clone();
            let before_len = map.len;
            let before_stale = map.stale;
            let before_generation = map.generation;

            let failure = catch_unwind(AssertUnwindSafe(|| {
                let _ = map.get_or_insert_with(&key, || panic!("constructor failed"));
            }));
            assert!(failure.is_err());

            // Check raw structure BEFORE lookup/iteration: on the broken
            // implementation an occupied slot can contain uninitialized V.
            // These assertions make that version fail without reading V.
            assert_eq!(map.len, before_len);
            assert_eq!(map.dense, before_dense);
            assert_eq!(map.ctrl, before_ctrl);
            assert_eq!(map.stamps, before_stamps);
            assert_eq!(map.stale, before_stale);
            assert_eq!(map.generation, before_generation);

            let (value, inserted) = map.get_or_insert_with(&key, || NonZeroU64::new(37).unwrap());
            assert!(inserted);
            assert_eq!(value.get(), 37);
            let (value, inserted) =
                map.get_or_insert_with(&key, || panic!("duplicate must not construct"));
            assert!(!inserted);
            assert_eq!(value.get(), 37);
            assert_eq!(map.len(), 1);
            assert_eq!(map.stale, 0);
            let entries: Vec<_> = map
                .iter()
                .map(|(key, value)| (key.to_vec(), value.get()))
                .collect();
            assert_eq!(entries, vec![(key.clone(), 37)]);

            map.clear();
            let (value, inserted) = map.get_or_insert_with(&key, || NonZeroU64::new(53).unwrap());
            assert!(inserted);
            assert_eq!(value.get(), 53);
            assert_eq!(map.len(), 1);
            assert_eq!(map.iter().count(), 1);
        }
    }
}

/// The dense rule: after a hot execution inflates capacity, iteration and
/// clearing stay O(len) — pinned structurally by insertion-order iteration over
/// a high-water map.
#[test]
fn iteration_is_dense_and_insertion_ordered_after_high_water() {
    let mut map: WordMap<u64> = WordMap::new(1);

    for i in 0..50_000u64 {
        map.get_or_insert_with(&[i], || i);
    }
    assert_eq!(map.len(), 50_000);

    map.clear();
    assert_eq!(map.len(), 0);
    assert_eq!(map.iter().count(), 0, "cleared maps iterate nothing");
    for i in [7u64, 3, 9] {
        map.get_or_insert_with(&[i], || i * 10);
    }
    let entries: Vec<(u64, u64)> = map.iter().map(|(k, v)| (k[0], *v)).collect();
    assert_eq!(
        entries,
        vec![(7, 70), (3, 30), (9, 90)],
        "exactly the occupied entries, in insertion order"
    );

    let mut grown: WordMap<()> = WordMap::new(1);
    for i in (0..100u64).rev() {
        grown.insert(&[i]);
    }
    let order: Vec<u64> = grown.iter().map(|(k, ())| k[0]).collect();
    assert_eq!(order, (0..100u64).rev().collect::<Vec<_>>());
}

#[test]
fn insert_dedups_and_survives_rehash() {
    let mut map: WordMap<()> = WordMap::new(2);
    for i in 0..100u64 {
        assert!(map.insert(&[i, i * 2]));
        assert!(!map.insert(&[i, i * 2]));
    }
    assert_eq!(map.len(), 100);
    let mut seen: Vec<u64> = map.iter().map(|(k, ())| k[0]).collect();
    seen.sort_unstable();
    assert_eq!(seen, (0..100).collect::<Vec<u64>>());
}

#[test]
fn values_accumulate_through_get_or_insert() {
    let mut map: WordMap<u64> = WordMap::new(1);
    for i in 0..30u64 {
        let (value, _) = map.get_or_insert_with(&[i % 3], || 0);
        *value += i;
    }
    let mut totals: Vec<(u64, u64)> = map.iter().map(|(k, v)| (k[0], *v)).collect();
    totals.sort_unstable();

    assert_eq!(totals, vec![(0, 135), (1, 145), (2, 155)]);
}

#[test]
fn grow_rewrites_the_dense_list_in_place() {
    let mut map: WordMap<u64> = WordMap::new(1);
    for i in 0..20u64 {
        map.get_or_insert_with(&[i], || i * 3);
    }
    let ptr = map.dense.as_ptr();
    let capacity = map.dense.capacity();
    map.grow();
    assert_eq!(map.dense.as_ptr(), ptr, "grow re-allocated the dense list");
    assert_eq!(map.dense.capacity(), capacity);
    assert_eq!(map.len(), 20);
    let keys: Vec<u64> = map.iter().map(|(k, _)| k[0]).collect();
    assert_eq!(
        keys,
        (0..20).collect::<Vec<u64>>(),
        "insertion order survives"
    );
    for i in 0..20u64 {
        let (value, inserted) = map.get_or_insert_with(&[i], || 0);
        assert!(!inserted);
        assert_eq!(*value, i * 3, "values survive the rehash");
    }
}

#[test]
fn zero_arity_keys_share_one_group() {
    let mut map: WordMap<u64> = WordMap::new(0);
    for _ in 0..5 {
        let (value, _) = map.get_or_insert_with(&[], || 0);
        *value += 1;
    }
    assert_eq!(map.len(), 1);
    assert_eq!(map.iter().next().map(|(k, v)| (k.len(), *v)), Some((0, 5)));
}

#[test]
fn differential_against_the_reference_model() {
    let ops_per_round: u64 = if cfg!(miri) { 256 } else { 2_000 };
    let mut rng = 0x2468_ACE0_1357_9BDFu64;
    let mut next = move || {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        rng >> 33
    };
    for arity in [1usize, 2, 3, 4, 5, 6, 7, 8, 9] {
        for round in 0..3 {
            let mut map: WordMap<u64> = if round == 0 {
                WordMap::new(arity)
            } else {
                WordMap::with_capacity_hint(arity, 64 << round)
            };
            let mut model: HashMap<Vec<u64>, u64> = HashMap::new();
            let mut order: Vec<Vec<u64>> = Vec::new();
            for op in 0..ops_per_round {
                let key: Vec<u64> = (0..arity)
                    .map(|_| match next() % 8 {
                        0 => 0,
                        1 => u64::MAX,
                        2 => next() << 32,
                        _ => next() % 64,
                    })
                    .collect();
                let (value, inserted) = map.get_or_insert_with(&key, || op);
                match model.get(&key) {
                    None => {
                        assert!(inserted, "model says new");
                        model.insert(key.clone(), op);
                        order.push(key.clone());
                    }
                    Some(existing) => {
                        assert!(!inserted, "model says present");
                        assert_eq!(value, existing, "value survives");
                    }
                }
            }
            assert_eq!(map.len(), model.len());
            let got: Vec<(Vec<u64>, u64)> = map.iter().map(|(k, v)| (k.to_vec(), *v)).collect();
            let expected: Vec<(Vec<u64>, u64)> =
                order.iter().map(|k| (k.clone(), model[k])).collect();
            assert_eq!(got, expected, "insertion-order iteration");

            map.clear();
            assert_eq!(map.len(), 0);
            assert!(map.insert(&vec![41u64; arity]));
            assert!(!map.insert(&vec![41u64; arity]));
        }
    }
}

/// The mirror invariant: ctrl's tail `WINDOW-1` bytes always equal its head
/// bytes — through inserts, clears, and growth — so window loads at high
/// indices see the wrapped slots correctly.
#[test]
fn the_ctrl_mirror_tracks_the_head() {
    let mut map: WordMap<()> = WordMap::with_capacity_hint(1, 4);
    for i in 0..200u64 {
        map.insert(&[i.wrapping_mul(0x9E37_79B9_7F4A_7C15)]);
        let capacity = map.values.len();
        assert_eq!(
            &map.ctrl[capacity..capacity + WINDOW - 1],
            &map.ctrl[..WINDOW - 1],
            "mirror out of sync after insert {i}"
        );
    }
    map.clear();
    let capacity = map.values.len();
    assert_eq!(
        &map.ctrl[capacity..capacity + WINDOW - 1],
        &map.ctrl[..WINDOW - 1],
        "mirror out of sync after clear"
    );

    for i in 0..200u64 {
        map.insert(&[i.wrapping_mul(0x9E37_79B9_7F4A_7C15)]);
        let capacity = map.values.len();
        assert_eq!(
            &map.ctrl[capacity..capacity + WINDOW - 1],
            &map.ctrl[..WINDOW - 1],
            "mirror out of sync after reclaiming insert {i}"
        );
    }
}

#[test]
fn generational_clear_never_ghosts_and_reclaims_warm_slots() {
    let mut map: WordMap<()> = WordMap::with_capacity_hint(2, 512);
    for round in 0..600u64 {
        let base = (round % 2) * 1_000_000;
        for i in 0..300u64 {
            assert!(map.insert(&[base + i, i]), "round {round}: first sight");
            assert!(!map.insert(&[base + i, i]), "round {round}: duplicate");
        }
        assert_eq!(map.len(), 300);
        map.clear();
        assert_eq!(map.len(), 0);
        assert_eq!(map.iter().count(), 0, "cleared maps iterate nothing");
    }
}

#[test]
fn clear_retains_ctrl_until_saturation_forces_the_physical_reset() {
    let mut map: WordMap<()> = WordMap::with_capacity_hint(1, 64);
    for i in 0..40u64 {
        map.insert(&[i]);
    }
    map.clear();
    assert!(
        map.ctrl.iter().any(|&c| c != 0),
        "the generation clear leaves ctrl bytes standing"
    );
    assert_eq!(map.stale, 40, "every cleared entry turned stale");

    for i in 0..40u64 {
        assert!(map.insert(&[i]), "cleared keys are first sights again");
    }
    assert_eq!(map.stale, 0, "same-universe reuse reclaims every slot");

    let mut universe = 1u64;
    loop {
        map.clear();
        if map.stale == 0 {
            break;
        }
        for i in 0..40u64 {
            map.insert(&[universe * 1_000 + i]);
        }
        universe += 1;
        assert!(universe < 20, "saturation must trip within the sweep");
    }
    assert!(
        map.ctrl.iter().all(|&c| c == 0),
        "the physical reset emptied every ctrl byte"
    );

    assert!(map.insert(&[7]));
    assert!(!map.insert(&[7]));
    assert_eq!(map.len(), 1);
}

#[test]
fn a_covering_hint_never_grows() {
    let mut map: WordMap<()> = WordMap::with_capacity_hint(2, 100_000);
    let capacity = map.values.len();
    for i in 0..100_000u64 {
        map.insert(&[i, i ^ 0x5555]);
    }
    assert_eq!(map.len(), 100_000);
    assert_eq!(map.values.len(), capacity, "no rehash under the hint");
    assert!(
        map.len() * LOAD_DEN <= capacity,
        "the covered hint keeps load at the shipped max"
    );
}

#[test]
fn bulk_rows_preserve_exact_single_row_state_through_growth_and_reuse() {
    fn assert_same(bulk: &WordMap<u64>, single: &WordMap<u64>) {
        assert_eq!(bulk.arity, single.arity);
        assert_eq!(bulk.ctrl, single.ctrl);
        assert_eq!(bulk.stamps, single.stamps);
        assert_eq!(bulk.dense, single.dense);
        assert_eq!(bulk.keys, single.keys);
        assert_eq!(bulk.len, single.len);
        assert_eq!(bulk.stale, single.stale);
        assert_eq!(bulk.generation, single.generation);
        assert_eq!(bulk.capacity(), single.capacity());
        assert_eq!(bulk.ctrl.capacity(), single.ctrl.capacity());
        assert_eq!(bulk.keys.capacity(), single.keys.capacity());
        assert_eq!(bulk.values.capacity(), single.values.capacity());
        assert_eq!(bulk.stamps.capacity(), single.stamps.capacity());
        assert_eq!(bulk.dense.capacity(), single.dense.capacity());
        // Iteration reads only initialized values, in insertion order.
        assert!(bulk.iter().eq(single.iter()));
    }

    // Miri exercises every kernel through growth and stale reuse with a
    // smaller stream; native tests cover the full chunk-boundary matrix.
    let chunk_sizes: &[usize] = if cfg!(miri) { &[7] } else { &[1, 2, 7, 31] };
    let rounds: u64 = if cfg!(miri) { 2 } else { 4 };
    let rows: u64 = if cfg!(miri) { 33 } else { 129 };
    for arity in 1..=9 {
        for &chunk_rows in chunk_sizes {
            let mut bulk = WordMap::<u64>::new(arity);
            let mut single = WordMap::<u64>::new(arity);
            for round in 0..rounds {
                let words: Vec<_> = (0..rows)
                    .flat_map(|index| {
                        (0..arity).map(move |column| {
                            // Adjacent duplicates cross growth thresholds;
                            // alternating universes also exercise stale slots.
                            (index / 2 + (round % 2) * 1_000)
                                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                                .rotate_left(u32::try_from(column).unwrap())
                        })
                    })
                    .collect();
                bulk.insert_rows(&[]);
                assert_same(&bulk, &single);
                for chunk in words.chunks(chunk_rows * arity) {
                    bulk.insert_rows(chunk);
                    for row in chunk.chunks_exact(arity) {
                        single.insert(row);
                    }
                    assert_same(&bulk, &single);
                }
                bulk.clear();
                single.clear();
                assert_same(&bulk, &single);
            }
        }
    }
}

#[test]
#[should_panic(expected = "bulk insertion requires nonzero arity")]
fn bulk_rows_refuse_zero_arity_even_for_empty_words() {
    WordMap::<()>::new(0).insert_rows(&[]);
}

#[test]
#[should_panic(expected = "incomplete bulk row")]
fn bulk_rows_refuse_incomplete_rows() {
    WordMap::<()>::new(2).insert_rows(&[1, 2, 3]);
}
