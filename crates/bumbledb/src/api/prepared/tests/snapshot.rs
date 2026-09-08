use super::*;

#[test]
fn pinned_plan_reads_fresh_data_at_newer_generations() {
    let fix = posting_store("prepared-snapshot-generations", &[(1, 7, "old", 1)]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let mut out = Answers::new();
    fix.execute_into(
        &mut prepared,
        &[BindValue::U64(7), BindValue::I64(0)],
        &mut out,
    )
    .expect("execute");
    assert_eq!(out.len(), 1);

    fix.insert_dyn(POSTING, &posting_rows(&[(2, 7, "new", 2)]));
    fix.execute_into(
        &mut prepared,
        &[BindValue::U64(7), BindValue::I64(0)],
        &mut out,
    )
    .expect("execute");
    assert_eq!(out.len(), 2, "stale plans read fresh generations");
}

#[test]
fn query_release_and_database_cache_clear_preserve_answers() {
    let fix = posting_store(
        "prepared-snapshot-trim",
        &[(1, 7, "a", 10), (2, 7, "b", 20)],
    );
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let params = [BindValue::U64(7), BindValue::I64(-100)];
    let before = answers_of(&fix.execute(&mut prepared, &params).expect("execute"));
    assert!(
        prepared.cache.image_count() > 0,
        "executions retain built images"
    );
    let warmed = prepared.cache.image_count();

    prepared.release_memory();
    assert_eq!(
        prepared.cache.image_count(),
        warmed,
        "query release does not clear shared cache"
    );
    fix.db.clear_cache();
    assert!(
        prepared.cache.image_count() < warmed,
        "database clear released the unpinned generation-keyed images"
    );
    let after = answers_of(&fix.execute(&mut prepared, &params).expect("re-execute"));
    assert_eq!(before, after, "a trim changes cost, never answers");
}

#[test]
fn database_clear_invalidates_warm_text_views_before_finalization() {
    let fix = posting_store(
        "prepared-sibling-text-generation",
        &[(1, 7, "alpha", 10), (2, 7, "beta", 20)],
    );
    let mut reader = fix.prepare(&by_account_query()).expect("reader");
    let mut sibling = fix.prepare(&by_memo_query()).expect("sibling");
    let params = [BindValue::U64(7), BindValue::I64(-100)];
    let before = answers_of(&fix.execute(&mut reader, &params).expect("warm reader"));
    fix.db.clear_cache();
    fix.execute(&mut sibling, &memo_param("beta"))
        .expect("new token order");
    let after = answers_of(
        &fix.execute(&mut reader, &params)
            .expect("reader after trim"),
    );
    assert_eq!(before, after, "old image tokens cannot use a new resolver");
}

#[test]
fn prepare_leaves_the_image_cache_empty_until_execution() {
    let fix = posting_store("prepared-snapshot-noimage", &[(1, 7, "a", 10)]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    assert_eq!(prepared.cache.image_count(), 0);
    fix.execute(&mut prepared, &[BindValue::U64(7), BindValue::I64(-100)])
        .expect("execute");
    assert_eq!(prepared.cache.image_count(), 1);
}
