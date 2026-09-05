//! Authored F1, executed F3. Gate mapping: `scorecard_*` → APP-FAST/MUTATE/
//! NUMERIC/LARGE/TENANTS/TARGETS/METHOD/MAGIC structure; `layers_*` →
//! the chapter 40 §7 decomposition contract (G15/RUN family evidence);
//! `hosted_*` → PERF-003; `runner_*` → APP-FAST/MUTATE/TENANTS mechanics
//! (tiny corpora — the measured F3 cells use real scales).

use super::hosted::{
    self, CommitCostSample, ContentionCell, HistoryMode, KeyMode, TerminalOutcome, WRITER_COUNTS,
};
use super::layers::{self, Layer, LayerSample};
use super::workloads::{self, FAMILIES, Family};
use super::{GATES, Gate, PhaseSplit, Regime};

// Scorecard structure.

#[test]
fn scorecard_is_compact_and_names_every_family_once() {
    let cells = workloads::scorecard();
    assert_eq!(
        cells.len(),
        13,
        "compact 13-cell scorecard, not a cartesian: {} cells",
        cells.len()
    );
    for family in FAMILIES {
        assert!(
            cells.iter().any(|cell| cell.family == family),
            "{} has no cell",
            family.label()
        );
    }
}

#[test]
fn scorecard_every_gate_has_at_least_one_cell() {
    let cells = workloads::scorecard();
    for gate in GATES {
        assert!(
            cells.iter().any(|cell| cell.gate == gate),
            "{} has no cell",
            gate.label()
        );
    }
}

#[test]
fn scorecard_names_the_special_regimes_once_each_at_least() {
    let cells = workloads::scorecard();
    for regime in [
        Regime::Selective,
        Regime::TenantChurn,
        Regime::HostedContention,
        Regime::Maintenance,
    ] {
        assert!(
            cells.iter().any(|cell| cell.regime == regime),
            "missing {}",
            regime.label()
        );
    }
}

#[test]
fn scorecard_cells_name_visits_or_requests_not_elapsed_smoke() {
    for cell in workloads::scorecard() {
        let extras = cell.extra_outputs.join(" ");
        assert!(
            extras.contains("visit")
                || extras.contains("owner")
                || extras.contains("request")
                || extras.contains("bit")
                || extras.contains("roster")
                || extras.contains("distribution")
                || extras.contains("class"),
            "{} reports ritual time instead of work: {extras}",
            cell.id
        );
    }
}

#[test]
fn scorecard_cells_all_carry_an_oracle_and_a_criterion() {
    for cell in workloads::scorecard() {
        assert!(!cell.oracle.is_empty(), "{}: no oracle", cell.id);
        assert!(!cell.criterion.is_empty(), "{}: no criterion", cell.id);
        assert!(
            !cell.extra_outputs.is_empty(),
            "{}: no extra outputs",
            cell.id
        );
    }
}

#[test]
fn scorecard_numeric_cells_demand_independent_bit_oracles() {
    let cells = workloads::scorecard();
    let numeric: Vec<_> = cells
        .iter()
        .filter(|cell| cell.gate == Gate::AppNumeric)
        .collect();
    assert!(
        numeric.len() >= 2,
        "sum/mean and intervals are separate cells"
    );
    for cell in numeric {
        assert!(
            cell.oracle.contains("independent") || cell.oracle.contains("oracle"),
            "{}: numeric oracle must be independent, got `{}`",
            cell.id,
            cell.oracle
        );
        assert!(
            !cell.oracle.contains("production accumulator applied twice"),
            "{}: never the production algorithm",
            cell.id
        );
    }
}

#[test]
fn scorecard_mutation_cells_charge_first_read_and_forbid_id_allocation_work() {
    let cells = workloads::scorecard();
    let post_write: Vec<_> = cells
        .iter()
        .filter(|cell| cell.regime == Regime::PostWrite)
        .collect();
    assert_eq!(post_write.len(), 1, "one mutation→read cell, not a cartesian");
    for cell in post_write {
        assert_eq!(cell.gate, Gate::AppMutate);
        assert_eq!(cell.family, Family::MutationRead);
        assert!(
            cell.extra_outputs
                .iter()
                .any(|o| o.contains("rebuild") || o.contains("judge")),
            "{}: first-read locality must be reported",
            cell.id
        );
    }
}

#[test]
fn scorecard_tenant_lifecycle_owns_churn_and_maintenance_evidence() {
    let cells = workloads::scorecard();
    assert!(cells.iter().any(|cell| cell.family == Family::TenantLifecycle
        && cell.regime == Regime::TenantChurn
        && cell.gate == Gate::AppTenants));
    assert!(
        cells
            .iter()
            .any(|cell| cell.family == Family::TenantLifecycle && cell.regime == Regime::Maintenance)
    );
}

#[test]
fn l21_semantic_checks_cover_the_owned_discriminators() {
    let checks = super::plan::l21_semantic_checks();
    for needed in [
        "D04",
        "D08",
        "D09",
        "D11",
        "D29",
        "G05",
        "G12",
        "G15",
        "REVIEW-001",
        "C-D04-collision-bytes",
        "C-D19-cancel",
        "C-D19-mean-once",
        "C-D19-merge-not-idemp",
        "C-G03-mutable-support",
        "C-G03-add-wins",
        "C-G03-raw-commute",
    ] {
        assert!(
            checks.iter().any(|(gate, _, _)| *gate == needed),
            "missing {needed}"
        );
    }
}

// Layer decomposition wire format.

fn sample(layer: Layer, op: &str, ns: u64) -> LayerSample {
    LayerSample {
        layer,
        op: op.to_owned(),
        ns,
        queue_ns: None,
        conv_ns: None,
        event_loop_delay_ns: None,
        bytes_copied: None,
        gc_count: None,
        external_bytes: None,
    }
}

#[test]
fn layers_json_round_trips_with_optional_fields_omitted_not_zeroed() {
    let mut full = sample(Layer::Bridge, "warm-read", 1200);
    full.queue_ns = Some(100);
    full.bytes_copied = Some(4096);
    let bare = sample(Layer::Native, "warm-read", 800);
    let text = layers::to_json(&[full.clone(), bare.clone()]);
    assert!(!text.contains("\"conv_ns\""), "absent fields are omitted");
    assert!(text.contains("\"queue_ns\":100"));
    let parsed = layers::parse_file(&text).expect("round trip");
    assert_eq!(parsed, vec![full, bare]);
}

#[test]
fn layers_parser_refuses_unknown_layers_and_missing_fields() {
    assert!(
        layers::parse_file("{\"samples\":[{\"layer\":\"promise\",\"op\":\"x\",\"ns\":1}]}")
            .is_err()
    );
    assert!(layers::parse_file("{\"samples\":[{\"op\":\"x\",\"ns\":1}]}").is_err());
    assert!(layers::parse_file("{\"samples\":[{\"layer\":\"native\",\"ns\":1}]}").is_err());
    assert!(layers::parse_file("{\"samples\":[{\"layer\":\"native\",\"op\":\"x\"}]}").is_err());
    assert!(
        layers::parse_file("{\"samples\":[{\"layer\":\"native\",\"op\":\"x\",\"ns\":-5}]}")
            .is_err(),
        "negative counters refuse"
    );
    assert!(layers::parse_file("{}").is_err());
}

#[test]
fn layers_summary_groups_by_op_and_layer_and_finds_coverage_holes() {
    let samples = vec![
        sample(Layer::Native, "warm-read", 100),
        sample(Layer::Native, "warm-read", 200),
        sample(Layer::Bridge, "warm-read", 300),
        sample(Layer::Effect, "warm-read", 400),
        sample(Layer::Effect, "ingest", 900),
    ];
    let summaries = layers::summarize(&samples);
    let native = summaries
        .iter()
        .find(|s| s.op == "warm-read" && s.layer == Layer::Native)
        .expect("native row");
    assert_eq!(native.samples, 2);
    assert!(native.p50_ns == 100 || native.p50_ns == 200);
    let holes = layers::coverage_holes(&summaries);
    assert!(
        holes
            .iter()
            .any(|h| h.contains("ingest") && h.contains("native")),
        "ingest lacks its native baseline: {holes:?}"
    );
    assert!(
        !holes.iter().any(|h| h.contains("warm-read")),
        "warm-read is fully decomposed"
    );
}

#[test]
fn layers_event_loop_delay_takes_the_max_and_bytes_accumulate() {
    let mut a = sample(Layer::Effect, "ingest", 100);
    a.event_loop_delay_ns = Some(500);
    a.bytes_copied = Some(10);
    let mut b = sample(Layer::Effect, "ingest", 100);
    b.event_loop_delay_ns = Some(1500);
    b.bytes_copied = Some(30);
    let summaries = layers::summarize(&[a, b]);
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].max_event_loop_delay_ns, Some(1500));
    assert_eq!(summaries[0].total_bytes_copied, Some(40));
}

// PERF-003 hosted accounting.

fn commit_sample(
    outcome: TerminalOutcome,
    ns: u64,
    requests: u64,
    retries: u64,
) -> CommitCostSample {
    CommitCostSample {
        outcome,
        phases: PhaseSplit {
            prepare_ns: None,
            execute_ns: None,
            deliver_ns: None,
            end_to_end_ns: ns,
        },
        queue_ns: None,
        local_durable_ns: None,
        judgment_apply_ns: None,
        publication_ns: Some(ns / 2),
        catch_up_ns: None,
        settlement_ns: None,
        requests,
        request_bytes: requests * 1000,
        response_bytes: requests * 100,
        retries,
    }
}

#[test]
fn hosted_per_terminal_summary_separates_the_four_outcomes() {
    let samples = vec![
        commit_sample(TerminalOutcome::Accepted, 10_000, 2, 0),
        commit_sample(TerminalOutcome::Accepted, 20_000, 4, 1),
        commit_sample(TerminalOutcome::Rejected, 8_000, 2, 0),
        commit_sample(TerminalOutcome::NoChange, 5_000, 1, 0),
        commit_sample(TerminalOutcome::Unknown, 50_000, 7, 3),
    ];
    hosted::check_samples(&samples).expect("lawful samples");
    let summary = hosted::per_terminal_summary(&samples);
    let accepted = summary[0];
    assert_eq!(accepted.commands, 2);
    assert_eq!(accepted.total_requests, 6);
    assert!((accepted.requests_per_command() - 3.0).abs() < f64::EPSILON);
    assert!((accepted.bytes_per_command() - 3300.0).abs() < f64::EPSILON);
    assert_eq!(summary[1].commands, 1, "rejected");
    assert_eq!(summary[2].commands, 1, "no-change");
    let unknown = summary[3];
    assert_eq!(unknown.commands, 1);
    assert_eq!(unknown.total_retries, 3, "unknown outcomes still cost");
}

#[test]
fn hosted_check_rejects_summed_timer_artifacts_and_free_unknowns() {
    let mut bad = commit_sample(TerminalOutcome::Accepted, 1_000, 1, 0);
    bad.publication_ns = Some(2_000);
    let err = hosted::check_samples(&[bad]).expect_err("segment above end-to-end");
    assert!(err.contains("critical path"), "{err}");
    let free_unknown = CommitCostSample {
        requests: 0,
        ..commit_sample(TerminalOutcome::Unknown, 1_000, 0, 0)
    };
    assert!(hosted::check_samples(&[free_unknown]).is_err());
    let phantom_retry = commit_sample(TerminalOutcome::Accepted, 1_000, 0, 2);
    assert!(hosted::check_samples(&[phantom_retry]).is_err());
}

#[test]
fn hosted_contention_schedule_covers_writers_keys_checkpoint_and_loss() {
    let cells = hosted::contention_schedule();
    for &writers in &WRITER_COUNTS {
        for key_mode in [KeyMode::SameKey, KeyMode::DisjointKeys] {
            assert!(
                cells.iter().any(|cell| cell.writers == writers
                    && cell.key_mode == key_mode
                    && cell.history_mode == HistoryMode::SharedHistory),
                "missing shared-history {writers}-writer {key_mode:?}"
            );
        }
        assert!(cells.iter().any(|cell| cell.writers == writers
            && cell.history_mode == HistoryMode::IndependentHistories));
    }
    // Same-key across independent histories is meaningless and excluded.
    assert!(!cells.iter().any(
        |cell| cell.history_mode == HistoryMode::IndependentHistories
            && cell.key_mode == KeyMode::SameKey
    ));
    // Every writer count runs with and without checkpoint and loss.
    for flag in [true, false] {
        assert!(cells.iter().any(|c| c.checkpoint_active == flag));
        assert!(cells.iter().any(|c| c.loss_injection == flag));
    }
    let full: Vec<&ContentionCell> = cells
        .iter()
        .filter(|c| c.writers == 8 && c.checkpoint_active && c.loss_injection)
        .collect();
    assert!(!full.is_empty(), "the worst cell exists");
}

// Runner mechanics on tiny corpora (real engine, tiny scale — F3 measures
// real scales; these prove the plumbing and the invariants).

#[test]
fn runner_post_write_alternation_restores_the_loaded_state() {
    use crate::corpus_gen::{GenConfig, Scale};
    let dir = std::env::temp_dir().join("bumbledb-bench-appperf-postwrite");
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = GenConfig {
        seed: 1,
        scale: Scale::Tiny,
    };
    let db = bumbledb::Db::create(
        &dir,
        crate::schema::Ledger,
        crate::harness::bench_work().expect("work"),
    )
    .expect("create")
    .expect("accepted");
    crate::corpus::load_bumbledb(&db, cfg).expect("load");
    let before = db
        .read(crate::harness::bench_work().expect("work"), |snap| {
            snap.count(crate::schema::ids::POSTING_TAG)
        })
        .expect("count");
    let row = super::runner::post_write_first_read(&db, cfg, Some(8)).expect("regime runs");
    assert_eq!(row.regime, super::Regime::PostWrite);
    assert!(
        row.work > 0 && row.account.source_visits == Some(row.work),
        "admitted visits, not a positive-time claim"
    );
    let after = db
        .read(crate::harness::bench_work().expect("work"), |snap| {
            snap.count(crate::schema::ids::POSTING_TAG)
        })
        .expect("count");
    // 4 warmups + 8 samples = 12 mutations: even count restores the corpus.
    assert_eq!(
        before, after,
        "alternating delete/insert restores the store"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn runner_large_result_reports_split_segments_below_end_to_end() {
    use crate::corpus_gen::{GenConfig, Scale};
    let dir = std::env::temp_dir().join("bumbledb-bench-appperf-large");
    let _ = std::fs::remove_dir_all(&dir);
    let db = bumbledb::Db::create(
        &dir,
        crate::schema::Ledger,
        crate::harness::bench_work().expect("work"),
    )
    .expect("create")
    .expect("accepted");
    crate::corpus::load_bumbledb(
        &db,
        GenConfig {
            seed: 2,
            scale: Scale::Tiny,
        },
    )
    .expect("load");
    let row = super::runner::large_result(&db, Some(4)).expect("regime runs");
    let phases = row.phases.expect("split reported");
    assert!(row.work > 0, "rows were delivered");
    assert!(phases.execute_ns.expect("execute") <= phases.end_to_end_ns);
    assert!(phases.deliver_ns.expect("deliver") <= phases.end_to_end_ns);
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn runner_tenant_churn_releases_descriptors_and_reports_latency() {
    let dir = std::env::temp_dir().join("bumbledb-bench-appperf-churn");
    let _ = std::fs::remove_dir_all(&dir);
    let row = super::runner::tenant_churn(&dir, 3, 12, 7).expect("churn runs");
    assert_eq!(row.regime, super::Regime::TenantChurn);
    assert!(row.stats.p99 >= row.stats.p50);
    if let Some(leaked) = row.account.live_resources {
        assert!(
            leaked <= 2,
            "activation churn must not accumulate descriptors, grew by {leaked}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn runner_cold_open_times_open_plus_first_read() {
    use crate::corpus_gen::{GenConfig, Scale};
    let dir = std::env::temp_dir().join("bumbledb-bench-appperf-cold");
    let _ = std::fs::remove_dir_all(&dir);
    let db = bumbledb::Db::create(
        &dir,
        crate::schema::Ledger,
        crate::harness::bench_work().expect("work"),
    )
    .expect("create")
    .expect("accepted");
    crate::corpus::load_bumbledb(
        &db,
        GenConfig {
            seed: 3,
            scale: Scale::Tiny,
        },
    )
    .expect("load");
    drop(db);
    let row = super::runner::cold_open(&dir).expect("cold regime runs");
    assert_eq!(row.regime, super::Regime::ColdOpen);
    assert!(row.work > 0, "the first read counted admitted rows");
    let _ = std::fs::remove_dir_all(&dir);
}
