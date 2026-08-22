use std::collections::HashMap;

use super::*;

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
