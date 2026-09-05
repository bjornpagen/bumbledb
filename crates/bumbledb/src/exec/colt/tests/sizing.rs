use super::*;

#[test]
fn construction_is_lazy_until_the_first_get() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..10_000).map(|i| (i % 100, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let baseline = colt.watermark();
    assert_eq!(baseline, 1, "one root node, nothing else");

    let root = Colt::root();
    let child = colt.get(root, 0, &[7]).expect("key 7 exists");
    assert!(colt.watermark() > baseline);

    assert!(matches!(child, Cursor::Node(_)));
    assert!(matches!(colt.key_count(child), KeyCount::Estimate(100)));
}

#[test]
fn suffix_iteration_never_forces() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..500).map(|i| (i, i * 2)).collect();
    let view = view_of(&schema, &rows);

    let mut colt = Colt::new(all(&view), &[], vec![vec![0, 1]]);
    let before = colt.watermark();
    let root = Colt::root();
    let entries = drain(&mut colt, root, 0);
    assert_eq!(entries.len(), 500);
    assert_eq!(colt.watermark(), before, "no forcing, no allocation");

    assert!(entries.iter().all(|(_, c)| matches!(c, Cursor::Row(_))));
}

#[test]
fn singleton_keys_allocate_no_chunks() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..100).map(|i| (i, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let child = colt.get(Colt::root(), 0, &[5]).expect("hit");

    assert!(matches!(child, Cursor::Row(_)));
    assert_eq!(colt.chunks.len(), 0);
}

#[test]
fn small_fanouts_reserve_small_first_chunks() {
    let schema = schema();

    let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i / 2, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    colt.ensure_forced(Colt::root(), 0);
    assert_eq!(colt.chunks.len(), 500, "one small frame per fanout-2 key");
    assert_eq!(
        colt.chunk_positions.len(),
        500 * 8,
        "the first frame is FIRST_CHUNK_CAP positions, not 64"
    );

    let rows: Vec<(u64, u64)> = (0..100).map(|i| (0, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    colt.ensure_forced(Colt::root(), 0);
    assert_eq!(colt.chunks.len(), 3);
    assert_eq!(colt.chunk_positions.len(), 8 + 64 + 64);
    let child = colt.get(Colt::root(), 0, &[0]).expect("hit");
    assert_eq!(drain(&mut colt, child, 1).len(), 100);
}

#[test]
fn key_count_labels_are_honest_in_both_states() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..60).map(|i| (i % 3, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();

    assert_eq!(colt.key_count(root), KeyCount::Estimate(60));
    colt.get(root, 0, &[0]);

    assert_eq!(colt.key_count(root), KeyCount::Exact(3));
}

#[test]
fn zero_arity_levels_gate_on_nonemptiness() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = vec![(1, 2), (3, 4)];
    let view = view_of(&schema, &rows);

    let mut colt = Colt::new(all(&view), &[], vec![vec![]]);
    let root = Colt::root();
    let entries = drain(&mut colt, root, 0);

    assert_eq!(entries.len(), 2);
    let mut colt = Colt::new(all(&view), &[], vec![vec![]]);
    assert!(colt.get(Colt::root(), 0, &[]).is_some());
}
