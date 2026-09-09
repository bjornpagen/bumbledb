use super::*;

#[test]
fn result_backing_waits_for_actual_rows() {
    let mut set = SpillSet::with_capacity_hint(1, 1 << 21, true);
    assert_eq!(
        set.ram.retained_bytes_for_test(),
        0,
        "no speculative result hash backing before the first row"
    );
    for value in 0..5 {
        assert!(set.insert(&[value]));
        assert!(!set.insert(&[value]));
    }
    assert_eq!(set.ram.retained_bytes_for_test(), 199);
    assert_eq!(
        set.ram.iter().map(|(key, ())| key[0]).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
}

#[test]
fn all_widths_grow_preserve_order_and_reuse_then_release() {
    for width in [0, 1, 2, 4, 8, 9] {
        for ordered in [false, true] {
            let mut set = SpillSet::with_capacity_hint(width, 0, ordered);
            assert_eq!(set.ram.retained_bytes_for_test(), 0);
            let mut key = vec![0; width];
            let count = if width == 0 { 1 } else { 513 };
            for value in 0..count {
                key.fill(value);
                assert!(set.insert(&key));
                assert!(!set.insert(&key));
            }
            assert_eq!(set.len(), usize::try_from(count).unwrap());
            for (value, (key, ())) in set.ram.iter().enumerate() {
                assert!(
                    key.iter()
                        .all(|word| *word == u64::try_from(value).unwrap())
                );
            }
            let retained = set.ram.retained_bytes_for_test();
            // Exercise both sparse stale reuse and generation rollover.
            for round in 0..300 {
                set.clear();
                key.fill(round);
                let before = crate::alloc_counter::snapshot().window;
                assert!(set.insert(&key));
                assert!(!set.insert(&key));
                let after = crate::alloc_counter::snapshot().window;
                assert_eq!(set.ram.retained_bytes_for_test(), retained);
                assert_eq!(before.allocs, after.allocs);
                assert_eq!(before.alloc_bytes, after.alloc_bytes);
                assert_eq!(set.ram.iter().next().unwrap().0, key);
            }
            set.release_memory();
            assert_eq!(set.ram.retained_bytes_for_test(), 0);
            set.clear();
            assert!(set.insert(&key));
            assert_eq!(set.len(), 1);
            assert_eq!(set.ram.iter().next().unwrap().0, key);
        }
    }
}

#[test]
fn aggregate_regimes_do_not_reserve_speculative_tables() {
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let spans = [(0, 1), (1, 1)];
    let sinks = [
        AggregateSink::with_capacity_hint(&finds, 2, 0, &[]),
        AggregateSink::for_union(&finds, 2, 0),
        AggregateSink::for_dnf_union(&finds, 2, &spans, 0),
    ];
    for sink in sinks {
        let GroupTable::Hashed(groups) = &sink.groups else {
            panic!("hashed fixture");
        };
        assert_eq!(groups.retained_bytes_for_test(), 0);
        assert_eq!(sink.dedup.seen().unwrap().ram.retained_bytes_for_test(), 0);
    }
    let dense = AggregateSink::with_capacity_hint(&finds, 2, 0, &[2]);
    let GroupTable::Dense {
        radixes,
        table,
        ordinals,
    } = &dense.groups
    else {
        panic!("dense fixture");
    };
    assert_eq!(radixes.as_ref(), [2]);
    assert_eq!(table.as_ref(), [0, 0]);
    assert!(ordinals.is_empty());
    assert_eq!(dense.dedup.seen().unwrap().ram.retained_bytes_for_test(), 0);
}
