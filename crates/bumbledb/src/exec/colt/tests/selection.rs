use super::*;

/// Selection levels (docs/architecture/30-execution.md): probing lands exactly on the
/// filtered subtrie a view scan used to produce.
#[test]
fn selection_levels_probe_to_the_filtered_subtrie() {
    let dir = TempDir::new("colt-select");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    // Selection on k (column 0); one join level on v (column 1).
    let mut colt = Colt::new(all(&view), &[0], vec![vec![1]]);
    let cursor = colt.select(&[7]).expect("key 7 exists");
    assert_eq!(colt.start(), cursor);
    let entries = drain(&mut colt, cursor, 0);
    assert_eq!(entries.len(), 100, "exactly k = 7's positions");
    assert!(entries.iter().all(|(key, _)| key[0] % 10 == 7));
    // An absent key: the occurrence is empty on this snapshot.
    assert!(colt.select(&[42]).is_none());
}

/// Two selections chain; a contradictory pair yields `None` with no
/// special casing.
#[test]
fn chained_selections_intersect_and_contradict() {
    let dir = TempDir::new("colt-select-chain");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..100).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    // Selections on k then v; the join level is 0-arity (a constant
    // atom's shape: trie_schema = [[]]).
    let mut colt = Colt::new(all(&view), &[0, 1], vec![vec![]]);
    let cursor = colt.select(&[3, 13]).expect("(3, 13) exists");
    let entries = drain(&mut colt, cursor, 0);
    assert_eq!(entries.len(), 1, "one fact carries (3, 13)");
    // 14 % 10 == 4, so (k = 3, v = 14) contradicts at level 1.
    assert!(colt.select(&[3, 14]).is_none());
}

/// A selection-free trie is the old trie: `select(&[])` is the root
/// and iteration is identical.
#[test]
fn zero_selection_tries_are_the_old_tries() {
    let dir = TempDir::new("colt-select-zero");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..200).map(|i| (i % 20, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut plain = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    let mut selected = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
    assert_eq!(selected.start(), Colt::root());
    let cursor = selected.select(&[]).expect("no selections always hit");
    assert_eq!(cursor, Colt::root());
    let a = drain(&mut plain, Colt::root(), 0);
    let b = drain(&mut selected, cursor, 0);
    assert_eq!(a.len(), b.len());
    assert_eq!(
        a.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
        b.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>()
    );
}

/// `key_count` labels stay honest below a selection probe.
#[test]
fn key_count_labels_below_selections() {
    let dir = TempDir::new("colt-select-count");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&view), &[0], vec![vec![1]]);
    let cursor = colt.select(&[7]).expect("key 7 exists");
    // Unforced below the selection: a position-count Estimate.
    assert_eq!(colt.key_count(cursor), KeyCount::Estimate(100));
    // Forcing the join level turns it Exact (v values are distinct).
    colt.ensure_forced(cursor, 0);
    assert_eq!(colt.key_count(cursor), KeyCount::Exact(100));
}

/// Two reset + select rounds land on the same pool shape — slabs are
/// recycled, not regrown.
#[test]
fn reset_retains_selection_capacity() {
    let dir = TempDir::new("colt-select-reset");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..500).map(|i| (i % 5, i)).collect();
    let image = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&image), &[0], vec![vec![1]]);
    colt.select(&[3]).expect("key 3 exists");
    let first = colt.watermark();
    colt.reset(apply(&image, &[], &[], Vec::new()));
    assert_eq!(colt.watermark(), 1, "reset empties the pools");
    colt.select(&[3]).expect("key 3 exists");
    assert_eq!(colt.watermark(), first, "same shape, same footprint");
}

/// PRD 04: starting a selection-bearing colt before `select()` is a
/// release panic — silently dropped selections are wrong results.
#[test]
fn start_before_select_panics() {
    let dir = TempDir::new("colt-hard-start");
    let schema = schema();
    let view = view_of(&dir, &schema, &[(1, 5)]);
    let colt = Colt::new(all(&view), &[0], vec![vec![1]]);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| colt.start()))
        .expect_err("unselected start must panic");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .expect("string panic payload");
    assert!(
        message.contains("select() runs before the join"),
        "{message}"
    );
}
