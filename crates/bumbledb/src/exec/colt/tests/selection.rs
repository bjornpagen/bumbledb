use super::*;

#[test]
fn selection_levels_probe_to_the_filtered_subtrie() {
    let dir = TempDir::new("colt-select");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);

    let mut colt = Colt::new(all(&view), &scalars(&[0]), vec![vec![1]]);
    let cursor = colt.select(&[vec![7]]).expect("key 7 exists");
    assert_eq!(colt.start(), cursor);
    let entries = drain(&mut colt, cursor, 0);
    assert_eq!(entries.len(), 100, "exactly k = 7's positions");
    assert!(entries.iter().all(|(key, _)| key[0] % 10 == 7));

    assert!(colt.select(&[vec![42]]).is_none());
}

#[test]
fn chained_selections_intersect_and_contradict() {
    let dir = TempDir::new("colt-select-chain");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..100).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);

    let mut colt = Colt::new(all(&view), &scalars(&[0, 1]), vec![vec![]]);
    let cursor = colt.select(&[vec![3], vec![13]]).expect("(3, 13) exists");
    let entries = drain(&mut colt, cursor, 0);
    assert_eq!(entries.len(), 1, "one fact carries (3, 13)");

    assert!(colt.select(&[vec![3], vec![14]]).is_none());
}

#[test]
fn zero_selection_tries_are_the_old_tries() {
    let dir = TempDir::new("colt-select-zero");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..200).map(|i| (i % 20, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut plain = Colt::new(all(&view), &scalars(&[]), vec![vec![0], vec![1]]);
    let mut selected = Colt::new(all(&view), &scalars(&[]), vec![vec![0], vec![1]]);
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

#[test]
fn key_count_labels_below_selections() {
    let dir = TempDir::new("colt-select-count");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&view), &scalars(&[0]), vec![vec![1]]);
    let cursor = colt.select(&[vec![7]]).expect("key 7 exists");

    assert_eq!(colt.key_count(cursor), KeyCount::Estimate(100));

    colt.ensure_forced(cursor, 0);
    assert_eq!(colt.key_count(cursor), KeyCount::Exact(100));
}

#[test]
fn reset_retains_selection_capacity() {
    let dir = TempDir::new("colt-select-reset");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..500).map(|i| (i % 5, i)).collect();
    let image = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&image), &scalars(&[0]), vec![vec![1]]);
    colt.select(&[vec![3]]).expect("key 3 exists");
    let first = colt.watermark();
    colt.reset(apply(&image, &[], &[], Vec::new()));
    assert_eq!(colt.watermark(), 1, "reset empties the pools");
    colt.select(&[vec![3]]).expect("key 3 exists");
    assert_eq!(colt.watermark(), first, "same shape, same footprint");
}

#[test]
fn start_before_select_panics() {
    let dir = TempDir::new("colt-hard-start");
    let schema = schema();
    let view = view_of(&dir, &schema, &[(1, 5)]);
    let colt = Colt::new(all(&view), &scalars(&[0]), vec![vec![1]]);
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

#[test]
fn set_probes_union_survivors_per_element() {
    let dir = TempDir::new("colt-select-set");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&view), &set_level(0), vec![vec![1]]);
    let cursor = colt.select(&[vec![3, 7]]).expect("both keys exist");
    let entries = drain(&mut colt, cursor, 0);
    assert_eq!(entries.len(), 200, "the union of k = 3 and k = 7");
    assert!(
        entries
            .iter()
            .all(|(key, _)| key[0] % 10 == 3 || key[0] % 10 == 7)
    );

    let cursor = colt.select(&[vec![7]]).expect("key 7 exists");
    assert_eq!(drain(&mut colt, cursor, 0).len(), 100);

    let cursor = colt.select(&[vec![7, 42]]).expect("key 7 exists");
    assert_eq!(drain(&mut colt, cursor, 0).len(), 100);
    assert!(colt.select(&[vec![41, 42]]).is_none());
}

#[test]
fn a_single_position_union_pins_a_row() {
    let dir = TempDir::new("colt-select-set-pin");
    let schema = schema();
    let view = view_of(&dir, &schema, &[(1, 10), (2, 20), (3, 30)]);
    let mut colt = Colt::new(all(&view), &set_level(0), vec![vec![1]]);
    let cursor = colt.select(&[vec![2, 9]]).expect("key 2 exists");
    assert!(matches!(cursor, Cursor::Row(_)), "one survivor pins");
    let entries = drain(&mut colt, cursor, 0);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, vec![20]);
}

#[test]
fn set_levels_chain_with_scalar_levels() {
    let dir = TempDir::new("colt-select-set-chain");
    let schema = schema();

    let rows: Vec<(u64, u64)> = (0..100).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(
        all(&view),
        &[
            SelectionLevel::Point { columns: vec![0] },
            SelectionLevel::Set { columns: vec![1] },
        ],
        vec![vec![]],
    );

    let cursor = colt
        .select(&[vec![7], vec![7, 40, 47, 87]])
        .expect("hits exist");
    assert_eq!(drain(&mut colt, cursor, 0).len(), 3);

    assert!(colt.select(&[vec![7], vec![40, 50]]).is_none());
}

#[test]
fn set_rebinds_reach_a_pool_fixpoint() {
    let dir = TempDir::new("colt-select-set-fixpoint");
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i % 10, i)).collect();
    let view = view_of(&dir, &schema, &rows);
    let mut colt = Colt::new(all(&view), &set_level(0), vec![vec![1]]);
    let run = |colt: &mut Colt, words: Vec<u64>| {
        let cursor = colt.select(&[words]).expect("keys exist");

        colt.ensure_forced(cursor, 0);
        colt.watermark()
    };
    let first_a = run(&mut colt, vec![3, 7]);
    let first_b = run(&mut colt, vec![2, 4, 8]);
    for _ in 0..5 {
        assert_eq!(run(&mut colt, vec![3, 7]), first_a, "fixpoint under A");
        assert_eq!(run(&mut colt, vec![2, 4, 8]), first_b, "fixpoint under B");
    }
}
