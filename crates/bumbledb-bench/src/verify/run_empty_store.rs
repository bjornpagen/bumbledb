use super::run::{family_lane, random_lane};
use super::{Db, EMPTY_STORE_RANDOM_CASES, MAX_BUNDLES, Run, VerifyConfig};

use crate::querygen::target;
use crate::schema::{Ledger, schema};
use crate::sqlmap;

/// # Panics
/// On tool-level invariant violations, including the structural gate check: the
/// randomized slice must contain at least one gate-bearing query, so gate
/// falsity is exercised by construction, not by luck.
pub(super) fn run_empty_store<S>(cfg: &VerifyConfig, run: &mut Run<'_, S>) {
    let empty_dir = cfg.out_dir.join("empty-db");
    let _ = std::fs::remove_dir_all(&empty_dir);
    let empty_db = Db::create(&empty_dir, Ledger)
        .expect("create empty store")
        .expect("accepted");
    let empty_conn = rusqlite::Connection::open_in_memory().expect("empty oracle");
    for statement in sqlmap::ddl(schema()) {
        empty_conn.execute(&statement, []).expect("empty ddl");
    }

    for statement in sqlmap::extension_ddl(&bumbledb::Theory::descriptor(Ledger)) {
        empty_conn.execute(&statement, []).expect("empty extension");
    }
    run.lane(&empty_db, &empty_conn, |lane| {
        family_lane(lane, cfg, "empty family", &|_| None);
    });

    let empty_cal_dir = cfg.out_dir.join("empty-cal-db");
    let _ = std::fs::remove_dir_all(&empty_cal_dir);
    let empty_cal = Db::create(&empty_cal_dir, crate::calendar::Scheduling)
        .expect("create empty calendar")
        .expect("accepted");
    let cal_conn = rusqlite::Connection::open_in_memory().expect("empty calendar oracle");
    for statement in crate::calendar::corpus::ddl() {
        cal_conn
            .execute(&statement, [])
            .expect("empty calendar ddl");
    }
    run.lane(&empty_cal, &cal_conn, |lane| {
        super::run_calendar::calendar_lane(lane, cfg, "empty calendar", false);
    });

    let empty_target_dir = cfg.out_dir.join("empty-target-db");
    let _ = std::fs::remove_dir_all(&empty_target_dir);
    let empty_target = target::publish_admitted(&empty_target_dir);
    let target_conn = rusqlite::Connection::open_in_memory().expect("empty target oracle");
    for statement in sqlmap::schema_ddl(target::schema()) {
        target_conn
            .execute(&statement, [])
            .expect("empty target ddl");
    }
    for statement in sqlmap::extension_ddl(&target::descriptor()) {
        target_conn
            .execute(&statement, [])
            .expect("empty target extension");
    }
    let mut gate_bearing = 0u32;
    let mut naive_routed = Vec::new();
    run.lane(&empty_target, &target_conn, |lane| {
        random_lane(
            lane,
            cfg,
            EMPTY_STORE_RANDOM_CASES,
            0x0112_0002,
            "empty random",
            |query| {
                gate_bearing +=
                    u32::from(query.rules()[0].atoms.iter().any(|a| a.bindings.is_empty()));
            },
            &mut naive_routed,
        );
    });

    if !naive_routed.is_empty() {
        let mut naive = crate::naive::NaiveDb::new(&target::descriptor());
        super::run::naive_routed_lane(
            run,
            "empty random",
            &empty_target,
            &mut naive,
            &naive_routed,
        );
    }

    assert!(
        run.bundles.len() >= MAX_BUNDLES || gate_bearing > 0,
        "the empty-store slice generated no gate-bearing query"
    );
}
