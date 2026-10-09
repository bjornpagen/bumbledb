#[test]
fn runner_post_write_alternation_restores_the_loaded_state() {
    use crate::worlds::corpus_gen::{GenConfig, Scale};
    let dir = crate::fixture::TempDir::new("appperf-postwrite");
    let cfg = GenConfig {
        seed: 1,
        scale: Scale::Tiny,
    };
    let db = bumbledb::Db::create(
        &dir,
        crate::worlds::ledger::Ledger,
        crate::harness::bench_work(),
    )
    .expect("create")
    .expect("accepted");
    crate::worlds::corpus::load_bumbledb(&db, cfg).expect("load");
    let before = db
        .read(crate::harness::bench_work(), |snap| {
            snap.count(crate::worlds::ledger::ids::POSTING_TAG)
        })
        .expect("count");
    let row = super::post_write_first_read(&db, cfg, Some(7)).expect("regime runs");
    assert_eq!(row.regime, super::Regime::PostWrite);
    assert_eq!(
        row.work,
        7 * before - 4,
        "four measured deletes and three reinserts; restore is untimed"
    );
    let after = db
        .read(crate::harness::bench_work(), |snap| {
            snap.count(crate::worlds::ledger::ids::POSTING_TAG)
        })
        .expect("count");
    // Four warmups plus seven samples end deleted; the runner restores it.
    assert_eq!(
        before, after,
        "alternating delete/insert restores the store"
    );
    drop(db);
}

#[test]
fn runner_large_result_reports_split_segments_below_end_to_end() {
    use crate::worlds::corpus_gen::{GenConfig, Scale};
    let dir = crate::fixture::TempDir::new("appperf-large");
    let db = bumbledb::Db::create(
        &dir,
        crate::worlds::ledger::Ledger,
        crate::harness::bench_work(),
    )
    .expect("create")
    .expect("accepted");
    let empty = super::large_result(&db, Some(1))
        .expect("an empty query still delivers its terminal cursor frame");
    assert_eq!(empty.work, 0);
    crate::worlds::corpus::load_bumbledb(
        &db,
        GenConfig {
            seed: 2,
            scale: Scale::Tiny,
        },
    )
    .expect("load");
    let row = super::large_result(&db, Some(4)).expect("regime runs");
    let phases = row.phases.expect("split reported");
    assert_eq!(
        row.work,
        4 * crate::worlds::corpus_gen::Sizes::of(Scale::Tiny).postings
    );
    assert!(phases.execute_ns <= phases.end_to_end_ns);
    assert!(phases.deliver_ns <= phases.end_to_end_ns);
    drop(db);
}

#[test]
fn runner_tenant_churn_releases_descriptors_and_reports_latency() {
    let dir = crate::fixture::TempDir::new("appperf-churn");
    let row = super::tenant_churn(&dir, 3, 12, 7).expect("churn runs");
    assert_eq!(row.regime, super::Regime::TenantChurn);
    assert!(row.stats.p99 >= row.stats.p50);
    if let Some(leaked) = row.fd_growth {
        assert!(
            leaked <= 2,
            "activation churn must not accumulate descriptors, grew by {leaked}"
        );
    }
}

#[test]
fn runner_cold_open_times_open_plus_first_read() {
    use crate::worlds::corpus_gen::{GenConfig, Scale};
    let dir = crate::fixture::TempDir::new("appperf-cold");
    let db = bumbledb::Db::create(
        &dir,
        crate::worlds::ledger::Ledger,
        crate::harness::bench_work(),
    )
    .expect("create")
    .expect("accepted");
    crate::worlds::corpus::load_bumbledb(
        &db,
        GenConfig {
            seed: 3,
            scale: Scale::Tiny,
        },
    )
    .expect("load");
    drop(db);
    let row = super::cold_open(&dir, Some(2)).expect("cold regime runs");
    assert_eq!(row.regime, super::Regime::ColdOpen);
    assert_eq!(
        row.work,
        2 * crate::worlds::corpus_gen::Sizes::of(Scale::Tiny).accounts
    );
}

#[test]
fn runner_refuses_existing_output_without_erasing_evidence() {
    let dir = crate::fixture::TempDir::new("appperf-existing");
    std::fs::create_dir(&dir).expect("fresh test root");
    let sentinel = dir.join("app-perf.json");
    std::fs::write(&sentinel, "previous evidence").expect("existing artifact");
    let args = crate::cli::AppPerfArgs {
        out: Some(dir.to_path_buf()),
        ..crate::cli::AppPerfArgs::default()
    };
    let error = super::run(&args).expect_err("never overwrite a previous run");
    assert!(error.contains("fresh output"), "{error}");
    assert_eq!(
        std::fs::read_to_string(sentinel).unwrap(),
        "previous evidence"
    );
    assert!(!dir.join("scratch").exists());
}
