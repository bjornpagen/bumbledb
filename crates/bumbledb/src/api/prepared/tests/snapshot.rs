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
fn trim_releases_cached_images_and_answers_stay_identical() {
    // Q-LIFETIME: the prepared query's retained images/views release on
    // trim; the next execution rebuilds and answers identically.
    let fix = posting_store(
        "prepared-snapshot-trim",
        &[(1, 7, "a", 10), (2, 7, "b", 20)],
    );
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let params = [BindValue::U64(7), BindValue::I64(-100)];
    let before = answers_of(&fix.execute(&mut prepared, &params).expect("execute"));
    assert!(
        prepared.retained_cache_bytes() > 0,
        "executions retain built images"
    );
    let warmed = prepared.retained_cache_bytes();

    prepared.trim();
    assert!(
        prepared.retained_cache_bytes() < warmed,
        "trim released the generation-keyed images"
    );
    let after = answers_of(&fix.execute(&mut prepared, &params).expect("re-execute"));
    assert_eq!(before, after, "a trim changes cost, never answers");
}

#[cfg(feature = "trace")]
#[test]
fn prepare_emits_no_image_events() {
    use crate::obs;

    let fix = posting_store("prepared-snapshot-noimage", &[(1, 7, "a", 10)]);

    obs::start_capture();
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let events = obs::finish_capture();
    let names: Vec<obs::TracePoint> = events.iter().map(|e| e.point()).collect();
    assert!(
        !names.contains(&obs::names::IMAGE_BUILD),
        "prepare built an image: {names:?}"
    );
    assert!(
        !names.contains(&obs::names::CACHE_HIT),
        "prepare touched the image cache: {names:?}"
    );

    obs::start_capture();
    fix.execute(&mut prepared, &[BindValue::U64(7), BindValue::I64(-100)])
        .expect("execute");
    let events = obs::finish_capture();
    let names: Vec<obs::TracePoint> = events.iter().map(|e| e.point()).collect();
    assert!(
        names.contains(&obs::names::IMAGE_BUILD),
        "the first execution pays the build: {names:?}"
    );
}
