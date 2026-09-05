use super::*;

#[test]
fn overflowing_home_buckets_chain_to_the_next_and_round_trip() {
    // 8 buckets: count 12 → guess 16 → next_pow2(16·5/16) = 8).
    let mut keys: Vec<u64> = Vec::new();
    let mut candidate = 0u64;
    while keys.len() < 12 {
        if usize::try_from(hash_words(&[candidate])).expect("64-bit") & 7 == 3 {
            keys.push(candidate);
        }
        candidate += 1;
    }
    let schema = schema();
    let rows: Vec<(u64, u64)> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, i as u64))
        .collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    colt.ensure_forced(root, 0).expect("force");
    assert_eq!(colt.forced_capacity(root), Some(64), "8 buckets");

    let column: Vec<u64> = view.column_words(0).to_vec();
    for key in &keys {
        let child = colt.get(root, 0, &[*key]).expect("overflowed key probes");
        let position = column.iter().position(|w| w == key).expect("committed");
        assert_eq!(
            child,
            Cursor::Row(u32::try_from(position).expect("small")),
            "key {key}"
        );
    }

    let mut miss = candidate;
    let mut checked = 0;
    while checked < 4 {
        if usize::try_from(hash_words(&[miss])).expect("64-bit") & 7 == 3 {
            assert_eq!(colt.get(root, 0, &[miss]), None, "miss {miss}");
            checked += 1;
        }
        miss += 1;
    }

    let drained: Vec<u64> = drain(&mut colt, root, 0)
        .iter()
        .map(|(k, _)| k[0])
        .collect();
    assert_eq!(drained, column, "dense drain in ingest order");
}
