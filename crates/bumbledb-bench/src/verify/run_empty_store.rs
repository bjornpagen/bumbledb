use super::run::{family_lane, random_lane};
use super::{Db, EMPTY_STORE_RANDOM_CASES, MAX_BUNDLES, Run, VerifyConfig};

use crate::querygen::target;
use crate::schema::{Ledger, schema};
use crate::sqlmap;

/// The empty-store pass: a fresh store pair with the schema loaded and
/// **zero facts anywhere**, over which every family (the ledger pair) and
/// a seeded slice of randomized queries (a target-schema pair) run and
/// compare. Every gate is false, every scan empty, every aggregate folds
/// nothing (the empty-set-not-NULL rule and the HAVING template earn
/// their keep), every selection misses — the entire empty-relation
/// semantic surface, oracle-checked in milliseconds with zero corpus
/// churn. Cases count into the stamp's evidence; bundles land beside the
/// main run's.
///
/// # Panics
///
/// On tool-level invariant violations, including the structural gate
/// check: the randomized slice must contain at least one gate-bearing
/// query, so gate falsity is exercised by construction, not by luck.
pub(super) fn run_empty_store<S>(cfg: &VerifyConfig, run: &mut Run<'_, S>) {
    let empty_dir = cfg.out_dir.join("empty-db");
    let _ = std::fs::remove_dir_all(&empty_dir);
    let empty_db = Db::create(&empty_dir, Ledger).expect("create empty store");
    let empty_conn = rusqlite::Connection::open_in_memory().expect("empty oracle");
    for statement in sqlmap::ddl(schema()) {
        empty_conn.execute(&statement, []).expect("empty ddl");
    }
    // "Empty" spares no axiom: a closed relation is never empty — the
    // engine answers its extension virtually over a rowless store, so
    // the mirror carries the same ground rows.
    for statement in sqlmap::extension_ddl(&bumbledb::Theory::descriptor(Ledger)) {
        empty_conn.execute(&statement, []).expect("empty extension");
    }
    run.lane(&empty_db, &empty_conn, |lane| {
        family_lane(lane, cfg, "empty family", &|_| None);
    });

    // The calendar families over an empty pair of their own: every
    // union arm empty, the Pack folding nothing, the anti-probe
    // vacuously true against an empty gate.
    let empty_cal_dir = cfg.out_dir.join("empty-cal-db");
    let _ = std::fs::remove_dir_all(&empty_cal_dir);
    let empty_cal =
        Db::create(&empty_cal_dir, crate::calendar::Scheduling).expect("create empty calendar");
    let cal_conn = rusqlite::Connection::open_in_memory().expect("empty calendar oracle");
    for statement in crate::calendar::corpus::ddl() {
        cal_conn
            .execute(&statement, [])
            .expect("empty calendar ddl");
    }
    run.lane(&empty_cal, &cal_conn, |lane| {
        super::run_calendar::calendar_lane(lane, cfg, "empty calendar", false);
    });

    // The randomized slice runs over an empty target-schema pair (the
    // generated queries speak the target ledger).
    let empty_target_dir = cfg.out_dir.join("empty-target-db");
    let _ = std::fs::remove_dir_all(&empty_target_dir);
    let empty_target = Db::create(&empty_target_dir, target::Target).expect("empty target");
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
    run.lane(&empty_target, &target_conn, |lane| {
        random_lane(
            lane,
            cfg,
            EMPTY_STORE_RANDOM_CASES,
            0x0112_0002,
            "empty random",
            |query| {
                gate_bearing +=
                    u32::from(query.rules[0].atoms.iter().any(|a| a.bindings.is_empty()));
            },
        );
    });
    // The structural check holds only for a full slice — a bundle-budget
    // cutoff already fails the run.
    assert!(
        run.bundles.len() >= MAX_BUNDLES || gate_bearing > 0,
        "the empty-store slice generated no gate-bearing query"
    );
}
