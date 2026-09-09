use super::{Dir, OverlapCache, Probe};
use std::collections::BTreeMap;

fn build(cache: &mut OverlapCache, key: &[u64], group: &[(u64, u64, u32)]) -> u32 {
    let tree_base = cache.tree.len();
    assert_eq!(
        cache.probe(key, |_| panic!("first probe must decline")),
        Probe::Declined
    );
    let Probe::Ready(dir) = cache.probe(key, |triples| triples.extend_from_slice(group)) else {
        panic!("second probe must build");
    };
    let end_words = if group.len() <= super::FLAT_SWEEP_CEILING {
        group.len()
    } else {
        2 * group.len().next_power_of_two()
    };
    assert_eq!(
        cache.tree.len(),
        tree_base + end_words,
        "only large groups need tree padding"
    );
    dir
}

#[test]
fn flat_and_tree_groups_preserve_two_inequality_order_through_growth_and_reset() {
    let mut cache = OverlapCache::default();
    let mut out = Vec::with_capacity(1024);
    let out_capacity = out.capacity();
    for generation in 0..2u64 {
        cache.reset();
        let mut lengths = [
            0usize, 1, 7, 8, 9, 15, 16, 17, 31, 32, 33, 127, 128, 129, 256, 511,
        ];
        if generation != 0 {
            lengths.reverse();
        }
        let mut groups = Vec::new();
        for (index, len) in lengths.into_iter().enumerate() {
            let group: Vec<_> = (0..len)
                .map(|i| {
                    let start = (i as u64 % 8) * 3 + generation;
                    let end = if i % 5 == 0 {
                        u64::MAX
                    } else {
                        start + 1 + i as u64 % 17
                    };
                    let position = u32::try_from((len - i) * 17 + 3).unwrap();
                    (start, end, position)
                })
                .collect();
            let key = [generation, index as u64];
            let dir = build(&mut cache, &key, &group);
            groups.push((key, dir, group));
        }
        // Rehash after the mixed empty/flat/tree slabs have been built.
        for key in 10_000..10_300 {
            assert_eq!(
                cache.probe(&[key], |_| panic!("tally only")),
                Probe::Declined
            );
        }
        for (key, dir, group) in &groups {
            assert_eq!(
                cache.probe(key, |_| panic!("already built")),
                Probe::Ready(*dir)
            );
            assert_eq!(cache.len_of(*dir), group.len());
            let Dir::Built { base, len, .. } = cache.dirs[*dir as usize] else {
                panic!("built");
            };
            let span = base as usize..(base + len) as usize;
            assert!(
                cache.starts[span.clone()]
                    .windows(2)
                    .all(|pair| pair[0] <= pair[1])
            );
            let source: BTreeMap<_, _> = group
                .iter()
                .map(|&(start, end, pos)| (pos, (start, end)))
                .collect();
            let order = &cache.positions[span];
            let mut positions = order.to_vec();
            positions.sort_unstable();
            assert_eq!(positions, source.keys().copied().collect::<Vec<_>>());
            // Equal/reversed constraints are two inequalities, not invalid
            // interval values: a spanning interval may satisfy both.
            for (q_start, q_end) in [
                (0, u64::MAX),
                (u64::MAX, u64::MAX),
                (12, 8),
                (8, 8),
                (0, 0),
                (0, 1),
                (10, 20),
                (u64::MAX - 1, u64::MAX),
                (u64::MAX, 0),
                (0, u64::MAX),
            ] {
                let expected: Vec<_> = order
                    .iter()
                    .copied()
                    .filter(|pos| {
                        let &(start, end) = source.get(pos).unwrap();
                        start < q_end && end > q_start
                    })
                    .collect();
                cache.query_into(*dir, q_start, q_end, &mut out);
                assert_eq!(
                    out, expected,
                    "generation={generation} len={len} bounds={q_start}/{q_end}"
                );
                assert_eq!(
                    out.capacity(),
                    out_capacity,
                    "preallocated output never grows"
                );
            }
        }
    }
}
