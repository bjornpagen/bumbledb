use super::*;

#[test]
fn construction_is_lazy_until_the_first_get() {
    let dir = TempDir::new("colt-lazy");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..10_000).map(|i| (i % 100, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let baseline = colt.watermark();
    assert_eq!(baseline, 1, "one root node, nothing else");
    // The first get forces exactly one level.
    let root = Colt::root();
    let child = colt.get(root, 0, &[7]).expect("key 7 exists");
    assert!(colt.watermark() > baseline);
    // The child is a real (chunked) node, still unforced.
    assert!(matches!(child, Cursor::Node(_)));
    assert!(matches!(colt.key_count(child), KeyCount::Estimate(100)));
}

#[test]
fn suffix_iteration_never_forces() {
    let dir = TempDir::new("colt-suffix");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..500).map(|i| (i, i * 2)).collect();
    let view = view_of(&dir, &schema, &rows);
    // Single-level schema: the root's remaining schema is a suffix.
    let mut colt = Colt::new(all(&view), &[], vec![vec![0, 1]]);
    let before = colt.watermark();
    let root = Colt::root();
    let entries = drain(&mut colt, root, 0);
    assert_eq!(entries.len(), 500);
    assert_eq!(colt.watermark(), before, "no forcing, no allocation");
    // Every child is a pinned row.
    assert!(entries.iter().all(|(_, c)| matches!(c, Cursor::Row(_))));
}

#[test]
fn singleton_keys_allocate_no_chunks() {
    let dir = TempDir::new("colt-singleton");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..100).map(|i| (i, i)).collect(); // all unique
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let child = colt.get(Colt::root(), 0, &[5]).expect("hit");
    // Singletons pin rows inline: no chunk, no extra node.
    assert!(matches!(child, Cursor::Row(_)));
    assert_eq!(colt.chunks.len(), 0);
}

#[test]
fn key_count_labels_are_honest_in_both_states() {
    let dir = TempDir::new("colt-key-count");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..60).map(|i| (i % 3, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    // Unforced: duplicate-inflated Estimate.
    assert_eq!(colt.key_count(root), KeyCount::Estimate(60));
    colt.get(root, 0, &[0]);
    // Forced: exact distinct keys.
    assert_eq!(colt.key_count(root), KeyCount::Exact(3));
}

#[test]
fn zero_arity_levels_gate_on_nonemptiness() {
    let dir = TempDir::new("colt-nullary");
    let schema = schema();
    let rows: Vec<(u64, u64)> = vec![(1, 2), (3, 4)];
    let view = view_of(&dir, &schema, &rows);
    // A zero-binding occurrence: one empty level.
    let mut colt = Colt::new(all(&view), &[], vec![vec![]]);
    let root = Colt::root();
    let entries = drain(&mut colt, root, 0);
    // Suffix iteration yields one entry per position (empty keys);
    // a probe with the empty key forces and hits iff nonempty.
    assert_eq!(entries.len(), 2);
    let mut colt = Colt::new(all(&view), &[], vec![vec![]]);
    assert!(colt.get(Colt::root(), 0, &[]).is_some());
}
