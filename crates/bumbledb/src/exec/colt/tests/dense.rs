use super::*;

#[test]
fn skewed_maps_size_by_the_formula_and_iterate_densely() {
    let schema = schema();

    let rows: Vec<(u64, u64)> = (0..100_000).map(|i| (i % 500, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    colt.ensure_forced(root, 0).expect("force");

    // next_pow2(12_500 * 5 / 16) = 4_096 → 32_768 slots

    assert_eq!(colt.forced_capacity(root), Some(32_768));

    let mut keys = vec![0u64; 64];
    let mut children = vec![Cursor::Row(0); 64];
    let mut token = BatchToken::default();
    let mut calls = 0;
    let mut total = 0;
    loop {
        let (n, next) = colt
            .iter_batch(root, 0, token, &mut keys, &mut children, 64)
            .expect("iter");
        if n == 0 {
            break;
        }
        calls += 1;
        total += n;
        assert_eq!(n, if calls <= 7 { 64 } else { 500 - 7 * 64 });
        token = next;
    }
    assert_eq!((calls, total), (8, 500), "O(keys) drain");
}

#[test]
fn near_distinct_maps_grow_to_the_pinned_capacity() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..10_000).map(|i| (i, i * 2)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    colt.ensure_forced(root, 0).expect("force");

    assert_eq!(colt.forced_capacity(root), Some(32_768));
    let entries = drain(&mut colt, root, 0);
    assert_eq!(entries.len(), 10_000);
    let keys: Vec<u64> = entries.iter().map(|(k, _)| k[0]).collect();
    let mut seen = keys.clone();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 10_000, "each key exactly once");

    let mut again = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let repeat: Vec<u64> = drain(&mut again, root, 0)
        .iter()
        .map(|(k, _)| k[0])
        .collect();
    assert_eq!(keys, repeat, "ingestion order survives growth");
}

#[test]
fn dense_tokens_resume_across_interleaved_probes() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..300).map(|i| (i % 40, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    let single_shot = drain(&mut colt, root, 0);

    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let mut keys = vec![0u64; 1];
    let mut children = vec![Cursor::Row(0); 1];
    let mut token = BatchToken::default();
    let mut stepped = Vec::new();
    loop {
        let (n, next) = colt
            .iter_batch(root, 0, token, &mut keys, &mut children, 1)
            .expect("iter");
        if n == 0 {
            break;
        }
        stepped.push((keys.clone(), children[0]));
        // An interleaved probe must not disturb the resume token.
        let _ = colt.get(root, 0, &[stepped.len() as u64 % 40]);
        token = next;
    }
    assert_eq!(stepped.len(), single_shot.len());
    for (a, b) in stepped.iter().zip(single_shot.iter()) {
        assert_eq!((&a.0, a.1), (&b.0, b.1));
    }
}

#[test]
fn chunked_lists_round_trip_far_beyond_one_chunk() {
    let schema = schema();

    let rows: Vec<(u64, u64)> = (0..300).map(|i| (42, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let child = colt.get(Colt::root(), 0, &[42]).expect("hit");
    assert!(matches!(colt.key_count(child), KeyCount::Estimate(300)));
    let values = drain(&mut colt, child, 1);
    assert_eq!(values.len(), 300);
    let mut got: Vec<u64> = values.into_iter().map(|(k, _)| k[0]).collect();
    got.sort_unstable();
    assert_eq!(got, (0..300).collect::<Vec<u64>>());
}

/// A resume token minted under positions iteration is refused after its node is
/// forced — the release assert fires instead of silently reinterpreting the
/// token as a dense index (the omission wrong-results class). A fresh token
/// after the force drains the full, correct key set.
#[test]
fn a_token_that_outlives_a_force_is_refused() {
    let schema = schema();

    let rows: Vec<(u64, u64)> = (0..200).map(|i| (7, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let child = colt.get(Colt::root(), 0, &[7]).expect("key 7 exists");
    let mut keys = vec![0u64; 8];
    let mut children = vec![Cursor::Row(0); 8];
    let (n, token) = colt
        .iter_batch(child, 1, BatchToken::default(), &mut keys, &mut children, 8)
        .expect("iter");
    assert_eq!(n, 8);
    let (n, stale) = colt
        .iter_batch(child, 1, token, &mut keys, &mut children, 8)
        .expect("iter");
    assert_eq!(n, 8, "two positions batches drained");

    colt.ensure_forced(child, 1).expect("force");
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut keys = vec![0u64; 8];
        let mut children = vec![Cursor::Row(0); 8];
        colt.iter_batch(child, 1, stale, &mut keys, &mut children, 8)
    }))
    .expect_err("the stale token must be refused");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .expect("string panic payload");
    assert!(message.contains("outlived a force"), "{message}");

    let entries = drain(&mut colt, child, 1);
    assert_eq!(entries.len(), 200);
    let mut values: Vec<u64> = entries.iter().map(|(k, _)| k[0]).collect();
    values.sort_unstable();
    assert_eq!(values, (0..200).collect::<Vec<u64>>());
}

/// A resume token minted in one generation is refused after a reset — the epoch
/// field (bits 56–62) closes the second staleness axis the bit-63 tag left
/// open: silent truncation (Root arm) or cross-node position yields
/// (Chunks/dense arms) against the re-minted pools.
#[test]
fn a_token_that_outlives_a_reset_is_refused() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..200).map(|i| (7, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let child = colt.get(Colt::root(), 0, &[7]).expect("key 7 exists");
    let mut keys = vec![0u64; 8];
    let mut children = vec![Cursor::Row(0); 8];
    let (n, stale) = colt
        .iter_batch(child, 1, BatchToken::default(), &mut keys, &mut children, 8)
        .expect("iter");
    assert_eq!(n, 8);

    let _ = colt.reset(all(&view));
    let child = colt.get(Colt::root(), 0, &[7]).expect("key 7 exists again");
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut keys = vec![0u64; 8];
        let mut children = vec![Cursor::Row(0); 8];
        colt.iter_batch(child, 1, stale, &mut keys, &mut children, 8)
    }))
    .expect_err("the stale token must be refused");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .expect("string panic payload");
    assert!(message.contains("outlived a reset"), "{message}");

    let entries = drain(&mut colt, child, 1);
    assert_eq!(entries.len(), 200);
    let mut values: Vec<u64> = entries.iter().map(|(k, _)| k[0]).collect();
    values.sort_unstable();
    assert_eq!(values, (0..200).collect::<Vec<u64>>());
}

#[test]
fn row_cursor_iteration_honors_max() {
    let schema = schema();
    let view = view_of(&schema, &[(1, 5)]);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let child = colt.get(Colt::root(), 0, &[1]).expect("key 1 exists");
    assert!(matches!(child, Cursor::Row(_)), "singleton pins a row");

    let (n, token) = colt
        .iter_batch(child, 1, BatchToken::default(), &mut [], &mut [], 0)
        .expect("iter");
    assert_eq!(n, 0, "max = 0 yields nothing");
    let mut keys = vec![0u64; 1];
    let mut children = vec![Cursor::Row(0); 1];
    let (n, token) = colt
        .iter_batch(child, 1, token, &mut keys, &mut children, 1)
        .expect("iter");
    assert_eq!((n, keys[0]), (1, 5), "max = 1 yields exactly the row");
    let (n, _) = colt
        .iter_batch(child, 1, token, &mut keys, &mut children, 1)
        .expect("iter");
    assert_eq!(n, 0, "the row yields once");
}
