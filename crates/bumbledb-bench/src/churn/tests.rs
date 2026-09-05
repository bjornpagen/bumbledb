//! — the oracle gate the lane must pass before the owner ever times it.
use crate::corpus_gen::{GenConfig, Scale, Sizes};
use crate::storemode::StoreMode;

use super::engines::{self, OursLane};
use super::lanes;
use super::ops::{self, ChurnConfig, Mix};
use super::report::Counters;
use super::run;
use super::verify_end;

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-bench-churn-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn drive(
    cfg: &ChurnConfig,
    mix: &Mix,
    mirror_labels: &[&'static str],
    tag: &str,
) -> (
    OursLane,
    Vec<(&'static str, rusqlite::Connection)>,
    ops::LiveSet,
    std::path::PathBuf,
) {
    ops::validate(cfg, mix).expect("a smoke config validates");
    let dir = scratch(tag);
    let mut lane =
        engines::create_ours(&dir.join("ours"), cfg.r#gen, StoreMode::Durable).expect("ours lane");
    let mirrors: Vec<(&'static str, rusqlite::Connection)> = mirror_labels
        .iter()
        .map(|label| {
            let path = dir.join(format!("mirror-{label}.sqlite"));
            let conn = engines::create_sqlite(&path, cfg.r#gen).expect("mirror");
            (*label, conn)
        })
        .collect();
    let mut live = ops::LiveSet::from_corpus(cfg.r#gen);
    for cycle in 1..=cfg.cycles {
        let plan = ops::cycle_plan(cfg.r#gen, mix, cycle, live.len());
        let removals = live.resolve(&plan);
        let added = engines::apply_ours(&mut lane, &removals, &plan.bodies).expect("apply ours");
        for (label, conn) in &mirrors {
            engines::apply_sqlite(conn, &removals, &added)
                .unwrap_or_else(|e| panic!("apply sqlite-{label}: {e}"));
        }
        live.apply(&plan, added);
    }
    (lane, mirrors, live, dir)
}

fn assert_agreement(
    lane: &OursLane,
    mirrors: &[(&'static str, rusqlite::Connection)],
    live: &ops::LiveSet,
) {
    let refs: Vec<(&str, &rusqlite::Connection)> =
        mirrors.iter().map(|(label, conn)| (*label, conn)).collect();
    verify_end::assert_end_state(lane, &refs, live).expect("end states agree");
}

fn tiny_postings() -> usize {
    usize::try_from(Sizes::of(Scale::Tiny).postings).expect("64-bit usize")
}

#[test]
fn churn_smoke_end_states_agree_three_ways() {
    // Two independently-loaded FULL-sync mirrors: replication and the
    // end-state gate are mirror-instance independent (the OFF mirror went
    // with the retired nosync lane — ENG-008).
    let cfg = ChurnConfig::smoke(1);
    let (lane, mirrors, live, dir) = drive(&cfg, &ops::STEADY, &["a", "b"], "smoke");
    assert_agreement(&lane, &mirrors, &live);
    assert_eq!(
        live.len(),
        tiny_postings(),
        "steady state: facts in == facts out"
    );
    drop((lane, mirrors));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn churn_growth_mode_grows_the_working_set() {
    let cfg = ChurnConfig {
        cycles: 4,
        sample_every: 2,
        ..ChurnConfig::smoke(2)
    };
    let mix = Mix {
        churn: 8,
        updates: 4,
        growth: 2,
    };
    let (lane, mirrors, live, dir) = drive(&cfg, &mix, &["full"], "growth");
    assert_agreement(&lane, &mirrors, &live);
    assert_eq!(live.len(), tiny_postings() + 8, "4 cycles x growth 2");
    drop((lane, mirrors));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn churn_delete_heavy_end_state_agrees() {
    let cfg = ChurnConfig {
        cycles: 4,
        sample_every: 2,
        ..ChurnConfig::smoke(3)
    };
    let (lane, mirrors, live, dir) = drive(&cfg, &ops::DELETE_HEAVY, &["full"], "delete-heavy");
    assert_agreement(&lane, &mirrors, &live);
    assert_eq!(live.len(), tiny_postings(), "delete-heavy stays steady");
    drop((lane, mirrors));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn churn_replay_is_deterministic() {
    let cfg = ChurnConfig::smoke(4);
    let (first, _, _, dir_a) = drive(&cfg, &ops::STEADY, &[], "replay-a");
    let (second, _, _, dir_b) = drive(&cfg, &ops::STEADY, &[], "replay-b");
    let ours_a = verify_end::posting_multiset_ours(&first.db).expect("first multiset");
    let ours_b = verify_end::posting_multiset_ours(&second.db).expect("second multiset");
    crate::compare::multisets(ours_a, ours_b).expect("replayed multisets are equal");
    assert_eq!(
        first.last_minted, second.last_minted,
        "the id burn replays exactly"
    );
    drop((first, second));
    let _ = std::fs::remove_dir_all(&dir_a);
    let _ = std::fs::remove_dir_all(&dir_b);
}

/// The delete-bearing contract falsified from both sides (the `posting_swap`
/// test's twin): a live removal commits; the SAME removal again must refuse the
/// whole cycle, and the refusal commits nothing — the generation does not move.
#[test]
fn churn_stale_removal_refuses_the_whole_cycle() {
    let dir = scratch("stale");
    let cfg = ChurnConfig::smoke(6);
    let mut lane =
        engines::create_ours(&dir.join("ours"), cfg.r#gen, StoreMode::Durable).expect("ours lane");
    let live = ops::LiveSet::from_corpus(cfg.r#gen);
    let victim = live.rows()[0];
    engines::apply_ours(&mut lane, std::slice::from_ref(&victim), &[])
        .expect("the live removal commits");
    let generation = lane.db.generation().expect("generation");
    let refusal = engines::apply_ours(&mut lane, &[victim], &[]);
    assert!(
        refusal.is_err(),
        "a stale removal must refuse the whole cycle"
    );
    assert_eq!(
        lane.db.generation().expect("generation"),
        generation,
        "a refused cycle must leave the store untouched"
    );
    drop(lane);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn churn_cycle_plan_is_pure_and_distinct() {
    let r#gen = GenConfig {
        seed: 9,
        scale: Scale::Tiny,
    };
    let mix = ops::STEADY;
    let live_len = tiny_postings();
    let plan = ops::cycle_plan(r#gen, &mix, 5, live_len);
    assert_eq!(
        plan,
        ops::cycle_plan(r#gen, &mix, 5, live_len),
        "the plan is a pure function of (seed, cycle, live_len)"
    );
    let mut seen = std::collections::BTreeSet::new();
    for &index in plan.updates.iter().chain(plan.deletes.iter()) {
        assert!(index < live_len, "every removal index is live");
        assert!(seen.insert(index), "removal indices are distinct");
    }
    assert_eq!(
        u64::try_from(plan.updates.len()).expect("fits"),
        mix.updates
    );
    assert_eq!(u64::try_from(plan.deletes.len()).expect("fits"), mix.churn);
    assert_eq!(
        u64::try_from(plan.bodies.len()).expect("fits"),
        mix.arrivals()
    );
}

/// ENG-008: every churn mirror is FULL sync — no weakened pragma set is
/// reachable from the registry or the constructor.
#[test]
fn churn_mirrors_are_full_sync_always() {
    let dir = scratch("full-sync");
    let r#gen = GenConfig {
        seed: 8,
        scale: Scale::Tiny,
    };
    let full = engines::create_sqlite(&dir.join("full.sqlite"), r#gen).expect("full mirror");
    let sync: i64 = full
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .expect("pragma");
    assert_eq!(sync, 2, "FULL");
    drop(full);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every refusal arm of [`ops::validate`], hit once — and the shipped configs
/// pass.
#[test]
fn churn_validate_refuses_bad_configs() {
    let good = ChurnConfig::smoke(1);
    assert!(ops::validate(&good, &ops::STEADY).is_ok());
    assert!(
        ops::validate(&good, &ops::DELETE_HEAVY).is_ok(),
        "Tiny admits DELETE_HEAVY exactly: 1024 == 2 x 512"
    );

    let zero_cycles = ChurnConfig {
        cycles: 0,
        ..good.clone()
    };
    assert!(ops::validate(&zero_cycles, &ops::STEADY).is_err());

    let zero_stride = ChurnConfig {
        sample_every: 0,
        ..good.clone()
    };
    assert!(ops::validate(&zero_stride, &ops::STEADY).is_err());

    let off_boundary = ChurnConfig {
        cycles: 7,
        sample_every: 3,
        ..good.clone()
    };
    assert!(
        ops::validate(&off_boundary, &ops::STEADY).is_err(),
        "samples must land on cycle boundaries"
    );

    let zero_vacuum = ChurnConfig {
        vacuum_every: 0,
        ..good.clone()
    };
    assert!(ops::validate(&zero_vacuum, &ops::STEADY).is_err());

    let zero_analyze = ChurnConfig {
        analyze_every: 0,
        ..good.clone()
    };
    assert!(ops::validate(&zero_analyze, &ops::STEADY).is_err());

    let empty_mix = Mix {
        churn: 0,
        updates: 0,
        growth: 0,
    };
    assert!(ops::validate(&good, &empty_mix).is_err());

    let past_the_floor = Mix {
        churn: 600,
        updates: 0,
        growth: 0,
    };
    assert!(
        ops::validate(&good, &past_the_floor).is_err(),
        "2 x 600 > Tiny's 1024 postings"
    );
}

const PROBE_NAMES: [&str; 3] = ["churn_point", "churn_balance", "churn_window"];

fn assert_series_shape(series: &super::report::RunSeries, cycles: &[u64]) {
    for lane in &series.lanes {
        let sampled: Vec<u64> = lane.samples.iter().map(|sample| sample.cycle).collect();
        assert_eq!(sampled, cycles, "{}: samples land on the stride", lane.lane);
        for sample in &lane.samples {
            let names: Vec<&str> = sample
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect();
            assert_eq!(names, PROBE_NAMES, "{}: the probe registry", lane.lane);
            assert!(sample.disk_bytes > 0, "{}: disk_bytes > 0", lane.lane);
            assert!(
                sample.commits_per_sec > 0.0,
                "{}: commits_per_sec > 0",
                lane.lane
            );
        }
    }
}

#[test]
fn churn_run_steady_smoke_produces_the_full_series() {
    let cfg = ChurnConfig::smoke(1);
    let dir = scratch("run-steady");
    let spec = &lanes::all()[0];
    assert_eq!(spec.name, "steady");
    let series = run::run_spec(spec, &cfg, &dir).expect("the steady run drives");
    let labels: Vec<&str> = series.lanes.iter().map(|lane| lane.lane.as_str()).collect();
    assert_eq!(labels, ["ours-durable", "sqlite-bare", "sqlite-maint"]);
    assert_series_shape(&series, &[3, 6]);
    let ours = &series.lanes[0];
    assert_eq!(ours.engine, super::report::Engine::Bumbledb);
    for sample in &ours.samples {
        let Counters::Ours {
            generation,
            id_high_water,
        } = sample.counters
        else {
            panic!("the ours lane carries ours counters");
        };
        assert!(
            id_high_water > 1023,
            "application-owned mints moved the high-water"
        );
        assert!(generation > 0, "committed cycles moved the generation");
        assert_eq!(sample.maintenance_ns, 0, "ours never maintains");
    }
    for lane in &series.lanes[1..] {
        assert_eq!(lane.engine, super::report::Engine::Sqlite);
        for sample in &lane.samples {
            let Counters::Sqlite { page_count, .. } = sample.counters else {
                panic!("a sqlite lane carries sqlite counters");
            };
            assert!(page_count > 0, "{}: a loaded store has pages", lane.lane);
        }
    }
    assert!(
        series.lanes[2]
            .samples
            .iter()
            .any(|sample| sample.maintenance_ns > 0),
        "vacuum_every 2 guarantees maintenance inside every maint window"
    );
    assert!(
        series.lanes[1]
            .samples
            .iter()
            .all(|sample| sample.maintenance_ns == 0),
        "the bare lane never maintains"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn churn_run_delete_heavy_smoke_agrees() {
    let cfg = ChurnConfig {
        cycles: 4,
        sample_every: 2,
        ..ChurnConfig::smoke(3)
    };
    let dir = scratch("run-delete-heavy");
    let spec = &lanes::all()[1];
    assert_eq!(spec.name, "delete-heavy");
    let series = run::run_spec(spec, &cfg, &dir).expect("the delete-heavy run drives");
    assert_eq!(series.lanes.len(), 2, "one minter, one twin");
    assert_series_shape(&series, &[2, 4]);
    let last = series.lanes[0].samples.last().expect("a final sample");
    let Counters::Ours { id_high_water, .. } = last.counters else {
        panic!("the ours lane carries ours counters");
    };
    assert_eq!(
        id_high_water,
        1023 + 2048,
        "the monotone burn is exact: 4 cycles x 512 mints over the initial 1023"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn churn_registry_covers_the_mandated_lanes() {
    let specs = lanes::all();
    assert_eq!(
        specs.len(),
        2,
        "two rows cover the three surviving lanes (the ephemeral/nosync \
         pair retired with ENG-008)"
    );
    let mut labels = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for spec in specs {
        assert!(names.insert(spec.name), "spec names are unique");
        labels.insert(lanes::ours_label(spec.ours));
        for kind in spec.sqlite {
            labels.insert(kind.label());
        }
    }
    let mandated: std::collections::BTreeSet<&str> =
        ["ours-durable", "sqlite-bare", "sqlite-maint"]
            .into_iter()
            .collect();
    assert_eq!(labels, mandated, "the three surviving lanes, exactly");
}
