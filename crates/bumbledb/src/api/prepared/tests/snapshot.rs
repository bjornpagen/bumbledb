use super::*;

#[test]
fn pinned_plan_reads_fresh_data_at_newer_generations() {
    let dir = TempDir::new("prepared-fresh-data");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "old", 1)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let mut out = ResultBuffer::new();
    prepared
        .execute(
            &txn,
            &cache,
            &[BindValue::U64(7), BindValue::I64(0)],
            &mut out,
        )
        .expect("execute");
    assert_eq!(out.len(), 1);
    drop(txn);

    // New commit, new snapshot: the pinned *plan* runs over fresh data.
    insert_postings(&env, &schema, &[(2, 7, "new", 2)]);
    let txn = env.read_txn().expect("txn");
    prepared
        .execute(
            &txn,
            &cache,
            &[BindValue::U64(7), BindValue::I64(0)],
            &mut out,
        )
        .expect("execute");
    assert_eq!(out.len(), 2);
}

/// Prepare pins no image — the refcount
/// proof. Executions bind views; a commit plus one execution at the
/// new generation reaps every stale binding, releasing the old
/// image entirely (only the test's own Arc survives).
#[test]
fn prepare_pins_no_images_and_reaping_releases_them() {
    let dir = TempDir::new("prepared-unbound-views");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 10), (2, 7, "b", 20)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let held = cache
        .get_or_build(&txn, &schema, POSTING)
        .expect("generation-1 image");
    let baseline = std::sync::Arc::strong_count(&held);

    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    assert_eq!(
        std::sync::Arc::strong_count(&held),
        baseline,
        "prepare pinned an image"
    );

    // Two residual windows: the active and one parked binding both
    // hold views over the generation-1 image.
    for floor in [-100, 15] {
        prepared
            .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(floor)])
            .expect("execute");
    }
    assert!(
        std::sync::Arc::strong_count(&held) > baseline,
        "executions bind real views"
    );
    drop(txn);

    // Commit generation 2 and evict, exactly as Db::write does; the
    // first execution at the new generation reaps the stale parked
    // binding and rebuilds the active one.
    insert_postings(&env, &schema, &[(3, 7, "c", 30)]);
    cache.evict_older_than(2);
    let txn = env.read_txn().expect("txn");
    prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100)])
        .expect("execute at generation 2");
    assert_eq!(
        std::sync::Arc::strong_count(&held),
        1,
        "the prepared query holds nothing of generation 1"
    );
}

/// Prepare on a cold cache builds no images — zero
/// `image_build`/`cache_hit` events; the first execution pays the
/// build exactly where a cold execution always paid it.
#[cfg(feature = "trace")]
#[test]
fn prepare_emits_no_image_events() {
    use crate::obs;

    let dir = TempDir::new("prepared-no-image-events");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a", 10)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    obs::start_capture();
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let events = obs::finish_capture();
    let names: Vec<&str> = events.iter().map(|e| e.name).collect();
    assert!(
        !names.contains(&obs::names::IMAGE_BUILD),
        "prepare built an image: {names:?}"
    );
    assert!(
        !names.contains(&obs::names::CACHE_HIT),
        "prepare touched the image cache: {names:?}"
    );

    obs::start_capture();
    prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(-100)])
        .expect("execute");
    let events = obs::finish_capture();
    let names: Vec<&str> = events.iter().map(|e| e.name).collect();
    assert!(
        names.contains(&obs::names::IMAGE_BUILD),
        "the first execution pays the build: {names:?}"
    );
}
